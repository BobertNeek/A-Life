import copy
import unittest

from consolidation_retention import summarize


class RetentionTests(unittest.TestCase):
    def fixture(self, n=100):
        spec = {"domains": ["care", "language"], "domain_owners": {"care": ["cloud"], "language": ["pc"]},
                "roles": ["cloud", "pc", "candidate"], "heldout_seeds": list(range(n)),
                "development_seeds": [1001], "historical_freshness": "unknown"}
        spec["cases"] = [dict(domain=domain, seed=seed, condition="same")
                         for domain in spec["domains"] for seed in range(n)]
        rows = [dict(domain=domain, role=role, seed=seed, condition="same",
                     partition="heldout", success=True, safe=True,
                     censored=False,
                     native_evidence_verified=True, matched_inputs_verified=True,
                     offline_optimizer_updates=0, teacher_removed=True, prior_removed=True,
                     evidence_sha256="a" * 64, input_match_sha256="b" * 64)
                for domain in spec["domains"] for seed in range(n) for role in spec["roles"]]
        return spec, rows

    def test_no_observed_loss_with_sufficient_precision_can_only_reach_review(self):
        spec, rows = self.fixture()
        result = summarize(spec, rows)
        self.assertTrue(result["candidate_review"]["candidate"]["eligible_for_owner_review"])
        self.assertFalse(result["automatic_adoption"])
        self.assertFalse(result["accepted_state_changed"])
        self.assertEqual(result["historical_freshness"], "unknown")

    def test_one_skill_loss_is_not_cancelled_by_other_domain_gain(self):
        spec, rows = self.fixture()
        next(r for r in rows if r["domain"] == "language" and r["role"] == "candidate")["success"] = False
        result = summarize(spec, rows)
        self.assertFalse(result["candidate_review"]["candidate"]["eligible_for_owner_review"])
        self.assertTrue(any("observed losses" in s for s in result["candidate_review"]["candidate"]["blockers"]))

    def test_insufficient_sample_cannot_qualify(self):
        spec, rows = self.fixture(2)
        self.assertFalse(summarize(spec, rows)["candidate_review"]["candidate"]["eligible_for_owner_review"])

    def test_physiology_failure_blocks_even_perfect_success(self):
        spec, rows = self.fixture()
        next(r for r in rows if r["role"] == "candidate")["safe"] = False
        self.assertFalse(summarize(spec, rows)["candidate_review"]["candidate"]["eligible_for_owner_review"])

    def test_all_zero_parents_do_not_prove_retained_skill(self):
        spec, rows = self.fixture()
        for row in rows: row["success"] = False
        self.assertFalse(summarize(spec, rows)["candidate_review"]["candidate"]["eligible_for_owner_review"])

    def test_malformed_missing_unmatched_or_contaminated_evidence_rejected(self):
        spec, rows = self.fixture()
        for field, value in (("native_evidence_verified", False), ("matched_inputs_verified", False),
                             ("teacher_removed", False), ("prior_removed", False),
                             ("offline_optimizer_updates", 1), ("success", 1),
                             ("censored", True),
                             ("partition", "development"), ("input_match_sha256", "")):
            bad = copy.deepcopy(rows); bad[0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError): summarize(spec, bad)
        with self.assertRaises(ValueError): summarize(spec, rows[1:])
        with self.assertRaises(ValueError): summarize(spec, rows + [rows[0]])

    def test_predeclared_failed_seed_cannot_be_dropped(self):
        spec, rows = self.fixture()
        with self.assertRaises(ValueError): summarize(spec, [r for r in rows if r["seed"] != 99])

    def test_parent_tradeoff_has_no_single_winner(self):
        spec, rows = self.fixture()
        for row in rows:
            if (row["domain"], row["role"]) in (("care", "pc"), ("language", "cloud")):
                row["success"] = False
        result = summarize(spec, rows)
        self.assertEqual(result["parent_dominance"], {"pc": False, "cloud": False})

    def test_parent_dominance_is_reported_without_adoption(self):
        spec, rows = self.fixture()
        next(r for r in rows if r["role"] == "cloud")["success"] = False
        result = summarize(spec, rows)
        self.assertTrue(result["parent_dominance"]["pc"])
        self.assertFalse(result["automatic_adoption"])

    def test_development_seed_overlap_rejected(self):
        spec, rows = self.fixture(); spec["development_seeds"].append(0)
        with self.assertRaises(ValueError): summarize(spec, rows)

    def test_dropping_a_hard_condition_is_rejected(self):
        spec, rows = self.fixture()
        spec["cases"].append(dict(domain="care", seed=0, condition="hard"))
        with self.assertRaises(ValueError): summarize(spec, rows)

    def test_repeated_conditions_do_not_inflate_independent_seed_count(self):
        spec, rows = self.fixture(2)
        more = []
        for condition in range(100):
            for row in rows:
                item = dict(row, condition=str(condition)); more.append(item)
            for domain in spec["domains"]:
                for seed in range(2):
                    spec["cases"].append(dict(domain=domain, seed=seed, condition=str(condition)))
        result = summarize(spec, rows + more)
        self.assertEqual(result["domains"]["care"]["distinct_seeds"], 2)
        self.assertFalse(result["candidate_review"]["candidate"]["eligible_for_owner_review"])


if __name__ == "__main__":
    unittest.main()
