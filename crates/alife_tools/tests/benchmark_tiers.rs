use std::process::Command;

use alife_core::{
    BrainActivityPolicyV1, BrainCapacityClass, BrainClassId, SchemaKind, SchemaVersions,
    SensorProfile,
};
use alife_game_app::{
    compile_gpu_closed_loop_benchmark_phenotype, GpuClosedLoopBenchmarkTrialOptions,
};
use alife_gpu_backend::{GpuAdmissionReceipt, GpuRuntimeBudget, GpuRuntimeProfile};
use alife_tools::benchmark::gpu_closed_loop::{
    adapter_identity_digest, canonical_performance_targets_v1, is_lower_hex_oid, nearest_rank_p95,
    timestamp_ticks_to_ns, GpuBenchmarkEnvironmentReceipt, GpuBenchmarkStatus,
    GpuClosedLoopBenchmarkManifest, GpuClosedLoopBenchmarkProtocolV1, GpuClosedLoopBenchmarkRow,
    GpuPerformanceTargetRowV1, GpuPerformanceTargetsV1, GPU_BENCHMARK_UNAVAILABLE_ADMISSION,
    GPU_BENCHMARK_UNAVAILABLE_NO_ADAPTER, GPU_BENCHMARK_UNAVAILABLE_REQUIRED_CAPABILITY,
};
use alife_world::GpuBackendProvenanceSave;

fn adapter_fixture(device_id: u32) -> GpuBackendProvenanceSave {
    GpuBackendProvenanceSave {
        schema_version: 1,
        backend_api_raw: 1,
        vendor_id: 0x1002,
        device_id,
        backend_version_major: 29,
        backend_version_minor: 0,
        backend_version_patch: 3,
        adapter_name_len: 0,
        adapter_name_utf8: [0; 128],
        driver_digest: [11; 4],
        required_features_digest: [12; 4],
        required_limits_digest: [13; 4],
        available_features_digest: [14; 4],
        adapter_limits_digest: [15; 4],
    }
}

fn admitted_adapter_fixture(target: GpuPerformanceTargetRowV1) -> GpuBackendProvenanceSave {
    let capacity =
        BrainCapacityClass::production_for_id(BrainClassId(target.class_id_raw)).unwrap();
    let runtime = GpuRuntimeBudget::minimum_for_testing(
        GpuRuntimeProfile::production_v1(),
        capacity.execution(),
    )
    .unwrap();
    let mut adapter = adapter_fixture(0x744c);
    adapter.required_features_digest = runtime.required_features_digest().unwrap();
    adapter.required_limits_digest = runtime
        .required_limits_digest_for(capacity.execution())
        .unwrap();
    adapter.adapter_limits_digest = runtime.adapter_limits_digest;
    adapter
}

fn admission_fixture(capacity: &BrainCapacityClass, population: u32) -> GpuAdmissionReceipt {
    let runtime = GpuRuntimeBudget::minimum_for_testing(
        GpuRuntimeProfile::production_v1(),
        capacity.execution(),
    )
    .unwrap();
    let committed = u64::from(population);
    GpuAdmissionReceipt {
        schema_version: 1,
        runtime,
        logical_committed_bytes: committed,
        logical_available_bytes: runtime.logical_neural_heap_budget_bytes - committed,
        physical_allocated_bytes: committed,
        physical_unused_retained_bytes: 0,
        physical_shared_bytes: 0,
        physical_alignment_slack_bytes: 0,
        peak_logical_committed_bytes: committed,
        peak_physical_allocated_bytes: committed,
        live_brains: population,
        max_hot_brains: runtime.max_hot_brains,
        allocation_generation: 0,
        last_event: None,
    }
}

fn benchmark_row_fixture(
    target: GpuPerformanceTargetRowV1,
    status: GpuBenchmarkStatus,
    adapter: Option<GpuBackendProvenanceSave>,
) -> GpuClosedLoopBenchmarkRow {
    let protocol = GpuClosedLoopBenchmarkProtocolV1::canonical();
    let capacity =
        BrainCapacityClass::production_for_id(BrainClassId(target.class_id_raw)).unwrap();
    let sensor_profile = SensorProfile::try_from_raw(target.sensor_profile_id_raw).unwrap();
    let options = GpuClosedLoopBenchmarkTrialOptions {
        capacity,
        sensor_profile,
        population: target.population,
        fixture_seed: protocol.row_seed(
            target.class_id_raw,
            target.sensor_profile_id_raw,
            target.population,
        ),
        warmup_ticks: protocol.warmup_ticks,
        measured_ticks: protocol.measured_ticks,
    };
    let phenotype = compile_gpu_closed_loop_benchmark_phenotype(options).unwrap();
    let reason = match status {
        GpuBenchmarkStatus::Completed | GpuBenchmarkStatus::Missed => 0,
        GpuBenchmarkStatus::Unavailable { reason_code } => reason_code,
    };
    let executed = matches!(
        status,
        GpuBenchmarkStatus::Completed | GpuBenchmarkStatus::Missed
    );
    let sample_count = protocol.measured_ticks as usize;
    let samples = if executed {
        vec![1_u64; sample_count]
    } else {
        Vec::new()
    };
    let neural_ns = if executed {
        vec![2_u64; sample_count]
    } else {
        Vec::new()
    };
    let events = u64::from(protocol.measured_ticks) * u64::from(target.population);
    let mut row = GpuClosedLoopBenchmarkRow {
        schema_version: 1,
        class_id_raw: target.class_id_raw,
        sensor_profile_id_raw: target.sensor_profile_id_raw,
        sensor_profile_schema: SchemaVersions::current_for(SchemaKind::SensorProfile).raw(),
        sensory_abi_raw: SchemaVersions::CURRENT.sensory_abi.raw(),
        population: target.population,
        fixture_seed: options.fixture_seed,
        phenotype_hash: phenotype.manifest.phenotype_hash,
        phenotype_manifest: phenotype.manifest.clone(),
        phenotype_manifest_digest: phenotype.manifest.manifest_digest,
        capacity_digest: capacity.canonical_digest(),
        runtime_profile_digest: GpuRuntimeProfile::production_v1()
            .canonical_digest()
            .unwrap(),
        activity_policy_digest: BrainActivityPolicyV1::production_v1().policy_digest,
        protocol_digest: protocol.protocol_digest,
        target_p95_ns: target.target_p95_ns,
        measured_p95_ns: executed.then_some(2),
        timestamp_period_ns_q24: if executed { 1 << 24 } else { 0 },
        raw_inference_timestamp_ticks: samples.clone(),
        raw_plasticity_timestamp_ticks: samples,
        raw_neural_tick_ns: neural_ns,
        environment: GpuBenchmarkEnvironmentReceipt::new(reason, adapter).unwrap(),
        admission: executed.then(|| admission_fixture(&capacity, target.population)),
        gpu_selections: if executed { events } else { 0 },
        executed_actions: if executed { events } else { 0 },
        sealed_patches: if executed { events } else { 0 },
        learning_commits: if executed { events } else { 0 },
        distinct_selected_families: if executed { 2 } else { 0 },
        active_synapses: phenotype.active_synapses,
        status,
        row_digest: [0; 4],
    };
    row.seal_digest().unwrap();
    row
}

fn benchmark_manifest_fixture() -> (GpuPerformanceTargetsV1, GpuClosedLoopBenchmarkManifest) {
    let mut targets = canonical_performance_targets_v1();
    targets.rows.truncate(2);
    targets.seal_digest().unwrap();
    let adapter = adapter_fixture(0x744c);
    let rows = targets
        .rows
        .iter()
        .copied()
        .map(|target| {
            benchmark_row_fixture(
                target,
                GpuBenchmarkStatus::Unavailable {
                    reason_code: GPU_BENCHMARK_UNAVAILABLE_REQUIRED_CAPABILITY,
                },
                Some(adapter.clone()),
            )
        })
        .collect();
    let mut manifest = GpuClosedLoopBenchmarkManifest {
        schema_version: 1,
        git_commit: "a".repeat(40),
        source_tree_digest: "b".repeat(40),
        adapter_identity_digest_or_zero: adapter_identity_digest(&adapter).unwrap(),
        adapter: Some(adapter),
        protocol: GpuClosedLoopBenchmarkProtocolV1::canonical(),
        rows,
        manifest_digest: [0; 4],
    };
    manifest.seal_digest().unwrap();
    manifest.validate(&targets).unwrap();
    (targets, manifest)
}

#[test]
fn benchmark_cli_validates_canonical_manifest_without_requiring_targets_flag() {
    let (_, manifest) = benchmark_manifest_fixture();
    let root =
        std::env::temp_dir().join(format!("alife-benchmark-validator-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let manifest_path = root.join("benchmark.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_benchmark_tiers"))
        .arg("--validate")
        .arg(&manifest_path)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "validator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("validated"));
}

#[test]
fn benchmark_accepts_selected_rows_and_custom_budgets() {
    let (mut targets, mut manifest) = benchmark_manifest_fixture();
    targets.rows[0].target_p95_ns += 1;
    targets.seal_digest().unwrap();
    targets.validate().unwrap();
    manifest.rows.truncate(1);
    manifest.rows[0].target_p95_ns = targets.rows[0].target_p95_ns;
    manifest.rows[0].seal_digest().unwrap();
    manifest.seal_digest().unwrap();
    manifest.validate(&targets).unwrap();

    targets.rows.truncate(1);
    targets.seal_digest().unwrap();
    manifest.validate(&targets).unwrap();
    targets.rows.push(targets.rows[0]);
    targets.seal_digest().unwrap();
    assert!(targets.validate().is_err());
}

#[test]
fn population_scaling_reuses_one_class_profile_phenotype() {
    let protocol = GpuClosedLoopBenchmarkProtocolV1::canonical();
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        for sensor_profile in [
            SensorProfile::PrivilegedAffordanceV1,
            SensorProfile::GroundedObjectSlotsV1,
        ] {
            let class_id_raw = capacity.id().raw();
            let sensor_profile_id_raw = sensor_profile.raw();
            let compile = |population| {
                compile_gpu_closed_loop_benchmark_phenotype(GpuClosedLoopBenchmarkTrialOptions {
                    capacity,
                    sensor_profile,
                    population,
                    fixture_seed: protocol.row_seed(
                        class_id_raw,
                        sensor_profile_id_raw,
                        population,
                    ),
                    warmup_ticks: protocol.warmup_ticks,
                    measured_ticks: protocol.measured_ticks,
                })
                .unwrap()
            };
            let population_one = compile(1);
            let population_five_hundred = compile(500);
            assert_eq!(population_one, population_five_hundred);
        }
    }
}

#[test]
fn gpu_closed_loop_nearest_rank_p95_uses_exact_protocol_rank() {
    let mut samples = (0_u64..1_024).rev().collect::<Vec<_>>();
    assert_eq!(nearest_rank_p95(&mut samples).unwrap(), 972);
    assert_eq!(samples[972], 972);

    let mut equal = vec![41_u64; 1_024];
    assert_eq!(nearest_rank_p95(&mut equal).unwrap(), 41);
    assert!(nearest_rank_p95(&mut []).is_err());
    assert!(nearest_rank_p95(&mut vec![1; 1_023]).is_err());
}

#[test]
fn gpu_timestamp_conversion_is_checked_q24_round_half_up() {
    assert_eq!(timestamp_ticks_to_ns(1, 1 << 23).unwrap(), 1);
    assert_eq!(timestamp_ticks_to_ns(3, 1 << 23).unwrap(), 2);
    assert_eq!(timestamp_ticks_to_ns(7, 1 << 24).unwrap(), 7);
    assert!(timestamp_ticks_to_ns(0, 1 << 24).is_err());
    assert!(timestamp_ticks_to_ns(1, 0).is_err());

    let mut ordered = (0_u64..1_024).collect::<Vec<_>>();
    let mut permuted = ordered.clone();
    permuted.rotate_left(317);
    assert_eq!(
        nearest_rank_p95(&mut ordered).unwrap(),
        nearest_rank_p95(&mut permuted).unwrap()
    );
}

#[test]
fn completed_gpu_rows_prove_exact_populated_causal_work() {
    let target = canonical_performance_targets_v1().rows[0];
    let protocol = GpuClosedLoopBenchmarkProtocolV1::canonical();
    let row = benchmark_row_fixture(
        target,
        GpuBenchmarkStatus::Completed,
        Some(admitted_adapter_fixture(target)),
    );
    row.validate(&protocol, &target).unwrap();
    assert_eq!(row.raw_inference_timestamp_ticks.len(), 1_024);
    assert_eq!(row.raw_plasticity_timestamp_ticks.len(), 1_024);
    assert_eq!(row.raw_neural_tick_ns.len(), 1_024);
    assert_eq!(row.gpu_selections, 1_024);
    assert_eq!(row.gpu_selections, row.executed_actions);
    assert_eq!(row.executed_actions, row.sealed_patches);
    assert_eq!(row.learning_commits, row.sealed_patches);
    assert!(row.distinct_selected_families >= 2);
    assert!(row.active_synapses > 0);
}

#[test]
fn unavailable_gpu_rows_cannot_forge_hardware_or_execution() {
    let target = canonical_performance_targets_v1().rows[0];
    let protocol = GpuClosedLoopBenchmarkProtocolV1::canonical();
    let row = benchmark_row_fixture(
        target,
        GpuBenchmarkStatus::Unavailable {
            reason_code: GPU_BENCHMARK_UNAVAILABLE_NO_ADAPTER,
        },
        None,
    );
    row.validate(&protocol, &target).unwrap();
    assert!(row.environment.adapter.is_none());
    assert!(row.admission.is_none());
    assert_eq!(row.measured_p95_ns, None);
    assert!(row.raw_inference_timestamp_ticks.is_empty());
    assert!(row.raw_plasticity_timestamp_ticks.is_empty());
    assert!(row.raw_neural_tick_ns.is_empty());
    assert_eq!(row.gpu_selections, 0);
    assert_eq!(
        row.phenotype_manifest.manifest_digest,
        row.phenotype_manifest_digest
    );
}

#[test]
fn completed_gpu_row_tampering_and_forged_statuses_are_rejected() {
    let target = canonical_performance_targets_v1().rows[0];
    let protocol = GpuClosedLoopBenchmarkProtocolV1::canonical();
    let valid = benchmark_row_fixture(
        target,
        GpuBenchmarkStatus::Completed,
        Some(admitted_adapter_fixture(target)),
    );

    let mut phenotype = valid.clone();
    phenotype.phenotype_manifest.lobe_layout_digest[0] ^= 1;
    phenotype.seal_digest().unwrap();
    assert!(phenotype.validate(&protocol, &target).is_err());

    let mut environment = valid.clone();
    environment.environment.environment_digest[0] ^= 1;
    environment.seal_digest().unwrap();
    assert!(environment.validate(&protocol, &target).is_err());

    let mut mismatch = valid.clone();
    let mut mismatched_adapter = mismatch.environment.adapter.clone().unwrap();
    mismatched_adapter.required_features_digest[0] ^= 1;
    mismatch.environment =
        GpuBenchmarkEnvironmentReceipt::new(0, Some(mismatched_adapter)).unwrap();
    mismatch.seal_digest().unwrap();
    assert!(mismatch.validate(&protocol, &target).is_err());

    let mut cardinality = valid.clone();
    cardinality.raw_neural_tick_ns.pop();
    cardinality.seal_digest().unwrap();
    assert!(cardinality.validate(&protocol, &target).is_err());

    let mut admission = valid.clone();
    admission.admission = None;
    admission.seal_digest().unwrap();
    assert!(admission.validate(&protocol, &target).is_err());

    let mut status = valid.clone();
    status.status = GpuBenchmarkStatus::Missed;
    status.seal_digest().unwrap();
    assert!(status.validate(&protocol, &target).is_err());

    let mut seed = valid.clone();
    seed.fixture_seed ^= 1;
    seed.seal_digest().unwrap();
    assert!(seed.validate(&protocol, &target).is_err());

    let mut digest = valid;
    digest.row_digest[0] ^= 1;
    assert!(digest.validate(&protocol, &target).is_err());
}

#[test]
fn admission_unavailable_rows_allow_only_honest_optional_snapshots() {
    let target = canonical_performance_targets_v1().rows[0];
    let protocol = GpuClosedLoopBenchmarkProtocolV1::canonical();
    let adapter = admitted_adapter_fixture(target);
    let mut row = benchmark_row_fixture(
        target,
        GpuBenchmarkStatus::Unavailable {
            reason_code: GPU_BENCHMARK_UNAVAILABLE_ADMISSION,
        },
        Some(adapter),
    );
    row.validate(&protocol, &target).unwrap();

    let capacity =
        BrainCapacityClass::production_for_id(BrainClassId(target.class_id_raw)).unwrap();
    row.admission = Some(admission_fixture(&capacity, 0));
    row.seal_digest().unwrap();
    row.validate(&protocol, &target).unwrap();

    row.admission = Some(admission_fixture(&capacity, target.population));
    row.seal_digest().unwrap();
    assert!(row.validate(&protocol, &target).is_err());
}

#[test]
fn adapter_display_name_does_not_change_benchmark_identity() {
    let adapter = adapter_fixture(0x744c);
    let expected = adapter_identity_digest(&adapter).unwrap();
    let mut renamed = adapter;
    renamed.set_adapter_name("renamed by driver").unwrap();
    assert_eq!(adapter_identity_digest(&renamed).unwrap(), expected);
}

#[test]
fn missed_status_is_derived_only_from_an_executed_over_target_row() {
    let mut target = canonical_performance_targets_v1().rows[0];
    target.target_p95_ns = 1;
    let protocol = GpuClosedLoopBenchmarkProtocolV1::canonical();
    let row = benchmark_row_fixture(
        target,
        GpuBenchmarkStatus::Missed,
        Some(admitted_adapter_fixture(target)),
    );
    row.validate(&protocol, &target).unwrap();
    assert_eq!(row.measured_p95_ns, Some(2));
    assert!(row.admission.is_some());
}

#[test]
fn benchmark_manifest_rejects_duplicate_tampered_and_mixed_adapter_rows() {
    let (targets, valid) = benchmark_manifest_fixture();

    let mut duplicate = valid.clone();
    duplicate.rows[1] = duplicate.rows[0].clone();
    duplicate.seal_digest().unwrap();
    assert!(duplicate.validate(&targets).is_err());

    let mut mixed = valid.clone();
    let target = targets.rows[1];
    mixed.rows[1] = benchmark_row_fixture(
        target,
        GpuBenchmarkStatus::Unavailable {
            reason_code: GPU_BENCHMARK_UNAVAILABLE_REQUIRED_CAPABILITY,
        },
        Some(adapter_fixture(0x9999)),
    );
    mixed.seal_digest().unwrap();
    assert!(mixed.validate(&targets).is_err());

    let mut git = valid.clone();
    git.git_commit = "A".repeat(40);
    git.seal_digest().unwrap();
    assert!(git.validate(&targets).is_err());

    let mut digest = valid;
    digest.manifest_digest[0] ^= 1;
    assert!(digest.validate(&targets).is_err());
    assert!(is_lower_hex_oid(&"a".repeat(40)));
    assert!(!is_lower_hex_oid(&"A".repeat(40)));
    assert!(!is_lower_hex_oid(&"a".repeat(39)));
}
