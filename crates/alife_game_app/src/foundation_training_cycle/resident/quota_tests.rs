use super::*;

fn root(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "resident-budget-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    path
}
fn request(first: u64) -> ResidentSerialRequest {
    ResidentSerialRequest {
        schema: 1,
        seed_range: [first, first + 4],
        rows: vec![
            ResidentSerialRow {
                lesson: FoundationTeacherLesson::Feeding,
                decisions: 16,
                food_need_condition: Default::default(),
            };
            3
        ],
        checkpoint_every_episodes: 3,
        sampling_temperature: 1.0,
        collection_cutoff_unix_seconds: now_seconds().unwrap() + 600,
        max_temporary_bytes: RECOVERY_HEADROOM + 1024 * 1024,
    }
}

#[test]
fn output_budget_too_small_refuses_before_wal_or_baseline_access() {
    let store = root("too-small");
    let current = store.join("current.alife-checkpoint");
    fs::write(&current, b"not opened: validation must fail first").unwrap();
    let mut r = request(100);
    r.max_temporary_bytes = RECOVERY_HEADROOM;
    assert!(run_resident_serial_care(&store, r).is_err());
    assert!(!store.join(".run").exists());
    assert_eq!(
        fs::read(&current).unwrap(),
        b"not opened: validation must fail first"
    );
    fs::remove_dir_all(store).unwrap();
}

#[test]
fn output_budget_exhaustion_still_recovers_consumed_seeds_and_exact_full_baseline() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            super::super::portable_handoff::tests::private_fixture,
        );
    let source_seeds = CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: vec![[1, u64::MAX]],
        pending_range: None,
    };
    let mut baseline = migrate_current_training_checkpoint(&f.target, source_seeds).unwrap();
    baseline.seeds = CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: vec![[1, baseline.seed]],
        pending_range: None,
    };
    let store = f.root.join("budget-store");
    fs::create_dir(&store).unwrap();
    let current = store.join("current.alife-checkpoint");
    let original = baseline.encode().unwrap();
    fs::write(&current, &original).unwrap();
    let first = baseline.seed + 100;
    let r = request(first);
    stage_resident_loss_fixture(&store, r.clone(), 2).unwrap();
    let run = store.join(".run");
    let budget = alife_storage::OutputBudget::register(&run, r.max_temporary_bytes).unwrap();
    let reserve = budget.reserve(RECOVERY_HEADROOM).unwrap();
    let available = r.max_temporary_bytes - budget.usage().unwrap().used - RECOVERY_HEADROOM;
    let episode = run.join("episode-00001");
    fs::create_dir(&episode).unwrap();
    alife_storage::write(episode.join("partial"), vec![0x75; available as usize]).unwrap();
    assert!(alife_storage::write(episode.join("denied"), b"x").is_err());
    assert!(budget.usage().unwrap().high_water <= r.max_temporary_bytes);
    drop(reserve);
    drop(budget);
    let report = recover_resident_serial_checkpoint(&store).unwrap();
    let recovered =
        read_current_training_checkpoint(Path::new(report["candidate"].as_str().unwrap())).unwrap();
    assert_eq!(recovered.handoff, baseline.handoff);
    assert_eq!(recovered.life, baseline.life);
    assert_eq!(recovered.seeds.pending_range, Some([first + 2, first + 4]));
    assert!(recovered.seeds.contains(first));
    assert!(recovered.seeds.contains(first + 1));
    assert_eq!(fs::read(&current).unwrap(), original);
    assert!(logical_bytes(&run).unwrap() <= r.max_temporary_bytes);
    assert!(run
        .join("alife-current-private-validation-v1/lease")
        .is_file());
    assert!(!run
        .join("alife-current-private-validation-v1/work")
        .exists());
}
