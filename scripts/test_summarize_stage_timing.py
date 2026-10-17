"""Synthetic-only stdlib regressions; no training, Rust, GPU or live receipts."""

import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import summarize_stage_timing as summary


def span(identifier=0, name="collection", elapsed=6, exclusive=6, parent=None,
         status="complete"):
    return {"id": identifier, "name": name, "parent_id": parent, "call_count": 1,
            "status": status, "elapsed_seconds": elapsed, "exclusive_seconds": exclusive}


def fixture():
    # Recorder close-order: child before parent; never sum both inclusive values.
    return {"schema_version": 1, "time_basis": summary.TIME_BASIS,
            "status": "complete", "error": None,
            "context": {"source_head_declared": "synthetic-source",
                        "binary_sha256_declared": "synthetic-binary", "seed": 7,
                        "actual_decisions": 16, "terminal_death_tick": None},
            "native_total_seconds": 10, "exclusive_covered_seconds": 6,
            "unassigned_seconds": 4,
            "spans": [span(1, "capture", 2, 2, 0), span(0, exclusive=4)]}


def entries(*receipts):
    return [{"receipt": receipt, "content_sha256": str(index),
             "paths": [f"SYNTHETIC/{index}/stage-timing.json"]}
            for index, receipt in enumerate(receipts)]


class TimingSummaryTests(unittest.TestCase):
    def setUp(self):
        self.receipt = fixture()

    def invalid(self, mutate):
        mutate(self.receipt)
        with self.assertRaises(summary.ReceiptError):
            summary.validate(self.receipt)

    def test_postorder_nested_exclusive_accounting(self):
        summary.validate(self.receipt)
        result = summary.summarize(entries(self.receipt), 1)
        stages = result["cohorts"]["complete"]["stages"]
        self.assertEqual(stages["collection"]["exclusive_seconds"]["median"], 4)
        self.assertEqual(stages["capture"]["exclusive_seconds"]["median"], 2)
        self.assertEqual(result["cohorts"]["complete"]["totals"]
                         ["exclusive_covered_seconds"]["median"], 6)

    def test_parent_first_input_also_valid(self):
        self.receipt["spans"].reverse()
        summary.validate(self.receipt)

    def test_unknown_stage_name_preserved(self):
        self.receipt["spans"][0]["name"] = "future_stage"
        summary.validate(self.receipt)
        self.assertIn("future_stage", summary.cohort_summary([self.receipt])["stages"])

    def test_repeated_stage_samples_aggregate_per_receipt(self):
        self.receipt["spans"][0]["name"] = "collection"
        summary.validate(self.receipt)
        stage = summary.cohort_summary([self.receipt])["stages"]["collection"]
        self.assertEqual(stage["exclusive_seconds"]["count"], 1)
        self.assertEqual(stage["exclusive_seconds"]["median"], 6)
        self.assertEqual(stage["calls"], 2)

    def test_absent_stage_is_null_not_zero(self):
        stage = summary.cohort_summary([self.receipt])["stages"]["ppo"]
        self.assertEqual(stage["missing_receipts"], 1)
        self.assertEqual(stage["zero_receipts"], 0)
        self.assertIsNone(stage["exclusive_seconds"]["median"])
        self.assertIsNone(stage["exclusive_seconds"]["p95"])

    def test_present_zero_is_observed(self):
        self.receipt.update(native_total_seconds=0, exclusive_covered_seconds=0,
                            unassigned_seconds=0, spans=[span(name="ppo", elapsed=0, exclusive=0)])
        summary.validate(self.receipt)
        stage = summary.cohort_summary([self.receipt])["stages"]["ppo"]
        self.assertEqual(stage["observed_receipts"], 1)
        self.assertEqual(stage["missing_receipts"], 0)
        self.assertEqual(stage["zero_receipts"], 1)
        self.assertEqual(stage["exclusive_seconds"]["median"], 0)

    def test_missing_receipt_excluded_from_stage_distribution(self):
        other = copy.deepcopy(self.receipt)
        other.update(spans=[], exclusive_covered_seconds=0, unassigned_seconds=10)
        summary.validate(other)
        stage = summary.cohort_summary([self.receipt, other])["stages"]["collection"]
        self.assertEqual(stage["exclusive_seconds"]["median"], 4)
        self.assertEqual(stage["missing_receipts"], 1)

    def test_nearest_rank_p95_and_median(self):
        result = summary.stats(list(range(1, 21)))
        self.assertEqual(result["median"], 10.5)
        self.assertEqual(result["p95"], 19)
        self.assertEqual(summary.stats([5])["p95"], 5)

    def test_empty_statistics_all_unknown(self):
        self.assertEqual(summary.stats([]),
                         {"count": 0, "min": None, "median": None, "p95": None, "max": None})

    def test_finite_large_median_does_not_overflow(self):
        self.assertEqual(summary.stats([1e308, 1e308])["median"], 1e308)

    def test_canonical_equivalent_zeroes_and_numeric_values(self):
        self.assertEqual(summary.canonical_json({"a": -0.0, "b": [1.0]}),
                         summary.canonical_json({"b": [1], "a": 0}))

    def test_partial_error_separate_cohort(self):
        other = copy.deepcopy(self.receipt)
        other.update(status="error", error="SYNTHETIC failure")
        other["context"] = {"seed": 7}
        other["spans"][0]["status"] = "interrupted"
        other["spans"][1]["status"] = "interrupted"
        summary.validate(other)
        result = summary.summarize(entries(self.receipt, other), 2)
        self.assertEqual(result["cohorts"]["complete"]["receipt_count"], 1)
        self.assertEqual(result["cohorts"]["error"]["stages"]["collection"]
                         ["interrupted_receipts"], 1)
        self.assertEqual(result["missing_observation_fields"]["actual_decisions"], 1)
        self.assertEqual(result["receipts"][1]["error"], "SYNTHETIC failure")

    def test_null_and_missing_context_differ(self):
        other = copy.deepcopy(self.receipt)
        del other["context"]["terminal_death_tick"]
        result = summary.summarize(entries(self.receipt, other), 2)
        self.assertIn("terminal_death_tick", result["mixed_context_fields"])
        self.assertEqual(result["missing_observation_fields"]["terminal_death_tick"], 1)

    def test_mixed_build_workload_optional_work_warned(self):
        other = copy.deepcopy(self.receipt)
        other["context"].update(source_head_declared="different", actual_decisions=8,
                                rehearsal_epochs=2, seed=8)
        result = summary.summarize(entries(self.receipt, other), 2)
        self.assertEqual(result["mixed_context_fields"],
                         ["actual_decisions", "rehearsal_epochs", "seed", "source_head_declared"])
        self.assertTrue(any("Mixed contexts" in warning for warning in result["warnings"]))

    def test_nested_legacy_counters_are_not_added_or_context_groups(self):
        other = copy.deepcopy(self.receipt)
        self.receipt["context"][summary.NESTED_COUNTERS] = {"production_tick_seconds": 999}
        other["context"][summary.NESTED_COUNTERS] = {"production_tick_seconds": 222}
        result = summary.summarize(entries(self.receipt, other), 2)
        self.assertEqual(result["mixed_context_fields"], [])
        self.assertNotIn("production_tick_seconds", result["cohorts"]["complete"]["stages"])
        self.assertEqual(result["cohorts"]["complete"]["totals"]
                         ["native_total_seconds"]["median"], 10)

    def test_unknown_build_labels_warned(self):
        self.receipt["context"]["source_head_declared"] = None
        del self.receipt["context"]["binary_sha256_declared"]
        result = summary.summarize(entries(self.receipt), 1)
        self.assertEqual(len(result["warnings"]), 2)

    def test_early_error_empty_spans_valid(self):
        self.receipt.update(status="error", error="failed", spans=[],
                            exclusive_covered_seconds=0, unassigned_seconds=10, context={})
        summary.validate(self.receipt)

    def test_bad_schema_including_bool_rejected(self):
        for value in (True, 1.0, 0, 2, None):
            with self.subTest(value=value), self.assertRaises(summary.ReceiptError):
                summary.validate(dict(fixture(), schema_version=value))

    def test_bad_time_basis_rejected(self):
        self.invalid(lambda r: r.update(time_basis="gpu"))

    def test_missing_required_fields_rejected(self):
        for field in ("context", "spans", "native_total_seconds", "exclusive_covered_seconds",
                      "unassigned_seconds", "status", "error"):
            with self.subTest(field=field), self.assertRaises(summary.ReceiptError):
                receipt = fixture()
                del receipt[field]
                summary.validate(receipt)

    def test_bad_duration_types_ranges_rejected(self):
        for value in (True, -1, float("inf"), float("nan"), "1", None, 10 ** 400):
            with self.subTest(value=value), self.assertRaises(summary.ReceiptError):
                summary.validate(dict(fixture(), native_total_seconds=value))

    def test_duplicate_ids_rejected(self):
        self.invalid(lambda r: r["spans"][0].update(id=0))

    def test_bool_id_rejected(self):
        self.invalid(lambda r: r["spans"][0].update(id=True))

    def test_dangling_parent_rejected(self):
        self.invalid(lambda r: r["spans"][0].update(parent_id=999))

    def test_self_cycle_rejected(self):
        self.invalid(lambda r: r["spans"][0].update(parent_id=1))

    def test_multispan_cycle_rejected(self):
        self.invalid(lambda r: r["spans"][1].update(parent_id=1))

    def test_invalid_span_shape_rejected(self):
        for change in ({"call_count": True}, {"call_count": 2}, {"status": "error"},
                       {"name": " "}, {"parent_id": -1}, {"exclusive_seconds": -1},
                       {"elapsed_seconds": True}):
            with self.subTest(change=change), self.assertRaises(summary.ReceiptError):
                receipt = fixture()
                receipt["spans"][0].update(change)
                summary.validate(receipt)

    def test_complete_with_interrupted_span_rejected(self):
        self.invalid(lambda r: r["spans"][0].update(status="interrupted"))

    def test_bad_error_status_consistency_rejected(self):
        for changes in ({"status": "bad"}, {"error": "unexpected"},
                        {"status": "error", "error": None}):
            with self.subTest(changes=changes), self.assertRaises(summary.ReceiptError):
                summary.validate(dict(fixture(), **changes))

    def test_inclusive_exclusive_child_mismatch_rejected(self):
        self.invalid(lambda r: r["spans"][1].update(exclusive_seconds=6))

    def test_covered_mismatch_rejected(self):
        self.invalid(lambda r: r.update(exclusive_covered_seconds=8))

    def test_total_mismatch_rejected(self):
        self.invalid(lambda r: r.update(native_total_seconds=8))

    def test_small_float_roundoff_accepted(self):
        self.receipt["spans"][1]["exclusive_seconds"] += 1e-10
        summary.validate(self.receipt)

    def test_context_exponent_overflow_rejected(self):
        self.invalid(lambda r: r["context"].update(bad=float("inf")))

    def test_nonobject_receipt_rejected(self):
        with self.assertRaises(summary.ReceiptError):
            summary.validate([])

    def test_observation_counts_are_not_assumed_bounds(self):
        self.receipt["context"].update(requested_decisions=16, actual_decisions=17)
        summary.validate(self.receipt)


class InputAndCliTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="SYNTHETIC-stage-summary-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.script = Path(summary.__file__).resolve()

    def write(self, name, receipt=None):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(fixture() if receipt is None else receipt), encoding="utf-8")
        return path

    def cli(self, *args):
        return subprocess.run([sys.executable, str(self.script), *map(str, args)],
                              capture_output=True, text=True, check=False)

    def test_content_copy_reformatted_key_reordered_dedup(self):
        first = self.write("a/stage-timing.json")
        second = self.write("b/stage-timing.json")
        receipt = fixture()
        receipt["native_total_seconds"] = 10.0
        second.write_text(json.dumps(dict(reversed(list(receipt.items()))), indent=4), encoding="utf-8")
        loaded, count = summary.load_receipts([self.root])
        self.assertEqual(count, 2)
        self.assertEqual(len(loaded), 1)
        self.assertEqual(loaded[0]["paths"], sorted([str(first), str(second)]))
        result = summary.summarize(loaded, count)
        self.assertEqual(result["duplicate_copies"], 1)
        self.assertTrue(any("heuristic" in warning for warning in result["warnings"]))

    def test_same_context_different_spans_not_deduped(self):
        self.write("a.json")
        other = fixture()
        other["spans"][1].update(exclusive_seconds=5, elapsed_seconds=7)
        other.update(exclusive_covered_seconds=7, unassigned_seconds=3)
        self.write("b.json", other)
        loaded, count = summary.load_receipts([self.root / "a.json", self.root / "b.json"])
        self.assertEqual((len(loaded), count), (2, 2))

    def test_differing_unknown_metadata_not_deduped(self):
        self.write("a/stage-timing.json")
        other = fixture()
        other["extra"] = "retained in dedup identity"
        self.write("b/stage-timing.json", other)
        self.assertEqual(len(summary.load_receipts([self.root])[0]), 2)

    def test_overlapping_paths_count_once(self):
        path = self.write("a/stage-timing.json")
        loaded, count = summary.load_receipts([self.root, path, self.root / "a"])
        self.assertEqual((len(loaded), count), (1, 1))

    def test_directory_discovers_only_sidecars(self):
        self.write("nested/stage-timing.json")
        (self.root / "cycle.json").write_text("not a timing receipt", encoding="utf-8")
        self.assertEqual(summary.load_receipts([self.root])[1], 1)

    def test_missing_and_empty_directory_fail(self):
        for path in (self.root, self.root / "missing"):
            with self.subTest(path=path), self.assertRaises(summary.ReceiptError):
                summary.load_receipts([path])

    def test_duplicate_keys_rejected(self):
        path = self.write("duplicate.json")
        path.write_text(path.read_text().replace('"schema_version": 1',
                                                '"schema_version": 1, "schema_version": 1'))
        result = self.cli(path)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")
        self.assertIn("duplicate JSON key", result.stderr)

    def test_nonfinite_and_exponent_overflow_json_rejected(self):
        for value in ("NaN", "Infinity", "-Infinity", "1e9999"):
            path = self.write("bad.json")
            path.write_text(path.read_text().replace('"seed": 7', '"seed": ' + value))
            with self.subTest(value=value), self.assertRaises(summary.ReceiptError):
                summary.load_receipts([path])

    def test_invalid_batch_names_all_errors_and_emits_no_partial(self):
        good = self.write("good.json")
        a = self.root / "a.json"
        b = self.root / "b.json"
        a.write_text("{bad", encoding="utf-8")
        b.write_text("[]", encoding="utf-8")
        result = self.cli(good, a, b)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")
        self.assertIn(str(a), result.stderr)
        self.assertIn(str(b), result.stderr)

    def test_json_cli_is_read_only_and_complete(self):
        path = self.write("stage-timing.json")
        before = {p: hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in self.root.rglob("*") if p.is_file()}
        result = self.cli(self.root, "--format", "json")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["unique_receipts"], 1)
        self.assertEqual(report["receipts"][0]["paths"], [str(path)])
        after = {p: hashlib.sha256(p.read_bytes()).hexdigest()
                 for p in self.root.rglob("*") if p.is_file()}
        self.assertEqual(before, after)

    def test_text_cli_labels_unknown_and_host_limits(self):
        path = self.write("stage-timing.json")
        result = self.cli(path)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("collection: median=4, p95=4", result.stdout)
        self.assertIn("unknown, not zero", result.stdout)
        self.assertIn("not GPU kernel timings", result.stdout)
        self.assertIn("profile-off/on", result.stdout)

    def test_cli_requires_inputs_and_valid_format(self):
        self.assertEqual(self.cli().returncode, 2)
        self.assertEqual(self.cli("--format", "csv", self.root).returncode, 2)


if __name__ == "__main__":
    unittest.main()
