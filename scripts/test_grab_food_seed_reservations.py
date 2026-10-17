"""Standard-library allocation fixtures; no binaries, GPU, or training."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import grab_food_seed_reservations as seeds


class SeedReservationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.history_path = self.root / "history.json"
        self.ledger_path = self.root / seeds.LEDGER_NAME
        self.hashes = {name: hashlib.sha256(name.encode()).hexdigest() for name in seeds.SOURCE_FILES}
        self.history = {"schema": 1, "kind": "grab-food-seed-history", "complete": True,
                        "source_sha256": self.hashes, "training_seed_ranges": [[10, 10]],
                        "assessment_seeds": [5], "evidence": "complete synthetic allocation receipt"}
        self.write_history()

    def write_history(self, changes=None):
        self.history_path.write_text(json.dumps({**self.history, **(changes or {})}))

    def prepare(self, first=20, count=3, assessment=5):
        return seeds.prepare(self.history_path, self.ledger_path, self.hashes, 10, first, count, assessment)

    def commit(self, first=20, count=3, assessment=5):
        prepared, digest = self.prepare(first, count, assessment)
        seeds.commit(self.ledger_path, prepared, {"output": str(self.root / f"run-{first}"),
            "training_seed_range": [first, first + count - 1], "assessment_seed": assessment,
            "source_sha256": self.hashes, "history_path": str(self.history_path), "history_sha256": digest})

    def test_fresh_history_prepares_without_reserving_and_hashes_parsed_bytes(self):
        before = self.history_path.read_bytes()
        prepared, digest = self.prepare()
        self.assertEqual(prepared["training_seed_ranges"], [[10, 10], [20, 22]])
        self.assertEqual(digest, hashlib.sha256(before).hexdigest())
        self.assertFalse(self.ledger_path.exists())
        self.assertEqual(before, self.history_path.read_bytes())

    def test_stale_history_cannot_erase_committed_unsealed_allocation(self):
        self.commit()
        before = self.ledger_path.read_bytes()
        for first, count in ((20, 1), (22, 1), (19, 2), (21, 4), (18, 8)):
            with self.subTest(first=first, count=count), self.assertRaisesRegex(ValueError, "previously reserved"):
                self.prepare(first, count)
        self.assertEqual(before, self.ledger_path.read_bytes())
        self.assertEqual(self.prepare(23)[0]["training_seed_ranges"], [[10, 10], [20, 25]])
        self.assertEqual(self.prepare(17)[0]["training_seed_ranges"], [[10, 10], [17, 22]])

    def test_normal_continuation_preserves_prior_allocations_and_assessment(self):
        self.commit()
        self.commit(23, 2)
        ledger = json.loads(self.ledger_path.read_text())
        self.assertEqual(ledger["training_seed_ranges"], [[10, 10], [20, 24]])
        self.assertEqual(ledger["assessment_seeds"], [5])
        self.assertEqual(len(ledger["allocations"]), 2)

    def test_prior_partial_failed_and_unused_bounded_allocations_are_excluded(self):
        self.write_history({"training_seed_ranges": [[10, 10], [20, 100]]})
        for first in (20, 57, 100):
            with self.subTest(first=first), self.assertRaises(ValueError):
                self.prepare(first, 1)
        self.prepare(101, 1)

    def test_assessment_exclusions_are_symmetric_and_repetition_is_allowed(self):
        self.write_history({"assessment_seeds": [5, 25]})
        with self.assertRaisesRegex(ValueError, "assessment seed"):
            self.prepare(20, 6)
        with self.assertRaisesRegex(ValueError, "previously reserved training"):
            self.prepare(20, 3, 10)
        self.prepare(20, 3, 5)
        self.commit()
        with self.assertRaisesRegex(ValueError, "previously reserved training"):
            self.prepare(30, 2, 21)

    def test_missing_malformed_incomplete_and_mismatched_history_fail_closed(self):
        for data in ("{", "[]", '{"schema":1,"schema":1}', "null"):
            self.history_path.write_text(data)
            with self.subTest(data=data), self.assertRaises(ValueError):
                self.prepare()
        for change in ({"schema": True}, {"schema": 2}, {"complete": False}, {"complete": 1},
                       {"evidence": ""}, {"training_seed_ranges": []}, {"source_sha256": {}},
                       {"source_sha256": {**self.hashes, "cycle.json": "0" * 64}}):
            self.write_history(change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.prepare()
        self.history_path.unlink()
        with self.assertRaises(FileNotFoundError):
            self.prepare()
        self.assertFalse(self.ledger_path.exists())

    def test_strict_uint64_and_range_types_reject_bool_float_and_overflow(self):
        for pair in ([True, 10], [10, 10.0], [0, 10], [11, 10], [10, seeds.MAX_SEED + 1], [10], "10,10"):
            self.write_history({"training_seed_ranges": [pair]})
            with self.subTest(pair=pair), self.assertRaises(ValueError):
                self.prepare()
        self.write_history()
        for first, count, assessment in ((0, 1, 5), (True, 1, 5), (20.0, 1, 5),
                (20, 0, 5), (20, True, 5), (20, seeds.MAX_ENTRIES + 1, 5),
                (seeds.MAX_SEED, 2, 5), (20, 1, False), (20, 1, seeds.MAX_SEED + 1)):
            with self.subTest(first=first, count=count, assessment=assessment), self.assertRaises(ValueError):
                self.prepare(first, count, assessment)
        self.prepare(seeds.MAX_SEED, 1)
        self.prepare(seeds.MAX_SEED - seeds.MAX_ENTRIES + 1, seeds.MAX_ENTRIES)

    def test_bounded_input_lists_and_document_size_fail_closed(self):
        with patch.object(seeds, "MAX_ENTRIES", 2):
            self.write_history({"assessment_seeds": [1, 2, 3]})
            with self.assertRaisesRegex(ValueError, "bounded lists"):
                self.prepare(count=1)
        self.write_history()
        with patch.object(seeds, "MAX_DOCUMENT_BYTES", 10), self.assertRaisesRegex(ValueError, "document size"):
            self.prepare()

    def test_malformed_or_symlinked_ledger_is_not_replaced(self):
        for value in ({}, [], {"schema": 1, "kind": "wrong"},
                      {"schema": 1, "kind": "grab-food-seed-ledger", "training_seed_ranges": [],
                       "assessment_seeds": [], "allocations": [None]}):
            self.ledger_path.write_text(json.dumps(value))
            before = self.ledger_path.read_bytes()
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.prepare()
            self.assertEqual(before, self.ledger_path.read_bytes())
        self.ledger_path.unlink()
        self.ledger_path.symlink_to(self.history_path)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.prepare()

    def test_output_document_bound_stops_before_creating_ledger(self):
        prepared, digest = self.prepare()
        allocation = {"output": str(self.root / "run"), "training_seed_range": [20, 22],
                      "assessment_seed": 5, "source_sha256": self.hashes,
                      "history_path": str(self.history_path), "history_sha256": digest}
        with patch.object(seeds, "MAX_DOCUMENT_BYTES", 10), self.assertRaisesRegex(ValueError, "document size"):
            seeds.commit(self.ledger_path, prepared, allocation)
        self.assertFalse(self.ledger_path.exists())
        self.assertEqual(list(self.root.glob("*.tmp")), [])

    def test_allocation_receipt_excludes_range_even_if_summary_omits_it(self):
        self.commit()
        ledger = json.loads(self.ledger_path.read_text())
        ledger["training_seed_ranges"] = []
        self.ledger_path.write_text(json.dumps(ledger))
        with self.assertRaisesRegex(ValueError, "previously reserved"):
            self.prepare(21, 1)

    def test_fsync_or_replace_failure_preserves_existing_ledger_and_history(self):
        self.commit()
        before = self.ledger_path.read_bytes(), self.history_path.read_bytes()
        for operation in ("fsync", "replace"):
            with self.subTest(operation=operation), patch.object(seeds.os, operation, side_effect=OSError("fixture disk fault")):
                with self.assertRaisesRegex(OSError, "disk fault"):
                    self.commit(23)
            self.assertEqual(before, (self.ledger_path.read_bytes(), self.history_path.read_bytes()))
            self.assertEqual(list(self.root.glob("*.tmp")), [])

    @unittest.skipIf(seeds.os.name == "nt", "directory fsync is POSIX-only")
    def test_directory_fsync_failure_keeps_conservative_surviving_reservation(self):
        real_fsync = seeds.os.fsync
        calls = []

        def fail_directory(descriptor):
            calls.append(descriptor)
            if len(calls) == 2:
                raise OSError("fixture directory flush fault")
            real_fsync(descriptor)

        with patch.object(seeds.os, "fsync", side_effect=fail_directory):
            with self.assertRaisesRegex(OSError, "directory flush fault"):
                self.commit()
        with self.assertRaisesRegex(ValueError, "previously reserved"):
            self.prepare()


if __name__ == "__main__":
    unittest.main()
