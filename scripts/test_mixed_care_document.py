"""Compact ledger persistence, without changing recovery data or size limits."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import grab_food_seed_reservations as seeds


class CompactDocumentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / 'ledger.json'
        self.value = {'runs': [{'seed': 202610220672, 'age': 2**60 + 1,
                               'value': 0.25, 'note': 'spaces stay\n雪',
                               'pending': None, 'accepted': True}]}

    def test_lossless_compaction_and_legacy_default(self):
        seeds.write_document(self.path, self.value)
        pretty = self.path.read_bytes()
        self.assertEqual(pretty, (json.dumps(self.value, indent=2, allow_nan=False) + '\n').encode())
        seeds.write_document(self.path, self.value, compact=True)
        compact = self.path.read_bytes()
        self.assertLess(len(compact), len(pretty))
        self.assertEqual(json.loads(compact), self.value)
        self.assertIsInstance(json.loads(compact)['runs'][0]['age'], int)

    def test_compact_document_fits_same_bound(self):
        size = len((json.dumps(self.value, separators=(',', ':')) + '\n').encode())
        with patch.object(seeds, 'MAX_DOCUMENT_BYTES', size):
            with self.assertRaises(ValueError):
                seeds.write_document(self.path, self.value)
            seeds.write_document(self.path, self.value, compact=True)
        self.assertEqual(json.loads(self.path.read_bytes()), self.value)

    def test_oversize_preserves_previous_file(self):
        self.path.write_bytes(b'previous')
        with patch.object(seeds, 'MAX_DOCUMENT_BYTES', 1), self.assertRaises(ValueError):
            seeds.write_document(self.path, self.value, compact=True)
        self.assertEqual(self.path.read_bytes(), b'previous')
        self.assertEqual(list(self.path.parent.glob('*.tmp')), [])

    def test_failed_replace_preserves_previous_file_and_cleans_temp(self):
        self.path.write_bytes(b'previous')
        with patch.object(seeds.os, 'replace', side_effect=OSError('fixture')), self.assertRaises(OSError):
            seeds.write_document(self.path, self.value, compact=True)
        self.assertEqual(self.path.read_bytes(), b'previous')
        self.assertEqual(list(self.path.parent.glob('*.tmp')), [])

    def test_file_fsync_precedes_replace(self):
        events = []
        real_replace = seeds.os.replace
        def replace(source, destination):
            events.append('replace')
            return real_replace(source, destination)
        with patch.object(seeds.os, 'fsync', side_effect=lambda fd: events.append('fsync')), \
                patch.object(seeds.os, 'replace', side_effect=replace):
            seeds.write_document(self.path, self.value, compact=True)
        self.assertEqual(events[:2], ['fsync', 'replace'])
        if seeds.os.name != 'nt':
            self.assertEqual(events[-1], 'fsync')

    def test_symlink_rejected_and_nonfinite_rejected(self):
        target = self.path.parent / 'target.json'
        target.write_bytes(b'previous')
        self.path.symlink_to(target)
        with self.assertRaises(ValueError):
            seeds.write_document(self.path, self.value, compact=True)
        self.assertEqual(target.read_bytes(), b'previous')
        self.path.unlink()
        with self.assertRaises(ValueError):
            seeds.write_document(self.path, {'value': float('nan')}, compact=True)
        self.assertFalse(self.path.exists())


if __name__ == '__main__':
    unittest.main()
