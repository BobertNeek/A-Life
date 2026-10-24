#!/usr/bin/env python3
"""Disposable synthetic storage benchmark; no native training/checkpoint evidence.

Run from any directory. Fixture construction and warming are excluded from timing.
Each operation gets untraced wall/CPU samples, a separate traced invocation, and
one further traversal for counts. Traced live blocks are retained allocations,
not a count of all allocations performed. Reads use the warmed filesystem cache.
"""
import argparse
from contextlib import contextmanager
import gc
import hashlib
import json
from pathlib import Path
import platform
import statistics
import sys
import tempfile
import time
import tracemalloc

import grab_food_seed_reservations as seeds
import mixed_care_ledger as store


def whole_prefix_hash(runs, count=None):
    """Pre-change hash, including its complete prefix slice/text/bytes."""
    prefix = runs if count is None else runs[:count]
    return hashlib.sha256(json.dumps(prefix, sort_keys=True, allow_nan=False).encode()).hexdigest()


@contextmanager
def use_hash(function):
    previous = store.canonical_hash
    store.canonical_hash = function
    try:
        yield
    finally:
        store.canonical_hash = previous


def allocation(index, attempts, payload_bytes):
    output = f"/synthetic/run-{index}"
    return {"output": output, "plan_sha256": "0" * 64, "attempts": [
        {"seed": index * attempts + slot + 1, "slot": slot, "status": "accepted",
         "directory": f"{output}/attempt-{slot}",
         "receipt": {"policy_version": index * attempts + slot,
                     "actor_optimizer_step": 2**60 + slot,
                     "value_optimizer_step": 2**60 + 2 * slot,
                     "note": "synthetic 雪\n" + "x" * payload_bytes,
                     "trained_replay_rows": 48, "captured_replay_rows": 49},
         "sha256": {name: "0" * 64 for name in seeds.SOURCE_FILES}}
        for slot in range(attempts)]}


def measure(action, repeats):
    result = action(seeds._read)  # Warm once, outside all measurements.
    del result
    wall, cpu = [], []
    for _ in range(repeats):
        gc.collect()
        started, cpu_started = time.perf_counter(), time.process_time()
        result = action(seeds._read)
        cpu.append(time.process_time() - cpu_started)
        wall.append(time.perf_counter() - started)
        del result
    gc.collect()
    tracemalloc.start()
    try:
        result = action(seeds._read)  # Keep the returned graph alive for the snapshot.
        current, peak = tracemalloc.get_traced_memory()
        live_blocks = sum(stat.count for stat in tracemalloc.take_snapshot().statistics("filename"))
    finally:
        tracemalloc.stop()
    del result
    return {"wall_median_seconds": statistics.median(wall),
            "cpu_median_seconds": statistics.median(cpu),
            "traced_retained_bytes": current, "traced_peak_bytes": peak,
            "traced_live_blocks": live_blocks}


def counted(action, hash_function, directory):
    counts = {"read_documents": 0, "segment_reads": 0,
              "hash_calls": 0, "runs_hashed": 0}

    def reader(path):
        counts["read_documents"] += 1
        if Path(path).parent == directory and not Path(path).name.startswith("legacy-"):
            counts["segment_reads"] += 1
        return seeds._read(path)

    def hashed(runs, count=None):
        counts["hash_calls"] += 1
        counts["runs_hashed"] += len(runs) if count is None else count
        return hash_function(runs, count)

    with use_hash(hashed):
        result = action(reader)
        del result
    return counts


def benchmark(size, args, production_hash):
    with tempfile.TemporaryDirectory(prefix="alife-ledger-benchmark-") as temporary:
        path = Path(temporary).resolve() / "ledger.json"
        runs = [allocation(index, args.attempts, args.payload_bytes) for index in range(size)]
        state = store.State({"schema": 1, "kind": store.LEDGER, "excluded_seed_ranges": [],
                             "assessment_seeds": [], "runs": runs})
        store.write(path, state)  # One fresh write creates size - 1 immutable segments.
        directory = store.folder(path)
        fixture = {"allocations": size, "archived_allocations": size - 1,
                   "attempts_per_allocation": args.attempts, "payload_bytes": args.payload_bytes,
                   "logical_json_bytes": len(json.dumps(runs, sort_keys=True, allow_nan=False).encode()),
                   "manifest_bytes": path.stat().st_size,
                   "segment_bytes": sum(item.stat().st_size for item in directory.iterdir())}
        expected = whole_prefix_hash(runs, size - 1)
        if production_hash(runs, size - 1) != expected:
            raise ValueError("production prefix hash differs from the pre-change digest")
        rows = []
        for variant, function in (("whole_prefix_baseline", whole_prefix_hash),
                                  ("production", production_hash)):
            with use_hash(function):
                admitted = store.read(path)  # Preparation for unchanged writes is excluded.
                if admitted["runs"] != runs:
                    raise ValueError("synthetic storage round trip changed the logical history")
                actions = (
                    ("prefix_hash", lambda reader: store.canonical_hash(runs, size - 1)),
                    ("full_read", lambda reader: store.read(path, read_document=reader)),
                    ("unchanged_write", lambda reader: store.write(path, admitted)),
                )
                for operation, action in actions:
                    row = {**fixture, "variant": variant, "operation": operation,
                           **measure(action, args.repeats)}
                    row["instrumented_traversal"] = counted(action, function, directory)
                    rows.append(row)
        return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sizes", type=int, nargs="+", default=[10, 100, 1000])
    parser.add_argument("--attempts", type=int, default=16)
    parser.add_argument("--payload-bytes", type=int, default=512)
    parser.add_argument("--repeats", type=int, default=5)
    args = parser.parse_args()
    if min(args.sizes) < 1 or args.attempts < 1 or args.payload_bytes < 0 or args.repeats < 1:
        parser.error("sizes, attempts and repeats must be positive; payload bytes must be nonnegative")
    production_hash = store.canonical_hash
    rows = [row for size in args.sizes for row in benchmark(size, args, production_hash)]
    print(json.dumps({"schema": 1, "python": sys.version, "platform": platform.platform(),
                      "scope": "synthetic storage-layer fixtures; no native trainer or checkpoint payloads",
                      "timing": "warmed reads; fixture creation excluded; untraced medians",
                      "memory": "separate incremental traced call; result retained; live blocks are not allocation churn or RSS",
                      "repeats": args.repeats, "rows": rows}, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
