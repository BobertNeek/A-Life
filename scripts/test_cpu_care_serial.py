import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import run_cpu_care_serial as cpu


class CpuScheduleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / 'source'
        self.lane = self.root / 'cpu-lane'
        self.source.mkdir()
        self.lane.mkdir()
        self.binary = self.root / 'binary'
        self.binary.write_bytes(b'fixture executable identity')
        self.write_source(self.source, 20)
        self.history = self.root / 'history.json'
        self.write(self.history, {'schema': 1, 'kind': 'mixed-care-seed-history', 'complete': False,
                                 'seed_admission': {'policy': 'known-exclusions', 'historical_freshness': 'unknown'},
                                 'source_sha256': cpu.pins(self.source), 'imports': [],
                                 'reserved_seed_ranges': [[1, 19]], 'consumed_seed_ranges': [[20, 20]],
                                 'assessment_seeds': [29], 'missing_history': 'fixture missing historical evidence'})
        self.plan = {'schema': 1, 'kind': 'native-cpu-care-plan', 'source': str(self.source),
                     'source_sha256': cpu.pins(self.source), 'history': str(self.history),
                     'history_sha256': cpu.mixed.host.sha256(self.history), 'lane': str(self.lane),
                     'binary': str(self.binary), 'binary_sha256': cpu.mixed.host.sha256(self.binary),
                     'scheduler_sha256': cpu.mixed.host.sha256(Path(cpu.__file__)),
                     'schedule': {'schema': 1, 'steps': [{'lesson': 'eat_held_food', 'episodes': 2, 'decisions': 2}]},
                     'seed_range': [30, 33], 'temperature': 1., 'prior_environment': {},
                     'seed_admission': {'policy': 'known-exclusions', 'historical_freshness': 'unknown'}}
        self.calls = []

    def write(self, path, value):
        path.write_text(json.dumps(value))

    def write_source(self, path, seed):
        path.mkdir(exist_ok=True)
        self.write(path / 'cpu-cycle.json', {'seed': seed, 'policy_version': seed})
        for name in ('trained.alife-foundation', 'actor-checkpoint.json', 'value-checkpoint.json'):
            (path / name).write_text(str(seed))

    def command(self, binary, *args):
        self.calls.append(args)
        if args[0] == '--validate-output':
            return {'policy_version': 31, 'episode': {'trained_nourished_meals': 0, 'lesson_completed': False}}
        if args[0] == '--reconcile-pending':
            ledger = cpu.read(self.lane / cpu.LEDGER)
            ledger['cpu_care_attempts'][-1]['status'] = 'failed'
            self.write(self.lane / cpu.LEDGER, ledger)
            return {'dispatch': False}
        values = dict(zip(args[::2], args[1::2])) if args[-1] != '--known-exclusions' else dict(zip(args[:-1:2], args[1:-1:2]))
        self.assertEqual(args[-1], '--known-exclusions')
        output = Path(values['--output'])
        previous = Path(values['--previous'])
        history = cpu.read(Path(values['--seed-history']))
        self.assertFalse(history['complete'])
        self.assertEqual(history['seed_admission']['historical_freshness'], 'unknown')
        self.assertEqual(history['source_sha256'], cpu.pins(previous))
        self.assertEqual(history['assessment_seeds'], [29])
        self.write_source(output, values['--seed'])
        path = self.lane / cpu.LEDGER
        ledger = cpu.read(path) if path.exists() else {'cpu_care_attempts': []}
        ledger['cpu_care_attempts'].append({'status': 'accepted', 'seed': values['--seed'],
            'output': str(output), 'source_sha256': cpu.pins(previous), 'lesson': values['--lesson'],
            'decisions': values['--decisions'], 'native_output_sha256': cpu.pins(output)})
        self.write(path, ledger)
        return {}

    def execute(self):
        with patch.object(cpu, 'command', self.command), patch.object(cpu, 'verify_source'), patch.dict(cpu.os.environ, {}, clear=True):
            return cpu.execute(self.plan)

    def test_resume_rejects_changed_checkout_or_shared_validator(self):
        root = Path(cpu.__file__).resolve().parent.parent
        plan = {**self.plan, 'git_head':'head', 'git_tree':'tree',
                'cargo_lock_sha256':cpu.mixed.host.sha256(root / 'Cargo.lock'),
                'helper_sha256':{name:cpu.mixed.host.sha256(root / 'scripts' / name) for name in cpu.HELPERS}}
        with patch.object(cpu.subprocess, 'check_output', side_effect=['', 'head', 'tree']):
            cpu.verify_source(plan)
        with patch.object(cpu.subprocess, 'check_output', return_value='dirty'):
            with self.assertRaises(ValueError):
                cpu.verify_source(plan)
        plan['helper_sha256']['run_mixed_care_serial.py'] = 'changed'
        with patch.object(cpu.subprocess, 'check_output', side_effect=['', 'head', 'tree']):
            with self.assertRaises(ValueError):
                cpu.verify_source(plan)

    def test_serial_uses_latest_full_native_source_and_keeps_unknown_freshness(self):
        result = self.execute()
        self.assertEqual(result['accepted_episodes'], 2)
        self.assertFalse(result['promoted'])
        self.assertEqual(result['seed_admission']['historical_freshness'], 'unknown')
        dispatched = [c for c in self.calls if c[0] == '--previous']
        self.assertEqual(dispatched[1][1], self.lane / 'episode-00000')
        self.calls.clear()
        self.assertEqual(self.execute(), result)
        self.assertFalse(any(c[0] == '--previous' for c in self.calls))

    def test_recent_policy_resumes_without_retired_payloads_or_initial_source(self):
        self.plan['retention'] = {'mode': 'recent-checkpoints', 'keep': 2}
        self.plan['schedule']['steps'][0]['episodes'] = 4
        result = self.execute()
        for name in ('episode-00000', 'episode-00001'):
            shutil.rmtree(self.lane / name)
        shutil.rmtree(self.source)
        self.calls.clear()
        self.assertEqual(self.execute(), result)
        self.assertFalse(any(c[0] == '--previous' for c in self.calls))
        checked = [Path(c[1]).name for c in self.calls if c[0] == '--validate-output']
        self.assertEqual(checked, ['episode-00002', 'episode-00003'])
        # Old seed and source links remain checked even without old payloads.
        ledger = cpu.read(self.lane / cpu.LEDGER)
        ledger['cpu_care_attempts'][1]['source_sha256']['actor-checkpoint.json'] = '0' * 64
        self.write(self.lane / cpu.LEDGER, ledger)
        with self.assertRaises(ValueError):
            self.execute()

    def test_recent_policy_rejects_missing_latest_state_and_keeps_failed_seeds(self):
        self.plan['retention'] = {'mode': 'recent-checkpoints', 'keep': 2}
        self.execute()
        (self.lane / 'episode-00001' / 'actor-checkpoint.json').unlink()
        self.calls.clear()
        with self.assertRaises(FileNotFoundError):
            self.execute()
        self.assertFalse(any(c[0] == '--previous' for c in self.calls))

    def test_pending_attempt_is_reconciled_without_seed_retry_or_slot_advance(self):
        pending = {'status': 'pending', 'seed': 30, 'output': str(self.lane / 'episode-00000'),
                   'source_sha256': cpu.pins(self.source), 'lesson': 'eat_held_food', 'decisions': 2}
        self.write(self.lane / cpu.LEDGER, {'cpu_care_attempts': [pending]})
        self.execute()
        dispatched = [c for c in self.calls if c[0] == '--previous']
        self.assertEqual(dict(zip(dispatched[0][:-1:2], dispatched[0][1:-1:2]))['--seed'], 31)
        self.assertEqual(cpu.read(self.lane / cpu.LEDGER)['cpu_care_attempts'][0]['status'], 'failed')

    def test_changed_sources_outputs_and_binary_fail_before_dispatch(self):
        self.execute()
        self.calls.clear()
        accepted = self.lane / 'episode-00000' / 'actor-checkpoint.json'
        accepted.write_text('changed')
        with self.assertRaises(ValueError):
            self.execute()
        self.assertFalse(any(c[0] == '--previous' for c in self.calls))
        self.binary.write_text('changed executable')
        with self.assertRaises(ValueError):
            self.execute()

    def test_retirement_report_preserves_seed_provenance_and_recent_checkpoints(self):
        import cpu_retention_report
        self.plan['retention'] = {'mode': 'recent-checkpoints', 'keep': 2}
        self.write(self.lane / cpu.PLAN, self.plan)
        attempts = []
        for index in range(3):
            directory = self.lane / f'episode-{index}'
            directory.mkdir()
            self.write(directory / 'cpu-cycle.json', {'policy_version': index})
            self.write(directory / 'care-seed-admission.json', {'historical_freshness': 'unknown'})
            (directory / 'care-replay.json').write_bytes(b'diagnostic replay')
            attempts.append({'status': 'accepted', 'output': str(directory)})
        self.write(self.lane / cpu.LEDGER, {'cpu_care_attempts': attempts})
        before = {str(p): p.read_bytes() for p in self.lane.rglob('*') if p.is_file()}
        result = cpu_retention_report.report(self.lane)
        candidates = result['retirement_candidates']
        self.assertEqual(len(candidates), 1)
        self.assertEqual([Path(f['path']).name for f in candidates[0]['files']], ['care-replay.json'])
        self.assertEqual(result['retained_checkpoints'], [a['output'] for a in attempts[-2:]])
        self.assertEqual(result['candidate_logical_bytes'], len(b'diagnostic replay'))
        self.assertFalse(result['physical_savings_measured'])
        self.assertEqual(before, {str(p): p.read_bytes() for p in self.lane.rglob('*') if p.is_file()})

    def test_cpu_blocked_route_is_explicit_and_gpu_schedule_stays_unchanged(self):
        schedule = {'schema': 1, 'steps': [{'lesson': 'obstacle_navigation', 'episodes': 2, 'decisions': 64}]}
        rows = cpu.mixed.schedule_rows(schedule, supported_lessons=cpu.CPU_LESSONS)
        self.assertEqual(rows, [{'lesson': 'obstacle_navigation', 'decisions': 64}] * 2)
        with self.assertRaises(ValueError):
            cpu.mixed.schedule_rows(schedule)

    def test_unsupported_curriculum_and_inconsistent_dose_rejected(self):
        for lesson, decisions in [('obstacle_navigation', 2), ('feeding', 0), ('eat_held_food', 129)]:
            plan = copy.deepcopy(self.plan)
            plan['schedule']['steps'][0].update(lesson=lesson, decisions=decisions)
            with self.assertRaises(ValueError):
                cpu.mixed.schedule_rows(plan['schedule'])

    def test_windows_resume_paths_accept_verbatim_drive_unc_and_missing_attempts(self):
        spellings = [(r'C:\Lane\episode-00000', r'\\?\C:\Lane\episode-00000'),
                     (r'c:\lane\episode-00000', r'\\?\C:\Lane\episode-00000'),
                     (r'\\server\share\Lane\episode-00000', r'\\?\UNC\server\share\Lane\episode-00000')]
        key = cpu.path_key
        with patch.object(cpu.os.path, 'samefile', side_effect=FileNotFoundError), \
             patch.object(cpu, 'path_key', side_effect=lambda p: key(p, windows=True)):
            for ordinary, verbatim in spellings:
                self.assertTrue(cpu.same_path(ordinary, verbatim))
                self.assertFalse(cpu.same_path(ordinary, verbatim + '-different'))
        self.assertNotEqual(key('/lane/Episode', windows=False), key('/lane/episode', windows=False))


if __name__ == '__main__':
    unittest.main()
