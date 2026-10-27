"""Synthetic host ownership tests; no native process, GPU or world is created."""
import argparse
import copy
import datetime as dt
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import current_checkpoint_store as cp
import run_continuous_care as session

GUARD_FACTORY = session.make_guard


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
            final_reserve_seconds=180, max_temporary_bytes=2 * 2**30, reconcile=False,
            fresh_world=True, auto_seed=False,
            idle_seconds=300, min_disk_gib=20, min_memory_gib=8, gpu_index=0,
            max_gpu_c=75, min_gpu_free_mib=2048)

    def native(self, binary, *argv):
        mode, *arguments = argv
        self.calls.append(mode)
        if mode == '--resident-contract':
            return {'schema': 1, 'kind': 'native-resident-serial-v1', 'finite_continuous_feeding': True,
                    'max_continuous_feeding_decisions': 36000, 'mid_world_offline_policy_install': False,
                    'bound_run_cleanup': True, 'temporary_budget_enforced_during_writes': True,
                    'recoverable_predispatch_initialization': True}
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
        self.assertEqual(request['seed_range'], self.initial['seeds']['pending_range'])  # Preserve the unused owned tail.
        self.assertEqual(int(deadline.timestamp()), request['collection_cutoff_unix_seconds'])
        with cp.writer_lock(self.store):
            self.assertEqual(cp.read_json(self.store / cp.CURRENT), self.initial)
            run = self.store / cp.RUN
            run.mkdir()
            candidate = copy.deepcopy(self.initial)
            first, last = candidate['seeds']['pending_range']
            candidate['seeds']['pending_range'] = [first + 1, last] if first < last else None
            if not self.crash:
                candidate.update(policy_version=self.initial['policy_version'] + 1,
                    actor_optimizer_step=self.initial['actor_optimizer_step'] + 2,
                    value_optimizer_step=self.initial['value_optimizer_step'] + 3)
                candidate['private_lifetime'] = [7, 8]
            cp.write_json(run / 'recovered-current.alife-checkpoint', candidate)
            if self.crash:
                raise RuntimeError('synthetic mid-world stop, unsnapshotted updates lost')
            event = {'kind': 'resident-provisional-episode-v1', 'seed': first, 'trained_rows': 512,
                'policy_version': candidate['policy_version'], 'exact_trained_nourishing_events': 2,
                'generic_consumed_events': 3, 'causal_collection': None}
            Path(str(log) + '.stdout.log').write_text(json.dumps(event) + '\n')
            Path(str(log) + '.stderr.log').write_text('synthetic device warning\n')

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
        self.assertEqual(receipt['native_exit_code'], 0)
        self.assertEqual(receipt['retained_trained_rows'], 512)
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
        self.assertEqual(receipt['retained_trained_rows'], 0)

    def test_whole_host_wall_includes_admission_before_native_dispatch(self):
        real_datetime = dt.datetime

        class Clock(real_datetime):
            offset = dt.timedelta()

            @classmethod
            def now(cls, tz=None):
                return real_datetime.now(tz) + cls.offset

        def native(binary, *argv):
            if argv[0] == '--resident-contract':
                Clock.offset = dt.timedelta(seconds=120)  # Synthetic admission cost.
            return self.native(binary, *argv)

        args = self.args()
        with patch.object(session.dt, 'datetime', Clock), patch.object(session.current, 'tool', side_effect=native):
            receipt = session.run(args)
        self.assertGreaterEqual(receipt['host_wall_seconds'], 120)
        self.assertIn('reconciliation downtime', receipt['host_wall_interval_scope'])

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

    def test_old_initialization_contract_is_rejected_before_admission_or_dispatch(self):
        def native(binary, *argv):
            report = self.native(binary, *argv)
            if argv[0] == '--resident-contract':
                del report['recoverable_predispatch_initialization']
            return report

        with patch.object(session.current, 'tool', side_effect=native):
            with self.assertRaisesRegex(ValueError, 'recoverable initialization'):
                session.run(self.args())
        self.assertEqual(self.calls, ['--resident-contract'])
        self.assertEqual(self.child.call_count, 0)
        self.assertEqual(cp.read_json(self.store / cp.CURRENT), self.initial)

    def test_new_run_never_overwrites_unreconciled_staging_sidecars(self):
        for name in (session.STATE + '.tmp', session.REQUEST + '.tmp'):
            with self.subTest(sidecar=name):
                path = self.lane / name
                path.write_bytes(b'original interrupted host metadata')
                with self.assertRaisesRegex(ValueError, 'unexplained prior sidecars'):
                    session.run(self.args())
                self.assertEqual(path.read_bytes(), b'original interrupted host metadata')
                self.assertEqual(cp.read_json(self.store / cp.CURRENT), self.initial)
                self.assertEqual(self.child.call_count, 0)
                path.unlink()

    def test_actual_activity_thermal_and_memory_guards_fail_before_dispatch(self):
        args = self.args()
        resources = {'disk_free_bytes': 100 * 2**30, 'available_memory_bytes': 16 * 2**30, 'idle_seconds': 1000}
        gpu = {'adapter': 'synthetic', 'temperature_c': 40, 'free_memory_mib': 4096, 'utilization_percent': 0}
        self.mock(session.host, 'resource_snapshot', return_value=resources)
        self.mock(session, 'gpu_snapshot', return_value=gpu)
        # Only the module's platform branch is replaced, not global os/path behavior.
        with patch.object(session, 'os', SimpleNamespace(name='nt')):
            resources['idle_seconds'] = 0
            with self.assertRaisesRegex(ValueError, 'interactive input resumed'):
                GUARD_FACTORY(args, self.lane, self.store)()
            resources['idle_seconds'] = 1000
            gpu['temperature_c'] = 76
            with self.assertRaisesRegex(ValueError, 'GPU thermal'):
                GUARD_FACTORY(args, self.lane, self.store)()
            gpu['temperature_c'] = 40
            resources['available_memory_bytes'] = 1
            with self.assertRaisesRegex(ValueError, 'disk/memory'):
                GUARD_FACTORY(args, self.lane, self.store)()
        self.assertEqual(cp.read_json(self.store / cp.CURRENT), self.initial)
        self.assertEqual(self.child.call_count, 0)

    def test_plan_has_no_native_or_store_access(self):
        result = session.main(['--store', 'does-not-exist', '--lane', 'also-absent',
            '--binary', 'absent', '--checkpoint-tool', 'absent', '--seed', '80', '--last-seed', '87',
            '--fresh-world',
            '--hard-end', (dt.datetime.now(session.host.UTC) + dt.timedelta(hours=1)).isoformat()])
        self.assertEqual(result['request']['rows'][0]['decisions'], 512)
        self.assertFalse(result['mutation'])
        self.assertEqual(self.calls, [])

    def clock_at(self, now):
        class Clock(dt.datetime):
            @classmethod
            def now(cls, tz=None):
                return now.astimezone(tz) if tz is not None else now.replace(tzinfo=None)
        return patch.object(session.dt, 'datetime', Clock)

    def test_nightly_plan_derives_abt_end_without_checkpoint_or_native_access(self):
        with self.clock_at(dt.datetime(2026, 11, 2, 6, tzinfo=session.host.UTC)):
            result = session.main(['--store', 'does-not-exist', '--lane', 'also-absent',
                '--binary', 'absent', '--checkpoint-tool', 'absent', '--seed', '80', '--last-seed', '87',
                '--fresh-world',
                '--night-date', '2026-11-02'])
        manifest = result['nightly_window']
        self.assertEqual(manifest['start_utc'], '2026-11-02T07:00:00+00:00')
        self.assertEqual(manifest['hard_end_utc'], '2026-11-02T15:00:00+00:00')
        self.assertEqual(result['request']['collection_cutoff_unix_seconds'],
            int(dt.datetime(2026, 11, 2, 14, 57, tzinfo=session.host.UTC).timestamp()))
        self.assertFalse(manifest['scheduler_future_wake_verified'])
        self.assertFalse(result['mutation'])
        self.assertEqual(self.calls, [])

    def test_nightly_plan_refuses_stale_dst_end(self):
        with self.clock_at(dt.datetime(2026, 11, 2, 6, tzinfo=session.host.UTC)):
            with self.assertRaisesRegex(ValueError, '15:00 UTC'):
                session.main(['--store', 'does-not-exist', '--lane', 'also-absent',
                    '--binary', 'absent', '--checkpoint-tool', 'absent', '--seed', '80', '--last-seed', '87',
                    '--night-date', '2026-11-02', '--hard-end', '2026-11-02T16:00:00+00:00'])
        self.assertEqual(self.calls, [])

    def test_nightly_early_and_expired_runs_refuse_before_path_or_native_access(self):
        args = self.args()
        args.night_date = dt.date(2026, 11, 2)
        args.hard_end = dt.datetime(2026, 11, 2, 15, tzinfo=session.host.UTC)
        args.store = self.root / 'does-not-exist'
        for now in (dt.datetime(2026, 11, 2, 6, 59, tzinfo=session.host.UTC),
                    dt.datetime(2026, 11, 2, 14, 57, tzinfo=session.host.UTC),
                    dt.datetime(2026, 11, 3, 7, tzinfo=session.host.UTC)):
            with self.subTest(now=now), self.clock_at(now):
                with self.assertRaisesRegex(ValueError, 'dated Alberta'):
                    session.run(args)
        self.assertEqual(self.calls, [])
        self.assertEqual(self.child.call_count, 0)
        self.assertFalse((self.lane / session.STATE).exists())

    def test_nightly_guard_rechecks_window_and_generic_idle_mode_remains_available(self):
        args = self.args()
        args.night_date = dt.date(2026, 11, 2)
        args.hard_end = dt.datetime(2026, 11, 2, 15, tzinfo=session.host.UTC)
        self.mock(session.host, 'resource_snapshot', return_value={
            'disk_free_bytes': 100 * 2**30, 'available_memory_bytes': 16 * 2**30, 'idle_seconds': 1000})
        self.mock(session, 'gpu_snapshot', return_value={
            'adapter': 'synthetic', 'temperature_c': 40, 'free_memory_mib': 4096, 'utilization_percent': 0})
        guard = GUARD_FACTORY(args, self.lane, self.store)
        with self.clock_at(dt.datetime(2026, 11, 2, 7, tzinfo=session.host.UTC)):
            guard()
        for hour, minute in ((6, 59), (14, 57)):
            with self.clock_at(dt.datetime(2026, 11, 2, hour, minute, tzinfo=session.host.UTC)):
                with self.assertRaisesRegex(ValueError, 'dated Alberta'):
                    guard()
        args.night_date = None  # Independently authorized idle work retains its explicit UTC deadline.
        with self.clock_at(dt.datetime(2026, 11, 2, 6, 59, tzinfo=session.host.UTC)):
            GUARD_FACTORY(args, self.lane, self.store)()

    def test_nightly_publication_retains_window_and_expired_recovery_never_starts_a_world(self):
        args = self.args()
        args.night_date = dt.date(2026, 11, 2)
        args.hard_end = dt.datetime(2026, 11, 2, 15, tzinfo=session.host.UTC)
        with self.clock_at(dt.datetime(2026, 11, 2, 8, tzinfo=session.host.UTC)):
            original_receipt = self.stage_nightly_retirement(args)
        args.reconcile = True
        with self.clock_at(dt.datetime(2026, 11, 3, 16, tzinfo=session.host.UTC)):
            receipt = session.run(args)
        self.assertEqual(receipt, original_receipt)
        self.assertEqual(receipt['nightly_window']['hard_end_utc'], '2026-11-02T15:00:00+00:00')
        self.assertEqual(self.child.call_count, 1)
        self.assertFalse((self.lane / session.STATE).exists())

    def stage_nightly_retirement(self, args):
        with patch.object(session, 'retire_sidecars', side_effect=RuntimeError('before sidecar retirement')):
            with self.assertRaisesRegex(RuntimeError, 'before sidecar'):
                session.run(args)
        return cp.read_json(self.lane / session.RECEIPT)

    def live_args(self):
        args = self.args()
        args.fresh_world = False
        args.food_need_condition = None
        args.total_decisions = 1024
        args.snapshot_every_updates = 2
        return args

    def live_native(self, binary, *argv):
        report = self.native(binary, *argv)
        if argv[0] == '--resident-contract':
            report.update(same_individual_ppo=True, restore_current_private_world=True,
                full_sampling_state_checkpoint=True, in_world_cutoff=True,
                max_total_trained_decisions=10_000_000, max_decisions_per_update=36000,
                world_ticks_per_second=20)
        return report

    def continuous_producer(self, argv, log, deadline, guard, environment, on_started):
        self.assertEqual(argv[1], '--resident-continuous')
        guard()
        on_started(Process())
        request = cp.read_json(Path(argv[3]))
        self.assertNotIn('rows', request)
        self.assertNotIn('food_need_condition', request)
        self.assertNotIn('allocation_admission_required', request)
        self.assertEqual(request['total_trained_decisions'], 1024)
        self.assertEqual(request['decisions_per_update'], 512)
        self.assertEqual(request['snapshot_every_updates'], 2)
        self.assertEqual(int(deadline.timestamp()), request['finalization_deadline_unix_seconds'])
        self.assertEqual(request['finalization_deadline_unix_seconds'] -
            request['collection_cutoff_unix_seconds'], 120)
        with cp.writer_lock(self.store):
            self.assertEqual(cp.read_json(self.store / cp.CURRENT), self.initial)
            run = self.store / cp.RUN
            run.mkdir()
            candidate = copy.deepcopy(self.initial)
            first, last = request['seed_range']
            candidate['seeds']['pending_range'] = [first + 1, last] if first < last else None
            candidate.update(policy_version=42, actor_optimizer_step=104, value_optimizer_step=206)
            candidate['private_lifetime'] = [7, 8]
            candidate['same_individual'] = {'baseline_current_sha256': cp.digest(self.store / cp.CURRENT),
                'baseline_policy_version':40, 'baseline_actor_optimizer_step':100,
                'baseline_value_optimizer_step':200, 'baseline_trained_rows':0, 'baseline_completed_updates':0,
                'total_trained_rows':1024, 'total_completed_updates':2,
                'progress':{'session_nonce':first, 'organism_id':1, 'session_start_world_tick':100,
                    'world_tick':1126, 'trained_rows':1024, 'completed_updates':2, 'sampling_temperature':1.0}}
            cp.write_json(run / 'recovered-current.alife-checkpoint', candidate)
            # A later provisional event is deliberately absent from the admitted snapshot.
            events = [{'kind': 'resident-continuous-update-v1', 'policy_version': 41+i,
                'actor_optimizer_step': 102+i*2, 'value_optimizer_step': 203+i*3,
                'trained_rows': 512} for i in range(3)]
            Path(str(log) + '.stdout.log').write_text(''.join(json.dumps(e)+'\n' for e in events))
            Path(str(log) + '.stderr.log').write_text('')

    def test_default_plan_is_same_individual_without_store_access_or_seed_allocation(self):
        result = session.main(['--store', 'absent-store', '--lane', 'absent-owner',
            '--binary', 'absent-native', '--checkpoint-tool', 'absent-tool', '--auto-seed',
            '--hard-end', (dt.datetime.now(session.host.UTC) + dt.timedelta(hours=1)).isoformat()])
        request = result['request']
        self.assertEqual(request['total_trained_decisions'], 1_000_000)
        self.assertEqual(request['decisions_per_update'], 512)
        self.assertEqual(request['snapshot_every_updates'], 32)
        self.assertIsNone(request['seed_range'])
        self.assertTrue(request['allocation_admission_required'])
        self.assertNotIn('rows', request)
        self.assertFalse(result['native_execution'])
        self.assertEqual(self.calls, [])

    def test_live_default_refuses_older_binary_before_loading_or_changing_current(self):
        with self.assertRaisesRegex(ValueError, 'no fresh-world fallback'):
            session.run(self.live_args())
        self.assertEqual(self.calls, ['--resident-contract'])
        self.assertEqual(self.child.call_count, 0)
        self.assertEqual(cp.read_json(self.store / cp.CURRENT), self.initial)

    def test_continuous_mode_retains_only_saved_updates_and_leaves_native_finalization_time(self):
        with patch.object(session.current, 'tool', side_effect=self.live_native), \
                patch.object(session.host, 'run_child', side_effect=self.continuous_producer):
            receipt = session.run(self.live_args())
        self.assertEqual(receipt['retained_trained_rows'], 1024)
        self.assertEqual(len(receipt['observed_updates']), 3)
        self.assertEqual(receipt['after']['policy_version'], 42)
        self.assertEqual(receipt['after']['actor_optimizer_step'], 104)
        self.assertEqual(receipt['after']['value_optimizer_step'], 206)
        self.assertFalse(receipt['one_fresh_world'])
        self.assertTrue(receipt['same_individual_across_sessions'])
        self.assertEqual(receipt['retained_continuous_progress']['unique_world_ticks'], 1026)
        self.assertEqual(receipt['retained_simulation_seconds'], 51.3)
        self.assertIsNone(receipt['causal_speed'])
        self.assertFalse((self.store / cp.RUN).exists())

    def test_auto_seed_uses_admitted_tail_and_does_not_claim_unknown_history_fresh(self):
        args = self.live_args()
        args.auto_seed = True
        args.seed = args.last_seed = None
        with patch.object(session.current, 'tool', side_effect=self.live_native), \
                patch.object(session.host, 'run_child', side_effect=self.continuous_producer):
            receipt = session.run(args)
        self.assertEqual(receipt['request']['seed_range'], [80, 87])
        self.assertEqual(receipt['seed_admission']['historical_freshness'], 'unknown')
        self.assertEqual(cp.read_json(self.store / cp.CURRENT)['seeds']['pending_range'], [81, 87])

    def test_next_seed_follows_known_exclusions_without_releasing_an_owned_tail(self):
        report = copy.deepcopy(self.initial)
        self.assertEqual(session.admitted_seed_range(report), [80, 87])
        report['seeds']['pending_range'] = None
        self.assertEqual(session.admitted_seed_range(report), [88, 88])
        report['seeds']['excluded_ranges'] = [[1, 2**64-1]]
        with self.assertRaisesRegex(ValueError, 'namespace exhausted'):
            session.admitted_seed_range(report)

    def test_missing_update_or_optimizer_mismatch_never_fabricates_retained_rows(self):
        before = {'policy_version': 40}
        admitted = {'policy_version': 42, 'actor_optimizer_step': 104, 'value_optimizer_step': 206}
        updates = [{'policy_version':41, 'trained_rows':512, 'actor_optimizer_step':102, 'value_optimizer_step':203},
            {'policy_version':42, 'trained_rows':512, 'actor_optimizer_step':104, 'value_optimizer_step':206}]
        self.assertEqual(session.retained_continuous_rows(before, admitted, updates), 1024)
        self.assertIsNone(session.retained_continuous_rows(before, admitted, updates[1:]))
        updates[-1]['actor_optimizer_step'] = 999
        self.assertIsNone(session.retained_continuous_rows(before, admitted, updates))

    def test_admitted_counters_count_without_logs_but_reject_wrong_session_or_baseline(self):
        before={'policy_version':40,'actor_optimizer_step':100,'value_optimizer_step':200,'same_individual':None}
        admitted={'policy_version':42,'actor_optimizer_step':104,'value_optimizer_step':206,
            'same_individual':{'baseline_current_sha256':'baseline', 'baseline_policy_version':40,
                'baseline_actor_optimizer_step':100,'baseline_value_optimizer_step':200,
                'baseline_trained_rows':0,'baseline_completed_updates':0,'total_trained_rows':1024,
                'total_completed_updates':2,'progress':{'session_nonce':80,'organism_id':1,
                    'session_start_world_tick':100,'world_tick':1126,'trained_rows':1024,'completed_updates':2}}}
        self.assertEqual(session.retained_continuous_rows(before,admitted,[],
            baseline_sha256='baseline',session_nonce=80),1024)
        self.assertIsNone(session.retained_continuous_rows(before,admitted,[],
            baseline_sha256='changed',session_nonce=80))
        self.assertIsNone(session.retained_continuous_rows(before,admitted,[],
            baseline_sha256='baseline',session_nonce=81))
        admitted['same_individual']['total_trained_rows']=1025
        self.assertIsNone(session.bound_continuous_progress(before,admitted,'baseline',80))

    def test_restore_mode_refuses_need_repreparation_and_invalid_horizon(self):
        args = self.live_args()
        args.food_need_condition = 'ordinary_healthy_aging_v1'
        with self.assertRaisesRegex(ValueError, 'does not accept food/need preparation'):
            session.request_for(args, dt.datetime.now(session.host.UTC))
        args.food_need_condition = None
        args.total_decisions = 10_000_001
        with self.assertRaisesRegex(ValueError, 'continuous total'):
            session.request_for(args, dt.datetime.now(session.host.UTC))

    def test_live_finalization_guard_has_time_to_save_but_cannot_admit_new_collection(self):
        args = self.live_args()
        args.night_date = dt.date(2026, 11, 2)
        args.hard_end = dt.datetime(2026, 11, 2, 15, tzinfo=session.host.UTC)
        self.mock(session.host, 'resource_snapshot', return_value={
            'disk_free_bytes': 100 * 2**30, 'available_memory_bytes': 16 * 2**30, 'idle_seconds': 1000})
        self.mock(session, 'gpu_snapshot', return_value={
            'adapter': 'synthetic', 'temperature_c': 40, 'free_memory_mib': 4096, 'utilization_percent': 0})
        guard = GUARD_FACTORY(args, self.lane, self.store, allow_finalization=True)
        with self.clock_at(dt.datetime(2026, 11, 2, 14, 58, tzinfo=session.host.UTC)):
            guard()  # Existing Native writer may finalize after soft collection cutoff.
            with self.assertRaisesRegex(ValueError, 'dated Alberta'):
                session.run(args)  # Another session may not start in that save reserve.
        with self.clock_at(dt.datetime(2026, 11, 2, 14, 59, tzinfo=session.host.UTC)):
            with self.assertRaisesRegex(ValueError, 'finalization window'):
                guard()

    def stage_committed_retirement(self):
        with patch.object(session, 'retire_sidecars', side_effect=RuntimeError('before sidecar retirement')):
            with self.assertRaisesRegex(RuntimeError, 'before sidecar'):
                session.run(self.args())
        return cp.read_json(self.lane / session.RECEIPT)

    def test_retirement_retries_after_every_sidecar_removal_without_losing_outcomes(self):
        original_retire = session.retire_sidecars
        for index, target in enumerate(session.SIDECARS):
            with self.subTest(removed=target):
                self.store = self.root / f'store-retire-{index}'
                self.lane = self.root / f'owner-retire-{index}'
                self.store.mkdir()
                self.lane.mkdir()
                cp.write_json(self.store / cp.CURRENT, self.initial)

                def native(binary, *argv):
                    result = self.native(binary, *argv)
                    if argv[0] == '--finish-resident-run':
                        for name in (session.STATE + '.tmp', session.REQUEST + '.tmp'):
                            (self.lane / name).write_bytes(b'owned bounded staging bytes')
                    return result

                def interrupted(lane, receipt, validate):
                    def after_remove(name):
                        if name == target:
                            raise RuntimeError('synthetic sidecar retirement interruption')
                    return original_retire(lane, receipt, validate, after_remove=after_remove)

                count = self.child.call_count
                with patch.object(session.current, 'tool', side_effect=native), \
                        patch.object(session, 'retire_sidecars', side_effect=interrupted):
                    with self.assertRaisesRegex(RuntimeError, 'sidecar retirement interruption'):
                        session.run(self.args())
                committed = cp.read_json(self.lane / session.RECEIPT)
                args = self.args()
                args.reconcile = True
                resumed = session.run(args)
                self.assertEqual(resumed, committed)
                self.assertEqual(resumed['observed_rows'][0]['exact_trained_nourishing_events'], 2)
                self.assertEqual(resumed['retained_trained_rows'], 512)
                self.assertTrue(all(not (self.lane / n).exists() for n in session.SIDECARS))
                self.assertEqual(self.child.call_count, count + 1)

    def test_missing_state_with_original_leftovers_uses_bound_receipt(self):
        receipt = self.stage_committed_retirement()
        (self.lane / session.STATE).unlink()  # Reproduce the reviewed old deletion order.
        self.assertTrue((self.lane / session.REQUEST).exists())
        args = self.args()
        args.reconcile = True
        self.assertEqual(session.run(args), receipt)
        self.assertEqual(self.child.call_count, 1)

    def test_retirement_refuses_changed_current_without_removing_sidecars(self):
        receipt = self.stage_committed_retirement()
        changed = cp.read_json(self.store / cp.CURRENT)
        changed['policy_version'] += 1
        cp.write_json(self.store / cp.CURRENT, changed)
        args = self.args()
        args.reconcile = True
        with self.assertRaisesRegex(ValueError, 'published current changed'):
            session.run(args)
        self.assertTrue((self.lane / session.STATE).exists())
        self.assertTrue((self.lane / session.REQUEST).exists())
        self.assertEqual(cp.read_json(self.lane / session.RECEIPT), receipt)

    def test_committed_same_session_changed_state_never_reconstructs_outcomes(self):
        receipt = self.stage_committed_retirement()
        # Simulate partial log retirement while the owner state survives.
        (self.lane / (session.LOG + '.stdout.log')).unlink()
        state = cp.read_json(self.lane / session.STATE)
        state['native_exit_error'] = 'changed after final receipt commit'
        cp.write_json(self.lane / session.STATE, state)
        args = self.args()
        args.reconcile = True
        with self.assertRaisesRegex(ValueError, 'committed retirement owner conflict'):
            session.run(args)
        self.assertEqual(cp.read_json(self.lane / session.RECEIPT), receipt)
        self.assertTrue((self.lane / session.STATE).exists())
        self.assertTrue((self.lane / session.REQUEST).exists())

    def test_fresh_next_session_binds_and_replaces_prior_completed_receipt(self):
        previous = session.run(self.args())
        previous_digest = cp.digest(self.lane / session.RECEIPT)
        self.initial = cp.read_json(self.store / cp.CURRENT)
        args = self.args()
        args.seed = 81
        receipt = session.run(args)
        owner = receipt['retirement']['owner']
        self.assertNotEqual(owner['session_id'], previous['retirement']['owner']['session_id'])
        self.assertEqual(owner['previous_receipt_sha256'], previous_digest)
        self.assertEqual(receipt['after']['policy_version'], 42)
        self.assertEqual(receipt['retained_trained_rows'], 512)
        self.assertEqual(self.child.call_count, 2)

    def test_old_receipt_cannot_retire_recreated_identical_request(self):
        receipt = self.stage_committed_retirement()
        (self.lane / session.STATE).unlink()
        path = self.lane / session.REQUEST
        contents = path.read_bytes()
        path.unlink()
        path.write_bytes(contents)
        args = self.args()
        args.reconcile = True
        with self.assertRaisesRegex(ValueError, 'identity/digest conflict'):
            session.run(args)
        self.assertEqual(path.read_bytes(), contents)
        self.assertTrue((self.lane / (session.LOG + '.stdout.log')).exists())
        self.assertEqual(cp.read_json(self.lane / session.RECEIPT), receipt)

    def test_retirement_refuses_changed_log_and_unknown_file(self):
        receipt = self.stage_committed_retirement()
        log = self.lane / (session.LOG + '.stdout.log')
        log.write_text('different session\n')
        args = self.args()
        args.reconcile = True
        with self.assertRaisesRegex(ValueError, 'identity/digest conflict'):
            session.run(args)
        unknown = self.lane / 'unrelated-world-save'
        unknown.write_bytes(b'must preserve')
        with self.assertRaisesRegex(ValueError, 'unknown lane entries'):
            session.run(args)
        self.assertTrue((self.lane / session.REQUEST).exists())
        self.assertEqual(unknown.read_bytes(), b'must preserve')
        self.assertEqual(cp.read_json(self.lane / session.RECEIPT), receipt)

    def test_pending_native_retirement_routes_before_python_store_even_without_run(self):
        first = True
        for marker_name in (session.NATIVE_RETIREMENT, session.NATIVE_RETIREMENT_NEXT):
            with self.subTest(marker=marker_name):
                # A new isolated session per marker form.
                self.store = self.root / marker_name.replace('.', '-')
                self.lane = self.root / (marker_name.replace('.', '-') + '-owner')
                self.store.mkdir()
                self.lane.mkdir()
                cp.write_json(self.store / cp.CURRENT, self.initial)
                first = True

                def native(binary, *argv):
                    nonlocal first
                    marker = self.store / marker_name
                    if argv[0] == '--finish-resident-run' and marker.exists():
                        with cp.writer_lock(self.store):
                            marker.unlink()
                        return {'removed_run': True, 'retirement_completed': True}
                    result = self.native(binary, *argv)
                    if argv[0] == '--finish-resident-run' and first:
                        first = False
                        marker.write_bytes(b'synthetic native-owned marker')
                        raise RuntimeError('after Native run removed, before marker retirement')
                    return result

                with patch.object(session.current, 'tool', side_effect=native):
                    with self.assertRaisesRegex(RuntimeError, 'before marker retirement'):
                        session.run(self.args())
                    self.assertFalse((self.store / cp.RUN).exists())
                    args = self.args()
                    args.reconcile = True
                    receipt = session.run(args)
                self.assertEqual(receipt['retained_trained_rows'], 512)
                self.assertFalse((self.store / marker_name).exists())

    def test_pending_initialization_abort_routes_before_store_without_dispatch_or_publication(self):
        for marker_name in (session.NATIVE_INITIALIZATION, session.NATIVE_INITIALIZATION_NEXT):
            with self.subTest(marker=marker_name):
                self.store = self.root / marker_name.replace('.', '-')
                self.lane = self.root / (marker_name.replace('.', '-') + '-owner')
                self.store.mkdir()
                self.lane.mkdir()
                cp.write_json(self.store / cp.CURRENT, self.initial)
                before = cp.digest(self.store / cp.CURRENT)

                def startup(argv, log, deadline, guard, environment, on_started):
                    on_started(Process())
                    (self.store / marker_name).write_bytes(b'synthetic bound pre-dispatch commitment')
                    raise RuntimeError('interrupted Native initialization before run/world creation')

                def native(binary, *argv):
                    if argv[0] != '--finish-resident-run':
                        return self.native(binary, *argv)
                    self.calls.append(argv[0])
                    # The old Python Store rejects this marker, and this proves
                    # its lease is not held while Native performs its cleanup.
                    with cp.writer_lock(self.store):
                        self.assertFalse((self.store / cp.RUN).exists())
                        (self.store / marker_name).unlink()
                    return {'kind': 'resident-initialization-aborted-v1', 'already_published_current': True,
                        'candidate': str(self.store / cp.CURRENT), 'candidate_sha256': before,
                        'cleanup_completed': True, 'initialization_aborted': True,
                        'initialization_binding_verified': marker_name == session.NATIVE_INITIALIZATION,
                        'dispatched_worlds': 0, 'seeds_consumed': 0, 'new_request_allocation_committed': False,
                        'temporary_budget': None, 'neural_execution': False}

                self.child.side_effect = startup
                with patch.object(session.current, 'tool', side_effect=native), \
                        patch.object(cp.Store, 'publish', side_effect=AssertionError('zero-dispatch abort must not publish')) as publish:
                    receipt = session.run(self.args())
                    publish.assert_not_called()
                self.assertEqual(cp.digest(self.store / cp.CURRENT), before)
                self.assertEqual(cp.read_json(self.store / cp.CURRENT), self.initial)
                self.assertTrue(receipt['recovery']['already_published_current'])
                self.assertTrue(receipt['recovery']['initialization_aborted'])
                self.assertTrue(receipt['cleanup']['cleanup_completed'])
                self.assertEqual(receipt['retained_trained_rows'], 0)
                self.assertFalse(receipt['one_fresh_world'])
                self.assertNotIn('--recover-resident', self.calls)

    def test_fresh_admission_refuses_either_initialization_marker_without_finishing_it(self):
        for marker_name in (session.NATIVE_INITIALIZATION, session.NATIVE_INITIALIZATION_NEXT):
            with self.subTest(marker=marker_name):
                path = self.store / marker_name
                path.write_bytes(b'belongs to existing startup owner')
                with self.assertRaisesRegex(ValueError, 'reconcile existing native run/owner'):
                    session.run(self.args())
                self.assertEqual(path.read_bytes(), b'belongs to existing startup owner')
                self.assertEqual(self.child.call_count, 0)
                self.assertEqual(self.calls, [])
                path.unlink()

    def test_old_final_receipt_cannot_finish_either_fresh_initialization_marker(self):
        receipt = session.run(self.args())
        count = len(self.calls)
        for marker_name in (session.NATIVE_INITIALIZATION, session.NATIVE_INITIALIZATION_NEXT):
            with self.subTest(marker=marker_name):
                path = self.store / marker_name
                path.write_bytes(b'new startup is not owned by the completed receipt')
                args = self.args()
                args.reconcile = True
                with self.assertRaisesRegex(ValueError, 'unexpected native run/retirement'):
                    session.run(args)
                self.assertEqual(path.read_bytes(), b'new startup is not owned by the completed receipt')
                self.assertEqual(cp.read_json(self.lane / session.RECEIPT), receipt)
                self.assertEqual(len(self.calls), count)
                self.assertEqual(self.child.call_count, 1)
                path.unlink()


if __name__ == '__main__':
    unittest.main()
