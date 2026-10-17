#!/usr/bin/env python3
"""CPU-only acceptance and provenance checks for saved PPO target reports."""
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

SCRIPT = Path(__file__).with_name("summarize_ppo_diagnostics.py")
SPEC = importlib.util.spec_from_file_location("ppo_summary", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
REAL_RUN = subprocess.run


class PpoDiagnosticSummaryTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name) / "cycle"
        self.directory.mkdir()
        self.cycle = {"policy_version": 42, "seed": 7, "trained_replay_rows": 1,
                      "old_asset_digest": "01" * 32, "next_cohort_tick_captured": True,
                      "next_cohort_optimizer_rebound": True, "new_asset_digest": "03" * 32,
                      "founder_seed_base": 99, "actor_optimizer_step": 12, "value_optimizer_step": 18}
        self.inspector = self.directory.parent / "inspect_archived_success.exe"
        self.inspector.write_bytes(b"mocked, never executed")
        self.inspection = {"expected_policy_version": 42, "policy_version": 42,
                           "founder_seed_base": 99, "foundation_digest": [3] * 32,
                           "actor_optimizer_step": 12, "value_optimizer_step": 18,
                           "neural_execution": False, "weights_modified": False}
        patcher = mock.patch.object(MODULE.subprocess, "run", side_effect=self.inspect)
        self.runner = patcher.start()
        self.addCleanup(patcher.stop)
        self.diagnostic = {
            "schema_version": 1, "phase": "prepared_before_optimizer_update",
            "training_commit_requires_cycle_receipt": True, "policy_version": 42, "seed": 7,
            "sampling_temperature": 32.0, "discount_half_life_seconds": 60.0,
            "gae_half_life_seconds": 10.0,
            "total_rows": 1, "emitted_rows": 1, "rows_truncated": False,
            "source": {"source_revision": "a" * 40, "policy_version": 42,
                       "actor_checkpoint_digest": [2] * 32, "foundation_asset_digest": [1] * 32,
                       "phenotype_hash": [3] * 4, "compiler_inputs_digest": [4] * 4},
            "bootstrap_decision_tick": 11, "bootstrap_value": 0.5, "terminal_death_tick": None,
            "rows": [{"row": 0, "step": 0, "trajectory_id": 7, "decision_tick": 10,
                      "reward": 0.25, "old_value": 2.0, "next_value": 0.5,
                      "elapsed_seconds": 0.05, "gamma": 0.999, "lambda": 0.99,
                      "boundary": "truncated", "advantage": -1.25, "return": 0.75}]}
        for name in MODULE.ARTIFACTS:
            (self.directory / name).write_bytes(b"fixture")
        self.save()

    def save(self):
        for name, value in (("cycle.json", self.cycle), (MODULE.SIDECAR, self.diagnostic)):
            (self.directory / name).write_text(json.dumps(value), encoding="utf-8")

    def report(self):
        return MODULE.summarize([self.directory], self.inspector)

    def inspect(self, argv, **kwargs):
        self.assertEqual(argv[1:], ["--validate-checkpoint", argv[2], "--expected-policy", "42"])
        self.assertEqual(kwargs["timeout"], MODULE.INSPECTOR_TIMEOUT)
        self.assertFalse(kwargs["check"])
        directory = Path(argv[2])
        if any((directory / name).read_bytes() != b"fixture" for name in MODULE.ARTIFACTS):
            return subprocess.CompletedProcess(argv, 1, b"", b"native handoff rejected")
        return subprocess.CompletedProcess(argv, 0, json.dumps(self.inspection).encode(), b"")

    def test_sealed_exact_targets_preserve_negative_advantage_despite_positive_reward(self):
        before = {path.name: path.read_bytes() for path in self.directory.iterdir()}
        report = MODULE.summarize([self.directory, self.directory], self.inspector)
        self.assertEqual(report["errors"], [])
        self.assertEqual(report["committed_diagnostic_cycles"], 1)
        self.assertEqual(report["emitted_rows"], 1)
        self.assertEqual(report["reward_sum"], 0.25)
        self.assertEqual(report["advantage_sum"], -1.25)
        self.assertEqual(report["positive_reward_negative_advantage_rows"], 1)
        self.assertEqual(self.runner.call_count, 1)
        self.assertEqual(before, {path.name: path.read_bytes() for path in self.directory.iterdir()})

    def test_prepared_attempt_without_cycle_is_excluded(self):
        (self.directory / "cycle.json").unlink()
        report = MODULE.summarize([self.directory])
        self.assertEqual(report["errors"], [])
        self.assertEqual(report["committed_diagnostic_cycles"], 0)
        self.assertEqual(report["emitted_rows"], 0)
        self.assertEqual(report["excluded_unsealed"], [str(self.directory)])
        self.runner.assert_not_called()

    def test_copied_diagnostics_count_once_and_conflicting_copy_is_an_error(self):
        copied = self.directory.parent / "copied"
        shutil.copytree(self.directory, copied)
        report = MODULE.summarize([self.directory, copied], self.inspector)
        self.assertEqual(report["errors"], [])
        self.assertEqual(report["committed_diagnostic_cycles"], 1)
        self.assertEqual(report["advantage_sum"], -1.25)
        self.assertEqual(len(report["duplicate_aliases"]), 1)
        alias = report["duplicate_aliases"][0]
        self.assertEqual({alias["directory"], alias["same_as"]}, {str(self.directory), str(copied)})
        self.diagnostic["rows"][0]["advantage"] = -2.0
        self.save()
        report = MODULE.summarize([self.directory, copied], self.inspector)
        self.assertEqual(len(report["errors"]), 1)
        self.assertIn("conflicting diagnostics", report["errors"][0]["error"])
        self.assertEqual(report["committed_diagnostic_cycles"], 1)
        self.assertEqual(report["emitted_rows"], 1)

    def test_missing_zero_negative_or_nonfinite_configuration_is_an_error(self):
        original = copy.deepcopy(self.diagnostic)
        for key in ("sampling_temperature", "discount_half_life_seconds", "gae_half_life_seconds"):
            for invalid in (None, 0, -1, float("inf")):
                with self.subTest(key=key, invalid=invalid):
                    self.diagnostic = copy.deepcopy(original)
                    if invalid is None:
                        self.diagnostic.pop(key)
                    else:
                        self.diagnostic[key] = invalid
                    self.save()
                    report = self.report()
                    self.assertEqual(len(report["errors"]), 1)
                    self.assertEqual(report["emitted_rows"], 0)

    def test_incomplete_handoff_or_missing_artifact_is_excluded(self):
        for flag in ("next_cohort_tick_captured", "next_cohort_optimizer_rebound"):
            with self.subTest(flag=flag):
                self.cycle[flag] = False
                self.save()
                self.assertEqual(len(self.report()["excluded_unsealed"]), 1)
                self.cycle[flag] = True
        self.save()
        for name in MODULE.ARTIFACTS:
            path = self.directory / name
            path.unlink()
            self.assertEqual(self.report()["emitted_rows"], 0)
            path.write_bytes(b"fixture")

    def test_old_sealed_cycle_without_sidecar_is_missing_not_zero_advantage(self):
        (self.directory / MODULE.SIDECAR).unlink()
        report = MODULE.summarize([self.directory])
        self.assertEqual(report["missing_diagnostics"], [str(self.directory)])
        self.assertEqual(report["committed_diagnostic_cycles"], 0)
        self.runner.assert_not_called()

    def test_identity_and_row_count_mismatches_are_errors(self):
        original = copy.deepcopy(self.diagnostic)
        mutations = [lambda d: d.update(policy_version=43), lambda d: d.update(seed=8),
                     lambda d: d["source"].update(policy_version=43),
                     lambda d: d["source"].update(foundation_asset_digest=[8] * 32),
                     lambda d: d["rows"][0].update(trajectory_id=9),
                     lambda d: d["rows"][0].update(row=1),
                     lambda d: d["rows"][0].update(step=1),
                     lambda d: d.update(total_rows=2, rows_truncated=True)]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                self.diagnostic = copy.deepcopy(original)
                mutate(self.diagnostic)
                self.save()
                report = self.report()
                self.assertEqual(len(report["errors"]), 1)
                self.assertEqual(report["emitted_rows"], 0)

    def test_truncated_view_does_not_reconstruct_omitted_advantages(self):
        self.cycle["trained_replay_rows"] = 100
        first = self.diagnostic["rows"][0]
        rows = [copy.deepcopy(first) for _ in range(64)]
        for index, row in enumerate(rows):
            row.update(row=index, step=index, decision_tick=10 + index, boundary="continuing")
            if index:
                row.update(reward=0, advantage=0)
        self.diagnostic.update(total_rows=100, emitted_rows=64, rows_truncated=True,
                               rows=rows, bootstrap_decision_tick=110)
        self.save()
        report = self.report()
        self.assertEqual(report["errors"], [])
        self.assertEqual(report["emitted_rows"], 64)
        self.assertEqual(report["omitted_rows_unknown"], 36)
        self.assertEqual(report["truncated_diagnostic_cycles"], 1)
        self.assertEqual(report["advantage_sum"], -1.25)
        self.assertIn("not_assessed", report["competence"])

    def test_terminal_rollout_requires_zero_next_value_and_no_bootstrap(self):
        self.cycle["terminal_death_tick"] = 11
        self.diagnostic.update(terminal_death_tick=11, bootstrap_decision_tick=None,
                               bootstrap_value=None)
        self.diagnostic["rows"][0].update(boundary="terminated", next_value=0, gamma=0, **{"lambda": 0})
        self.save()
        self.assertEqual(self.report()["errors"], [])
        self.diagnostic["rows"][0]["next_value"] = 0.5
        self.save()
        self.assertEqual(len(self.report()["errors"]), 1)

    def test_malformed_nonfinite_oversized_and_overlong_inputs_are_errors(self):
        path = self.directory / MODULE.SIDECAR
        original = json.dumps(self.diagnostic)
        for text in ("{", "{}", original.replace("-1.25", "NaN"),
                     original.replace("-1.25", "1e999"),
                     original.replace('"schema_version": 1', '"schema_version": 1, "schema_version": 1'),
                     " " * (MODULE.MAX_BYTES + 1)):
            with self.subTest(text=text[:25]):
                path.write_text(text, encoding="utf-8")
                self.assertEqual(len(self.report()["errors"]), 1)
                self.assertEqual(self.report()["emitted_rows"], 0)
        self.diagnostic.update(emitted_rows=65, total_rows=65)
        self.save()
        self.assertEqual(len(self.report()["errors"]), 1)

    def test_cli_errors_have_nonzero_exit_and_json_output(self):
        self.diagnostic["rows"][0]["advantage"] = float("inf")
        self.save()
        result = REAL_RUN([sys.executable, str(SCRIPT), str(self.directory)],
                          capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(len(json.loads(result.stdout)["errors"]), 1)

    def test_missing_inspector_is_required_only_for_potentially_committed_diagnostics(self):
        for inspector in (None, self.inspector.with_name("missing")):
            report = MODULE.summarize([self.directory], inspector)
            self.assertEqual(report["emitted_rows"], 0)
            self.assertIn("--inspector", report["errors"][0]["error"])
        self.runner.assert_not_called()

    def test_inspector_timeout_failure_and_malformed_output_fail_closed(self):
        outcomes = [subprocess.TimeoutExpired("inspector", 30), OSError("cannot execute"),
                    subprocess.CompletedProcess([], 1, b"{}", b"failed"),
                    subprocess.CompletedProcess([], 0, b"not json", b""),
                    subprocess.CompletedProcess([], 0, b"{}", b""),
                    subprocess.CompletedProcess([], 0, b" " * 65537, b"")]
        for outcome in outcomes:
            with self.subTest(outcome=outcome):
                self.runner.side_effect = (outcome if isinstance(outcome, Exception) else
                                           lambda *args, result=outcome, **kwargs: result)
                report = self.report()
                self.assertEqual(len(report["errors"]), 1)
                self.assertEqual(report["emitted_rows"], 0)

    def test_inspector_receipt_must_match_sealed_cycle_and_read_only_contract(self):
        original = copy.deepcopy(self.inspection)
        for key, invalid in (("expected_policy_version", 43), ("policy_version", 43),
                             ("founder_seed_base", 100), ("actor_optimizer_step", 13),
                             ("value_optimizer_step", 19), ("foundation_digest", [4] * 32),
                             ("neural_execution", True), ("weights_modified", True)):
            with self.subTest(key=key):
                self.inspection = dict(original, **{key: invalid})
                report = self.report()
                self.assertEqual(len(report["errors"]), 1)
                self.assertEqual(report["emitted_rows"], 0)

    def test_stale_truncated_or_swapped_artifact_requires_native_rejection(self):
        for name, content in zip(MODULE.ARTIFACTS, (b"stale actor", b"", b"swapped asset")):
            with self.subTest(name=name):
                path = self.directory / name
                path.write_bytes(content)
                self.runner.reset_mock()
                report = self.report()
                self.runner.assert_called_once()
                self.assertIn("inspector failed", report["errors"][0]["error"])
                self.assertEqual(report["emitted_rows"], 0)
                path.write_bytes(b"fixture")

    def test_mutation_of_any_bound_input_during_native_call_is_rejected(self):
        for name in MODULE.FILE_LIMITS:
            with self.subTest(name=name):
                path, previous = self.directory / name, (self.directory / name).read_bytes()
                def mutate(argv, **kwargs):
                    result = self.inspect(argv, **kwargs)
                    path.write_bytes(previous + b" ")
                    return result
                self.runner.side_effect = mutate
                report = self.report()
                self.assertIn("changed during inspection", report["errors"][0]["error"])
                self.assertEqual(report["emitted_rows"], 0)
                path.write_bytes(previous)

    def test_cycle_parsed_bytes_and_artifact_limits_are_checked_before_native_call(self):
        original_snapshot = MODULE.snapshot
        def mutate(directory):
            path = directory / "cycle.json"
            path.write_bytes(path.read_bytes() + b" ")
            return original_snapshot(directory)
        with mock.patch.object(MODULE, "snapshot", side_effect=mutate):
            report = self.report()
        self.assertIn("changed after JSON parsing", report["errors"][0]["error"])
        with mock.patch.dict(MODULE.FILE_LIMITS, {MODULE.ARTIFACTS[0]: 1}):
            report = self.report()
        self.assertIn("oversized", report["errors"][0]["error"])
        self.runner.assert_not_called()


if __name__ == "__main__":
    unittest.main()
