#![cfg(feature = "cpu-backend")]
//! Disposable synthetic native hosts prove coordinator boundaries and optimizer
//! continuity. They do not measure learned competence or cooperative synergy.

use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, *};
use alife_world::*;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const VERSION: u64 = 7400;
const WORLD_SEED: u64 = 0x7A_1123;

struct FixtureDirectory(PathBuf);
impl FixtureDirectory {
    fn new() -> Self {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "alife-cooperative-round-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
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

fn config() -> PpoConfig {
    PpoConfig {
        temperature: 0.9,
        epochs: 1,
        ..Default::default()
    }
}
fn write_json(path: impl AsRef<Path>, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn load<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn file_hash(path: impl AsRef<Path>) -> [u8; 32] {
    Sha256::digest(fs::read(path).unwrap()).into()
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
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

fn fixture_source() -> FoundationTrainingHandoffV1 {
    let builtin =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let game = create_canonical_new_game_with_n2048_candidate(&game_config(), &builtin).unwrap();
    let record = game.world.organism_registry().iter().next().unwrap();
    let genome = record.phenotype().brain_genome.clone();
    let capacity = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        &genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let base = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_trained_weights(
        &base,
        base.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())
            .unwrap();
    // Keep one frozen coordinate with its own retained history as well as a
    // complete nonzero trainable Adam history. No checkpoint is loaded privately.
    let count = phenotype.synapses().len();
    let mut trainer = CpuFoundationTrainer::new(
        phenotype.clone(),
        asset,
        StageTrainableMask::from_synapse_indices(
            &phenotype,
            &(0..count as u32 - 1).collect::<Vec<_>>(),
        )
        .unwrap(),
        AdamWConfig::default(),
    )
    .unwrap();
    let mut actor = trainer.checkpoint();
    actor.optimizer_step = 16_684;
    actor.update_ages = (0..count)
        .map(|index| 16_684 - (index % 3) as u32)
        .collect();
    actor.first_moment.fill(0.001);
    actor.second_moment.fill(0.002);
    trainer.restore_checkpoint(actor.clone()).unwrap();
    let n = phenotype.neuron_count() as usize + 1;
    let mut parameters = vec![0.; 3 * n];
    parameters[..n].fill(0.001);
    parameters[n..2 * n].fill(0.002);
    parameters[2 * n..].fill(0.003);
    FoundationTrainingHandoffV1::new(
        FoundationTrainingBackendV1::NativeCpu,
        VERSION - 1,
        inputs,
        actor,
        PpoValueHeadCheckpoint {
            feature_count: phenotype.neuron_count(),
            optimizer_step: 19_800,
            parameters,
            last_updated_policy_version: Some(VERSION - 1),
        },
    )
    .unwrap()
}
fn game_config() -> CanonicalNewGameConfig {
    let mut config = CanonicalNewGameConfig::phase3(WORLD_SEED, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config
}
fn learner(source: &FoundationTrainingHandoffV1) -> (CpuFoundationTrainer, CpuValueHead) {
    let admitted = source.clone().admit().unwrap();
    (
        CpuFoundationTrainer::from_checkpoint(
            admitted.phenotype,
            admitted.foundation,
            admitted.actor,
        )
        .unwrap(),
        CpuValueHead::from_checkpoint(admitted.value).unwrap(),
    )
}

struct RoundFixture {
    root: FixtureDirectory,
    source_path: PathBuf,
    source: FoundationTrainingHandoffV1,
    snapshot: CpuCollectionSnapshotV1,
    collectors: Vec<CpuCooperativeRoundInput>,
    members: Vec<CpuCooperativeRoundMemberV1>,
}
impl RoundFixture {
    fn new() -> Self {
        let root = FixtureDirectory::new();
        let source_path = root.join("explicit-synthetic-source.json");
        let source = fixture_source();
        write_json(&source_path, &source);
        let (trainer, value) = learner(&source);
        let snapshot = CpuCollectionSnapshotV1::capture(
            &trainer,
            &value,
            &source.compiler_inputs,
            config(),
            VERSION,
        )
        .unwrap();
        let mut collectors = Vec::new();
        let mut members = Vec::new();
        for (run, seed) in [("a-complementary", 42), ("b-integration", 43)] {
            let directory = root.join(run);
            fs::create_dir(&directory).unwrap();
            let game = create_canonical_new_game_with_n2048_candidate(
                &game_config(),
                source.compiler_inputs.n2048_candidate_asset().unwrap(),
            )
            .unwrap();
            let mut host = CpuCanonicalHost::new_cohort(game, &trainer, seed).unwrap();
            let mut decisions = Vec::new();
            for _ in 0..4 {
                decisions.extend(host.step(&trainer, config().temperature).unwrap().decisions);
            }
            assert_eq!(decisions.len(), 4);
            let checkpoint = host.checkpoint(&trainer).unwrap();
            let record = host
                .world()
                .organism_registry()
                .get(decisions[0].organism_id)
                .unwrap()
                .clone();
            let shard = CpuCollectorShardV1::seal(
                snapshot.clone(),
                CpuCollectorAssignmentV1 {
                    collector_run_id: run.into(),
                    backend: FoundationTrainingBackendV1::NativeCpu,
                    producer_revision: "synthetic-coordinator-fixture-v1".into(),
                    world_seed: WORLD_SEED,
                    sampling_seed: seed,
                    sampling_counter_start: 0,
                    sampling_counter_end: checkpoint.sampling_counter,
                    lesson: format!("synthetic-{run}"),
                    episode_config_sha256: [1; 32],
                },
                vec![CpuCollectorTrajectoryV1 {
                    final_record: record,
                    decisions,
                }],
            )
            .unwrap();
            fs::write(
                directory.join("collection-policy.alife-foundation"),
                source
                    .compiler_inputs
                    .n2048_candidate_asset()
                    .unwrap()
                    .encode_canonical()
                    .unwrap(),
            )
            .unwrap();
            write_json(
                directory.join("collection-compiler-inputs.json"),
                &source.compiler_inputs,
            );
            write_json(
                directory.join("collection-actor-checkpoint.json"),
                &source.actor,
            );
            write_json(
                directory.join("initial-value-checkpoint.json"),
                &source.value,
            );
            write_json(directory.join("collector-checkpoint.json"), &checkpoint);
            write_cpu_collector_output(&directory, &shard).unwrap();
            members.push(CpuCooperativeRoundMemberV1 {
                assignment: shard.assignment,
                collector_commit_sha256: file_hash(directory.join(CPU_COLLECTOR_COMMIT_FILE)),
            });
            collectors.push(CpuCooperativeRoundInput {
                collector_run_id: run.into(),
                directory,
            });
        }
        Self {
            root,
            source_path,
            source,
            snapshot,
            collectors,
            members,
        }
    }
    fn plan(&self, round_id: &str) -> CpuCooperativeRoundPlanV1 {
        self.plan_with(round_id, self.snapshot.clone(), self.members.clone())
    }
    fn plan_with(
        &self,
        round_id: &str,
        snapshot: CpuCollectionSnapshotV1,
        members: Vec<CpuCooperativeRoundMemberV1>,
    ) -> CpuCooperativeRoundPlanV1 {
        CpuCooperativeRoundPlanV1::seal(
            round_id.into(),
            file_hash(&self.source_path),
            snapshot,
            config(),
            2,
            1,
            4,
            members,
        )
        .unwrap()
    }
    fn protected_bytes(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut bytes = BTreeMap::new();
        bytes.insert(
            self.source_path.clone(),
            fs::read(&self.source_path).unwrap(),
        );
        for collector in &self.collectors {
            for (path, data) in tree(&collector.directory) {
                bytes.insert(collector.directory.join(path), data);
            }
        }
        bytes
    }
    fn assert_protected(&self, expected: &BTreeMap<PathBuf, Vec<u8>>) {
        assert_eq!(&self.protected_bytes(), expected);
    }
    fn reject(
        &self,
        name: &str,
        plan: &CpuCooperativeRoundPlanV1,
        collectors: &[CpuCooperativeRoundInput],
    ) {
        let before = self.protected_bytes();
        let output = self.root.join(name);
        assert!(
            run_cpu_cooperative_round(&self.source_path, plan, collectors, &output).is_err(),
            "invalid evidence admitted: {name}"
        );
        assert!(!output.exists(), "invalid evidence owned output: {name}");
        self.assert_protected(&before);
    }
}

#[test]
fn two_collectors_update_one_history_and_retry_preserves_the_completed_round() {
    let fixture = RoundFixture::new();
    let plan = fixture.plan("synthetic-two-collector-proof");
    let protected = fixture.protected_bytes();
    let shards = fixture
        .collectors
        .iter()
        .map(|input| read_cpu_collector_output(&input.directory).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        shards[0].trajectories[0].final_record.organism_id(),
        shards[1].trajectories[0].final_record.organism_id()
    );
    let (mut expected_actor, mut expected_value) = learner(&fixture.source);
    let cohort = admit_cpu_collector_shards(
        &expected_actor,
        &expected_value,
        &fixture.source.compiler_inputs,
        config(),
        VERSION,
        &fixture
            .members
            .iter()
            .map(|member| member.assignment.clone())
            .collect::<Vec<_>>(),
        shards,
        2,
        1,
    )
    .unwrap();
    let expected_ids = cohort.window_trajectories().to_vec();
    let expected_update = cohort
        .train(
            &mut expected_actor,
            &mut expected_value,
            &fixture.source.compiler_inputs,
            config(),
            4,
        )
        .unwrap();
    let before_rebind = expected_actor.checkpoint();
    let output = fixture.root.join("central-result");
    let result =
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output)
            .unwrap();
    assert_eq!(result.plan, plan);
    assert_eq!(result.source_handoff, fixture.source);
    assert_eq!(result.update, expected_update);
    assert_eq!(result.windows, 4);
    assert_eq!(result.training_rows, 6);
    assert_eq!(result.window_trajectories, expected_ids);
    assert_eq!(
        result.window_trajectories[0].collector_run_id,
        "a-complementary"
    );
    assert_eq!(
        result.window_trajectories[2].collector_run_id,
        "b-integration"
    );
    assert_eq!(result.output_handoff.policy_version, VERSION);
    assert_eq!(
        result.output_handoff.origin,
        FoundationTrainingBackendV1::NativeCpu
    );
    result.output_handoff.validate().unwrap();
    assert_eq!(result.output_handoff.actor.optimizer_step, 16_685);
    assert_eq!(result.output_handoff.value.optimizer_step, 19_801);
    assert_eq!(result.output_handoff.value, expected_value.checkpoint());
    let next_inputs = &result.output_handoff.compiler_inputs;
    let next =
        PhenotypeCompiler::compile_validated(next_inputs, &BrainCapacityClass::n2048()).unwrap();
    expected_actor
        .rebind_for_next_cohort(next, next_inputs.n2048_candidate_asset().unwrap().clone())
        .unwrap();
    assert_eq!(result.output_handoff.actor, expected_actor.checkpoint());
    assert_eq!(
        result.output_handoff.actor.first_moment,
        before_rebind.first_moment
    );
    assert_eq!(
        result.output_handoff.actor.second_moment,
        before_rebind.second_moment
    );
    assert_eq!(
        result.output_handoff.actor.update_ages,
        before_rebind.update_ages
    );
    assert_eq!(
        result.output_handoff.actor.stage_mask,
        fixture.source.actor.stage_mask
    );
    let frozen = fixture.source.actor.weights.len() - 1;
    assert_eq!(
        result.output_handoff.actor.weights[frozen],
        fixture.source.actor.weights[frozen]
    );
    assert_eq!(
        result.output_handoff.actor.update_ages[frozen],
        fixture.source.actor.update_ages[frozen]
    );
    assert_ne!(
        result.output_handoff.actor.first_moment,
        fixture.source.actor.first_moment
    );
    assert_ne!(
        result.output_handoff.value.parameters,
        fixture.source.value.parameters
    );
    let completed = tree(&output);
    let read = read_cpu_cooperative_round(&output).unwrap();
    let retried =
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output)
            .unwrap();
    assert_eq!(read.payload_sha256, result.payload_sha256);
    assert_eq!(retried.output_handoff, result.output_handoff);
    assert_eq!(retried.update, result.update);
    assert_eq!(tree(&output), completed);
    fixture.assert_protected(&protected);
}

#[test]
fn canonical_plan_and_shuffled_directory_mapping_produce_identical_results() {
    let fixture = RoundFixture::new();
    let plan = fixture.plan("canonical-order");
    let mut members = fixture.members.clone();
    members.reverse();
    let reordered_plan = fixture.plan_with("canonical-order", fixture.snapshot.clone(), members);
    assert_eq!(plan, reordered_plan);
    let mut inputs = fixture.collectors.clone();
    inputs.reverse();
    let protected = fixture.protected_bytes();
    let first_output = fixture.root.join("canonical-first");
    let shuffled_output = fixture.root.join("canonical-shuffled");
    let first = run_cpu_cooperative_round(
        &fixture.source_path,
        &plan,
        &fixture.collectors,
        &first_output,
    )
    .unwrap();
    let shuffled = run_cpu_cooperative_round(
        &fixture.source_path,
        &reordered_plan,
        &inputs,
        &shuffled_output,
    )
    .unwrap();
    assert_eq!(first.payload_sha256, shuffled.payload_sha256);
    assert_eq!(first.output_handoff, shuffled.output_handoff);
    assert_eq!(first.window_trajectories, shuffled.window_trajectories);
    assert_eq!(tree(&first_output), tree(&shuffled_output));
    fixture.assert_protected(&protected);
}

#[test]
fn incomplete_duplicate_stale_and_mismatched_evidence_reject_before_output_ownership() {
    let fixture = RoundFixture::new();
    let plan = fixture.plan("rejection-boundaries");
    fixture.reject("missing-member", &plan, &fixture.collectors[..1]);
    fixture.reject(
        "duplicate-member",
        &plan,
        &[fixture.collectors[0].clone(), fixture.collectors[0].clone()],
    );
    let mut swapped = fixture.collectors.clone();
    let other = swapped[1].directory.clone();
    swapped[1].directory = swapped[0].directory.clone();
    swapped[0].directory = other;
    fixture.reject("wrong-directory-mapping", &plan, &swapped);
    let mut wrong_members = fixture.members.clone();
    wrong_members[0].assignment.lesson = "unapproved-lesson".into();
    fixture.reject(
        "wrong-assignment",
        &fixture.plan_with("wrong-assignment", fixture.snapshot.clone(), wrong_members),
        &fixture.collectors,
    );
    let mut stale = fixture.snapshot.clone();
    stale.policy_version += 1;
    fixture.reject(
        "stale-snapshot",
        &fixture.plan_with("stale-snapshot", stale, fixture.members.clone()),
        &fixture.collectors,
    );
    let mut broken_seal = plan.clone();
    broken_seal.payload_sha256[0] ^= 1;
    fixture.reject("tampered-plan", &broken_seal, &fixture.collectors);

    let source_bytes = fs::read(&fixture.source_path).unwrap();
    let mut changed_source = fixture.source.clone();
    changed_source.actor.first_moment[0] += 0.01;
    write_json(&fixture.source_path, &changed_source);
    fixture.reject("changed-source-history", &plan, &fixture.collectors);
    fs::write(&fixture.source_path, source_bytes).unwrap();

    let manifest_path = fixture.collectors[0]
        .directory
        .join(CPU_COLLECTOR_COMMIT_FILE);
    let original_manifest = fs::read(&manifest_path).unwrap();
    let mut changed_manifest = original_manifest.clone();
    changed_manifest.push(b'\n');
    fs::write(&manifest_path, changed_manifest).unwrap();
    fixture.reject("changed-raw-manifest", &plan, &fixture.collectors);
    fs::write(&manifest_path, original_manifest).unwrap();

    let recovery_path = fixture.collectors[0]
        .directory
        .join("collector-checkpoint.json");
    let original_recovery = fs::read(&recovery_path).unwrap();
    let mut changed_recovery = original_recovery.clone();
    changed_recovery.push(b'\n');
    fs::write(&recovery_path, changed_recovery).unwrap();
    fixture.reject("changed-private-recovery", &plan, &fixture.collectors);
    fs::write(&recovery_path, original_recovery).unwrap();
}

#[test]
fn well_sealed_wrong_old_probability_rejects_before_creating_the_output() {
    let fixture = RoundFixture::new();
    let original = &fixture.collectors[0];
    let mut shard = read_cpu_collector_output(&original.directory).unwrap();
    shard.trajectories[0].decisions[0]
        .action
        .old_joint_log_probability -= 0.1;
    let shard =
        CpuCollectorShardV1::seal(shard.snapshot, shard.assignment, shard.trajectories).unwrap();
    let directory = fixture.root.join("resealed-wrong-probability");
    fs::create_dir(&directory).unwrap();
    for name in [
        "collection-policy.alife-foundation",
        "collection-compiler-inputs.json",
        "collection-actor-checkpoint.json",
        "initial-value-checkpoint.json",
        "collector-checkpoint.json",
    ] {
        fs::copy(original.directory.join(name), directory.join(name)).unwrap();
    }
    write_cpu_collector_output(&directory, &shard).unwrap();
    let mut members = fixture.members.clone();
    members[0].collector_commit_sha256 = file_hash(directory.join(CPU_COLLECTOR_COMMIT_FILE));
    let plan = fixture.plan_with(
        "resealed-wrong-probability",
        fixture.snapshot.clone(),
        members,
    );
    let mut inputs = fixture.collectors.clone();
    inputs[0].directory = directory.clone();
    let before = tree(&directory);
    fixture.reject("invalid-probability-output", &plan, &inputs);
    assert_eq!(tree(&directory), before);
}

#[test]
fn completed_output_rejects_a_different_plan_or_tampered_recovery() {
    let fixture = RoundFixture::new();
    let plan = fixture.plan("completed-plan");
    let output = fixture.root.join("completed-output");
    run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output).unwrap();
    let protected = fixture.protected_bytes();
    let completed = tree(&output);
    let different = fixture.plan("different-plan");
    assert!(run_cpu_cooperative_round(
        &fixture.source_path,
        &different,
        &fixture.collectors,
        &output,
    )
    .is_err());
    assert_eq!(tree(&output), completed);
    let result_path = output.join(CPU_COOPERATIVE_ROUND_RESULT_FILE);
    let mut result: serde_json::Value = load(&result_path);
    result["source_handoff"]["actor"]["first_moment"][0] = serde_json::json!(0.25);
    write_json(&result_path, &result);
    let damaged = tree(&output);
    assert!(read_cpu_cooperative_round(&output).is_err());
    assert!(
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output,)
            .is_err()
    );
    assert_eq!(tree(&output), damaged);
    fixture.assert_protected(&protected);
}

fn stage_name(plan: &CpuCooperativeRoundPlanV1, suffix: &str) -> String {
    let digest = plan
        .payload_sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("round-{digest}-{suffix}.pending")
}

// Deliberately repair transport integrity after synthetic corruption: a valid
// content hash must not bypass the reader's optimizer-continuity checks.
fn reseal_result(result: &mut CpuCooperativeRoundResultV1) {
    let mut digest = Sha256::new();
    digest.update(b"ALIFE-CPU-COOPERATIVE-ROUND-RESULT-V1");
    serde_json::to_writer(
        &mut digest,
        &(
            result.schema_version,
            &result.mode,
            &result.plan,
            &result.source_handoff,
            &result.output_handoff,
            &result.update,
            result.windows,
            result.training_rows,
            &result.window_trajectories,
        ),
    )
    .unwrap();
    result.payload_sha256 = digest.finalize().into();
}

#[test]
fn complete_stage_is_recovered_exactly_and_torn_stage_retains_its_prefix() {
    let fixture = RoundFixture::new();
    let plan = fixture.plan("recoverable-round");
    let output = fixture.root.join("recover-complete-stage");
    let result =
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output)
            .unwrap();
    let canonical_path = output.join(CPU_COOPERATIVE_ROUND_RESULT_FILE);
    let complete_bytes = fs::read(&canonical_path).unwrap();
    fs::rename(&canonical_path, output.join(stage_name(&plan, "test"))).unwrap();
    let protected = fixture.protected_bytes();
    let recovered =
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output)
            .unwrap();
    assert_eq!(recovered.payload_sha256, result.payload_sha256);
    assert_eq!(recovered.output_handoff, result.output_handoff);
    assert_eq!(recovered.update, result.update);
    assert_eq!(fs::read(&canonical_path).unwrap(), complete_bytes);
    read_cpu_cooperative_round(&output).unwrap();

    let torn_output = fixture.root.join("recover-torn-stage");
    fs::create_dir(&torn_output).unwrap();
    let torn_stage = torn_output.join(stage_name(&plan, "interrupted"));
    let prefix = &complete_bytes[..complete_bytes.len() / 2];
    fs::write(&torn_stage, prefix).unwrap();
    let recomputed = run_cpu_cooperative_round(
        &fixture.source_path,
        &plan,
        &fixture.collectors,
        &torn_output,
    )
    .unwrap();
    assert_eq!(recomputed.payload_sha256, result.payload_sha256);
    assert_eq!(recomputed.output_handoff, result.output_handoff);
    assert_eq!(fs::read(torn_stage).unwrap(), prefix);
    read_cpu_cooperative_round(&torn_output).unwrap();
    fixture.assert_protected(&protected);
}

#[test]
fn a_complete_stage_for_a_different_plan_is_preserved_and_rejected() {
    let fixture = RoundFixture::new();
    let original_plan = fixture.plan("original-stage-plan");
    let output = fixture.root.join("wrong-plan-stage");
    run_cpu_cooperative_round(
        &fixture.source_path,
        &original_plan,
        &fixture.collectors,
        &output,
    )
    .unwrap();
    fs::rename(
        output.join(CPU_COOPERATIVE_ROUND_RESULT_FILE),
        output.join(stage_name(&original_plan, "other-plan")),
    )
    .unwrap();
    let before = tree(&output);
    let protected = fixture.protected_bytes();
    let different_plan = fixture.plan("requested-new-plan");
    assert!(run_cpu_cooperative_round(
        &fixture.source_path,
        &different_plan,
        &fixture.collectors,
        &output,
    )
    .is_err());
    assert_eq!(tree(&output), before);
    assert!(!output.join(CPU_COOPERATIVE_ROUND_RESULT_FILE).exists());
    fixture.assert_protected(&protected);
}

#[test]
fn different_valid_complete_same_plan_stages_are_ambiguous_and_never_overwritten() {
    let fixture = RoundFixture::new();
    let plan = fixture.plan("ambiguous-complete-stages");
    let output = fixture.root.join("ambiguous-stages");
    let original =
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output)
            .unwrap();
    let mut other = original.clone();
    // Both are structurally valid checkpoints. A checksum does not authenticate
    // which locally produced complete result was intended, so neither can win.
    other.output_handoff.actor.first_moment[0] += 0.00001;
    reseal_result(&mut other);
    assert_ne!(other.payload_sha256, original.payload_sha256);
    let verifier = fixture.root.join("second-result-validation");
    fs::create_dir(&verifier).unwrap();
    write_json(verifier.join(CPU_COOPERATIVE_ROUND_RESULT_FILE), &other);
    read_cpu_cooperative_round(&verifier).unwrap();
    fs::rename(
        output.join(CPU_COOPERATIVE_ROUND_RESULT_FILE),
        output.join(stage_name(&plan, "first")),
    )
    .unwrap();
    write_json(output.join(stage_name(&plan, "second")), &other);
    let before = tree(&output);
    let protected = fixture.protected_bytes();
    assert!(
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output,)
            .is_err()
    );
    assert_eq!(tree(&output), before);
    assert!(!output.join(CPU_COOPERATIVE_ROUND_RESULT_FILE).exists());
    fixture.assert_protected(&protected);
}

#[test]
fn recomputed_result_seals_cannot_authorize_age_frozen_history_or_compiler_changes() {
    let fixture = RoundFixture::new();
    let plan = fixture.plan("semantic-result-validation");
    let output = fixture.root.join("resealed-invalid-result");
    let original =
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output)
            .unwrap();
    let protected = fixture.protected_bytes();
    for corruption in [
        "trainable-age-reset",
        "frozen-moment-change",
        "inherited-parent-change",
    ] {
        let mut changed = original.clone();
        match corruption {
            "trainable-age-reset" => {
                changed.output_handoff.actor.update_ages[0] = fixture.source.actor.update_ages[0];
            }
            "frozen-moment-change" => {
                *changed
                    .output_handoff
                    .actor
                    .first_moment
                    .last_mut()
                    .unwrap() += 0.001;
            }
            "inherited-parent-change" => {
                let mut genome = fixture.source.compiler_inputs.genome().clone();
                genome.parent_genome_ids.push(GenomeId(0xFEED_1234));
                let (phenotype, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
                    genome,
                    fixture.source.compiler_inputs.development().clone(),
                    changed
                        .output_handoff
                        .compiler_inputs
                        .n2048_candidate_asset()
                        .unwrap()
                        .clone(),
                )
                .unwrap();
                changed.output_handoff.actor.phenotype_hash = phenotype.phenotype_hash();
                changed.output_handoff.compiler_inputs = inputs;
            }
            _ => unreachable!(),
        }
        changed.output_handoff.validate().unwrap();
        reseal_result(&mut changed);
        write_json(output.join(CPU_COOPERATIVE_ROUND_RESULT_FILE), &changed);
        let before = tree(&output);
        assert!(read_cpu_cooperative_round(&output).is_err(), "{corruption}");
        assert!(
            run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output,)
                .is_err(),
            "{corruption}"
        );
        assert_eq!(tree(&output), before);
    }
    fixture.assert_protected(&protected);
}

#[test]
fn a_truthful_zero_update_kl_receipt_keeps_the_source_policy_version() {
    let fixture = RoundFixture::new();
    let plan = fixture.plan("zero-update-recovery");
    let output = fixture.root.join("zero-update-result");
    let mut result =
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output)
            .unwrap();
    // Synthetic recovery receipt tests the no-update boundary directly; no
    // claim is made that this fixture's real rollout triggers the KL guard.
    result.output_handoff = fixture.source.clone();
    result.update.completed_epochs = 0;
    result.update.stopped_for_kl = true;
    result.update.mean_kl_before_updates = vec![config().target_kl * 2.];
    result.update.actor_optimizer_step = fixture.source.actor.optimizer_step;
    result.update.value_optimizer_step = fixture.source.value.optimizer_step;
    reseal_result(&mut result);
    write_json(output.join(CPU_COOPERATIVE_ROUND_RESULT_FILE), &result);
    let protected = fixture.protected_bytes();
    let before = tree(&output);
    let restored = read_cpu_cooperative_round(&output).unwrap();
    assert_eq!(restored.output_handoff, fixture.source);
    assert_eq!(restored.output_handoff.policy_version, VERSION - 1);
    assert_eq!(restored.update.collection_policy_version, VERSION);
    let retried =
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output)
            .unwrap();
    assert_eq!(retried.output_handoff, fixture.source);
    assert_eq!(tree(&output), before);

    result.output_handoff.policy_version = VERSION;
    result.output_handoff.value.last_updated_policy_version = Some(VERSION);
    result.output_handoff.validate().unwrap();
    reseal_result(&mut result);
    write_json(output.join(CPU_COOPERATIVE_ROUND_RESULT_FILE), &result);
    let forced_advance = tree(&output);
    assert!(read_cpu_cooperative_round(&output).is_err());
    assert!(
        run_cpu_cooperative_round(&fixture.source_path, &plan, &fixture.collectors, &output,)
            .is_err()
    );
    assert_eq!(tree(&output), forced_advance);
    fixture.assert_protected(&protected);
}

#[test]
fn plan_rejects_unbounded_optimizer_budgets_and_overflowing_collection_limits() {
    // Constructor-only bounds need no world, checkpoint or optimizer update.
    let snapshot = CpuCollectionSnapshotV1 {
        policy_version: VERSION,
        biological_objective_version: 2,
        critic_objective_version: 1,
        actor_sha256: [1; 32],
        critic_sha256: [1; 32],
        compiler_sha256: [1; 32],
        ppo_config_sha256: [1; 32],
    };
    let member = CpuCooperativeRoundMemberV1 {
        assignment: CpuCollectorAssignmentV1 {
            collector_run_id: "synthetic-bounds".into(),
            backend: FoundationTrainingBackendV1::NativeCpu,
            producer_revision: "synthetic-coordinator-fixture-v1".into(),
            world_seed: WORLD_SEED,
            sampling_seed: 42,
            sampling_counter_start: 0,
            sampling_counter_end: 4,
            lesson: "synthetic-bounds".into(),
            episode_config_sha256: [1; 32],
        },
        collector_commit_sha256: [2; 32],
    };
    let seal = |config, window, burn, batch, members| {
        CpuCooperativeRoundPlanV1::seal(
            "synthetic-plan-bounds".into(),
            [3; 32],
            snapshot.clone(),
            config,
            window,
            burn,
            batch,
            members,
        )
    };
    assert!(seal(config(), 2, 1, 4, vec![member.clone()]).is_ok());
    let mut excessive_epochs = config();
    excessive_epochs.epochs = 65_537;
    assert!(seal(excessive_epochs, 2, 1, 4, vec![member.clone()]).is_err());
    for (window, burn) in [(0, 1), (usize::MAX, 1), (2, usize::MAX)] {
        assert!(seal(config(), window, burn, 4, vec![member.clone()]).is_err());
    }
    assert!(seal(config(), 2, 1, 0, vec![member.clone()]).is_err());
    assert!(seal(config(), 2, 1, 4, vec![member.clone(), member.clone()]).is_err());
    let too_many = (0..65)
        .map(|index| {
            let mut member = member.clone();
            member.assignment.collector_run_id = format!("synthetic-bounds-{index}");
            member
        })
        .collect();
    assert!(seal(config(), 2, 1, 4, too_many).is_err());
    let mut too_many_decisions = member.clone();
    too_many_decisions.assignment.sampling_counter_end = 65_537;
    assert!(seal(config(), 2, 1, 4, vec![too_many_decisions]).is_err());
    let mut reversed_range = member;
    reversed_range.assignment.sampling_counter_start = 5;
    assert!(seal(config(), 2, 1, 4, vec![reversed_range]).is_err());
}
