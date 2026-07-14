use alife_core::{
    cpu_reference_arbitrate, ActionArbitrationConfig, ActionCandidate, ActionId, ActionKind,
    ActionProposal, ActionTarget, BodySnapshot, BrainClassSpec, BrainScaleTier,
    CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef, Confidence,
    CreatureMind, DenseTile, DriveDelta, DurationTicks, EndocrineDelta, HomeostaticDelta,
    HomeostaticParameters, HomeostaticSnapshot, Intensity, LobeKind, MemoryBank, MemoryBankConfig,
    MemoryExpectancy, MemoryOutcomeSummary, MemoryRecord, NeuralProjectionSchema, NormalizedScalar,
    OrganismId, PerceptionFrame, PhysicalActionOutcome, PhysicalContactKind, PostActionOutcome,
    ProjectionRoutingRef, ProjectionTile, ScaffoldContractError, SensorProfile, SensoryChannels,
    SensorySnapshot, SignedValence, SleepConsolidationConfig, SleepConsolidator, SleepController,
    SleepPhase, SleepTrigger, SparseTileCoord, SparseTilePayload, StableLifetimeTraitKind,
    StructuralEditBatch, StructuralEditCandidate, StructuralEditKind, StructuralEditReason,
    SynapseWeightSplit, Tick, TopologicalMap, TopologicalMapConfig, Vec3f, Velocity, WorldEntityId,
    MICROTILE_CELLS,
};

fn organism() -> OrganismId {
    OrganismId(77)
}

fn spec() -> BrainClassSpec {
    BrainClassSpec::for_tier(BrainScaleTier::Nano512)
}

fn config() -> SleepConsolidationConfig {
    SleepConsolidationConfig {
        entering_duration: DurationTicks::new(1),
        consolidation_duration: DurationTicks::new(2),
        waking_duration: DurationTicks::new(1),
        stable_trait_promotion_threshold: 2,
        stable_trait_strength_threshold: NormalizedScalar::new(0.45).unwrap(),
        stable_trait_variance_threshold: NormalizedScalar::new(0.05).unwrap(),
        memory_max_records_after: 2,
        concept_simplex_consolidation_limit: 4,
        structural_edit_candidate_limit: 3,
        h_shadow_drain_rate: NormalizedScalar::new(0.5).unwrap(),
        h_shadow_decay_rate: NormalizedScalar::new(0.25).unwrap(),
        lifetime_staging_rate: NormalizedScalar::new(0.5).unwrap(),
        reset_alpha_after_lifetime_staging: true,
        ..SleepConsolidationConfig::reference()
    }
}

fn high_fatigue_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    let mut drives = alife_core::DriveSnapshot::baseline();
    drives.fatigue = 0.95;
    let mut hormones = alife_core::EndocrineSnapshot::baseline();
    hormones.sleep_pressure = 0.92;
    HomeostaticSnapshot::new(tick, drives, hormones).unwrap()
}

fn seizure_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    let mut drives = alife_core::DriveSnapshot::baseline();
    drives.brain_atp = 0.75;
    let mut hormones = alife_core::EndocrineSnapshot::baseline();
    hormones.adrenaline = 0.99;
    hormones.cortisol = 0.95;
    HomeostaticSnapshot::new(tick, drives, hormones).unwrap()
}

fn catatonia_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    let mut drives = alife_core::DriveSnapshot::baseline();
    drives.brain_atp = 0.02;
    HomeostaticSnapshot::new(tick, drives, alife_core::EndocrineSnapshot::baseline()).unwrap()
}

fn neural_schema_with_h_shadow() -> NeuralProjectionSchema {
    let spec = spec();
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    let mut dense = vec![SynapseWeightSplit::zero(); MICROTILE_CELLS];
    dense[0] = SynapseWeightSplit::new(0.25, 0.1, 0.5, 0.2, 0.4).unwrap();
    schema.projections[0].tiles.push(ProjectionTile::new_dense(
        0,
        SparseTileCoord::new(0, 0).unwrap(),
        DenseTile::new(dense).unwrap(),
    ));
    schema.rebuild_supertile_masks();
    schema
}

fn first_weights(schema: &NeuralProjectionSchema) -> SynapseWeightSplit {
    let SparseTilePayload::Dense(tile) = &schema.projections[0].tiles[0].payload else {
        panic!("expected dense test tile");
    };
    tile.weights[0]
}

fn memory_record(raw_id: u64, tick: u64, valence: f32) -> MemoryRecord {
    MemoryRecord {
        memory_id: alife_core::MemoryId(raw_id),
        organism_id: organism(),
        source_sequence_id: alife_core::ExperienceSequenceId(raw_id),
        source_tick: Tick::new(tick),
        features: vec![0.25, 0.5, 0.75, 0.1],
        expected_valence: SignedValence::new(valence).unwrap(),
        predicted_drive_delta: DriveDelta::zero(),
        outcome_summary: MemoryOutcomeSummary::neutral(),
        affordance_bias: NormalizedScalar::new(0.4).unwrap(),
        danger_bias: NormalizedScalar::new(0.1).unwrap(),
        safety_bias: NormalizedScalar::new(0.5).unwrap(),
        social_trust_bias: NormalizedScalar::new(0.2).unwrap(),
        social_fear_bias: NormalizedScalar::new(0.0).unwrap(),
        novelty_bias: NormalizedScalar::new(0.2).unwrap(),
        curiosity_bias: NormalizedScalar::new(0.3).unwrap(),
        selected_action_id: Some(ActionId::new(400).unwrap()),
        selected_action_kind: Some(ActionKind::Interact),
    }
}

fn memory_bank_with_three_records() -> MemoryBank {
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(4, 16, 2, 0.01, Confidence::new(0.05).unwrap()).unwrap(),
    )
    .unwrap();
    for (id, tick, valence) in [(1, 10, 0.1), (2, 11, 0.2), (3, 12, 0.3)] {
        bank.insert_record(memory_record(id, tick, valence))
            .unwrap();
    }
    bank
}

fn sensory(tick: Tick, target: WorldEntityId) -> SensorySnapshot {
    let mut visual = [0.0; alife_core::SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT];
    visual[0] = 0.9;
    let channels = SensoryChannels::try_from_groups(
        visual,
        [0.0; alife_core::SENSORY_AUDITORY_CHANNEL_COUNT],
        [0.0; alife_core::SENSORY_SMELL_CHANNEL_COUNT],
        [0.0; alife_core::SENSORY_TACTILE_CHANNEL_COUNT],
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.8).unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut snapshot = SensorySnapshot::new(
        organism(),
        tick,
        Vec3f::new(1.0, 0.0, 0.0),
        channels,
        Default::default(),
    )
    .unwrap();
    snapshot.social_context.nearest_agents[0] = Some(alife_core::SocialAgentSnapshot {
        agent_id: OrganismId(88),
        body_entity: Some(target),
        relative_position: Vec3f::new(0.5, 0.0, 0.0),
        gaze_direction: Vec3f::new(0.0, 0.0, 1.0),
        orientation_forward: Vec3f::new(0.0, 0.0, 1.0),
        affinity: SignedValence::new(0.3).unwrap(),
        proximity: NormalizedScalar::new(0.7).unwrap(),
    });
    snapshot
}

fn sealed_patch(
    sequence_raw: u64,
    tick_raw: u64,
    target: WorldEntityId,
    contradiction: bool,
) -> alife_core::ExperiencePatch {
    let sequence_id = alife_core::ExperienceSequenceId(sequence_raw);
    let spec = spec();
    let genome = alife_core::BrainGenome::scaffold(42, spec.id);
    let body = BodySnapshot {
        pose: alife_core::Pose {
            translation: Vec3f::new(1.0, 0.0, 0.0),
            rotation: alife_core::Quatf::IDENTITY,
        },
        velocity: Velocity::ZERO,
    };
    let homeostasis = HomeostaticSnapshot::baseline(Tick::new(tick_raw));
    let candidate_target = ActionTarget::new(Some(target), Some(Vec3f::new(0.0, 0.0, 1.0)));
    let perception = PerceptionFrame::new(
        organism(),
        Tick::new(tick_raw),
        SensorProfile::PrivilegedAffordanceV1,
        sensory(Tick::new(tick_raw), target),
        body,
        homeostasis,
        vec![ActionCandidate::new(
            0,
            ActionId(400),
            ActionKind::Interact,
            CandidateActionFamily::Contact,
            CandidateObservationRef::None,
            candidate_target,
            CandidateFeatureVector::zero(),
            Confidence::new(0.8).unwrap(),
            NormalizedScalar::new(0.0).unwrap(),
            DurationTicks::new(4),
            DurationTicks::new(4),
        )
        .unwrap()],
    )
    .unwrap();
    let pre = alife_core::PreActionSnapshot::from_heuristic_frame(
        sequence_id,
        perception,
        spec.clone(),
        genome.clone(),
        alife_core::DevelopmentState::new(
            genome.id,
            Tick::new(tick_raw),
            NormalizedScalar::new(0.35).unwrap(),
        )
        .with_enabled_lobes([
            LobeKind::SensoryGrounding,
            LobeKind::CoreAssociation,
            LobeKind::MotorArbitration,
        ]),
        alife_core::WeightSplitContract::for_brain_class(
            spec.id,
            spec.max_active_synapses,
            spec.max_active_microtiles,
            genome.genetic_prior_seed,
        )
        .unwrap(),
        alife_core::MemoryExpectancySnapshot::neutral(),
    )
    .unwrap();
    let proposals = vec![ActionProposal::new(
        ActionId::new(400).unwrap(),
        ActionKind::Interact,
        0.8,
        Confidence::new(0.8).unwrap(),
        None,
        0b101,
        ActionTarget::new(Some(target), Some(Vec3f::new(0.0, 0.0, 1.0))),
        NormalizedScalar::new(0.6).unwrap(),
    )
    .unwrap()
    .with_intensity(Intensity::new(0.7).unwrap())];
    let decision = cpu_reference_arbitrate(
        organism(),
        &proposals,
        ActionArbitrationConfig {
            default_duration_ticks: DurationTicks::new(4),
            ..ActionArbitrationConfig::default()
        },
    )
    .unwrap();
    let decision = alife_core::DecisionSnapshot::from_action_decision(
        sequence_id,
        Tick::new(tick_raw),
        proposals,
        decision,
    )
    .unwrap();
    let mut outcome = PostActionOutcome::new(
        organism(),
        sequence_id,
        Tick::new(tick_raw + 1),
        !contradiction,
        PhysicalActionOutcome {
            contact: if contradiction {
                PhysicalContactKind::Blocked
            } else {
                PhysicalContactKind::Touch
            },
            target_entity: Some(target),
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta {
            drives: DriveDelta {
                curiosity: if contradiction { 0.2 } else { 0.0 },
                ..DriveDelta::zero()
            },
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(if contradiction { -0.5 } else { 0.4 }).unwrap(),
        NormalizedScalar::new(if contradiction { 0.6 } else { 0.0 }).unwrap(),
        NormalizedScalar::new(if contradiction { 0.4 } else { 0.0 }).unwrap(),
        SignedValence::new(-0.1).unwrap(),
        NormalizedScalar::new(if contradiction { 0.9 } else { 0.1 }).unwrap(),
    )
    .unwrap();
    outcome.contradiction_observed = contradiction;
    alife_core::ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}

fn topology_with_gap() -> TopologicalMap {
    let mut map = TopologicalMap::new(TopologicalMapConfig {
        max_concepts: 16,
        max_edges: 32,
        max_simplexes: 16,
        max_unresolved_gaps: 8,
        edge_decay_per_tick: NormalizedScalar::new(0.05).unwrap(),
    })
    .unwrap();
    map.apply_patch(&sealed_patch(1, 10, WorldEntityId(2), false))
        .unwrap();
    map.apply_patch(&sealed_patch(2, 20, WorldEntityId(2), true))
        .unwrap();
    map
}

#[test]
fn fatigue_and_recovery_triggers_drive_deterministic_sleep_states() {
    let mut controller = SleepController::new(config()).unwrap();

    let transition = controller
        .evaluate_homeostasis(
            &high_fatigue_homeostasis(Tick::new(10)),
            HomeostaticParameters::reference(),
            Tick::new(10),
        )
        .unwrap()
        .unwrap();
    assert_eq!(transition.from, SleepPhase::Awake);
    assert_eq!(transition.to, SleepPhase::EnteringSleep);
    assert_eq!(transition.trigger, SleepTrigger::FatigueThreshold);

    let transition = controller.advance(Tick::new(11)).unwrap().unwrap();
    assert_eq!(transition.from, SleepPhase::EnteringSleep);
    assert_eq!(transition.to, SleepPhase::Consolidating);

    let mut recovery = SleepController::new(config()).unwrap();
    let transition = recovery
        .evaluate_homeostasis(
            &seizure_homeostasis(Tick::new(20)),
            HomeostaticParameters::reference(),
            Tick::new(20),
        )
        .unwrap()
        .unwrap();
    assert_eq!(transition.to, SleepPhase::ForcedRecoverySleep);
    assert_eq!(transition.trigger, SleepTrigger::SeizureHyperactivity);

    let mut catatonia = SleepController::new(config()).unwrap();
    assert_eq!(
        catatonia
            .evaluate_homeostasis(
                &catatonia_homeostasis(Tick::new(30)),
                HomeostaticParameters::reference(),
                Tick::new(30),
            )
            .unwrap()
            .unwrap()
            .trigger,
        SleepTrigger::CatatoniaEnergyHypoplasia
    );
}

#[test]
fn forced_sleep_is_explicit_and_inspectable_without_engine_time() {
    let mut controller = SleepController::new(config()).unwrap();

    let transition = controller
        .force_sleep(Tick::new(5), SleepTrigger::ForcedRequest)
        .unwrap();

    assert_eq!(transition.from, SleepPhase::Awake);
    assert_eq!(transition.to, SleepPhase::ForcedRecoverySleep);
    assert_eq!(controller.state().phase, SleepPhase::ForcedRecoverySleep);
    assert_eq!(controller.state().phase_started_tick, Tick::new(5));
}

#[test]
fn h_shadow_drains_decays_and_promotes_lifetime_without_genetic_mutation() {
    let mut schema = neural_schema_with_h_shadow();
    let before = first_weights(&schema);
    let consolidator = SleepConsolidator::new(config()).unwrap();
    let mut traits = alife_core::LifetimeTraitLedger::new(8).unwrap();
    traits
        .observe(
            alife_core::LifetimeTraitEvidence::new(
                10,
                StableLifetimeTraitKind::MotorHabit,
                NormalizedScalar::new(0.75).unwrap(),
                NormalizedScalar::new(0.01).unwrap(),
                1,
            )
            .unwrap(),
        )
        .unwrap();
    traits
        .observe(
            alife_core::LifetimeTraitEvidence::new(
                10,
                StableLifetimeTraitKind::MotorHabit,
                NormalizedScalar::new(0.76).unwrap(),
                NormalizedScalar::new(0.01).unwrap(),
                2,
            )
            .unwrap(),
        )
        .unwrap();

    let report = consolidator
        .consolidate_neural_schema(&mut schema, &mut traits, Tick::new(100))
        .unwrap();
    let after = first_weights(&schema);

    assert_eq!(after.genetic_fixed, before.genetic_fixed);
    assert!(after.h_operational > before.h_operational);
    assert!(after.h_shadow.abs() < before.h_shadow.abs());
    assert!(after.lifetime_consolidated > before.lifetime_consolidated);
    assert_eq!(after.alpha, 0.0);
    assert_eq!(report.active_synapses, 1);
    assert!(report.h_operational_delta_l1 > 0.0);
    assert!(report.lifetime_delta_l1 > 0.0);
    assert!(report.genetic_layer_unchanged);
}

#[test]
fn trait_promotion_requires_stable_repeated_evidence() {
    let consolidator = SleepConsolidator::new(config()).unwrap();
    let mut insufficient = alife_core::LifetimeTraitLedger::new(8).unwrap();
    insufficient
        .observe(
            alife_core::LifetimeTraitEvidence::new(
                99,
                StableLifetimeTraitKind::TopologyCorrelation,
                NormalizedScalar::new(0.8).unwrap(),
                NormalizedScalar::new(0.01).unwrap(),
                1,
            )
            .unwrap(),
        )
        .unwrap();

    let rejected = consolidator
        .promote_stable_traits(&mut insufficient, Tick::new(40))
        .unwrap();
    assert_eq!(rejected.promoted_count, 0);
    assert_eq!(rejected.insufficient_evidence_count, 1);

    insufficient
        .observe(
            alife_core::LifetimeTraitEvidence::new(
                99,
                StableLifetimeTraitKind::TopologyCorrelation,
                NormalizedScalar::new(0.85).unwrap(),
                NormalizedScalar::new(0.02).unwrap(),
                2,
            )
            .unwrap(),
        )
        .unwrap();
    let promoted = consolidator
        .promote_stable_traits(&mut insufficient, Tick::new(41))
        .unwrap();
    assert_eq!(promoted.promoted_count, 1);
    assert_eq!(insufficient.promoted_traits().len(), 1);
    assert!(insufficient.promoted_traits()[0].confidence.raw() >= 0.8);
}

#[test]
fn memory_compression_is_bounded_and_preserves_expectancy_not_action_replay() {
    let mut bank = memory_bank_with_three_records();
    let consolidator = SleepConsolidator::new(config()).unwrap();

    let report = consolidator.compress_memory_bank(&mut bank).unwrap();

    assert_eq!(report.input_records, 3);
    assert_eq!(report.output_records, 2);
    assert_eq!(bank.len(), 2);
    assert_eq!(
        report.retained_source_memory_ids,
        vec![alife_core::MemoryId(2), alife_core::MemoryId(3)]
    );

    let expectancy = MemoryExpectancy::neutral(Confidence::new(0.05).unwrap()).unwrap();
    let MemoryExpectancy {
        expected_valence: _,
        predicted_drive_delta: _,
        predicted_sensory_outcome: _,
        affordance_bias: _,
        danger_bias: _,
        safety_bias: _,
        social_trust_bias: _,
        social_fear_bias: _,
        novelty_bias: _,
        curiosity_bias: _,
        confidence: _,
        source_memory_ids: _,
    } = expectancy;
}

#[test]
fn concept_consolidation_preserves_gaps_and_cannot_emit_actions() {
    let mut map = topology_with_gap();
    let consolidator = SleepConsolidator::new(config()).unwrap();

    let report = consolidator.consolidate_topology(&mut map, 3).unwrap();

    assert!(report.concepts_considered > 0);
    assert!(report.simplexes_considered > 0);
    assert_eq!(
        report.preserved_gap_count,
        map.unresolved_gaps().len() as u32
    );
    assert!(report.curiosity_bias_count > 0);
    fn topology_report_is_not_action(_: alife_core::ConceptConsolidationReport) {}
    topology_report_is_not_action(report);
}

#[test]
fn structural_edit_batches_validate_sort_and_defer_application() {
    let projection = ProjectionRoutingRef {
        source_lobe: LobeKind::CoreAssociation,
        target_lobe: LobeKind::MotorArbitration,
        projection_type: alife_core::ProjectionType::FeedForward,
    };
    let candidates = vec![
        StructuralEditCandidate::new(
            2,
            projection,
            StructuralEditKind::SynaptogenesisCandidate,
            StructuralEditReason::TopologyCorrelation,
            NormalizedScalar::new(0.7).unwrap(),
            Confidence::new(0.7).unwrap(),
            16,
        )
        .unwrap(),
        StructuralEditCandidate::new(
            1,
            projection,
            StructuralEditKind::PruneMarker,
            StructuralEditReason::LowSalience,
            NormalizedScalar::new(0.2).unwrap(),
            Confidence::new(0.6).unwrap(),
            16,
        )
        .unwrap(),
    ];

    let batch = StructuralEditBatch::new(Tick::new(50), candidates, 3).unwrap();
    assert_eq!(batch.candidates()[0].candidate_id, 1);
    assert_eq!(batch.candidates()[1].candidate_id, 2);
    assert_eq!(
        batch.application_status,
        alife_core::StructuralEditApplicationStatus::DeferredForSleepCompilation
    );

    assert_eq!(
        StructuralEditCandidate::new(
            0,
            projection,
            StructuralEditKind::Strengthen,
            StructuralEditReason::MemoryCorrelation,
            NormalizedScalar::new(0.5).unwrap(),
            Confidence::new(0.5).unwrap(),
            16,
        ),
        Err(ScaffoldContractError::InvalidId)
    );
    assert_eq!(
        StructuralEditBatch::new(
            Tick::new(51),
            vec![
                StructuralEditCandidate::new(
                    1,
                    projection,
                    StructuralEditKind::Strengthen,
                    StructuralEditReason::MemoryCorrelation,
                    NormalizedScalar::new(0.5).unwrap(),
                    Confidence::new(0.5).unwrap(),
                    16,
                )
                .unwrap(),
                StructuralEditCandidate::new(
                    2,
                    projection,
                    StructuralEditKind::Weaken,
                    StructuralEditReason::Fatigue,
                    NormalizedScalar::new(0.5).unwrap(),
                    Confidence::new(0.5).unwrap(),
                    16,
                )
                .unwrap(),
            ],
            1,
        ),
        Err(ScaffoldContractError::ScalarOutOfRange)
    );
}

#[test]
fn structural_candidates_are_generated_but_not_active_applied() {
    let schema = neural_schema_with_h_shadow();
    let map = topology_with_gap();
    let consolidator = SleepConsolidator::new(config()).unwrap();

    let batch = consolidator
        .generate_structural_edit_batch(&schema, &map, Tick::new(80))
        .unwrap();

    assert!(!batch.candidates().is_empty());
    assert_eq!(
        consolidator.reject_active_tick_structural_application(&batch),
        Err(ScaffoldContractError::InvalidSparseProjectionSchema)
    );
}

#[test]
fn creature_mind_can_force_and_run_sleep_without_active_tick_structural_application() {
    let mut mind =
        CreatureMind::scaffold(organism(), BrainScaleTier::Nano512, 44, Tick::ZERO).unwrap();

    let transition = mind
        .force_sleep(Tick::ZERO, SleepTrigger::ForcedRequest)
        .unwrap();
    assert_eq!(transition.to, SleepPhase::ForcedRecoverySleep);
    assert_eq!(mind.sleep_state().phase, SleepPhase::ForcedRecoverySleep);

    let report = mind.run_sleep_consolidation(Tick::new(1)).unwrap();
    assert_eq!(report.sleep_phase, SleepPhase::ForcedRecoverySleep);
    assert!(mind.development_state().sleep_cycle_count >= 1);
    assert!(!mind.pending_structural_edits().is_empty());

    let before_pending = mind.pending_structural_edits().len();
    assert_eq!(mind.current_tick(), Tick::ZERO);
    assert_eq!(mind.pending_structural_edits().len(), before_pending);
}

#[test]
fn sleep_consolidation_contract_stays_engine_independent() {
    let source = include_str!("../src/sleep.rs");
    for forbidden in [
        concat!("be", "vy"),
        concat!("av", "ian"),
        concat!("wg", "pu"),
        concat!("Render", "Device"),
        concat!("Render", "Queue"),
        concat!("Ent", "ity"),
    ] {
        assert!(
            !source.contains(forbidden),
            "sleep consolidation core must not embed engine type {forbidden}"
        );
    }
}
