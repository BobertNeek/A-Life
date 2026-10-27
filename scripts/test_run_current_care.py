"""Synthetic host lifecycle tests. Native checkpoint/biology commands are mocked."""
import argparse
import copy
import datetime as dt
import hashlib
import json
import os
import shutil
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import current_checkpoint_store as cp
import run_current_care as current


class Process:
    pid = os.getpid()
    def poll(self):
        return 0


class CurrentCareTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.store_root = self.root / 'store'
        self.store_root.mkdir()
        self.binary = self.root / 'native'
        self.binary.write_bytes(b'synthetic executable identity')
        self.schedule = self.root / 'schedule.json'
        cp.write_json(self.schedule, {'schema': 1, 'steps': [{'lesson': 'feeding', 'episodes': 2, 'decisions': 2}]})
        self.initial = {'policy_version': 10, 'seed': 20, 'founder_seed_base': 7,
                        'actor_optimizer_step': 23165, 'value_optimizer_step': 26281,
                        'new_asset_digest': 'asset10', 'asset_sha256': self.sha(b'asset10'),
                        'seeds': {'historical_freshness': 'unknown', 'excluded_ranges': [[1, 20], [29, 29]], 'pending_range': None},
                        'compiler': [0, 7, 2], 'actor_moments': [0.125, 0.25], 'ages': [4, 9],
                        'value_moments': [0.33, 0.44], 'private_lifetime': [255, 1, 0, 128]}
        cp.write_json(self.store_root / cp.CURRENT, self.initial)
        self.calls = []
        self.patch(current, 'tool', side_effect=self.native)
        self.patch(current, 'checkout_context', return_value={'head': 'fixture', 'tree': 'fixture', 'cargo_lock_sha256': 'fixture'})
        self.patch(current.host, 'assert_lane_idle')
        self.patch(current.host, 'resource_snapshot', return_value={'disk_free_bytes': 10**12,
                    'available_memory_bytes': 10**12, 'idle_seconds': 10**6})
        self.patch(current.host, 'run_child', side_effect=self.trainer)

    def patch(self, target, name, **kwargs):
        p = patch.object(target, name, **kwargs)
        self.addCleanup(p.stop)
        return p.start()

    def sha(self, value):
        return hashlib.sha256(value).hexdigest()

    def native(self, binary, *args, on_started=None):
        if on_started:
            on_started(Process())
        mode, source, *rest = args
        state = cp.read_json(Path(source))
        self.assertTrue(all(key in state for key in ('compiler', 'actor_moments', 'value_moments', 'private_lifetime')))
        if mode == '--inspect-current':
            return state
        if mode in ('--reserve-current', '--check-current-allocation'):
            first, last = rest[:2]
            seeds = state['seeds']
            if seeds['pending_range'] is None:
                self.assertFalse(any(first <= b and a <= last for a, b in seeds['excluded_ranges']))
                seeds['excluded_ranges'] = current.mixed.seeds._ranges(seeds['excluded_ranges'] + [[first, last]])
                seeds['pending_range'] = [first, last]
            else:
                current.require(seeds['pending_range'] == [first, last], 'resume the current unused seed allocation first')
            if mode == '--check-current-allocation':
                return {'valid': True}
            target = rest[2]
        elif mode == '--consume-current':
            seed, target = rest
            a, b = state['seeds']['pending_range']
            self.assertEqual(seed, a)
            state['seeds']['pending_range'] = [a + 1, b] if a < b else None
        elif mode == '--advance-current':
            episode, target = rest
            receipt = cp.read_json(Path(episode) / 'cycle.json')
            self.assertEqual(receipt['policy_version'], state['policy_version'] + 1)
            state.update(seed=receipt['seed'], policy_version=receipt['policy_version'],
                         new_asset_digest=receipt['new_asset_digest'],
                         asset_sha256=self.sha(receipt['new_asset_digest'].encode()))
            state['actor_optimizer_step'] += receipt['actor_optimizer_updates']
            state['value_optimizer_step'] += receipt['value_optimizer_updates']
        else:
            raise AssertionError(mode)
        self.assertFalse(Path(target).exists())
        cp.write_json(Path(target), state)
        return state

    def trainer(self, argv, log, deadline, guard, environment, on_started):
        guard()
        on_started(Process())
        source, output = Path(argv[2]), Path(argv[3])
        self.assertFalse(output.exists())
        state = cp.read_json(source)
        seed = int(argv[argv.index('--seed') + 1])
        # Consumption is durable before a child can learn anything.
        durable = cp.read_json(self.store_root / cp.CURRENT)
        self.assertNotEqual(durable['seeds']['pending_range'], [seed, 33])
        self.calls.append(seed)
        output.mkdir()
        (output / 'initial.alife-foundation').write_bytes(state['new_asset_digest'].encode())
        (output / 'private-recovery').write_bytes(b'private synthetic state; may be removed only after reconciliation')
        receipt = {'seed': seed, 'lesson': 'feeding', 'founder_seed_base': state['founder_seed_base'],
            'biological_objective_version': 2, 'objective_state_reset': False,
            'policy_version': state['policy_version'] + 1, 'old_asset_digest': state['new_asset_digest'],
            'new_asset_digest': f"asset{state['policy_version']+1}",
            'terminal_death_tick': None, 'speech_target_rows': 0,
            'requested_training_ticks': 2, 'trained_replay_rows': 2, 'training_ticks': 2,
            'captured_replay_rows': 3, 'bootstrap_replay_rows': 1, 'completed_epochs': 1,
            'actor_optimizer_updates': 2, 'value_optimizer_updates': 3,
            'prepared_held_food_opportunities': 0, 'prepared_reachable_food_opportunities': 0,
            'held_food_consumption_events': 0, 'trained_held_food_consumption_events': 0, 'consumed_events': 1,
            'food_need_initialization': {'condition': 'newborn_v1'}}
        cp.write_json(output / 'cycle.json', receipt)

    def args(self, first=30, count=4):
        return argparse.Namespace(schedule=self.schedule, seed=first, seed_count=count,
                cutoff=dt.datetime.now(current.host.UTC) + dt.timedelta(hours=1), binary=self.binary,
                checkpoint_tool=self.binary, command_seconds=60, min_disk_gib=1, min_memory_gib=1,
                idle_seconds=0, food_need_condition='newborn_v1')

    def store(self, hook=lambda _: None):
        return cp.Store(self.store_root, current.validator(self.binary), hook)

    def test_multiple_episodes_replace_one_checkpoint_and_remove_all_temporary_files(self):
        with self.store() as store:
            result = current.execute(store, current.start(store, self.args()))
        self.assertEqual(self.calls, [30, 31])
        self.assertEqual(result['policy_version'], 12)
        self.assertEqual(result['actor_optimizer_step'], 23169)
        self.assertEqual(result['value_optimizer_step'], 26287)
        self.assertEqual(result['seeds']['pending_range'], [32, 33])
        for key in ('compiler', 'actor_moments', 'value_moments', 'ages', 'private_lifetime'):
            self.assertEqual(result[key], self.initial[key])
        self.assertEqual({p.name for p in self.store_root.iterdir()}, {cp.CURRENT, cp.LOCK})
        with self.store() as store:
            with self.assertRaisesRegex(ValueError, 'unused seed allocation'):
                current.start(store, self.args(first=40))
            current.execute(store, current.start(store, self.args(first=32, count=2)))
        self.assertEqual(self.calls, [30, 31, 32, 33])
        self.assertEqual({p.name for p in self.store_root.iterdir()}, {cp.CURRENT, cp.LOCK})

    def test_crash_after_acceptance_reconciles_exactly_once_then_resumes(self):
        class Crash(BaseException):
            pass
        def crash(stage):
            if stage == 'new-moved' and cp.read_json(self.store_root / cp.CURRENT)['policy_version'] == 11:
                raise Crash()
        with self.assertRaises(Crash):
            with self.store(crash) as store:
                current.execute(store, current.start(store, self.args()))
        self.assertTrue((self.store_root / cp.RUN / 'episode/private-recovery').exists())
        with self.store() as store:
            state = current.reconcile(store)
            self.assertEqual(state['cursor'], 1)
            self.assertFalse((self.store_root / cp.RUN / 'episode').exists())
            result = current.execute(store, state)
        self.assertEqual(self.calls, [30, 31])
        self.assertEqual(result['policy_version'], 12)

    def test_stopped_unsealed_learning_is_not_automatically_removed(self):
        def interrupted(*args, **kwargs):
            self.trainer(*args, **kwargs)
            (self.store_root / cp.RUN / 'episode/cycle.json').unlink()
            raise RuntimeError('synthetic interruption')
        with patch.object(current.host, 'run_child', side_effect=interrupted):
            with self.assertRaisesRegex(RuntimeError, 'interruption'):
                with self.store() as store:
                    current.execute(store, current.start(store, self.args()))
        with self.store() as store:
            with self.assertRaisesRegex(ValueError, 'unsealed interrupted work retained'):
                current.reconcile(store)
            self.assertTrue((self.store_root / cp.RUN / 'episode/private-recovery').exists())
            state = current.reconcile(store, abandon_unsealed=True)
            self.assertEqual(state['cursor'], 0)
            self.assertEqual(state['next_seed'], 31)
            self.assertEqual(cp.read_json(store.current)['seeds']['pending_range'], [31, 33])
            result = current.execute(store, state)
        self.assertEqual(self.calls, [30, 31, 32])
        self.assertEqual(result['policy_version'], 12)

    def test_admitted_candidate_recovers_without_episode_or_replay_inputs(self):
        class Crash(BaseException):
            pass
        def crash(stage):
            pending = self.store_root / cp.NEXT
            if stage == 'candidate-written' and pending.exists() and cp.read_json(pending)['policy_version'] == 11:
                raise Crash()
        with self.assertRaises(Crash):
            with self.store(crash) as store:
                current.execute(store, current.start(store, self.args()))
        shutil.rmtree(self.store_root / cp.RUN / 'episode')
        (self.store_root / cp.RUN / 'source.alife-checkpoint').unlink()
        with self.store() as store:
            state = current.reconcile(store)
            self.assertEqual(state['cursor'], 1)
            self.assertEqual(cp.read_json(store.current)['policy_version'], 11)
            current.execute(store, state)
        self.assertEqual(self.calls, [30, 31])

    def test_active_or_unknown_writer_cannot_be_abandoned(self):
        with self.store() as store:
            state = current.start(store, self.args())
            state['worker'] = {'pid': os.getpid(), 'stopped': False}
            current.save(store, state)
            with self.assertRaisesRegex(ValueError, 'active'):
                current.reconcile(store, abandon_unsealed=True)
            state['worker']['pid'] = None
            current.save(store, state)
            with self.assertRaisesRegex(ValueError, 'unknown writer'):
                current.reconcile(store, abandon_unsealed=True)
            self.assertTrue((self.store_root / cp.RUN).exists())

    def test_plan_does_not_create_or_write_a_store(self):
        root = self.root / 'not-created'
        self.assertEqual(current.main(['--store', str(root), '--checkpoint-tool', str(self.binary),
                                       '--schedule', str(self.schedule)]), 0)
        self.assertFalse(root.exists())

    def test_oversized_schedule_is_rejected_before_any_run_or_checkpoint_mutation(self):
        cp.write_json(self.schedule, {'schema': 1, 'steps': [{'lesson': 'feeding', 'episodes': 20000, 'decisions': 2}]})
        with self.store():
            pass  # Initialize the stable lock before taking the baseline.
        before = {path.name: path.read_bytes() for path in self.store_root.iterdir()}
        with self.store() as store:
            with self.assertRaisesRegex(ValueError, 'metadata (byte limit|budget)'):
                current.start(store, self.args(count=20000))
            self.assertFalse((self.store_root / cp.RUN).exists())
        # Windows prohibits reading the locked byte through another handle.
        self.assertEqual({path.name: path.read_bytes() for path in self.store_root.iterdir()}, before)
        self.assertFalse(self.calls)

    def test_supported_large_schedule_stays_readable_and_resumable(self):
        cp.write_json(self.schedule, {'schema': 1, 'steps': [{'lesson': 'feeding', 'episodes': 12000, 'decisions': 2}]})
        with self.store() as store:
            state = current.start(store, self.args(count=12000))
            state['worker'] = {'pid': os.getpid(), 'stopped': True}
            current.save(store, state)
        with self.store() as store:
            restored = current.reconcile(store)
            self.assertEqual(restored['rows'], state['rows'])
            self.assertEqual(restored['cursor'], 0)
            self.assertEqual(restored['next_seed'], 30)
            self.assertLess((self.store_root / cp.RUN / 'run.json').stat().st_size, cp.MAX_METADATA_BYTES)

    def test_legacy_seed_reduction_preserves_exclusions_unknown_freshness_and_unused_tail(self):
        source = self.root / 'legacy'
        source.mkdir()
        cp.write_json(source / 'cycle.json', {'seed': 30})
        for name in current.mixed.FILES:
            if name != 'cycle.json':
                (source / name).write_bytes(b'synthetic native source pin')
        pins = current.mixed.file_hashes(source)
        ledger = {'excluded_seed_ranges': [[100, 102]], 'assessment_seeds': [103], 'runs': [{
            'plan': {'seed_range': [30, 35]}, 'attempts': [
                {'status': 'accepted', 'directory': str(source), 'seed': 30, 'sha256': pins},
                {'status': 'abandoned', 'directory': str(self.root / 'abandoned'), 'seed': 31}]}]}
        lane = self.root / 'legacy-lane'
        lane.mkdir()
        (lane / current.mixed.LEDGER).write_text('synthetic ledger; parser mocked in this reduction test')
        with patch.object(current.mixed, 'history', return_value=({'complete': False}, 'hash', [[1, 20], [30, 35]], {29})), \
             patch.object(current.mixed, 'read_ledger', return_value=ledger), \
             patch.object(current.mixed, 'reconcile_prior_runs') as reconcile:
            metadata = current.import_seeds(source, self.root / 'history', lane, 'known-exclusions')
        self.assertEqual(metadata, {'historical_freshness': 'unknown',
            'excluded_ranges': [[1, 20], [29, 35], [100, 103]], 'pending_range': [32, 35]})
        reconcile.assert_called_once_with({'complete': False}, ledger['runs'], check_payloads=False)
        self.assertEqual(current.mixed.file_hashes(source), pins)

    def test_legacy_unused_tail_cannot_reclassify_known_seed_exclusions(self):
        source, lane = self.root / 'legacy-conflict', self.root / 'legacy-conflict-lane'
        source.mkdir()
        lane.mkdir()
        cp.write_json(source / 'cycle.json', {'seed': 30})
        for name in current.mixed.FILES:
            if name != 'cycle.json':
                (source / name).write_bytes(b'synthetic source identity')
        pins = current.mixed.file_hashes(source)
        plan = {'source': str(source), 'source_sha256': pins, 'seed_range': [30, 35]}
        ledger = {'schema': 1, 'kind': 'mixed-care-seed-ledger', 'excluded_seed_ranges': [],
            'assessment_seeds': [], 'runs': [{'output': str(lane / 'old-run'), 'plan': plan,
                'plan_sha256': current.mixed.object_hash(plan), 'history_import': {'complete': True},
                'attempts': [
                    {'status': 'accepted', 'directory': str(source), 'seed': 30, 'slot': 0, 'sha256': pins},
                    {'status': 'abandoned', 'directory': str(self.root / 'abandoned'), 'seed': 31, 'slot': 1}]}]}
        history = {'schema': 1, 'kind': 'mixed-care-seed-history', 'complete': True,
            'source_sha256': pins, 'reserved_seed_ranges': [[30, 35]], 'consumed_seed_ranges': [[30, 31]],
            'assessment_seeds': [], 'imports': []}
        foreign = self.root / 'foreign-seeds.json'
        cp.write_json(foreign, {'schema': 1, 'kind': 'grab-food-seed-ledger', 'training_seed_ranges': [[33, 33]],
                               'assessment_seeds': [], 'allocations': []})
        for origin in ('history-assessment', 'ledger-assessment', 'consumed', 'excluded', 'foreign'):
            with self.subTest(origin=origin):
                history['assessment_seeds'] = [33] if origin == 'history-assessment' else []
                ledger['assessment_seeds'] = [33] if origin == 'ledger-assessment' else []
                history['consumed_seed_ranges'] = [[30, 33]] if origin == 'consumed' else [[30, 31]]
                ledger['excluded_seed_ranges'] = [[33, 33]] if origin == 'excluded' else []
                history['imports'] = [{'path': str(foreign), 'sha256': cp.digest(foreign)}] if origin == 'foreign' else []
                cp.write_json(lane / current.mixed.LEDGER, ledger)
                cp.write_json(self.root / 'history.json', history)
                # Exercise the real legacy history/ledger parsers and admission.
                with self.assertRaisesRegex(ValueError, 'assessment|exclusions'):
                    current.import_seeds(source, self.root / 'history.json', lane, 'strict')
        self.assertEqual(current.mixed.file_hashes(source), pins)


if __name__ == '__main__':
    unittest.main()
