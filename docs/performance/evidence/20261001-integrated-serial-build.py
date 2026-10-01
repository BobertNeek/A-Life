"""Build the serial combined-guard comparison from emitted release artifacts."""
import hashlib
import json
from pathlib import Path
import subprocess

root = Path.cwd()
fixture_commit = "61edc70b46af1b8fd1884677271a9b23a3dc9f4d"
probe_commit = "7a8de010811b4c7a2e19fcb78f667e1e3838bd31"
fixture_path = "docs/performance/evidence/20261001-parallel-preparation-build.py"
probe_path = "docs/performance/evidence/20261001-finalization-probe.rs"
fixture_bytes = subprocess.check_output(["git", "show", fixture_commit + ":" + fixture_path])
probe_bytes = subprocess.check_output(["git", "show", probe_commit + ":" + probe_path])
source = fixture_bytes.decode()
probe_source = probe_bytes.decode()
preparation_and_proof = probe_source[
    probe_source.index("fn preparation("):probe_source.index("fn frame_validation(")
]


def checked(text, old, new):
    assert text.count(old) == 1, (old, text.count(old))
    return text.replace(old, new)


# Preserve the complete preparation and proof functions; remove the separate
# threading screen. This executable performs only serial row preparation.
start = probe_source.index("fn frame_validation(")
end = probe_source.index("fn main()", start)
probe_source = probe_source[:start] + probe_source[end:]
start = probe_source.index("                let mut serial = Vec::new();")
end = probe_source.index("                assert_eq!(digest(), input_digest);", start)
probe_source = probe_source[:start] + probe_source[end:]
probe_source = checked(
    probe_source,
    '"public_finalized_validation_samples":validation,\n'
    '            "serial_frame_validation_samples":serial,"two_worker_frame_validation_samples":parallel',
    '"public_finalized_validation_samples":validation',
)
asset = root / "assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
probe_source = checked(
    probe_source,
    '"../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"',
    json.dumps(str(asset)),
)
assert "std::thread" not in probe_source
assert preparation_and_proof in probe_source
probe_source = checked(probe_source, "use std::{hint::black_box, time::Instant};",
                       "use std::time::Instant;")
generated_probe = Path("/tmp/alife-integrated-serial-probe.rs")
generated_probe.write_text(probe_source)
for old, new in [
    ("/tmp/alife-parallel-preparation-reference.rs", "/tmp/alife-integrated-serial-reference.rs"),
    ("/tmp/alife-parallel-preparation-helpers.rs", "/tmp/alife-integrated-serial-helpers.rs"),
    ('root / "docs/performance/evidence/20261001-parallel-preparation-probe.rs"',
     "Path(" + json.dumps(str(generated_probe)) + ")"),
    ('print("Built CPU-only parallel prototype", executable)',
     'print("Built serial integrated preparation probe", executable)'),
    ('command += ["-L", "dependency=" + str(Path(libraries["alife_core"][0]).parent), "-o", str(executable)]',
     'dependency_dir = Path(libraries["alife_core"][0]).parent\n'
     'if dependency_dir.name != "deps":\n'
     '    dependency_dir = dependency_dir / "deps"\n'
     'assert dependency_dir.is_dir()\n'
     'command += ["-L", "dependency=" + str(dependency_dir), "-o", str(executable)]'),
]:
    source = checked(source, old, new)
exec(compile(source, str(Path(__file__).resolve()), "exec"))
manifest_path = executable.with_suffix(".build.json")
manifest = json.loads(manifest_path.read_text())
manifest.update({
    "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"]).decode().strip(),
    "core_memory_sha256": hashlib.sha256((root / "crates/alife_core/src/memory.rs").read_bytes()).hexdigest(),
    "asset_sha256": hashlib.sha256(asset.read_bytes()).hexdigest(),
    "fixture_builder_commit": fixture_commit,
    "fixture_builder_sha256": hashlib.sha256(fixture_bytes).hexdigest(),
    "original_probe_commit": probe_commit,
    "original_probe_sha256": hashlib.sha256(probe_bytes).hexdigest(),
    "serial_only": True,
    "threading_screen_removed": True,
    "preparation_and_output_proof_functions_unchanged": True,
    "dependency_directory": str(dependency_dir),
})
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
