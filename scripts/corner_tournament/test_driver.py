"""Driver-only tests; the mock executable is not a creature or policy simulation."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("corner_driver", Path(__file__).with_name("run.py"))
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)

class CreateOnlyTests(unittest.TestCase):
    def test_existing_job_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as root:
            directory = Path(root) / "job"
            directory.mkdir()
            sentinel = directory / "original"
            sentinel.write_bytes(b"preserve")
            with self.assertRaises(FileExistsError):
                driver.run("never-executed", {}, directory)
            self.assertEqual(sentinel.read_bytes(), b"preserve")

    def test_success_exit_without_native_marker_is_not_completion(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as root:
            directory = Path(root) / "job"
            with patch.object(driver.subprocess, "run", return_value=type("Result", (), {"returncode": 0})()):
                with self.assertRaises(RuntimeError):
                    driver.run("mock-not-native", {}, directory)
            result = json.loads((directory / "process.json").read_text())
            self.assertFalse(result["complete"])
            self.assertEqual(result["returncode"], 0)

if __name__ == "__main__":
    unittest.main()
