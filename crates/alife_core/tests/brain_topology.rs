use alife_core::{
    ActiveTilePolicy, BiologicalPriority, BrainClassRegistry, BrainScaleTier, LobeKind,
    ProjectionType, ScaffoldContractError, UpdateCadence,
};

#[test]
fn canonical_registry_exposes_scalable_class_budgets() {
    let tiers = BrainClassRegistry::canonical_tiers();
    assert_eq!(
        tiers,
        &[
            BrainScaleTier::Nano512,
            BrainScaleTier::Small1024,
            BrainScaleTier::Standard2048,
            BrainScaleTier::Large4096,
            BrainScaleTier::Cognitive32768,
            BrainScaleTier::Student131k,
            BrainScaleTier::Ascended1M,
            BrainScaleTier::Ascended5M,
        ]
    );

    let expected_budgets = [
        (BrainScaleTier::Nano512, 512, 8_192, 64),
        (BrainScaleTier::Small1024, 1_024, 16_384, 128),
        (BrainScaleTier::Standard2048, 2_048, 32_768, 192),
        (BrainScaleTier::Large4096, 4_096, 65_536, 384),
    ];

    for (tier, neurons, active_synapses, active_tiles) in expected_budgets {
        let spec = BrainClassRegistry::spec_for_tier(tier).unwrap();
        assert_eq!(spec.neuron_count, neurons);
        assert_eq!(spec.compute_budget.max_active_synapses, active_synapses);
        assert_eq!(spec.compute_budget.max_active_tiles, active_tiles);
        assert!(!spec.active_loop_resizing_allowed);
        spec.validate().unwrap();
    }
}

#[test]
fn standard2048_lobe_topology_matches_reference_boundaries() {
    let spec = BrainClassRegistry::spec_for_tier(BrainScaleTier::Standard2048).unwrap();
    let layout = &spec.lobe_layout;

    let expected = [
        (LobeKind::PerceptualIntegration, 0, 336),
        (LobeKind::InteroceptiveMotivational, 336, 128),
        (LobeKind::MultimodalAssociation, 464, 448),
        (LobeKind::TemporalPredictive, 912, 272),
        (LobeKind::WorkingContextExecutive, 1184, 208),
        (LobeKind::MemoryInterface, 1392, 144),
        (LobeKind::ActionPlanning, 1536, 208),
        (LobeKind::SocialCommunication, 1744, 160),
        (LobeKind::FlexibleReserve, 1904, 144),
    ];

    for (kind, start, len) in expected {
        let region = layout.region(kind).unwrap();
        assert!(region.enabled, "{kind:?} should be enabled");
        assert_eq!(region.id, kind.stable_id());
        assert_eq!(region.start, start);
        assert_eq!(region.len, len);
        assert_eq!(region.end_exclusive(), start + len);
        assert!(!kind.purpose().is_empty());
    }

    assert_eq!(spec.motor_logical_nodes, 208);
    assert_eq!(spec.motor_physical_stride, 256);
    assert_eq!(
        layout.lobe_by_neuron_index(1391).unwrap().kind,
        LobeKind::WorkingContextExecutive
    );
    assert_eq!(
        layout.lobe_by_neuron_index(1536).unwrap().kind,
        LobeKind::ActionPlanning
    );
    assert_eq!(
        layout.lobe_by_neuron_index(1743).unwrap().kind,
        LobeKind::ActionPlanning
    );
    assert_eq!(
        layout.lobe_by_neuron_index(1904).unwrap().kind,
        LobeKind::FlexibleReserve
    );
    assert!(layout.lobe_by_neuron_index(2048).is_none());
}

#[test]
fn all_canonical_layouts_cover_neurons_without_overlap_and_align() {
    for spec in BrainClassRegistry::canonical_specs() {
        spec.validate().unwrap();
        assert_eq!(spec.microtile_edge, 16);
        assert_eq!(spec.supertile_edge, 128);
        assert!(spec.motor_physical_stride >= spec.motor_logical_nodes);
        assert_eq!(spec.motor_logical_nodes % 16, 0);
        assert_eq!(spec.motor_physical_stride % 16, 0);

        let mut cursor = 0;
        for region in spec.lobe_layout.enabled_regions() {
            assert_eq!(region.start, cursor, "{:?} starts after a gap", region.kind);
            assert_eq!(region.start % 16, 0);
            assert_eq!(region.len % 16, 0);
            cursor = region.end_exclusive();
        }
        assert_eq!(cursor, spec.neuron_count);
    }
}

#[test]
fn routing_matrix_references_enabled_lobes_and_uses_budget_metadata() {
    let spec = BrainClassRegistry::spec_for_tier(BrainScaleTier::Standard2048).unwrap();
    let routing = &spec.routing_matrix;
    routing.validate_for_layout(&spec.lobe_layout).unwrap();

    let sensory_to_association = routing
        .route(
            LobeKind::PerceptualIntegration,
            LobeKind::TemporalPredictive,
        )
        .unwrap();
    assert_eq!(
        sensory_to_association.projection_type,
        ProjectionType::FeedForward
    );
    assert_eq!(
        sensory_to_association.active_tile_policy,
        ActiveTilePolicy::EssentialReservation
    );
    assert_eq!(
        sensory_to_association.update_cadence,
        UpdateCadence::Hot60Hz
    );
    assert_eq!(
        sensory_to_association.priority,
        BiologicalPriority::Essential
    );

    let memory_feedback = routing
        .route(LobeKind::MemoryInterface, LobeKind::TemporalPredictive)
        .unwrap();
    assert_eq!(memory_feedback.update_cadence, UpdateCadence::Hot5To15Hz);
    assert_eq!(
        memory_feedback.active_tile_policy,
        ActiveTilePolicy::SalienceGated
    );

    assert!(routing.masks().iter().all(|mask| {
        spec.lobe_layout
            .region(mask.source_lobe)
            .is_some_and(|region| region.enabled)
            && spec
                .lobe_layout
                .region(mask.target_lobe)
                .is_some_and(|region| region.enabled)
    }));
}

#[test]
fn routing_validation_rejects_missing_founder_lobe_references() {
    let spec = BrainClassRegistry::spec_for_tier(BrainScaleTier::Nano512).unwrap();
    let mut invalid_layout = spec.lobe_layout.clone();
    invalid_layout
        .regions
        .retain(|region| region.kind != LobeKind::SocialCommunication);

    assert_eq!(
        spec.routing_matrix.validate_for_layout(&invalid_layout),
        Err(ScaffoldContractError::RoutingReferencesDisabledLobe)
    );
}
