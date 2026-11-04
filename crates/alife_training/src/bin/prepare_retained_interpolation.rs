//! Inference-only preparation. No Current export, optimizer, world dispatch or
//! model promotion is performed. Real source admission remains owner-qualified.
use alife_training::{
    cpu::retained_interpolation_experiment::*, resolve_isolated_training_output,
    MAX_CURRENT_CHECKPOINT_BYTES,
};
use alife_world::{with_saved_body_rounding_compatibility, SavedBodyRoundingCompatibilityV1};
use serde::Deserialize;
use std::{
    error::Error,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    pc_current: PathBuf,
    expected_pc_current_sha256: String,
    cloud_native_cpu_root: PathBuf,
    expected_cloud_tree_sha256: String,
    cloud_fraction: f64,
    output: PathBuf,
    body_rounding_proof: Option<PathBuf>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BodyProof {
    source_current_sha256: String,
    proof: SavedBodyRoundingCompatibilityV1,
}
fn scoped<T>(proof: Option<&[u8]>, pin: &str, action: impl FnOnce() -> Result<T>) -> Result<T> {
    if let Some(bytes) = proof {
        let declared: BodyProof = serde_json::from_slice(bytes)?;
        if declared.source_current_sha256 != pin {
            return Err("body proof differs from selected source SHA256".into());
        }
        let (result, used) = with_saved_body_rounding_compatibility(declared.proof, action)
            .map_err(io::Error::other)?;
        let result = result?;
        if !used {
            return Err("declared body proof was not used".into());
        }
        Ok(result)
    } else {
        action()
    }
}
fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() > limit {
        return Err("source must be a bounded regular nonsymlink file".into());
    }
    let bytes = fs::read(path)?;
    if bytes.len() as u64 != m.len() {
        return Err("source changed while reading".into());
    }
    Ok(bytes)
}
fn write(path: PathBuf, bytes: &[u8]) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
fn verify(root: &Path) -> Result<RetainedInterpolationExperiment> {
    let file = fs::File::open(root.join("experiment.json.zst"))?;
    let reader = zstd::stream::read::Decoder::new(file)?.take(1024 * 1024 * 1024);
    let candidate: RetainedInterpolationExperiment = serde_json::from_reader(reader)?;
    let proof = match fs::read(root.join("source-body-proof.json")) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    scoped(
        proof.as_deref(),
        &candidate.inputs().expected_pc_current_sha256,
        || {
            candidate.validate()?;
            Ok(())
        },
    )?;
    if bounded(
        &root.join("candidate-compiler-inputs.json"),
        MAX_CURRENT_CHECKPOINT_BYTES,
    )? != serde_json::to_vec(candidate.compiler_inputs())?
        || bounded(
            &root.join("candidate-retained-neural.json"),
            MAX_CURRENT_CHECKPOINT_BYTES,
        )? != serde_json::to_vec(candidate.retained())?
        || bounded(
            &root.join("candidate.alife-foundation"),
            MAX_CURRENT_CHECKPOINT_BYTES,
        )? != candidate
            .compiler_inputs()
            .n2048_candidate_asset()
            .ok_or("candidate asset missing")?
            .encode_canonical()?
        || bounded(
            &root.join("inference-only-receipt.json"),
            MAX_CURRENT_CHECKPOINT_BYTES,
        )? != serde_json::to_vec(candidate.receipt())?
    {
        return Err("candidate export differs from validated experiment".into());
    }
    Ok(candidate)
}
fn prepare(spec: &Path) -> Result<()> {
    let raw = bounded(spec, 1024 * 1024)?;
    let request: Request = serde_json::from_slice(&raw)?;
    let mut protected = vec![
        spec.as_ref(),
        request.pc_current.as_path(),
        request.cloud_native_cpu_root.as_path(),
    ];
    if let Some(proof) = &request.body_rounding_proof {
        protected.push(proof.as_path());
    }
    let output = resolve_isolated_training_output(&request.output, protected)?;
    if output.exists() {
        return Err("existing experiment is preserved; choose a new absent output".into());
    }
    let bytes = bounded(&request.pc_current, MAX_CURRENT_CHECKPOINT_BYTES)?;
    let proof = request
        .body_rounding_proof
        .as_ref()
        .map(|p| bounded(p, 1024 * 1024))
        .transpose()?;
    let archive = capture_native_cpu_archive(
        &request.cloud_native_cpu_root,
        &request.expected_cloud_tree_sha256,
    )?;
    let inputs = InterpolationExperimentInputs {
        pc_current: bytes.clone(),
        expected_pc_current_sha256: request.expected_pc_current_sha256.clone(),
        cloud_parent: InterpolationCloudParent::NativeCpuArchive { archive },
        cloud_fraction: request.cloud_fraction,
    };
    let candidate = scoped(
        proof.as_deref(),
        &request.expected_pc_current_sha256,
        || Ok(prepare_retained_interpolation_experiment(inputs)?),
    )?;
    fs::create_dir_all(output.parent().ok_or("output lacks parent")?)?;
    fs::create_dir(&output)?;
    write(output.join("request.json"), &raw)?;
    if let Some(proof) = proof {
        write(output.join("source-body-proof.json"), &proof)?;
    }
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("experiment.json.zst"))?;
    let mut encoder = zstd::stream::write::Encoder::new(file, 3)?;
    serde_json::to_writer(&mut encoder, &candidate)?;
    encoder.finish()?.sync_all()?;
    write(
        output.join("candidate-compiler-inputs.json"),
        &serde_json::to_vec(candidate.compiler_inputs())?,
    )?;
    write(
        output.join("candidate-retained-neural.json"),
        &serde_json::to_vec(candidate.retained())?,
    )?;
    write(
        output.join("candidate.alife-foundation"),
        &candidate
            .compiler_inputs()
            .n2048_candidate_asset()
            .ok_or("candidate asset missing")?
            .encode_canonical()?,
    )?;
    write(
        output.join("inference-only-receipt.json"),
        &serde_json::to_vec(candidate.receipt())?,
    )?;
    verify(&output)?;
    if bounded(&request.pc_current, MAX_CURRENT_CHECKPOINT_BYTES)? != bytes
        || bounded(spec, 1024 * 1024)? != raw
    {
        return Err("source or request changed during preparation; preserve outputs".into());
    }
    capture_native_cpu_archive(
        &request.cloud_native_cpu_root,
        &request.expected_cloud_tree_sha256,
    )?;
    println!("{}", serde_json::to_string(candidate.receipt())?);
    Ok(())
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice(){
        [mode,root]if mode=="inspect-cpu"=>println!("{}",serde_json::to_string(&inspect_native_cpu_archive(Path::new(root))?)?),
        [mode,spec]if mode=="prepare"=>prepare(Path::new(spec))?,
        [mode,root]if mode=="verify"=>println!("{}",serde_json::to_string(verify(Path::new(root))?.receipt())?),
        _=>return Err("usage: prepare_retained_interpolation inspect-cpu CPU_ROOT | prepare SPEC.json | verify OUTPUT".into()),
    }
    Ok(())
}
