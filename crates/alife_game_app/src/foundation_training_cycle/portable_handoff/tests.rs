use super::*;
fn current_seeds() -> alife_training::CheckpointSeedsV1 {
    alife_training::CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: vec![[1, u64::MAX]],
        pending_range: None,
    }
}

fn private_fixture(source: &Path) {
    let actor_path = source.join("actor-checkpoint.json");
    let mut actor: alife_training::FoundationTrainerCheckpoint =
        serde_json::from_slice(&std::fs::read(&actor_path).unwrap()).unwrap();
    actor.first_moment[17] = 0.125;
    actor.second_moment[17] = 0.03125;
    actor.update_ages[17] = 1;
    std::fs::write(actor_path, serde_json::to_vec(&actor).unwrap()).unwrap();
    use alife_core::{
        BrainCapacityClass, BrainScaleTier, CreatureGenome, FoundationGeneticIdentity, OrganismId,
        Tick, Vec3f,
    };
    use alife_world::{
        AssetKind, AssetManifest, AssetManifestEntry, AssetPresence, HeadlessScenarioBuilder,
        PortableAssetDigest, PortableSaveFile, RuntimeConfig, WorldOrganismRecord,
    };
    let receipt: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(source.join("cycle.json")).unwrap()).unwrap();
    let mut world = HeadlessScenarioBuilder::new(receipt.seed)
        .agent("private-life", OrganismId(1), Vec3f::ZERO)
        .build()
        .unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        73128,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let record = WorldOrganismRecord::newborn(
        OrganismId(1),
        world.entity_id("private-life").unwrap(),
        genome,
        phenotype,
        Tick::ZERO,
    )
    .unwrap();
    world.register_organism_record(record).unwrap();
    let assets = source.join("world-assets");
    std::fs::create_dir(&assets).unwrap();
    let bytes = [0, 128, 255, 42, 0, 3];
    std::fs::write(assets.join("private-sidecar.bin"), bytes).unwrap();
    let mut manifest = AssetManifest::empty();
    manifest.entries.push(AssetManifestEntry {
        asset_id: "private-sidecar".into(),
        kind: AssetKind::Other,
        relative_path: "private-sidecar.bin".into(),
        digest: PortableAssetDigest::for_bytes(&bytes),
        presence: AssetPresence::Required,
        schema_version: 1,
        size_bytes: Some(bytes.len() as u64),
        provenance: None,
    });
    let save = PortableSaveFile::from_headless_world(
        "private-current-fixture",
        &world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        manifest,
        vec![],
    )
    .unwrap();
    GpuDurableSaveManifest::publish_snapshot(source.join("world.json"), &assets, &save).unwrap();
}

#[test]
fn current_checkpoint_migration_relocates_without_any_legacy_dependencies() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            private_fixture,
        );
    let legacy = load_cycle_continuation(&f.target, None).unwrap();
    let current = migrate_current_training_checkpoint(&f.target, current_seeds()).unwrap();
    assert_eq!(current.handoff.actor, legacy.actor);
    assert_eq!(current.handoff.value, legacy.value);
    assert_eq!(current.handoff.value.optimizer_step, 6); // Corrected age, not old receipt's 1.
    let compiler_bytes = serde_json::to_vec(&current.handoff.compiler_inputs).unwrap();
    let bytes = current.encode().unwrap();
    let first = f.root.join("current.alife-checkpoint");
    std::fs::write(&first, &bytes).unwrap();
    let relocated = f.root.join("relocated");
    std::fs::create_dir(&relocated).unwrap();
    let copy = relocated.join("current.alife-checkpoint");
    std::fs::copy(&first, &copy).unwrap();
    std::fs::remove_file(&first).unwrap();
    std::fs::remove_dir_all(&f.target).unwrap();
    std::fs::remove_dir_all(&f.source).unwrap();
    let restored = read_current_training_checkpoint(&copy).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.handoff.compiler_inputs).unwrap(),
        compiler_bytes
    );
    assert_eq!(restored, current);
    let admitted = load_cycle_continuation(&copy, None).unwrap();
    assert_eq!(admitted.actor, legacy.actor);
    assert_eq!(admitted.value, legacy.value);
    assert_eq!(
        admitted.asset.encode_canonical().unwrap(),
        legacy.asset.encode_canonical().unwrap()
    );
    assert_eq!(admitted.policy_version, legacy.policy_version);
    assert_eq!(admitted.founder_seed_base, legacy.founder_seed_base);
    assert!(load_cycle_continuation(&copy, Some(1)).is_err());
    assert_eq!(std::fs::read_dir(&relocated).unwrap().count(), 1);
    let extracted = relocated.join("extracted-private");
    extract_current_training_life(&copy, &extracted).unwrap();
    let (_, loaded) = GpuDurableSaveManifest::open_loaded(
        extracted.join("world.json"),
        extracted.join("world-assets"),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_string(&loaded.save).unwrap(),
        current.life.as_ref().unwrap().world_json
    );
    assert_eq!(
        loaded.save.world.organism_records.as_ref().unwrap().len(),
        1
    );
    assert_eq!(
        std::fs::read(extracted.join("world-assets/private-sidecar.bin")).unwrap(),
        [0, 128, 255, 42, 0, 3]
    );
    // A fully checksummed outer file cannot conceal a missing/corrupt private asset.
    let mut corrupt = current;
    corrupt.life.as_mut().unwrap().assets_base64.clear();
    std::fs::write(&copy, corrupt.encode().unwrap()).unwrap();
    assert!(read_current_training_checkpoint(&copy).is_err());
}

#[test]
fn current_checkpoint_rejects_corrupt_or_truncated_state_and_legacy_tampering() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            private_fixture,
        );
    let checkpoint = migrate_current_training_checkpoint(&f.target, current_seeds()).unwrap();
    let bytes = checkpoint.encode().unwrap();
    for length in [0, 16, 24, 55, 56, bytes.len() - 1] {
        assert!(alife_training::CurrentTrainingCheckpointV1::decode(&bytes[..length]).is_err());
    }
    for index in [0, 17, 25, 60, bytes.len() - 1] {
        let mut corrupt = bytes.clone();
        corrupt[index] ^= 1;
        assert!(alife_training::CurrentTrainingCheckpointV1::decode(&corrupt).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(alife_training::CurrentTrainingCheckpointV1::decode(&trailing).is_err());
    // A correctly checksummed but inconsistent semantic checkpoint still rejects.
    let mut wrong = checkpoint;
    wrong.handoff.actor.first_moment[0] = f32::NAN;
    assert!(wrong.encode().is_err());
    let path = f.target.join("actor-checkpoint.json");
    let mut actor: alife_training::FoundationTrainerCheckpoint =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    actor.first_moment[0] = 0.25;
    std::fs::write(path, serde_json::to_vec(&actor).unwrap()).unwrap();
    assert!(migrate_current_training_checkpoint(&f.target, current_seeds()).is_err());
}

#[test]
fn current_checkpoint_eight_synthetic_publications_keep_only_current_full_state() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            private_fixture,
        );
    let mut checkpoint = migrate_current_training_checkpoint(&f.target, current_seeds()).unwrap();
    let original_compiler = serde_json::to_vec(&checkpoint.handoff.compiler_inputs).unwrap();
    let original_actor = serde_json::to_vec(&checkpoint.handoff.actor).unwrap();
    let original_life = checkpoint.life.clone();
    let current = f.root.join("current.alife-checkpoint");
    let mut measurements = Vec::new();
    for _ in 0..8 {
        // Exercise actual codec/admission with nonzero carried moments, without
        // claiming that these synthetic publications are biological training.
        checkpoint.handoff.policy_version += 1;
        checkpoint.handoff.value.last_updated_policy_version =
            Some(checkpoint.handoff.policy_version);
        let candidate = f.root.join("candidate");
        let bytes = checkpoint.encode().unwrap();
        std::fs::write(&candidate, &bytes).unwrap();
        read_current_training_checkpoint(&candidate).unwrap();
        std::fs::rename(&candidate, &current).unwrap();
        let restored = read_current_training_checkpoint(&current).unwrap();
        assert_eq!(
            serde_json::to_vec(&restored.handoff.compiler_inputs).unwrap(),
            original_compiler
        );
        assert_eq!(
            serde_json::to_vec(&restored.handoff.actor).unwrap(),
            original_actor
        );
        assert_eq!(restored.handoff.value, checkpoint.handoff.value);
        assert_eq!(restored.life, original_life);
        let metadata = std::fs::metadata(&current).unwrap();
        #[cfg(unix)]
        let allocated = {
            use std::os::unix::fs::MetadataExt;
            Some(metadata.blocks() * 512)
        };
        #[cfg(not(unix))]
        let allocated: Option<u64> = None;
        measurements.push(serde_json::json!({"logical_bytes":metadata.len(), "inode_allocated_bytes":allocated,"checkpoint_files":1}));
    }
    assert!(measurements.windows(2).all(|w| w[0] == w[1]));
    eprintln!(
        "eight synthetic current publication measurements: {}",
        serde_json::to_string(&measurements).unwrap()
    );
    if let Some(output) = std::env::var_os("ALIFE_CURRENT_FIXTURE_OUTPUT") {
        use std::io::Write;
        checkpoint.seeds.excluded_ranges = vec![[checkpoint.seed, checkpoint.seed]];
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .unwrap();
        file.write_all(&checkpoint.encode().unwrap()).unwrap();
        file.sync_all().unwrap();
    }
}

#[test]
fn current_checkpoint_advancement_preserves_full_state_and_rejects_wrong_predecessor() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            private_fixture,
        );
    let current = migrate_current_training_checkpoint(&f.target, current_seeds()).unwrap();
    let before = current.encode().unwrap();
    let episode = f.root.join("synthetic-next");
    std::fs::create_dir(&episode).unwrap();
    for name in SERIAL_HANDOFF_SOURCE_FILES
        .into_iter()
        .chain(["critic-residual-update.json"])
    {
        std::fs::copy(f.target.join(name), episode.join(name)).unwrap();
    }
    std::fs::copy(
        f.target.join("trained.alife-foundation"),
        episode.join("initial.alife-foundation"),
    )
    .unwrap();
    let receipt_path = episode.join("cycle.json");
    let mut receipt: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(&receipt_path).unwrap()).unwrap();
    receipt.seed += 1;
    receipt.policy_version += 1;
    receipt.actor_optimizer_step = current.handoff.actor.optimizer_step + 1;
    receipt.value_optimizer_step = current.handoff.value.optimizer_step + 1;
    receipt.actor_optimizer_updates = 1;
    receipt.value_optimizer_updates = 1;
    std::fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    private_fixture(&episode);
    let mut actor = current.handoff.actor.clone();
    actor.optimizer_step += 1;
    actor.update_ages[17] += 1;
    actor.first_moment[17] = 0.25;
    actor.second_moment[17] = 0.0625;
    let mut value = current.handoff.value.clone();
    value.optimizer_step += 1;
    value.last_updated_policy_version = Some(receipt.policy_version);
    value.parameters[0] = -0.125;
    std::fs::write(
        episode.join("actor-checkpoint.json"),
        serde_json::to_vec(&actor).unwrap(),
    )
    .unwrap();
    std::fs::write(
        episode.join("value-checkpoint.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let next = advance_current_training_checkpoint(&current, &episode).unwrap();
    assert_eq!(next.handoff.actor, actor);
    assert_eq!(next.handoff.value, value);
    assert_eq!(
        next.handoff.compiler_inputs,
        current.handoff.compiler_inputs
    );
    assert_eq!(next.seeds, current.seeds);
    assert_eq!(current.encode().unwrap(), before);
    receipt.actor_optimizer_updates = 0;
    std::fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    assert!(advance_current_training_checkpoint(&current, &episode).is_err());
    receipt.actor_optimizer_updates = 1;
    std::fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    std::fs::write(
        episode.join("initial.alife-foundation"),
        b"wrong predecessor",
    )
    .unwrap();
    assert!(advance_current_training_checkpoint(&current, &episode).is_err());
    assert_eq!(current.encode().unwrap(), before);
}

#[test]
fn current_private_validation_cleans_process_exit_orphans_and_preserves_unknown_files() {
    const FLAG: &str = "ALIFE_CURRENT_VALIDATION_FAULT_ROOT";
    if let Some(root) = std::env::var_os(FLAG) {
        let workspace = current::validation_workspace_for_test(Path::new(&root)).unwrap();
        std::fs::write(
            workspace.as_ref().join("orphan-private-copy"),
            b"disposable synthetic private bytes",
        )
        .unwrap();
        std::process::exit(0); // No Drop: simulate loss of the validator process.
    }
    let root = std::env::temp_dir().join(format!(
        "alife-validation-fault-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "foundation_training_cycle::portable_handoff::tests::current_private_validation_cleans_process_exit_orphans_and_preserves_unknown_files"])
        .env(FLAG, &root).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(root.join("work/orphan-private-copy").exists());
    let workspace = current::validation_workspace_for_test(&root).unwrap();
    assert!(!workspace.as_ref().join("orphan-private-copy").exists());
    drop(workspace);
    assert!(!root.join("work").exists());
    std::fs::write(root.join("other-owner"), b"preserve unknown data").unwrap();
    assert!(current::validation_workspace_for_test(&root).is_err());
    assert_eq!(
        std::fs::read(root.join("other-owner")).unwrap(),
        b"preserve unknown data"
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn native_serial_export_preserves_corrected_critic_age_and_original_source() {
    let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture();
    let before =
        SERIAL_HANDOFF_SOURCE_FILES.map(|name| std::fs::read(f.target.join(name)).unwrap());
    let output = f.root.join("portable");
    let binding = export_native_training_handoff(&f.target, &output).unwrap();
    let handoff: FoundationTrainingHandoffV1 =
        serde_json::from_slice(&std::fs::read(output.join(SERIAL_HANDOFF_FILE)).unwrap()).unwrap();
    assert_eq!(handoff.actor.optimizer_step, 1);
    assert_eq!(handoff.value.optimizer_step, 6);
    let original_receipt: FoundationCycleReceipt = serde_json::from_slice(&before[0]).unwrap();
    assert_eq!(original_receipt.value_optimizer_step, 1);
    assert_eq!(handoff.value.parameters[0], 0.25);
    assert!(
        handoff.actor.stage_mask
            == serde_json::from_slice::<alife_training::FoundationTrainerCheckpoint>(&before[2])
                .unwrap()
                .stage_mask
    );
    for (name, bytes) in SERIAL_HANDOFF_SOURCE_FILES.iter().zip(before) {
        assert_eq!(std::fs::read(f.target.join(name)).unwrap(), bytes);
        assert_eq!(std::fs::read(output.join(name)).unwrap(), bytes);
    }
    binding
        .validate(
            &binding.source_sha256,
            &sha(&std::fs::read(output.join(SERIAL_HANDOFF_FILE)).unwrap()),
        )
        .unwrap();
    assert!(!output.join("critic-correction.json").exists());
}
#[test]
fn native_serial_export_runs_correction_verifier_before_creating_output() {
    let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture();
    let actor_path = f.target.join("actor-checkpoint.json");
    let mut actor: alife_training::FoundationTrainerCheckpoint =
        serde_json::from_slice(&std::fs::read(&actor_path).unwrap()).unwrap();
    actor.first_moment[0] = 0.125;
    let changed = serde_json::to_vec(&actor).unwrap();
    std::fs::write(&actor_path, &changed).unwrap();
    let output = f.root.join("rejected");
    assert!(export_native_training_handoff(&f.target, &output).is_err());
    assert!(!output.exists());
    assert_eq!(std::fs::read(&actor_path).unwrap(), changed);
}

#[test]
fn native_export_cannot_mutate_target_or_original_correction_tree() {
    let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture();
    for root in [&f.target, &f.source] {
        let output = root.join("new-parent/portable");
        assert!(export_native_training_handoff(&f.target, &output)
            .unwrap_err()
            .to_string()
            .contains("overlap"));
        assert!(!root.join("new-parent").exists());
        crate::inspect_foundation_checkpoint(&f.target, 5124).unwrap();
    }
}
#[cfg(unix)]
#[test]
fn native_export_resolves_output_parent_symlinks_before_ownership() {
    let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture();
    let alias = f.root.join("original-source-alias");
    std::os::unix::fs::symlink(&f.source, &alias).unwrap();
    let output = alias.join("not-created/portable");
    assert!(export_native_training_handoff(&f.target, &output)
        .unwrap_err()
        .to_string()
        .contains("overlap"));
    assert!(!f.source.join("not-created").exists());
    crate::inspect_foundation_checkpoint(&f.target, 5124).unwrap();
}
