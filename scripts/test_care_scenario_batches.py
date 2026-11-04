import json
import os
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest
from unittest.mock import patch

import care_scenario_batches as batches
from llm_care_scenarios import LEGEND, RANDOM_ROLL


class BatchTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name); self.root = self.base / 'ledger'
        self.binary = self.base / 'native'; self.binary.write_bytes(b'mock evaluator, never executed')
        self.plan = {'schema': 1, 'scope': batches.SCOPE, 'batches': 2, 'family_count': 2,
                     'levels': 2, 'initial_difficulty': 0, 'decision_budget': 4, 'slots': 1,
                     'trials_per_family': 3, 'pool_per_family': 1, 'holdouts_per_family': 1,
                     'seed_start': 1001, 'sampler_seed': 4, 'excluded_seeds': [1000],
                     'native_evaluator_sha256': batches.file_pin(self.binary)['sha256']}
        self.recipe = {'schema': 1, 'legend': LEGEND, 'scenarios': [
            {'id': 'family_%02d' % i, 'map': rows, 'condition': 'newborn_v1',
             'difficulty': i, 'goal': 'nourish_and_survive', 'random_roll': RANDOM_ROLL}
            for i, rows in enumerate((['#####', '#L.F#', '#####'], ['#####', '#F.L#', '#####']))]}
        self.run_root = self.base / 'run'; self.run_root.mkdir()
        self.recipe_path = self.base / 'recipe.json'; self.recipe_path.write_text(json.dumps(self.recipe))

    def prepare_proposal(self):
        batches.initialize(self.root, self.plan)
        public = batches.prepare(self.root)
        batches.propose(self.root, self.recipe_path)
        private = self.dispatch()
        return public, private

    def dispatch(self):
        if not (self.root / 'founder-claim.json').exists():
            batches.claim_founder(self.root, {'run_root': str(self.run_root), 'founder_seed': 1000,
                'care_binary_sha256': self.plan['native_evaluator_sha256'],
                'evaluator_sha256': self.plan['native_evaluator_sha256']})
        return batches.claim_dispatch(self.root, {
            'source_sha256': {name: 'c'*64 for name in batches.SOURCE_FILES}, 'run_root': str(self.run_root),
            'evaluator_sha256': self.plan['native_evaluator_sha256']})

    def native_rows(self, private):
        rows, paths = {}, []
        families = {f['id']: f for f in private['bank']['families']}
        for i, task in enumerate(private['discovery']):
            path = self.run_root / 'batches' / ('%04d' % private['batch']) / ('assessment-%04d' % i)
            path.mkdir(parents=True)
            (path / 'native-evidence').write_text('fixed mock')
            (path / 'care-evaluation.json').write_text(json.dumps({'input_sha256': {}}))
            if getattr(self, 'full_inventory', False):
                # Observed native smoke: 19 archive files + 9 request/command controls per trial.
                for n in range(17): (path / ('native-%02d' % n)).write_bytes(b'mock private evidence')
                for n in range(9): (path.parent / ('control-%04d-%02d' % (i, n))).write_bytes(b'mock control')
            paths.append(path)
            f = families[task['family']]
            rows[path] = {'lesson': f['lesson'], 'food_need_condition': f['food_need_condition'],
                         'layout': f['layout'], 'seed': task['seed'], 'decisions': 4, 'temperature': 1.0,
                         'actor_sha256': 'a'*64, 'value_sha256': 'b'*64, 'source_sha256': {name: 'c'*64 for name in batches.SOURCE_FILES},
                         'policy_version': 1, 'success': True, 'survived_evaluated_window': True,
                         'receipt_sha256': batches.file_pin(path / 'care-evaluation.json')['sha256']}
        return paths, rows

    def assess(self, private, mutate=None):
        paths, rows = self.native_rows(private)
        if mutate: mutate(rows)
        with patch.object(batches, 'score_assessment', side_effect=lambda p, _: rows[p]) as verify:
            result = batches.assess(self.root, paths, self.binary)
            self.assertEqual(verify.call_count, len(paths))
        self.seal(private['batch'], result)
        return result

    def seal(self, batch, aggregate):
        root = self.run_root / 'batches' / ('%04d' % batch)
        inventory = {'files': batches.raw_tree(root),
            'directories': sorted(str(Path(p).relative_to(root)) for p, _, _ in os.walk(root))}
        batches.write_new(root / 'completed.json', {'batch': batch, 'aggregate': aggregate,
            'training_enabled': False, 'promotion': 'none', 'inventory': inventory})

    def test_ledger_reader_and_writer_share_large_record_bound(self):
        # A maximum plan can produce >1 MB of validated rows/archive pins.
        path = self.base / 'record.json'
        value = {'evidence': 'x' * 1_100_000}
        batches.write_new(path, value)
        self.assertEqual(batches.read_record(path), value)
        too_big = self.base / 'oversize.json'
        with self.assertRaisesRegex(ValueError, 'exceeds bound'):
            batches.write_new(too_big, {'evidence': 'x' * batches.MAX_RECORD_BYTES})
        self.assertFalse(too_big.exists())

    def test_maximum_family_trial_batch_remains_readable(self):
        self.plan.update(family_count=16, levels=8, trials_per_family=4)
        self.full_inventory = True
        scenarios = []
        for i in range(16):
            grid = [list('#' * 17)] + [list('#' + '.' * 15 + '#') for _ in range(15)] + [list('#' * 17)]
            grid[1][1] = 'L'; grid[1 + i // 14][2 + i % 14] = 'F'
            scenarios.append(dict(self.recipe['scenarios'][0], id='family_%02d' % i,
                                  map=[''.join(row) for row in grid], difficulty=i % 8))
        self.recipe['scenarios'] = scenarios
        self.recipe_path.write_text(json.dumps(self.recipe))
        # Legal long archive paths make full sampler rows + evidence >1 MB.
        for _ in range(12): self.run_root = self.run_root / ('a' * 100)
        self.run_root.mkdir(parents=True)
        _, private = self.prepare_proposal()
        self.assess(private)
        record = self.root / '0004' / 'record.json'
        self.assertLess(record.stat().st_size, batches.MAX_RECORD_BYTES)
        self.assertGreaterEqual(len(batches.raw_tree(self.run_root / 'batches' / '0000')), 64 * 28)
        self.assertEqual(batches.prepare(self.root)['current_difficulty'], 1)

    def test_cli_prepare_and_one_shot_proposal(self):
        plan = self.base / 'plan.json'; plan.write_text(json.dumps(self.plan))
        script = Path(batches.__file__)
        def run(*args):
            return subprocess.run([sys.executable, str(script), '--root', str(self.root), *args],
                                  capture_output=True, text=True, timeout=10)
        self.assertEqual(run('init', '--plan', str(plan)).returncode, 0)
        request = run('prepare')
        self.assertEqual(request.returncode, 0, request.stderr)
        self.assertEqual(json.loads(request.stdout)['kind'], 'care_recipe_request')
        proposal = run('propose', '--recipe', str(self.recipe_path))
        self.assertEqual(proposal.returncode, 0, proposal.stderr)
        self.assertEqual(json.loads(proposal.stdout)['scope'], batches.SCOPE)
        self.assertNotEqual(run('propose', '--recipe', str(self.recipe_path)).returncode, 0)
        self.assertEqual(json.loads(run('status').stdout)['stage'], 'proposed')

    def test_two_batch_loop_bounded_feedback_and_no_seed_reuse(self):
        public, private = self.prepare_proposal()
        self.assertIsNone(public['feedback'])
        self.assertNotIn('1001', json.dumps(public))
        self.assertNotIn(str(self.base), json.dumps(public))
        aggregate = self.assess(private)
        self.assertEqual(aggregate['difficulty_after'], 1)
        public2 = batches.prepare(self.root)
        self.assertEqual(public2['feedback'], aggregate)
        self.assertEqual(set(public2['feedback']), {'difficulty_before', 'difficulty_after', 'families'})
        for f in public2['feedback']['families']:
            self.assertEqual(set(f), {'id', 'trials', 'successes'})
        batches.propose(self.root, self.recipe_path); private2 = self.dispatch()
        def seeds(p):
            return {t['seed'] for k in ('pool', 'holdouts') for t in p['bank'][k]} | {t['seed'] for t in p['discovery']}
        self.assertFalse(seeds(private) & seeds(private2))
        self.assess(private2)
        with self.assertRaisesRegex(ValueError, 'exhausted'): batches.prepare(self.root)
        with self.assertRaises(ValueError): batches.assess(self.root, [], self.binary)
        self.assertEqual(len(batches.load(self.root)), 9)

    def test_aggregate_dispatch_budget_rejects_before_any_claim(self):
        for families, trials in ((16, 32), (16, 5), (3, 22)):
            with self.assertRaisesRegex(ValueError, '64 discovery'):
                batches.initialize(self.root, dict(self.plan, family_count=families, trials_per_family=trials))
            self.assertFalse(self.root.exists())
        batches.initialize(self.root, dict(self.plan, family_count=16, trials_per_family=4))
        self.assertEqual(len(batches.load(self.root)), 1)

    def test_missing_completion_seal_blocks_next_public_request(self):
        _, private = self.prepare_proposal(); self.assess(private)
        (self.run_root / 'batches' / '0000' / 'completed.json').unlink()
        with self.assertRaises(FileNotFoundError): batches.prepare(self.root)
        self.assertFalse((self.root / '0005').exists())

    def test_seed_overlap_rejects_before_root_creation(self):
        for update in ({'excluded_seeds': [1001]}, {'seed_start': 2**64-2},
                       {'excluded_seeds': [2, 2]}, {'trials_per_family': 0},
                       {'scope': 'retained_training'}):
            plan = dict(self.plan, **update)
            with self.assertRaises(ValueError): batches.initialize(self.root, plan)
            self.assertFalse(self.root.exists())

    def test_interrupted_claim_and_duplicate_worker_block(self):
        batches.initialize(self.root, self.plan)
        stage = batches.claim(self.root, 1)
        with self.assertRaises(FileExistsError): batches.claim(self.root, 1)
        for action in (lambda: batches.prepare(self.root),
                       lambda: batches.propose(self.root, self.recipe_path)):
            with self.assertRaisesRegex(ValueError, 'incomplete'): action()
        self.assertEqual(list(stage.iterdir()), [])

    def test_bad_proposal_burns_claim_without_retry(self):
        batches.initialize(self.root, self.plan); batches.prepare(self.root)
        self.recipe_path.write_text('{"schema":1,"schema":1}')
        with self.assertRaises(ValueError): batches.propose(self.root, self.recipe_path)
        self.recipe_path.write_text(json.dumps(self.recipe))
        with self.assertRaisesRegex(ValueError, 'incomplete'): batches.propose(self.root, self.recipe_path)
        with self.assertRaisesRegex(ValueError, 'incomplete'): batches.prepare(self.root)

    def test_model_cannot_change_assigned_ladder(self):
        batches.initialize(self.root, self.plan); batches.prepare(self.root)
        self.recipe['scenarios'][0]['difficulty'] = 1
        self.recipe['scenarios'][1]['difficulty'] = 0
        self.recipe_path.write_text(json.dumps(self.recipe))
        with self.assertRaisesRegex(ValueError, 'assigned'): batches.propose(self.root, self.recipe_path)

    def test_record_tamper_is_detected(self):
        self.prepare_proposal()
        path = self.root / '0001' / 'record.json'
        data = json.loads(path.read_text()); data['payload']['host']['decision_budget'] = 2
        path.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'changed ledger'): batches.load(self.root)

    def test_record_byte_changes_do_not_pass_canonical_hash(self):
        self.prepare_proposal()
        path = self.root / '0001' / 'record.json'
        path.write_bytes(b' ' + path.read_bytes())
        with self.assertRaisesRegex(ValueError, 'changed ledger'): batches.load(self.root)

    def test_native_failure_blocks_replay(self):
        _, private = self.prepare_proposal(); paths, _ = self.native_rows(private)
        with patch.object(batches, 'score_assessment', side_effect=ValueError('native rejected')):
            with self.assertRaisesRegex(ValueError, 'native rejected'): batches.assess(self.root, paths, self.binary)
        with self.assertRaisesRegex(ValueError, 'incomplete'): batches.assess(self.root, paths, self.binary)

    def test_wrong_discovery_seed_mixed_source_and_dose_reject(self):
        for kind in ('holdout', 'duplicate', 'source', 'dose', 'layout'):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as tmp:
                self.root = Path(tmp) / 'ledger'
                self.run_root = self.base / ('run-' + kind); self.run_root.mkdir()
                _, private = self.prepare_proposal()
                paths, rows = self.native_rows(private)
                first = rows[paths[0]]
                if kind == 'holdout': first['seed'] = private['bank']['holdouts'][0]['seed']
                if kind == 'duplicate': first['seed'] = rows[paths[1]]['seed']
                if kind == 'source': first['actor_sha256'] = 'e'*64
                if kind == 'dose': first['decisions'] = 3
                if kind == 'layout': first['layout'] = {'rows': ['#####', '#F.L#', '#####']}
                with patch.object(batches, 'score_assessment', side_effect=lambda p, _: rows[p]):
                    with self.assertRaises(ValueError): batches.assess(self.root, paths, self.binary)

    def test_archives_outside_claimed_batch_reject_before_native_call(self):
        _, private = self.prepare_proposal()
        other = self.base / 'other-run'; other.mkdir()
        self.run_root = other
        paths, _ = self.native_rows(private)
        with patch.object(batches, 'score_assessment') as verify:
            with self.assertRaisesRegex(ValueError, 'claimed batch output paths'):
                batches.assess(self.root, paths, self.binary)
            verify.assert_not_called()

    def test_pin_mutations_during_native_admission_reject(self):
        _, private = self.prepare_proposal(); paths, rows = self.native_rows(private)
        def changed(p, _):
            if p == paths[-1]: (paths[0] / 'native-evidence').write_text('mutated after first verification')
            return rows[p]
        with patch.object(batches, 'score_assessment', side_effect=changed):
            with self.assertRaisesRegex(ValueError, 'archive changed'): batches.assess(self.root, paths, self.binary)

    def test_evaluator_pin_rejects_without_execution(self):
        _, private = self.prepare_proposal(); paths, _ = self.native_rows(private)
        self.binary.write_bytes(b'changed')
        with patch.object(batches, 'score_assessment') as verify:
            with self.assertRaisesRegex(ValueError, 'evaluator changed'): batches.assess(self.root, paths, self.binary)
            verify.assert_not_called()

    def test_cross_batch_frozen_source_change_rejects(self):
        _, private = self.prepare_proposal(); self.assess(private)
        batches.prepare(self.root); batches.propose(self.root, self.recipe_path); private = self.dispatch()
        with self.assertRaisesRegex(ValueError, 'source differs'):
            self.assess(private, lambda rows: [r.update(source_sha256='e'*64) for r in rows.values()])

    def test_founder_initialization_cannot_be_reclaimed(self):
        self.prepare_proposal()
        identity = batches.read_record(self.root / 'founder-claim.json')['identity']
        with self.assertRaises(ValueError): batches.claim_founder(self.root, identity)
        # Also exclusive before any dispatch, including a different run root.
        root = self.base / 'other-ledger'; batches.initialize(root, self.plan)
        batches.claim_founder(root, identity)
        with self.assertRaises(FileExistsError):
            batches.claim_founder(root, dict(identity, run_root=str(self.base / 'run-two')))

    def test_feedback_rechecks_prior_native_inputs(self):
        _, private = self.prepare_proposal(); paths, rows = self.native_rows(private)
        control = self.run_root / 'history.json'; control.write_text('{}')
        for path in paths:
            (path / 'care-evaluation.json').write_text(json.dumps({
                'input_sha256': {str(control): batches.file_pin(control)['sha256']}}))
            rows[path]['receipt_sha256'] = batches.file_pin(path / 'care-evaluation.json')['sha256']
        with patch.object(batches, 'score_assessment', side_effect=lambda p, _: rows[p]):
            batches.assess(self.root, paths, self.binary)
        control.write_text('corrupt history')
        with self.assertRaisesRegex(ValueError, 'input changed'): batches.prepare(self.root)
        self.assertFalse((self.root / '0005').exists())

    def test_dispatch_cannot_be_claimed_twice(self):
        self.prepare_proposal()
        with self.assertRaisesRegex(ValueError, 'never replay'): self.dispatch()
        self.assertEqual(batches.load(self.root)[-1]['kind'], 'dispatched')

    def test_exclusive_init_and_symlink_reject(self):
        batches.initialize(self.root, self.plan)
        with self.assertRaises(FileExistsError): batches.initialize(self.root, self.plan)
        link = self.base / 'alias'; link.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(ValueError): batches.prepare(link)


if __name__ == '__main__': unittest.main()
