"""Missing-evidence admission regressions; no creature or numerical fixtures."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name('audit_memory_readout.py')
spec = importlib.util.spec_from_file_location('readout_audit', SCRIPT)
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class MissingEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.run = Path(self.directory.name)

    def placeholders(self):
        """Presence-only files: preflight must reject before interpreting them."""
        (self.run/'GRAPH-LOCAL.json').write_text('{}')
        for interval in audit.INTERVALS:
            (self.run/f'STEP-{interval:03}-0-TAPE.json').write_text('{}')
            (self.run/f'STEP-{interval:03}.rows.json').write_text('[]')
            (self.run/f'STEP-{interval:03}-0-COEFFICIENT-COPY.json').write_text(
                json.dumps({'trials':[{'gain':gain} for gain in audit.GAINS]}))

    def incomplete(self, native_only=False):
        with patch.object(audit.reference, 'Graph', side_effect=AssertionError('no evaluation allowed')):
            result = audit.audit_run(self.run, native_only=native_only)
        self.assertFalse(result['passed'])
        self.assertFalse(result['complete'])
        self.assertEqual(result['status'], 'incomplete')
        self.assertEqual(result['expected_rows'], 3)
        self.assertEqual(result['evaluated_rows'], 0)
        self.assertEqual(result['coefficient_copy_evaluations'], 0)
        self.assertEqual(result['rows'], [])
        self.assertTrue(result['missing_evidence'])
        return result

    def test_empty_directory_cannot_pass(self):
        result = self.incomplete()
        self.assertIn('GRAPH-LOCAL.json', result['missing_evidence'])
        self.assertEqual(result['expected_coefficient_copy_evaluations'], 15)

    def test_graph_alone_cannot_pass_with_zero_comparisons(self):
        (self.run/'GRAPH-LOCAL.json').write_text('{}')
        result = self.incomplete()
        self.assertEqual(len(result['missing_evidence']), 9)

    def test_missing_one_tape_cannot_count_three_evaluated_rows(self):
        self.placeholders()
        (self.run/'STEP-108-0-TAPE.json').unlink()
        self.assertEqual(self.incomplete()['missing_evidence'], ['STEP-108-0-TAPE.json'])

    def test_missing_native_action_row_cannot_pass(self):
        self.placeholders()
        (self.run/'STEP-110.rows.json').unlink()
        self.assertEqual(self.incomplete()['missing_evidence'], ['STEP-110.rows.json'])

    def test_missing_coefficient_file_is_incomplete(self):
        self.placeholders()
        (self.run/'STEP-102-0-COEFFICIENT-COPY.json').unlink()
        self.assertEqual(self.incomplete()['missing_evidence'],
                         ['STEP-102-0-COEFFICIENT-COPY.json'])

    def test_empty_partial_or_duplicated_population_is_incomplete(self):
        self.placeholders()
        for gains in ([], [-16, -4, 0, 4], [-16, -4, 0, 4, 4]):
            with self.subTest(gains=gains):
                (self.run/'STEP-102-0-COEFFICIENT-COPY.json').write_text(
                    json.dumps({'trials':[{'gain':gain} for gain in gains]}))
                result = self.incomplete()
                self.assertIn('expected each declared gain exactly once', result['missing_evidence'][0])

    def test_explicit_native_only_still_requires_every_tape(self):
        (self.run/'GRAPH-LOCAL.json').write_text('{}')
        result = self.incomplete(native_only=True)
        self.assertEqual(result['mode'], 'native_replay_only')
        self.assertEqual(result['expected_coefficient_copy_evaluations'], 0)
        self.assertEqual(len(result['missing_evidence']), 6)

    def test_cli_incomplete_exit_and_create_only_report_are_explicit(self):
        (self.run/'GRAPH-LOCAL.json').write_text('{}')
        command = [sys.executable, '-I', str(SCRIPT), str(self.run), '--output', 'audit.json']
        completed = subprocess.run(command, text=True, capture_output=True, check=False)
        self.assertEqual(completed.returncode, 2, completed.stderr)
        console = json.loads(completed.stdout)
        saved = json.loads((self.run/'audit.json').read_text())
        self.assertFalse(console['passed'])
        self.assertEqual(console['evaluated_rows'], 0)
        self.assertEqual(console['coefficient_copy_evaluations'], 0)
        self.assertEqual(saved['status'], 'incomplete')
        original = (self.run/'audit.json').read_bytes()
        repeated = subprocess.run(command, text=True, capture_output=True, check=False)
        self.assertNotEqual(repeated.returncode, 0)
        self.assertEqual((self.run/'audit.json').read_bytes(), original)


if __name__ == '__main__':
    unittest.main()
