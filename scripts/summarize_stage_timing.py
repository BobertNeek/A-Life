#!/usr/bin/env python3
"""Read-only, stdlib-only summary of schema-1 serial-cycle timing sidecars."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics
import sys


TIME_BASIS = "monotonic_host_wall_clock_around_existing_calls_not_gpu_kernel_time"
KNOWN_STAGES = (
    "source_admission", "world_lesson_setup", "backend_runtime_setup",
    "initial_durable_boundary", "collection", "first_priming_tick_capture",
    "trainer_construction_restore", "initial_actor_checkpoint_capture",
    "remaining_collection", "old_value_and_rollout_target_preparation", "update",
    "ppo", "acquisition_rehearsal", "archived_success_replay",
    "export_write_redecode", "next_cohort_construction_decision", "trainer_rebind",
    "final_actor_capture_write", "final_value_capture_write", "final_receipt_write",
)
BUILD_LABELS = ("source_head_declared", "binary_sha256_declared")
OBSERVATIONS = (
    "actual_decisions", "captured_replay_rows", "terminal_death_tick",
    "replay_segment_count", "uncaptured_world_ticks_between_rows",
)
# These nested diagnostic counters are timings, not configuration or extra stages.
NESTED_COUNTERS = "legacy_collection_counters_nested_do_not_sum_with_collection"
LIMITATIONS = (
    "Host wall-clock spans include waits; these are not GPU kernel timings.",
    "Declared source/binary labels are not verified identities. The recorder does "
    "not establish adapter, prior, cold/warm, tree/dirty state, host-wrapper total "
    "or profile-off/on observer overhead; equal contexts do not prove comparability.",
    "Canonical full-JSON content deduplication is a copy heuristic: schema 1 has "
    "no unique run ID, so independent runs with identical receipt content collapse. "
    "Retain the listed aliases and original run provenance.",
    "Missing stages/observations are unknown, never invented zeros. Replay gaps "
    "are capture-gap observations, not identified sleep time.",
)


class ReceiptError(ValueError):
    """An input is not a supported, internally consistent timing receipt."""


def _pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ReceiptError(f"duplicate JSON key: {key!r}")
        result[key] = value
    return result


def _bad_constant(value):
    raise ReceiptError(f"nonfinite JSON number: {value}")


def _canonical(value):
    """Normalize JSON formatting, key order and equivalent 1/1.0 or -0/0."""
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ReceiptError("nonfinite number, including exponent overflow")
        return int(value) if value.is_integer() else value
    if isinstance(value, dict):
        return {key: _canonical(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_canonical(item) for item in value]
    return value


def canonical_json(value):
    return json.dumps(_canonical(value), sort_keys=True, separators=(",", ":"),
                      ensure_ascii=True, allow_nan=False)


def _integer(value, label):
    if type(value) is not int or value < 0:
        raise ReceiptError(f"{label} must be a nonnegative integer")


def _seconds(value, label):
    if type(value) not in (int, float):
        raise ReceiptError(f"{label} must be a finite nonnegative number")
    try:
        valid = math.isfinite(value) and value >= 0
    except OverflowError:
        valid = False
    if not valid:
        raise ReceiptError(f"{label} must be a finite nonnegative number")


def _close(left, right, label):
    # Rust's separately converted f64 Durations can differ by rounding.
    if not math.isclose(left, right, rel_tol=1e-7, abs_tol=1e-9):
        raise ReceiptError(f"inconsistent {label}: {left!r} versus {right!r}")


def validate(receipt):
    if not isinstance(receipt, dict):
        raise ReceiptError("receipt must be a JSON object")
    if type(receipt.get("schema_version")) is not int or receipt["schema_version"] != 1:
        raise ReceiptError("schema_version must be integer 1")
    if receipt.get("time_basis") != TIME_BASIS:
        raise ReceiptError("unsupported time_basis")
    status = receipt.get("status")
    if status not in ("complete", "error"):
        raise ReceiptError("status must be complete or error")
    if "error" not in receipt or (status == "complete" and receipt["error"] is not None):
        raise ReceiptError("complete receipts require error: null")
    if status == "error" and not isinstance(receipt["error"], str):
        raise ReceiptError("error receipts require an error string")
    if not isinstance(receipt.get("context"), dict):
        raise ReceiptError("context must be an object")
    spans = receipt.get("spans")
    if not isinstance(spans, list):
        raise ReceiptError("spans must be an array")
    for field in ("native_total_seconds", "exclusive_covered_seconds", "unassigned_seconds"):
        _seconds(receipt.get(field), field)
    by_id = {}
    for span in spans:
        if not isinstance(span, dict):
            raise ReceiptError("each span must be an object")
        _integer(span.get("id"), "span id")
        if span["id"] in by_id:
            raise ReceiptError(f"duplicate span id: {span['id']}")
        by_id[span["id"]] = span
        if not isinstance(span.get("name"), str) or not span["name"].strip():
            raise ReceiptError("span name must be a nonempty string")
        if "parent_id" not in span:
            raise ReceiptError("span must contain parent_id")
        if span["parent_id"] is not None:
            _integer(span["parent_id"], "parent_id")
        if type(span.get("call_count")) is not int or span["call_count"] != 1:
            raise ReceiptError("schema-1 span call_count must be integer 1")
        if span.get("status") not in ("complete", "interrupted"):
            raise ReceiptError("span status must be complete or interrupted")
        if status == "complete" and span["status"] != "complete":
            raise ReceiptError("complete receipt contains an interrupted span")
        _seconds(span.get("elapsed_seconds"), "elapsed_seconds")
        _seconds(span.get("exclusive_seconds"), "exclusive_seconds")
    children = {key: [] for key in by_id}
    for span in spans:
        parent = span["parent_id"]
        if parent is not None:
            if parent not in by_id:
                raise ReceiptError(f"dangling parent_id: {parent}")
            children[parent].append(span)
    # Input order is close/post-order. Walk IDs without recursive stack limits.
    finished = set()
    for start in by_id:
        active = set()
        current = start
        while current is not None and current not in finished:
            if current in active:
                raise ReceiptError("cyclic span parents")
            active.add(current)
            current = by_id[current]["parent_id"]
        finished.update(active)
    try:
        for key, span in by_id.items():
            _close(span["elapsed_seconds"], math.fsum(
                [span["exclusive_seconds"]] +
                [child["elapsed_seconds"] for child in children[key]]),
                f"span {key} exclusive/direct-child accounting")
        _close(receipt["exclusive_covered_seconds"],
               math.fsum(span["exclusive_seconds"] for span in spans),
               "exclusive covered time")
        _close(receipt["native_total_seconds"], math.fsum(
            [receipt["exclusive_covered_seconds"], receipt["unassigned_seconds"]]),
            "native total time")
    except OverflowError as error:
        raise ReceiptError("duration sum overflow") from error
    canonical_json(receipt)  # Also reject exponent overflow anywhere in context.
    return receipt


def discover(inputs):
    paths = set()
    for raw in inputs:
        path = Path(raw)
        if path.is_dir():
            paths.update(item.resolve() for item in path.rglob("stage-timing.json")
                         if item.is_file())
        elif path.is_file():
            paths.add(path.resolve())
        else:
            raise ReceiptError(f"input does not exist or is not a file/directory: {path}")
    if not paths:
        raise ReceiptError("no stage-timing.json receipts found")
    return sorted(paths)


def load_receipts(inputs):
    paths = discover(inputs)
    unique = {}
    errors = []
    for path in paths:
        try:
            receipt = validate(json.loads(path.read_text(encoding="utf-8"),
                                         object_pairs_hook=_pairs,
                                         parse_constant=_bad_constant))
            identity = hashlib.sha256(canonical_json(receipt).encode("ascii")).hexdigest()
            if identity not in unique:
                unique[identity] = {"content_sha256": identity, "paths": [],
                                    "receipt": receipt}
            unique[identity]["paths"].append(str(path))
        except (OSError, ValueError, RecursionError) as error:
            errors.append(f"{path}: {error}")
    if errors:
        raise ReceiptError("invalid input; no partial summary emitted:\n" + "\n".join(errors))
    return list(unique.values()), len(paths)


def stats(values):
    values = sorted(values)
    if not values:
        return {"count": 0, "min": None, "median": None, "p95": None, "max": None}
    median = statistics.median(values)
    if not math.isfinite(median):
        # Avoid the even-sample (a+b)/2 overflow for otherwise finite inputs.
        middle = len(values) // 2
        median = values[middle - 1] / 2 + values[middle] / 2
    return {"count": len(values), "min": values[0], "median": median,
            "p95": values[math.ceil(0.95 * len(values)) - 1], "max": values[-1]}


def context_differences(receipts):
    keys = set().union(*(receipt["context"] for receipt in receipts)) - {NESTED_COUNTERS}
    return sorted(key for key in keys if len({
        canonical_json([key in receipt["context"], receipt["context"].get(key)])
        for receipt in receipts}) > 1)


def cohort_summary(receipts):
    names = set(KNOWN_STAGES).union(span["name"] for receipt in receipts
                                         for span in receipt["spans"])
    stages = {}
    for name in sorted(names):
        values = []
        calls = 0
        interrupted = 0
        for receipt in receipts:
            spans = [span for span in receipt["spans"] if span["name"] == name]
            if spans:
                values.append(math.fsum(span["exclusive_seconds"] for span in spans))
                calls += len(spans)
                interrupted += any(span["status"] == "interrupted" for span in spans)
        stages[name] = {"observed_receipts": len(values),
                        "missing_receipts": len(receipts) - len(values),
                        "zero_receipts": values.count(0),
                        "interrupted_receipts": interrupted, "calls": calls,
                        "exclusive_seconds": stats(values)}
    return {"receipt_count": len(receipts),
            "mixed_context_fields": context_differences(receipts),
            "totals": {key: stats([receipt[key] for receipt in receipts]) for key in (
                "native_total_seconds", "exclusive_covered_seconds", "unassigned_seconds")},
            "stages": stages}


def summarize(entries, input_count):
    receipts = [entry["receipt"] for entry in entries]
    differences = context_differences(receipts)
    warnings = []
    if differences:
        warnings.append("Mixed contexts: " + ", ".join(differences) +
                        ". Pooled medians/p95 are descriptive only; select comparable inputs.")
    if input_count > len(entries):
        warnings.append("Content-identical copied/reformatted receipts were counted once; "
                        "this is heuristic deduplication, not verified run identity.")
    for key in BUILD_LABELS:
        count = sum(not isinstance(receipt["context"].get(key), str) or
                    not receipt["context"][key].strip() for receipt in receipts)
        if count:
            warnings.append(f"{key} missing/null/empty in {count} receipt(s); build identity unknown.")
    if any(receipt["status"] == "error" for receipt in receipts):
        warnings.append("Error receipts are partial and reported in a separate cohort; "
                        "their stage observations are not completed-cycle timings.")
    missing = {key: sum(key not in receipt["context"] for receipt in receipts)
               for key in OBSERVATIONS}
    return {"summary_schema_version": 1, "time_basis": TIME_BASIS,
            "percentile_method": "nearest-rank: sorted[ceil(0.95*n)-1]",
            "stage_sample": "sum exclusive seconds for each stage name per receipt",
            "input_files": input_count, "unique_receipts": len(entries),
            "duplicate_copies": input_count - len(entries),
            "mixed_context_fields": differences, "missing_observation_fields": missing,
            "warnings": warnings, "limitations": list(LIMITATIONS),
            "receipts": [{"content_sha256": entry["content_sha256"], "paths": entry["paths"],
                          "status": entry["receipt"]["status"],
                          "error": entry["receipt"]["error"],
                          "context": entry["receipt"]["context"]} for entry in entries],
            "cohorts": {status: cohort_summary([receipt for receipt in receipts
                                               if receipt["status"] == status])
                        for status in ("complete", "error")
                        if any(receipt["status"] == status for receipt in receipts)}}


def render_text(report):
    lines = [f"{report['unique_receipts']} unique receipt(s), "
             f"{report['duplicate_copies']} duplicate copy/copies",
             "Exclusive host-wall seconds; p95 uses nearest rank; samples are per receipt."]
    lines.extend("WARNING: " + warning for warning in report["warnings"])
    missing = [f"{key}={count}" for key, count in report["missing_observation_fields"].items()
               if count]
    if missing:
        lines.append("Missing observation fields (unknown): " + ", ".join(missing))
    for status, cohort in report["cohorts"].items():
        lines.append(f"\n{status} cohort: {cohort['receipt_count']} receipt(s)")
        for name, values in cohort["totals"].items():
            lines.append(f"{name}: median={values['median']:.9g}, p95={values['p95']:.9g}")
        observed = [(name, stage) for name, stage in cohort["stages"].items()
                    if stage["observed_receipts"]]
        for name, stage in sorted(observed, key=lambda pair: (
                -pair[1]["exclusive_seconds"]["median"], pair[0])):
            values = stage["exclusive_seconds"]
            lines.append(f"{name}: median={values['median']:.9g}, p95={values['p95']:.9g}; "
                         f"observed={stage['observed_receipts']}/{cohort['receipt_count']}, "
                         f"missing={stage['missing_receipts']}, zero={stage['zero_receipts']}, "
                         f"interrupted={stage['interrupted_receipts']}, calls={stage['calls']}")
        absent = [name for name, stage in cohort["stages"].items()
                  if not stage["observed_receipts"]]
        if absent:
            lines.append("Not observed (unknown, not zero): " + ", ".join(absent))
    lines.append("\nLimitations:")
    lines.extend("- " + limitation for limitation in report["limitations"])
    lines.append("Use --format json for original contexts, error strings and duplicate source aliases.")
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", help="receipt files or directories recursively searched for stage-timing.json")
    parser.add_argument("--format", choices=("text", "json"), default="text")
    args = parser.parse_args(argv)
    try:
        entries, count = load_receipts(args.inputs)
        report = summarize(entries, count)
        output = (json.dumps(report, indent=2, sort_keys=True, allow_nan=False)
                  if args.format == "json" else render_text(report))
    except (OSError, ValueError, OverflowError, RecursionError) as error:
        parser.exit(2, f"{parser.prog}: {error}\n")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
