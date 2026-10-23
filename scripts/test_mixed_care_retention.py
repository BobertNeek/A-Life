"""Host recovery contracts; native inspector/trainer execution is mocked, not GPU evidence."""
import copy
import datetime as dt
import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import run_mixed_care_serial as mixed


class MixedRetentionTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.output = self.root / 'run'
        self.output.mkdir()
        self.source = self.root / 'source'
        self.binary = self.root / 'binary'
        self.binary.write_text('fixture binary identity')
        initial = self.cycle(self.source, 10, 90)
        self.plan = {'schema': 1, 'kind': 'mixed-care-plan', 'output': str(self.output),
            'source': str(self.source), 'source_sha256': mixed.file_hashes(self.source),
            'source_receipt': initial, 'source_seed': 90,
            'retention': {'mode': 'recent-checkpoints', 'keep': 2},
            'binary': str(self.binary), 'inspector': str(self.binary),
            'binary_sha256': {str(self.binary): mixed.host.sha256(self.binary)},
            'history': str(self.root / 'history.json'), 'history_sha256': 'history',
            'schedule': {'schema': 1, 'steps': [{'lesson': 'feeding', 'episodes': 5, 'decisions': 2}]},
            'seed_range': [100, 106], 'cutoff': (dt.datetime.now(mixed.host.UTC) + dt.timedelta(hours=1)).isoformat(),
            'command_seconds': 60, 'min_disk_gib': 1, 'min_memory_gib': 1, 'idle_seconds': 0}
        self.run = {'output': str(self.output), 'plan': self.plan,
                    'plan_sha256': mixed.object_hash(self.plan), 'history_import': {'complete': True}, 'attempts': []}
        previous = self.source
        for i in range(4):
            directory = self.output / f'attempt-{i:05d}'
            receipt = self.cycle(directory, 11 + i, 100 + i)
            hashes = mixed.file_hashes(directory)
            hashes['initial.alife-foundation'] = mixed.host.sha256(directory / 'initial.alife-foundation')
            self.run['attempts'].append({'slot': i, 'seed': 100+i, 'directory': str(directory),
                'previous': str(previous), 'status': 'accepted', 'receipt': receipt, 'sha256': hashes})
            previous = directory
        self.ledger = {'schema': 1, 'kind': 'mixed-care-seed-ledger', 'excluded_seed_ranges': [],
                       'assessment_seeds': [], 'runs': [self.run]}
        self.save()
        self.inspected = []
        self.dispatched = []
        self.patch(mixed, 'admit_run_context')
        self.patch(mixed.host, 'assert_lane_idle')
        self.patch(mixed.host, 'resource_snapshot', return_value={'disk_free_bytes': 10**12,
                   'available_memory_bytes': 10**12, 'idle_seconds': 10**6})
        self.patch(mixed, 'history', return_value=({'complete': True}, 'history', [], set()))
        self.patch(mixed.inspector_io, 'run_inspector', side_effect=self.inspector)
        self.patch(mixed.host, 'run_child', side_effect=self.trainer)

    def patch(self, target, name, **kw):
        p = patch.object(target, name, **kw)
        self.addCleanup(p.stop)
        return p.start()

    def save(self):
        self.run['plan_sha256'] = mixed.object_hash(self.plan)
        mixed.seeds.write_document(self.root / mixed.LEDGER, self.ledger)

    def cycle(self, directory, policy, seed):
        directory.mkdir()
        receipt = {'seed': seed, 'lesson': 'feeding', 'founder_seed_base': 42,
            'biological_objective_version': 2, 'objective_state_reset': False,
            'policy_version': policy, 'old_asset_digest': f'asset{policy-1}', 'new_asset_digest': f'asset{policy}',
            'terminal_death_tick': None, 'speech_target_rows': 0,
            'requested_training_ticks': 2, 'trained_replay_rows': 2, 'training_ticks': 2,
            'captured_replay_rows': 3, 'bootstrap_replay_rows': 1, 'completed_epochs': 1,
            'actor_optimizer_updates': 2, 'value_optimizer_updates': 3,
            'prepared_held_food_opportunities': 0, 'prepared_reachable_food_opportunities': 0,
            'held_food_consumption_events': 0, 'trained_held_food_consumption_events': 0, 'consumed_events': 1}
        for name, obj in [('cycle.json', receipt), ('actor-checkpoint.json', {'optimizer_step': policy*2}),
                          ('value-checkpoint.json', {'optimizer_step': policy*3})]:
            (directory / name).write_text(json.dumps(obj))
        (directory / 'trained.alife-foundation').write_text(f'asset{policy}')
        (directory / 'initial.alife-foundation').write_text(f'asset{policy-1}')
        return receipt

    def inspector(self, argv, **_):
        directory = Path(argv[2]); self.inspected.append(directory)
        receipt = json.loads((directory / 'cycle.json').read_text())
        report = {'neural_execution': False, 'weights_modified': False,
                  'policy_version': receipt['policy_version'], 'expected_policy_version': int(argv[4]),
                  'founder_seed_base': 42, 'file_digests': [
                    {'file_name': n, 'sha256': mixed.host.sha256(directory/n)} for n in mixed.FILES]}
        for head in ('actor', 'value'):
            report[f'{head}_optimizer_step'] = json.loads((directory / f'{head}-checkpoint.json').read_text())['optimizer_step']
        return subprocess.CompletedProcess(argv, 0, json.dumps(report))

    def trainer(self, argv, *_args, **_kwargs):
        previous, output = Path(argv[2]), Path(argv[3])
        seed = int(argv[argv.index('--seed')+1]); self.dispatched.append(seed)
        policy = json.loads((previous / 'cycle.json').read_text())['policy_version'] + 1
        self.cycle(output, policy, seed)

    def retire_synthetic_payloads(self):
        for path in (self.source, self.output/'attempt-00000', self.output/'attempt-00001'):
            shutil.rmtree(path)

    def test_resume_without_old_payloads_or_source(self):
        self.retire_synthetic_payloads()
        result = mixed.execute(self.plan, resume=True)
        self.assertEqual(result['accepted_episodes'], 5)
        self.assertEqual(self.dispatched, [104])
        self.assertFalse(any(p.name in ('source', 'attempt-00000', 'attempt-00001') for p in self.inspected))
        attempts = mixed.read_ledger(self.root/mixed.LEDGER)['runs'][0]['attempts']
        self.assertEqual([a['seed'] for a in attempts], list(range(100,105)))
        self.assertEqual(json.loads((self.output/'attempt-00004'/'actor-checkpoint.json').read_text())['optimizer_step'], 30)

    def test_legacy_plan_still_requires_its_payloads(self):
        del self.plan['retention']; del self.plan['source_receipt']; self.save()
        self.retire_synthetic_payloads()
        with self.assertRaises(OSError):
            mixed.execute(self.plan, resume=True)
        self.assertEqual(self.dispatched, [])

    def test_resume_compacts_ledger_but_keeps_summary_readable(self):
        mixed.execute(self.plan, resume=True)
        path = self.root / mixed.LEDGER
        actual = path.read_bytes()
        ledger = mixed.read_ledger(path)
        self.assertEqual(actual, (json.dumps(ledger, separators=(',', ':'), allow_nan=False) + '\n').encode())
        summary = (self.output / 'schedule.json').read_bytes()
        self.assertEqual(summary, (json.dumps(json.loads(summary), indent=2, allow_nan=False) + '\n').encode())

    def test_changed_latest_checkpoint_blocks_dispatch(self):
        self.retire_synthetic_payloads()
        (self.output/'attempt-00003'/'actor-checkpoint.json').write_text('{"optimizer_step":0}')
        with self.assertRaisesRegex(ValueError, 'accepted dose or checkpoint changed'):
            mixed.execute(self.plan, resume=True)
        self.assertEqual(self.dispatched, [])

    def test_retired_metadata_cannot_change_source_link(self):
        self.retire_synthetic_payloads()
        self.run['attempts'][0]['sha256']['initial.alife-foundation'] = '0'*64; self.save()
        with self.assertRaisesRegex(ValueError, 'accepted predecessor hash changed'):
            mixed.execute(self.plan, resume=True)
        self.assertEqual(self.dispatched, [])

    def test_pending_interruption_does_not_reuse_its_seed(self):
        self.retire_synthetic_payloads()
        self.run['attempts'].append({'slot': 4, 'seed': 104, 'directory': str(self.output/'attempt-00004'),
            'previous': str(self.output/'attempt-00003'), 'status': 'pending'})
        self.save()
        result = mixed.execute(self.plan, resume=True)
        self.assertEqual(result['accepted_episodes'], 5)
        self.assertEqual(self.dispatched, [105])
        attempts = mixed.read_ledger(self.root/mixed.LEDGER)['runs'][0]['attempts']
        self.assertEqual(attempts[4]['status'], 'abandoned')
        self.assertEqual(attempts[5]['seed'], 105)

    def test_sealed_pending_attempt_is_reconciled_without_redispatch(self):
        self.retire_synthetic_payloads()
        self.cycle(self.output/'attempt-00004', 15, 104)
        self.run['attempts'].append({'slot': 4, 'seed': 104, 'directory': str(self.output/'attempt-00004'),
            'previous': str(self.output/'attempt-00003'), 'status': 'pending'})
        self.save()
        self.assertEqual(mixed.execute(self.plan, resume=True)['accepted_episodes'], 5)
        self.assertEqual(self.dispatched, [])

    def test_native_inspector_rejection_blocks_dispatch(self):
        self.retire_synthetic_payloads()
        with patch.object(mixed.inspector_io, 'run_inspector', side_effect=subprocess.CalledProcessError(1, 'fixture')):
            with self.assertRaises(subprocess.CalledProcessError):
                mixed.execute(self.plan, resume=True)
        self.assertEqual(self.dispatched, [])

    def test_full_payload_reconciliation_keeps_legacy_metadata_contract(self):
        legacy = copy.deepcopy(self.run)
        for name in ('schedule', 'source_receipt', 'retention'):
            del legacy['plan'][name]
        doc = {'recovery_reconciliation': {'prior_ledger_runs_sha256': mixed.object_hash([legacy])}}
        mixed.reconcile_prior_runs(doc, [legacy])

    def test_prior_reconciliation_uses_bound_metadata_without_payloads(self):
        self.retire_synthetic_payloads()
        doc = {'recovery_reconciliation': {'prior_ledger_runs_sha256': mixed.object_hash([self.run])}}
        mixed.reconcile_prior_runs(doc, [self.run], check_payloads=False)
        altered = copy.deepcopy(self.run); altered['attempts'][0]['receipt']['seed'] += 1
        with self.assertRaisesRegex(ValueError, 'prior ledger runs changed'):
            mixed.reconcile_prior_runs(doc, [altered], check_payloads=False)


if __name__ == '__main__':
    unittest.main()
