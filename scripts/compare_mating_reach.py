#!/usr/bin/env python3
"""Compare warmed baseline, linear guard and local-candidate mating work.

This measures query components and idle-bundle/world-tick CPU cycles in the
optimized test profile, not neural policy, GPU or physical-device gameplay.

Opt-in Linux diagnostic, requiring Rust and Python 3. From an isolated checkout:
    python3 scripts/compare_mating_reach.py

It temporarily rewrites headless.rs for historical arms. Normal build failures
restore the original bytes, but a hard kill cannot. Do not run in an active
working tree or alongside edits. Build settings below reproduce the measured
profile without relying on CI environment variables. This is not an automatic
CI performance gate.

Qualified receipt: commit 2d60f619394ef6292ad4fbc92307b62f189fb280,
https://github.com/BobertNeek/A-Life/actions/runs/42315157339/job/128225840243
The earlier scoped-candidate receipt is at commit 782be6c1a591adad38e9ba70c4c671c9e81b827f,
https://github.com/BobertNeek/A-Life/actions/runs/42314025099/job/128222153023
"""

import hashlib
import json
import os
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
        if !reachable(maternal_object.position, paternal_object.position) {
            return Ok(None);
        }

"""
LINEAR_GUARD = GUARD.replace(b"!reachable(", b"!self.solid_segment_clear(")
UNFILTERED = b"""    if obstacles.any(|object| Some(object.id) != ignored && object.blocks_segment(start, end)) {
"""
FILTERED = b"""    if obstacles.any(|object| {
        Some(object.id) != ignored
            && object.kind == WorldObjectKind::Obstacle
            && !object.consumed
            && object.blocks_segment(start, end)
    }) {
"""
BUILD_SETTINGS = {
    "CARGO_BUILD_JOBS": "2",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
}


def build(destination):
    result = subprocess.run(
        ["cargo", "test", "--locked", "-p", "alife_world", "--lib", "--no-run",
         "--message-format=json"],
        cwd=ROOT, check=False, text=True, stdout=subprocess.PIPE,
        env={**os.environ, **BUILD_SETTINGS},
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
        key = (row["scene"], row["organisms"], row["clustered"], row["objects"], row["terrain_boxes"])
        if key in rows:
            raise RuntimeError(f"Repeated timing case {key}")
        rows[key] = row
    if len(rows) != 22:
        raise RuntimeError(f"Expected all 22 timing cases, found {len(rows)}")
    return rows


def main():
    original = SOURCE.read_bytes()
    if original.count(GUARD) != 1 or original.count(FILTERED) != 1:
        raise RuntimeError("The exact guard and filter must occur once; refusing baseline edit")
    unfiltered_source = original.replace(FILTERED, UNFILTERED, 1).replace(GUARD, LINEAR_GUARD, 1)
    baseline_source = original.replace(FILTERED, UNFILTERED, 1).replace(GUARD, b"", 1)
    print(json.dumps({
        "scope": "same-host optimized-test-profile CPU queries and idle-bundle/world-tick cycle",
        "platform": platform.platform(),
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "build_environment": BUILD_SETTINGS,
        "candidate_source_sha256": hashlib.sha256(original).hexdigest(),
        "candidate_module_sha256": hashlib.sha256(
            (ROOT / "crates/alife_world/src/headless/mating_reach.rs").read_bytes()).hexdigest(),
        "fixture_sha256": hashlib.sha256(
            (ROOT / "crates/alife_world/src/headless/task_4_3a2_tests/mating_reach.rs").read_bytes()).hexdigest(),
        "baseline_source_sha256": hashlib.sha256(baseline_source).hexdigest(),
        "unfiltered_source_sha256": hashlib.sha256(unfiltered_source).hexdigest(),
        "baseline_change": "bypass reach entirely; remove early type/consumed filter",
        "unfiltered_change": "bypass local candidates with original full scan; remove early type/consumed filter",
        "warmup_runs_per_binary": 1, "rotating_matched_triples": 6,
        "cycle_measurement": "mean of three fresh-state single idle-bundle/tick cycles; production rollback clones included",
    }), flush=True)
    with tempfile.TemporaryDirectory(prefix="mating-query-") as temporary:
        arms = ("baseline", "unfiltered", "candidate")
        binaries = {arm: Path(temporary) / arm for arm in arms}
        try:
            SOURCE.write_bytes(baseline_source)
            build(binaries["baseline"])
            SOURCE.write_bytes(unfiltered_source)
            build(binaries["unfiltered"])
        finally:
            SOURCE.write_bytes(original)
        build(binaries["candidate"])
        if SOURCE.read_bytes() != original:
            raise RuntimeError("Candidate source was not restored exactly")
        for binary in binaries.values():
            run(binary)
        samples = []
        for pair in range(6):
            orders = (
                ("baseline", "unfiltered", "candidate"),
                ("unfiltered", "candidate", "baseline"),
                ("candidate", "baseline", "unfiltered"),
                ("candidate", "unfiltered", "baseline"),
                ("unfiltered", "baseline", "candidate"),
                ("baseline", "candidate", "unfiltered"),
            )
            order = orders[pair]
            measured = {arm: run(binaries[arm]) for arm in order}
            if any(measured[arm].keys() != measured["baseline"].keys() for arm in arms):
                raise RuntimeError("Baseline and candidate cases differ")
            for key in measured["baseline"]:
                before = measured["baseline"][key]
                for arm in arms:
                    after = measured[arm][key]
                    for field in ("scene", "organisms", "clustered", "objects", "obstacles",
                                  "terrain_boxes", "eligible_pairs", "cycle_signature"):
                        if before[field] != after[field]:
                            raise RuntimeError(f"Pair workload changed for {field}")
                row = {"pair": pair, "order": order, **{arm: measured[arm][key] for arm in arms}}
                samples.append(row)
                print("MATING_PAIR " + json.dumps(row), flush=True)
        for key in measured["baseline"]:
            matching = [row for row in samples if (
                row["baseline"]["scene"], row["baseline"]["organisms"], row["baseline"]["clustered"],
                row["baseline"]["objects"], row["baseline"]["terrain_boxes"]) == key]
            summary = dict(zip(("scene", "organisms", "clustered", "objects", "terrain_boxes"), key))
            for component in ("collect_ns", "responses_ns", "bundle_tick_ns"):
                baseline = [row["baseline"][component] for row in matching]
                unfiltered = [row["unfiltered"][component] for row in matching]
                candidate = [row["candidate"][component] for row in matching]
                summary[component] = {
                    "baseline_median": statistics.median(baseline),
                    "unfiltered_median": statistics.median(unfiltered),
                    "candidate_median": statistics.median(candidate),
                    "paired_delta_median": statistics.median(b - a for a, b in zip(baseline, candidate)),
                    "paired_ratio_median": statistics.median(b / a for a, b in zip(baseline, candidate)),
                    "candidate_vs_unfiltered_delta_median": statistics.median(b - a for a, b in zip(unfiltered, candidate)),
                    "candidate_vs_unfiltered_ratio_median": statistics.median(b / a for a, b in zip(unfiltered, candidate)),
                }
            print("MATING_SUMMARY " + json.dumps(summary), flush=True)
    if SOURCE.read_bytes() != original:
        raise RuntimeError("Source changed during measurement")


if __name__ == "__main__":
    main()
