#!/usr/bin/env python3
"""CPU receipt/admission fixtures. Never execute a training binary or provider."""
import contextlib
import datetime as dt
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location("grab_food_night", Path(__file__).with_name("run_grab_food_night.py"))
night = importlib.util.module_from_spec(spec)
spec.loader.exec_module(night)

def setup():
    return {"target": 3, "organism": 1, "setup_tick": 100, "sampling_seed": 10,
            "sampling_attempts": 1, "realized_body_position": {"x": 0., "y": 0., "z": 0.},
            "realized_body_yaw": 0., "food_position": {"x": .5, "y": 0., "z": 0.},
            "initial_owner": None, "initial_consumed": False, "initial_sleeping": False,
            "initial_energy": .85, "initial_hunger": .15, "initial_health": 1.}

def acquisition(row):
    return {"row": row, "organism": 1, "target": 3, "sequence_id": row + 1,
            "decision_tick": 101 + row, "outcome_tick": 102 + row,
            "owner_before": None, "owner_after": 1, "consumed_after": False,
            "selected_command": {"channel": "Manipulation", "primitive": 211,
                                 "target": {"entity": 3, "position": None}},
            "physical": {"contact": "Touch", "target_entity": 3}}

class GrabNightTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "source"
        self.source.mkdir()
        self.write_cycle(self.source, 602, 1844, 1204)

    def write_cycle(self, directory, policy, actor_age, value_age, extra=None):
        directory.mkdir(exist_ok=True)
        receipt = {"founder_seed_base": 539363617, "seed": 202610061275,
                   "biological_objective_version": 2, "objective_state_reset": False,
                   "next_cohort_tick_captured": True, "next_cohort_optimizer_rebound": True,
                   "policy_version": policy, "actor_optimizer_step": actor_age,
                   "value_optimizer_step": value_age, "lesson": "eat_held_food"}
        receipt.update(extra or {})
        (directory / "cycle.json").write_text(json.dumps(receipt))
        (directory / "trained.alife-foundation").write_bytes(b"nonexecutable synthetic asset fixture")
        (directory / "actor-checkpoint.json").write_text(json.dumps({"optimizer_step": actor_age}))
        (directory / "value-checkpoint.json").write_text(json.dumps({"optimizer_step": value_age}))
        return receipt

    def test_explicit_off_survives_the_night_child_environment(self):
        with patch.dict(night.os.environ, {"ALIFE_SLM_PRIOR": "off"}):
            self.assertEqual(night.prior_environment()["ALIFE_SLM_PRIOR"], "off")
        with patch.dict(night.os.environ, {"ALIFE_SLM_PRIOR_BACKEND": "recorded"}):
            self.assertEqual(night.prior_environment()["ALIFE_SLM_PRIOR_BACKEND"], "recorded")

    def test_all_optional_prior_failures_warn_and_continue(self):
        for prior in (None, {}, "invalid receipt", {"decision_inputs": None}, {"decision_inputs": ["invalid row"]}, {"failures": 1}, {"prime_timeouts": 1},
                      {"decision_frames_without_prior": 3}, {"provider_failures": [{"error": "parse failed"}]}):
            diagnostics = night.assert_prior({"semantic_prior": prior}, 16)
            self.assertEqual(diagnostics["failure_policy"], "log_and_continue_without_prior")
            self.assertTrue(diagnostics["warnings"])

    def test_only_complete_objective2_cycle_heads_are_eligible(self):
        founder, hashes = night.source_info(self.source)
        self.assertEqual(founder, 539363617); self.assertEqual(len(hashes), 4)
        receipt = night.read_json(self.source / "cycle.json")
        for change in ({"biological_objective_version": 1}, {"objective_state_reset": True},
                       {"next_cohort_optimizer_rebound": False}):
            (self.source / "cycle.json").write_text(json.dumps({**receipt, **change}))
            with self.assertRaises(ValueError):
                night.source_info(self.source)
        (self.source / "cycle.json").write_text(json.dumps(receipt))
        (self.source / "adaptation.json").write_text("{}")
        with self.assertRaisesRegex(ValueError, "existing sealed"):
            night.source_info(self.source)

    def test_checkpoint_age_mismatch_is_rejected(self):
        (self.source / "value-checkpoint.json").write_text(json.dumps({"optimizer_step": 0}))
        with self.assertRaisesRegex(ValueError, "checkpoint age"):
            night.source_info(self.source)

    def test_acquisition_requires_exact_target_touch_and_ownership(self):
        for mutation in ({"owner_before": 1}, {"owner_after": 2}, {"consumed_after": True},
                         {"target": 4}, {"physical": {"contact": "Consumed", "target_entity": 3}},
                         {"selected_command": {"channel": "Manipulation", "primitive": 210,
                                               "target": {"entity": 3}}}):
            event = {**acquisition(0), **mutation}
            with self.assertRaises(ValueError):
                night.validate_acquisitions({"grab_food_acquisitions": [event]}, setup(), 3)
        self.assertEqual(night.validate_acquisitions(
            {"grab_food_acquisitions": [acquisition(0)]}, setup(), 3), [0])

    def test_held_or_consumed_setup_cannot_be_grab_only(self):
        receipt = {"lesson": "grab_food", "grab_food_setup": setup(),
                   "teacher_cue_frames": 1, "teacher_cue_tokens": [9, 1]}
        night.assert_setup(receipt)
        for mutation in ({"initial_owner": 1}, {"initial_consumed": True},
                         {"food_position": {"x": 2., "y": 0., "z": 0.}}):
            with self.assertRaises(ValueError):
                night.assert_setup({**receipt, "grab_food_setup": {**setup(), **mutation}})

    def test_unqualified_run_fails_before_native_output_or_spawn(self):
        binary = self.root / "nonexecutable.exe"
        binary.write_bytes(b"not an executable; never launch")
        output = self.root / "run-output"
        argv = ["--binary", str(binary), "--source", str(self.source), "--output", str(output),
                "--cutoff", (dt.datetime.now(night.UTC) + dt.timedelta(hours=1)).isoformat(), "--run"]
        with patch.object(night.subprocess, "Popen") as spawn, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as failure:
                night.main(argv)
            self.assertEqual(failure.exception.code, 2)
            spawn.assert_not_called()
        self.assertFalse(output.exists())

    def test_unsupported_sampling_temperature_fails_before_output_or_spawn(self):
        argv = ["--binary", "not-executable", "--source", str(self.source),
                "--output", str(self.root / "temperature-output"), "--cutoff",
                (dt.datetime.now(night.UTC) + dt.timedelta(hours=1)).isoformat(),
                "--sampling-temperature", "nan", "--run"]
        with patch.object(night.subprocess, "Popen") as spawn, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                night.main(argv)
            spawn.assert_not_called()
        self.assertFalse((self.root / "temperature-output").exists())

    def test_bounded_temperature_matches_native_float32(self):
        import struct
        for value in (.0001, 1.0, 2.0, 32.0, 128.0):
            actual = night.sampling_temperature(str(value))
            self.assertEqual(struct.pack("<f", actual), struct.pack("<f", value))
        for value in (0, -.1, .00001, 129, float("inf"), float("nan")):
            with self.assertRaises(Exception):
                night.sampling_temperature(str(value))

    def test_exploration_cools_only_after_frozen_grabbing_and_eating_pass(self):
        grab = {"grab_acquisitions": 1}
        eating = {"held_food_meals": 1}
        self.assertEqual(night.cooled_temperature(32., 1., grab, eating, 1), 16.)
        self.assertEqual(night.cooled_temperature(1., 1., grab, eating, 1), 1.)
        self.assertEqual(night.cooled_temperature(32., 1., {"grab_acquisitions": 0}, eating, 1), 32.)
        self.assertEqual(night.cooled_temperature(32., 1., grab, {"held_food_meals": 0}, 0), 32.)
        self.assertEqual(night.cooled_temperature(32., 1., grab, eating, 2), 32.)

    def test_rehearsal_requires_actual_credit_complete_dose_and_preserved_value(self):
        rehearsal = {"requested_epochs": 4, "completed_epochs": 4, "acquisition_row": 2,
                     "loss_rows_per_epoch": 1, "imitation_temperature": 1.,
                     "value_checkpoint_unchanged": True, "action_losses": [8., 7., 6., 5.],
                     "actor_optimizer_step_before": 100, "actor_optimizer_step_after": 104}
        receipt = {"curriculum_reward_rows": [{"row": 2, "curriculum_reward": .25}],
                   "actor_optimizer_step": 104, "acquisition_rehearsal": rehearsal}
        self.assertEqual(night.validate_rehearsal(receipt, 4), 4)
        for change in ({"completed_epochs": 3}, {"requested_epochs": 8}, {"acquisition_row": 3},
                       {"value_checkpoint_unchanged": False}, {"actor_optimizer_step_before": 99},
                       {"action_losses": [8., 7., 6., float("nan")]}):
            with self.assertRaises(ValueError):
                night.validate_rehearsal({**receipt, "acquisition_rehearsal": {**rehearsal, **change}}, 4)

    def test_no_acquisition_has_zero_offline_epochs_and_no_fabricated_dose(self):
        rehearsal = {"requested_epochs": 512, "completed_epochs": 0, "acquisition_row": None,
                     "loss_rows_per_epoch": 0, "imitation_temperature": 1.,
                     "value_checkpoint_unchanged": True, "action_losses": [],
                     "actor_optimizer_step_before": 100, "actor_optimizer_step_after": 100}
        receipt = {"curriculum_reward_rows": [{"row": 0, "curriculum_reward": 0.}],
                   "actor_optimizer_step": 100, "acquisition_rehearsal": rehearsal}
        self.assertEqual(night.validate_rehearsal(receipt, 512), 0)
        with self.assertRaises(ValueError):
            night.validate_rehearsal({**receipt, "acquisition_rehearsal": {**rehearsal, "completed_epochs": 512}}, 512)
        self.assertEqual(night.validate_rehearsal({}, 0), 0)
        with self.assertRaises(ValueError):
            night.validate_rehearsal(receipt, 0)

    def test_invalid_recipe_fails_before_source_output_or_spawn(self):
        for options in (["--acquisition-rehearsal-epochs", "513"],
                        ["--sampling-temperature", "4", "--sampling-temperature-floor", "8"],
                        ["--max-stalled-batches", "65"]):
            output = self.root / "invalid-recipe-output"
            argv = ["--binary", "not-executable", "--source", str(self.source), "--output", str(output),
                    "--cutoff", (dt.datetime.now(night.UTC) + dt.timedelta(hours=1)).isoformat(), *options]
            with patch.object(night.subprocess, "Popen") as spawn, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    night.main(argv)
                spawn.assert_not_called()
            self.assertFalse(output.exists())

    def fixture_run(self, failures=None, fail_episode=None, final_grab_seconds=0, final_remaining_seconds=None,
                    invalid_receipts=(), resource_fault=None, stop_before=None, lane_fault=None, source_fault=False,
                    retention_meals=None, retention_changes=None, recipe_options=()):
        """Exercise real orchestration/validators with a nonexecuting child stub."""
        self.run_index = getattr(self, "run_index", 0) + 1
        output = self.root / "target/founder-training" / f"run-{self.run_index}"
        binary = self.root / "nonexecutable-trainer"
        binary.write_bytes(b"fixture; never execute")
        now = dt.datetime.now(night.UTC)
        clock = [now]
        cutoff = now + dt.timedelta(hours=1)
        argv = ["--binary", str(binary), "--source", str(self.source), "--output", str(output),
                "--cutoff", cutoff.isoformat(), "--target-episodes", "1" if fail_episode is None else "2",
                "--batch-episodes", "2", "--assessment-seconds", "10", "--idle-seconds", "0",
                "--sampling-temperature-floor", "1", *recipe_options]
        args = night.arguments().parse_args(argv)
        with patch("builtins.print"):
            self.assertEqual(night.main(argv), 0)
        state_path = output / "night.json"
        state = night.read_json(state_path)
        checkpoint_hashes = state["input_sha256"].copy()
        attempts = []
        launched = []
        active_command = [None]
        fault_observed = [False]
        real_sha256 = night.sha256

        class FixtureDatetime(dt.datetime):
            @classmethod
            def now(cls, timezone=None):
                return clock[0]

        def fake_child(command, log, deadline, guard, environment):
            name = log.name
            active_command[0] = name
            attempts.append((name, command, deadline, clock[0]))
            if name == stop_before:
                (output / "STOP").touch()
            guard()
            self.assertLessEqual(deadline, cutoff)
            if command[1] == "--evaluate":
                self.assertLessEqual(deadline, clock[0] + dt.timedelta(seconds=args.assessment_seconds))
            if clock[0] >= deadline:
                raise night.CommandTimeoutError("command budget exhausted before launch")
            launched.append(name)
            directory = Path(command[2] if command[1] == "--evaluate" else command[3])
            directory.mkdir()
            if name.startswith("frozen-final"):
                clock[0] += dt.timedelta(seconds=final_grab_seconds)
                self.assertLessEqual(clock[0], deadline)
            if name in (failures or {}):
                raise failures[name]
            if command[1] == "--resume-cycle":
                index = int(name.split("-")[1])
                if index == fail_episode:
                    raise night.CommandTimeoutError("fixture interrupted partial collection")
                previous = Path(command[2])
                (directory / "initial.alife-foundation").write_bytes(
                    (previous / "trained.alife-foundation").read_bytes())
                self.write_cycle(directory, 603, 1845, 1205, {
                    "seed": int(command[6]), "lesson": "grab_food", "grab_food_setup": setup(),
                    "teacher_cue_frames": 1, "teacher_cue_tokens": [9, 1],
                    "prepared_grab_food_opportunities": 1, "training_ticks": 1,
                    "captured_replay_rows": 2, "trained_replay_rows": 1, "bootstrap_replay_rows": 1,
                    "grab_food_acquisitions": [], "trained_grab_food_acquisitions": 0,
                    "grab_food_curriculum_version": 1, "curriculum_reward_rows": [{
                        "row": 0, "physiological_reward": -.01, "curriculum_reward": 0., "combined_reward": -.01}],
                    "curriculum_reward_total": 0., "held_food_consumption_events": 0,
                    "trained_held_food_consumption_events": 0, "completed_epochs": 1,
                    "actor_optimizer_updates": 1, "value_optimizer_updates": 1})
                checkpoint_hashes.update(night.source_info(directory)[1])
                return
            lesson = command[4]
            if lesson == "grab_food":
                receipt = {"lesson": lesson, "grab_food_setup": setup(), "ticks": 16,
                           "teacher_cue_frames": 1, "teacher_cue_tokens": [9, 1], "grab_food_acquisitions": []}
                step = {"representative_mask": 1, "candidates": [{"action": 211, "target": 3, "index": 0}]}
            else:
                held_setup = {**setup(), "realized_grip_position": {"x": .5, "y": 0., "z": 0.}}
                receipt = {"lesson": lesson, "held_food_setup": held_setup, "ticks": 16,
                           "seed": int(command[6]), "founder_seed_base": int(command[7]), "teacher_mode": False,
                           "teacher_cue_frames": 1, "teacher_cue_tokens": [8, 1],
                           "held_food_consumption_events": (retention_meals or {}).get(name, 1)}
                for key, value in (retention_changes or {}).get(name, {}).items():
                    if value is None:
                        receipt.pop(key, None)
                    else:
                        receipt[key] = value
                step = {"lesson_target_held_before_decision": True, "representative_mask": 1,
                        "candidates": [{"action": 210, "target": receipt["held_food_setup"]["target"], "index": 0}]}
            if name in invalid_receipts:
                step["representative_mask"] = 0
            (directory / "pilot.json").write_text(json.dumps(receipt))
            (directory / "lesson-trace.json").write_text(json.dumps({"steps": [step]}))
            if name == "frozen-after-00000" and final_remaining_seconds is not None:
                clock[0] = cutoff - dt.timedelta(seconds=final_remaining_seconds)

        resources = {"disk_free_bytes": 100 * 2**30, "available_memory_bytes": 8 * 2**30, "idle_seconds": 999}

        def fake_resources(root):
            if resource_fault and active_command[0] == resource_fault[0] and not fault_observed[0]:
                fault_observed[0] = True
                fault = resource_fault[1]
                if isinstance(fault, Exception):
                    raise fault
                return {**resources, **fault}
            return resources

        def fake_lane():
            if lane_fault and active_command[0] == "frozen-after-00000":
                raise lane_fault

        def fake_sha256(path):
            if (source_fault and active_command[0] == "frozen-after-00000" and
                    Path(path).name == "trained.alife-foundation"):
                return "fixture mismatched checkpoint digest"
            return real_sha256(path)

        with patch.object(night.dt, "datetime", FixtureDatetime), \
             patch.object(night, "__file__", str(self.root / "scripts/runner.py")), \
             patch.object(night.Path, "cwd", return_value=self.root), \
             patch.object(night, "run_child", side_effect=fake_child), \
             patch.object(night, "resource_snapshot", side_effect=fake_resources), \
             patch.object(night, "assert_lane_idle", side_effect=fake_lane), \
             patch.object(night, "sha256", side_effect=fake_sha256), \
             patch.object(night.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, stdout="")), \
             patch.object(night.subprocess, "check_output", return_value="fixture-source-sha\n"), \
             patch.object(night.subprocess, "Popen") as spawn, \
             patch("builtins.print"), contextlib.redirect_stderr(io.StringIO()):
            night.run_night(args, state, state_path, state["input_sha256"])
            spawn.assert_not_called()
        self.assertEqual(state, night.read_json(state_path))
        self.assertFalse((output.parent / ".eat-held-food-owner.lock").exists())
        self.assertFalse(state["promoted"])
        accepted = self.source if not state["distinct_completed_episodes"] else output / "episode-00000"
        self.assertEqual(state["latest_checkpoint"], str(accepted))
        for path, expected in checkpoint_hashes.items():
            self.assertEqual(night.sha256(path), expected, path)
        finals = [attempt for attempt in attempts if "final" in attempt[0]]
        self.assertEqual(len({attempt[0] for attempt in finals}), len(finals))
        self.assertTrue(all(attempt[1][3] == str(accepted / "trained.alife-foundation")
                            for attempt in finals))
        return state, finals, launched

    def test_final_grab_confirmed_timeout_still_attempts_eat_once(self):
        error = night.CommandTimeoutError("fixture Grab command timeout")
        state, finals, _ = self.fixture_run({"frozen-final": error}, final_grab_seconds=10)
        self.assertEqual([row[0] for row in finals], ["frozen-final", "eating-retention-final"])
        self.assertEqual(state["status"], "interrupted")
        self.assertEqual(state["final_assessment_errors"], {"frozen-final": str(error)})
        self.assertTrue(state["retention_assessments"][-1]["directory"].endswith("eating-retention-final"))
        self.assertFalse(any(row["directory"].endswith("frozen-final") for row in state["assessments"]))

    def test_final_eat_failure_preserves_successful_grab_result(self):
        for error in (night.RunnerSafetyError("fixture Eat child failed"),
                      night.CommandTimeoutError("fixture Eat command timeout")):
            with self.subTest(error=type(error).__name__):
                state, finals, _ = self.fixture_run({"eating-retention-final": error})
                self.assertEqual([row[0] for row in finals], ["frozen-final", "eating-retention-final"])
                self.assertEqual(state["status"], "interrupted" if isinstance(error, TimeoutError) else "failed")
                self.assertTrue(state["assessments"][-1]["directory"].endswith("frozen-final"))
                self.assertEqual(state["final_assessment_errors"], {"eating-retention-final": str(error)})

    def test_invalid_final_grab_receipt_stops_without_companion(self):
        state, finals, _ = self.fixture_run(invalid_receipts=("frozen-final",))
        self.assertEqual([row[0] for row in finals], ["frozen-final"])
        self.assertEqual(state["status"], "failed")
        self.assertIn("selectable Grab", state["final_assessment_errors"]["frozen-final"])
        self.assertFalse(any("final" in row["directory"] for row in state["retention_assessments"]))

    def test_invalid_final_eat_receipt_preserves_recorded_grab(self):
        state, finals, _ = self.fixture_run(invalid_receipts=("eating-retention-final",))
        self.assertEqual(len(finals), 2)
        self.assertEqual(state["status"], "failed")
        self.assertIn("selectable Eat", state["final_assessment_errors"]["eating-retention-final"])
        self.assertTrue(state["assessments"][-1]["directory"].endswith("frozen-final"))

    def test_both_final_failures_are_reported_without_retry(self):
        state, finals, _ = self.fixture_run({"frozen-final": night.CommandTimeoutError("Grab timed out"),
                                           "eating-retention-final": night.CommandTimeoutError("Eat timed out")})
        self.assertEqual(len(finals), 2)
        self.assertEqual(state["error"], "Grab timed out")
        self.assertEqual(state["final_assessment_errors"], {"frozen-final": "Grab timed out",
                                                         "eating-retention-final": "Eat timed out"})

    def test_collection_timeout_assesses_both_from_last_accepted_checkpoint(self):
        state, finals, _ = self.fixture_run(fail_episode=1)
        self.assertEqual([row[0] for row in finals], ["frozen-final-after-interruption",
                                                   "eating-retention-final-after-interruption"])
        self.assertEqual(state["status"], "interrupted")
        self.assertTrue(state["partial_episode"].endswith("episode-00001"))
        self.assertEqual(state["distinct_completed_episodes"], 1)

    def test_interrupted_final_grab_failure_still_assesses_eat_and_records_both_errors(self):
        for eat_error in (None, ValueError("Eat invalid")):
            with self.subTest(eat_error=eat_error):
                failures = {"frozen-final-after-interruption": night.CommandTimeoutError("Grab timed out")}
                if eat_error is not None:
                    failures["eating-retention-final-after-interruption"] = eat_error
                state, finals, _ = self.fixture_run(failures, fail_episode=1)
                self.assertEqual(len(finals), 2)
                self.assertEqual(state["error"], "fixture interrupted partial collection")
                self.assertEqual(state["final_assessment_error"], "Grab timed out" if eat_error is None else "Eat invalid")
                self.assertEqual(state["final_assessment_errors"], {name: str(error) for name, error in failures.items()})
                if eat_error is None:
                    self.assertTrue(state["retention_assessments"][-1]["directory"].endswith(
                        "eating-retention-final-after-interruption"))

    def test_unqualified_collection_timeout_does_not_start_final_assessments(self):
        state, finals, _ = self.fixture_run(fail_episode=0)
        self.assertEqual(finals, [])
        self.assertEqual(state["status"], "interrupted")
        self.assertEqual(state["distinct_completed_episodes"], 0)

    def test_owner_keyboard_interrupt_does_not_launch_companion_or_recovery(self):
        state, finals, launched = self.fixture_run({"frozen-final": KeyboardInterrupt()})
        self.assertEqual([row[0] for row in finals], ["frozen-final"])
        self.assertNotIn("eating-retention-final", launched)
        self.assertEqual(state["status"], "interrupted")
        self.assertEqual(state["error"], "owner interrupted the runner")

    def test_transient_resource_guard_failure_stops_without_companion(self):
        for fault in ({"available_memory_bytes": 0}, {"disk_free_bytes": 0},
                      TimeoutError("fixture resource telemetry timeout")):
            with self.subTest(fault=fault):
                state, finals, launched = self.fixture_run(resource_fault=("frozen-final", fault))
                self.assertEqual([row[0] for row in finals], ["frozen-final"])
                self.assertNotIn("eating-retention-final", launched)
                self.assertEqual(state["status"], "failed")
                self.assertIn("frozen-final", state["final_assessment_errors"])

    def test_owner_stop_aborts_companion(self):
        state, finals, launched = self.fixture_run(stop_before="frozen-final")
        self.assertEqual([row[0] for row in finals], ["frozen-final"])
        self.assertNotIn("eating-retention-final", launched)
        self.assertEqual(state["status"], "failed")
        self.assertEqual(state["final_assessment_errors"]["frozen-final"], "owner STOP file requested interruption")

    def test_source_or_lane_guard_failure_does_not_launch_assessments(self):
        for options in ({"lane_fault": RuntimeError("fixture competing lane")}, {"source_fault": True}):
            with self.subTest(options=options):
                state, finals, launched = self.fixture_run(**options)
                self.assertEqual(finals, [])
                self.assertNotIn("eating-retention-final", launched)
                self.assertEqual(state["status"], "failed")
                self.assertIn("guard failed", state["final_assessment_errors"]["frozen-final"])

    def test_unknown_exception_or_unconfirmed_cleanup_stops_companion(self):
        for error in (RuntimeError("fixture unclassified native error"),
                      TimeoutError("fixture unclassified I/O timeout"),
                      night.RunnerSafetyError("owned command cleanup failed: fixture exit unconfirmed")):
            with self.subTest(error=error):
                state, finals, launched = self.fixture_run({"frozen-final": error})
                self.assertEqual([row[0] for row in finals], ["frozen-final"])
                self.assertNotIn("eating-retention-final", launched)
                self.assertEqual(state["status"], "failed")
                self.assertEqual(state["final_assessment_errors"], {"frozen-final": str(error)})

    def test_command_timeout_requires_confirmed_cleanup(self):
        now = dt.datetime.now(night.UTC)
        deadline = now + dt.timedelta(seconds=10)

        class DeadlineDatetime(dt.datetime):
            calls = 0

            @classmethod
            def now(cls, timezone=None):
                cls.calls += 1
                return now if cls.calls == 1 else deadline

        for cleanup_error in (None, subprocess.TimeoutExpired("fixture-owned-process", 10)):
            with self.subTest(cleanup_error=cleanup_error):
                DeadlineDatetime.calls = 0
                process = Mock()
                process.poll.return_value = None
                expected = night.CommandTimeoutError if cleanup_error is None else night.RunnerSafetyError
                log = self.root / ("confirmed-timeout" if cleanup_error is None else "unconfirmed-timeout")
                with patch.object(night.dt, "datetime", DeadlineDatetime), \
                     patch.object(night.subprocess, "Popen", return_value=process), \
                     patch.object(night, "stop_owned", side_effect=cleanup_error) as cleanup:
                    with self.assertRaises(expected):
                        night.run_child(["fixture; never execute"], log, deadline, lambda: None, {})
                    cleanup.assert_called_once_with(process)

    def test_native_nonzero_exit_is_a_fatal_diagnostic_stop(self):
        process = Mock()
        process.poll.return_value = 1
        process.returncode = 1
        deadline = dt.datetime.now(night.UTC) + dt.timedelta(seconds=10)
        with patch.object(night.subprocess, "Popen", return_value=process), \
             patch.object(night, "stop_owned") as cleanup:
            with self.assertRaisesRegex(night.RunnerSafetyError, "child exit 1"):
                night.run_child(["fixture; never execute"], self.root / "native-error", deadline, lambda: None, {})
            cleanup.assert_called_once_with(process)

    def test_cutoff_rejects_second_assessment_before_launch_without_extending_budget(self):
        state, finals, launched = self.fixture_run({"frozen-final": night.CommandTimeoutError("Grab reached cutoff")},
                                                  final_grab_seconds=10, final_remaining_seconds=10)
        self.assertEqual(len(finals), 2)
        self.assertEqual(finals[1][2], finals[1][3])
        self.assertNotIn("eating-retention-final", launched)
        self.assertEqual(state["final_assessment_errors"]["eating-retention-final"],
                         "command budget exhausted before launch")
        with patch.object(night.subprocess, "Popen") as spawn:
            with self.assertRaisesRegex(TimeoutError, "before launch"):
                night.run_child([], self.root / "expired", dt.datetime.now(night.UTC) - dt.timedelta(seconds=1),
                                lambda: None, {})
            spawn.assert_not_called()

    def test_successful_final_assessments_complete_once_without_errors(self):
        state, finals, _ = self.fixture_run()
        self.assertEqual([row[0] for row in finals], ["frozen-final", "eating-retention-final"])
        self.assertEqual(state["status"], "completed")
        self.assertNotIn("final_assessment_errors", state)

    def test_final_retention_lower_equal_higher_are_report_only(self):
        for before, final, expected in ((1, 0, "lower"), (1, 1, "equal"), (0, 0, "equal"), (0, 1, "higher")):
            with self.subTest(before=before, final=final):
                state, finals, _ = self.fixture_run(retention_meals={
                    "eating-retention-before": before, "eating-retention-final": final},
                    retention_changes={"eating-retention-before": {"ticks": 1 if before else 16},
                                       "eating-retention-final": {"ticks": 1 if final else 16}})
                diagnostic = state["eating_retention_diagnostic"]
                self.assertEqual(diagnostic["status"], expected)
                self.assertTrue(diagnostic["comparable"])
                self.assertEqual((diagnostic["before_meals"], diagnostic["final_meals"]), (before, final))
                self.assertEqual(bool(diagnostic["warnings"]), expected == "lower")
                self.assertEqual(state["status"], "completed")
                self.assertEqual(state["distinct_completed_episodes"], 1)
                self.assertEqual(len(finals), 2)

    def test_missing_or_invalid_explicit_retention_count_is_unknown_never_zero(self):
        for phase in ("before", "final"):
            name = f"eating-retention-{phase}"
            for count in (None, -1, True, "0", .5, 2, 17):
                with self.subTest(phase=phase, count=count):
                    state, _, _ = self.fixture_run(retention_changes={name: {"held_food_consumption_events": count}})
                    diagnostic = state["eating_retention_diagnostic"]
                    self.assertEqual(diagnostic["status"], "unknown")
                    self.assertIsNone(diagnostic[f"{phase}_meals"])
                    self.assertFalse(diagnostic["comparable"])
                    self.assertEqual(diagnostic["warnings"], [])
                    self.assertEqual(state["status"], "completed")
                    # The legacy reader/default is deliberately preserved.
                    if count is None:
                        assessment = next(row for row in state["retention_assessments"]
                                          if Path(row["directory"]).name == name)
                        self.assertEqual(assessment["held_food_meals"], 0)

    def test_final_retention_timeout_invalid_or_unattempted_is_unknown(self):
        for options in ({"failures": {"eating-retention-final": night.CommandTimeoutError("Eat timed out")}},
                        {"failures": {"eating-retention-before": night.CommandTimeoutError("baseline timed out")}},
                        {"invalid_receipts": ("eating-retention-final",)},
                        {"invalid_receipts": ("frozen-final",)}, {"fail_episode": 0}):
            with self.subTest(options=options):
                state, _, _ = self.fixture_run(**options)
                diagnostic = state["eating_retention_diagnostic"]
                self.assertEqual(diagnostic["status"], "unknown")
                self.assertIsNone(diagnostic["final_meals"])
                self.assertFalse(diagnostic["comparable"])
                self.assertEqual(diagnostic["warnings"], [])

    def test_confirmed_grab_timeout_preserves_successful_eat_comparison(self):
        state, _, _ = self.fixture_run({"frozen-final": night.CommandTimeoutError("Grab timed out")},
                                      retention_meals={"eating-retention-final": 0})
        self.assertEqual(state["status"], "interrupted")
        self.assertEqual(state["eating_retention_diagnostic"]["status"], "lower")

    def test_interrupted_collection_compares_last_accepted_source(self):
        state, _, _ = self.fixture_run(fail_episode=1,
                                      retention_meals={"eating-retention-final-after-interruption": 0})
        self.assertEqual(state["status"], "interrupted")
        self.assertEqual(state["eating_retention_diagnostic"]["status"], "lower")
        self.assertTrue(state["eating_retention_diagnostic"]["final_directory"].endswith("after-interruption"))

    def test_changed_or_missing_assay_context_is_unknown(self):
        for changes in ({"seed": 123}, {"founder_seed_base": 456}, {"teacher_cue_tokens": [9, 1]},
                        {"teacher_mode": True}, {"seed": None}, {"ticks": 0},
                        {"held_food_setup": {**setup(), "initial_hunger": float("nan"),
                                             "realized_grip_position": {"x": .5, "y": 0., "z": 0.}}},
                        {"held_food_setup": {**setup(), "sampling_seed": True,
                                             "realized_grip_position": {"x": .5, "y": 0., "z": 0.}}},
                        {"held_food_setup": {**setup(), "initial_hunger": .3,
                                             "realized_grip_position": {"x": .5, "y": 0., "z": 0.}}}):
            with self.subTest(changes=changes):
                state, _, _ = self.fixture_run(retention_meals={"eating-retention-final": 0},
                                              retention_changes={"eating-retention-final": changes})
                diagnostic = state["eating_retention_diagnostic"]
                self.assertEqual(diagnostic["status"], "unknown")
                self.assertFalse(diagnostic["comparable"])
                self.assertTrue(diagnostic["reasons"])
                self.assertEqual(diagnostic["warnings"], [])
                self.assertEqual(state["status"], "completed")

    def test_early_eating_and_fresh_instance_identity_do_not_block_comparison(self):
        state, _, _ = self.fixture_run(retention_meals={"eating-retention-before": 0},
                                      retention_changes={"eating-retention-final": {"ticks": 1,
                                          "held_food_setup": {**setup(), "organism": 99, "target": 101,
                                              "realized_grip_position": {"x": .5, "y": 0., "z": 0.}}}})
        diagnostic = state["eating_retention_diagnostic"]
        self.assertEqual(diagnostic["status"], "higher")
        self.assertEqual((diagnostic["before_decisions"], diagnostic["final_decisions"]), (16, 1))
        self.assertNotIn("organism", diagnostic["before_context"]["held_setup"])
        self.assertNotIn("target", diagnostic["before_context"]["held_setup"])

    def test_recipe_batch_failure_and_final_report_only_policy_are_preserved(self):
        state, finals, _ = self.fixture_run(retention_meals={"eating-retention-after-00000": 0},
                                          recipe_options=("--max-stalled-batches", "2"))
        self.assertEqual(state["status"], "failed")
        self.assertEqual(state["error"], "frozen batch lost source eating retention")
        self.assertEqual(finals, [])
        state, finals, _ = self.fixture_run(retention_meals={"eating-retention-final": 0},
                                          recipe_options=("--max-stalled-batches", "2"))
        self.assertEqual(state["status"], "completed")
        self.assertEqual(state["eating_retention_diagnostic"]["status"], "lower")
        self.assertEqual(len(finals), 2)

    def test_shared_malformed_cue_evidence_is_unknown_without_changing_admission(self):
        for change in ({"teacher_cue_tokens": [-1, 1]}, {"teacher_cue_tokens": [65536, 1]},
                       {"teacher_cue_tokens": [True, 1]}, {"teacher_cue_frames": "1"},
                       {"teacher_cue_frames": True}, {"teacher_cue_frames": 17}):
            with self.subTest(change=change):
                state, _, _ = self.fixture_run(retention_meals={"eating-retention-final": 0},
                    retention_changes={name: change for name in
                                       ("eating-retention-before", "eating-retention-final")})
                diagnostic = state["eating_retention_diagnostic"]
                self.assertEqual(diagnostic["status"], "unknown")
                self.assertTrue(diagnostic["reasons"])
                self.assertFalse(diagnostic["comparable"])
                self.assertEqual(diagnostic["warnings"], [])
                self.assertEqual(state["status"], "completed")

    def test_shared_out_of_native_need_or_health_bounds_is_unknown(self):
        for key in ("initial_hunger", "initial_health"):
            with self.subTest(key=key):
                held_setup = {**setup(), key: 2., "realized_grip_position": {"x": .5, "y": 0., "z": 0.}}
                state, _, _ = self.fixture_run(retention_meals={"eating-retention-final": 0},
                    retention_changes={name: {"held_food_setup": held_setup} for name in
                                       ("eating-retention-before", "eating-retention-final")})
                self.assertEqual(state["status"], "completed")
                self.assertEqual(state["eating_retention_diagnostic"]["status"], "unknown")
                self.assertEqual(state["eating_retention_diagnostic"]["warnings"], [])

if __name__ == "__main__":
    unittest.main()
