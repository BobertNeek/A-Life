//! Explicit experimental fork admission/export. This is not the ordinary
//! single-parent continuation receipt domain and cannot promote a brain.
use super::*;
use alife_core::*;
use alife_training::{
    cpu::retained_interpolation_experiment::*, cpu::retained_training_fork::*, *,
};
use alife_world::*;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
mod run;
mod verify;
pub use run::run_pc_donor_retained_training_fork;
pub use verify::verify_pc_donor_retained_training_fork;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    prepared: PathBuf,
    expected_experiment_sha256: String,
    pc_current: PathBuf,
    expected_pc_sha256: String,
    cloud_native_root: PathBuf,
    expected_cloud_tree_sha256: String,
    cloud_archive: PathBuf,
    expected_cloud_archive_sha256: String,
    seed_history: PathBuf,
    expected_seed_history_sha256: String,
    seed: u64,
    recovery_seed: u64,
    decisions: usize,
    optimizer_initialization: String,
    output: PathBuf,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForkManifest {
    schema: u32,
    kind: String,
    experiment_sha256: String,
    training_seed: u64,
    recovery_seed: u64,
    policy_version: u64,
    ppo: PpoConfig,
    artifacts: BTreeMap<String, String>,
}
struct Context {
    experiment_bytes: Vec<u8>,
    experiment: RetainedInterpolationExperiment,
    pc: CurrentTrainingCheckpointV1,
    source_proof: Option<SavedBodyRoundingCompatibilityV1>,
    seed_history: CheckpointSeedsV1,
    seed_evidence: Vec<Value>,
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn write(path: impl AsRef<Path>, v: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec(v)?)?;
    Ok(())
}
fn read<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn scope<T>(
    proof: &Option<SavedBodyRoundingCompatibilityV1>,
    f: impl FnOnce() -> Result<T>,
) -> Result<T> {
    if let Some(p) = proof {
        let (result, _) = with_saved_body_rounding_compatibility(p.clone(), f)?;
        result
    } else {
        f()
    }
}
fn decode_zstd<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let decoder =
        zstd::stream::read::Decoder::new(std::io::Cursor::new(bytes))?.take(1024 * 1024 * 1024);
    Ok(serde_json::from_reader(decoder)?)
}
fn write_zstd(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    let mut encoder = zstd::stream::write::Encoder::new(fs::File::create_new(path)?, 1)?;
    encoder.include_checksum(true)?;
    serde_json::to_writer(&mut encoder, value)?;
    encoder.flush()?;
    encoder.finish()?.sync_all()?;
    Ok(())
}
fn pinned(path: impl AsRef<Path>, expected: &str) -> Result<Vec<u8>> {
    let path = path.as_ref();
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 1024 * 1024 * 1024 {
        return Err("fork input must be a bounded regular file".into());
    }
    let bytes = fs::read(path)?;
    if hash(&bytes) != expected {
        return Err(format!("fork source pin differs: {}", path.display()).into());
    }
    Ok(bytes)
}
fn check_seed(history: &CheckpointSeedsV1, evidence: &[Value], seed: u64) -> Result<()> {
    history.validate()?;
    if seed == 0 || history.contains(seed) {
        return Err("fork seed overlaps a disclosed exclusion".into());
    }
    for e in evidence {
        crate::foundation_training_cycle::verify_native_cpu_care_training_exclusions(e, seed)?;
    }
    Ok(())
}
fn context(req: &Request) -> Result<Context> {
    if req.optimizer_initialization != "complete_pc_donor_optimizer_transfer"
        || !(1..=128).contains(&req.decisions)
        || req.seed == req.recovery_seed
        || req.recovery_seed != req.seed.checked_add(1).ok_or("fork seed overflow")?
    {
        return Err(
            "explicit complete PC donor, bounded dose and consecutive fresh seeds required".into(),
        );
    }
    let pc_bytes = pinned(&req.pc_current, &req.expected_pc_sha256)?;
    pinned(&req.cloud_archive, &req.expected_cloud_archive_sha256)?;
    let (pc, source_proof) = match crate::read_current_training_checkpoint(&req.pc_current) {
        Ok(p) => (p, None),
        Err(_) => {
            let p = crate::propose_current_body_rounding_compatibility(
                &req.pc_current,
                &req.expected_pc_sha256,
            )?;
            (
                crate::read_current_training_checkpoint_with_body_rounding(
                    &req.pc_current,
                    &req.expected_pc_sha256,
                    &p.proof,
                )?,
                Some(p.proof),
            )
        }
    };
    if inspect_native_cpu_archive(&req.cloud_native_root)?.tree_sha256
        != req.expected_cloud_tree_sha256
    {
        return Err("fork cloud tree differs".into());
    }
    let cloud = read_cpu_founder_source(&req.cloud_native_root)?;
    let lineage = native_receipt::Lineage::load(&cloud.receipt, &req.cloud_native_root)?;
    let mut seed_evidence = vec![
        cloud.receipt,
        native_receipt::read(&req.cloud_native_root.join("care-seed-admission.json"))?.1,
    ];
    for frame in lineage.frames() {
        seed_evidence.push(frame?);
    }
    let seed_history: CheckpointSeedsV1 = serde_json::from_slice(&pinned(
        &req.seed_history,
        &req.expected_seed_history_sha256,
    )?)?;
    seed_history.validate()?;
    if seed_history.historical_freshness != "unknown"
        || seed_history.pending_range.is_some()
        || pc.seeds.excluded_ranges.iter().any(|[a, b]| {
            !seed_history
                .excluded_ranges
                .iter()
                .any(|[x, y]| x <= a && b <= y)
        })
    {
        return Err("fork history must retain all PC exclusions and unknown freshness".into());
    }
    for seed in [req.seed, req.recovery_seed] {
        check_seed(&seed_history, &seed_evidence, seed)?;
    }
    let experiment_bytes = pinned(
        req.prepared.join("experiment.json.zst"),
        &req.expected_experiment_sha256,
    )?;
    let experiment: RetainedInterpolationExperiment = decode_zstd(&experiment_bytes)?;
    if experiment.inputs().pc_current != pc_bytes || experiment.inputs().cloud_fraction != 0.25 {
        return Err("fork experiment differs from selected actual PC or fraction".into());
    }
    match &experiment.inputs().cloud_parent {
        InterpolationCloudParent::NativeCpuArchive { archive }
            if archive.tree_sha256()? == req.expected_cloud_tree_sha256 => {}
        _ => return Err("fork experiment differs from selected actual cloud parent".into()),
    }
    scope(&source_proof, || {
        experiment.validate()?;
        Ok(())
    })?;
    let asset = experiment
        .compiler_inputs()
        .n2048_candidate_asset()
        .ok_or("fork asset missing")?;
    if fs::read(req.prepared.join("candidate.alife-foundation"))? != asset.encode_canonical()? {
        return Err("fork prepared asset differs".into());
    }
    Ok(Context {
        experiment_bytes,
        experiment,
        pc,
        source_proof,
        seed_history,
        seed_evidence,
    })
}
fn body_inputs(
    world: &HeadlessWorld,
    asset: &FoundationWeightAsset,
) -> Result<(BrainPhenotype, PhenotypeCompilerInputs)> {
    let r = world
        .organism_registry()
        .iter()
        .next()
        .ok_or("fork body missing")?;
    let genome = r.phenotype().brain_genome.clone();
    let development = alife_runtime::canonical_preparation::foundation_construction_development(
        &genome,
        &BrainCapacityClass::n2048(),
        &r.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    Ok(PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome,
        development,
        asset.clone(),
    )?)
}
fn restore_private(
    root: &Path,
    trainer: &mut CpuFoundationTrainer,
    proof: &Option<SavedBodyRoundingCompatibilityV1>,
) -> Result<CpuCanonicalHost> {
    let raw = read_cpu_private_checkpoint_bytes(root, 1024 * 1024 * 1024)?;
    let cp: CpuCanonicalCheckpoint = scope(proof, || Ok(serde_json::from_slice(&raw)?))?;
    let world = scope(proof, || Ok(cp.world.restore_headless_world()?))?;
    let asset = FoundationWeightAsset::decode_canonical(&fs::read(
        root.join("collection-policy.alife-foundation"),
    )?)?;
    trainer.bind_equivalent_assessment_body(&body_inputs(&world, &asset)?.0)?;
    let host = scope(proof, || {
        Ok(CpuCanonicalHost::from_checkpoint(cp, trainer)?)
    })?;
    if serde_json::to_vec(&host.checkpoint(trainer)?)? != raw {
        return Err("fork private cold roundtrip differs".into());
    }
    Ok(host)
}
fn inventory(root: &Path) -> Result<BTreeMap<String, String>> {
    fn visit(
        root: &Path,
        dir: &Path,
        out: &mut BTreeMap<String, String>,
        total: &mut u64,
    ) -> Result<()> {
        for e in fs::read_dir(dir)? {
            let path = e?.path();
            let m = fs::symlink_metadata(&path)?;
            if m.file_type().is_symlink() {
                return Err("fork artifact symlink refused".into());
            }
            if m.is_dir() {
                visit(root, &path, out, total)?;
            } else if m.is_file() {
                let rel = path
                    .strip_prefix(root)?
                    .to_str()
                    .ok_or("fork path UTF8")?
                    .replace(std::path::MAIN_SEPARATOR, "/");
                if rel == "retained-fork.json" {
                    continue;
                }
                *total = total.checked_add(m.len()).ok_or("fork size overflow")?;
                if *total > 1024 * 1024 * 1024 || out.len() >= 256 {
                    return Err("fork artifact bound exceeded".into());
                }
                out.insert(rel, hash(&fs::read(&path)?));
            } else {
                return Err("fork artifact nonregular file".into());
            }
        }
        Ok(())
    }
    if fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err("fork root symlink refused".into());
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out, &mut 0)?;
    Ok(out)
}
