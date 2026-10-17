#!/usr/bin/env python3
"""CPU-only acceptance and provenance checks for saved PPO target reports."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import time
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
        patcher = mock.patch.object(MODULE, "run_inspector", side_effect=self.inspect)
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
        diagnostic_bytes = json.dumps(self.diagnostic).encode()
        self.cycle["ppo_rollout_diagnostics_sha256"] = hashlib.sha256(diagnostic_bytes).hexdigest()
        for name, value in (("cycle.json", self.cycle), (MODULE.SIDECAR, self.diagnostic)):
            (self.directory / name).write_text(json.dumps(value), encoding="utf-8")

    def report(self):
        return MODULE.summarize([self.directory], self.inspector)

    def inspect(self, argv, **kwargs):
        self.assertEqual(argv[1:], ["--validate-checkpoint", argv[2], "--expected-policy", "42"])
        self.assertEqual(kwargs["timeout"], MODULE.INSPECTOR_TIMEOUT)
        directory = Path(argv[2])
        if any((directory / name).read_bytes() != b"fixture" for name in MODULE.ARTIFACTS):
            return subprocess.CompletedProcess(argv, 1, b"", b"native handoff rejected")
        inspection = dict(self.inspection)
        inspection.setdefault("file_digests", self.file_digests(directory))
        return subprocess.CompletedProcess(argv, 0, json.dumps(inspection).encode(), b"")

    def file_digests(self, directory):
        return [{"file_name": name, "encoded_bytes": len((directory / name).read_bytes()),
                 "sha256": hashlib.sha256((directory / name).read_bytes()).hexdigest()}
                for name in ("cycle.json", *MODULE.ARTIFACTS)]

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

    def test_edited_or_copied_schema_valid_sidecar_is_not_bound_to_sealed_cycle(self):
        original = copy.deepcopy(self.diagnostic)
        for kind in ("edited", "copied"):
            with self.subTest(kind=kind):
                changed = copy.deepcopy(original)
                if kind == "edited":
                    changed["rows"][0]["advantage"] = -0.75
                else:
                    changed["source"]["source_revision"] = "b" * 40
                MODULE.validate(changed, self.cycle)
                (self.directory / MODULE.SIDECAR).write_text(json.dumps(changed), encoding="utf-8")
                report = self.report()
                self.assertIn("diagnostic SHA256", report["errors"][0]["error"])
                self.assertEqual(report["emitted_rows"], 0)
        self.runner.assert_not_called()

    def test_missing_sidecar_binding_or_legacy_native_sha_is_rejected(self):
        self.cycle.pop("ppo_rollout_diagnostics_sha256")
        (self.directory / "cycle.json").write_text(json.dumps(self.cycle), encoding="utf-8")
        self.assertIn("missing binding", self.report()["errors"][0]["error"])
        self.save()
        files = self.file_digests(self.directory)
        for entry in files:
            entry.pop("sha256")
        self.inspection["file_digests"] = files
        report = self.report()
        self.assertEqual(report["emitted_rows"], 0)
        self.assertIn("file digest mismatch", report["errors"][0]["error"])

    def test_transient_restored_cycle_bytes_cannot_satisfy_native_binding(self):
        path = self.directory / "cycle.json"
        original = path.read_bytes()
        def transient_swap(argv, **kwargs):
            path.write_bytes(original + b" ")
            result = self.inspect(argv, **kwargs)
            path.write_bytes(original)
            return result
        self.runner.side_effect = transient_swap
        report = self.report()
        self.assertEqual(path.read_bytes(), original)
        self.assertEqual(report["emitted_rows"], 0)
        self.assertIn("file digest mismatch: cycle.json", report["errors"][0]["error"])

    def test_native_file_digest_entries_are_complete_unique_and_exact(self):
        files = self.file_digests(self.directory)
        invalid_lists = [None, files[:-1], files + [files[0]], [files[0]] * 4]
        for field, value in (("file_name", "extra.json"), ("encoded_bytes", 0),
                             ("encoded_bytes", True), ("sha256", "ff" * 32)):
            changed = copy.deepcopy(files)
            changed[0][field] = value
            invalid_lists.append(changed)
        for entries in invalid_lists:
            with self.subTest(entries=entries):
                self.inspection["file_digests"] = entries
                report = self.report()
                self.assertEqual(len(report["errors"]), 1)
                self.assertEqual(report["emitted_rows"], 0)


class BoundedInspectorRunnerTests(unittest.TestCase):
    def run_child(self, code, timeout=5):
        children, launch = [], subprocess.Popen
        def record_child(*args, **kwargs):
            self.assertEqual(kwargs["stdin"], subprocess.DEVNULL)
            child = launch(*args, **kwargs)
            children.append(child)
            return child
        with mock.patch.object(MODULE.subprocess, "Popen", side_effect=record_child):
            try:
                return MODULE.run_inspector([sys.executable, "-c", code], timeout=timeout)
            finally:
                self.assertEqual(len(children), 1)
                child = children[0]
                self.assertIsNotNone(child.poll())
                self.assertTrue(child.stdout.closed and child.stderr.closed)
                self.assertFalse(any(thread.name.startswith(f"ppo-inspector-{child.pid}-")
                                     for thread in threading.enumerate()))

    def test_both_streams_at_cap_complete_without_deadlock(self):
        result = self.run_child("import os; os.write(1,b'x'*65536); os.write(2,b'y'*65536)")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"x" * MODULE.OUTPUT_LIMIT)
        self.assertEqual(result.stderr, b"y" * MODULE.OUTPUT_LIMIT)

    def test_each_overflow_kills_child_promptly_and_keeps_only_one_extra_byte(self):
        for stream in (1, 2):
            with self.subTest(stream=stream):
                started = time.monotonic()
                result = self.run_child(
                    f"import os,time; os.write({stream},b'x'*66560); time.sleep(30)", timeout=10)
                self.assertLess(time.monotonic() - started, 5)
                self.assertNotEqual(result.returncode, 0)
                captured = result.stdout if stream == 1 else result.stderr
                self.assertEqual(len(captured), MODULE.OUTPUT_LIMIT + 1)
                self.assertLessEqual(len(result.stdout), MODULE.OUTPUT_LIMIT + 1)
                self.assertLessEqual(len(result.stderr), MODULE.OUTPUT_LIMIT + 1)

    def test_timeout_reaps_child_closes_pipes_and_joins_readers(self):
        started = time.monotonic()
        with self.assertRaises(subprocess.TimeoutExpired) as raised:
            self.run_child("import time; print('ready',flush=True); time.sleep(30)", timeout=0.25)
        self.assertLess(time.monotonic() - started, 5)
        self.assertLessEqual(len(raised.exception.output), MODULE.OUTPUT_LIMIT + 1)
        self.assertLessEqual(len(raised.exception.stderr), MODULE.OUTPUT_LIMIT + 1)

    def test_exited_parent_with_inherited_open_pipes_still_obeys_deadline(self):
        # Model an exited direct child whose descendant keeps both writers open.
        child = mock.Mock(returncode=0)
        child.poll.return_value = 0
        child.wait.return_value = 0
        child.stdout.read.return_value = None
        child.stderr.read.return_value = None
        started = time.monotonic()
        with mock.patch.object(MODULE.subprocess, "Popen", return_value=child), \
                mock.patch.object(MODULE.os, "set_blocking") as nonblocking:
            with self.assertRaises(subprocess.TimeoutExpired):
                MODULE.run_inspector(["exited-parent"], timeout=0.03)
        self.assertLess(time.monotonic() - started, 1)
        self.assertEqual(nonblocking.call_count, 2)
        child.kill.assert_not_called()
        child.wait.assert_called_once_with(timeout=1)
        child.stdout.close.assert_called_once()
        child.stderr.close.assert_called_once()


if __name__ == "__main__":
    unittest.main()
