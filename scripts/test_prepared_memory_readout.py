"""Research auditor completeness guards; no neural or creature fixtures."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('prepared_readout_audit', Path(__file__).with_name('check_prepared_memory_readout.py'))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class CompletenessTests(unittest.TestCase):
    def test_optimized_execution_is_rejected_before_auditing(self):
        script = Path(__file__).with_name('check_prepared_memory_readout.py')
        with tempfile.TemporaryDirectory() as directory:
            for flags, setting in [(['-O'], None), ([], '1')]:
                with self.subTest(flags=flags, pythonoptimize=setting):
                    env = dict(os.environ)
                    env.pop('PYTHONOPTIMIZE', None)
                    if setting is not None:
                        env['PYTHONOPTIMIZE'] = setting
                    result = subprocess.run(
                        [sys.executable, *flags, str(script), directory],
                        env=env, capture_output=True, text=True, check=False,
                    )
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn('optimized Python is unsupported', result.stderr)
                    self.assertEqual(list(Path(directory).iterdir()), [])

    def test_graph_only_never_reaches_evaluation_or_reports_success(self):
        for coefficient in (None, 0, 4):
            with self.subTest(coefficient=coefficient), tempfile.TemporaryDirectory() as directory:
                run = Path(directory)
                (run/'REQUEST.json').write_text(json.dumps({'prepared_readout_coefficient':coefficient}))
                (run/'GRAPH-LOCAL.json').write_text('{}')
                with patch.object(audit.reference,'Graph',side_effect=AssertionError('must preflight')):
                    with self.assertRaisesRegex(ValueError,'incomplete expected evidence'):
                        audit.audit(run)
                self.assertEqual(len(list(run.iterdir())),2)

    def test_missing_intervention_file_never_qualifies_native_only(self):
        with tempfile.TemporaryDirectory() as directory:
            run = Path(directory)
            (run/'REQUEST.json').write_text(json.dumps({'prepared_readout_coefficient':0}))
            names = ['GRAPH-LOCAL.json','PREPARED-BIRTH.cp.json','FINAL.cp.json',
                     'FOUNDER-ADAM-BIRTH.json','FOUNDER-ADAM-FINAL.json','GENOME.json',
                     'READOUT-CONSTRUCTION.json']
            for interval in audit.INTERVALS:
                names += [f'STEP-{interval:03}-0-TAPE.json',f'STEP-{interval:03}.rows.json',
                          f'STEP-{interval:03}-0-PREPARED-READOUT-COPY.json']
            for name in names:
                if name != 'STEP-108-0-PREPARED-READOUT-COPY.json':
                    (run/name).write_text('{}')
            with patch.object(audit.reference,'Graph',side_effect=AssertionError('must preflight')):
                with self.assertRaisesRegex(ValueError,'STEP-108-0-PREPARED-READOUT-COPY.json'):
                    audit.audit(run)


if __name__ == '__main__':
    unittest.main()
