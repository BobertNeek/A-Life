"""Phase timings remain observational and preserve planner failures."""
import contextlib
import hashlib
import io
import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

import profile_mixed_care_plan as profiling


class PlanningProfileTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.evidence = self.root / "evidence"
        self.evidence.write_bytes(b"immutable fixture evidence")
        self.output = self.root / "profile.json"
        self.host = SimpleNamespace(sha256=lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest())
        self.mixed = SimpleNamespace(host=self.host)
        self.mixed.history = lambda path: self.host.sha256(path)
        self.mixed.main = lambda path: self.mixed.history(path)

    def test_reports_actual_calls_bytes_and_preserves_result(self):
        original = (self.mixed.main, self.mixed.history, self.host.sha256)
        expected = self.host.sha256(self.evidence)
        with profiling.profile_planning(self.mixed, self.output, metadata={"fixture": True}):
            self.assertEqual(self.mixed.main(self.evidence), expected)
        report = json.loads(self.output.read_text())
        self.assertEqual([row["stage"] for row in report["calls"]], ["sha256", "history", "main"])
        self.assertEqual(report["calls"][0]["hashed_file_bytes"], self.evidence.stat().st_size)
        self.assertTrue(report["timings_are_nested"])
        self.assertIsNone(report["exception_type"])
        self.assertEqual((self.mixed.main, self.mixed.history, self.host.sha256), original)

    def test_original_failure_propagates_and_functions_are_restored(self):
        def reject(_):
            raise ValueError("fixture evidence rejected")
        self.mixed.history = reject
        original = (self.mixed.main, self.mixed.history, self.host.sha256)
        with self.assertRaisesRegex(ValueError, "fixture evidence rejected"):
            with profiling.profile_planning(self.mixed, self.output):
                self.mixed.main(self.evidence)
        self.assertEqual(json.loads(self.output.read_text())["exception_type"], "ValueError")
        self.assertEqual((self.mixed.main, self.mixed.history, self.host.sha256), original)

    def test_timing_output_failure_does_not_fail_successful_planning(self):
        original = (self.mixed.main, self.mixed.history, self.host.sha256)
        with patch.object(profiling, "_write_report", side_effect=OSError("fixture full")), \
                contextlib.redirect_stderr(io.StringIO()):
            with profiling.profile_planning(self.mixed, self.output):
                self.assertEqual(self.mixed.main(self.evidence), self.host.sha256(self.evidence))
        self.assertEqual((self.mixed.main, self.mixed.history, self.host.sha256), original)

    def test_script_exit_code_propagates_and_is_recorded(self):
        original = (self.mixed.main, self.mixed.history, self.host.sha256)
        with self.assertRaises(SystemExit) as stopped:
            with profiling.profile_planning(self.mixed, self.output):
                raise SystemExit(7)
        self.assertEqual(stopped.exception.code, 7)
        self.assertEqual(json.loads(self.output.read_text())["exit_code"], 7)
        self.assertEqual((self.mixed.main, self.mixed.history, self.host.sha256), original)


if __name__ == "__main__":
    unittest.main()
