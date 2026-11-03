"""Synthetic evidence tests; the separate pilot uses actual native executables."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import subprocess
import sys

import care_frontier_sampler as sampler
from run_care_frontier_fixture import Pilot


def bank():
    return {"schema": 1, "decision_budget": 2, "slots": 3, "sampler_seed": 7, "current_difficulty": 1,
            "families": [{"id": name, "lesson": lesson, "food_need_condition": "newborn_v1", "placement": "native_seeded_v1", "difficulty": i}
                         for i, (name, lesson) in enumerate((("easy", "eat_held_food"), ("edge", "eat_reachable_food"), ("hard", "feeding")))],
            "pool": [{"family": name, "seed": 1000+100*i+j}
                     for i, name in enumerate(("easy", "edge", "hard")) for j in range(5)],
            "holdouts": [{"family": "edge", "seed": 9000}]}


def observations():
    rows = []
    for i, (lesson, outcomes) in enumerate((("eat_held_food", [1, 1, 1, 1]),
                                           ("eat_reachable_food", [1, 0, 1, 0]), ("feeding", [0, 0, 0, 0]))):
        for j, success in enumerate(outcomes):
            rows.append({"lesson": lesson, "food_need_condition": "newborn_v1", "seed": 100+10*i+j,
                         "decisions": 2, "temperature": 1.0, "actor_sha256": "a"*64, "value_sha256": "b"*64,
                         "source_sha256": {"actor-checkpoint.json": "c"*64}, "policy_version": 1,
                         "nourished_meals": success, "survived_evaluated_window": True,
                         "success": bool(success), "receipt_sha256": "d"*64})
    return rows


def fixture(root, mutate=None):
    rows = []
    for i in range(3):
        before = {"body": {"health": 1.0, "energy": .5, "injury": 0.0}, "homeostasis": {"drives": {"hunger": .2}}}
        after = copy.deepcopy(before)
        if i == 0:
            after["body"]["energy"] = .6
        rows.append({"row": i, "loss_row": False, "teacher_heard": False, "evaluation_row": i < 2,
                     "bootstrap_row": i >= 2, "organism": 1, "decision_tick": i, "outcome_tick": i+1,
                     "outcome": {"measured_physiology": {"before": before, "after": after}}, "nourished_meal": i == 0})
    trace = {"backend": "native-cpu", "lesson": "feeding", "rows": rows}
    request = {"previous": "synthetic-source", "output": str(root), "seed_history": "synthetic-history.json",
               "decisions": 2, "seed": 1234, "lesson": "feeding", "temperature": 1.0, "known_exclusions": False}
    result = {"mode": "native-cpu-care-assessment", "backend": "native-cpu", "lesson": "feeding", "seed": 1234,
              "evaluated_decisions": 2, "bootstrap_decisions": 1, "teacher_present": False, "teacher_cue_frames": 0,
              "semantic_prior_enabled": False, "offline_learning_disabled": True,
              "offline_actor_optimizer_updates": 0, "offline_value_optimizer_updates": 0,
              "actor_checkpoint_unchanged": True, "value_checkpoint_unchanged": True, "promotion": "none",
              "actor_checkpoint_before_sha256": "a"*64, "actor_checkpoint_after_sha256": "a"*64,
              "value_checkpoint_before_sha256": "b"*64, "value_checkpoint_after_sha256": "b"*64,
              "evaluated_nourished_meals": 1, "source_sha256": {}, "input_sha256": {}, "policy_version": 1}
    if mutate:
        mutate(trace, request, result)
    path = root / "care-trace.json"
    path.write_text(json.dumps(trace))
    receipt = {"schema_version": 1, "request": request, "result": result, "input_sha256": {},
               "artifacts": {"care-trace.json": hashlib.sha256(path.read_bytes()).hexdigest()}}
    receipt["payload_sha256"] = sampler.native_receipt_digest(receipt)
    (root / "care-evaluation.json").write_text(json.dumps(receipt))


class FrontierTests(unittest.TestCase):
    def test_selects_empirically_feasible_frontier_and_keeps_fixed_budget_and_holdouts(self):
        b, rows = bank(), observations()
        original = copy.deepcopy((b, rows))
        chosen = sampler.select(b, rows)
        self.assertEqual(chosen, sampler.select(b, rows))
        self.assertEqual(chosen["status"], "ready")
        self.assertEqual([t["family"] for t in chosen["baseline"]], ["easy", "edge", "hard"])
        self.assertTrue(all(t["family"] == "edge" for t in chosen["adaptive"]))
        self.assertEqual(len({t["seed"] for t in chosen["adaptive"]}), 3)
        self.assertEqual(chosen["requested_training_decisions_per_arm"], 6)
        self.assertFalse(chosen["solver_rewards_changed"])
        self.assertEqual(chosen["statistics"]["hard"]["status"], "no_observed_success")
        self.assertEqual(chosen["statistics"]["easy"]["status"], "observed_trivial")
        self.assertEqual((b, rows), original)

    def test_no_empirical_frontier_is_negative_and_never_fabricates_dispatch(self):
        rows = observations()
        for row in rows:
            row["success"] = False
        result = sampler.select(bank(), rows)
        self.assertEqual(result["status"], "no_sufficient_empirical_frontier")
        self.assertEqual(result["adaptive"], [])
        self.assertEqual(len(result["baseline"]), 3)

    def test_bounded_batch_difficulty_up_down_and_retention_mix(self):
        self.assertEqual(sampler.adjust_difficulty(1, 4, 4, 2)[0], 2)
        self.assertEqual(sampler.adjust_difficulty(1, 0, 4, 2)[0], 0)
        self.assertEqual(sampler.adjust_difficulty(1, 2, 4, 2)[0], 1)
        self.assertEqual(sampler.adjust_difficulty(0, 0, 4, 2)[0], 0)
        self.assertEqual(sampler.adjust_difficulty(2, 4, 4, 2)[0], 2)
        self.assertEqual(sampler.adjust_difficulty(1, 0, 2, 2)[0], 1)
        b = bank(); b["slots"] = 4
        result = sampler.select(b, observations())
        self.assertEqual(result["retention_slots_filled"], 1)
        self.assertEqual(result["adaptive"][0]["family"], "easy")
        self.assertEqual([t["family"] for t in result["adaptive"]][1:], ["edge"]*3)

    def test_rejects_custom_placements_reward_changes_seed_leakage_and_identity_or_dose_drift(self):
        mutations = [lambda b: b["families"][0].update(placement=[999, 999]),
                     lambda b: b.update(reward_override=1), lambda b: b["pool"][0].update(seed=True),
                     lambda b: b.update(schema=True), lambda b: b.update(current_difficulty=True),
                     lambda b: b["holdouts"][0].update(seed=1000),
                     lambda b: b["pool"][0].update(seed=100),
                     lambda b: b["families"][0].update(food_need_condition="ordinary_healthy_aging_v1")]
        for mutation in mutations:
            b = bank(); mutation(b)
            with self.assertRaises(ValueError):
                sampler.select(b, observations())
        for field, value in (("actor_sha256", "wrong"), ("value_sha256", "wrong"),
                             ("decisions", 3), ("temperature", .9), ("policy_version", 2), ("seed", 9000)):
            rows = observations(); rows[1][field] = value
            with self.assertRaises(ValueError):
                sampler.select(bank(), rows)

    def test_native_score_excludes_bootstrap_and_binds_the_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); fixture(root)
            score = sampler.score_assessment(root)
            self.assertEqual(score["nourished_meals"], 1)
            self.assertTrue(score["success"])
            self.assertTrue(score["survived_evaluated_window"])
            (root / "care-trace.json").write_text("changed")
            with self.assertRaisesRegex(ValueError, "archive changed"):
                sampler.score_assessment(root)

    def test_receipt_only_edits_reject_without_changing_artifacts(self):
        for change in (lambda r: r["request"].update(food_need_condition="ordinary_healthy_aging_v1"),
                       lambda r: r["result"].update(actor_checkpoint_before_sha256="forged"),
                       lambda r: r["result"].update(semantic_prior_enabled=True)):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); fixture(root)
                path = root / "care-evaluation.json"
                receipt = json.loads(path.read_text()); change(receipt); path.write_text(json.dumps(receipt))
                with self.assertRaisesRegex(ValueError, "seal mismatch"):
                    sampler.score_assessment(root)

    def test_native_score_refuses_cues_prior_updates_censoring_and_unmeasured_success(self):
        mutations = [lambda t, q, r: r.update(teacher_present=True),
                     lambda t, q, r: r.update(semantic_prior_enabled=True),
                     lambda t, q, r: r.update(offline_actor_optimizer_updates=1),
                     lambda t, q, r: r.update(actor_checkpoint_after_sha256="changed"),
                     lambda t, q, r: t["rows"].pop(),
                     lambda t, q, r: t["rows"][0].update(teacher_heard=True),
                     lambda t, q, r: t.update(recovery_prefix=True),
                     lambda t, q, r: t.update(complete=False),
                     lambda t, q, r: t["rows"][2].update(evaluation_row=True),
                     lambda t, q, r: t["rows"][0]["outcome"]["measured_physiology"]["after"]["body"].update(energy=.5)]
        for mutation in mutations:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); fixture(root, mutation)
                with self.assertRaises((ValueError, KeyError)):
                    sampler.score_assessment(root)

    def test_fixture_runner_cannot_resume_or_accept_a_retained_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(ValueError, "fresh disposable"):
                Pilot(Path(tmp), Path(__file__), Path(__file__), 2, 3, 1)

    def test_standalone_bounds_before_io_and_never_verifies_another_output(self):
        script = Path(__file__).with_name("care_frontier_sampler.py")
        args = [sys.executable, "-B", str(script), "--bank", "missing-bank", "--native-evaluator", "missing-binary"]
        args.extend([item for _ in range(1025) for item in ("--assessment", "never-open")])
        result = subprocess.run(args, capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"bounded discovery evidence required before opening", result.stderr)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); fixture(root)
            path = root / "care-evaluation.json"
            receipt = json.loads(path.read_text())
            receipt["request"]["output"] = str(root.parent)
            receipt["payload_sha256"] = sampler.native_receipt_digest(receipt)
            path.write_text(json.dumps(receipt))
            with self.assertRaisesRegex(ValueError, "differs from selected archive"):
                sampler.revalidate_completed(root, Path("missing-binary"))


if __name__ == "__main__":
    unittest.main()
