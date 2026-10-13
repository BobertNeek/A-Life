#!/usr/bin/env python3
"""CPU receipt fixtures only; never launch a learner, GPU, or prior provider."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest


def load_night(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


grab = load_night("run_grab_food_night")
held = load_night("run_eat_held_food_night")
MISSING = object()
COUNT_KEYS = ("decision_frames", "decision_frames_with_prior", "decision_frames_without_prior")
VALIDATORS = (("grab", grab.validate_probe),
              ("retention", grab.validate_retention_probe),
              ("held", held.validate_probe))


def delivered_prior():
    return {"rich_information_required": True, "decision_frames": 2,
            "decision_frames_with_prior": 2, "decision_frames_without_prior": 0,
            "decision_inputs": [{"organism": 1, "sequence": index + 1, "tick": index,
                                 "status": "delivered", "nonzero_prior_lanes": 2,
                                 "nonzero_encoded_prior_lanes": 2} for index in range(2)]}


def write_probe(directory, kind, prior, success=True, selectable=True):
    directory.mkdir()
    setup = {"target": 3, "organism": 1, "sampling_seed": 10, "sampling_attempts": 1,
             "realized_body_position": {"x": 0., "y": 0., "z": 0.},
             "realized_body_yaw": 0., "initial_sleeping": False,
             "initial_energy": .85, "initial_hunger": .15, "initial_health": 1.}
    receipt = {"ticks": 2, "teacher_cue_frames": 1}
    if kind == "grab":
        setup.update({"food_position": {"x": .5, "y": 0., "z": 0.},
                      "initial_owner": None, "initial_consumed": False})
        receipt.update({"lesson": "grab_food", "grab_food_setup": setup,
                        "teacher_cue_tokens": [9, 1], "lesson_completed": success,
                        "grab_food_acquisitions": [{
                            "row": 0, "organism": 1, "target": 3,
                            "owner_before": None, "owner_after": 1, "consumed_after": False,
                            "selected_command": {"channel": "Manipulation", "primitive": 211,
                                                 "target": {"entity": 3}},
                            "physical": {"contact": "Touch", "target_entity": 3}}
                            ] if success else []})
        expected = {"lesson": "grab_food", "grab_acquisitions": int(success),
                    "first_acquisition_row": 0 if success else None,
                    "decisions": 2, "directory": str(directory)}
        action = 211
    else:
        setup["realized_grip_position"] = {"x": .5, "y": 0., "z": 0.}
        receipt.update({"lesson": "eat_held_food", "held_food_setup": setup,
                        "held_food_consumption_events": int(success)})
        expected = {"held_food_meals": int(success), "decisions": 2, "directory": str(directory)}
        action = 210
    if prior is not MISSING:
        receipt["semantic_prior"] = prior
    (directory / "pilot.json").write_text(json.dumps(receipt), encoding="utf-8")
    (directory / "lesson-trace.json").write_text(json.dumps({"steps": [{
        "lesson_target_held_before_decision": kind != "grab",
        "representative_mask": int(selectable),
        "candidates": [{"action": action, "target": 3, "index": 0}]}]}), encoding="utf-8")
    return receipt, expected


class ProbePriorSummaryTests(unittest.TestCase):
    def check_summaries(self, prior, counts, warnings, success=True):
        with tempfile.TemporaryDirectory() as temporary:
            for kind, validate in VALIDATORS:
                with self.subTest(probe=kind):
                    directory = Path(temporary) / kind
                    receipt, expected = write_probe(directory, kind, prior, success)
                    original = (directory / "pilot.json").read_bytes()
                    stderr = io.StringIO()
                    with contextlib.redirect_stderr(stderr):
                        summary = validate(directory)
                    diagnostics = summary.pop("semantic_prior_diagnostics")
                    self.assertEqual(summary, expected)
                    self.assertEqual(set(diagnostics), {"failure_policy", "warnings", *COUNT_KEYS})
                    self.assertEqual(diagnostics["failure_policy"], "log_and_continue_without_prior")
                    self.assertEqual(tuple(diagnostics[key] for key in COUNT_KEYS), counts)
                    self.assertEqual(bool(diagnostics["warnings"]), warnings)
                    self.assertEqual(bool(stderr.getvalue()), warnings)
                    self.assertEqual(json.loads(json.dumps(diagnostics)), diagnostics)
                    # Validation reports diagnostics without rewriting the original receipt.
                    self.assertEqual((directory / "pilot.json").read_bytes(), original)
                    self.assertEqual(json.loads(original), receipt)

    def test_complete_delivery_survives_all_three_compact_summaries(self):
        self.check_summaries(delivered_prior(), (2, 2, 0), False)

    def test_unavailable_provider_logs_and_keeps_successful_behavior(self):
        prior = delivered_prior()
        prior.update({"decision_frames_with_prior": 0, "decision_frames_without_prior": 2,
                      "failures": 1, "prime_timeouts": 1,
                      "provider_failures": [{"error": "provider unavailable"}]})
        for row in prior["decision_inputs"]:
            row.update({"status": "unavailable", "nonzero_prior_lanes": 0,
                        "nonzero_encoded_prior_lanes": 0})
        self.check_summaries(prior, (2, 0, 2), True)

    def test_missing_receipts_keep_coverage_unknown(self):
        for prior in (MISSING, None, {}):
            with self.subTest(prior="absent" if prior is MISSING else prior):
                self.check_summaries(prior, (None, None, None), True)

    def test_partial_receipt_does_not_infer_coverage_from_episode_decisions(self):
        self.check_summaries({"decision_frames": 2}, (2, None, None), True)

    def test_invalid_counts_remain_unknown_and_nonblocking(self):
        for key in COUNT_KEYS:
            for invalid in (-1, "2", True):
                with self.subTest(count=key, value=invalid):
                    prior = delivered_prior()
                    prior[key] = invalid
                    counts = tuple(None if name == key else prior[name] for name in COUNT_KEYS)
                    self.check_summaries(prior, counts, True)

    def test_malformed_optional_identities_warn_without_rejecting_the_probe(self):
        for key in ("organism", "sequence", "tick"):
            for invalid in ([], {}, "1", True, None, -1, 2**64):
                with self.subTest(identity=key, value=invalid):
                    prior = delivered_prior()
                    prior["decision_inputs"][0][key] = invalid
                    self.check_summaries(prior, (2, 2, 0), True)

    def test_valid_u64_identity_boundaries_are_preserved(self):
        prior = delivered_prior()
        prior["decision_inputs"][0].update({"organism": 0, "sequence": 0, "tick": 0})
        prior["decision_inputs"][1].update({"organism": 2**64 - 1, "sequence": 2**64 - 1,
                                            "tick": 2**64 - 1})
        self.check_summaries(prior, (2, 2, 0), False)

    def test_missing_prior_does_not_invent_behavioral_success(self):
        self.check_summaries(MISSING, (None, None, None), True, success=False)

    def test_missing_prior_does_not_waive_selectable_action_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            for kind, validate in VALIDATORS:
                with self.subTest(probe=kind):
                    directory = Path(temporary) / kind
                    write_probe(directory, kind, MISSING, selectable=False)
                    with contextlib.redirect_stderr(io.StringIO()):
                        with self.assertRaisesRegex(ValueError, "selectable"):
                            validate(directory)


if __name__ == "__main__":
    unittest.main()
