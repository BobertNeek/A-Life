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
                         position=[0, 0, 0], gaze_rotation=0, heard_tokens=[], semantic_context=[0],
                         object_slots=[dict(id=2)], terrain_ranges=[1], global_grip_before_decision=None,
                         lesson_target=2, lesson_target_held_before_decision=False,
                         lesson_target_ownership_before_decision=None, representative_mask=1,
                         motor_masks=[0] * 6, sampling=sampling,
                         physiology=dict(before_energy=1.0, before_brain_atp=1.0, before_fatigue=0.0,
                             before_health=1.0, before_hunger=0.1, before_injury=0.0, before_pain=0.0,
                             after_energy=0.9), chosen_action=211 if byte==1 else 210,
                         candidates=[dict(action=211, family="manipulation", index=0, logit=float(byte),
                                          observation=[1], target=2, target_position=[1, 0, 0])])
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
        for key, changed in (("position", [1, 0, 0]), ("heard_tokens", [3]),
                             ("semantic_context", [1]), ("object_slots", [dict(id=3)]),
                             ("terrain_ranges", [2]), ("physiology", dict(original["steps"][0]["physiology"], before_energy=0.8)),
                             ("representative_mask", 3), ("motor_masks", [1] * 6),
                             ("lesson_target", 3), ("sequence_id", 2)):
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
        with self.assertRaisesRegex(InvalidComparisonEvidence, "noncanonical"):
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


if __name__ == "__main__":
    unittest.main()
