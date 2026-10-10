#!/usr/bin/env python3
"""CPU orchestration fixtures only; never execute the training binary."""
import datetime as dt
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("run_eat_held_food_night.py")
spec = importlib.util.spec_from_file_location("held_food_night", SCRIPT)
night = importlib.util.module_from_spec(spec); spec.loader.exec_module(night)


def setup(seed=1):
    return {"target": 3, "sampling_seed": seed, "sampling_attempts": 1,
            "realized_body_position": {"x": float(seed), "y": 0., "z": 0.}, "realized_body_yaw": .5,
            "realized_grip_position": {"x": float(seed) + .5, "y": 0., "z": 0.},
            "initial_sleeping": False, "initial_energy": .85,
            "initial_hunger": .15, "initial_health": 1.}


def prior(frames=1):
    return {"rich_information_required": True, "decision_frames": frames,
            "decision_frames_with_prior": frames, "decision_frames_without_prior": 0,
            "decision_inputs": [{"organism": 1, "sequence": index + 1, "tick": index,
                                 "status": "delivered", "nonzero_prior_lanes": 2,
                                 "nonzero_encoded_prior_lanes": 2} for index in range(frames)]}


class HeldFoodNightTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.binary = self.root / "nonexecutable-trainer"; self.binary.write_bytes(b"fixture; cannot execute")
        self.source = self.root / "source"; self.source.mkdir()
        (self.source / "adaptation.json").write_text(json.dumps({"founder_seed_base": 42}))
        (self.source / "trained.alife-foundation").write_bytes(b"fixture initial source")
        self.output = self.root / "target/founder-training/night"
        self.cutoff = (dt.datetime.now(night.UTC) + dt.timedelta(hours=8)).isoformat()

    def argv(self):
        return ["--binary", str(self.binary), "--source", str(self.source),
                "--output", str(self.output), "--cutoff", self.cutoff]

    def cycle(self, directory, previous, seed, index=0):
        directory.mkdir()
        (directory / "initial.alife-foundation").write_bytes((previous / "trained.alife-foundation").read_bytes())
        (directory / "trained.alife-foundation").write_bytes(f"fixture trained {index}".encode())
        for head in ("actor", "value"):
            (directory / f"{head}-checkpoint.json").write_text('{}')
        receipt = {"lesson": "eat_held_food", "held_food_setup": setup(seed), "semantic_prior": prior(4),
                   "teacher_cue_frames": 1, "seed": seed, "founder_seed_base": 42,
                   "next_cohort_tick_captured": True, "next_cohort_optimizer_rebound": True,
                   "prepared_held_food_opportunities": 1, "training_ticks": 3,
                   "captured_replay_rows": 4, "trained_replay_rows": 3, "bootstrap_replay_rows": 1,
                   "held_food_consumption_events": 1, "trained_held_food_consumption_events": 0,
                   "actor_optimizer_step": (index + 1) * 8, "value_optimizer_step": (index + 1) * 8,
                   "actor_optimizer_updates": 8, "value_optimizer_updates": 8,
                   "policy_version": index + 1, "completed_epochs": 8}
        (directory / "cycle.json").write_text(json.dumps(receipt))
        return receipt

    def probe(self, directory):
        directory.mkdir()
        receipt = {"lesson": "eat_held_food", "held_food_setup": setup(), "semantic_prior": prior(16),
                   "teacher_cue_frames": 1, "ticks": 16, "held_food_consumption_events": 0}
        (directory / "pilot.json").write_text(json.dumps(receipt))
        (directory / "lesson-diagnostic.json").write_text(json.dumps({"steps": [{
            "lesson_target_held_before_decision": True, "representative_mask": 1,
            "candidates": [{"action": 210, "target": 3, "index": 0}]}]}))

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

    def test_plan_targets_thousands_without_launching_nonexecutable_trainer(self):
        result = subprocess.run([sys.executable, str(SCRIPT), *self.argv()], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        state = night.read_json(self.output / "night.json")
        self.assertEqual(state["target_distinct_episodes"], 3000)
        self.assertEqual(state["distinct_completed_episodes"], 0)
        self.assertEqual(state["training_seed_range"], [2026100502, 2026103501])
        self.assertEqual(state["first_episode_argv"][-2:], ["--lesson", "eat_held_food"])
        self.assertFalse(state["individual_lifetime_continues_between_episodes"])
        self.assertFalse((self.output.parent / ".eat-held-food-owner.lock").exists())
        self.assertEqual(list(self.output.iterdir()), [self.output / "night.json"])

    def test_invalid_seed_overlap_or_trace_window_creates_no_output(self):
        for extra in (["--assessment-seed", "2026100502"], ["--seed", str(2**64 - 1)],
                      ["--decisions", "256"], ["--min-disk-gib", "0"], ["--min-memory-gib", "nan"]):
            result = subprocess.run([sys.executable, str(SCRIPT), *self.argv(), *extra], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertFalse(self.output.exists())

    def test_cycle_separates_bootstrap_meal_and_verifies_exact_resume(self):
        first = self.root / "first"; self.cycle(first, self.source, 10)
        receipt, _, _ = night.validate_cycle(first, self.source, 10)
        self.assertEqual(receipt["held_food_consumption_events"], 1)
        self.assertEqual(receipt["trained_held_food_consumption_events"], 0)
        second = self.root / "second"; receipt = self.cycle(second, first, 11, 1)
        night.validate_cycle(second, first, 11)
        receipt["actor_optimizer_updates"] -= 1
        (second / "cycle.json").write_text(json.dumps(receipt))
        with self.assertRaisesRegex(ValueError, "optimizer age"):
            night.validate_cycle(second, first, 11)

    def test_configured_prior_or_unselectable_ground_target_is_rejected(self):
        self.assertTrue(night.assert_prior({"semantic_prior": {"rich_information_required": True}})["warnings"])
        directory = self.root / "probe"; self.probe(directory)
        self.assertEqual(night.validate_probe(directory)["held_food_meals"], 0)
        trace = night.read_json(directory / "lesson-diagnostic.json")
        trace["steps"][0]["representative_mask"] = 0
        (directory / "lesson-diagnostic.json").write_text(json.dumps(trace))
        with self.assertRaisesRegex(ValueError, "selectable Eat"):
            night.validate_probe(directory)
        self.assertTrue(night.assert_prior({"semantic_prior": prior(1)}, 16)["warnings"])

    def test_native_pose_and_ambiguous_source_receipts_fail_explicitly(self):
        with self.assertRaises(ValueError):
            night.coordinates([1., 0., 2.])
        (self.source / "cycle.json").write_text('{}')
        with self.assertRaisesRegex(ValueError, "unambiguous"):
            night.source_info(self.source)

    def test_later_held_food_source_cannot_repeat_an_earlier_training_prefix(self):
        (self.source / "adaptation.json").unlink()
        (self.source / "cycle.json").write_text(json.dumps({"founder_seed_base": 42,
            "lesson": "eat_held_food", "seed": 2026106000}))
        for head in ("actor", "value"):
            (self.source / f"{head}-checkpoint.json").write_text('{}')
        result = subprocess.run([sys.executable, str(SCRIPT), *self.argv()], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("fresh range", result.stderr)
        self.assertFalse(self.output.exists())

    def test_owned_cpu_child_is_stopped_at_its_process_budget(self):
        # This launches only a disposable Python sleep fixture, never a GPU.
        deadline = dt.datetime.now(night.UTC) + dt.timedelta(seconds=.2)
        with self.assertRaises(TimeoutError):
            night.run_child([sys.executable, "-c", "import time; time.sleep(30)"],
                            self.root / "child", deadline, lambda: None, os.environ.copy())

    def test_start_receipt_occurs_only_after_actual_process_creation(self):
        started = []
        deadline = dt.datetime.now(night.UTC) + dt.timedelta(seconds=10)
        def blocked():
            raise RuntimeError("host became interactive")
        with self.assertRaises(RuntimeError):
            night.run_child([sys.executable, "-c", "pass"], self.root / "blocked", deadline,
                            blocked, os.environ.copy(), on_started=lambda p: started.append(p.pid))
        self.assertEqual(started, [])
        night.run_child([sys.executable, "-c", "pass"], self.root / "accepted", deadline,
                        lambda: None, os.environ.copy(), on_started=lambda p: started.append(p.pid))
        self.assertEqual(len(started), 1)
        self.assertGreater(started[0], 0)

    def test_capacity_is_bounded_by_measured_time_and_disk(self):
        samples = [{"seconds": 10, "bytes": 100}, {"seconds": 20, "bytes": 200}]
        self.assertEqual(night.estimate_capacity(samples, 100, 10000, 0)["estimated_remaining_episodes"], 4)
        self.assertEqual(night.estimate_capacity(samples, 1000, 450, 250)["estimated_remaining_episodes"], 1)

    def fixture_run(self, fail_index=None, batch_episodes=2, meals=None, preserve_from=None):
        parser = night.arguments()
        transition = [] if preserve_from is None else ["--preserve-objective-state-from", str(preserve_from)]
        args = parser.parse_args([*self.argv(), "--target-episodes", "3", "--batch-episodes", str(batch_episodes), *transition])
        args.binary = self.binary; args.source = self.source
        self.output.mkdir(parents=True)
        # Populate the same plan that production uses; it never runs a trainer.
        self.output.rmdir()
        with patch("builtins.print"):
            self.assertEqual(night.main([*self.argv(), "--target-episodes", "3", "--batch-episodes", "2", *transition]), 0)
        state_path = self.output / "night.json"; state = night.read_json(state_path)
        commands = []
        probe_index = 0

        def fake_child(argv, log, deadline, guard, environment):
            nonlocal probe_index
            guard(); commands.append(argv)
            self.assertEqual(environment["ALIFE_SLM_PRIOR"], "on")
            if argv[1] == "--evaluate":
                self.probe(Path(argv[2]))
                if meals is not None:
                    path = Path(argv[2]) / "pilot.json"
                    receipt = night.read_json(path)
                    receipt["held_food_consumption_events"] = meals[probe_index]
                    path.write_text(json.dumps(receipt))
                probe_index += 1
            else:
                index = int(Path(argv[3]).name.split('-')[1])
                if index == fail_index:
                    Path(argv[3]).mkdir()
                    raise TimeoutError("fixture cutoff during owned episode")
                self.cycle(Path(argv[3]), Path(argv[2]), int(argv[6]), index)

        old_cwd = Path.cwd(); os.chdir(self.root); self.addCleanup(os.chdir, old_cwd)
        resources = {"disk_free_bytes": 100 * 2**30, "available_memory_bytes": 8 * 2**30, "idle_seconds": 999}
        with patch.object(night, "__file__", str(self.root / "scripts/runner.py")), \
             patch.object(night, "run_child", side_effect=fake_child), \
             patch.object(night, "resource_snapshot", return_value=resources), \
             patch.object(night, "assert_lane_idle"), \
             patch.object(night.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, stdout="")), \
             patch.object(night.subprocess, "check_output", return_value="fixture-source-sha\n"), \
             patch("builtins.print"), \
             patch.dict(os.environ, {"ALIFE_SLM_PRIOR_MODEL": "fixture", "ALIFE_SLM_PRIOR_MODEL_SHA256": "a" * 64}):
            night.run_night(args, state, state_path, state["input_sha256"])
        return state, commands

    def test_multi_batch_continuation_counts_examples_separately_from_repeats(self):
        state, commands = self.fixture_run()
        self.assertEqual(state["status"], "completed", state["error"])
        self.assertEqual(state["distinct_completed_episodes"], 3)
        self.assertEqual(state["trained_decision_rows"], 9)
        self.assertEqual(state["captured_decision_rows"], 12)
        self.assertEqual(state["bootstrap_rows"], 3)
        self.assertEqual(state["completed_epoch_repeats"], 24)
        self.assertEqual(state["actor_optimizer_updates"], 24)
        self.assertEqual(state["captured_held_meals"], 3)
        self.assertEqual(state["trained_row_held_meals"], 0)
        cycles = [argv for argv in commands if argv[1] == "--resume-cycle"]
        self.assertEqual(len(cycles), 3)
        self.assertEqual(cycles[1][2], cycles[0][3])
        self.assertEqual(cycles[2][2], cycles[1][3])
        self.assertTrue(all(argv[-1] == "eat_held_food" for argv in cycles))
        self.assertFalse((self.output.parent / ".eat-held-food-owner.lock").exists())

    def test_partial_episode_never_becomes_next_checkpoint(self):
        state, commands = self.fixture_run(fail_index=1)
        self.assertEqual(state["status"], "interrupted")
        self.assertEqual(state["distinct_completed_episodes"], 1)
        self.assertTrue(state["latest_checkpoint"].endswith("episode-00000"))
        self.assertTrue(state["partial_episode"].endswith("episode-00001"))
        self.assertEqual(len([argv for argv in commands if argv[1] == "--resume-cycle"]), 2)
        self.assertEqual(commands[-1][1], "--evaluate")
        self.assertTrue(commands[-1][2].endswith("frozen-final-after-interruption"))
        self.assertEqual(commands[-1][3], str(self.output / "episode-00000/trained.alife-foundation"))

    def test_explicit_objective_transition_reaches_only_first_executed_cycle(self):
        state, commands = self.fixture_run(preserve_from=1)
        self.assertEqual(state["status"], "completed", state["error"])
        cycles = [argv for argv in commands if argv[1] == "--resume-cycle"]
        self.assertEqual(state["first_episode_argv"][-2:], ["--preserve-objective-state-from", "1"])
        self.assertEqual(cycles[0][-2:], ["--preserve-objective-state-from", "1"])
        self.assertTrue(all("--preserve-objective-state-from" not in argv for argv in cycles[1:]))

    def test_learning_then_two_lost_frozen_meals_stops_without_promotion(self):
        state, _ = self.fixture_run(batch_episodes=1, meals=[0, 1, 0, 0])
        self.assertEqual(state["status"], "failed")
        self.assertIn("two frozen batch checks", state["error"])
        self.assertFalse(state["promoted"])


if __name__ == "__main__":
    unittest.main()
