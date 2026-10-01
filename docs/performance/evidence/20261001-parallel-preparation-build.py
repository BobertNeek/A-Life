"""Build the dated CPU-only parallel preparation experiment from Cargo artifacts."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path.cwd()
artifacts, executable = map(Path, sys.argv[1:])
reference = root / "docs/performance/evidence/20261001-attention-preparation-probe.rs"
original = reference.read_text()
def replace_checked(text, old, new, count=1):
    assert text.count(old) == count, (old, text.count(old), count)
    return text.replace(old, new)


source = original[:original.index("fn paired_samples(")]
source = replace_checked(source, "//! One-off CPU preparation probe; never creates or dispatches a GPU device.",
                        "// Reused dated CPU preparation implementation; no device execution.")
source = replace_checked(source, '"../../../crates/alife_core/tests/candidate_memory_retrieval.rs"',
                        json.dumps(str(root / "crates/alife_core/tests/candidate_memory_retrieval.rs")))
source = replace_checked(source, "pub fn input(count: usize, learned_records: usize)",
                        "pub fn input(owner: OrganismId, count: usize, learned_records: usize)")
start = source.index("    pub fn input(")
end = source.index("// Generated verbatim", start)
fixture = replace_checked(source[start:end], "ORGANISM,", "owner,", count=2)
fixture = replace_checked(fixture, "grounded_draft(0.4)", "grounded_draft(0.25 + (owner.raw() % 5) as f32 * 0.08)")
fixture = replace_checked(fixture, "bank.observe_sealed_patch(&sequenced_patch_for_object(",
                          "bank.observe_sealed_patch(&patch_for_owner(owner, sequenced_patch_for_object(")
fixture = replace_checked(fixture, "            ))\n            .unwrap();", "            )))\n            .unwrap();")
source = source[:start] + fixture + source[end:]
source = replace_checked(source, "type CpuResult = (", "pub type CpuResult = (")
source = replace_checked(source, "fn topology_fixture(", "pub fn topology_fixture(")
source = replace_checked(source, "fn full_cpu_preparation(", "pub fn full_cpu_preparation(")
source += """
pub fn input(owner: OrganismId, count: usize, records: usize) -> (PerceptionFrameDraft, MemoryBank) {
    fixture::input(owner, count, records)
}
"""
# Fixtures are sealed again using public constructors; no digest/owner rewriting.
insert = source.index("    pub fn input(")
source = source[:insert] + """
    fn patch_for_owner(owner: OrganismId, patch: ExperiencePatch) -> ExperiencePatch {
        let old = patch.pre_action().perception();
        let draft = PerceptionFrameDraft::new(owner, old.tick(), old.sensor_profile(),
            SensorySnapshot::new(owner, old.tick(), old.sensory().observer_position,
                old.sensory().channels, old.sensory().context_streams.clone()).unwrap(),
            old.body(), *old.homeostasis(), old.candidates().to_vec(),
            old.profile_provenance(), old.grounded_object_slots().to_vec()).unwrap();
        let (frame, recall) = empty_bank().recall_frame(&draft).unwrap().finalize(draft).unwrap();
        let sequence = patch.header().sequence_id;
        let decision = DecisionSnapshot::from_neural_selection(sequence, PhenotypeHash([1,2,3,4]),
            sequence.raw(), (sequence.raw() & 1) as u8, &frame,
            NeuralActionSelection {candidate_index:0, logit:0.7, confidence:Confidence(0.8),
                active_tiles:8, active_synapses:64},
            frame.candidates()[0].to_command(owner, Confidence(0.8)).unwrap()).unwrap()
            .with_finalized_memory_recall(&frame, &recall, 0).unwrap();
        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let genome = BrainGenome::scaffold(321, spec.id);
        let development = DevelopmentState::new(genome.id, frame.tick(), NormalizedScalar(0.5))
            .with_enabled_lobes([LobeKind::PerceptualIntegration, LobeKind::TemporalPredictive,
                LobeKind::ActionPlanning]);
        let pre = PreActionSnapshot::from_neural_frame(sequence, spec.id, PhenotypeHash([1,2,3,4]),
            genome.id, genome.schema_version, development, frame).unwrap();
        let mut outcome = patch.outcome().clone();
        outcome.organism_id = owner;
        outcome.validate_contract().unwrap();
        ExperiencePatchBuilder::new(sequence).record_pre_action(pre).unwrap()
            .record_decision(decision).unwrap().record_outcome(outcome).unwrap().seal().unwrap()
    }
""" + source[insert:]
generated = Path("/tmp/alife-parallel-preparation-reference.rs")
generated.write_text(source)
app = (root / "crates/alife_game_app/src/gpu_live_runtime.rs").read_text()
start = app.index("fn cognitive_context_for_recall(")
end = app.index("fn grounded_successor_state(", start)
helper = Path("/tmp/alife-parallel-preparation-helpers.rs")
helper.write_text(app[start:end])
libraries = {}
for line in artifacts.read_text().splitlines():
    if not line.startswith("{"):
        continue
    item = json.loads(line)
    if item.get("reason") == "compiler-artifact" and "lib" in item["target"]["kind"]:
        for filename in item["filenames"]:
            if filename.endswith(".rlib"):
                libraries[item["target"]["name"]] = (filename, item["profile"], item["features"])
probe = root / "docs/performance/evidence/20261001-parallel-preparation-probe.rs"
command = ["rustc", "--edition=2021", "-O", "-C", "debug-assertions=no", str(probe)]
linked = {}
for name in ["alife_core", "alife_gpu_backend", "alife_world", "serde_json", "blake3"]:
    filename, profile, features = libraries[name]
    assert profile["opt_level"] == "3" and not profile["debug_assertions"]
    command += ["--extern", name + "=" + filename]
    linked[name] = {"filename": filename, "profile": profile, "features": features,
                    "sha256": hashlib.sha256(Path(filename).read_bytes()).hexdigest()}
command += ["-L", "dependency=" + str(Path(libraries["alife_core"][0]).parent), "-o", str(executable)]
subprocess.run(command, check=True, env=dict(os.environ,
    ALIFE_PARALLEL_REFERENCE=str(generated), ALIFE_ATTENTION_APP_HELPERS=str(helper)))
identities = {"linked": linked}
for name, path in {"reference": reference, "generated_reference": generated, "helper": helper,
                   "probe": probe, "builder": Path(__file__), "executable": executable}.items():
    identities[name + "_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
executable.with_suffix(".build.json").write_text(json.dumps(identities, indent=2) + "\n")
print("Built CPU-only parallel prototype", executable)
