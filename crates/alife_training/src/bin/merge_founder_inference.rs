//! Evaluation-only adapter. Ancestry authentication remains the caller's responsibility.
use alife_core::{
    BrainCapacityClass, BrainPhenotype, FoundationWeightAsset, PhenotypeCompiler,
    PhenotypeCompilerInputs, TrainingStageManifest,
};
use alife_training::{
    merge_founder_branches, native_receipt::sha256, resolve_isolated_training_output,
    FoundationTrainerCheckpoint, FounderMergeBranch,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    phenotype: Option<PathBuf>,
    compiler_inputs: Option<PathBuf>,
    foundation: PathBuf,
    actor_checkpoint: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    parent: Source,
    pc: Source,
    cpu: Source,
    ancestry_audit: PathBuf,
}
struct Saved {
    phenotype: BrainPhenotype,
    asset: FoundationWeightAsset,
    actor: FoundationTrainerCheckpoint,
}

fn read(path: &Path, pins: &mut BTreeMap<String, String>) -> Result<Vec<u8>> {
    let path = path.canonicalize()?;
    let bytes = fs::read(&path)?;
    pins.insert(path.to_string_lossy().into_owned(), sha256(&bytes));
    Ok(bytes)
}
fn load(source: &Source, pins: &mut BTreeMap<String, String>) -> Result<Saved> {
    let phenotype = match (&source.phenotype, &source.compiler_inputs) {
        (Some(path), None) => serde_json::from_slice(&read(path, pins)?)?,
        (None, Some(path)) => {
            let inputs: PhenotypeCompilerInputs = serde_json::from_slice(&read(path, pins)?)?;
            PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048())
                .map_err(|e| format!("compile_validated {}: {e:?}", path.display()))?
        }
        _ => {
            return Err("each source requires exactly one phenotype or compiler_inputs file".into())
        }
    };
    let saved = Saved {
        phenotype,
        asset: FoundationWeightAsset::decode_canonical(&read(&source.foundation, pins)?)?,
        actor: serde_json::from_slice(&read(&source.actor_checkpoint, pins)?)?,
    };
    saved
        .asset
        .validate_against(&saved.phenotype)
        .map_err(|e| {
            format!(
                "asset.validate_against: {e:?}; asset={:?}; phenotype={:?}",
                saved.asset.digest(),
                saved.phenotype.phenotype_hash()
            )
        })?;
    saved.actor.validate_for(&saved.phenotype, saved.asset.digest())
        .map_err(|e| format!("actor.validate_for: {e:?}; actor phenotype={:?}; compiled phenotype={:?}; actor source={:?}; asset={:?}", saved.actor.phenotype_hash, saved.phenotype.phenotype_hash(), saved.actor.source_foundation_digest, saved.asset.digest()))?;
    if saved
        .actor
        .weights
        .iter()
        .zip(saved.asset.weights())
        .zip(saved.phenotype.synapses())
        .any(|((a, b), s)| {
            a.to_bits() != b.to_bits() || a.to_bits() != s.genetic_weight().to_bits()
        })
    {
        return Err("actor, founder asset and compiled genetic parameters differ".into());
    }
    Ok(saved)
}
fn same_graph(a: &BrainPhenotype, b: &BrainPhenotype) -> bool {
    a.foundation_abi_selection().canonical_v2().is_some()
        && b.foundation_abi_selection().canonical_v2().is_some()
        && a.schema_version() == b.schema_version()
        && a.brain_class_id() == b.brain_class_id()
        && a.neuron_count() == b.neuron_count()
        && a.microstep_count() == b.microstep_count()
        && a.sensor_profile() == b.sensor_profile()
        && a.lobe_layout() == b.lobe_layout()
        && a.language_codebook() == b.language_codebook()
        && a.cognitive_architecture() == b.cognitive_architecture()
        && a.projections() == b.projections()
        && a.neuron_dynamics() == b.neuron_dynamics()
        && a.sensor_encoder() == b.sensor_encoder()
        && a.candidate_decoder() == b.candidate_decoder()
        && a.speech_decoder() == b.speech_decoder()
        && a.memory_decoder() == b.memory_decoder()
        && a.cognitive_decoder() == b.cognitive_decoder()
        && a.cognitive_channel_plan() == b.cognitive_channel_plan()
        && a.plasticity_receptors() == b.plasticity_receptors()
        && a.replay_capture_plan() == b.replay_capture_plan()
        && a.sleep_consolidation_plan() == b.sleep_consolidation_plan()
        && a.plasticity_plan_digest() == b.plasticity_plan_digest()
        && a.persistent_address_map() == b.persistent_address_map()
        && a.route_abi_digest() == b.route_abi_digest()
        && a.plasticity_abi_digest() == b.plasticity_abi_digest()
        && a.budgets() == b.budgets()
        && a.synapses().len() == b.synapses().len()
        && a.synapses().iter().zip(b.synapses()).all(|(x, y)| {
            x.source() == y.source()
                && x.target() == y.target()
                && x.alpha().to_bits() == y.alpha().to_bits()
                && x.route_index() == y.route_index()
                && x.receptor_index() == y.receptor_index()
                && x.kind() == y.kind()
        })
}
fn merge(
    parent: &Saved,
    pc: &Saved,
    cpu: &Saved,
) -> Result<(
    FoundationWeightAsset,
    alife_training::FounderMergeReceipt,
    Value,
)> {
    let parent_stage = parent.asset.manifest().training_stage();
    for (role, branch) in [("PC", pc), ("CPU", cpu)] {
        if !same_graph(&parent.phenotype, &branch.phenotype)
            || branch.actor.stage_mask != parent.actor.stage_mask
        {
            return Err("branch graph or actual stage mask differs from common parent".into());
        }
        let stage = branch.asset.manifest().training_stage();
        if stage.curriculum_version() != parent_stage.curriculum_version() {
            return Err(format!("{role} curriculum version differs from common parent").into());
        }
        if stage.evaluation_version() != parent_stage.evaluation_version() {
            return Err(format!("{role} evaluation version differs from common parent").into());
        }
    }
    let branch = |saved: &Saved| -> Result<FounderMergeBranch> {
        let stage = saved.asset.manifest().training_stage();
        let original = FoundationWeightAsset::from_trained_weights(
            &parent.phenotype,
            saved.asset.weights().to_vec(),
            stage,
        )?;
        if original != saved.asset {
            return Err(
                "original branch metadata differs from its own-stage reconstruction or is promoted"
                    .into(),
            );
        }
        let asset = FoundationWeightAsset::from_trained_weights(
            &parent.phenotype,
            saved.asset.weights().to_vec(),
            TrainingStageManifest::new(
                stage.curriculum_version(),
                stage.evaluation_version(),
                parent_stage.completed_stage_count(),
            ),
        )?;
        if asset
            .weights()
            .iter()
            .zip(saved.asset.weights())
            .any(|(a, b)| a.to_bits() != b.to_bits())
        {
            return Err("evaluation view changed original weights".into());
        }
        Ok(FounderMergeBranch {
            asset,
            parent_asset_digest: parent.asset.digest(),
            parent_phenotype_hash: parent.phenotype.phenotype_hash(),
            parent_compiler_inputs_digest: parent.phenotype.compiler_inputs_digest(),
        })
    };
    let pc_view = branch(pc)?;
    let cpu_view = branch(cpu)?;
    let views = json!({"derivation":"FoundationWeightAsset::from_trained_weights using the common parent graph and completed_stage_count; curriculum/evaluation versions remain unchanged; original checkpoints are not rebound",
        "weights_bit_identical":true,"original_own_stage_reconstruction_equal":true,
        "pc":{"original_digest":pc.asset.digest(),"original_stage":pc.asset.manifest().training_stage(),
            "view_digest":pc_view.asset.digest(),"view_stage":pc_view.asset.manifest().training_stage()},
        "cpu":{"original_digest":cpu.asset.digest(),"original_stage":cpu.asset.manifest().training_stage(),
            "view_digest":cpu_view.asset.digest(),"view_stage":cpu_view.asset.manifest().training_stage()}});
    let (asset, receipt) = merge_founder_branches(
        &parent.phenotype,
        &parent.asset,
        &pc_view,
        &cpu_view,
        &parent.actor.stage_mask,
    )?;
    Ok((asset, receipt, views))
}
fn publish(
    output: &Path,
    asset: &[u8],
    receipt: &[u8],
    write_receipt: impl FnOnce(&mut fs::File, &[u8]) -> io::Result<()>,
) -> io::Result<()> {
    fs::create_dir(output)?;
    let mut owned = Vec::new();
    let result = (|| {
        let asset_path = output.join("merged.alife-foundation");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&asset_path)?;
        owned.push(asset_path);
        file.write_all(asset)?;
        file.sync_all()?;
        drop(file);

        let receipt_path = output.join("merge-receipt.json");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&receipt_path)?;
        owned.push(receipt_path);
        write_receipt(&mut file, receipt)?;
        file.sync_all()?;
        drop(file);
        #[cfg(unix)]
        fs::File::open(output)?.sync_all()?;
        Ok(())
    })();
    if let Err(error) = result {
        let mut failures = Vec::new();
        for path in owned.iter().rev() {
            if let Err(cleanup) = fs::remove_file(path) {
                if cleanup.kind() != io::ErrorKind::NotFound {
                    failures.push(format!("{}: {cleanup}", path.display()));
                }
            }
        }
        if let Err(cleanup) = fs::remove_dir(output) {
            if cleanup.kind() != io::ErrorKind::NotFound {
                failures.push(format!("{}: {cleanup}", output.display()));
            }
        }
        return if failures.is_empty() {
            Err(error)
        } else {
            Err(io::Error::other(format!(
                "publication failed: {error}; cleanup failed: {}",
                failures.join("; ")
            )))
        };
    }
    Ok(())
}

fn run(request_path: &Path, output: &Path) -> Result<Value> {
    let mut pins = BTreeMap::new();
    let request: Request = serde_json::from_slice(&read(request_path, &mut pins)?)?;
    let _: Value = serde_json::from_slice(&read(&request.ancestry_audit, &mut pins)?)?;
    let sources = [&request.parent, &request.pc, &request.cpu];
    let directories = sources
        .iter()
        .map(|s| {
            s.foundation.canonicalize().and_then(|p| {
                p.parent()
                    .map(Path::to_path_buf)
                    .ok_or_else(|| std::io::Error::other("input has no parent"))
            })
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    let preserved = directories.iter().map(PathBuf::as_path);
    let output = resolve_isolated_training_output(output, preserved)?;
    if output.exists() {
        return Err("output directory must be absent".into());
    }
    let parent = load(&request.parent, &mut pins).map_err(|e| format!("parent admission: {e}"))?;
    let pc = load(&request.pc, &mut pins).map_err(|e| format!("PC admission: {e}"))?;
    let cpu = load(&request.cpu, &mut pins).map_err(|e| format!("CPU admission: {e}"))?;
    let (asset, merged, views) = merge(&parent, &pc, &cpu)
        .map_err(|e| format!("merge_founder_branches strict contract: {e:?}"))?;
    let encoded = asset.encode_canonical()?;
    if FoundationWeightAsset::decode_canonical(&encoded)? != asset
        || asset
            != FoundationWeightAsset::from_trained_weights(
                &parent.phenotype,
                asset.weights().to_vec(),
                parent.asset.manifest().training_stage(),
            )?
    {
        return Err("merged candidate canonical roundtrip failed".into());
    }
    // Recheck the exact read-only inputs before publishing any candidate bytes.
    for (path, expected) in &pins {
        if sha256(&fs::read(path)?) != *expected {
            return Err("input changed during admission".into());
        }
    }
    let receipt = json!({"schema":1,"mode":"offline_optimizer_frozen_inference_only","ancestry_authenticated_by_tool":false,
        "ancestry_audit":request.ancestry_audit,"input_sha256":pins,"algorithm":"alife_training::merge_founder_branches",
        "parent_asset_digest":parent.asset.digest(),"pc_asset_digest":pc.asset.digest(),"cpu_asset_digest":cpu.asset.digest(),
        "candidate_asset_digest":asset.digest(),"candidate_sha256":sha256(&encoded),"unpromoted":true,
        "optimizer_updates":0,"critic_merged":false,"adam_merged":false,"private_state_merged":false,
        "non_resumable":true,"evaluation_views":views,
        "unique_changes":merged.unique_changes,"averaged_changes":merged.averaged_changes,
        "sign_conflicts":merged.sign_conflicts,"unchanged":merged.unchanged,"route_thresholds":merged.route_thresholds});
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)?;
    publish(&output, &encoded, &receipt_bytes, |file, bytes| {
        file.write_all(bytes)
    })?;
    Ok(receipt)
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: merge_founder_inference REQUEST.json ABSENT_OUTPUT_DIRECTORY".into());
    }
    let result = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || run(Path::new(&args[0]), Path::new(&args[1])).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "inference merge worker panicked")??;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "alife-inference-publication-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn partial_receipt_failure_cleans_owned_output_and_allows_retry_without_changing_inputs() {
        let root = fixture();
        let asset_input = root.join("input-asset");
        let receipt_input = root.join("input-receipt");
        fs::write(&asset_input, b"synthetic asset").unwrap();
        fs::write(&receipt_input, b"synthetic receipt").unwrap();
        let asset = fs::read(&asset_input).unwrap();
        let receipt = fs::read(&receipt_input).unwrap();
        let output = root.join("output");
        let error = publish(&output, &asset, &receipt, |file, bytes| {
            file.write_all(&bytes[..4])?;
            Err(io::Error::other("injected late receipt failure"))
        })
        .unwrap_err();
        assert!(error.to_string().contains("injected late receipt failure"));
        assert!(!output.exists());
        assert_eq!(fs::read(&asset_input).unwrap(), asset);
        assert_eq!(fs::read(&receipt_input).unwrap(), receipt);

        publish(&output, &asset, &receipt, |file, bytes| {
            file.write_all(bytes)
        })
        .unwrap();
        assert_eq!(
            fs::read(output.join("merged.alife-foundation")).unwrap(),
            asset
        );
        assert_eq!(
            fs::read(output.join("merge-receipt.json")).unwrap(),
            receipt
        );
        assert_eq!(fs::read(&asset_input).unwrap(), asset);
        assert_eq!(fs::read(&receipt_input).unwrap(), receipt);
        for path in [
            output.join("merged.alife-foundation"),
            output.join("merge-receipt.json"),
            asset_input,
            receipt_input,
        ] {
            fs::remove_file(path).unwrap();
        }
        fs::remove_dir(output).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn failed_publication_preserves_unrelated_files_and_reports_incomplete_cleanup() {
        let root = fixture();
        let output = root.join("output");
        let unrelated = output.join("unrelated");
        let error = publish(&output, b"asset", b"receipt", |file, bytes| {
            file.write_all(&bytes[..3])?;
            fs::write(&unrelated, b"preserve me")?;
            Err(io::Error::other("injected late receipt failure"))
        })
        .unwrap_err();
        assert!(error.to_string().contains("injected late receipt failure"));
        assert!(error.to_string().contains("cleanup failed"));
        assert!(!output.join("merged.alife-foundation").exists());
        assert!(!output.join("merge-receipt.json").exists());
        assert_eq!(fs::read(&unrelated).unwrap(), b"preserve me");
        fs::remove_file(unrelated).unwrap();
        fs::remove_dir(output).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
