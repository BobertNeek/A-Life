#!/usr/bin/env python3
"""CPU receipt/admission fixtures. Never execute a training binary or provider."""
import contextlib
import datetime as dt
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

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

if __name__ == "__main__":
    unittest.main()
