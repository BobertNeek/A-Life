#![cfg(feature = "cpu-backend")]
//! Disposable native fixtures only. These tests exercise collection and recovery,
//! not a retained brain, live training run, or evidence of learned competence.

use alife_core::{BrainCapacityClass, PhenotypeCompiler, PhenotypeCompilerInputs};
use alife_training::{
    cpu::*, FoundationTrainerCheckpoint, FoundationTrainingBackendV1, PpoValueHeadCheckpoint,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

struct FixtureDirectory(PathBuf);
impl FixtureDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "alife-collect-only-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture_config(output: PathBuf, seed: u64) -> CpuContinuationConfig {
    CpuContinuationConfig {
        previous: None,
        output,
        seed,
        ticks: 6,
        temperature: 0.9,
        fixture: true,
    }
}
fn source_config(previous: PathBuf, output: PathBuf, seed: u64) -> CpuContinuationConfig {
    CpuContinuationConfig {
        previous: Some(previous),
        output,
        seed,
        ticks: 6,
        temperature: 0.9,
        fixture: false,
    }
}
fn load<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}
fn assert_no_training_output(output: &Path) {
    for name in [
        "cpu-cycle.json",
        "trained.alife-foundation",
        "training-handoff.json",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ] {
        assert!(!output.join(name).exists(), "collection published {name}");
    }
}
fn assert_zero_offline_updates(output: &Path) {
    let metadata: serde_json::Value = load(output.join("collection-metadata.json"));
    assert_eq!(
        metadata["offline_actor_optimizer_updates"].as_u64(),
        Some(0)
    );
    assert_eq!(
        metadata["offline_value_optimizer_updates"].as_u64(),
        Some(0)
    );
}
fn assignment(collection: &CpuCollectedEpisode, seed: u64) -> CpuCollectorAssignmentV1 {
    let checkpoint = collection.host_checkpoint().unwrap();
    CpuCollectorAssignmentV1 {
        collector_run_id: "synthetic-collect-only".into(),
        backend: FoundationTrainingBackendV1::NativeCpu,
        producer_revision: "synthetic-fixture-v1".into(),
        world_seed: seed,
        sampling_seed: (seed as u32) ^ ((seed >> 32) as u32),
        sampling_counter_start: 0,
        sampling_counter_end: checkpoint.sampling_counter,
        lesson: "native-default-fixture".into(),
        episode_config_sha256: [1; 32],
    }
}
fn restore_trainer(
    inputs: &PhenotypeCompilerInputs,
    actor: FoundationTrainerCheckpoint,
) -> CpuFoundationTrainer {
    let phenotype =
        PhenotypeCompiler::compile_validated(inputs, &BrainCapacityClass::n2048()).unwrap();
    CpuFoundationTrainer::from_checkpoint(
        phenotype,
        inputs.n2048_candidate_asset().unwrap().clone(),
        actor,
    )
    .unwrap()
}

#[test]
fn fixture_collection_roundtrips_private_recovery_and_admits_without_optimizer_updates() {
    let root = FixtureDirectory::new();
    let seed = 0x62_1101;
    let output = root.join("collection");
    let collection =
        collect_cpu_episode(fixture_config(output.clone(), seed), &mut DefaultCpuEpisode).unwrap();
    let actor = collection.actor_checkpoint();
    let critic = collection.value_checkpoint();
    assert_eq!(actor.optimizer_step, 0);
    assert_eq!(critic.optimizer_step, 0);
    assert!(actor.update_ages.iter().all(|age| *age == 0));
    assert!(actor
        .first_moment
        .iter()
        .chain(&actor.second_moment)
        .all(|moment| *moment == 0.));
    assert!(critic.parameters.iter().all(|parameter| *parameter == 0.));
    assert_eq!(actor, load(output.join("collection-actor-checkpoint.json")));
    assert_eq!(critic, load(output.join("initial-value-checkpoint.json")));
    assert_zero_offline_updates(&output);
    assert_no_training_output(&output);

    let checkpoint = collection.host_checkpoint().unwrap();
    assert!(checkpoint.sampling_counter >= 2);
    assert!(checkpoint.residents.iter().any(|resident| resident
        .neural
        .eligibility
        .iter()
        .any(|value| *value != 0.)));
    let inputs = collection.compiler_inputs().clone();
    let trainer = restore_trainer(&inputs, actor.clone());
    let value = CpuValueHead::from_checkpoint(critic.clone()).unwrap();
    let restored = CpuCanonicalHost::from_checkpoint(checkpoint.clone(), &trainer).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap(),
        serde_json::to_vec(&checkpoint).unwrap()
    );
    let private_bytes = fs::read(output.join("collector-checkpoint.json")).unwrap();
    assert_eq!(private_bytes, serde_json::to_vec(&checkpoint).unwrap());
    let config = collection.config();
    let version = collection.policy_version();
    let planned = assignment(&collection, seed);
    let _commit = collection.seal(planned.clone()).unwrap();
    let shard = read_cpu_collector_output(&output).unwrap();
    assert_eq!(shard.assignment, planned);
    assert_eq!(
        fs::read(output.join("collector-checkpoint.json")).unwrap(),
        private_bytes
    );
    let admitted = admit_cpu_collector_shards(
        &trainer,
        &value,
        &inputs,
        config,
        version,
        std::slice::from_ref(&planned),
        vec![shard],
        3,
        2,
    )
    .unwrap();
    assert!(!admitted.windows().is_empty());
    assert_eq!(trainer.checkpoint(), actor);
    assert_eq!(value.checkpoint(), critic);
    assert_no_training_output(&output);
    let sealed_before = snapshot(&output);
    let implicit_continuation = root.join("must-not-promote-collection");
    assert!(run_cpu_continuation(
        source_config(output.clone(), implicit_continuation.clone(), seed + 1),
        &mut DefaultCpuEpisode,
    )
    .is_err());
    assert!(!implicit_continuation.exists());
    assert_eq!(snapshot(&output), sealed_before);
}

#[test]
fn explicit_native_source_retains_full_nonzero_optimizer_history_and_source_bytes() {
    let root = FixtureDirectory::new();
    let source = root.join("synthetic-source");
    run_cpu_continuation(
        fixture_config(source.clone(), 0x62_1201),
        &mut DefaultCpuEpisode,
    )
    .unwrap();
    let actor: FoundationTrainerCheckpoint = load(source.join("actor-checkpoint.json"));
    let critic: PpoValueHeadCheckpoint = load(source.join("value-checkpoint.json"));
    assert!(actor.optimizer_step > 0 && critic.optimizer_step > 0);
    assert!(actor.update_ages.iter().any(|age| *age > 0));
    assert!(actor.first_moment.iter().any(|moment| *moment != 0.));
    assert!(critic.parameters[critic.feature_count as usize + 1..]
        .iter()
        .any(|moment| *moment != 0.));
    let source_before = snapshot(&source);
    let output = root.join("collected-from-explicit-source");
    let seed = 0x62_1202;
    let collection = collect_cpu_episode(
        source_config(source.clone(), output.clone(), seed),
        &mut DefaultCpuEpisode,
    )
    .unwrap();
    assert_eq!(collection.actor_checkpoint(), actor);
    assert_eq!(collection.value_checkpoint(), critic);
    assert_zero_offline_updates(&output);
    assert_eq!(
        fs::read(output.join("collection-actor-checkpoint.json")).unwrap(),
        source_before[Path::new("actor-checkpoint.json")]
    );
    assert_eq!(
        fs::read(output.join("initial-value-checkpoint.json")).unwrap(),
        source_before[Path::new("value-checkpoint.json")]
    );
    let planned = assignment(&collection, seed);
    collection.seal(planned).unwrap();
    read_cpu_collector_output(&output).unwrap();
    assert_eq!(snapshot(&source), source_before);
    assert_no_training_output(&output);
}

#[test]
fn missing_newly_retained_parent_rejects_seal_and_keeps_private_recovery_and_source() {
    let root = FixtureDirectory::new();
    let source = root.join("synthetic-source");
    run_cpu_continuation(
        fixture_config(source.clone(), 0x62_1601),
        &mut DefaultCpuEpisode,
    )
    .unwrap();
    assert!(!source.join("source-lineage").exists());
    let source_before = snapshot(&source);
    let output = root.join("missing-direct-parent");
    let seed = 0x62_1602;
    let collection = collect_cpu_episode(
        source_config(source.clone(), output.clone(), seed),
        &mut DefaultCpuEpisode,
    )
    .unwrap();
    let planned = assignment(&collection, seed);
    let lineage_files = fs::read_dir(output.join("source-lineage"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let pack = lineage_files
        .iter()
        .find(|p| p.extension().is_some_and(|e| e == "pack"))
        .unwrap();
    fs::remove_file(pack).unwrap();
    let private_bytes = fs::read(output.join("collector-checkpoint.json")).unwrap();
    assert!(collection.seal(planned).is_err());
    assert!(!output.join("collector-commit.json").exists());
    assert!(!output.join("collector-shard.json").exists());
    assert_eq!(
        fs::read(output.join("collector-checkpoint.json")).unwrap(),
        private_bytes
    );
    assert_eq!(snapshot(&source), source_before);
    assert_no_training_output(&output);
}

#[test]
fn tampered_source_is_rejected_before_collection_output_ownership() {
    let root = FixtureDirectory::new();
    let source = root.join("synthetic-source");
    run_cpu_continuation(
        fixture_config(source.clone(), 0x62_1301),
        &mut DefaultCpuEpisode,
    )
    .unwrap();
    let path = source.join("value-checkpoint.json");
    let mut critic: PpoValueHeadCheckpoint = load(&path);
    critic.parameters[0] += 0.125;
    fs::write(path, serde_json::to_vec(&critic).unwrap()).unwrap();
    let damaged_before = snapshot(&source);
    let output = root.join("must-not-exist");
    assert!(collect_cpu_episode(
        source_config(source.clone(), output.clone(), 0x62_1302),
        &mut DefaultCpuEpisode
    )
    .is_err());
    assert!(!output.exists());
    assert_eq!(snapshot(&source), damaged_before);
}

struct FailAfterCollection;
impl CpuEpisodeHooks for FailAfterCollection {
    fn finish_collection(
        &mut self,
        _host: &CpuCanonicalHost,
        founder: &CpuFoundationTrainer,
        rows: &BTreeMap<u64, Vec<CpuCollectedDecision>>,
        train_rows: usize,
        output: &Path,
    ) -> CpuContinuationResult<CpuEpisodeReceipt> {
        assert!(train_rows > 0 && !rows.is_empty());
        assert_eq!(founder.optimizer_step(), 0);
        fs::write(
            output.join("unfinished-episode.json"),
            b"{\"synthetic\":true}",
        )?;
        Err("synthetic interruption after private checkpoint capture".into())
    }
}

struct FailBeforeThirdStep;
impl CpuEpisodeHooks for FailBeforeThirdStep {
    fn preserve_collection_recovery(
        &mut self,
        host: &CpuCanonicalHost,
        _founder: &CpuFoundationTrainer,
        output: &Path,
    ) -> CpuContinuationResult<()> {
        fs::write(
            output.join("synthetic-private-recovery.json"),
            serde_json::to_vec(&host.world().tick())?,
        )?;
        Ok(())
    }
    fn before_step(
        &mut self,
        host: &mut CpuCanonicalHost,
        _founder: &CpuFoundationTrainer,
    ) -> CpuContinuationResult<()> {
        if host.world().tick().raw() >= 2 {
            return Err("synthetic early collection interruption".into());
        }
        Ok(())
    }
}

#[test]
fn hook_failure_inside_collection_retains_the_last_sealed_private_host() {
    let root = FixtureDirectory::new();
    let output = root.join("interrupted-during-collection");
    let result = collect_cpu_episode(
        fixture_config(output.clone(), 0x62_1501),
        &mut FailBeforeThirdStep,
    );
    let error = match result {
        Ok(_) => panic!("collection unexpectedly completed after its failing hook"),
        Err(error) => error,
    };
    assert_eq!(error.to_string(), "synthetic early collection interruption");
    let checkpoint: CpuCanonicalCheckpoint = load(output.join("collector-checkpoint.json"));
    let private_tick: serde_json::Value = load(output.join("synthetic-private-recovery.json"));
    assert!(checkpoint.sampling_counter > 0);
    assert!(checkpoint.residents.iter().any(|resident| resident
        .neural
        .eligibility
        .iter()
        .any(|value| *value != 0.)));
    let actor: FoundationTrainerCheckpoint = load(output.join("collection-actor-checkpoint.json"));
    let critic: PpoValueHeadCheckpoint = load(output.join("initial-value-checkpoint.json"));
    assert_eq!(actor.optimizer_step, 0);
    assert_eq!(critic.optimizer_step, 0);
    let inputs: PhenotypeCompilerInputs = load(output.join("collection-compiler-inputs.json"));
    let trainer = restore_trainer(&inputs, actor);
    let restored = CpuCanonicalHost::from_checkpoint(checkpoint.clone(), &trainer).unwrap();
    assert!(restored.world().tick().raw() >= 2);
    assert_eq!(
        private_tick,
        serde_json::to_value(restored.world().tick()).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap(),
        serde_json::to_vec(&checkpoint).unwrap()
    );
    assert!(!output.join("collector-shard.json").exists());
    assert!(!output.join("collector-commit.json").exists());
    assert_no_training_output(&output);
}

#[test]
fn finalization_failure_and_preseal_archive_change_preserve_collected_recovery() {
    let root = FixtureDirectory::new();
    let interrupted = root.join("interrupted");
    assert!(collect_cpu_episode(
        fixture_config(interrupted.clone(), 0x62_1401),
        &mut FailAfterCollection
    )
    .is_err());
    let inputs: PhenotypeCompilerInputs = load(interrupted.join("collection-compiler-inputs.json"));
    let actor: FoundationTrainerCheckpoint =
        load(interrupted.join("collection-actor-checkpoint.json"));
    let critic: PpoValueHeadCheckpoint = load(interrupted.join("initial-value-checkpoint.json"));
    assert_eq!(actor.optimizer_step, 0);
    assert_eq!(critic.optimizer_step, 0);
    let trainer = restore_trainer(&inputs, actor);
    let checkpoint: CpuCanonicalCheckpoint = load(interrupted.join("collector-checkpoint.json"));
    assert!(checkpoint.sampling_counter > 0);
    let restored = CpuCanonicalHost::from_checkpoint(checkpoint.clone(), &trainer).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap(),
        serde_json::to_vec(&checkpoint).unwrap()
    );
    assert!(interrupted.join("unfinished-episode.json").exists());
    assert!(!interrupted.join("collector-commit.json").exists());
    assert_no_training_output(&interrupted);

    let output = root.join("changed-before-seal");
    let seed = 0x62_1402;
    let collection =
        collect_cpu_episode(fixture_config(output.clone(), seed), &mut DefaultCpuEpisode).unwrap();
    let planned = assignment(&collection, seed);
    let path = output.join("collector-checkpoint.json");
    let mut changed = fs::read(&path).unwrap();
    changed.push(b'\n');
    fs::write(path, changed).unwrap();
    let before = snapshot(&output);
    assert!(collection.seal(planned).is_err());
    assert_eq!(snapshot(&output), before);
    assert!(!output.join("collector-commit.json").exists());
    assert_no_training_output(&output);
}

#[test]
fn legacy_flat_ancestry_seals_converted_source_copy_without_mutating_predecessor() {
    let root = FixtureDirectory::new();
    let initial = root.join("initial");
    let source = root.join("legacy-flat-source");
    run_cpu_continuation(
        fixture_config(initial.clone(), 0x62_1701),
        &mut DefaultCpuEpisode,
    )
    .unwrap();
    run_cpu_continuation(
        source_config(initial, source.clone(), 0x62_1702),
        &mut DefaultCpuEpisode,
    )
    .unwrap();
    let receipt = alife_training::native_receipt::read(&source.join("cpu-cycle.json"))
        .unwrap()
        .1;
    let lineage = alife_training::native_receipt::Lineage::load(&receipt, &source).unwrap();
    let flat = root.join("flat-export");
    fs::create_dir(&flat).unwrap();
    lineage.write_to(&flat).unwrap();
    fs::rename(
        source.join("source-lineage"),
        root.join("original-packed-archive"),
    )
    .unwrap();
    fs::rename(flat.join("source-lineage"), source.join("source-lineage")).unwrap();
    read_cpu_founder_source(&source).unwrap();
    let source_before = snapshot(&source);
    for corrupt in [false, true] {
        let output = root.join(if corrupt {
            "corrupt-source-copy"
        } else {
            "valid-source-copy"
        });
        let seed = if corrupt { 0x62_1704 } else { 0x62_1703 };
        let collection = collect_cpu_episode(
            source_config(source.clone(), output.clone(), seed),
            &mut DefaultCpuEpisode,
        )
        .unwrap();
        let planned = assignment(&collection, seed);
        assert!(output
            .join("source-native/source-lineage/manifest.json")
            .is_file());
        let private = fs::read(output.join("collector-checkpoint.json")).unwrap();
        if corrupt {
            let pack = fs::read_dir(output.join("source-native/source-lineage"))
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| p.extension().is_some_and(|e| e == "pack"))
                .unwrap();
            let mut raw = fs::read(&pack).unwrap();
            let n = raw.len() / 2;
            raw[n] ^= 1;
            fs::write(pack, raw).unwrap();
            assert!(collection.seal(planned).is_err());
            assert!(!output.join("collector-commit.json").exists());
        } else {
            collection.seal(planned).unwrap();
            read_cpu_collector_output(&output).unwrap();
            read_cpu_founder_source(&output.join("source-native")).unwrap();
        }
        assert_eq!(
            fs::read(output.join("collector-checkpoint.json")).unwrap(),
            private
        );
        assert_eq!(snapshot(&source), source_before);
    }
}
