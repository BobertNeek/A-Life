#!/usr/bin/env python3
"""Build two isolated core libraries and run the same allocation probe on both.

Usage: python3 this_file.py BASELINE_ROOT CANDIDATE_ROOT DEPS OUTPUT
Run after the candidate's scoped tests, reusing only their third-party dependency
rlibs. Both core libraries are compiled directly from their stated source roots. All output is local; this script does not access a GPU/network.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

baseline, candidate, deps, output = map(Path, sys.argv[1:])
output.mkdir(parents=True, exist_ok=True)
rustc = shutil.which("rustc")
assert rustc, "rustc must be on PATH"
source = Path(__file__).with_name("20261016-retained-recall-context-bench.rs")
commands = []

def run(command, env=None):
    commands.append(command)
    result = subprocess.run(command, env=env, capture_output=True, text=True)
    (output / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
    if result.returncode:
        print(result.stdout, file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        result.check_returncode()
    return result.stdout

externs = []
for name in ["bitflags", "blake3", "bytemuck", "serde", "smallvec", "thiserror"]:
    matches = list(deps.glob(f"lib{name}-*.rlib"))
    assert len(matches) == 1, (name, matches)
    externs += ["--extern", f"{name}={matches[0]}"]
native = list(deps.parent.glob("build/blake3-*/out"))
assert native, "compiled blake3 native library directory is required"
flags = ["--edition=2021", "-C", "opt-level=2", "-C", "codegen-units=256",
         "-C", "debug-assertions=off", "-C", "strip=debuginfo", "-L", f"dependency={deps}",
         *[item for path in sorted(native) for item in ["-L", f"native={path}"]]]
libraries = {}
for label, root in [("baseline", baseline), ("retained", candidate)]:
    lib_out = output / f"{label}-rlib"
    lib_out.mkdir(exist_ok=True)
    env = dict(os.environ, CARGO_MANIFEST_DIR=str(root / "crates/alife_core"))
    run([rustc, "--crate-name", "alife_core", "--crate-type", "lib", "--emit=dep-info,link",
         "-C", f"metadata=recall_encoding_{label}", "-C", f"extra-filename=-recall_encoding_{label}",
         *flags, *externs, str(root / "crates/alife_core/src/lib.rs"), "--out-dir", str(lib_out)], env)
    libraries[label] = lib_out / f"libalife_core-recall_encoding_{label}.rlib"
records = {}
for label, library in libraries.items():
    binary = output / f"recall-context-{label}"
    run([rustc, *flags, "--extern", f"alife_core={library}", str(source), "-o", str(binary)])
    raw = run([str(binary), label])
    (output / f"{label}.jsonl").write_text(raw)
    records[label] = [json.loads(line) for line in raw.splitlines()]
assert len(records["baseline"]) == len(records["retained"]) == 12
for old, new in zip(records["baseline"], records["retained"]):
    assert (old["mode"], old["candidates"]) == (new["mode"], new["candidates"])
    if "frame_digest" in old:
        assert old["frame_digest"] == new["frame_digest"]
        assert old["candidate_key_digests"] == new["candidate_key_digests"]
receipt = {
    "rustc": run([rustc, "--version"]).strip(),
    "profile": "core opt-level=2, debug assertions off, codegen-units=256; shared test-built third-party dependencies",
    "commands": commands,
    "measure": "one recall -> cognitive attachment -> finalization, 16 repetitions with identical counts; prepared clone measured separately",
    "excluded": "input/cache setup, input cloning, result validation, digest proof collection, destruction",
    "source_sha256": {},
    "records": records,
}
for label, root in [("baseline", baseline), ("retained", candidate)]:
    receipt["source_sha256"][label] = {
        str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted((root / "crates/alife_core/src").rglob("*.rs"))
    }
receipt["probe_sha256"] = hashlib.sha256(source.read_bytes()).hexdigest()
receipt["runner_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
receipt["library_sha256"] = {label: hashlib.sha256(path.read_bytes()).hexdigest()
                              for label, path in libraries.items()}
(output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
for old, new in zip(records["baseline"], records["retained"]):
    print(old["mode"], old["candidates"], old["alloc_realloc_calls"], "->", new["alloc_realloc_calls"],
          "calls;", old["requested_bytes"], "->", new["requested_bytes"], "requested bytes")
