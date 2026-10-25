"""CPU-only evidence tests; synthetic assets are never admitted or executed."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import compare_foundation_probes as compare


class FoundationCheckpointComparisonTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.control = Path(self.temp.name) / 'control'
        self.candidate = Path(self.temp.name) / 'candidate'
        self.executable = 'a' * 64
        self.hashes = []
        self.pilots = []
        fixture = json.loads((Path(__file__).parent / 'testdata/foundation_probe_sensory_v1.json').read_text())
        self.first = dict(fixture, organism_id=1, tick=0, decision_tick=0, sequence_id=1,
                          dispatch_generation=1, frame_digest=[1, 2, 3, 4], lesson_target=3,
                          lesson_target_held_before_decision=False,
                          lesson_target_ownership_before_decision={'owner': None, 'consumed': False},
                          representative_mask=1, motor_masks=[1] * 6,
                          physiology={f'before_{k}': 0.5 for k in
                                      ('brain_atp', 'energy', 'fatigue', 'health', 'hunger', 'injury', 'pain')})
        for i, directory in enumerate((self.control, self.candidate)):
            directory.mkdir()
            digest = bytes([i + 1] * 32)
            asset = b'synthetic test-only policy' + bytes([i]) + digest
            (directory / 'initial.alife-foundation').write_bytes(asset)
            policy_hash = compare.sha256(asset)
            self.hashes.append(policy_hash)
            common = dict(schema_version=1, policy_asset_digest=list(digest),
                          policy_sha256=policy_hash, executable_sha256=self.executable)
            world = dict(schema_version=1, subject=1, words=[1, 2, 3, 4])
            # The narrower v1 still differs when checkpoint provenance differs.
            narrow = dict(common, exclusion=compare.POLICY_WEIGHTS_SCOPE,
                          world=dict(world, words=[i, 2, 3, 4]))
            assessment = dict(common, scope=compare.CHECKPOINT_SCOPE, world=world)
            sampling = dict(seed=42, counter=0, temperature=1.0, demonstrator=False)
            pilot = dict(initial_comparison_identity=narrow, initial_checkpoint_assessment_identity=assessment,
                         source_asset_digest=digest.hex(), initial_world_signature=f'canonical-{i}',
                         seed=42, founder_seed_base=55, lesson='grab_food', teacher_mode=False,
                         initial_sampling=sampling, silent_request=False, request_target=None,
                         vocabulary_token=None, teacher_cue_tokens=[], grab_food_setup={'food': 3},
                         held_food_setup=None, semantic_prior=None)
            self.pilots.append(pilot)
            (directory / 'lesson-trace.json').write_text(json.dumps(dict(steps=[dict(self.first, sampling=sampling)])))
        self.save()

    def save(self):
        for directory, pilot in zip((self.control, self.candidate), self.pilots):
            (directory / 'pilot.json').write_text(json.dumps(pilot))

    def run_comparison(self, scope=compare.CHECKPOINT_SCOPE):
        self.save()
        return compare.compare(self.control, self.candidate,
                               control_policy_sha256=self.hashes[0], candidate_policy_sha256=self.hashes[1],
                               executable_sha256=self.executable, assessment_scope=scope)

    def test_explicit_new_scope_matches_and_keeps_full_audit_identities(self):
        result = self.run_comparison()
        self.assertTrue(result['matched'])
        self.assertEqual(result['assessment_scope'], compare.CHECKPOINT_SCOPE)
        self.assertNotEqual(result['control']['policy_sha256'], result['candidate']['policy_sha256'])
        self.assertNotEqual(result['control']['canonical_world_signature'], result['candidate']['canonical_world_signature'])
        self.assertFalse(self.run_comparison(compare.POLICY_WEIGHTS_SCOPE)['matched'])
        # The default remains the original narrow comparison.
        result = compare.compare(self.control, self.candidate, control_policy_sha256=self.hashes[0],
                                 candidate_policy_sha256=self.hashes[1], executable_sha256=self.executable)
        self.assertFalse(result['matched'])

    def test_no_fallback_or_historical_feeding_certification(self):
        for change in ('missing', 'null', 'old_scope', 'unsupported_lesson', 'unknown_field'):
            with self.subTest(change=change):
                saved = copy.deepcopy(self.pilots)
                pilot = self.pilots[1]
                if change == 'missing':
                    del pilot['initial_checkpoint_assessment_identity']
                elif change == 'null':
                    pilot['initial_checkpoint_assessment_identity'] = None
                elif change == 'old_scope':
                    pilot['initial_checkpoint_assessment_identity']['scope'] = compare.POLICY_WEIGHTS_SCOPE
                elif change == 'unsupported_lesson':
                    pilot['lesson'] = 'vision_search'
                else:
                    pilot['initial_checkpoint_assessment_identity']['unexpected'] = 1
                with self.assertRaises(compare.InvalidComparisonEvidence):
                    self.run_comparison()
                self.pilots = saved

    def test_need_condition_binds_fresh_setup_without_changing_weights_only_v1(self):
        self.use_feeding()
        self.assertTrue(self.run_comparison()['matched'])
        self.pilots[1]['food_need_initialization'].update(condition='ordinary_healthy_aging_v1')
        self.pilots[1]['food_need_initialization']['prepared'].update(tick=240, hunger=0.12, energy=0.7)
        result = self.run_comparison()
        self.assertFalse(result['matched'])
        self.assertIn('setup', result['mismatched_components'])

    def test_weights_only_v1_does_not_reinterpret_new_condition_evidence(self):
        before = self.run_comparison(compare.POLICY_WEIGHTS_SCOPE)
        self.pilots[1]['food_need_initialization'] = {'condition': 'malformed-new-field'}
        after = self.run_comparison(compare.POLICY_WEIGHTS_SCOPE)
        self.assertEqual(before, after)

    def test_fresh_need_setup_missing_malformed_or_unsafe_fails_closed(self):
        self.use_feeding()
        saved = copy.deepcopy(self.pilots)
        changes = [lambda p: p.pop('food_need_initialization'),
                   lambda p: p.update(food_need_initialization=None),
                   lambda p: p['food_need_initialization'].update(condition='unknown'),
                   lambda p: p['food_need_initialization'].update(organism=True),
                   lambda p: p['food_need_initialization']['prepared'].update(tick=1),
                   lambda p: p['food_need_initialization']['prepared'].update(energy=float('nan'))]
        for change in changes:
            self.pilots = copy.deepcopy(saved)
            change(self.pilots[1])
            with self.assertRaises(compare.InvalidComparisonEvidence):
                self.run_comparison()

    def use_feeding(self):
        for pilot in self.pilots:
            pilot['lesson'] = 'feeding'
            pilot['food_position'] = [1.0, 2.0]
            pilot['initial_food_distance'] = 2.0
            state = dict(tick=0, energy=0.7925, hunger=0.0, health=1.0, injury=0.0, pain=0.0, sleeping=False)
            pilot['food_need_initialization'] = dict(condition='newborn_v1', organism=1,
                                                     before=state.copy(), prepared=state.copy())
            del pilot['grab_food_setup']
            del pilot['held_food_setup']

    def test_new_feeding_uses_captured_controls_and_never_infers_historical_identity(self):
        self.use_feeding()
        self.assertTrue(self.run_comparison()['matched'])
        self.pilots[1]['food_position'][0] = 3.0
        self.assertIn('setup', self.run_comparison()['mismatched_components'])
        self.pilots[1]['food_position'][0] = 1.0
        self.pilots[1]['initial_food_distance'] = None
        with self.assertRaises(compare.InvalidComparisonEvidence):
            self.run_comparison()
        self.pilots[1]['initial_food_distance'] = 2.0
        trace = self.candidate / 'lesson-trace.json'
        document = json.loads(trace.read_text())
        document['steps'][0]['physiology']['before_hunger'] = 0.8
        trace.write_text(json.dumps(document))
        self.assertIn('context', self.run_comparison()['mismatched_components'])
        del self.pilots[1]['initial_checkpoint_assessment_identity']
        with self.assertRaises(compare.InvalidComparisonEvidence):
            self.run_comparison()

    def test_policy_and_executable_binding_fail_closed(self):
        for key in ('policy_sha256', 'policy_asset_digest', 'executable_sha256'):
            with self.subTest(key=key):
                saved = copy.deepcopy(self.pilots)
                self.pilots[1]['initial_checkpoint_assessment_identity'][key] = [0] * 32 if key.endswith('digest') else '0' * 64
                with self.assertRaises(compare.InvalidComparisonEvidence):
                    self.run_comparison()
                self.pilots = saved
        (self.candidate / 'initial.alife-foundation').write_bytes(b'corrupt')
        with self.assertRaises(compare.InvalidComparisonEvidence):
            self.run_comparison()

    def test_nuisance_mismatches_still_bind(self):
        mutations = (
            ('world', lambda p: p['initial_checkpoint_assessment_identity']['world']['words'].__setitem__(0, 9)),
            ('provenance', lambda p: p.__setitem__('seed', 43)),
            ('setup', lambda p: p['grab_food_setup'].__setitem__('food', 4)),
        )
        for component, mutate in mutations:
            with self.subTest(component=component):
                saved = copy.deepcopy(self.pilots)
                mutate(self.pilots[1])
                self.assertIn(component, self.run_comparison()['mismatched_components'])
                self.pilots = saved
        trace = self.candidate / 'lesson-trace.json'
        document = json.loads(trace.read_text())
        document['steps'][0]['physiology']['before_energy'] = 0.4
        trace.write_text(json.dumps(document))
        self.assertIn('context', self.run_comparison()['mismatched_components'])

    def test_first_prior_and_provider_are_controls(self):
        self.use_feeding()
        prior_input = dict(organism=1, tick=0, sequence=1, dispatch_generation=1, frame_digest=[1, 2, 3, 4],
                           natural_senses_context='synthetic', output_digest='c' * 64)
        prior = dict(backend='deterministic', context_contract='test-v1', source_provider_identity=None,
                     recorded_bank_digest=None, model='test', provider_identity='test', model_sha256='b' * 64,
                     decision_inputs=[prior_input], delivered_frames=1)
        for pilot in self.pilots:
            pilot['semantic_prior'] = copy.deepcopy(prior)
        # Later counters and requests are outcomes, not initial controls.
        self.pilots[1]['semantic_prior']['delivered_frames'] = 9
        self.pilots[1]['semantic_prior']['decision_inputs'].append(dict(prior_input, tick=1))
        self.assertTrue(self.run_comparison()['matched'])
        self.pilots[1]['semantic_prior']['decision_inputs'][0]['natural_senses_context'] = 'different'
        self.assertIn('prior', self.run_comparison()['mismatched_components'])
        self.pilots[1]['semantic_prior']['decision_inputs'][0]['tick'] = 9
        with self.assertRaises(compare.InvalidComparisonEvidence):
            self.run_comparison()
        self.pilots[1]['semantic_prior']['decision_inputs'] = []
        with self.assertRaises(compare.InvalidComparisonEvidence):
            self.run_comparison()

    def test_sampler_admission_and_sensory_schema_remain_strict(self):
        self.use_feeding()
        trace = self.candidate / 'lesson-trace.json'
        document = json.loads(trace.read_text())
        document['steps'][0]['sampling']['temperature'] = 0.5
        trace.write_text(json.dumps(document))
        with self.assertRaises(compare.InvalidComparisonEvidence):
            self.run_comparison()
        document['steps'][0]['sampling']['temperature'] = 1.0
        document['steps'][0]['unknown_input'] = 1
        trace.write_text(json.dumps(document))
        with self.assertRaises(compare.InvalidComparisonEvidence):
            self.run_comparison()


if __name__ == '__main__':
    unittest.main()
