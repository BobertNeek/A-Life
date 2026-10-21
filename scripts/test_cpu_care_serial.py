import copy
import json
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
        with patch.object(cpu, 'command', self.command), patch.dict(cpu.os.environ, {}, clear=True):
            return cpu.execute(self.plan)

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

    def test_unsupported_curriculum_and_inconsistent_dose_rejected(self):
        for lesson, decisions in [('obstacle_navigation', 2), ('feeding', 0), ('eat_held_food', 129)]:
            plan = copy.deepcopy(self.plan)
            plan['schedule']['steps'][0].update(lesson=lesson, decisions=decisions)
            with self.assertRaises(ValueError):
                cpu.mixed.schedule_rows(plan['schedule'])


if __name__ == '__main__':
    unittest.main()
