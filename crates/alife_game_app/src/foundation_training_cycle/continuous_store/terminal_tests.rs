use super::*;

fn one_organism_private_fixture(source: &Path) {
    // Use the ordinary canonical birth path and its actual creature save
    // summaries. The older portable fixture deliberately saves no creatures.
    let receipt: FoundationCycleReceipt =
        serde_json::from_slice(&fs::read(source.join("cycle.json")).unwrap()).unwrap();
    let asset = alife_core::FoundationWeightAsset::decode_canonical(
        &fs::read(source.join("trained.alife-foundation")).unwrap(),
    )
    .unwrap();
    let mut config = alife_world::CanonicalNewGameConfig::phase3(receipt.seed, 1).unwrap();
    config.brain_class = alife_core::BrainScaleTier::Standard2048;
    config.founder_seed_base = if receipt.founder_seed_base == 0 {
        receipt.seed
    } else {
        receipt.founder_seed_base
    };
    config.sensor_profile = asset.manifest().sensor_profile();
    let game =
        alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
    assert_eq!(game.creatures.len(), 1);
    assert_eq!(game.world.organism_registry().len(), 1);
    let save = alife_world::PortableSaveFile::from_headless_world(
        "continuous-terminal-canonical-cpu-fixture",
        &game.world,
        alife_world::RuntimeConfig::deterministic_default(receipt.seed, config.brain_class),
        alife_world::AssetManifest::empty(),
        game.creatures,
    )
    .unwrap();
    let assets = source.join("world-assets");
    fs::create_dir(&assets).unwrap();
    save.validate_with_asset_root(&assets).unwrap();
    let restored = save.restore_headless_world().unwrap();
    let organism = restored.organism_registry().iter().next().unwrap();
    assert_eq!(save.creatures[0].organism_id, organism.organism_id());
    assert_eq!(save.creatures[0].genome_id, organism.genome().id);
    assert_eq!(
        save.creatures[0].development_tick,
        organism.biochemistry().development.last_update_tick
    );
    assert_eq!(
        save.creatures[0].mind.homeostasis,
        organism.biochemistry().homeostasis
    );
    GpuDurableSaveManifest::publish_snapshot(source.join("world.json"), &assets, &save).unwrap();
}

#[test]
fn terminal_recovery_retains_exact_brain_and_cannot_resume_a_copied_current() {
    terminal_recovery_case(None);
}

#[test]
fn terminal_recovery_after_replay_cleanup_failure_cannot_resume_copied_current() {
    terminal_recovery_case(Some("replay cleanup"));
}

#[test]
fn terminal_recovery_after_causal_measure_failure_cannot_resume_copied_current() {
    terminal_recovery_case(Some("causal measure"));
}

fn terminal_recovery_case(injected_failure: Option<&str>) {
    let fixture =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            one_organism_private_fixture,
        );
    // The correction fixture clones its file inventory, which cannot represent
    // this canonical world's declared but empty asset directory.
    let source_assets = fixture.source.join("world-assets");
    assert!(source_assets.is_dir());
    assert!(fs::read_dir(source_assets).unwrap().next().is_none());
    fs::create_dir(fixture.target.join("world-assets")).unwrap();
    let mut baseline = migrate_current_training_checkpoint(
        &fixture.target,
        CheckpointSeedsV1 {
            historical_freshness: "unknown".into(),
            excluded_ranges: vec![[1, u64::MAX]],
            pending_range: None,
        },
    )
    .unwrap();
    baseline.seeds.excluded_ranges = vec![[baseline.seed, baseline.seed]];
    let original = baseline.encode().unwrap();
    let store = fixture.root.join("terminal-store");
    fs::create_dir(&store).unwrap();
    let store = store.canonicalize().unwrap();
    let current = store.join("current.alife-checkpoint");
    fs::write(&current, &original).unwrap();
    let nonce = baseline.seed + 100;
    let now = now_seconds().unwrap();
    let request = ResidentContinuousRequest {
        schema: 1,
        seed_range: [nonce, nonce + 2],
        total_trained_decisions: 100,
        decisions_per_update: 10,
        snapshot_every_updates: 1,
        sampling_temperature: 1.,
        collection_cutoff_unix_seconds: now + 3600,
        finalization_deadline_unix_seconds: now + 7200,
        max_temporary_bytes: 2 * 1024 * 1024 * 1024,
    };
    let o = Owner {
        schema: 1,
        kind: KIND.into(),
        store: store.clone(),
        pid: std::process::id(),
        baseline_sha256: hash(&original),
        request_sha256: hash(&serde_json::to_vec(&request).unwrap()),
        request: request.clone(),
    };
    let run = store.join(".run");
    fs::create_dir(&run).unwrap();
    write_new(&run.join("run.json"), &serde_json::to_vec(&o).unwrap()).unwrap();
    write_new(&run.join("baseline.alife-checkpoint"), &original).unwrap();
    let mut wal = initial_wal(&o, &baseline).unwrap();
    wal.consumed = true;
    wal.seeds = session_seeds(&baseline.seeds, &request, true).unwrap();
    write_metadata(&run.join("seed-wal.bin"), WAL, &wal).unwrap();
    write_metadata(
        &run.join("continuous-frontier.bin"),
        FRONTIER,
        &initial_frontier(&o),
    )
    .unwrap();
    let save =
        alife_world::PortableSaveFile::from_json_str(&baseline.life.as_ref().unwrap().world_json)
            .unwrap();
    assert_eq!(save.creatures.len(), 1);
    let restored = save.restore_headless_world().unwrap();
    let organism = restored.organism_registry().iter().next().unwrap();
    assert_eq!(
        &organism.phenotype().brain_genome,
        baseline.handoff.compiler_inputs.genome()
    );
    let event = serde_json::json!({"kind":"continuous-terminal-v1","terminal":true,
        "session_nonce":nonce,"source_world_seed":baseline.seed,
        "organism_id":save.creatures[0].organism_id.raw(),"world_tick":save.world.tick.raw()+7,
        "terminal_death_tick":save.world.tick.raw()+7});
    if let Some(stage) = injected_failure {
        let window = run.join("live/window-00000000");
        fs::create_dir_all(&window).unwrap();
        fs::write(
            window.join("unretained-replay"),
            b"preserve after failed cleanup",
        )
        .unwrap();
        let mut record = |value| persist_outcome(&run, &o, "continuous-terminal.bin", &value);
        let result: Result<()> = super::super::super::continuous_world::record_terminal_then(
            event.clone(),
            &mut record,
            || {
                assert_eq!(
                    read_outcome(&run, &o, "continuous-terminal.bin").unwrap(),
                    Some(event.clone())
                );
                Err(std::io::Error::other(format!("injected {stage} failure")).into())
            },
        );
        assert!(result.unwrap_err().to_string().contains(stage));
        assert_eq!(
            fs::read(window.join("unretained-replay")).unwrap(),
            b"preserve after failed cleanup"
        );
        // The real producer returns its postprocessing error before the final
        // result callback; committed terminal authority alone must be enough.
        assert!(!run.join("continuous-result.bin").exists());
    } else {
        persist_outcome(&run, &o, "continuous-terminal.bin", &event).unwrap();
        persist_outcome(
            &run,
            &o,
            "continuous-result.bin",
            &serde_json::json!({"terminal":true}),
        )
        .unwrap();
    }
    let report = recover_resident_serial_checkpoint(&store).unwrap();
    assert_eq!(report["terminal_event"], event);
    if injected_failure.is_some() {
        assert!(report["producer_result"].is_null());
    }
    assert_eq!(report["retained_session_updates"], 0);
    assert_eq!(report["retained_session_trained_rows"], 0);
    let candidate = Path::new(report["candidate"].as_str().unwrap());
    let retired = read_current_training_checkpoint(candidate).unwrap();
    assert!(retired.terminal_disposition.is_some());
    assert_eq!(retired.handoff, baseline.handoff);
    assert_eq!(retired.life, baseline.life);
    assert_eq!(retired.seeds, wal.seeds);
    assert!(retired.require_living_continuation().is_err());
    let mut changed = retired.clone();
    changed.handoff.actor.first_moment[0] += 0.001;
    assert!(changed.encode().is_err());
    let mut changed = retired.clone();
    changed
        .terminal_disposition
        .as_mut()
        .unwrap()
        .terminal_event["organism_id"] = 999.into();
    assert!(changed.encode().is_err());
    fs::copy(candidate, &current).unwrap();
    assert_eq!(
        recover_resident_serial_checkpoint(&store).unwrap()["already_published_current"],
        true
    );
    // Retirement staging happens while the intact continuous owner still
    // exists. A header shared with the legacy format must use that owner.
    fs::write(store.join(RETIREMENT_NEXT), b"ALIF").unwrap();
    assert_eq!(
        finish_resident_serial_run(&store).unwrap()["cleanup_completed"],
        true
    );
    assert!(!run.exists());
    assert!(read_current_training_checkpoint(&current)
        .unwrap()
        .terminal_disposition
        .is_some());
    let copied_store = fixture.root.join("different-lane");
    fs::create_dir(&copied_store).unwrap();
    let copied = copied_store.join("current.alife-checkpoint");
    fs::copy(&current, &copied).unwrap();
    let copied_before = fs::read(&copied).unwrap();
    let mut next = request;
    next.seed_range = [nonce + 1, nonce + 2];
    assert!(run_resident_continuous_care(&copied_store, next)
        .unwrap_err()
        .to_string()
        .contains("terminal"));
    assert!(!copied_store.join(".run").exists());
    assert!(!copied_store.join(INITIALIZATION).exists());
    let serial = ResidentSerialRequest {
        schema: 1,
        seed_range: [nonce + 1, nonce + 2],
        rows: vec![ResidentSerialRow {
            lesson: FoundationTeacherLesson::Feeding,
            decisions: 16,
            food_need_condition: Default::default(),
        }],
        checkpoint_every_episodes: 1,
        sampling_temperature: 1.,
        collection_cutoff_unix_seconds: now + 3600,
        max_temporary_bytes: 2 * 1024 * 1024 * 1024,
    };
    assert!(run_resident_serial_care(&copied_store, serial)
        .unwrap_err()
        .to_string()
        .contains("terminal"));
    assert!(!copied_store.join(".run").exists());
    assert!(!copied_store.join(INITIALIZATION).exists());
    assert!(
        materialize_current_training_life(&retired, &copied_store.join("private-life")).is_err()
    );
    assert!(advance_current_training_checkpoint(&retired, &fixture.target).is_err());
    assert_eq!(fs::read(&copied).unwrap(), copied_before);
    // Optional output is a disposable synthetic CPU fixture for actual CLI gates.
    if injected_failure.is_none() {
        explicit_training_branch_case(&fixture.root, &copied, &retired, nonce, &baseline);
        if let Some(path) = std::env::var_os("ALIFE_CONTINUOUS_TERMINAL_FIXTURE_OUTPUT") {
            write_new(Path::new(&path), &copied_before).unwrap();
        }
    }
}

fn explicit_training_branch_case(
    root: &Path,
    source: &Path,
    retired: &CurrentTrainingCheckpointV1,
    old_nonce: u64,
    original_baseline: &CurrentTrainingCheckpointV1,
) {
    let original = fs::read(source).unwrap();
    let nonce = old_nonce + 1; // The actual first reserved-unused tail.
    assert!(original_baseline
        .begin_training_attempt_from_terminal(&original_baseline.encode().unwrap(), nonce)
        .is_err());
    for invalid in [0, retired.seed, old_nonce, nonce + 1] {
        assert!(retired
            .begin_training_attempt_from_terminal(&original, invalid)
            .is_err());
    }
    let branch = retired
        .begin_training_attempt_from_terminal(&original, nonce)
        .unwrap();
    assert_eq!(fs::read(source).unwrap(), original);
    assert_eq!(branch.handoff, retired.handoff);
    assert_eq!(branch.life, retired.life);
    assert_eq!(branch.seeds, retired.seeds); // No reserve or consume at reset.
    assert_eq!(branch.same_individual, retired.same_individual);
    assert!(branch.terminal_disposition.is_none());
    let origin = branch.training_attempt_origin.as_ref().unwrap();
    assert_eq!(origin.attempt_nonce, nonce);
    assert_eq!(origin.source_current_sha256, hash(&original));
    assert_eq!(
        Some(&origin.source_terminal),
        retired.terminal_disposition.as_ref()
    );
    assert!(origin.previous.is_empty());
    assert!(branch.require_training_session_nonce(nonce).is_ok());
    assert!(branch.require_training_session_nonce(nonce + 1).is_err());
    assert!(branch.require_fresh_cohort_continuation().is_err());
    assert_eq!(
        CurrentTrainingCheckpointV1::decode(&branch.encode().unwrap()).unwrap(),
        branch
    );
    let mut changed_heads = branch.clone();
    changed_heads.handoff.actor.first_moment[0] += 0.001;
    assert!(changed_heads.encode().is_err());
    let mut changed = branch.clone();
    changed
        .training_attempt_origin
        .as_mut()
        .unwrap()
        .source_terminal
        .terminal_event["world_tick"] = 0.into();
    assert!(changed.encode().is_err());

    let store = root.join("explicit-training-branch");
    fs::create_dir(&store).unwrap();
    let store = store.canonicalize().unwrap();
    let current = store.join("current.alife-checkpoint");
    let bytes = branch.encode().unwrap();
    fs::write(&current, &bytes).unwrap();
    assert_eq!(read_current_training_checkpoint(&current).unwrap(), branch);
    let now = now_seconds().unwrap();
    let mut request = ResidentContinuousRequest {
        schema: 1,
        seed_range: [nonce + 1, nonce + 2],
        total_trained_decisions: 100,
        decisions_per_update: 10,
        snapshot_every_updates: 1,
        sampling_temperature: 1.,
        collection_cutoff_unix_seconds: now + 3600,
        finalization_deadline_unix_seconds: now + 7200,
        max_temporary_bytes: 2 * 1024 * 1024 * 1024,
    };
    assert!(run_resident_continuous_care(&store, request.clone())
        .unwrap_err()
        .to_string()
        .contains("attempt nonce"));
    assert!(!store.join(".run").exists());
    assert!(!store.join(INITIALIZATION).exists());
    // Simulate the real dispatch WAL boundary, without a world/GPU execution.
    // Recovery must retain all acquired state and origin, and consume only nonce.
    request.seed_range = [nonce, nonce + 1];
    let owner = Owner {
        schema: 1,
        kind: KIND.into(),
        store: store.clone(),
        pid: std::process::id(),
        baseline_sha256: hash(&bytes),
        request_sha256: hash(&serde_json::to_vec(&request).unwrap()),
        request: request.clone(),
    };
    let run = store.join(".run");
    fs::create_dir(&run).unwrap();
    write_new(&run.join("run.json"), &serde_json::to_vec(&owner).unwrap()).unwrap();
    write_new(&run.join("baseline.alife-checkpoint"), &bytes).unwrap();
    let mut wal = initial_wal(&owner, &branch).unwrap();
    wal.consumed = true;
    wal.seeds = session_seeds(&branch.seeds, &request, true).unwrap();
    write_metadata(&run.join("seed-wal.bin"), WAL, &wal).unwrap();
    write_metadata(
        &run.join("continuous-frontier.bin"),
        FRONTIER,
        &initial_frontier(&owner),
    )
    .unwrap();
    let report = recover_resident_serial_checkpoint(&store).unwrap();
    let successor =
        read_current_training_checkpoint(Path::new(report["candidate"].as_str().unwrap())).unwrap();
    assert_eq!(
        successor.training_attempt_origin,
        branch.training_attempt_origin
    );
    assert_eq!(successor.handoff, branch.handoff);
    assert_eq!(successor.life, branch.life);
    assert_eq!(successor.seeds.pending_range, Some([nonce + 1, nonce + 1]));
    assert!(successor.require_training_session_nonce(nonce + 1).is_ok());
    assert_eq!(report["seeds_consumed"], 1);
    // A later observed terminal retains this origin rather than erasing it.
    let mut event = origin.source_terminal.terminal_event.clone();
    event["session_nonce"] = nonce.into();
    let terminal = successor
        .with_terminal_disposition(&hash(&bytes), &owner.request_sha256, nonce, event)
        .unwrap();
    let second = terminal
        .begin_training_attempt_from_terminal(&terminal.encode().unwrap(), nonce + 1)
        .unwrap();
    let previous = &second.training_attempt_origin.as_ref().unwrap().previous;
    assert_eq!(previous.len(), 1);
    assert_eq!(previous[0].record_sha256, origin.record_sha256);
    assert_eq!(previous[0].source_terminal, origin.source_terminal);
    assert_eq!(second.seeds, terminal.seeds);
    let mut altered_ancestry = second.clone();
    altered_ancestry
        .training_attempt_origin
        .as_mut()
        .unwrap()
        .previous[0]
        .source_terminal
        .terminal_event["world_tick"] = 0.into();
    assert!(altered_ancestry.encode().is_err());
    repeated_training_attempt_roundtrips(second, &owner.request_sha256);
    assert_eq!(fs::read(source).unwrap(), original);
}

fn repeated_training_attempt_roundtrips(
    mut current: CurrentTrainingCheckpointV1,
    request_sha256: &str,
) {
    // More retries than serde_json's depth limit. These are synthetic terminal
    // observations, never executions or claims that the original life survived.
    let handoff = current.handoff.clone();
    let life = current.life.clone();
    let first = current.training_attempt_origin.as_ref().unwrap().previous[0].clone();
    for _ in 0..140 {
        let origin = current.training_attempt_origin.as_ref().unwrap();
        let nonce = origin.attempt_nonce;
        let mut event = origin.source_terminal.terminal_event.clone();
        event["session_nonce"] = nonce.into();
        if current.seeds.pending_range.is_none() {
            current.seeds.reserve(nonce, nonce).unwrap();
        }
        current.seeds.consume(nonce).unwrap();
        let before = current.encode().unwrap();
        let terminal = current
            .with_terminal_disposition(&hash(&before), request_sha256, nonce, event)
            .unwrap();
        assert!(terminal.require_living_continuation().is_err());
        let source = terminal.encode().unwrap();
        let next_nonce = terminal.seeds.pending_range.map_or_else(
            || {
                terminal
                    .seeds
                    .excluded_ranges
                    .iter()
                    .map(|r| r[1])
                    .max()
                    .unwrap()
                    + 1
            },
            |r| r[0],
        );
        let next = terminal
            .begin_training_attempt_from_terminal(&source, next_nonce)
            .unwrap();
        assert_eq!(next.seeds, terminal.seeds);
        let encoded = next.encode().unwrap();
        current = CurrentTrainingCheckpointV1::decode(&encoded).unwrap();
        assert_eq!(current, next);
        assert_eq!(current.handoff, handoff);
        assert_eq!(current.life, life);
        assert_eq!(
            current.training_attempt_origin.as_ref().unwrap().previous[0],
            first
        );
        assert_eq!(terminal.encode().unwrap(), source);
    }
    let origin = current.training_attempt_origin.as_ref().unwrap();
    assert_eq!(origin.previous.len(), 141);
    let mut reordered = current.clone();
    reordered
        .training_attempt_origin
        .as_mut()
        .unwrap()
        .previous
        .swap(0, 1);
    assert!(reordered.encode().is_err());
}
