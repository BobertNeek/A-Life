"""CPU-only synthetic contracts; none of these fixtures is a native replay result."""

import contextlib
import copy
import io
import itertools
import json
import math
from pathlib import Path
import random
import struct
import tempfile
import unittest

import audit_manipulation_probabilities as audit


def row(kinds=("Interact", "Interact", "Idle", "Move"), logits=None, enabled=63):
    if logits is None:
        logits = [0.0] * len(kinds)
    values = [audit.parse_logit(v, "fixture")[0] for v in logits]
    rep = sum(1 << i for i, value in enumerate(values) if math.isfinite(value))
    motors = [0] * 6
    forced = []
    for i, kind in enumerate(kinds):
        slot = audit.KIND_SLOT[kind]
        forced.append(4 if slot is None else slot)
        if slot is not None and enabled & (1 << slot) and rep & (1 << i):
            motors[slot] |= 1 << i
    return {"row_id": "synthetic-row", "evidence_kind": "synthetic", "behavior": "OnPolicy",
            "sampling_temperature": 1.0, "enabled_channels": enabled, "logits": list(logits),
            "candidate_kinds": list(kinds), "candidate_labels": [f"candidate-{i}" for i in range(len(kinds))],
            "representative_mask": rep, "motor_masks": motors, "forced_motor_slots": forced,
            "queries": [{"label": "Grab", "candidate_indices": [i for i, kind in enumerate(kinds) if kind == "Interact"][:1]},
                        {"label": "Eat", "candidate_indices": [i for i, kind in enumerate(kinds) if kind == "Interact"][1:]}]}


def document(value=None, temperatures=None):
    return {"schema": audit.INPUT_SCHEMA, "policy_source_commit": audit.SOURCE_COMMIT,
            "metadata": {"note": "synthetic test only"},
            "temperatures": temperatures or [{"label": "sampling", "value": 1.0}],
            "rows": [row() if value is None else value]}


def result(value=None, temperatures=None):
    return audit.audit(document(value, temperatures))["rows"][0]["at_temperatures"][0]


class ProbabilityContracts(unittest.TestCase):
    def test_uniform_decomposition_includes_forcing_and_independent_factor(self):
        report = result()
        self.assertEqual(report["representative_probabilities"], [.25] * 4)
        self.assertEqual(report["representative_forces_other_than_manipulation_probability"], .5)
        grab = report["queries"][0]
        self.assertEqual(grab["representative_forced_probability"], .25)
        self.assertEqual(grab["motor2_conditional_probability"], .5)
        self.assertEqual(grab["independent_contribution_probability"], .25)
        self.assertEqual(grab["manipulation_policy_probability"], .5)
        self.assertEqual(report["manipulation_candidate_probabilities"], [.5, .5, 0, 0])

    def test_disabled_manipulation_can_still_be_forced_by_representative(self):
        report = result(row(enabled=1))
        self.assertEqual(report["independent_motor_probabilities"][2], [0, 0, 0, 0])
        self.assertEqual(report["manipulation_candidate_probabilities"], [.25, .25, 0, 0])
        self.assertEqual(report["manipulation_empty_probability"], .5)
        self.assertEqual(report["manipulation_probability_sum_including_empty"], 1)

    def test_no_manipulation_candidates_is_empty_not_uniform(self):
        report = result(row(kinds=("Move", "Idle")))
        self.assertEqual(report["manipulation_empty_probability"], 1)
        self.assertEqual(report["manipulation_candidate_probabilities"], [0, 0])
        self.assertEqual(report["queries"][0]["manipulation_policy_probability"], 0)

    def test_all_representatives_force_manipulation(self):
        report = result(row(kinds=("Interact", "Write"), logits=[0, 1]))
        self.assertEqual(report["representative_forces_other_than_manipulation_probability"], 0)
        self.assertEqual(report["manipulation_candidate_probabilities"], report["representative_probabilities"])

    def test_idle_gesture_force_posture_without_independent_posture_membership(self):
        value = row(kinds=("Idle", "Gesture", "Rest", "Inspect", "Hold", "Interact"))
        self.assertEqual(value["forced_motor_slots"], [4, 4, 4, 4, 4, 2])
        self.assertEqual(value["motor_masks"][4], 0b11100)
        self.assertEqual(value["motor_masks"][5], 0)
        report = result(value)
        self.assertEqual(report["independent_motor_probabilities"][4], [0, 0, 1/3, 1/3, 1/3, 0])

    def test_every_action_kind_mapping_and_high_enabled_bits(self):
        kinds = tuple(audit.KIND_SLOT)
        value = row(kinds=kinds)
        self.assertEqual(value["forced_motor_slots"], [0, 1, 2, 2, 3, 4, 4, 4, 4, 4])
        self.assertEqual(value["motor_masks"], [1, 2, 12, 16, 224, 0])
        high = result(row(kinds=kinds, enabled=0x80000000))
        # Python 3.12 improves sum() rounding; eight 0.1 weights need not be
        # bit-identical to the decimal literal on every supported Python.
        self.assertAlmostEqual(high["manipulation_empty_probability"], .8, delta=1e-15)

    def test_multiple_targets_are_explicit_group_sum(self):
        value = row(kinds=("Interact", "Interact", "Write", "Move"))
        value["queries"] = [{"label": "caller-defined group", "candidate_indices": [0, 2]}]
        report = result(value)
        self.assertAlmostEqual(report["queries"][0]["manipulation_policy_probability"], 2/3)

    def test_logit_shift_invariance_and_large_finite_logits(self):
        a = result(row(logits=[1000, 1001, 999, 998]))
        b = result(row(logits=[0, 1, -1, -2]))
        self.assertEqual(a["manipulation_candidate_probabilities"], b["manipulation_candidate_probabilities"])
        self.assertTrue(all(math.isfinite(p) for p in a["representative_probabilities"]))

    def test_nonfinite_logits_are_explicitly_excluded_and_raw_bits_preserved(self):
        value = row(kinds=("Interact", "Interact", "Move", "Idle", "Write"),
                    logits=[1, {"f32_bits": "0x7fc00001"}, {"f32_bits": "0x7f800000"},
                            {"f32_bits": "0xff800000"}, {"f32_bits": "0x80000000"}])
        report = audit.audit(document(value))["rows"][0]
        self.assertEqual(report["masked_nonfinite_candidate_indices"], [1, 2, 3])
        self.assertEqual(report["logit_bits"], ["0x3f800000", "0x7fc00001", "0x7f800000", "0xff800000", "0x80000000"])
        self.assertEqual(report["input"]["logits"], value["logits"])
        json.dumps(report, allow_nan=False)

    def test_bit31_and_32_candidate_boundary(self):
        value = row(kinds=("Interact",) * 32)
        self.assertEqual(value["representative_mask"], 0xffffffff)
        self.assertEqual(result(value)["manipulation_candidate_probabilities"][31], 1/32)

    def test_temperatures_are_f32_and_do_not_relabel_counterfactual_as_behavior(self):
        specs = [{"label": "caller-label", "value": .1}, {"label": "counterfactual", "value": 2}]
        value = row(logits=[0, 1, -1, 0])
        value["sampling_temperature"] = .1
        report = audit.audit(document(value, specs))["rows"][0]["at_temperatures"]
        self.assertEqual(report[0]["temperature"], specs[0])
        self.assertEqual(report[0]["effective_temperature_f32"], struct.unpack("<f", struct.pack("<f", .1))[0])
        self.assertEqual(report[0]["interpretation"], "collected_sampling_temperature")
        self.assertEqual(report[1]["interpretation"], "counterfactual_fixed_logit_temperature")
        self.assertGreater(report[1]["representative_probabilities"][0], report[0]["representative_probabilities"][0])

    def test_preserves_row_and_metadata_without_mutating_input(self):
        data = document()
        before = copy.deepcopy(data)
        report = audit.audit(data)
        self.assertEqual(data, before)
        self.assertEqual(report["input_metadata"], before["metadata"])
        self.assertEqual(report["rows"][0]["evidence_status"], "synthetic_fixture")
        data["rows"][0]["evidence_kind"] = "exported_replay"
        self.assertIn("not_independently_verified", audit.audit(data)["rows"][0]["evidence_status"])

    def test_tiny_complement_is_summed_without_one_minus_cancellation(self):
        report = result(row(kinds=("Interact", "Move"), logits=[0, -100]))
        self.assertGreater(report["representative_forces_other_than_manipulation_probability"], 0)
        self.assertEqual(report["representative_forces_other_than_manipulation_probability"], math.exp(-100))

    def test_exact_enumeration_of_all_factorized_joint_actions(self):
        rng = random.Random(87234)
        for case in range(80):
            kinds = ("Interact", "Interact", "Idle", "Move", "Rest", "Look", "Vocalize", "Write")
            value = row(kinds=kinds, logits=[rng.uniform(-5, 5) for _ in kinds], enabled=rng.randrange(1, 64))
            tau = rng.choice([.1, .5, 1, 2, 8])
            report = result(value, [{"label": "enumeration", "value": tau}])
            probabilities = report["independent_motor_probabilities"]
            exact = [0.0] * len(kinds)
            empty = 0.0
            for rep, rep_p in enumerate(report["representative_probabilities"]):
                choices = []
                for slot in range(6):
                    if slot == value["forced_motor_slots"][rep]:
                        choices.append([(rep, 1.0)])
                    else:
                        choices.append([(i, p) for i, p in enumerate(probabilities[slot]) if p]
                                       or [(None, 1.0)])
                for action in itertools.product(*choices):
                    p = rep_p * math.prod(factor_p for _, factor_p in action)
                    selected = action[2][0]
                    if selected is None:
                        empty += p
                    else:
                        exact[selected] += p
            for actual, expected in zip(report["manipulation_candidate_probabilities"], exact):
                self.assertAlmostEqual(actual, expected, places=12, msg=f"case {case}")
            self.assertAlmostEqual(report["manipulation_empty_probability"], empty, places=12)
            self.assertAlmostEqual(report["manipulation_probability_sum_including_empty"], 1, places=12)


class SelectionContracts(unittest.TestCase):
    def selected(self):
        value = row()
        value["selection"] = {"representative_index": 2, "motor_indices": [3, None, 0, None, 2, None]}
        return value

    def test_receipt_factors_and_joint_use_f32_rounding(self):
        report = result(self.selected())["selected_likelihood"]
        self.assertEqual(report["factor_log_probabilities_f32"],
                         [audit.as_f32(-math.log(4), "test"), 0, 0, audit.as_f32(-math.log(2), "test"), 0, 0, 0])
        expected = audit.as_f32(audit.as_f32(-math.log(4), "test") + audit.as_f32(-math.log(2), "test"), "test")
        self.assertEqual(report["policy_joint_log_probability_f32"], expected)
        self.assertEqual(report["behavior_joint_log_probability_f32"], expected)

    def test_demonstrator_never_has_behavior_likelihood(self):
        value = self.selected()
        value["behavior"] = "Demonstrator"
        selected = result(value)["selected_likelihood"]
        self.assertIsNone(selected["behavior_joint_log_probability_f32"])
        self.assertLess(selected["policy_joint_log_probability_f32"], 0)

    def test_counterfactual_never_has_collected_behavior_likelihood(self):
        report = result(self.selected(), [{"label": "tau2", "value": 2}])["selected_likelihood"]
        self.assertIsNone(report["behavior_joint_log_probability_f32"])

    def test_disabled_forced_slot_is_valid_and_has_no_independent_log_factor(self):
        value = row(enabled=1)
        value["selection"] = {"representative_index": 0, "motor_indices": [3, None, 0, None, None, None]}
        self.assertEqual(result(value)["selected_likelihood"]["factor_log_probabilities_f32"][3], 0)

    def test_invalid_selections_are_rejected(self):
        for indices in ([3, None, 0, None, None, None], [3, 1, 0, None, 2, None],
                        [3, None, 3, None, 2, None], [3, None, None, None, 2, None]):
            with self.subTest(indices=indices), self.assertRaises(audit.AuditError):
                value = self.selected()
                value["selection"]["motor_indices"] = indices
                result(value)

    def test_nonfinite_f32_selected_logprob_is_rejected(self):
        value = row(kinds=("Interact", "Interact"), logits=[3e38, -3e38])
        value["selection"] = {"representative_index": 1, "motor_indices": [None, None, 1, None, None, None]}
        with self.assertRaises(audit.AuditError):
            result(value, [{"label": "low", "value": .0001}])

    def test_probability_underflow_does_not_lose_finite_log_likelihood(self):
        value = row(kinds=("Interact", "Interact"), logits=[0, -1000])
        value["selection"] = {"representative_index": 1, "motor_indices": [None, None, 1, None, None, None]}
        report = result(value)
        self.assertEqual(report["representative_probabilities"][1], 0)
        self.assertEqual(report["selected_likelihood"]["policy_joint_log_probability_f32"], -1000)

    def test_multiple_factors_round_each_f32_addition(self):
        kinds = ("Move", "Move", "Look", "Look", "Interact", "Write", "Vocalize", "Vocalize", "Rest", "Hold", "Idle")
        value = row(kinds=kinds, logits=[1, 1, -5, -1, 3, 2, 1, -1, 2, 0, 4])
        value["selection"] = {"representative_index": 10, "motor_indices": [0, 2, 4, 6, 10, None]}
        selected = result(value)["selected_likelihood"]
        self.assertEqual(audit.f32_bits(selected["policy_joint_log_probability_f32"]), 0xc0b801dd)
        single_cast = audit.as_f32(sum(selected["factor_log_probabilities_f32"]), "test")
        self.assertEqual(audit.f32_bits(single_cast), 0xc0b801de)

    def test_joint_overflow_is_deliberately_rejected_even_with_finite_factors(self):
        kinds = ("Move", "Move", "Look", "Look", "Interact", "Write", "Vocalize", "Vocalize", "Rest", "Hold", "Idle")
        value = row(kinds=kinds, logits=[0, -1.4e38, 0, -1.4e38, 0, -1.4e38, 0, -1.4e38, 0, -1.4e38, -1.4e38])
        value["selection"] = {"representative_index": 10, "motor_indices": [1, 3, 5, 7, 10, None]}
        with self.assertRaisesRegex(audit.AuditError, "policy joint log probability"):
            result(value)


class InvalidInputContracts(unittest.TestCase):
    def test_invalid_temperature(self):
        for tau in (0, -1, .00001, float("nan"), float("inf"), 1e99, True, "1", None):
            with self.subTest(tau=tau), self.assertRaises(audit.AuditError):
                result(temperatures=[{"label": "bad", "value": tau}])
        result(temperatures=[{"label": "minimum", "value": .0001}])

    def test_adjacent_f32_values_at_minimum_temperature(self):
        bits = audit.f32_bits(audit.as_f32(.0001, "test"))
        below = struct.unpack("<f", struct.pack("<I", bits - 1))[0]
        above = struct.unpack("<f", struct.pack("<I", bits + 1))[0]
        with self.assertRaises(audit.AuditError):
            result(temperatures=[{"label": "below", "value": below}])
        result(temperatures=[{"label": "above", "value": above}])

    def test_bad_source_schema_or_top_level_structure(self):
        for change in ({"schema": "native-replay"}, {"policy_source_commit": "unknown"},
                       {"rows": []}, {"temperatures": []}, {"rows": [None]},
                       {"temperatures": [None]}, {"temperatures": [{"label": "x", "value": 1}] * 2}):
            with self.subTest(change=change), self.assertRaises(audit.AuditError):
                audit.audit({**document(), **change})

    def test_bad_candidates_masks_forcing_and_semantics(self):
        mutations = [{"logits": []}, {"logits": [0] * 33}, {"logits": [0] * 3},
                     {"candidate_kinds": ["Grab"] * 4}, {"candidate_labels": [""] * 4},
                     {"representative_mask": 0}, {"representative_mask": 0x10f},
                     {"motor_masks": [0] * 6}, {"motor_masks": [0] * 5},
                     {"forced_motor_slots": [2, 2, 5, 0]}, {"forced_motor_slots": [2] * 3},
                     {"enabled_channels": 0}, {"enabled_channels": True},
                     {"logits": [None, 0, 0, 0]}, {"logits": [float("nan"), 0, 0, 0]},
                     {"logits": [{"f32_bits": "0xgarbage!"}, 0, 0, 0]},
                     {"logits": [{"f32_bits": 0, "extra": 0}, 0, 0, 0]},
                     {"behavior": "unknown"}, {"evidence_kind": "actual_verified_run"},
                     {"sampling_temperature": 0}]
        for change in mutations:
            with self.subTest(change=change), self.assertRaises(audit.AuditError):
                result({**row(), **change})

    def test_all_nonfinite_is_invalid_not_empty_policy(self):
        value = row(kinds=("Interact",), logits=[{"f32_bits": "0x7fc00001"}])
        with self.assertRaises(audit.AuditError):
            result(value)

    def test_bit_strings_require_exactly_eight_hex_digits(self):
        for text in ("0x+0000001", "0x000_0001", "0x 0000001", "0x0000001 ",
                     "0x0000001", "0x000000001", "0X00000001"):
            with self.subTest(text=text), self.assertRaises(audit.AuditError):
                audit.parse_logit({"f32_bits": text}, "test")
        self.assertEqual(audit.parse_logit({"f32_bits": "0x7fC00001"}, "test")[1], 0x7fc00001)

    def test_query_indices_must_be_unique_manipulation_candidates(self):
        for indices in ([0, 0], [4], [-1], [2], [True], "0"):
            with self.subTest(indices=indices), self.assertRaises(audit.AuditError):
                value = row()
                value["queries"] = [{"label": "invalid", "candidate_indices": indices}]
                result(value)

    def test_duplicate_row_ids_or_queries_are_rejected(self):
        data = document()
        data["rows"] *= 2
        with self.assertRaises(audit.AuditError):
            audit.audit(data)
        value = row()
        value["queries"] *= 2
        with self.assertRaises(audit.AuditError):
            result(value)


class CliContracts(unittest.TestCase):
    def invoke(self, args):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = audit.main(args)
        return code, out.getvalue(), err.getvalue()

    def test_stdout_and_new_output_preserve_input_and_bind_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "input.json"
            source.write_text(json.dumps(document()), encoding="utf-8")
            before = source.read_bytes()
            code, out, err = self.invoke([str(source)])
            self.assertEqual((code, err), (0, ""))
            report = json.loads(out)
            self.assertEqual(report["input_sha256"], audit.hashlib.sha256(before).hexdigest())
            output = Path(directory) / "report.json"
            self.assertEqual(self.invoke([str(source), "--output", str(output)])[0], 0)
            self.assertEqual(json.loads(output.read_text()), report)
            self.assertEqual(source.read_bytes(), before)
            self.assertEqual(self.invoke([str(source), "--output", str(output)])[0], 2)
            self.assertEqual(self.invoke([str(source), "--output", str(source)])[0], 2)
            self.assertEqual(source.read_bytes(), before)

    def test_nonstandard_constants_duplicate_keys_and_bad_json_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "bad.json"
            output = Path(directory) / "report.json"
            for raw in ('{"x": NaN}', '{"x": 0, "x": 1}', "{", "\ufeff{}"):
                with self.subTest(raw=raw):
                    source.write_text(raw, encoding="utf-8")
                    code, out, err = self.invoke([str(source), "--output", str(output)])
                    self.assertEqual(code, 2)
                    self.assertEqual(out, "")
                    self.assertIn("audit error:", err)
                    self.assertFalse(output.exists())

    def test_fixture_cli_is_synthetic_and_runs_without_external_dependencies(self):
        fixture = Path(__file__).parent.parent / "fixtures/manipulation_probabilities.synthetic.json"
        code, out, err = self.invoke([str(fixture)])
        self.assertEqual((code, err), (0, ""))
        report = json.loads(out)
        self.assertTrue(all(r["evidence_status"] == "synthetic_fixture" for r in report["rows"]))


if __name__ == "__main__":
    unittest.main()
