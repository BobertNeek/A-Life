import copy
import hashlib
import json
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest

from compare_foundation_probes import compare, InvalidComparisonEvidence


class ComparisonIdentityTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.binary = "b" * 64
        self.policies = {}
        for branch, byte in (("control", 1), ("candidate", 2)):
            directory = self.root / branch
            directory.mkdir()
            payload = b"fixture-canonical-policy" + bytes([byte]) * 32
            (directory / "initial.alife-foundation").write_bytes(payload)
            self.policies[branch] = hashlib.sha256(payload).hexdigest()
            sampling = dict(seed=7, counter=0, temperature=1.0, demonstrator=None)
            pilot = dict(seed=7, founder_seed_base=99, lesson="grab_food", teacher_mode=False,
                         initial_sampling=sampling, silent_request=False, request_target=None,
                         vocabulary_token=None, teacher_cue_tokens=[9, 1],
                         grab_food_setup=dict(body_position=[0, 0, 0], food_position=[1, 0, 0]),
                         held_food_setup=None, source_asset_digest=(bytes([byte]) * 32).hex(),
                         initial_world_signature=f"full-world-including-policy-{byte}",
                         initial_comparison_identity=dict(schema_version=1, exclusion="subject_n2048_policy_weights_v1",
                             world=dict(schema_version=1, subject=1, words=[11, 22, 33, 44]),
                             policy_asset_digest=[byte] * 32, policy_sha256=self.policies[branch],
                             executable_sha256=self.binary))
            first = dict(organism_id=1, tick=0, decision_tick=0, sequence_id=1, dispatch_generation=1, frame_digest=[3]*4,
                         lesson_target=3, lesson_target_held_before_decision=False,
                         lesson_target_ownership_before_decision=dict(owner=None, consumed=False), representative_mask=7,
                         motor_masks=[0] * 6, sampling=sampling,
                         physiology=dict(before_energy=1.0, before_brain_atp=1.0, before_fatigue=0.0,
                             before_health=1.0, before_hunger=0.1, before_injury=0.0, before_pain=0.0,
                             after_energy=0.9), chosen_action=101 if byte==1 else 211)
            first.update(json.loads((Path(__file__).parent / "testdata/foundation_probe_sensory_v1.json").read_text()))
            first["candidates"][0]["logit"] += float(byte)
            self.write(branch, "pilot.json", pilot)
            self.write(branch, "lesson-diagnostic.json", dict(steps=[first]))

    def write(self, branch, name, value):
        (self.root / branch / name).write_text(json.dumps(value), encoding="utf-8")

    def mutate(self, branch, name, function):
        value = json.loads((self.root / branch / name).read_text(encoding="utf-8"))
        function(value)
        self.write(branch, name, value)

    def compare(self):
        return compare(self.root / "control", self.root / "candidate",
                       control_policy_sha256=self.policies["control"],
                       candidate_policy_sha256=self.policies["candidate"], executable_sha256=self.binary)

    def paired(self, change, *, accepted=False):
        originals = {branch: json.loads((self.root / branch / "lesson-diagnostic.json").read_text())
                     for branch in ("control", "candidate")}
        try:
            for branch, original in originals.items():
                data = copy.deepcopy(original)
                change(data["steps"][0])
                self.write(branch, "lesson-diagnostic.json", data)
            if accepted:
                self.assertTrue(self.compare()["matched"])
            else:
                with self.assertRaises(InvalidComparisonEvidence):
                    self.compare()
        finally:
            for branch, original in originals.items():
                self.write(branch, "lesson-diagnostic.json", original)

    @staticmethod
    def token():
        return dict(utterance_id=1, sequence_position=0, source_kind="Teacher", speaker_id=None,
                    addressee=1, source_entity=2, token_id=9, source_position=dict(x=-2.0, y=0.0, z=3.0),
                    confidence=0.75, teacher_channel="Hearing")

    @staticmethod
    def semantic():
        return dict(feature_flags=2**32-1, confidence=0.5,
                    compressed_codes=[dict(codebook_id=1, code=0, salience=1.0)],
                    salience=[dict(concept_id=1, salience=0.0)])

    def test_reported_paired_position_string_is_rejected(self):
        self.paired(lambda row: row.__setitem__("position", "bad"))

    def test_paired_pose_shapes_and_numeric_substitutes_are_rejected(self):
        for field, values in (
            ("position", [None, [], [0, 0, 0], {}, dict(x=0, y=0), dict(x=0, y=0, z=0, extra=1),
                          dict(x=True, y=0, z=0), dict(x="0", y=0, z=0), dict(x=1e300, y=0, z=0),
                          dict(x=float("nan"), y=0, z=0), dict(x=float("inf"), y=0, z=0)]),
            ("gaze_rotation", [0, "bad", [0, 0, 0, 1], dict(x=0, y=0, z=0),
                               dict(x=0, y=0, z=0, w=0), dict(x=0, y=0, z=0, w=True),
                               dict(x=0, y=0, z=0, w=1e300)]),
        ):
            for value in values:
                with self.subTest(field=field, value=value):
                    self.paired(lambda row: row.__setitem__(field, value))

    def test_paired_terrain_dimensions_ranges_and_grip_are_rejected(self):
        for field, values in (
            ("terrain_ranges", [None, "bad", {}, [], [0]*15, [0]*17, [True]*16,
                                [-0.1]*16, [1.1]*16, [float("nan")]*16, [float("inf")]*16]),
            ("global_grip_before_decision", [None, True, "0", [], -0.1, 1.1, float("nan")]),
        ):
            for value in values:
                with self.subTest(field=field, value=value):
                    self.paired(lambda row: row.__setitem__(field, value))

    def test_paired_heard_token_shapes_and_native_ranges_are_rejected(self):
        for value in (None, "bad", [], [None]*15, [None]*17, [False]+[None]*15,
                      [dict(token_id=1)]+[None]*15):
            with self.subTest(outer=value):
                self.paired(lambda row: row.__setitem__("heard_tokens", value))
        for key, value in (("utterance_id", 0), ("utterance_id", 2**64),
                           ("sequence_position", True), ("sequence_position", 16),
                           ("source_kind", "teacher"), ("source_kind", []),
                           ("speaker_id", 0), ("addressee", True), ("source_entity", "2"),
                           ("token_id", 0), ("token_id", 256), ("token_id", True),
                           ("source_position", [0, 0, 0]), ("confidence", -0.1),
                           ("confidence", 1.1), ("teacher_channel", "Unknown")):
            with self.subTest(key=key, value=value):
                token = dict(self.token(), **{key: value})
                self.paired(lambda row: row.__setitem__("heard_tokens", [token]+[None]*15))
        for token in (dict(self.token(), extra=1), {k:v for k,v in self.token().items() if k != "confidence"}):
            self.paired(lambda row: row.__setitem__("heard_tokens", [token]+[None]*15))

    def test_paired_semantic_records_and_nested_ranges_are_rejected(self):
        for value in ("bad", [], 0, {}, dict(self.semantic(), confidence=True),
                      dict(self.semantic(), feature_flags=2**32), dict(self.semantic(), compressed_codes="bad"),
                      dict(self.semantic(), salience={}), dict(self.semantic(), extra=1)):
            with self.subTest(value=value):
                self.paired(lambda row: row.__setitem__("semantic_context", value))
        for key, value in (("codebook_id", 0), ("codebook_id", 2**16), ("code", -1),
                           ("code", 2**32), ("salience", 1.1), ("salience", True)):
            context = self.semantic()
            context["compressed_codes"][0][key] = value
            with self.subTest(code_key=key, value=value):
                self.paired(lambda row: row.__setitem__("semantic_context", context))
        for key, value in (("concept_id", 0), ("concept_id", 2**64), ("salience", -0.1)):
            context = self.semantic()
            context["salience"][0][key] = value
            self.paired(lambda row: row.__setitem__("semantic_context", context))

    def test_paired_object_slot_shapes_ranges_and_references_are_rejected(self):
        source = json.loads((self.root/"control/lesson-diagnostic.json").read_text())["steps"][0]["object_slots"][0]
        for value in (None, "bad", {}, [dict(id=9)], [dict(source, slot_index=i) for i in range(17)]):
            self.paired(lambda row: row.__setitem__("object_slots", value))
        for key, value in (("slot_index", 1), ("slot_index", True), ("tracked_object_id", 0),
                           ("tracked_object_id", 2**64), ("distance", -0.1), ("contact", 1.1),
                           ("confidence", True), ("temperature", -1.1), ("temperature", 1.1)):
            with self.subTest(key=key, value=value):
                self.paired(lambda row: row.__setitem__("object_slots", [dict(source, **{key:value})]))
        for key, length, lower in (("bearing", 2, -1), ("relative_velocity", 3, -1),
                                   ("chemical", 3, -1), ("proprioception", 2, -1),
                                   ("color", 3, 0), ("material", 3, 0), ("shape", 3, 0), ("terrain", 2, 0)):
            for value in ([], [0]*(length-1), [True]*length, [lower-0.1]*length, [1.1]*length):
                with self.subTest(key=key, value=value):
                    self.paired(lambda row: row.__setitem__("object_slots", [dict(source, **{key:value})]))
        self.paired(lambda row: row.__setitem__("object_slots", [dict(source, extra=1)]))
        self.paired(lambda row: row.__setitem__("object_slots", []))  # existing slot references become invalid

    def test_paired_candidate_schemas_ranges_and_references_are_rejected(self):
        for value in (None, [], "bad"):
            self.paired(lambda row: row.__setitem__("candidates", value))
        for key, value in (("index", True), ("index", 1), ("action", 0), ("action", 2**32),
                           ("family", "manipulation"), ("family", []), ("observation", None),
                           ("observation", {"ObjectSlot":1}), ("observation", {"ObjectSlot":True}),
                           ("observation", {"ObjectSlot":0, "extra":1}), ("target", 0),
                           ("target", True), ("target_position", [0,0,0]), ("logit", float("inf"))):
            with self.subTest(key=key, value=value):
                self.paired(lambda row: row["candidates"][0].__setitem__(key, value))

    def test_paired_held_target_types_and_ownership_must_be_native(self):
        for field, value in (("lesson_target", 0), ("lesson_target", True),
                             ("lesson_target_held_before_decision", 0),
                             ("lesson_target_ownership_before_decision", {}),
                             ("lesson_target_ownership_before_decision", dict(owner=0, consumed=False)),
                             ("lesson_target_ownership_before_decision", dict(owner=None, consumed=0)),
                             ("lesson_target_ownership_before_decision", dict(owner=1, consumed=False)),
                             ("lesson_target_ownership_before_decision", None)):
            with self.subTest(field=field, value=value):
                self.paired(lambda row: row.__setitem__(field, value))

    def test_native_valid_optional_and_boundary_values_still_match_policy_outputs(self):
        def native_values(row):
            row["position"] = dict(x=-1000., y=2000., z=-3.5)
            row["gaze_rotation"] = dict(x=0., y=0., z=0., w=2.)
            row["terrain_ranges"] = [0., 1.]*8
            row["global_grip_before_decision"] = 1.
            token = self.token()
            token.update(sequence_position=15, token_id=255, utterance_id=2**64-1)
            row["heard_tokens"] = [None]*7 + [token] + [None]*8
            row["semantic_context"] = self.semantic()
            row["semantic_context"]["compressed_codes"] *= 33  # no invented native Vec cap
            row["object_slots"][0]["chemical"] = [-1., 0., 1.]
            row["object_slots"][0]["terrain"] = [0., 1.]
        self.paired(native_values, accepted=True)

    def test_native_empty_slots_absent_prior_and_missing_target_are_valid(self):
        def native_values(row):
            row["object_slots"] = []
            row["semantic_context"] = None
            row["candidates"] = [dict(index=0,action=1,family="Idle",observation="None",target=None,target_position=None,logit=0.)]
            row["representative_mask"] = 1
            row["lesson_target_ownership_before_decision"] = None
            row["lesson_target_held_before_decision"] = None
        self.paired(native_values, accepted=True)

    def test_native_f32_conversion_rejects_underflowed_zero_quaternion(self):
        self.paired(lambda row: row.__setitem__("gaze_rotation", dict(x=1e-300,y=0,z=0,w=0)))

    def test_native_f32_boundary_decimals_remain_valid_without_rewriting_evidence(self):
        def native_values(row):
            row["position"] = dict(x=3.4028235e38, y=-3.4028235e38, z=1e-45)
            row["gaze_rotation"] = dict(x=1e-45, y=0, z=0, w=0)
        self.paired(native_values, accepted=True)
        # The validator uses f32 conversion for legality, but hashes originals.
        from foundation_probe_schema import validate_sensory_context
        value = json.loads((self.root/"control/lesson-diagnostic.json").read_text())["steps"][0]
        native_values(value)
        before = copy.deepcopy(value)
        validate_sensory_context(value, value["candidates"])
        self.assertEqual(before, value)

    def test_policy_weights_derived_digests_and_scores_may_differ(self):
        before = {p: p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        result = self.compare()
        self.assertTrue(result["matched"])
        self.assertNotEqual(result["control"]["policy_sha256"], result["candidate"]["policy_sha256"])
        self.assertEqual(before, {p: p.read_bytes() for p in before})
        self.assertFalse(result["neural_execution"])

    def test_world_physiology_and_context_differences_are_rejected(self):
        path = self.root / "candidate" / "lesson-diagnostic.json"
        original = json.loads(path.read_text())
        semantic = dict(feature_flags=0, confidence=1.0, compressed_codes=[], salience=[])
        for key, changed in (("position", dict(x=1, y=0, z=0)),
                             ("semantic_context", semantic),
                             ("object_slots", [dict(original["steps"][0]["object_slots"][0], tracked_object_id=10)]),
                             ("terrain_ranges", [1] * 16), ("physiology", dict(original["steps"][0]["physiology"], before_energy=0.8)),
                             ("representative_mask", 3), ("motor_masks", [1] * 6),
                             ("lesson_target", 4), ("sequence_id", 2)):
            with self.subTest(key=key):
                data = copy.deepcopy(original)
                data["steps"][0][key] = changed
                self.write("candidate", path.name, data)
                self.assertFalse(self.compare()["matched"])
        self.write("candidate", path.name, original)
        self.mutate("candidate", "pilot.json", lambda p: p["initial_comparison_identity"]["world"]["words"].__setitem__(0, 12))
        self.assertIn("world", self.compare()["mismatched_components"])

    def test_setup_seed_teacher_and_cue_provenance_differences_rejected(self):
        path = self.root / "candidate" / "pilot.json"
        original = json.loads(path.read_text())
        for key, changed in (("seed", 8), ("founder_seed_base", 100), ("teacher_mode", True),
                             ("teacher_cue_tokens", [1]), ("silent_request", True),
                             ("request_target", 3), ("grab_food_setup", dict(body_position=[2, 0, 0]))):
            with self.subTest(key=key):
                data = copy.deepcopy(original)
                data[key] = changed
                self.write("candidate", path.name, data)
                self.assertFalse(self.compare()["matched"])

    def test_sampler_provenance_and_first_decision_must_agree(self):
        self.mutate("candidate", "pilot.json", lambda p: p["initial_sampling"].__setitem__("temperature", 2.0))
        with self.assertRaisesRegex(InvalidComparisonEvidence, "sampler"):
            self.compare()
        self.mutate("candidate", "lesson-diagnostic.json", lambda p: p["steps"][0]["sampling"].__setitem__("temperature", 2.0))
        self.assertFalse(self.compare()["matched"])

    def test_policy_and_executable_must_match_predeclared_provenance(self):
        self.mutate("candidate", "pilot.json", lambda p: p["initial_comparison_identity"].__setitem__("executable_sha256", "c" * 64))
        with self.assertRaisesRegex(InvalidComparisonEvidence, "executable provenance"):
            self.compare()

    def test_tampered_policy_payload_rejected_even_when_native_digest_is_unchanged(self):
        with (self.root / "candidate" / "initial.alife-foundation").open("ab") as stream:
            stream.write(b"tampered")
        with self.assertRaisesRegex(InvalidComparisonEvidence, "policy asset"):
            self.compare()

    def test_historical_or_null_identity_never_falls_back_to_context_hash(self):
        path = self.root / "candidate" / "pilot.json"
        original = json.loads(path.read_text())
        for missing in (True, False):
            data = copy.deepcopy(original)
            if missing:
                del data["initial_comparison_identity"]
            else:
                data["initial_comparison_identity"] = None
            self.write("candidate", path.name, data)
            with self.assertRaises(InvalidComparisonEvidence):
                self.compare()

    def test_unknown_schema_subject_and_asset_digest_rejected(self):
        path = self.root / "candidate" / "pilot.json"
        original = json.loads(path.read_text())
        for key, changed in (("schema_version", 2), ("exclusion", "whole_genome"),
                             ("world", dict(schema_version=1, subject=2, words=[11, 22, 33, 44])),
                             ("policy_asset_digest", [9] * 32), ("policy_sha256", "d" * 64)):
            with self.subTest(key=key):
                data = copy.deepcopy(original)
                data["initial_comparison_identity"][key] = changed
                self.write("candidate", path.name, data)
                with self.assertRaises(InvalidComparisonEvidence):
                    self.compare()

    def test_malformed_or_nonfinite_context_fails_closed(self):
        self.mutate("candidate", "lesson-diagnostic.json", lambda p: p["steps"][0].__setitem__("position", [float("nan"), 0, 0]))
        with self.assertRaisesRegex(InvalidComparisonEvidence, "sensory.*position"):
            self.compare()

    def test_unknown_decision_input_cannot_silently_escape_comparison(self):
        self.mutate("candidate", "lesson-diagnostic.json", lambda p: p["steps"][0].__setitem__("future_context", [1]))
        with self.assertRaisesRegex(InvalidComparisonEvidence, "unsupported decision schema"):
            self.compare()

    def test_missing_sensory_and_physiology_cannot_match_as_null(self):
        self.mutate("candidate", "lesson-diagnostic.json", lambda p: p["steps"][0].__setitem__("object_slots", None))
        with self.assertRaisesRegex(InvalidComparisonEvidence, "sensory"):
            self.compare()

    def test_cli_consumes_versioned_identity_and_reports_mismatch_without_writes(self):
        command = [sys.executable, "-B", str(Path(__file__).with_name("compare_foundation_probes.py")),
                   str(self.root/"control"), str(self.root/"candidate"),
                   "--control-policy-sha256", self.policies["control"],
                   "--candidate-policy-sha256", self.policies["candidate"],
                   "--executable-sha256", self.binary]
        passed = subprocess.run(command, text=True, capture_output=True, check=False)
        self.assertEqual(passed.returncode, 0, passed.stderr)
        self.assertTrue(json.loads(passed.stdout)["matched"])
        self.mutate("candidate", "pilot.json", lambda p: p["initial_comparison_identity"]["world"]["words"].__setitem__(0, 12))
        mismatch = subprocess.run(command, text=True, capture_output=True, check=False)
        self.assertEqual(mismatch.returncode, 1, mismatch.stderr)
        self.assertFalse(json.loads(mismatch.stdout)["matched"])

    def test_paired_malformed_identity_digest_and_masks_cannot_match(self):
        originals = {branch: json.loads((self.root / branch / "lesson-diagnostic.json").read_text())
                     for branch in ("control", "candidate")}
        for key, malformed in (("dispatch_generation", None), ("dispatch_generation", 2**64),
                               ("sequence_id", True), ("tick", -1),
                               ("frame_digest", "not-a-digest"), ("frame_digest", [1, 2, 3]),
                               ("frame_digest", [1, 2, 3, 2**64]), ("frame_digest", [True] * 4),
                               ("representative_mask", 2**32), ("motor_masks", [2**32] * 6)):
            with self.subTest(key=key, malformed=malformed):
                for branch, original in originals.items():
                    data = copy.deepcopy(original)
                    data["steps"][0][key] = malformed
                    self.write(branch, "lesson-diagnostic.json", data)
                with self.assertRaises(InvalidComparisonEvidence):
                    self.compare()

    def test_paired_missing_or_nonfinite_physiology_cannot_match(self):
        originals = {branch: json.loads((self.root / branch / "lesson-diagnostic.json").read_text())
                     for branch in ("control", "candidate")}
        for malformed in (None, {}, dict(originals["control"]["steps"][0]["physiology"], before_energy=None),
                          dict(originals["control"]["steps"][0]["physiology"], before_energy=float("inf"))):
            with self.subTest(malformed=malformed):
                for branch, original in originals.items():
                    data = copy.deepcopy(original)
                    data["steps"][0]["physiology"] = malformed
                    self.write(branch, "lesson-diagnostic.json", data)
                with self.assertRaises(InvalidComparisonEvidence):
                    self.compare()


if __name__ == "__main__":
    unittest.main()
