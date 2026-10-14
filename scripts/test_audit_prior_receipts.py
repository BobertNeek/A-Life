"""Synthetic JSON fixtures only: no learner, provider, GPU, or checkpoint writes."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import audit_prior_receipts as audit


def prior(statuses=("delivered", "delivered")):
    delivered = statuses.count("delivered")
    return {"backend": "synthetic", "provider_identity": "fixture-provider",
            "model": "fixture-model", "source_provider_identity": None,
            "decision_frames": len(statuses), "decision_frames_with_prior": delivered,
            "decision_frames_without_prior": len(statuses) - delivered,
            "decision_inputs": [{"organism": 1, "sequence": index + 1, "tick": index + 10,
                                 "status": status, "nonzero_prior_lanes": int(status == "delivered"),
                                 "nonzero_encoded_prior_lanes": int(status == "delivered")}
                                for index, status in enumerate(statuses)]}


def token(token_id, position, utterance=7, source="Teacher", entity=3, channel="Hearing", addressee=1):
    return {"utterance_id": utterance, "sequence_position": position, "source_kind": source,
            "source_entity": entity, "speaker_id": None, "addressee": addressee, "token_id": token_id,
            "source_position": {"x": 0, "y": 0, "z": 0}, "confidence": 1,
            "teacher_channel": channel}


def step(*tokens):
    return {"tick": 10, "heard_tokens": list(tokens) + [None] * (16 - len(tokens))}


def fixture(directory, semantic_prior=None, steps=None):
    semantic_prior = prior() if semantic_prior is None else semantic_prior
    steps = [step(token(9, 0), token(1, 1)), step()] if steps is None else steps
    for index, value in enumerate(steps):
        value["tick"] = index + 10
    pilot = {"lesson": "grab_food", "teacher_mode": False, "ticks": len(steps),
             "grab_food_setup": {"organism": 1},
             "semantic_prior": semantic_prior, "teacher_cue_tokens": [9, 1], "teacher_cue_frames": 1}
    write(directory, "pilot.json", pilot)
    write(directory, "semantic-prior-readiness.json", {"lesson": "grab_food",
          "failure_policy": audit.FAILURE_POLICY, "semantic_prior": semantic_prior})
    write(directory, "lesson-trace.json", {"steps": steps, "teacher_cue_tokens": [9, 1]})
    return pilot


def write(directory, name, value):
    (directory / name).write_text(json.dumps(value), encoding="utf-8")


class PriorReceiptAuditTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)

    def test_complete_delivery_and_same_utterance_cue(self):
        fixture(self.directory)
        result = audit.audit(self.directory)
        self.assertEqual(result["warnings"], [])
        self.assertEqual(result["declared_provider"]["backend"], "synthetic")
        self.assertEqual(result["declared_provider"]["provider_identity"], "fixture-provider")
        self.assertEqual(result["decision_inputs"]["confirmed_delivered_encoded"], 2)
        self.assertTrue(result["decision_inputs"]["coverage_complete"])
        self.assertEqual(result["teacher_cues"]["full_ordered_cue_frames"], 1)
        self.assertEqual(result["teacher_cues"]["evidence"][0]["tokens"], [9, 1])

    def test_missing_receipts_are_unknown_not_zero(self):
        result = audit.audit(self.directory)
        self.assertEqual(set(result["files"].values()), {"missing"})
        self.assertEqual(result["decision_totals"], dict.fromkeys(audit.COUNT_KEYS))
        self.assertIsNone(result["decision_inputs"]["confirmed_delivered_encoded"])
        self.assertIsNone(result["teacher_cues"]["full_ordered_cue_frames"])
        self.assertTrue(result["warnings"])

    def test_measured_zero_is_preserved(self):
        fixture(self.directory, prior(()), steps=[])
        result = audit.audit(self.directory)
        self.assertEqual(result["decision_totals"], dict.fromkeys(audit.COUNT_KEYS, 0))
        self.assertEqual(result["decision_inputs"]["confirmed_delivered_encoded"], 0)
        self.assertTrue(result["decision_inputs"]["coverage_complete"])
        self.assertEqual(result["teacher_cues"]["full_ordered_cue_frames"], 0)

    def test_missing_totals_are_not_reconstructed_from_ticks_or_rows(self):
        value = prior()
        for key in audit.COUNT_KEYS:
            del value[key]
        fixture(self.directory, value)
        result = audit.audit(self.directory)
        self.assertEqual(result["decision_totals"], dict.fromkeys(audit.COUNT_KEYS))
        self.assertEqual(result["decision_inputs"]["confirmed_delivered_encoded"], 2)
        self.assertFalse(result["decision_inputs"]["coverage_complete"])

    def test_invalid_counts_are_unknown(self):
        for value in (-1, True, "2", {}, 2**64):
            with self.subTest(value=value):
                data = prior()
                data["decision_frames"] = value
                fixture(self.directory, data)
                self.assertIsNone(audit.audit(self.directory)["decision_totals"]["decision_frames"])

    def test_missing_failed_and_dropout_remain_diagnostics(self):
        statuses = ("unavailable", "provider_failure", "pending", "deliberate_dropout",
                    "no_semantic_encoder", "empty_hints", "unsupported_profile", "developmental_gain_zero")
        fixture(self.directory, prior(statuses))
        result = audit.audit(self.directory)
        self.assertEqual(result["decision_inputs"]["missing_or_failed"], len(statuses))
        self.assertEqual(result["decision_inputs"]["confirmed_delivered_encoded"], 0)
        self.assertTrue(result["decision_inputs"]["coverage_complete"])
        self.assertTrue(any("diagnostic" in warning for warning in result["warnings"]))

    def test_provider_failure_history_is_retained_even_when_recent_inputs_delivered(self):
        data = prior()
        data.update(failures=1, prime_timeouts=2, dropout_frames=0, empty_hint_frames=0,
                    provider_failures=[{"error": "synthetic provider unavailable"}],
                    last_error="synthetic provider unavailable")
        fixture(self.directory, data)
        result = audit.audit(self.directory)
        self.assertEqual(result["provider_events"]["failures"], 1)
        self.assertEqual(result["provider_events"]["prime_timeouts"], 2)
        self.assertEqual(result["provider_events"]["dropout_frames"], 0)
        self.assertEqual(result["provider_events"]["provider_failure_rows"], 1)
        self.assertTrue(result["warnings"])
        self.assertEqual(result["decision_inputs"]["confirmed_delivered_encoded"], 2)

    def test_delivered_label_requires_actual_encoded_nonzero_lanes(self):
        for key in ("nonzero_prior_lanes", "nonzero_encoded_prior_lanes"):
            for value in (0, None, True, "1"):
                with self.subTest(key=key, value=value):
                    data = prior()
                    data["decision_inputs"][0][key] = value
                    fixture(self.directory, data)
                    result = audit.audit(self.directory)["decision_inputs"]
                    self.assertEqual(result["confirmed_delivered_encoded"], 1)
                    self.assertEqual(result["unknown_rows"], 1)

    def test_invalid_and_duplicate_decision_identities_cannot_confirm_delivery(self):
        for change in ({"organism": []}, {"organism": 0}, {"sequence": {}}, {"sequence": 0},
                       {"tick": True}, {"sequence": 2}):
            with self.subTest(change=change):
                data = prior()
                data["decision_inputs"][0].update(change)
                fixture(self.directory, data)
                result = audit.audit(self.directory)["decision_inputs"]
                self.assertGreater(result["unknown_rows"], 0)
                self.assertFalse(result["coverage_complete"])

    def test_duplicate_pair_is_not_hidden_by_invalid_tick(self):
        data = prior()
        data["decision_inputs"][0].update(sequence=2, tick=None)
        fixture(self.directory, data)
        result = audit.audit(self.directory)["decision_inputs"]
        self.assertEqual(result["confirmed_delivered_encoded"], 0)
        self.assertEqual(result["unknown_rows"], 2)

    def test_non_delivered_status_cannot_contradict_encoded_delivery(self):
        data = prior()
        data.update(decision_frames_with_prior=1, decision_frames_without_prior=1)
        data["decision_inputs"][0]["status"] = "unavailable"
        fixture(self.directory, data)
        result = audit.audit(self.directory)["decision_inputs"]
        self.assertEqual(result["missing_or_failed"], 0)
        self.assertEqual(result["unknown_rows"], 1)
        self.assertFalse(result["coverage_complete"])

    def test_invalid_rows_and_statuses_warn_without_crashing(self):
        for row in (None, [], "bad row", {"status": []}, {"organism": 1, "sequence": 1,
                                                      "tick": 1, "status": "future_status"}):
            with self.subTest(row=row):
                data = prior()
                data["decision_inputs"][0] = row
                fixture(self.directory, data)
                self.assertEqual(audit.audit(self.directory)["decision_inputs"]["unknown_rows"], 1)

    def test_bounded_trace_is_not_lifetime_coverage(self):
        data = prior()
        data.update(decision_frames=300, decision_frames_with_prior=300)
        fixture(self.directory, data)
        result = audit.audit(self.directory)
        self.assertEqual(result["decision_totals"]["decision_frames"], 300)
        self.assertEqual(result["decision_inputs"]["observed_rows"], 2)
        self.assertFalse(result["decision_inputs"]["coverage_complete"])
        self.assertTrue(any("256" in warning for warning in result["warnings"]))

    def test_readiness_fallback_is_explicit_and_disagreement_is_not_merged(self):
        pilot = fixture(self.directory)
        pilot["semantic_prior"] = None
        write(self.directory, "pilot.json", pilot)
        result = audit.audit(self.directory)
        self.assertEqual(result["semantic_prior_source"], "semantic-prior-readiness.json")
        pilot["semantic_prior"] = prior()
        pilot["semantic_prior"]["backend"] = "different"
        write(self.directory, "pilot.json", pilot)
        result = audit.audit(self.directory)
        self.assertEqual(result["declared_provider"]["backend"], "different")
        self.assertTrue(any("without merging" in warning for warning in result["warnings"]))

    def test_any_token_headline_does_not_prove_full_cue(self):
        for tokens in ((token(1, 1),), (token(9, 0), token(1, 1, utterance=8)),
                       (token(1, 0), token(9, 1)),
                       (token(9, 0, source="Player"), token(1, 1, source="Player"))):
            with self.subTest(tokens=tokens):
                fixture(self.directory, steps=[step(*tokens)])
                result = audit.audit(self.directory)["teacher_cues"]
                self.assertEqual(result["declared_any_token_frames"], 1)
                self.assertEqual(result["full_ordered_cue_frames"], 0)
                self.assertEqual(result["evidence"], [])

    def test_native_positions_not_array_order_establish_cue_order(self):
        fixture(self.directory, steps=[step(token(1, 1), token(9, 0))])
        self.assertEqual(audit.audit(self.directory)["teacher_cues"]["full_ordered_cue_frames"], 1)

    def test_vision_and_unknown_channel_are_separate_from_hearing(self):
        for channel, full_count in (("Vision", 0), (None, None)):
            with self.subTest(channel=channel):
                fixture(self.directory, steps=[step(token(9, 0, channel=channel), token(1, 1, channel=channel))])
                result = audit.audit(self.directory)["teacher_cues"]
                self.assertEqual(result["generic_teacher_cue_frames"], 1)
                self.assertEqual(result["full_ordered_cue_frames"], full_count)
                self.assertEqual(result["evidence"], [])
                self.assertEqual(len(result["generic_teacher_evidence"]), 1)

    def test_other_addressee_does_not_count_as_lesson_hearing(self):
        fixture(self.directory, steps=[step(token(9, 0, addressee=2), token(1, 1, addressee=2))])
        result = audit.audit(self.directory)["teacher_cues"]
        self.assertEqual(result["expected_learner"], 1)
        self.assertEqual(result["full_ordered_cue_frames"], 0)
        self.assertEqual(result["generic_teacher_evidence"][0]["recipient"], "other_learner")

    def test_broadcast_hearing_preserves_exposure_without_claiming_directed_request(self):
        fixture(self.directory, steps=[step(token(9, 0, addressee=None), token(1, 1, addressee=None))])
        result = audit.audit(self.directory)["teacher_cues"]
        self.assertEqual(result["full_ordered_cue_frames"], 1)
        self.assertEqual(result["evidence"][0]["recipient"], "broadcast")
        self.assertFalse(result["evidence"][0]["learner_directed"])

    def test_missing_addressee_is_unknown(self):
        tokens = [token(9, 0), token(1, 1)]
        del tokens[0]["addressee"]
        fixture(self.directory, steps=[step(*tokens)])
        result = audit.audit(self.directory)["teacher_cues"]
        self.assertIsNone(result["full_ordered_cue_frames"])
        self.assertEqual(result["evidence"], [])

    def test_multiple_utterances_count_one_frame_and_retain_both_evidence_rows(self):
        fixture(self.directory, steps=[step(token(9, 0), token(1, 1),
                                          token(9, 0, utterance=8), token(1, 1, utterance=8))])
        result = audit.audit(self.directory)["teacher_cues"]
        self.assertEqual(result["full_ordered_cue_frames"], 1)
        self.assertEqual(len(result["evidence"]), 2)

    def test_malformed_duplicate_or_mixed_teacher_tokens_are_unknown(self):
        for tokens in ((token(9, 0), token(1, 1), token(1, 1)),
                       (token(9, 0), token(1, 1, entity=4)),
                       (token(9, 0), {**token(1, 1), "utterance_id": []}),
                       (token(9, 0), token(1, 1, source="unknown")),
                       (token(9, 0), token(1, 1), token(1, 1, source="Player"))):
            with self.subTest(tokens=tokens):
                fixture(self.directory, steps=[step(*tokens)])
                result = audit.audit(self.directory)["teacher_cues"]
                self.assertIsNone(result["full_ordered_cue_frames"])
                self.assertEqual(result["evidence"], [])

    def test_duplicate_trace_ticks_do_not_establish_full_episode_count(self):
        fixture(self.directory)
        trace = {"steps": [step(token(9, 0), token(1, 1)), step(token(9, 0), token(1, 1))]}
        write(self.directory, "lesson-trace.json", trace)
        self.assertIsNone(audit.audit(self.directory)["teacher_cues"]["full_ordered_cue_frames"])

    def test_partial_heard_fields_or_shortened_trace_do_not_claim_total(self):
        fixture(self.directory, steps=[step(token(9, 0), token(1, 1)), {"tick": 11}])
        result = audit.audit(self.directory)["teacher_cues"]
        self.assertEqual(result["observed_full_ordered_cue_frames"], 1)
        self.assertIsNone(result["full_ordered_cue_frames"])
        fixture(self.directory)
        write(self.directory, "lesson-trace.json", {"steps": [step(token(9, 0), token(1, 1))]})
        self.assertIsNone(audit.audit(self.directory)["teacher_cues"]["full_ordered_cue_frames"])

    def test_cue_configuration_disagreement_or_missing_is_unknown(self):
        fixture(self.directory)
        write(self.directory, "lesson-trace.json", {"steps": [step()], "teacher_cue_tokens": [8, 1]})
        self.assertIsNone(audit.audit(self.directory)["teacher_cues"]["full_ordered_cue_frames"])
        write(self.directory, "pilot.json", {"ticks": 1})
        write(self.directory, "lesson-trace.json", {"steps": [step()]})
        self.assertIsNone(audit.audit(self.directory)["teacher_cues"]["configured_tokens"])

    def test_failed_lesson_diagnostic_fallback(self):
        fixture(self.directory)
        (self.directory / "lesson-trace.json").rename(self.directory / "lesson-diagnostic.json")
        result = audit.audit(self.directory)
        self.assertEqual(result["files"]["lesson-diagnostic.json"], "present")
        self.assertEqual(result["teacher_cues"]["full_ordered_cue_frames"], 1)

    def test_invalid_json_is_unknown_and_other_evidence_is_kept(self):
        for content in ('{"ticks":NaN}', '{"teacher_channel":1e999}', '{"ticks":1,"ticks":2}', '[]', '{',
                        '{"x":' + '[' * 10000 + '0' + ']' * 10000 + '}'):
            with self.subTest(content=content):
                fixture(self.directory)
                (self.directory / "pilot.json").write_text(content, encoding="utf-8")
                result = audit.audit(self.directory)
                self.assertEqual(result["files"]["pilot.json"], "unreadable")
                self.assertEqual(result["semantic_prior_source"], "semantic-prior-readiness.json")

    def test_cli_multiple_inputs_is_json_and_leaves_all_inputs_unchanged(self):
        fixture(self.directory)
        protected = ("trained.alife-foundation", "actor-checkpoint.json", "value-checkpoint.json")
        for name in protected:
            (self.directory / name).write_bytes(b"protected unrelated checkpoint")
        def hashes():
            return {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                    for path in self.directory.iterdir()}
        before = hashes()
        process = subprocess.run([sys.executable, str(Path(audit.__file__)), str(self.directory),
                                  str(self.directory / "pilot.json")], check=True, capture_output=True, text=True)
        self.assertEqual(process.stderr, "")
        result = json.loads(process.stdout)
        self.assertEqual(result["failure_policy"], audit.FAILURE_POLICY)
        self.assertEqual(len(result["audits"]), 2)
        self.assertEqual(result["audits"][0], result["audits"][1])
        self.assertEqual(hashes(), before)


if __name__ == "__main__":
    unittest.main()
