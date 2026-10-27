use super::*;

fn fixture(test: impl FnOnce(&Path, &[u8], ResidentSerialRequest)) {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            super::super::super::portable_handoff::tests::private_fixture,
        );
    let mut baseline = migrate_current_training_checkpoint(
        &f.target,
        CheckpointSeedsV1 {
            historical_freshness: "unknown".into(),
            excluded_ranges: vec![[1, u64::MAX]],
            pending_range: None,
        },
    )
    .unwrap();
    baseline.seeds = CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: vec![[1, baseline.seed]],
        pending_range: None,
    };
    let first = baseline.seed + 100;
    let store = f.root.join("initialization-store");
    fs::create_dir(&store).unwrap();
    let bytes = baseline.encode().unwrap();
    fs::write(store.join("current.alife-checkpoint"), &bytes).unwrap();
    let request = ResidentSerialRequest {
        schema: 1,
        seed_range: [first, first + 2],
        rows: vec![
            ResidentSerialRow {
                lesson: FoundationTeacherLesson::Feeding,
                decisions: 16,
                food_need_condition: Default::default(),
            };
            2
        ],
        checkpoint_every_episodes: 2,
        sampling_temperature: 1.0,
        collection_cutoff_unix_seconds: now_seconds().unwrap() + 600,
        max_temporary_bytes: 1024 * 1024 * 1024,
    };
    test(&store, &bytes, request);
}
fn interrupt(store: &Path, request: &ResidentSerialRequest, phase: &str) {
    let mut observed = false;
    assert!(
        run_resident_with_startup_observer(store, request.clone(), |step, _| {
            if step == phase {
                observed = true;
                return Err("injected pre-dispatch interruption".into());
            }
            Ok(())
        })
        .is_err()
    );
    assert!(observed, "startup did not reach {phase}");
}
fn unchanged(store: &Path, original: &[u8]) {
    assert_eq!(
        fs::read(store.join("current.alife-checkpoint")).unwrap(),
        original
    );
    assert!(!store.join(".run").exists());
    assert!(!store.join(INITIALIZATION).exists());
    assert!(!store.join(INITIALIZATION_NEXT).exists());
}

#[test]
fn output_budget_initialization_every_commit_boundary_aborts_without_dispatch_or_seed_changes() {
    fixture(|store, original, request| {
        for phase in [
            "initialization-staged",
            "initialization-committed",
            "run-created",
            "baseline-admitted",
            "owner-written",
            "wal-written",
            "frontier-written",
        ] {
            interrupt(store, &request, phase);
            let report = recover_resident_serial_checkpoint(store).unwrap();
            assert_eq!(report["initialization_aborted"], true);
            assert_eq!(report["initialization_binding_verified"], true);
            assert_eq!(report["seeds_consumed"], 0);
            assert_eq!(report["new_request_allocation_committed"], false);
            unchanged(store, original);
            assert_eq!(
                finish_resident_serial_run(store).unwrap()["already_clean"],
                true
            );
        }
        // The committed initialization marker is gone before a first dispatch.
        // A crash here uses the ordinary fully initialized WAL=0 recovery.
        if let Some(output) = std::env::var_os("ALIFE_RESIDENT_INITIALIZATION_FIXTURE_OUTPUT") {
            let output = PathBuf::from(output);
            fs::create_dir(&output).unwrap();
            fs::write(output.join("current.alife-checkpoint"), original).unwrap();
            interrupt(&output, &request, "frontier-written");
        }
        interrupt(store, &request, "initialization-complete");
        let result = recover_resident_serial_checkpoint(store).unwrap();
        assert!(result.get("initialization_aborted").is_none());
        assert_eq!(
            result["seeds"]["pending_range"],
            serde_json::json!(request.seed_range)
        );
        fs::copy(
            result["candidate"].as_str().unwrap(),
            store.join("current.alife-checkpoint"),
        )
        .unwrap();
        finish_resident_serial_run(store).unwrap();
    });
}

#[test]
fn output_budget_initialization_torn_first_stage_never_claims_binding_or_foreign_run() {
    fixture(|store, original, request| {
        for length in [0, 8, 20, 55, 56, 100] {
            interrupt(store, &request, "initialization-staged");
            let path = store.join(INITIALIZATION_NEXT);
            let bytes = fs::read(&path).unwrap();
            fs::write(&path, &bytes[..length]).unwrap();
            if length == 20 {
                fs::create_dir(store.join(".run")).unwrap();
                assert!(recover_resident_serial_checkpoint(store).is_err());
                assert!(store.join(".run").is_dir());
                assert!(path.is_file());
                fs::remove_dir(store.join(".run")).unwrap();
            }
            let result = recover_resident_serial_checkpoint(store).unwrap();
            assert_eq!(result["initialization_binding_verified"], false);
            unchanged(store, original);
        }
        fs::write(store.join(INITIALIZATION_NEXT), b"foreign complete record").unwrap();
        assert!(finish_resident_serial_run(store).is_err());
        assert_eq!(
            fs::read(store.join(INITIALIZATION_NEXT)).unwrap(),
            b"foreign complete record"
        );
        fs::remove_file(store.join(INITIALIZATION_NEXT)).unwrap();
        interrupt(store, &request, "initialization-staged");
        let bytes = fs::read(store.join(INITIALIZATION_NEXT)).unwrap();
        let mut corrupt = bytes.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        fs::write(store.join(INITIALIZATION_NEXT), &corrupt).unwrap();
        assert!(finish_resident_serial_run(store).is_err());
        assert_eq!(fs::read(store.join(INITIALIZATION_NEXT)).unwrap(), corrupt);
        fs::write(store.join(INITIALIZATION_NEXT), &bytes).unwrap();
        finish_resident_serial_run(store).unwrap();
        unchanged(store, original);
    });
}

#[test]
fn output_budget_initialization_refuses_changed_current_unknown_files_and_consumed_wal() {
    fixture(|store, original, request| {
        interrupt(store, &request, "frontier-written");
        let run = store.join(".run");
        let foreign = run.join("foreign");
        fs::write(&foreign, b"preserve").unwrap();
        assert!(finish_resident_serial_run(store).is_err());
        assert_eq!(fs::read(&foreign).unwrap(), b"preserve");
        fs::remove_file(foreign).unwrap();
        fs::write(store.join("current.alife-checkpoint"), b"changed Current").unwrap();
        assert!(recover_resident_serial_checkpoint(store).is_err());
        assert!(store.join(INITIALIZATION).is_file());
        fs::write(store.join("current.alife-checkpoint"), original).unwrap();
        let wal_path = run.join("seed-wal.bin");
        let before = fs::read(&wal_path).unwrap();
        let mut wal: SeedWal = read_metadata(&wal_path, WAL_MAGIC).unwrap();
        wal.seeds.consume(request.seed_range[0]).unwrap();
        wal.dispatched = 1;
        fs::write(&wal_path, envelope(WAL_MAGIC, &wal).unwrap()).unwrap();
        assert!(recover_resident_serial_checkpoint(store).is_err());
        assert_eq!(
            read_metadata::<SeedWal>(&wal_path, WAL_MAGIC)
                .unwrap()
                .dispatched,
            1
        );
        fs::write(wal_path, before).unwrap();
        finish_resident_serial_run(store).unwrap();
        unchanged(store, original);
    });
}

#[test]
fn output_budget_initialization_partial_metadata_and_private_scratch_are_bounded_owned_prefixes() {
    fixture(|store, original, request| {
        for name in [
            "run.json",
            "seed-wal.bin",
            "seed-wal.next",
            "resident-frontier.bin",
            "resident-frontier.next",
        ] {
            interrupt(store, &request, "frontier-written");
            let run = store.join(".run");
            let source = if name.ends_with("next") {
                name.replace(".next", ".bin")
            } else {
                name.into()
            };
            let bytes = fs::read(run.join(source)).unwrap();
            fs::write(run.join(name), &bytes[..bytes.len() / 2]).unwrap();
            finish_resident_serial_run(store).unwrap();
            unchanged(store, original);
        }
        interrupt(store, &request, "run-created");
        let work = store.join(".run").join(WORKSPACE).join("work");
        fs::create_dir_all(&work).unwrap();
        let baseline = alife_training::CurrentTrainingCheckpointV1::decode(original).unwrap();
        let life = baseline.life.unwrap();
        fs::write(work.parent().unwrap().join("lease"), &WORKSPACE_LEASE[..8]).unwrap();
        fs::write(work.join("world.json"), &life.world_json.as_bytes()[..20]).unwrap();
        fs::write(work.join("unknown"), b"preserve").unwrap();
        assert!(finish_resident_serial_run(store).is_err());
        fs::remove_file(work.join("unknown")).unwrap();
        fs::write(work.join("world.json"), b"changed").unwrap();
        assert!(finish_resident_serial_run(store).is_err());
        fs::write(work.join("world.json"), &life.world_json.as_bytes()[..20]).unwrap();
        finish_resident_serial_run(store).unwrap();
        unchanged(store, original);
    });
}

#[test]
fn output_budget_initialization_abort_retries_after_partial_deletion_and_tree_gone() {
    fixture(|store, original, request| {
        for gone in [false, true] {
            interrupt(store, &request, "frontier-written");
            let store = store.canonicalize().unwrap();
            assert!(abort_with_remove(&store, |run| {
                if gone {
                    alife_storage::remove_dir_all(run)?;
                } else {
                    alife_storage::remove_file(run.join("resident-frontier.bin"))?;
                }
                Err(std::io::Error::other("injected cleanup interruption"))
            })
            .is_err());
            assert!(store.join(INITIALIZATION).is_file());
            finish_resident_serial_run(&store).unwrap();
            unchanged(&store, original);
        }
    });
}

#[test]
fn output_budget_initialization_failed_admission_can_abort_without_certifying_invalid_baseline() {
    fixture(|store, original, request| {
        fs::write(store.join("current.alife-checkpoint"), b"invalid input").unwrap();
        assert!(run_resident_serial_care(store, request.clone()).is_err());
        assert!(store.join(INITIALIZATION).is_file());
        let result = finish_resident_serial_run(store).unwrap();
        assert_eq!(result["initialization_aborted"], true);
        unchanged(store, b"invalid input");
        fs::write(store.join("current.alife-checkpoint"), original).unwrap();
        let mut invalid_allocation = request.clone();
        invalid_allocation.seed_range = [1, 3];
        assert!(run_resident_serial_care(store, invalid_allocation).is_err());
        assert_eq!(
            finish_resident_serial_run(store).unwrap()["seeds_consumed"],
            0
        );
        unchanged(store, original);
        let mut reserved = alife_training::CurrentTrainingCheckpointV1::decode(original).unwrap();
        reserved
            .seeds
            .reserve(request.seed_range[0], request.seed_range[1])
            .unwrap();
        let reserved_bytes = reserved.encode().unwrap();
        fs::write(store.join("current.alife-checkpoint"), &reserved_bytes).unwrap();
        interrupt(store, &request, "frontier-written");
        finish_resident_serial_run(store).unwrap();
        unchanged(store, &reserved_bytes);
    });
}
