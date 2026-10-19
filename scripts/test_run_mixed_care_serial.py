"""Stdlib host fixtures: no real trainer, evaluator, GPU or historical inputs."""
import contextlib
import copy
import datetime as dt
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

import run_mixed_care_serial as mixed
import run_grab_food_night as grab


class MixedCareTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.source = self.root / "source"
        self.source.mkdir()
        self.lane = self.root / "lane"
        self.lane.mkdir()
        self.output = self.lane / "run"
        self.binary = self.root / "fake-trainer"
        self.inspector = self.root / "fake-inspector"
        for path in (self.binary, self.inspector):
            path.write_bytes(b"nonexecutable host fixture")
        self.schedule = self.root / "schedule.json"
        self.put(self.schedule, {"schema": 1, "steps": [
            {"lesson": lesson, "episodes": 1, "decisions": index + 2}
            for index, lesson in enumerate(mixed.LESSONS)]})
        self.put(self.source / "cycle.json", {
            "biological_objective_version": 2, "objective_state_reset": False,
            "policy_version": 7400, "seed": 42, "founder_seed_base": 42,
            "next_cohort_tick_captured": True, "next_cohort_optimizer_rebound": True,
            "new_asset_digest": "initial", "actor_optimizer_step": 16684, "value_optimizer_step": 19800})
        (self.source / "trained.alife-foundation").write_bytes(b"initial")
        for head, age in (("actor", 16684), ("value", 19800)):
            self.put(self.source / f"{head}-checkpoint.json", {"optimizer_step": age})
        self.history_path = self.root / "history.json"
        self.history = {"schema": 1, "kind": "mixed-care-seed-history", "complete": True,
                        "source_sha256": mixed.file_hashes(self.source), "consumed_seed_ranges": [[42, 42], [700, 707]],
                        "reserved_seed_ranges": [[1000, 1662]], "assessment_seeds": [9000],
                        "imports": [], "evidence": "SYNTHETIC fixture; no historical reconciliation"}
        self.put(self.history_path, self.history)
        self.commands = []
        self.ledger_path = self.lane / mixed.LEDGER
        self.write = mixed.seeds.write_document
        self.run_inspector = mixed.inspector_io.run_inspector
        self.stack = contextlib.ExitStack()
        self.addCleanup(self.stack.close)
        self.stack.enter_context(patch.object(mixed, "admit_run_context"))
        self.stack.enter_context(patch.object(mixed.host, "assert_lane_idle"))
        self.stack.enter_context(patch.object(mixed.host, "resource_snapshot", return_value={
            "disk_free_bytes": 100 * 2**30, "available_memory_bytes": 100 * 2**30, "idle_seconds": 3600}))
        self.stack.enter_context(patch.object(mixed.inspector_io, "run_inspector", side_effect=self.fake_inspect))
        self.stack.enter_context(patch.object(mixed.host, "run_child", side_effect=self.fake_child))

    @staticmethod
    def put(path, value):
        path.write_text(json.dumps(value))

    @staticmethod
    def get(path):
        return json.loads(path.read_text())

    def plan(self, extra=()):
        argv = ["--output", str(self.output), "--binary", str(self.binary),
                "--checkpoint-inspector", str(self.inspector), "--source", str(self.source),
                "--schedule", str(self.schedule), "--seed-history", str(self.history_path),
                "--seed", "100", "--seed-count", "6", "--cutoff",
                (dt.datetime.now(mixed.host.UTC) + dt.timedelta(hours=2)).isoformat(), *extra]
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            result = mixed.main(argv)
        self.assertEqual(result, 0)
        return self.get(self.output / "plan.json")

    def fake_inspect(self, argv, timeout, *, on_started=None):
        if on_started is not None:
            on_started(Mock(poll=Mock(return_value=0)))
        self.assertEqual(argv[1], "--validate-checkpoint")
        source = Path(argv[2])
        receipt = self.get(source / "cycle.json")
        report = {"neural_execution": False, "weights_modified": False,
                  "expected_policy_version": int(argv[-1]), "policy_version": receipt["policy_version"],
                  "founder_seed_base": receipt["founder_seed_base"],
                  "file_digests": [{"file_name": name, "sha256": mixed.host.sha256(source / name)} for name in mixed.FILES]}
        for head in ("actor", "value"):
            report[f"{head}_optimizer_step"] = self.get(source / f"{head}-checkpoint.json")["optimizer_step"]
        return subprocess.CompletedProcess(argv, 0, json.dumps(report).encode(), b"")

    def fake_child(self, argv, log, deadline, guard, environment, on_started):
        guard()
        on_started(Mock(poll=Mock(return_value=0)))
        # Reservation and pending intent must already survive on disk.
        ledger = mixed.read_ledger(self.ledger_path)
        attempt = ledger["runs"][-1]["attempts"][-1]
        self.assertEqual(attempt["status"], "pending")
        self.assertEqual(attempt["seed"], int(argv[6]))
        self.commands.append(argv)
        previous, directory = Path(argv[2]), Path(argv[3])
        decisions, seed, lesson = int(argv[4]), int(argv[6]), argv[8]
        directory.mkdir()
        old = self.get(previous / "cycle.json")
        (directory / "initial.alife-foundation").write_bytes((previous / "trained.alife-foundation").read_bytes())
        (directory / "trained.alife-foundation").write_bytes(str(seed).encode())
        setup = {"target": 3, "sampling_seed": seed, "sampling_attempts": 1,
                 "initial_sleeping": False, "initial_energy": .85, "initial_hunger": .15, "initial_health": 1.,
                 "realized_body_position": {"x": seed, "y": 0., "z": 0.}, "realized_body_yaw": .5,
                 "realized_grip_position": {"x": seed + .5, "y": 0., "z": 0.},
                 "food_position": {"x": seed + .5, "y": 0., "z": 0.}, "initial_owner": None, "initial_consumed": False}
        receipt = {"lesson": lesson, "biological_objective_version": 2, "objective_state_reset": False,
                   "seed": seed, "founder_seed_base": 42, "policy_version": old["policy_version"] + 1,
                   "old_asset_digest": old["new_asset_digest"], "new_asset_digest": str(seed),
                   "next_cohort_tick_captured": True, "next_cohort_optimizer_rebound": True,
                   "speech_target_rows": 0, "teacher_cue_frames": 1, "training_ticks": decisions,
                   "requested_training_ticks": decisions, "trained_replay_rows": decisions,
                   "captured_replay_rows": decisions + 1, "bootstrap_replay_rows": 1,
                   "prepared_held_food_opportunities": int(lesson == "eat_held_food"),
                   "prepared_reachable_food_opportunities": int(lesson == "eat_reachable_food"),
                   "held_food_consumption_events": int(lesson != "feeding"),
                   "trained_held_food_consumption_events": int(lesson == "eat_reachable_food"),
                   "consumed_events": 3 if lesson == "feeding" else 1,
                   "actor_optimizer_updates": 2, "value_optimizer_updates": 3, "completed_epochs": 2}
        if lesson != "feeding":
            receipt["held_food_setup" if lesson == "eat_held_food" else "reachable_food_setup"] = setup
        for head in ("actor", "value"):
            age = self.get(previous / f"{head}-checkpoint.json")["optimizer_step"] + receipt[f"{head}_optimizer_updates"]
            receipt[f"{head}_optimizer_step"] = age
            self.put(directory / f"{head}-checkpoint.json", {"optimizer_step": age})
        self.put(directory / "cycle.json", receipt)

    def test_plan_only_and_cross_lesson_chain_counts(self):
        plan = self.plan()
        self.assertFalse(self.ledger_path.exists())
        self.assertEqual(self.commands, [])
        result = mixed.execute(plan)
        self.assertEqual(result["accepted_episodes"], 3)
        self.assertEqual([a[-1] for a in self.commands], list(mixed.LESSONS))
        for previous, current in zip(self.commands, self.commands[1:]):
            self.assertEqual(current[2], previous[3])
        totals = result["per_lesson"]
        self.assertEqual(totals["eat_held_food"]["trained_exact_nourishing_consumption_events"], 0)
        self.assertEqual(totals["eat_held_food"]["exact_nourishing_consumption_events"], 1)
        self.assertEqual(totals["eat_reachable_food"]["trained_exact_nourishing_consumption_events"], 1)
        self.assertEqual(totals["feeding"]["generic_consumed_events"], 3)
        self.assertEqual(totals["feeding"]["exact_nourishing_consumption_events"], 0)
        for index, lesson in enumerate(mixed.LESSONS):
            self.assertEqual(totals[lesson]["requested_training_ticks"], index + 2)
            self.assertEqual(totals[lesson]["bootstrap_replay_rows"], 1)
            self.assertEqual(totals[lesson]["actor_optimizer_updates"], 2)
            self.assertEqual(totals[lesson]["value_optimizer_updates"], 3)
        self.assertFalse(result["promoted"])
        self.assertEqual(mixed.execute(plan, resume=True), result)
        self.assertEqual(len(self.commands), 3)

    def test_interrupt_unsealed_uses_fresh_seed_and_same_latest_predecessor(self):
        plan = self.plan()
        def interrupted(argv, *args, **kwargs):
            if len(self.commands) == 1:
                Path(argv[3]).mkdir()
                raise KeyboardInterrupt()
            self.fake_child(argv, *args, **kwargs)
        with patch.object(mixed.host, "run_child", side_effect=interrupted):
            with self.assertRaises(KeyboardInterrupt):
                mixed.execute(plan)
        result = mixed.execute(plan, resume=True)
        attempts = mixed.read_ledger(self.ledger_path)["runs"][0]["attempts"]
        self.assertEqual([a["seed"] for a in attempts], [100, 101, 102, 103])
        self.assertEqual([a["status"] for a in attempts], ["accepted", "abandoned", "accepted", "accepted"])
        self.assertEqual(self.commands[1][2], self.commands[0][3])
        self.assertEqual(result["accepted_episodes"], 3)
        self.put(self.output / "attempt-00001/cycle.json", {})
        with self.assertRaisesRegex(ValueError, "abandoned attempt later acquired a seal"):
            mixed.execute(plan, resume=True)
        self.assertEqual(len(self.commands), 3)

    def fault_write(self, target, nth, after=False):
        calls = 0
        def write(path, value):
            nonlocal calls
            if Path(path) == target:
                calls += 1
                if calls == nth:
                    if after:
                        self.write(path, value)
                    raise OSError("fixture publication interruption")
            self.write(path, value)
        return patch.object(mixed.seeds, "write_document", side_effect=write)

    def test_reservation_failure_never_dispatches_and_surviving_commit_resumes(self):
        plan = self.plan()
        with self.fault_write(self.ledger_path, 1):
            with self.assertRaises(OSError):
                mixed.execute(plan)
        self.assertFalse(self.ledger_path.exists())
        self.assertEqual(self.commands, [])
        with self.fault_write(self.ledger_path, 1, after=True):
            with self.assertRaises(OSError):
                mixed.execute(plan)
        self.assertEqual(self.commands, [])
        mixed.execute(plan, resume=True)
        self.assertEqual(len(self.commands), 3)

    def test_pending_publication_failure_consumes_intent_without_reusing_seed(self):
        plan = self.plan()
        with self.fault_write(self.ledger_path, 2, after=True):
            with self.assertRaises(OSError):
                mixed.execute(plan)
        self.assertEqual(self.commands, [])
        mixed.execute(plan, resume=True)
        self.assertEqual(int(self.commands[0][6]), 101)

    def test_seal_before_acceptance_commit_is_reconciled_once(self):
        plan = self.plan()
        with self.fault_write(self.ledger_path, 3):
            with self.assertRaises(OSError):
                mixed.execute(plan)
        self.assertEqual(len(self.commands), 1)
        self.assertEqual(mixed.execute(plan, resume=True)["accepted_episodes"], 3)
        self.assertEqual(len(self.commands), 3)

    def test_summary_publication_failure_does_not_recount_accepted_dose(self):
        plan = self.plan()
        with self.fault_write(self.output / "schedule.json", 2):
            with self.assertRaises(OSError):
                mixed.execute(plan)
        self.assertEqual(len(self.commands), 1)
        result = mixed.execute(plan, resume=True)
        self.assertEqual(result["accepted_episodes"], 3)
        self.assertEqual(result["per_lesson"]["eat_held_food"]["episodes"], 1)

    def test_native_admission_failure_and_pinned_binary_tampering_stop_dispatch(self):
        plan = self.plan()
        with patch.object(mixed.inspector_io, "run_inspector", side_effect=ValueError("native admission rejected")):
            with self.assertRaisesRegex(ValueError, "native admission"):
                mixed.execute(plan)
        self.assertEqual(self.commands, [])
        self.binary.write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "executable changed"):
            mixed.execute(plan, resume=True)
        self.assertEqual(self.commands, [])

    def test_accepted_receipt_and_source_tampering_fail_closed(self):
        plan = self.plan()
        mixed.execute(plan)
        receipt = self.output / "attempt-00000/cycle.json"
        original = receipt.read_bytes()
        receipt.write_bytes(original + b" ")
        with self.assertRaisesRegex(ValueError, "accepted receipt/checkpoint changed"):
            mixed.execute(plan, resume=True)
        receipt.write_bytes(original)
        (self.source / "actor-checkpoint.json").write_text("{}")
        with self.assertRaisesRegex(ValueError, "sealed source changed"):
            mixed.execute(plan, resume=True)
        self.assertEqual(len(self.commands), 3)

    def test_seal_validation_tampering_remains_pending_and_cannot_be_recounted(self):
        plan = self.plan()
        def tamper(argv, *args, **kwargs):
            self.fake_child(argv, *args, **kwargs)
            receipt_path = Path(argv[3]) / "cycle.json"
            receipt = self.get(receipt_path)
            receipt["lesson"] = "feeding"
            self.put(receipt_path, receipt)
        with patch.object(mixed.host, "run_child", side_effect=tamper):
            with self.assertRaisesRegex(ValueError, "lesson"):
                mixed.execute(plan)
        with self.assertRaisesRegex(ValueError, "lesson"):
            mixed.execute(plan, resume=True)
        self.assertEqual(mixed.read_ledger(self.ledger_path)["runs"][0]["attempts"][0]["status"], "pending")
        self.assertEqual(len(self.commands), 1)

    def test_owner_lock_and_resume_plan_collision(self):
        plan = self.plan()
        lock = self.lane / ".eat-held-food-owner.lock"
        lock.write_text("another owner")
        with self.assertRaises(FileExistsError):
            mixed.execute(plan)
        self.assertEqual(lock.read_text(), "another owner")
        lock.unlink()
        mixed.execute(plan)
        changed = copy.deepcopy(plan)
        changed["schedule"]["steps"][0]["decisions"] = 8
        with self.assertRaisesRegex(ValueError, "plan collision"):
            mixed.execute(changed, resume=True)
        self.assertEqual(len(self.commands), 3)

    def test_explicit_import_preserves_reserved_consumed_and_grab_allocation_receipts(self):
        imported = self.lane / mixed.seeds.LEDGER_NAME
        self.put(imported, {"schema": 1, "kind": "grab-food-seed-ledger", "training_seed_ranges": [],
            "assessment_seeds": [], "allocations": [{"training_seed_range": [200, 210], "assessment_seed": 9010,
                "source_sha256": self.history["source_sha256"], "output": "fixture", "history_path": "fixture",
                "history_sha256": "fixture", "reserved_utc": "fixture"}]})
        original = imported.read_bytes()
        with self.assertRaisesRegex(ValueError, "explicit exact-byte import"):
            mixed.history(self.history_path, mixed.file_hashes(self.source), 42, self.lane)
        self.history["imports"] = [{"path": str(imported), "sha256": mixed.host.sha256(imported)}]
        self.put(self.history_path, self.history)
        plan = self.plan()
        for seed in (42, 700, 707, 1000, 1662, 200, 210, 9000, 9010):
            changed = {**plan, "seed_range": [seed, seed]}
            with self.assertRaisesRegex(ValueError, "overlaps excluded"):
                mixed.reserve(changed, self.ledger_path, commit=False)
        self.assertEqual(imported.read_bytes(), original)
        self.assertFalse(self.ledger_path.exists())

    def test_history_bytes_and_missing_ledger_never_silently_reconcile(self):
        plan = self.plan()
        self.history_path.write_text(self.history_path.read_text() + " ")
        with self.assertRaisesRegex(ValueError, "history changed"):
            mixed.execute(plan)
        self.assertEqual(self.commands, [])
        with self.assertRaises(FileNotFoundError):
            mixed.execute(plan, resume=True)

    def test_legacy_launchers_reject_mixed_ledger(self):
        plan = self.plan()
        mixed.execute(plan)
        for check in (mixed.host.assert_legacy_lane, grab.assert_legacy_lane):
            with self.assertRaisesRegex(ValueError, "legacy launchers cannot reserve"):
                check(self.lane)
        for module in (mixed.host, grab):
            output = self.lane / "legacy"
            errors = io.StringIO()
            with contextlib.redirect_stderr(errors), self.assertRaises(SystemExit) as exit:
                module.main(["--output", str(output), "--source", str(self.source), "--binary", str(self.binary),
                    "--seed-history", str(self.history_path), "--cutoff", plan["cutoff"]]
                if module is grab else ["--output", str(output), "--source", str(self.source), "--binary", str(self.binary),
                    "--cutoff", plan["cutoff"]])
            self.assertEqual(exit.exception.code, 2)
            self.assertIn("mixed-care ledger present", errors.getvalue())
            self.assertFalse(output.exists())

    def test_new_allocation_must_use_latest_seal_and_older_resume_cannot_fork(self):
        plan = self.plan()
        result = mixed.execute(plan)
        stale = {**plan, "output": str(self.lane / "next"), "seed_range": [300, 305]}
        with self.assertRaisesRegex(ValueError, "latest accepted"):
            mixed.reserve(stale, self.ledger_path, commit=False)
        latest = Path(result["latest_checkpoint"])
        self.history["source_sha256"] = mixed.file_hashes(latest)
        self.history["consumed_seed_ranges"].append([100, 102])
        self.put(self.history_path, self.history)
        self.output = self.lane / "next"
        self.source = latest
        next_plan = self.plan(["--seed", "300"])
        mixed.reserve(next_plan, self.ledger_path)
        with self.assertRaisesRegex(ValueError, "newer allocation"):
            mixed.execute(plan, resume=True)

    def test_bad_schedule_and_history_contracts(self):
        for step in ({"lesson": "grab_food", "episodes": 1, "decisions": 16},
                     {"lesson": "feeding", "episodes": True, "decisions": 16},
                     {"lesson": "feeding", "episodes": 1, "decisions": 129}):
            with self.assertRaises(ValueError):
                mixed.schedule_rows({"schema": 1, "steps": [step]})
        for key, value in (("complete", False), ("consumed_seed_ranges", []), ("imports", None),
                           ("source_sha256", {name: "0" * 64 for name in mixed.FILES})):
            invalid = {**self.history, key: value}
            self.put(self.history_path, invalid)
            with self.assertRaises(ValueError):
                mixed.history(self.history_path, mixed.file_hashes(self.source), 42, self.lane)

    def test_checked_in_example_and_saved_plan_start_then_resume(self):
        example, _ = mixed.load(Path(__file__).resolve().parent.parent / "docs/examples/mixed-care-schedule.json")
        self.assertEqual([row["lesson"] for row in mixed.schedule_rows(example)], list(mixed.LESSONS))
        self.plan()
        for mode in ("--start", "--resume"):
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(mixed.main(["--output", str(self.output), "--run", mode]), 0)
        self.assertEqual(len(self.commands), 3)

    def test_optimizer_bootstrap_reset_and_predecessor_mismatches_reject(self):
        plan = self.plan()
        mixed.execute(plan)
        directory = self.output / "attempt-00000"
        receipt_path = directory / "cycle.json"
        original = self.get(receipt_path)
        row = mixed.schedule_rows(plan["schedule"])[0]
        for key, value in (("actor_optimizer_updates", 1), ("value_optimizer_updates", 2),
                           ("bootstrap_replay_rows", 0), ("requested_training_ticks", 5),
                           ("objective_state_reset", True), ("biological_objective_version", 1),
                           ("trained_held_food_consumption_events", 2), ("captured_replay_rows", True),
                           ("consumed_events", original["captured_replay_rows"] + 1),
                           ("old_asset_digest", "wrong")):
            with self.subTest(field=key):
                self.put(receipt_path, {**original, key: value})
                with self.assertRaises(ValueError):
                    mixed.validate_cycle(directory, self.source, row, 100, self.inspector)
        self.put(receipt_path, original)
        (directory / "initial.alife-foundation").write_bytes(b"wrong predecessor")
        with self.assertRaisesRegex(ValueError, "predecessor"):
            mixed.validate_cycle(directory, self.source, row, 100, self.inspector)

    def test_whole_unused_allocation_and_ledger_tampering_remain_exclusions(self):
        plan = self.plan()
        result = mixed.execute(plan)
        latest = Path(result["latest_checkpoint"])
        self.history["source_sha256"] = mixed.file_hashes(latest)
        self.history["consumed_seed_ranges"].append([100, 102])
        self.put(self.history_path, self.history)
        candidate = {**plan, "source": str(latest), "source_sha256": mixed.file_hashes(latest),
                     "source_seed": 102, "history_sha256": mixed.host.sha256(self.history_path),
                     "output": str(self.lane / "new"), "seed_range": [104, 105]}
        with self.assertRaisesRegex(ValueError, "overlaps excluded"):
            mixed.reserve(candidate, self.ledger_path, commit=False)
        ledger = self.get(self.ledger_path)
        ledger["runs"][0]["attempts"][0]["seed"] = 99
        self.put(self.ledger_path, ledger)
        with self.assertRaisesRegex(ValueError, "attempt sequence"):
            mixed.execute(plan, resume=True)

    def test_output_collision_fails_before_pending_intent_or_dispatch(self):
        plan = self.plan()
        collision = self.output / "attempt-00000"
        collision.mkdir()
        with self.assertRaisesRegex(ValueError, "output collision"):
            mixed.execute(plan)
        self.assertEqual(self.commands, [])
        self.assertEqual(mixed.read_ledger(self.ledger_path)["runs"][0]["attempts"], [])

    def test_uncertain_child_cleanup_retains_owner_lock(self):
        plan = self.plan()
        class UncertainChild:
            @staticmethod
            def poll():
                return None
        def fail_cleanup(argv, log, deadline, guard, environment, on_started):
            on_started(UncertainChild())
            raise RuntimeError("fixture child cleanup uncertain")
        with patch.object(mixed.host, "run_child", side_effect=fail_cleanup):
            with self.assertRaisesRegex(RuntimeError, "cleanup uncertain"):
                mixed.execute(plan)
        self.assertTrue((self.lane / ".eat-held-food-owner.lock").exists())
        with self.assertRaises(FileExistsError):
            mixed.execute(plan, resume=True)

    def assert_inspector_cleanup_uncertainty(self, fail_at):
        plan = self.plan()
        child = Mock(returncode=None)
        child.poll.return_value = None
        child.stdout.read.side_effect = OSError("fixture inspector read failure")
        child.kill.side_effect = OSError("fixture inspector kill failure")
        child.wait.side_effect = subprocess.TimeoutExpired("fixture inspector cleanup", 1)
        calls = 0
        def invoke(argv, timeout, *, on_started=None):
            nonlocal calls
            calls += 1
            if calls == fail_at:
                return self.run_inspector(argv, timeout, on_started=on_started)
            return self.fake_inspect(argv, timeout, on_started=on_started)
        with patch.object(mixed.inspector_io, "run_inspector", side_effect=invoke), \
                patch.object(mixed.subprocess, "Popen", return_value=child), \
                patch.object(mixed.os, "set_blocking"):
            with self.assertRaises(subprocess.TimeoutExpired):
                mixed.execute(plan)
        self.assertTrue((self.lane / ".eat-held-food-owner.lock").exists())
        self.assertEqual(calls, fail_at)
        self.assertEqual(len(self.commands), 0 if fail_at == 1 else 1)
        child.kill.assert_called_once_with()
        child.wait.assert_called_once_with(timeout=1)
        child.stdout.close.assert_called_once_with()
        child.stderr.close.assert_called_once_with()

    def test_initial_inspector_cleanup_uncertainty_retains_owner_lock(self):
        self.assert_inspector_cleanup_uncertainty(fail_at=1)

    def test_seal_inspector_cleanup_uncertainty_retains_lock_after_trainer_exit(self):
        self.assert_inspector_cleanup_uncertainty(fail_at=2)

    def test_successful_real_inspector_helper_cleanup_releases_owner_lock(self):
        plan = self.plan()
        children = []
        def spawn(argv, **kwargs):
            report = self.fake_inspect(argv, timeout=90)
            child = Mock(returncode=0)
            child.poll.return_value = 0
            child.wait.return_value = 0
            child.stdout.read.side_effect = [report.stdout, b""]
            child.stderr.read.return_value = b""
            children.append(child)
            return child
        with patch.object(mixed.inspector_io, "run_inspector", side_effect=self.run_inspector), \
                patch.object(mixed.subprocess, "Popen", side_effect=spawn), \
                patch.object(mixed.os, "set_blocking"):
            result = mixed.execute(plan)
        self.assertEqual(result["accepted_episodes"], 3)
        self.assertEqual(len(children), 7)  # Initial admission plus current/predecessor for each seal.
        self.assertFalse((self.lane / ".eat-held-food-owner.lock").exists())
        for child in children:
            child.wait.assert_called_once_with(timeout=1)
            child.stdout.close.assert_called_once_with()
            child.stderr.close.assert_called_once_with()

    def test_stop_between_resume_inspectors_prevents_every_later_child(self):
        plan = self.plan()
        mixed.execute(plan)
        calls = []
        def stop_after_first(argv, timeout, *, on_started=None):
            calls.append(argv)
            result = self.fake_inspect(argv, timeout, on_started=on_started)
            (self.output / "STOP").touch()
            return result
        with patch.object(mixed.inspector_io, "run_inspector", side_effect=stop_after_first):
            with self.assertRaisesRegex(ValueError, "STOP"):
                mixed.execute(plan, resume=True)
        self.assertEqual(len(calls), 1)
        self.assertEqual(len(self.commands), 3)
        self.assertFalse((self.lane / ".eat-held-food-owner.lock").exists())

    def test_current_inspector_failure_guard_prevents_predecessor_dispatch(self):
        plan = self.plan()
        calls = []
        def stop_after_current(argv, timeout, *, on_started=None):
            calls.append(argv)
            result = self.fake_inspect(argv, timeout, on_started=on_started)
            if len(calls) == 2:  # Initial source, then the first trainer's current seal.
                (self.output / "STOP").touch()
            return result
        with patch.object(mixed.inspector_io, "run_inspector", side_effect=stop_after_current):
            with self.assertRaisesRegex(ValueError, "STOP"):
                mixed.execute(plan)
        self.assertEqual(len(calls), 2)
        self.assertEqual(len(self.commands), 1)

    def test_each_inspector_budget_is_bounded_by_absolute_cutoff(self):
        plan = self.plan()
        cutoff = dt.datetime.now(mixed.host.UTC) + dt.timedelta(seconds=5)
        budgets = []
        def invoke(argv, timeout, *, on_started=None):
            budgets.append(timeout)
            self.assertGreater(timeout, 0)
            self.assertLessEqual(timeout, (cutoff - dt.datetime.now(mixed.host.UTC)).total_seconds() + .01)
            return self.fake_inspect(argv, timeout, on_started=on_started)
        with patch.object(mixed.inspector_io, "run_inspector", side_effect=invoke):
            mixed.execute(plan, cutoff=cutoff)
        self.assertEqual(len(budgets), 7)
        self.assertTrue(all(timeout <= 5 for timeout in budgets))

    def test_expired_cutoff_between_resume_inspectors_prevents_next_child(self):
        plan = self.plan()
        mixed.execute(plan)
        now = dt.datetime.now(mixed.host.UTC)
        cutoff = now + dt.timedelta(seconds=5)
        clock = Mock(wraps=dt.datetime)
        clock.now.return_value = now
        calls = []
        def exhaust_after_first(argv, timeout, *, on_started=None):
            calls.append(argv)
            result = self.fake_inspect(argv, timeout, on_started=on_started)
            clock.now.return_value = cutoff + dt.timedelta(seconds=1)
            return result
        with patch.object(mixed.dt, "datetime", clock), \
                patch.object(mixed.inspector_io, "run_inspector", side_effect=exhaust_after_first):
            with self.assertRaisesRegex(ValueError, "cutoff reached"):
                mixed.execute(plan, resume=True, cutoff=cutoff)
        self.assertEqual(len(calls), 1)
        self.assertEqual(len(self.commands), 3)

    def test_feeding_generic_consumption_cannot_exceed_captured_decisions(self):
        plan = self.plan()
        mixed.execute(plan)
        directory = self.output / "attempt-00002"
        receipt = self.get(directory / "cycle.json")
        receipt["consumed_events"] = receipt["captured_replay_rows"] + 1
        self.put(directory / "cycle.json", receipt)
        with self.assertRaisesRegex(ValueError, "invalid integer count"):
            mixed.validate_cycle(directory, self.output / "attempt-00001",
                                 mixed.schedule_rows(plan["schedule"])[2], 102, self.inspector)

    def test_resume_with_explicit_cutoff_can_reconcile_after_original_budget(self):
        plan = self.plan()
        plan["cutoff"] = (dt.datetime.now(mixed.host.UTC) - dt.timedelta(minutes=1)).isoformat()
        self.write(self.output / "plan.json", plan)
        mixed.reserve(plan, self.ledger_path)
        with contextlib.redirect_stdout(io.StringIO()):
            result = mixed.main(["--output", str(self.output), "--resume", "--run", "--cutoff",
                (dt.datetime.now(mixed.host.UTC) + dt.timedelta(hours=1)).isoformat()])
        self.assertEqual(result, 0)
        self.assertEqual(len(self.commands), 3)

    def test_corrected_source_actual_value_age_is_not_counted_as_new_ppo_dose(self):
        self.put(self.source / "critic-correction.json", {"fixture": "read-only already-admitted correction"})
        self.put(self.source / "value-checkpoint.json", {"optimizer_step": 19900})
        self.history["source_sha256"] = {name: mixed.file_hashes(self.source)[name] for name in mixed.FILES}
        self.put(self.history_path, self.history)
        plan = self.plan()
        result = mixed.execute(plan)
        last = self.get(Path(result["latest_checkpoint"]) / "cycle.json")
        self.assertEqual(last["value_optimizer_step"], 19909)
        self.assertEqual(sum(row["value_optimizer_updates"] for row in result["per_lesson"].values()), 9)

    def test_changed_import_bytes_and_symlinked_missing_ledger_fail_closed(self):
        imported = self.root / "grab.json"
        self.put(imported, {"schema": 1, "kind": "grab-food-seed-ledger", "training_seed_ranges": [[300, 310]],
                           "assessment_seeds": [], "allocations": []})
        self.history["imports"] = [{"path": str(imported), "sha256": mixed.host.sha256(imported)}]
        self.put(self.history_path, self.history)
        plan = self.plan()
        imported.write_text(imported.read_text() + " ")
        with self.assertRaisesRegex(ValueError, "Grab import bytes"):
            mixed.execute(plan)
        self.assertEqual(self.commands, [])
        imported.write_text(imported.read_text().rstrip())
        self.ledger_path.symlink_to(self.root / "missing-ledger")
        with self.assertRaisesRegex(ValueError, "symlink"):
            mixed.reserve(plan, self.ledger_path, commit=False)


if __name__ == "__main__":
    unittest.main()
