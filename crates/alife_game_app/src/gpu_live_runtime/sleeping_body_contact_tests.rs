//! Fixed host scheduler/body/save fixtures. The deterministic driver below is
//! not evidence of GPU consolidation, device execution or live publication.
use super::*;
use alife_core::*;
use alife_world::{HeadlessScenarioBuilder, WorldEditorSpawnSpec};

struct HostAuthority {
    world: HeadlessWorld,
    residents: BTreeMap<u64, ResidentCognition>,
}
impl LiveAuthorityOwner for HostAuthority {
    fn world_and_residents(
        &mut self,
    ) -> (&mut HeadlessWorld, &mut BTreeMap<u64, ResidentCognition>) {
        (&mut self.world, &mut self.residents)
    }
}

/// Fixed scalar transaction identities, no neural data or real GPU job.
struct ScalarDriver;
impl GpuSleepConsolidationDriver for ScalarDriver {
    fn has_bounded_sleep_phase_data(
        &mut self,
        _: OrganismId,
        _: SleepState,
    ) -> Result<bool, ScaffoldContractError> {
        Ok(false)
    }
    fn progress(
        &mut self,
        _: OrganismId,
        state: SleepState,
        intent: Option<ConsolidationIntent>,
    ) -> SleepProgressResult {
        if let Some(intent) = intent {
            return Ok(Some(ConsolidationDriverEvent::ReplayAssetPersisted {
                intent,
                replay_digest: [11, 12, 13, 14],
                replay_event_count: 0,
                replay_eligibility_sample_count: 0,
            }));
        }
        Ok(Some(match state.consolidation {
            ConsolidationState::Pending {
                intent,
                replay_digest,
                ..
            } => {
                let mut request = GpuConsolidationRequest {
                    schema_version: GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
                    request_flags: 0,
                    cycle_id: intent.cycle_id,
                    phenotype_hash: PhenotypeHash([21, 22, 23, 24]),
                    input_generation: 1,
                    expected_output_generation: 2,
                    input_digest: [31, 32, 33, 34],
                    replay_digest,
                    max_replay_events: 1,
                    max_replay_eligibility_samples: 1,
                    request_digest: [0; 4],
                };
                request.request_digest = request.recompute_request_digest()?;
                ConsolidationDriverEvent::Prepared { request }
            }
            ConsolidationState::Prepared { request } => ConsolidationDriverEvent::Submitted {
                request,
                job_id: ConsolidationJobId::try_from_raw(1)?,
            },
            ConsolidationState::Submitted { request, job_id } => {
                let mut staged = ConsolidationStagedOutput {
                    job_id,
                    output_generation: request.expected_output_generation,
                    output_weight_bank: 1,
                    output_digest: [41, 42, 43, 44],
                    eligibility_reset_generation: 2,
                    output_eligibility_bank: 0,
                    eligibility_output_digest: [51, 52, 53, 54],
                    replay_journal_generation: 2,
                    replay_journal_cursor: 0,
                    replay_journal_event_count: 0,
                    replay_journal_output_digest: [61, 62, 63, 64],
                    staging_digest: [0; 4],
                    promoted_fast_l1_bits: 0,
                    replay_induced_fast_l1_bits: 0,
                };
                staged.staging_digest = staged.recompute_staging_digest(&request, 1, 1)?;
                ConsolidationDriverEvent::Completed { request, staged }
            }
            _ => return Ok(None),
        }))
    }
}

fn fixture() -> HostAuthority {
    let mut world = HeadlessScenarioBuilder::new(47_200)
        .agent("sleeper", OrganismId(1), Vec3f::ZERO)
        .agent("awake", OrganismId(2), Vec3f::new(4.0, 0.0, 0.0))
        .build()
        .unwrap();
    let mut residents: BTreeMap<_, _> = [1, 2]
        .into_iter()
        .map(|raw| {
            super::tests::register_sealing_test_organism(&mut world, OrganismId(raw));
            let (_, resident) = GpuLiveBrainRuntime::compile_birth(
                &world,
                BrainScaleTier::Nano512,
                SensorProfile::GroundedObjectSlotsV1,
                OrganismId(raw),
            )
            .unwrap();
            (raw, resident)
        })
        .collect();
    residents
        .get_mut(&1)
        .unwrap()
        .sleep_scheduler
        .force_recovery_sleep(Tick::ZERO)
        .unwrap();
    HostAuthority { world, residents }
}
fn object(
    world: &mut HeadlessWorld,
    label: &str,
    position: Vec3f,
    kind: WorldObjectKind,
    pain: f32,
) {
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: label.into(),
            kind,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: pain,
            radius: 0.6,
            token_id: None,
        })
        .unwrap();
}
fn body(world: &HeadlessWorld, raw: u64) -> &alife_world::WorldObject {
    let entity = world
        .organism_registry()
        .get(OrganismId(raw))
        .unwrap()
        .world_entity_id();
    world.entity(entity).unwrap()
}
fn schedule(owner: &mut HostAuthority) -> (BTreeMap<u64, BodyEventDelta>, BTreeSet<u64>) {
    let mut events = BTreeMap::new();
    let mut selected = BTreeSet::new();
    for (&raw, resident) in &mut owner.residents {
        let before = resident.sleep_scheduler.state();
        let held = matches!(before.consolidation, ConsolidationState::Completed { .. });
        if !held {
            let mut record = owner
                .world
                .organism_registry()
                .get(OrganismId(raw))
                .unwrap()
                .clone();
            let event = resident
                .sleep_scheduler
                .scheduled_tick_with_organism(
                    &mut record,
                    HomeostaticParameters::reference(),
                    owner.world.tick(),
                    &mut ScalarDriver,
                    false,
                )
                .unwrap();
            assert!(event.phase_receipt.sealed);
            owner.world.replace_organism_record_exact(record).unwrap();
        }
        let after = resident.sleep_scheduler.state();
        if sleeping_body_contact_due(before.phase, after.phase) {
            selected.insert(raw);
        }
        if sleep_recovery_body_event_due(before.phase, held) {
            events.insert(
                raw,
                BodyEventDelta {
                    sleep_recovery: 1.0,
                    ..BodyEventDelta::zero()
                },
            );
        }
    }
    (events, selected)
}
fn advance(
    owner: &mut HostAuthority,
    events: &BTreeMap<u64, BodyEventDelta>,
    selected: &BTreeSet<u64>,
) -> Result<(), ScaffoldContractError> {
    let next = Tick(owner.world.tick().raw() + 1);
    advance_and_synchronize_authority(
        &mut owner.world,
        &mut owner.residents,
        next,
        events,
        selected,
    )?;
    Ok(())
}
fn assert_round_trip(owner: &HostAuthority) {
    let save = PortableSaveFile::from_headless_world(
        "sleeping-body-host",
        &owner.world,
        alife_world::RuntimeConfig::deterministic_default(
            owner.world.seed(),
            BrainScaleTier::Nano512,
        ),
        AssetManifest::empty(),
        vec![],
    )
    .unwrap();
    let encoded = serde_json::to_vec(&save).unwrap();
    let reopened: PortableSaveFile = serde_json::from_slice(&encoded).unwrap();
    let restored = reopened.restore_headless_world().unwrap();
    assert_eq!(
        restored.canonical_signature_digest().unwrap(),
        owner.world.canonical_signature_digest().unwrap()
    );
    for (&raw, resident) in &owner.residents {
        let bytes = serde_json::to_vec(&resident.sleep_scheduler).unwrap();
        let restored: GpuSleepScheduler = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored, resident.sleep_scheduler);
        assert_eq!(
            restored.state().consolidation,
            resident.sleep_scheduler.state().consolidation
        );
        let record = reopened
            .restore_headless_world()
            .unwrap()
            .organism_registry()
            .get(OrganismId(raw))
            .unwrap()
            .clone();
        assert_eq!(
            record.sleep_seal(),
            owner
                .world
                .organism_registry()
                .get(OrganismId(raw))
                .unwrap()
                .sleep_seal()
        );
    }
}

#[test]
fn sleeping_body_host_actual_scheduler_contact_mixed_awake_and_exact_biology() {
    for blocked in [false, true] {
        let mut owner = fixture();
        object(
            &mut owner.world,
            "sleep-pain",
            Vec3f::new(0.2, 0.0, 0.0),
            WorldObjectKind::Hazard,
            0.4,
        );
        object(
            &mut owner.world,
            "awake-pain",
            Vec3f::new(4.2, 0.0, 0.0),
            WorldObjectKind::Hazard,
            0.4,
        );
        if blocked {
            object(
                &mut owner.world,
                "wall",
                Vec3f::new(-0.2, 0.0, 0.0),
                WorldObjectKind::Obstacle,
                0.0,
            );
        }
        let (events, selected) = schedule(&mut owner);
        assert_eq!(selected, BTreeSet::from([1]));
        let before = owner.residents.clone();
        let record = owner
            .world
            .organism_registry()
            .get(OrganismId(1))
            .unwrap()
            .clone();
        let before_yaw = body(&owner.world, 1).body_yaw;
        let mut oracle = owner.world.clone();
        let receipt = oracle
            .try_advance_tick_with_selected_body_contacts_in_staged_tick(&events, &selected)
            .unwrap();
        assert_eq!(receipt.responses.len(), 1);
        let response = &receipt.responses[0];
        assert!(response.sleep_recovery_interrupted);
        assert_eq!(response.withdrawal.as_ref().unwrap().blocked, blocked);
        assert_eq!(response.applied_body_event.sleep_recovery, 0.0);
        let expected = record
            .biochemistry()
            .advance(Tick(1), response.applied_body_event, record.phenotype())
            .unwrap();
        advance(&mut owner, &events, &selected).unwrap();
        assert_eq!(
            owner.world.canonical_signature_digest().unwrap(),
            oracle.canonical_signature_digest().unwrap()
        );
        assert_eq!(
            *owner
                .world
                .organism_registry()
                .get(OrganismId(1))
                .unwrap()
                .biochemistry(),
            expected
        );
        assert_eq!(body(&owner.world, 1).body_yaw, before_yaw);
        assert_eq!(body(&owner.world, 2).position, Vec3f::new(4.0, 0.0, 0.0));
        if blocked {
            assert_eq!(body(&owner.world, 1).position, Vec3f::ZERO);
        } else {
            assert!(body(&owner.world, 1).position.x < 0.0);
        }
        for raw in [1, 2] {
            let mut expected_resident = before[&raw].clone();
            synchronize_resident_from_record(
                &mut expected_resident,
                owner
                    .world
                    .organism_registry()
                    .get(OrganismId(raw))
                    .unwrap(),
                Tick(1),
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(&owner.residents[&raw]).unwrap(),
                serde_json::to_value(expected_resident).unwrap()
            );
        }
        assert_round_trip(&owner);
    }
}

#[test]
fn sleeping_body_host_quiet_and_chronic_pain_keep_ordinary_sleep_exact() {
    for chronic in [false, true] {
        let mut owner = fixture();
        if chronic {
            owner
                .world
                .try_advance_tick_with_body_events(&BTreeMap::from([(
                    1,
                    BodyEventDelta {
                        damage: 0.4,
                        ..BodyEventDelta::zero()
                    },
                )]))
                .unwrap();
            assert!(
                owner
                    .world
                    .organism_registry()
                    .get(OrganismId(1))
                    .unwrap()
                    .biochemistry()
                    .body
                    .injury
                    > 0.0
            );
        }
        let (events, selected) = schedule(&mut owner);
        let mut ordinary = owner.world.clone();
        ordinary.try_advance_tick_with_body_events(&events).unwrap();
        advance(&mut owner, &events, &selected).unwrap();
        assert_eq!(
            owner.world.canonical_signature_digest().unwrap(),
            ordinary.canonical_signature_digest().unwrap()
        );
        assert_eq!(body(&owner.world, 1).position, Vec3f::ZERO);
        assert_round_trip(&owner);
    }
}

#[test]
fn sleeping_body_host_all_inflight_payloads_and_completed_hold_survive() {
    let mut owner = fixture();
    let mut observed = BTreeSet::new();
    for _ in 0..24 {
        let (events, selected) = schedule(&mut owner);
        let state = owner.residents[&1].sleep_scheduler.state();
        observed.insert(state.consolidation.kind_raw());
        let mut painful = HostAuthority {
            world: owner.world.clone(),
            residents: owner.residents.clone(),
        };
        object(
            &mut painful.world,
            "pain",
            Vec3f::new(0.2, 0.0, 0.0),
            WorldObjectKind::Hazard,
            0.4,
        );
        let full_scheduler = painful.residents[&1].sleep_scheduler;
        if matches!(state.consolidation, ConsolidationState::Completed { .. }) {
            // The first Completed event still belongs to the normal interval;
            // the next scheduled interval is an explicit durability hold.
            let (hold_events, hold_selected) = schedule(&mut painful);
            assert!(!hold_events.contains_key(&1));
            assert!(hold_selected.contains(&1));
            advance(&mut painful, &hold_events, &hold_selected).unwrap();
        } else {
            advance(&mut painful, &events, &selected).unwrap();
        }
        assert!(body(&painful.world, 1).position.x < 0.0);
        assert_eq!(painful.residents[&1].sleep_scheduler, full_scheduler);
        assert_eq!(
            painful.residents[&1].next_sequence,
            owner.residents[&1].next_sequence
        );
        assert_round_trip(&painful);
        advance(&mut owner, &events, &selected).unwrap();
        if matches!(state.consolidation, ConsolidationState::Completed { .. }) {
            break;
        }
    }
    // None, Pending, Prepared, Submitted, Completed are the ordinary protocol.
    assert_eq!(observed, BTreeSet::from([0, 1, 2, 3, 4]));
}

#[test]
fn sleeping_body_host_late_failure_restores_scheduler_body_seals_then_retry_once() {
    let mut owner = fixture();
    object(
        &mut owner.world,
        "pain",
        Vec3f::new(0.2, 0.0, 0.0),
        WorldObjectKind::Hazard,
        0.4,
    );
    let before_world = owner.world.canonical_signature_digest().unwrap();
    let before_residents = serde_json::to_value(&owner.residents).unwrap();
    let mut expected = HostAuthority {
        world: owner.world.clone(),
        residents: owner.residents.clone(),
    };
    let (events, selected) = schedule(&mut expected);
    advance(&mut expected, &events, &selected).unwrap();
    let (result, _) = tick_with_sleep_progress_inner(&mut owner, false, |staged| {
        let (events, selected) = schedule(staged);
        advance(staged, &events, &selected)?;
        assert!(body(&staged.world, 1).position.x < 0.0);
        Err::<(), _>(ScaffoldContractError::InvalidDecisionEvidence)
    });
    assert_eq!(result, Err(ScaffoldContractError::InvalidDecisionEvidence));
    assert_eq!(
        owner.world.canonical_signature_digest().unwrap(),
        before_world
    );
    assert_eq!(
        serde_json::to_value(&owner.residents).unwrap(),
        before_residents
    );
    let (result, _) = tick_with_sleep_progress_inner(&mut owner, false, |staged| {
        let (events, selected) = schedule(staged);
        advance(staged, &events, &selected)
    });
    result.unwrap();
    assert_eq!(
        owner.world.canonical_signature_digest().unwrap(),
        expected.world.canonical_signature_digest().unwrap()
    );
    assert_eq!(
        serde_json::to_value(&owner.residents).unwrap(),
        serde_json::to_value(&expected.residents).unwrap()
    );
    assert_round_trip(&owner);
}

#[test]
fn sleeping_body_host_scope_excludes_awake_without_recovery_or_selection() {
    assert!(!sleeping_body_contact_due(
        SleepPhase::Awake,
        SleepPhase::Awake
    ));
    for phase in [
        SleepPhase::EnteringSleep,
        SleepPhase::Consolidating,
        SleepPhase::Waking,
        SleepPhase::ForcedRecoverySleep,
    ] {
        assert!(sleeping_body_contact_due(phase, phase));
        assert!(sleeping_body_contact_due(SleepPhase::Awake, phase));
        assert!(sleeping_body_contact_due(phase, SleepPhase::Awake));
    }
    let mut owner = fixture();
    object(
        &mut owner.world,
        "pain",
        Vec3f::new(4.2, 0.0, 0.0),
        WorldObjectKind::Hazard,
        0.4,
    );
    let (_, selected) = schedule(&mut owner);
    // Even a recovery dose for an awake resident cannot opt it into withdrawal.
    let events = BTreeMap::from([(
        2,
        BodyEventDelta {
            sleep_recovery: 1.0,
            ..BodyEventDelta::zero()
        },
    )]);
    advance(&mut owner, &events, &selected).unwrap();
    assert_eq!(body(&owner.world, 2).position, Vec3f::new(4.0, 0.0, 0.0));
}

#[cfg(feature = "gpu-tests")]
mod device {
    use super::*;

    fn fixture(label: &str) -> (PathBuf, GpuLiveBrainRuntime) {
        let root =
            std::env::temp_dir().join(format!("alife-sleep-body-{}-{label}", std::process::id()));
        assert!(!root.exists(), "preserve fixed-fixture evidence");
        std::fs::create_dir_all(root.join("assets")).unwrap();
        let mut config =
            alife_world::RuntimeConfig::deterministic_default(31_117, BrainScaleTier::Nano512);
        config.features.gpu_backend_enabled = true;
        let runtime = crate::new_game_lifecycle::create_canonical_new_game_software_runtime_test(
            crate::CanonicalNewGameLaunchRequest {
                world_seed: 31_117,
                population: if label == "mixed" { 2 } else { 1 },
                disable_age_death: false,
                save_path: root.join("live.json"),
                asset_root: root.join("assets"),
                config,
                assets: AssetManifest::empty(),
            },
        )
        .unwrap()
        .runtime;
        assert!(runtime.backend.backend().is_software_runtime_test());
        assert!(!runtime.backend.backend().is_software_training_test());
        let info = runtime
            .backend
            .backend()
            .software_runtime_test_adapter_info()
            .unwrap();
        assert_eq!(format!("{:?}", info.device_type), "Cpu");
        assert_eq!(format!("{:?}", info.backend), "Vulkan");
        println!(
            "SLEEP_BODY_EXPLICIT_OFFLINE_SOFTWARE_ADAPTER={:?}",
            runtime
                .backend
                .backend()
                .software_runtime_test_adapter_info()
                .unwrap()
        );
        (root, runtime)
    }

    fn restore_capture(runtime: &mut GpuLiveBrainRuntime, root: &Path, label: &str) {
        // Finish the existing capture/publication obligations without spending
        // a world tick or manufacturing a Completed -> Committed scheduler edge.
        let deadline = Instant::now() + std::time::Duration::from_secs(90);
        while !runtime.persistence_idle_for_shutdown() {
            assert!(
                Instant::now() < deadline,
                "{}",
                runtime.persistence_shutdown_diagnostics()
            );
            runtime.poll_persistence_for_shutdown().unwrap();
            assert!(!runtime.persistence_failed_for_shutdown());
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let expected_world = runtime.world.canonical_signature_digest().unwrap();
        let expected_sleep: Vec<_> = runtime
            .residents
            .iter()
            .map(|(raw, r)| (*raw, r.sleep_scheduler.state()))
            .collect();
        let save = runtime.capture_portable_checkpoint().unwrap();
        assert_eq!(save.world.tick, runtime.world.tick());
        let path = root.join(format!("{label}.json"));
        let assets = root.join("assets");
        GpuDurableSaveManifest::publish_snapshot(&path, &assets, &save).unwrap();
        let (manifest, loaded) = GpuDurableSaveManifest::open_loaded(path, assets).unwrap();
        let restored = GpuLiveBrainRuntime::restore_loaded_save(
            runtime.new_staging_like_live().unwrap(),
            manifest,
            loaded,
            save.deterministic_seed,
            BrainScaleTier::Nano512,
        )
        .unwrap();
        assert_eq!(
            restored.world.canonical_signature_digest().unwrap(),
            expected_world
        );
        for (raw, sleep) in expected_sleep {
            assert_eq!(restored.residents[&raw].sleep_scheduler.state(), sleep);
            assert_eq!(
                restored.residents[&raw].next_sequence,
                runtime.residents[&raw].next_sequence
            );
        }
        restored.world.validate_organism_bindings().unwrap();
        restored.backend.ensure_neural_actions_available().unwrap();
    }

    #[test]
    fn sleeping_body_software_runtime_normal_tick_mixed_control_capture_restore() {
        let (root, mut runtime) = fixture("mixed");
        for raw in runtime.residents.keys().copied().collect::<Vec<_>>() {
            if raw != 2 {
                runtime.request_recovery_sleep(OrganismId(raw)).unwrap();
            }
        }
        // The awake control deliberately has no prepared decision. It must not
        // receive the sleeping physical policy merely because no action ran.
        runtime.force_memory_preparation_failure_for_test(OrganismId(2));
        let start = body(&runtime.world, 1).position;
        let awake = body(&runtime.world, 2).position;
        object(
            &mut runtime.world,
            "sleeper-pain",
            Vec3f::new(start.x + 0.2, start.y, start.z),
            WorldObjectKind::Hazard,
            0.03,
        );
        object(
            &mut runtime.world,
            "awake-pain",
            Vec3f::new(awake.x + 0.2, awake.y, awake.z),
            WorldObjectKind::Hazard,
            0.03,
        );
        let sleep = runtime.residents[&1].sleep_scheduler;
        let patch_count = runtime.sealed_patch_count;
        let sequence = runtime.residents[&1].next_sequence;
        let before = sleep_atomicity_tests::HostSleepSnapshot::capture(&runtime);
        runtime.force_late_advance_failure_for_test();
        assert!(runtime.tick_outcome().is_err());
        assert_eq!(
            sleep_atomicity_tests::HostSleepSnapshot::capture(&runtime),
            before
        );
        runtime.backend.ensure_neural_actions_available().unwrap();
        let handle = runtime.handle_for(OrganismId(1)).unwrap();
        let charged = runtime.backend.snapshot_activity_state(handle).unwrap();
        // Fault-injection selectors are intentionally one-shot, so repeat only
        // the awake control's preparation failure for the ordinary retry.
        runtime.force_memory_preparation_failure_for_test(OrganismId(2));
        assert!(matches!(
            runtime.tick_outcome().unwrap(),
            GpuLiveTickOutcome::Progressed(_)
        ));
        assert_eq!(runtime.world.tick(), Tick(1));
        let retried = runtime.backend.snapshot_activity_state(handle).unwrap();
        assert_eq!(retried.last_world_atp_tick, charged.last_world_atp_tick);
        assert_eq!(retried.brain_atp_q16, charged.brain_atp_q16);
        assert!(body(&runtime.world, 1).position.x < start.x);
        assert_eq!(body(&runtime.world, 2).position, awake);
        assert_eq!(runtime.residents[&1].sleep_scheduler, sleep);
        assert_eq!(runtime.residents[&1].next_sequence, sequence);
        assert_eq!(runtime.sealed_patch_count, patch_count);
        assert!(runtime.last_learning_receipts.is_empty());
        assert!(runtime.last_gpu_authority_receipts.is_empty());
        assert!(
            runtime
                .world
                .organism_registry()
                .get(OrganismId(1))
                .unwrap()
                .biochemistry()
                .body
                .injury
                > 0.0
        );
        restore_capture(&mut runtime, &root, "withdrawn");
        println!("SLEEP_BODY_OFFLINE_SOFTWARE_MIXED_PASS tick=1 sleeping_withdrawal=1 awake_no_selection_withdrawal=0 voluntary_patches=0 learning_receipts=0 post_body_rollback_retry=1 restore=1");
    }

    #[test]
    fn sleeping_body_software_runtime_pending_jobs_and_completed_capture_guard() {
        let (root, mut runtime) = fixture("pipeline");
        for raw in runtime.residents.keys().copied().collect::<Vec<_>>() {
            runtime.request_recovery_sleep(OrganismId(raw)).unwrap();
        }
        let start = body(&runtime.world, 1).position;
        object(
            &mut runtime.world,
            "pipeline-pain",
            Vec3f::new(start.x + 0.2, start.y, start.z),
            WorldObjectKind::Hazard,
            0.01,
        );
        let hazard = runtime.world.entity_id("pipeline-pain").unwrap();
        let mut observed = BTreeSet::new();
        // Dropped before runtime on failure, so a held writer cannot deadlock
        // teardown. This is the existing normal checkpoint-worker test gate.
        let mut worker_release = None;
        // Bounded fixed empty-replay pipeline, never an awake policy episode.
        for _ in 0..24 {
            runtime.flush_sleep_journal_publication_blocking().unwrap();
            let before = runtime.residents[&1].sleep_scheduler.state();
            assert_ne!(before.phase, SleepPhase::Awake);
            if matches!(before.consolidation, ConsolidationState::Submitted { .. })
                && worker_release.is_none()
            {
                let (sender, receiver) = mpsc::sync_channel(1);
                runtime.next_checkpoint_worker_start_gate = Some(receiver);
                worker_release = Some(sender);
            }
            let position = body(&runtime.world, 1).position;
            runtime
                .world
                .editor_move_object(hazard, Vec3f::new(position.x + 0.2, position.y, position.z))
                .unwrap();
            assert!(matches!(
                runtime.tick_outcome().unwrap(),
                GpuLiveTickOutcome::Progressed(_)
            ));
            assert!(body(&runtime.world, 1).position.x < position.x);
            assert!(runtime.last_learning_receipts.is_empty());
            assert_eq!(runtime.sealed_patch_count, 0);
            let state = runtime.residents[&1].sleep_scheduler.state();
            let kind = state.consolidation.kind_raw();
            if observed.insert(kind) && kind > 0 && kind != 4 {
                restore_capture(&mut runtime, &root, &format!("state-{kind}"));
            }
            if matches!(state.consolidation, ConsolidationState::Completed { .. }) {
                let position = body(&runtime.world, 1).position;
                runtime
                    .world
                    .editor_move_object(
                        hazard,
                        Vec3f::new(position.x + 0.2, position.y, position.z),
                    )
                    .unwrap();
                let scheduler = runtime.residents[&1].sleep_scheduler;
                assert!(runtime.exact_checkpoint_coordinator.is_active());
                assert!(matches!(
                    runtime.tick_outcome().unwrap(),
                    GpuLiveTickOutcome::Progressed(_)
                ));
                assert_eq!(runtime.residents[&1].sleep_scheduler, scheduler);
                assert!(body(&runtime.world, 1).position.x < position.x);
                drop(worker_release.take());
                restore_capture(&mut runtime, &root, "completed-held");
                break;
            }
        }
        assert!(observed.contains(&1));
        assert!(observed.contains(&2));
        assert!(observed.contains(&3));
        assert!(observed.contains(&4));
        println!("SLEEP_BODY_OFFLINE_SOFTWARE_PIPELINE_PASS pending=1 prepared=1 submitted=1 completed=1 completed_durability_hold=1 capture_restore=4 voluntary_patches=0");
    }
}

#[test]
fn sleeping_body_host_unlocalized_acute_damage_reverses_heading_without_yaw() {
    let mut owner = fixture();
    let (mut events, selected) = schedule(&mut owner);
    events.get_mut(&1).unwrap().damage = 0.2;
    let before = body(&owner.world, 1).clone();
    advance(&mut owner, &events, &selected).unwrap();
    let after = body(&owner.world, 1);
    assert_eq!(after.body_yaw, before.body_yaw);
    let displacement = Vec3f::new(
        after.position.x - before.position.x,
        after.position.y - before.position.y,
        after.position.z - before.position.z,
    );
    assert!(displacement.x * before.body_yaw.cos() + displacement.z * before.body_yaw.sin() < 0.0);
    assert_round_trip(&owner);
}

#[test]
fn sleeping_body_host_selective_interval_never_repeats_action_biology() {
    let mut owner = fixture();
    let id = OrganismId(2);
    let record = owner.world.organism_registry().get(id).unwrap();
    let entity = record.world_entity_id();
    let frame = owner
        .world
        .perception_frame(
            id,
            Tick::ZERO,
            SensorProfile::GroundedObjectSlotsV1,
            record.biochemistry().homeostasis,
        )
        .unwrap();
    let rest = frame
        .candidates()
        .iter()
        .find(|c| c.kind == ActionKind::Rest)
        .unwrap();
    let command = rest.to_command(id, Confidence::new(1.0).unwrap()).unwrap();
    owner
        .world
        .apply_registered_neural_command(&command, entity, Tick(1), None, false)
        .unwrap();
    let biology = *owner
        .world
        .organism_registry()
        .get(id)
        .unwrap()
        .biochemistry();
    // Even an erroneously broad physical set cannot move or advance a body
    // whose voluntary transaction has already accounted for this interval.
    object(
        &mut owner.world,
        "late-pain",
        Vec3f::new(4.2, 0.0, 0.0),
        WorldObjectKind::Hazard,
        0.4,
    );
    let receipt = owner
        .world
        .try_advance_tick_with_selected_body_contacts_in_staged_tick(
            &BTreeMap::new(),
            &BTreeSet::from([1, 2]),
        )
        .unwrap();
    assert!(receipt.responses.is_empty());
    assert_eq!(
        *owner
            .world
            .organism_registry()
            .get(id)
            .unwrap()
            .biochemistry(),
        biology
    );
    assert_eq!(body(&owner.world, 2).position, Vec3f::new(4.0, 0.0, 0.0));
}

#[test]
fn sleeping_body_host_invalid_selection_is_rejected_before_body_mutation() {
    let mut owner = fixture();
    object(
        &mut owner.world,
        "pain",
        Vec3f::new(0.2, 0.0, 0.0),
        WorldObjectKind::Hazard,
        0.4,
    );
    let before = owner.world.canonical_signature_digest().unwrap();
    for selected in [BTreeSet::from([0]), BTreeSet::from([1, 99])] {
        assert!(owner
            .world
            .try_advance_tick_with_selected_body_contacts_in_staged_tick(
                &BTreeMap::new(),
                &selected
            )
            .is_err());
        assert_eq!(owner.world.canonical_signature_digest().unwrap(), before);
    }
}
