#!/usr/bin/env python3
"""Measure the complete disposable capture/export/component-assay process.

Requires an already-built cpu_collector test executable. Creates fresh local
fixtures only; never accepts a private checkpoint or changes production defaults.
This is not a GPU/full-neural backend benchmark. Its exact boundary is recorded.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bridge_binary", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--batch", type=int, choices=range(1, 65), default=16)
    parser.add_argument("--device", choices=["cpu", "cuda"], default="cpu")
    args = parser.parse_args()
    binary = args.bridge_binary.resolve(strict=True)
    # Refuse existing output; all failed prefixes and acquired fixtures remain.
    args.output.mkdir(parents=True, exist_ok=False)
    output = args.output.resolve()
    started = time.perf_counter()
    env = os.environ.copy()
    env["ALIFE_PACKED_OUTPUT_DIR"] = str(output / "fixture")
    env["ALIFE_PACKED_BRIDGE_BATCH"] = str(args.batch)
    env["RUST_TEST_THREADS"] = "1"
    env["OPENBLAS_NUM_THREADS"] = "1"
    env["OMP_NUM_THREADS"] = "1"
    bridge = [str(binary), "--ignored", "--exact",
              "packed_block_bridge::export_fresh_native_packed_block_fixture", "--nocapture"]
    report = {"schema_version": 1, "device_requested": args.device,
              "boundary": "fresh native capture, native admission/copied update, fixture IO, Python startup/import/IO, packing, component repeats and local VJP; excludes build; not full packed GPU neural training",
              "batch": args.batch, "commands": []}
    at = time.perf_counter()
    with (output / "bridge.log").open("w") as log:
        result = subprocess.run(bridge, env=env, stdout=log, stderr=subprocess.STDOUT)
    report.update(bridge_exit_code=result.returncode,
                  bridge_process_seconds=time.perf_counter() - at)
    report["commands"].append(bridge)
    exit_code = result.returncode
    if not exit_code:
        fixture = output / "fixture/fresh-native-block-fixture.json"
        if not fixture.is_file():
            raise RuntimeError("bridge did not emit the required fresh fixture")
        sidecar = [sys.executable, str(Path(__file__).with_name("experiment_packed_tiles.py")),
                   str(fixture), "--device", args.device, "--repeats", "5", "--warmup", "2"]
        at = time.perf_counter()
        with (output / "component.json").open("w") as stdout, (output / "component.stderr").open("w") as stderr:
            result = subprocess.run(sidecar, env=env, stdout=stdout, stderr=stderr)
        report.update(component_exit_code=result.returncode,
                      component_process_seconds=time.perf_counter() - at)
        report["commands"].append(sidecar)
        exit_code = result.returncode
    report.update(total_process_seconds=time.perf_counter() - started,
                  complete=exit_code == 0, tensor_core_execution_proven=False)
    (output / "process-ledger.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
