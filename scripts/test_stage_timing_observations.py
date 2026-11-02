"""Synthetic receipt admission regressions; no training, device or live data."""

import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import summarize_stage_timing as summary


COUNTERS = (
    "actual_decisions", "captured_replay_rows", "replay_segment_count",
    "uncaptured_world_ticks_between_rows",
)
SCRIPT = Path(summary.__file__).resolve()


def receipt():
    return {
        "schema_version": 1, "time_basis": summary.TIME_BASIS,
        "status": "complete", "error": None,
        "context": {
            "actual_decisions": 2, "captured_replay_rows": 3,
            "replay_segment_count": 1, "uncaptured_world_ticks_between_rows": 0,
            "terminal_death_tick": None,
            "source_head_declared": "synthetic", "binary_sha256_declared": "synthetic",
        },
        "native_total_seconds": 10, "exclusive_covered_seconds": 6,
        "unassigned_seconds": 4,
        "spans": [
            {"id": 1, "name": "capture", "parent_id": 0, "call_count": 1,
             "status": "complete", "elapsed_seconds": 2, "exclusive_seconds": 2},
            {"id": 0, "name": "collection", "parent_id": None, "call_count": 1,
             "status": "complete", "elapsed_seconds": 6, "exclusive_seconds": 4},
        ],
    }


def report(value):
    summary.validate(value)
    return summary.summarize([
        {"receipt": value, "content_sha256": "synthetic", "paths": ["synthetic"]},
    ], 1)


class ObservationAdmissionTests(unittest.TestCase):
    def test_counters_preserve_observed_zero_and_nonnegative_integers(self):
        for field in COUNTERS:
            for value in (0, 3, 2**64 - 1):
                with self.subTest(field=field, value=value):
                    item = receipt()
                    item["context"][field] = value
                    result = report(item)
                    self.assertEqual(result["missing_observation_fields"][field], 0)
                    self.assertEqual(result["receipts"][0]["context"][field], value)

    def test_present_malformed_counters_fail_admission_without_coercion(self):
        for field in COUNTERS:
            for value in (None, True, False, -1, 0.0, 2.5, "0", "unknown", [], {}):
                with self.subTest(field=field, value=value):
                    item = receipt()
                    item["context"][field] = value
                    with self.assertRaisesRegex(summary.ReceiptError, f"context\\.{field}"):
                        summary.validate(item)

    def test_death_observation_preserves_null_and_integer_ticks(self):
        for value in (None, 0, 123, 2**64 - 1):
            with self.subTest(value=value):
                item = receipt()
                item["context"]["terminal_death_tick"] = value
                result = report(item)
                self.assertEqual(result["missing_observation_fields"]["terminal_death_tick"], 0)
                self.assertEqual(result["receipts"][0]["context"]["terminal_death_tick"], value)

    def test_malformed_present_death_tick_fails_admission(self):
        for value in (True, False, -1, 0.0, 2.5, "0", [], {}):
            with self.subTest(value=value):
                item = receipt()
                item["context"]["terminal_death_tick"] = value
                with self.assertRaisesRegex(summary.ReceiptError, "context\\.terminal_death_tick"):
                    summary.validate(item)

    def test_older_and_early_error_receipts_keep_missing_observations_unknown(self):
        for status in ("complete", "error"):
            with self.subTest(status=status):
                item = receipt()
                item.update(status=status, context={})
                if status == "error":
                    item["error"] = "synthetic early failure"
                    for span in item["spans"]:
                        span["status"] = "interrupted"
                result = report(item)
                self.assertEqual(result["missing_observation_fields"],
                                 dict.fromkeys(summary.OBSERVATIONS, 1))
                self.assertIn("Missing observation fields (unknown):", summary.render_text(result))

    def test_one_missing_field_does_not_hide_other_known_observations(self):
        for field in summary.OBSERVATIONS:
            with self.subTest(field=field):
                item = receipt()
                del item["context"][field]
                missing = report(item)["missing_observation_fields"]
                self.assertEqual(missing[field], 1)
                self.assertEqual(sum(missing.values()), 1)

    def test_future_context_fields_remain_compatible(self):
        item = receipt()
        future = {"unknown_counter": None, "flag": True, "label": "unknown",
                  "signed_value": -1, "fraction": 2.5, "values": [None, True, "x"],
                  "details": {"counter": None}}
        item["context"]["future_context"] = copy.deepcopy(future)
        item["context"]["future_observation_count"] = None
        context = report(item)["receipts"][0]["context"]
        self.assertEqual(context["future_context"], future)
        self.assertIsNone(context["future_observation_count"])

    def test_host_second_units_and_exclusive_stage_accounting_are_preserved(self):
        result = report(receipt())
        self.assertEqual(result["time_basis"], summary.TIME_BASIS)
        stages = result["cohorts"]["complete"]["stages"]
        self.assertEqual(stages["collection"]["exclusive_seconds"]["median"], 4)
        self.assertEqual(stages["capture"]["exclusive_seconds"]["median"], 2)
        self.assertIn("Exclusive host-wall seconds", summary.render_text(result))

    def test_cli_rejects_malformed_observations_without_partial_output_or_file_writes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            good = root / "good.json"
            bad = root / "bad.json"
            good.write_text(json.dumps(receipt()), encoding="utf-8")
            for field, value in ((name, None) for name in COUNTERS):
                item = receipt()
                item["context"][field] = value
                bad.write_text(json.dumps(item), encoding="utf-8")
                before = {path.name: path.read_bytes() for path in root.iterdir()}
                for output_format in ("text", "json"):
                    with self.subTest(field=field, output_format=output_format):
                        result = subprocess.run(
                            [sys.executable, "-B", str(SCRIPT), str(good), str(bad),
                             "--format", output_format], capture_output=True, text=True, check=False,
                        )
                        self.assertEqual(result.returncode, 2)
                        self.assertEqual(result.stdout, "")
                        self.assertIn(f"context.{field}", result.stderr)
                        self.assertIn("no partial summary emitted", result.stderr)
                        self.assertEqual({path.name: path.read_bytes() for path in root.iterdir()}, before)

    def test_cli_preserves_nullable_death_and_unknown_future_context(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "stage-timing.json"
            item = receipt()
            item["context"]["future_counter"] = None
            path.write_text(json.dumps(item), encoding="utf-8")
            result = subprocess.run([sys.executable, "-B", str(SCRIPT), str(path), "--format", "json"],
                                    capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            parsed = json.loads(result.stdout)
            self.assertIsNone(parsed["receipts"][0]["context"]["terminal_death_tick"])
            self.assertIsNone(parsed["receipts"][0]["context"]["future_counter"])
            self.assertEqual(parsed["missing_observation_fields"], dict.fromkeys(summary.OBSERVATIONS, 0))


if __name__ == "__main__":
    unittest.main()
