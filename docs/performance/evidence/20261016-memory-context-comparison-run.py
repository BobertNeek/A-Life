#!/usr/bin/env python3
"""Compile the allocation probe with the exact current production helper."""
import argparse
import pathlib
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--rustc", default="rustc")
args = parser.parse_args()
root = pathlib.Path(__file__).resolve().parents[3]
source = (root / "crates/alife_gpu_backend/src/closed_loop_memory.rs").read_text()
start = source.index("fn memory_records_match_context(")
end = source.index("\n}\n", start) + 2
helper = source[start:end]
bench = pathlib.Path(__file__).with_name("20261016-memory-context-comparison-bench.rs").read_text()
assert bench.count("// PRODUCTION_HELPER") == 1
with tempfile.TemporaryDirectory(prefix="alife-memory-comparison-") as temp:
    path = pathlib.Path(temp)
    (path / "probe.rs").write_text(bench.replace("// PRODUCTION_HELPER", helper))
    subprocess.run([args.rustc, "--edition=2021", "-O", str(path / "probe.rs"), "-o", str(path / "probe")], check=True)
    subprocess.run([str(path / "probe")], check=True)
