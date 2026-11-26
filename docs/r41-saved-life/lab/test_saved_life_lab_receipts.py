from copy import deepcopy
import unittest

import saved_life_lab_receipts as receipts

SHA = "a" * 64


def identity(name):
    return dict(base_id=name, foundation_digest=SHA, dendritic_branches_sha256=SHA,
                source_current_sha256=SHA, dendritic_branch_count=1, semantic_prior_mode="off")


def reopen_fixture():
    contract = dict(identity("TEST-BASE"), executable_sha256=SHA)
    proof = dict(contract, schema="qualia.accepted_saved_life_reopen.v1",
                 executable_sha256=SHA, receipt_sha256=SHA, gpu_restored=True,
                 same_individual=True, private_world_memory_neural_and_sampling_preserved=True,
                 source_unchanged=True, independently_accepted=True, zero_cut=True, tick_start=3074,
                 tick_end=3074, parent_pid=12, child_pid=13, child_exit=0,
                 finished_at="2026-11-26T06:00:32+00:00")
    return proof, contract


def comparison_fixture():
    contract = dict(scorecard_id="synthetic-test-only", rubric_sha256=SHA, executable_sha256=SHA,
                    candidate_patch_id="test", min_improvement=0, base=identity("TEST-BASE"),
                    candidate=identity("TEST-CANDIDATE"), cases={
                        name: dict(decisions=1, scenario_sha256=SHA, weight=1, lesson="approach_food")
                        for name in ("improve", "regress")
                    })
    proof = dict(contract, schema="qualia.accepted_saved_life_scorecard.v1", full_score=True,
                 independently_accepted=True, cases={})
    for name, base, candidate in [("improve", 20, 90), ("regress", 100, 95)]:
        def native(score):
            return dict(native_receipt_sha256=SHA, rows_sha256=SHA, native_valid=True, gpu_restored=True,
                        decisions=1, source_current_sha256=SHA, score=score, completed=True)
        proof["cases"][name] = dict(scenario_sha256=SHA, matched=True, real_game_inputs=True,
                                    same_disposable_start=True, base=native(base), candidate=native(candidate))
    return proof, contract


class ReceiptsTest(unittest.TestCase):
    def test_zero_cut_reopen_requires_separate_pids_and_pin(self):
        proof, contract = reopen_fixture()
        self.assertEqual(receipts.reopen(proof, contract)["tick"], 3074)
        for key, value in (("parent_pid", 13), ("tick_end", 3075), ("receipt_sha256", "short")):
            invalid = dict(proof, **{key: value})
            with self.assertRaises(ValueError):
                receipts.reopen(invalid, contract)

    def test_average_promotes_despite_small_regression(self):
        proof, contract = comparison_fixture()
        receipt = receipts.comparison(proof, contract)
        self.assertEqual((receipt["base_average"], receipt["candidate_average"]), (60, 92.5))
        self.assertEqual(receipt["regressed_cases"], ["regress"])
        self.assertTrue(receipt["promoted"])

    def test_partial_and_unmatched_refused(self):
        proof, contract = comparison_fixture()
        for field, replacement in (("full_score", None), ("cases", {}), ("rubric_sha256", "b" * 64)):
            with self.assertRaises(ValueError):
                receipts.comparison(dict(proof, **{field: replacement}), contract)
        proof = deepcopy(proof)
        proof["cases"]["improve"]["candidate"]["decisions"] = 2
        with self.assertRaises(ValueError):
            receipts.comparison(proof, contract)
