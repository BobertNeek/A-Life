"""Receipt integrity regressions. Run with Python's standard unittest runner."""
import json
import pathlib
import tempfile
import unittest

from summarize_receipts import summarize, validate_coverage


class ReceiptIntegrity(unittest.TestCase):
    def test_declared_matrix_rejects_missing_whole_seed_and_pair(self):
        meta = dict(receipt_format="journal-v2", seed_start=1, seed_count=2,
                    scenes=["food"], arms=["old", "new"])
        complete = {(s, "food", a) for s in [1, 2] for a in ["old", "new"]}
        validate_coverage(meta, complete)
        for missing in [{(1, "food", "old"), (1, "food", "new")},
                        complete - {(2, "food", "new")}]:
            with self.assertRaisesRegex(AssertionError, "coverage"):
                validate_coverage(meta, missing)

    def test_duplicate_journal_filename_rejected_before_read(self):
        with tempfile.TemporaryDirectory() as d:
            path = pathlib.Path(d) / "receipt.json"
            path.write_text(json.dumps(dict(complete=True, trial_files=["x.json", "x.json"])))
            with self.assertRaisesRegex(AssertionError, "duplicate trial filename"):
                summarize(path)

    def test_journal_path_cannot_escape_receipt_directory(self):
        with tempfile.TemporaryDirectory() as d:
            path = pathlib.Path(d) / "receipt.json"
            path.write_text(json.dumps(dict(complete=True, trial_files=["../outside.json"])))
            with self.assertRaisesRegex(AssertionError, "journal path must be local"):
                summarize(path)


if __name__ == "__main__":
    unittest.main()
