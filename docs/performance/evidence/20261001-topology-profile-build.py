"""Build the dated, Linux-only topology CPU probe from exact Cargo artifacts."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

BASE = "471c55dfc2a29e760669e8bb6252865464e064fa"
root = Path.cwd()
artifacts_file, executable = map(Path, sys.argv[1:])
original = subprocess.check_output(
    ["git", "show", f"{BASE}:crates/alife_core/src/topology.rs"], text=True
)
source = original


def replace_once(old, new):
    global source
    assert source.count(old) == 1, old
    source = source.replace(old, new, 1)


replace_once("//! v0 scaffold: CPU-side topological concept map and curiosity gap contracts.",
             "// Source-derived one-off profiling copy.")
# Only the unexecuted context-contribution API crosses the copied module boundary.
replace_once("                    gap_id: gap.id,",
             "                    gap_id: crate::UnresolvedGapId(gap.id.raw()),")
start = source.index("    fn plan_observation(")
end = source.index("    pub fn config(", start)
plan = source[start:end]
for old, new in [
    ("        patch.validate_contract()?;", "        let profile_start=std::time::Instant::now();\n        patch.validate_contract()?;\n        profile_add(0,profile_start);"),
    ("        let mut planned = self.clone();", "        let profile_start=std::time::Instant::now();\n        let mut planned = self.clone();\n        profile_add(1,profile_start);"),
    ("        planned.validate_contract()?;", "        let profile_start=std::time::Instant::now();\n        planned.validate_contract()?;"),
    ("        let final_digest = planned.canonical_digest()?;", "        let final_digest = planned.canonical_digest()?;\n        profile_add(2,profile_start);"),
    ("        plan.validate_against(self)?;", "        let profile_start=std::time::Instant::now();\n        plan.validate_against(self)?;\n        profile_add(3,profile_start);"),
]:
    assert plan.count(old) == 1, old
    plan = plan.replace(old, new, 1)
source = source[:start] + plan + source[end:]
replace_once('    plan.validate_against(map)\n        .expect("private topology mutation plan was validated before commit");',
             '    let profile_start=std::time::Instant::now();\n    plan.validate_against(map)\n        .expect("private topology mutation plan was validated before commit");\n    profile_add(4,profile_start);')
replace_once('    apply_replacements_checked(map, &plan.replacements)\n        .expect("prevalidated topology replacements have exact targets");',
             '    let profile_start=std::time::Instant::now();\n    apply_replacements_checked(map, &plan.replacements)\n        .expect("prevalidated topology replacements have exact targets");\n    profile_add(5,profile_start);')
source += """
use std::sync::atomic::{AtomicU64,Ordering};
static PROFILE_NS:[AtomicU64;6]=[const { AtomicU64::new(0) };6];
pub fn profile_reset(){ for counter in &PROFILE_NS {counter.store(0,Ordering::Relaxed);} }
pub fn profile_read()->[u64;6] { std::array::from_fn(|i|PROFILE_NS[i].load(Ordering::Relaxed)) }
fn profile_add(index:usize,start:std::time::Instant){PROFILE_NS[index].fetch_add(start.elapsed().as_nanos() as u64,Ordering::Relaxed);}
"""
profiled = Path("/tmp/alife-topology-profiled.rs")
profiled.write_text(source)
app = (root / "crates/alife_game_app/src/gpu_live_runtime.rs").read_text()
start = app.index("fn cognitive_context_for_recall(")
end = app.index("fn apply_predecision_attention_evidence(", start)
helper = Path("/tmp/alife-topology-context-helper.rs")
helper.write_text(app[start:end])
libraries = {}
for line in artifacts_file.read_text().splitlines():
    if not line.startswith("{"):
        continue
    item = json.loads(line)
    if item.get("reason") == "compiler-artifact" and "lib" in item["target"]["kind"]:
        for filename in item["filenames"]:
            if filename.endswith(".rlib"):
                libraries[item["target"]["name"]] = (filename, item["profile"],
                    item["package_id"], item["features"])
probe = root / "docs/performance/evidence/20261001-topology-profile-probe.rs"
command = ["rustc", "--edition=2021", "-O", "-C", "debug-assertions=no", str(probe)]
linked = {}
for name in ["alife_core", "serde", "serde_json", "blake3"]:
    filename, profile, package_id, features = libraries[name]
    assert profile["opt_level"] == "3" and not profile["debug_assertions"]
    command += ["--extern", name + "=" + filename]
    linked[name] = {"filename": filename, "profile": profile,
                    "package_id": package_id, "features": features,
                    "sha256": hashlib.sha256(Path(filename).read_bytes()).hexdigest()}
command += ["-L", "dependency=" + str(Path(libraries["alife_core"][0]).parent),
            "-o", str(executable)]
subprocess.run(command, check=True, env=dict(os.environ,
               ALIFE_TOPOLOGY_PROFILE_SOURCE=str(profiled), ALIFE_TOPOLOGY_CONTEXT_HELPER=str(helper)))
identities = {"baseline": BASE, "linked": linked}
for name, content in {"baseline_topology": original.encode(), "profiled_topology": profiled.read_bytes(),
                      "context_helper": helper.read_bytes(), "probe": probe.read_bytes(),
                      "builder": Path(__file__).read_bytes(), "executable": executable.read_bytes()}.items():
    identities[name + "_sha256"] = hashlib.sha256(content).hexdigest()
executable.with_suffix(".build.json").write_text(json.dumps(identities, indent=2) + "\n")
print("Built dated topology probe; exact identities in", executable.with_suffix(".build.json"))
