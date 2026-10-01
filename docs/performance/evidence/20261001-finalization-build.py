"""Build this dated CPU experiment using the already published fixture builder."""
import hashlib
import json
from pathlib import Path
import subprocess

_finalization_reused_commit = "61edc70b46af1b8fd1884677271a9b23a3dc9f4d"
_finalization_reused_path = "docs/performance/evidence/20261001-parallel-preparation-build.py"
_finalization_builder_bytes = subprocess.check_output(
    ["git", "show", _finalization_reused_commit + ":" + _finalization_reused_path])
_finalization_builder_sha256 = hashlib.sha256(_finalization_builder_bytes).hexdigest()
source = _finalization_builder_bytes.decode()
for old, new in [
    ("/tmp/alife-parallel-preparation-reference.rs", "/tmp/alife-finalization-reference.rs"),
    ("/tmp/alife-parallel-preparation-helpers.rs", "/tmp/alife-finalization-helpers.rs"),
    ("docs/performance/evidence/20261001-parallel-preparation-probe.rs",
     "docs/performance/evidence/20261001-finalization-probe.rs"),
]:
    assert source.count(old) == 1, old
    source = source.replace(old, new)
exec(compile(source, str(Path(__file__).resolve()), "exec"))
manifest_path = executable.with_suffix(".build.json")
manifest = json.loads(manifest_path.read_text())
manifest["reused_builder_commit"] = _finalization_reused_commit
manifest["reused_builder_sha256"] = _finalization_builder_sha256
manifest["source_commit"] = subprocess.check_output(["git", "rev-parse", "HEAD"]).decode().strip()
manifest["core_memory_sha256"] = hashlib.sha256(
    (root / "crates/alife_core/src/memory.rs").read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
