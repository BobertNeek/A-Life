#!/usr/bin/env python3
"""Pair warmed, same-host mating queries with and without the solid guard.

This compares one CPU component in the optimized test profile. It does not
measure full ticks, GPU work, or physical-device game throughput.
"""

import hashlib
import json
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "crates/alife_world/src/headless.rs"
FILTER = "headless::task_4_3a2_tests::mating_reach::mating_query_cpu_cost"
GUARD = b"""        // Mating keeps its own proximity radius, but cannot cross the solids
        // that block ordinary contact. This is not a gaze or walking-path test.
        if !self.solid_segment_clear(maternal_object.position, paternal_object.position) {
            return Ok(None);
        }

"""


def build(destination):
    result = subprocess.run(
        ["cargo", "test", "--locked", "-p", "alife_world", "--lib", "--no-run",
         "--message-format=json"],
        cwd=ROOT, check=False, text=True, stdout=subprocess.PIPE,
    )
    if result.returncode:
        print(result.stdout, flush=True)
        result.check_returncode()
    executables = []
    for line in result.stdout.splitlines():
        row = json.loads(line)
        if (row.get("reason") == "compiler-artifact"
                and row.get("target", {}).get("name") == "alife_world"
                and row.get("executable")):
            executables.append(Path(row["executable"]))
    if len(executables) != 1:
        raise RuntimeError(f"Expected one alife_world test executable, found {executables}")
    shutil.copy2(executables[0], destination)


def run(binary):
    result = subprocess.run(
        [str(binary), FILTER, "--exact", "--ignored", "--nocapture",
         "--test-threads=1", "--color=never"],
        cwd=ROOT, check=False, text=True, stdout=subprocess.PIPE,
    )
    if result.returncode:
        print(result.stdout, flush=True)
        result.check_returncode()
    rows = {}
    for line in result.stdout.splitlines():
        if "MATING_TIMING " not in line:
            continue
        row = json.loads(line.split("MATING_TIMING ", 1)[1])
        key = (row["organisms"], row["clustered"], row["objects"], row["terrain_boxes"])
        if key in rows:
            raise RuntimeError(f"Repeated timing case {key}")
        rows[key] = row
    if len(rows) != 16:
        raise RuntimeError(f"Expected all 16 timing cases, found {len(rows)}")
    return rows


def main():
    original = SOURCE.read_bytes()
    if original.count(GUARD) != 1:
        raise RuntimeError("The exact bounded mating guard must occur once; refusing baseline edit")
    baseline_source = original.replace(GUARD, b"", 1)
    print(json.dumps({
        "scope": "same-host optimized-test-profile CPU query component",
        "platform": platform.platform(),
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "candidate_source_sha256": hashlib.sha256(original).hexdigest(),
        "baseline_source_sha256": hashlib.sha256(baseline_source).hexdigest(),
        "baseline_change": "remove only exact six-line obstruction guard",
        "warmup_runs_per_binary": 1, "alternating_pairs": 5,
    }), flush=True)
    with tempfile.TemporaryDirectory(prefix="mating-query-") as temporary:
        binaries = {arm: Path(temporary) / arm for arm in ("baseline", "candidate")}
        try:
            SOURCE.write_bytes(baseline_source)
            build(binaries["baseline"])
        finally:
            SOURCE.write_bytes(original)
        build(binaries["candidate"])
        if SOURCE.read_bytes() != original:
            raise RuntimeError("Candidate source was not restored exactly")
        for binary in binaries.values():
            run(binary)
        samples = []
        for pair in range(5):
            order = ("baseline", "candidate") if pair % 2 == 0 else ("candidate", "baseline")
            measured = {arm: run(binaries[arm]) for arm in order}
            if measured["baseline"].keys() != measured["candidate"].keys():
                raise RuntimeError("Baseline and candidate cases differ")
            for key in measured["baseline"]:
                before = measured["baseline"][key]
                after = measured["candidate"][key]
                for field in ("organisms", "clustered", "objects", "obstacles",
                              "terrain_boxes", "eligible_pairs"):
                    if before[field] != after[field]:
                        raise RuntimeError(f"Pair workload changed for {field}")
                row = {"pair": pair, "order": order, "baseline": before, "candidate": after}
                samples.append(row)
                print("MATING_PAIR " + json.dumps(row), flush=True)
        for key in measured["baseline"]:
            matching = [row for row in samples if (
                row["baseline"]["organisms"], row["baseline"]["clustered"],
                row["baseline"]["objects"], row["baseline"]["terrain_boxes"]) == key]
            summary = dict(zip(("organisms", "clustered", "objects", "terrain_boxes"), key))
            for component in ("collect_ns", "responses_ns"):
                baseline = [row["baseline"][component] for row in matching]
                candidate = [row["candidate"][component] for row in matching]
                summary[component] = {
                    "baseline_median": statistics.median(baseline),
                    "candidate_median": statistics.median(candidate),
                    "paired_delta_median": statistics.median(b - a for a, b in zip(baseline, candidate)),
                    "paired_ratio_median": statistics.median(b / a for a, b in zip(baseline, candidate)),
                }
            print("MATING_SUMMARY " + json.dumps(summary), flush=True)
    if SOURCE.read_bytes() != original:
        raise RuntimeError("Source changed during measurement")


if __name__ == "__main__":
    main()
