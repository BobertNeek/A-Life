use super::*;

#[test]
fn terminal_recovery_retains_exact_brain_and_cannot_resume_a_copied_current() {
    let fixture =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            crate::foundation_training_cycle::portable_handoff::tests::private_fixture,
        );
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
    let event = serde_json::json!({"kind":"continuous-terminal-v1","terminal":true,
        "session_nonce":nonce,"source_world_seed":baseline.seed,
        "organism_id":save.creatures[0].organism_id.raw(),"world_tick":save.world.tick.raw()+7,
        "terminal_death_tick":save.world.tick.raw()+7});
    persist_outcome(&run, &o, "continuous-terminal.bin", &event).unwrap();
    persist_outcome(
        &run,
        &o,
        "continuous-result.bin",
        &serde_json::json!({"terminal":true}),
    )
    .unwrap();
    let report = recover_resident_serial_checkpoint(&store).unwrap();
    assert_eq!(report["terminal_event"], event);
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
    if let Some(path) = std::env::var_os("ALIFE_CONTINUOUS_TERMINAL_FIXTURE_OUTPUT") {
        write_new(Path::new(&path), &copied_before).unwrap();
    }
}
