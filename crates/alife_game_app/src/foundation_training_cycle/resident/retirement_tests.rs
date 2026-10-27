use super::*;

fn published_store(test: impl FnOnce(&Path)) {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            super::super::portable_handoff::tests::private_fixture,
        );
    let mut current = migrate_current_training_checkpoint(
        &f.target,
        CheckpointSeedsV1 {
            historical_freshness: "unknown".into(),
            excluded_ranges: vec![[1, u64::MAX]],
            pending_range: None,
        },
    )
    .unwrap();
    current.seeds = CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: vec![[1, current.seed]],
        pending_range: None,
    };
    let store = f.root.join("retirement-store");
    fs::create_dir(&store).unwrap();
    fs::write(
        store.join("current.alife-checkpoint"),
        current.encode().unwrap(),
    )
    .unwrap();
    let first = current.seed + 100;
    let request = ResidentSerialRequest {
        schema: 1,
        seed_range: [first, first + 4],
        rows: vec![
            ResidentSerialRow {
                lesson: FoundationTeacherLesson::Feeding,
                decisions: 16,
                food_need_condition: Default::default()
            };
            3
        ],
        checkpoint_every_episodes: 3,
        sampling_temperature: 1.0,
        collection_cutoff_unix_seconds: 1,
        max_temporary_bytes: 1024 * 1024 * 1024,
    };
    stage_resident_loss_fixture(&store, request, 2).unwrap();
    let report = recover_resident_serial_checkpoint(&store).unwrap();
    fs::copy(
        report["candidate"].as_str().unwrap(),
        store.join("current.alife-checkpoint"),
    )
    .unwrap();
    test(&store);
}

#[test]
fn output_budget_retirement_retry_after_binding_removal_preserves_conflicts_and_current() {
    let bindings = [
        "run.json",
        "seed-wal.bin",
        "resident-frontier.bin",
        "recovery-intent.bin",
        "recovered-current.alife-checkpoint",
    ];
    for removed in 1..=bindings.len() {
        published_store(|store| {
            let current = store.join("current.alife-checkpoint");
            let original = fs::read(&current).unwrap();
            assert!(finish_resident_with_remove(store, |run| {
                for name in &bindings[..removed] {
                    fs::remove_file(run.join(name))?;
                }
                Err(std::io::Error::other(
                    "injected interruption after binding deletion",
                ))
            })
            .is_err());
            assert!(store.join(RETIREMENT).is_file());
            assert!(store.join(".run").is_dir());
            fs::write(store.join(".run/unknown-writer"), b"preserve").unwrap();
            assert!(finish_resident_serial_run(store).is_err());
            assert_eq!(
                fs::read(store.join(".run/unknown-writer")).unwrap(),
                b"preserve"
            );
            fs::remove_file(store.join(".run/unknown-writer")).unwrap();
            let mut changed = original.clone();
            changed.push(0);
            fs::write(&current, &changed).unwrap();
            assert!(finish_resident_serial_run(store).is_err());
            assert_eq!(fs::read(&current).unwrap(), changed);
            fs::write(&current, &original).unwrap();
            // Recover routes the committed retirement even though its old WAL and
            // frontier no longer exist. It dispatches no world and returns one object.
            assert_eq!(
                recover_resident_serial_checkpoint(store).unwrap()["cleanup_completed"],
                true
            );
            assert!(!store.join(".run").exists());
            assert!(!store.join(RETIREMENT).exists());
            assert_eq!(fs::read(&current).unwrap(), original);
            assert_eq!(
                finish_resident_serial_run(store).unwrap()["already_clean"],
                true
            );
        });
    }
}

#[test]
fn output_budget_retirement_retry_after_tree_disappeared_never_recreates_it() {
    published_store(|store| {
        let original = fs::read(store.join("current.alife-checkpoint")).unwrap();
        assert!(finish_resident_with_remove(store, |run| {
            alife_storage::remove_dir_all(run)?;
            Err(std::io::Error::other(
                "injected crash before marker removal",
            ))
        })
        .is_err());
        assert!(!store.join(".run").exists());
        assert!(store.join(RETIREMENT).is_file());
        let report = finish_resident_serial_run(store).unwrap();
        assert_eq!(report["retirement_completed"], true);
        assert!(!store.join(".run").exists());
        assert!(!store.join(RETIREMENT).exists());
        assert_eq!(
            fs::read(store.join("current.alife-checkpoint")).unwrap(),
            original
        );
    });
}

#[test]
fn output_budget_retirement_partial_stage_revalidates_but_complete_conflict_is_preserved() {
    published_store(|store| {
        let store = store.canonicalize().unwrap();
        let current = store.join("current.alife-checkpoint");
        let original = fs::read(&current).unwrap();
        let hash = hash(&original);
        let (record, budget) = prepare_retirement(&store, &hash).unwrap();
        drop(budget);
        let mut conflict = record.clone();
        conflict.request_sha256 = "0".repeat(64);
        let conflicting = envelope(RET_MAGIC, &conflict).unwrap();
        fs::write(store.join(RETIREMENT_NEXT), &conflicting).unwrap();
        assert!(finish_resident_serial_run(&store).is_err());
        assert_eq!(fs::read(store.join(RETIREMENT_NEXT)).unwrap(), conflicting);
        assert!(!store.join(RETIREMENT).exists());
        assert!(store.join(".run/run.json").exists());
        let bytes = envelope(RET_MAGIC, &record).unwrap();
        fs::write(store.join(RETIREMENT_NEXT), &bytes[..24]).unwrap();
        assert_eq!(
            finish_resident_serial_run(&store).unwrap()["retirement_completed"],
            true
        );
        assert!(!store.join(RETIREMENT_NEXT).exists());
        assert!(!store.join(RETIREMENT).exists());
        assert_eq!(fs::read(&current).unwrap(), original);
    });
}
