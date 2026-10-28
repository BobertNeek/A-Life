use super::*;

fn seeds() -> CheckpointSeedsV1 {
    CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: vec![[1, u64::MAX]],
        pending_range: None,
    }
}

fn final_private_fixture(source: &Path) {
    portable_handoff::tests::private_fixture(source);
    // Device-free fixture only: its persisted private world is at tick zero.
    let path = source.join("cycle.json");
    let mut receipt: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    receipt.world_ticks_elapsed = 0;
    std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}

#[test]
fn throughput_frontier_migration_preserves_full_admission_and_final_boundary() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            final_private_fixture,
        );
    let expected = migrate_current_training_checkpoint(&f.target, seeds()).unwrap();
    let inspected = inspect_native_training_frontier(&f.target, 5124).unwrap();
    let migrated = migrate_verified_native_frontier(&f.target, 5124, seeds()).unwrap();
    assert_eq!(migrated.encode().unwrap(), expected.encode().unwrap());
    assert_eq!(inspected["policy_version"], migrated.handoff.policy_version);
    assert_eq!(inspected["final_private_world_tick"], 0);
    assert!(migrate_verified_native_frontier(&f.target, 5123, seeds()).is_err());

    // Warm admission must not conceal a changed on-disk private asset.
    let asset = f.target.join("world-assets/private-sidecar.bin");
    let original = std::fs::read(&asset).unwrap();
    std::fs::write(&asset, b"changed-private-state").unwrap();
    assert!(inspect_native_training_frontier(&f.target, 5124).is_err());
    assert!(migrate_verified_native_frontier(&f.target, 5124, seeds()).is_err());
    std::fs::write(&asset, original).unwrap();
    migrate_verified_native_frontier(&f.target, 5124, seeds()).unwrap();

    // Keep the legacy inspector's explicit final-collection-boundary rejection,
    // including when the four native head files still pass their own admission.
    let path = f.source.join("cycle.json");
    let mut receipt: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    receipt.world_ticks_elapsed = 1;
    std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert!(crate::inspect_foundation_checkpoint(&f.source, 5124).is_ok());
    assert!(migrate_verified_native_frontier(&f.source, 5124, seeds())
        .unwrap_err()
        .to_string()
        .contains("final collection boundary"));
}

#[test]
fn throughput_private_validation_cache_preserves_quota_high_water_and_corruption_checks() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            final_private_fixture,
        );
    let checkpoint = migrate_current_training_checkpoint(&f.target, seeds()).unwrap();
    let snapshot = checkpoint.life.as_ref().unwrap();
    let materialized_bytes = snapshot.world_json.len() as u64
        + snapshot.sleep_journal_json.len() as u64
        + snapshot
            .assets_base64
            .keys()
            .map(|name| snapshot.asset_bytes(name).unwrap().len() as u64)
            .sum::<u64>();
    let root = f.root.join("validation-budget");
    std::fs::create_dir(&root).unwrap();
    let limit = materialized_bytes * 2 + 4096;
    let budget = alife_storage::OutputBudget::register(&root, limit).unwrap();
    let _scope = validation_scope(&root).unwrap();
    let workspace = root.join("alife-current-private-validation-v1");

    // Admission was already warmed without this budget. A new budget must still
    // observe a real materialization high-water mark before it may reuse a hit.
    life::validate(snapshot).unwrap();
    let first = budget.usage().unwrap();
    assert!(first.used > 0, "the owned lease remains accounted");
    assert!(first.high_water >= first.used + materialized_bytes);
    assert!(!workspace.join("work").exists());
    life::validate(snapshot).unwrap();
    assert_eq!(budget.usage().unwrap(), first);

    // A cached snapshot cannot borrow capacity held for recovery. Refusal must
    // leave the lease and accounting intact and remove owned temporary work.
    let held = budget
        .reserve(limit - first.used - materialized_bytes + 1)
        .unwrap();
    assert!(life::validate(snapshot).is_err());
    assert_eq!(budget.usage().unwrap().used, first.used);
    assert_eq!(budget.usage().unwrap().high_water, first.high_water);
    assert!(!workspace.join("work").exists());
    drop(held);
    life::validate(snapshot).unwrap();

    for changed in 0..3 {
        let mut corrupt = snapshot.clone();
        match changed {
            0 => corrupt.world_json = "not a world".into(),
            1 => corrupt.sleep_journal_json = "not a journal".into(),
            _ => corrupt.insert_asset("private-sidecar.bin".into(), b"changed-private-state"),
        }
        assert!(life::validate(&corrupt).is_err());
        assert_eq!(budget.usage().unwrap().used, first.used);
        assert!(!workspace.join("work").exists());
        life::validate(snapshot).unwrap();
    }

    // A cache hit is not permission to sweep unknown files in the scratch root.
    let foreign = workspace.join("unknown-preserve.bin");
    alife_storage::write(&foreign, b"preserve").unwrap();
    assert!(life::validate(snapshot).is_err());
    assert_eq!(std::fs::read(&foreign).unwrap(), b"preserve");
    alife_storage::remove_file(&foreign).unwrap();
    life::validate(snapshot).unwrap();
    assert!(budget.usage().unwrap().high_water <= limit);
}
