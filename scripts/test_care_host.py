"""Shared host imports and pinned-helper admission, without native execution."""
import datetime as dt
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

import care_host
import run_cpu_care_serial as cpu


class SharedCareHostTests(unittest.TestCase):
    def test_owned_child_is_stopped_when_the_guard_aborts(self):
        process = Mock()
        process.poll.return_value = None
        guard = Mock(side_effect=[None, RuntimeError('fixture guard stop')])
        deadline = dt.datetime.now(care_host.UTC) + dt.timedelta(minutes=1)
        with tempfile.TemporaryDirectory() as temporary, \
                patch.object(care_host.subprocess, 'Popen', return_value=process), \
                patch.object(care_host, 'stop_owned') as stop:
            with self.assertRaisesRegex(RuntimeError, 'fixture guard stop'):
                care_host.run_child(['not-executed'], Path(temporary) / 'child',
                                    deadline, guard, {})
            stop.assert_called_once_with(process)

    def test_live_hosts_import_without_the_held_food_cli(self):
        # Refuse the old CLI at import resolution, rather than merely inspecting
        # an import statement: all live hosts must remain usable without it.
        script = '''
import importlib.abc
import sys
class RefuseHeldFoodCli(importlib.abc.MetaPathFinder):
    def find_spec(self, fullname, path, target=None):
        if fullname == 'run_eat_held_food_night':
            raise AssertionError('live care still imports the held-food CLI')
sys.meta_path.insert(0, RefuseHeldFoodCli())
import run_mixed_care_serial
import run_cpu_care_serial
import run_current_care
import run_continuous_care
import compare_cpu_care_rounds
'''
        environment = os.environ.copy()
        environment['PYTHONPATH'] = str(Path(__file__).resolve().parent)
        result = subprocess.run([sys.executable, '-B', '-c', script],
                                env=environment, capture_output=True, text=True,
                                timeout=30, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_cpu_plan_refuses_changed_shared_host_before_dispatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            scripts = root / 'scripts'
            scripts.mkdir()
            (root / 'Cargo.lock').write_bytes(b'synthetic locked dependencies')
            for name in cpu.HELPERS:
                (scripts / name).write_bytes(b'synthetic pinned helper: ' + name.encode())
            plan = {'git_head': 'fixture-head', 'git_tree': 'fixture-tree',
                    'cargo_lock_sha256': cpu.mixed.host.sha256(root / 'Cargo.lock'),
                    'helper_sha256': {name: cpu.mixed.host.sha256(scripts / name)
                                      for name in cpu.HELPERS}}
            git = ['', 'fixture-head', 'fixture-tree']
            with patch.object(cpu, '__file__', str(scripts / 'run_cpu_care_serial.py')), \
                    patch.object(cpu.subprocess, 'check_output', side_effect=git):
                cpu.verify_source(plan)
            shared = scripts / 'care_host.py'
            self.assertTrue(shared.is_file(), 'new plans must pin the imported host library')
            shared.write_bytes(shared.read_bytes() + b'changed after planning')
            with patch.object(cpu, '__file__', str(scripts / 'run_cpu_care_serial.py')), \
                    patch.object(cpu.subprocess, 'check_output', side_effect=git), \
                    self.assertRaisesRegex(ValueError, 'pinned scheduling/admission helpers changed'):
                cpu.verify_source(plan)


if __name__ == '__main__':
    unittest.main()
