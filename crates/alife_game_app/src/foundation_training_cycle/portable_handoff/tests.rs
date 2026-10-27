use super::*;

#[test]
#[ignore = "requires exclusively admitted hardware GPU; resident driver is experimental"]
fn resident_two_episode_final_snapshot_keeps_actor_value_ages_and_private_state() {
    std::thread::Builder::new().name("resident-continuity-assay".into())
        .stack_size(32 * 1024 * 1024).spawn(|| {
            let root = std::env::temp_dir().join(format!("alife-resident-gpu-{}-{}", std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
            std::fs::create_dir(&root).unwrap();
            eprintln!("resident disposable fixture: {}", root.display());
            let initial = root.join("initial-policy-zero");
            let bootstrap = run_foundation_training_cycle(&initial, 90701, 2).unwrap();
            assert_eq!(bootstrap.policy_version, 0);
            let source = root.join("source");
            // The ordinary bootstrap is policy zero, deliberately inadmissible
            // as a portable handoff. Reach policy one through a genuine update;
            // do not rewrite its receipt, weights, ages or admission rules.
            let continued = resume_foundation_training_cycle_with_final_private_life(
                &initial, &source, 90702, 2, FoundationTeacherLesson::Feeding,
                1.0, Default::default(),
            ).unwrap();
            assert_eq!(continued.policy_version, 1);
            let seeds = alife_training::CheckpointSeedsV1 { historical_freshness: "unknown".into(),
                excluded_ranges: vec![[90701, 90702]], pending_range: None };
            let baseline = migrate_current_training_checkpoint(&source, seeds).unwrap();
            let store = root.join("store"); std::fs::create_dir(&store).unwrap();
            let current = store.join("current.alife-checkpoint");
            let original = baseline.encode().unwrap(); std::fs::write(&current, &original).unwrap();
            let request = ResidentSerialRequest {
                schema: 1, seed_range: [90703, 90704],
                rows: vec![ResidentSerialRow { lesson: FoundationTeacherLesson::Feeding, decisions: 2,
                    food_need_condition: Default::default() }; 2],
                checkpoint_every_episodes: 2, sampling_temperature: 1.0,
                collection_cutoff_unix_seconds: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 1800,
                max_temporary_bytes: 2 * 1024 * 1024 * 1024,
            };
            let started = std::time::Instant::now();
            let report = run_resident_serial_care(&store, request).unwrap();
            assert_eq!(report["completed_provisional_episodes"], 2);
            assert_eq!(report["canonical_current_unchanged"], true);
            let recovered = recover_resident_serial_checkpoint(&store).unwrap();
            let saved = read_current_training_checkpoint(std::path::Path::new(recovered["candidate"].as_str().unwrap())).unwrap();
            assert_eq!(saved.handoff.policy_version, baseline.handoff.policy_version + 2);
            assert!(saved.handoff.actor.optimizer_step > baseline.handoff.actor.optimizer_step);
            assert!(saved.handoff.value.optimizer_step > baseline.handoff.value.optimizer_step);
            assert_eq!(saved.seeds.pending_range, None);
            assert_eq!(saved.seed, 90704);
            assert!(saved.life.is_some());
            assert_eq!(std::fs::read(&current).unwrap(), original);
            std::fs::remove_dir_all(source).unwrap();
            std::fs::remove_dir_all(initial).unwrap();
            assert_eq!(read_current_training_checkpoint(std::path::Path::new(recovered["candidate"].as_str().unwrap())).unwrap(), saved);
            eprintln!("resident continuity elapsed_seconds={} actor_before={} actor_after={} value_before={} value_after={}", started.elapsed().as_secs_f64(),
                baseline.handoff.actor.optimizer_step, saved.handoff.actor.optimizer_step,
                baseline.handoff.value.optimizer_step, saved.handoff.value.optimizer_step);
            std::fs::remove_dir_all(root).unwrap();
        }).unwrap().join().unwrap();
}

/// This must be dispatched explicitly by the hardware owner, never by ordinary
/// device-free checkpoint CI. It uses only new disposable worlds and sources.
#[test]
#[ignore = "requires an exclusively admitted hardware GPU and a qualified test executable"]
fn directory_opt_in_keeps_final_private_life_across_two_native_continuations() {
    // Match the qualified CLI's Windows worker stack. The production runtime's
    // bounded debug frames do not fit the ordinary test-harness thread stack.
    std::thread::Builder::new()
        .name("directory-final-private-life-assay".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(directory_final_private_life_assay)
        .unwrap()
        .join()
        .unwrap();
}

fn directory_final_private_life_assay() {
    let root = std::env::temp_dir().join(format!(
        "alife-directory-final-private-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    struct OwnedRoot(std::path::PathBuf);
    impl Drop for OwnedRoot {
        fn drop(&mut self) {
            if std::thread::panicking() {
                eprintln!(
                    "failed disposable GPU fixture retained: {}",
                    self.0.display()
                );
                return;
            }
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _owned = OwnedRoot(root.clone());
    let initial = root.join("initial");
    let first = root.join("first-opt-in");
    let second = root.join("second-opt-in");
    let source = run_foundation_training_cycle(&initial, 90101, 2).unwrap();
    let mut previous = initial.clone();
    let mut old_actor = source.actor_optimizer_step;
    let mut old_value = source.value_optimizer_step;
    let mut old_policy = source.policy_version;
    for (output, seed) in [(&first, 90102), (&second, 90103)] {
        let receipt = resume_foundation_training_cycle_with_final_private_life(
            &previous,
            output,
            seed,
            2,
            FoundationTeacherLesson::Feeding,
            1.0,
            crate::FoundationFoodNeedCondition::default(),
        )
        .unwrap();
        let (_, loaded) = GpuDurableSaveManifest::open_loaded(
            output.join("world.json"),
            output.join("world-assets"),
        )
        .unwrap();
        let final_world = loaded.save.restore_headless_world().unwrap();
        assert_eq!(final_world.tick().raw(), receipt.world_ticks_elapsed);
        assert!(
            receipt.world_ticks_elapsed > 0,
            "initial-only save cannot pass"
        );
        assert_eq!(receipt.policy_version, old_policy + 1);
        assert_eq!(
            receipt.actor_optimizer_step - old_actor,
            receipt.actor_optimizer_updates
        );
        assert_eq!(
            receipt.value_optimizer_step - old_value,
            receipt.value_optimizer_updates
        );
        old_policy = receipt.policy_version;
        old_actor = receipt.actor_optimizer_step;
        old_value = receipt.value_optimizer_step;
        previous = output.clone();
    }
    let packed = migrate_current_training_checkpoint(&second, current_seeds()).unwrap();
    let current = root.join("current.alife-checkpoint");
    std::fs::write(&current, packed.encode().unwrap()).unwrap();
    std::fs::remove_dir_all(initial).unwrap();
    std::fs::remove_dir_all(first).unwrap();
    std::fs::remove_dir_all(second).unwrap();
    let admitted = read_current_training_checkpoint(&current).unwrap();
    assert_eq!(admitted, packed);
    assert_eq!(admitted.handoff.policy_version, old_policy);
    assert_eq!(admitted.handoff.actor.optimizer_step, old_actor);
    assert_eq!(admitted.handoff.value.optimizer_step, old_value);
}

fn current_seeds() -> alife_training::CheckpointSeedsV1 {
    alife_training::CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: vec![[1, u64::MAX]],
        pending_range: None,
    }
}

#[test]
fn resident_full_head_json_and_current_roundtrip_preserve_f32_bits() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            private_fixture,
        );
    let mut current = migrate_current_training_checkpoint(&f.target, current_seeds()).unwrap();
    let samples = [
        0.0_f32,
        -0.0,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        -f32::MAX,
        f32::from_bits(1.0_f32.to_bits() + 1),
        f32::from_bits(1.0_f32.to_bits() - 1),
    ];
    // Serialization stress only, not a claim that these extreme moments form
    // a useful optimizer trajectory. All values satisfy native restore bounds.
    let width = current.handoff.value.feature_count as usize + 1;
    for (i, value) in samples.iter().copied().enumerate() {
        current.handoff.actor.first_moment[i] = value;
        current.handoff.actor.second_moment[i] = if value == 0.0 { value } else { value.abs() };
        current.handoff.actor.update_ages[i] = 1;
        current.handoff.value.parameters[i] = value;
        current.handoff.value.parameters[width + i] = value;
        current.handoff.value.parameters[2 * width + i] =
            if value == 0.0 { value } else { value.abs() };
    }
    current.handoff.validate().unwrap();
    let actor_json = serde_json::to_vec(&current.handoff.actor).unwrap();
    let actor: alife_training::FoundationTrainerCheckpoint =
        serde_json::from_slice(&actor_json).unwrap();
    let value_json = serde_json::to_vec(&current.handoff.value).unwrap();
    let value: alife_training::PpoValueHeadCheckpoint =
        serde_json::from_slice(&value_json).unwrap();
    let path = f.root.join("bit-roundtrip.alife-checkpoint");
    std::fs::write(&path, current.encode().unwrap()).unwrap();
    let restored = read_current_training_checkpoint(&path).unwrap();
    let bits = |values: &[f32]| {
        values
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>()
    };
    for decoded in [&actor, &restored.handoff.actor] {
        assert_eq!(bits(&decoded.weights), bits(&current.handoff.actor.weights));
        assert_eq!(
            bits(&decoded.first_moment),
            bits(&current.handoff.actor.first_moment)
        );
        assert_eq!(
            bits(&decoded.second_moment),
            bits(&current.handoff.actor.second_moment)
        );
        assert_eq!(decoded.update_ages, current.handoff.actor.update_ages);
        assert_eq!(decoded.config, current.handoff.actor.config);
        assert_eq!(decoded.stage_mask, current.handoff.actor.stage_mask);
        assert_eq!(decoded.optimizer_step, current.handoff.actor.optimizer_step);
    }
    assert_eq!(
        bits(&value.parameters),
        bits(&current.handoff.value.parameters)
    );
    assert_eq!(
        bits(&restored.handoff.value.parameters),
        bits(&current.handoff.value.parameters)
    );
    assert_eq!(
        restored.handoff.value.last_updated_policy_version,
        current.handoff.value.last_updated_policy_version
    );
    assert_eq!(
        restored.handoff.value.optimizer_step,
        current.handoff.value.optimizer_step
    );
    assert_eq!(restored.life, current.life);
    assert_eq!(
        restored.handoff.compiler_inputs,
        current.handoff.compiler_inputs
    );
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

fn final_private_fixture(source: &Path) {
    private_fixture(source);
    // Synthetic fixture only: its persisted private world is at tick zero.
    // This does not claim a GPU trajectory or a learned biological outcome.
    let path = source.join("cycle.json");
    let mut receipt: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    receipt.world_ticks_elapsed = 0;
    std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}

#[test]
fn native_frontier_admits_full_heads_and_private_state_without_encoding_current() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            final_private_fixture,
        );
    let expected = migrate_current_training_checkpoint(&f.target, current_seeds()).unwrap();
    let report = inspect_native_training_frontier(&f.target, 5124).unwrap();
    assert_eq!(report["kind"], "native-training-frontier-v1");
    assert_eq!(report["policy_version"], 5124);
    assert_eq!(
        report["actor_optimizer_step"],
        expected.handoff.actor.optimizer_step
    );
    assert_eq!(report["value_optimizer_step"], 6);
    assert_eq!(report["final_private_world_tick"], 0);
    assert_eq!(report["current_file_encoded"], false);
    assert_eq!(report["neural_execution"], false);
    assert_eq!(report["weights_modified"], false);
    assert!(
        report.get("seeds").is_none(),
        "not an allocation/freshness authority"
    );
    assert_eq!(
        report["source_dependencies"],
        serde_json::json!([f.source.canonicalize().unwrap()])
    );
    let private = expected.life.unwrap();
    assert_eq!(
        report["canonical_private_world_sha256"],
        format!("{:x}", Sha256::digest(private.world_json.as_bytes()))
    );
    assert_eq!(
        report["canonical_sleep_journal_sha256"],
        format!(
            "{:x}",
            Sha256::digest(private.sleep_journal_json.as_bytes())
        )
    );
    assert_eq!(
        report["private_assets"]["private-sidecar.bin"]["sha256"],
        format!("{:x}", Sha256::digest([0, 128, 255, 42, 0, 3]))
    );
    assert!(inspect_native_training_frontier(&f.target, 5123).is_err());
    assert!(!f.target.join("current.alife-checkpoint").exists());
}

#[test]
fn native_frontier_rejects_private_world_asset_and_journal_corruption_ignored_by_four_file_inspection(
) {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            final_private_fixture,
        );
    inspect_native_training_frontier(&f.source, 5124).unwrap();
    let pointer: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.source.join("world.json")).unwrap()).unwrap();
    let journal = pointer["gpu_checkpoint_authority"]["journal"]["file_name"]
        .as_str()
        .expect("fixture uses the authoritative published journal");
    for path in [
        f.source.join("world.json"),
        f.source.join("world-assets/private-sidecar.bin"),
        f.source.join(journal),
    ] {
        let original = std::fs::read(&path).ok();
        std::fs::write(&path, b"corrupt private recovery material").unwrap();
        assert!(
            crate::inspect_foundation_checkpoint(&f.source, 5124).is_ok(),
            "four-file check must not pretend to cover private state"
        );
        assert!(
            inspect_native_training_frontier(&f.source, 5124).is_err(),
            "{}",
            path.display(),
        );
        if let Some(bytes) = original {
            std::fs::write(&path, bytes).unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
        inspect_native_training_frontier(&f.source, 5124).unwrap();
    }
}

#[test]
fn native_frontier_rejects_initial_only_private_save_at_a_later_collection_boundary() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            final_private_fixture,
        );
    let path = f.source.join("cycle.json");
    let mut receipt: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    receipt.world_ticks_elapsed = 1;
    std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert!(crate::inspect_foundation_checkpoint(&f.source, 5124).is_ok());
    assert!(inspect_native_training_frontier(&f.source, 5124)
        .unwrap_err()
        .to_string()
        .contains("final collection boundary"));
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
fn resident_crash_recovery_preserves_full_baseline_but_consumes_lost_world_seeds() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            private_fixture,
        );
    let mut baseline = migrate_current_training_checkpoint(&f.target, current_seeds()).unwrap();
    baseline.seeds = alife_training::CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: vec![[1, baseline.seed]],
        pending_range: None,
    };
    let store = f.root.join("resident-store");
    std::fs::create_dir(&store).unwrap();
    let current = store.join("current.alife-checkpoint");
    let original = baseline.encode().unwrap();
    std::fs::write(&current, &original).unwrap();
    let first = baseline.seed + 100;
    let request = super::super::resident::ResidentSerialRequest {
        schema: 1,
        seed_range: [first, first + 4],
        rows: vec![
            super::super::resident::ResidentSerialRow {
                lesson: FoundationTeacherLesson::Feeding,
                decisions: 16,
                food_need_condition: Default::default(),
            };
            3
        ],
        checkpoint_every_episodes: 3,
        sampling_temperature: 1.0,
        collection_cutoff_unix_seconds: 1,
        max_temporary_bytes: 1024 * 1024 * 1024,
    };
    // Simulate a crash after two WAL dispatches and before any full snapshot.
    super::super::resident::stage_resident_loss_fixture(&store, request, 2).unwrap();
    // Both the original experimental direct-write failure and an interrupted
    // new staging write must be repairable without touching accepted current.
    std::fs::write(
        store.join(".run/recovered-current.alife-checkpoint"),
        b"partial candidate",
    )
    .unwrap();
    std::fs::write(
        store.join(".run/recovered-current.pending"),
        b"partial staging",
    )
    .unwrap();
    std::fs::remove_dir_all(&f.target).unwrap();
    std::fs::remove_dir_all(&f.source).unwrap();
    let report = recover_resident_serial_checkpoint(&store).unwrap();
    let candidate = std::path::Path::new(report["candidate"].as_str().unwrap());
    let recovered = read_current_training_checkpoint(candidate).unwrap();
    assert_eq!(recovered.handoff, baseline.handoff);
    assert_eq!(recovered.life, baseline.life);
    assert_eq!(recovered.seed, baseline.seed);
    assert_eq!(recovered.founder_seed_base, baseline.founder_seed_base);
    assert_eq!(recovered.seeds.pending_range, Some([first + 2, first + 4]));
    assert!(recovered.seeds.contains(first));
    assert!(recovered.seeds.contains(first + 1));
    assert_eq!(std::fs::read(&current).unwrap(), original);
    assert_eq!(report["uncheckpointed_updates_restored"], false);
    // Recovery retries are idempotent. A committed owner publication is detected.
    std::fs::write(store.join(".run/recovery-intent.next"), b"partial intent").unwrap();
    assert_eq!(recover_resident_serial_checkpoint(&store).unwrap(), report);
    std::fs::copy(candidate, &current).unwrap();
    assert_eq!(
        recover_resident_serial_checkpoint(&store).unwrap()["already_published_current"],
        true
    );
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
