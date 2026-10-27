"""Synthetic host ownership tests; no native process, GPU or world is created."""
import argparse
import copy
import datetime as dt
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import current_checkpoint_store as cp
import run_continuous_care as session


class Process:
    pid = os.getpid()

    def poll(self):
        return 0


class ContinuousCareTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.store = self.root / 'store'
        self.lane = self.root / 'owner'
        self.store.mkdir()
        self.lane.mkdir()
        self.binary = self.root / 'native'
        self.binary.write_bytes(b'admitted synthetic native')
        self.initial = {'kind': 'single-current-training-checkpoint', 'external_dependencies': [],
            'policy_version': 40, 'actor_optimizer_step': 100, 'value_optimizer_step': 200,
            'seeds': {'pending_range': [80, 87], 'excluded_ranges': [[1, 87]], 'historical_freshness': 'unknown'},
            'actor_moments': [0.125, 0.25], 'value_moments': [0.4, 0.6],
            'compiler': [3, 4], 'private_lifetime': [5, 6]}
        cp.write_json(self.store / cp.CURRENT, self.initial)
        self.calls = []
        self.crash = False
        self.patches = []
        self.mock(session.current, 'tool', side_effect=self.native)
        self.mock(session.current, 'checkout_context', return_value={'head': 'synthetic', 'tree': 'synthetic'})
        self.mock(session.host, 'assert_lane_idle')
        self.mock(session, 'make_guard', return_value=lambda: {'host': {}, 'gpu': {}})
        self.child = self.mock(session.host, 'run_child', side_effect=self.producer)
        environment = patch.dict(os.environ, {'ALIFE_SLM_PRIOR': 'on', 'ALIFE_SLM_PRIOR_BACKEND': 'deterministic'})
        environment.start()
        self.addCleanup(environment.stop)

    def mock(self, target, name, **kwargs):
        p = patch.object(target, name, **kwargs)
        self.addCleanup(p.stop)
        return p.start()

    def args(self):
        return argparse.Namespace(store=self.store, lane=self.lane, binary=self.binary,
            checkpoint_tool=self.binary, binary_sha256=session.host.sha256(self.binary),
            checkpoint_tool_sha256=session.host.sha256(self.binary), seed=80, last_seed=87,
            decisions=512, food_need_condition='newborn_v1', hard_end=dt.datetime.now(session.host.UTC) + dt.timedelta(hours=1),
            final_reserve_seconds=180, max_temporary_bytes=2 * 2**30, reconcile=False)

    def native(self, binary, *argv):
        mode, *arguments = argv
        self.calls.append(mode)
        if mode == '--resident-contract':
            return {'schema': 1, 'kind': 'native-resident-serial-v1', 'finite_continuous_feeding': True,
                    'max_continuous_feeding_decisions': 36000, 'mid_world_offline_policy_install': False,
                    'bound_run_cleanup': True, 'temporary_budget_enforced_during_writes': True}
        if mode == '--inspect-current':
            return cp.read_json(Path(arguments[0]))
        if mode == '--check-current-allocation':
            state = cp.read_json(Path(arguments[0]))
            self.assertEqual(state['seeds']['pending_range'], list(arguments[1:]))
            return {'valid': True}
        # These commands own the Native lease: a Python Store lease must be absent.
        with cp.writer_lock(self.store):
            run = self.store / cp.RUN
            candidate = run / 'recovered-current.alife-checkpoint'
            if mode == '--recover-resident':
                after = cp.digest(candidate)
                return {'candidate': str(candidate), 'candidate_sha256': after,
                        'already_published_current': cp.digest(self.store / cp.CURRENT) == after}
            if mode == '--finish-resident-run':
                self.assertEqual(cp.digest(self.store / cp.CURRENT), cp.digest(candidate))
                import shutil
                shutil.rmtree(run)
                return {'removed_run': True}
        raise AssertionError(mode)

    def producer(self, argv, log, deadline, guard, environment, on_started):
        self.assertEqual(argv[1], '--resident-serial')
        self.assertEqual(environment['ALIFE_SLM_PRIOR_BACKEND'], 'deterministic')
        guard()
        on_started(Process())
        request = cp.read_json(Path(argv[3]))
        self.assertEqual(len(request['rows']), 1)
        self.assertEqual(request['rows'][0]['decisions'], 512)
        self.assertEqual(request['seed_range'], [80, 87])  # Preserve the unused owned tail.
        self.assertEqual(int(deadline.timestamp()), request['collection_cutoff_unix_seconds'])
        with cp.writer_lock(self.store):
            self.assertEqual(cp.read_json(self.store / cp.CURRENT), self.initial)
            run = self.store / cp.RUN
            run.mkdir()
            candidate = copy.deepcopy(self.initial)
            candidate['seeds']['pending_range'] = [81, 87]
            if not self.crash:
                candidate.update(policy_version=41, actor_optimizer_step=102, value_optimizer_step=203)
                candidate['private_lifetime'] = [7, 8]
            cp.write_json(run / 'recovered-current.alife-checkpoint', candidate)
            if self.crash:
                raise RuntimeError('synthetic mid-world stop, unsnapshotted updates lost')
            event = {'kind': 'resident-provisional-episode-v1', 'seed': 80, 'trained_rows': 512,
                'policy_version': 41, 'exact_trained_nourishing_events': 2,
                'generic_consumed_events': 3, 'causal_collection': None}
            Path(str(log) + '.stdout.log').write_text(json.dumps(event) + '\n')

    def test_one_long_world_full_state_publication_and_bound_cleanup(self):
        receipt = session.run(self.args())
        final = cp.read_json(self.store / cp.CURRENT)
        self.assertEqual(final['policy_version'], 41)
        self.assertEqual(final['actor_optimizer_step'], 102)
        self.assertEqual(final['value_optimizer_step'], 203)
        for field in ('actor_moments', 'value_moments', 'compiler'):
            self.assertEqual(final[field], self.initial[field])
        self.assertEqual(final['private_lifetime'], [7, 8])
        self.assertEqual(final['seeds']['pending_range'], [81, 87])
        self.assertFalse((self.store / cp.RUN).exists())
        self.assertFalse((self.lane / session.STATE).exists())
        self.assertTrue((self.lane / session.RECEIPT).is_file())
        self.assertEqual(self.child.call_count, 1)
        self.assertIsNone(receipt['causal_speed'])  # Missing instrumentation is unknown.
        self.assertEqual(receipt['observed_rows'][0]['exact_trained_nourishing_events'], 2)
        self.assertEqual(receipt['observed_rows'][0]['generic_consumed_events'], 3)

    def test_mid_world_loss_retains_baseline_learning_and_consumes_seed(self):
        self.crash = True
        receipt = session.run(self.args())
        final = cp.read_json(self.store / cp.CURRENT)
        for field in self.initial.keys() - {'seeds'}:
            self.assertEqual(final[field], self.initial[field])
        self.assertEqual(final['seeds']['pending_range'], [81, 87])
        self.assertIn('mid-world stop', receipt['native_exit_error'])
        self.assertEqual(receipt['observed_rows'], [])

    def test_interrupted_publication_reconciles_without_second_world(self):
        publish = cp.Store.publish

        def interrupted(store, candidate, expected):
            result = publish(store, candidate, expected)
            raise RuntimeError('after valid atomic publication')

        with patch.object(cp.Store, 'publish', interrupted):
            with self.assertRaisesRegex(RuntimeError, 'after valid'):
                session.run(self.args())
        self.assertTrue((self.store / cp.RUN).exists())
        args = self.args()
        args.reconcile = True
        receipt = session.run(args)
        self.assertEqual(receipt['after']['policy_version'], 41)
        self.assertFalse((self.store / cp.RUN).exists())
        self.assertEqual(self.child.call_count, 1)

    def test_unknown_launch_outcome_preserves_owner_and_refuses_recovery(self):
        self.child.side_effect = RuntimeError('launch identity unavailable')
        with self.assertRaisesRegex(ValueError, 'launch outcome unknown'):
            session.run(self.args())
        self.assertTrue((self.lane / session.STATE).is_file())
        self.assertNotIn('--recover-resident', self.calls)
        self.assertEqual(cp.read_json(self.store / cp.CURRENT), self.initial)

    def test_changed_binary_blocks_reconciliation(self):
        self.child.side_effect = RuntimeError('unknown launch')
        with self.assertRaises(ValueError):
            session.run(self.args())
        self.binary.write_bytes(b'changed binary')
        args = self.args()
        args.reconcile = True
        with self.assertRaisesRegex(ValueError, 'pinned executable changed'):
            session.run(args)
        self.assertTrue((self.lane / session.STATE).exists())

    def test_request_horizon_deadline_and_allocation_bounds(self):
        args = self.args()
        now = dt.datetime.now(session.host.UTC)
        request = session.request_for(args, now)
        self.assertEqual(request['rows'], [{'lesson': 'feeding', 'decisions': 512, 'food_need_condition': 'newborn_v1'}])
        args.decisions = 36001
        with self.assertRaises(ValueError):
            session.request_for(args, now)
        args.decisions = 512
        args.hard_end = now
        with self.assertRaises(ValueError):
            session.request_for(args, now)
        args.hard_end = now + dt.timedelta(hours=1)
        args.last_seed = args.seed - 1
        with self.assertRaises(ValueError):
            session.request_for(args, now)

    def test_old_quota_contract_is_rejected_before_dispatch(self):
        report = self.native(self.binary, '--resident-contract')
        del report['temporary_budget_enforced_during_writes']
        with self.assertRaisesRegex(ValueError, 'active-writer-budget'):
            session.capabilities(report, 512)

    def test_plan_has_no_native_or_store_access(self):
        result = session.main(['--store', 'does-not-exist', '--lane', 'also-absent',
            '--binary', 'absent', '--checkpoint-tool', 'absent', '--seed', '80', '--last-seed', '87',
            '--hard-end', (dt.datetime.now(session.host.UTC) + dt.timedelta(hours=1)).isoformat()])
        self.assertEqual(result['request']['rows'][0]['decisions'], 512)
        self.assertFalse(result['mutation'])
        self.assertEqual(self.calls, [])


if __name__ == '__main__':
    unittest.main()
