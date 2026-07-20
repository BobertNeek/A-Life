use std::collections::BTreeMap;

use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, BrainClassSpec, BrainGenome,
    BrainScaleTier, CandidateActionFamily, CandidateObservationRef, Confidence, DecisionSnapshot,
    DevelopmentState, DurationTicks, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    GroundedObjectSlotV1, HomeostaticDelta, HomeostaticSnapshot, LobeKind, MemoryBank,
    MemoryBankConfig, NeuralActionSelection, NormalizedScalar, OrganismId, PerceptionFrameDraft,
    PhenotypeHash, PhysicalActionOutcome, PhysicalContactKind, Pose, PostActionOutcome,
    PreActionSnapshot, SensorProfile, SensorProfileProvenance, SensoryAbiVersion, SensoryChannels,
    SensorySnapshot, SignedValence, Tick, TopologicalMapConfig, TopologyDegradationKind,
    TopologySidecar, TrackedObjectId, Vec3f, Velocity, WorldEntityId,
};

fn tiny_config() -> TopologicalMapConfig {
    TopologicalMapConfig {
        max_concepts: 2,
        max_edges: 2,
        max_simplexes: 2,
        max_unresolved_gaps: 1,
        edge_decay_per_tick: NormalizedScalar::new(0.0).unwrap(),
    }
}

fn grounded_profile() -> alife_core::SensorProfileIdentity {
    SensorProfileProvenance::new(
        SensorProfile::GroundedObjectSlotsV1,
        SensoryAbiVersion::CURRENT,
        Tick::new(1),
    )
    .unwrap()
    .identity()
}

fn grounded_sidecar(owner: u64) -> TopologySidecar {
    TopologySidecar::new_profiled(OrganismId(owner), grounded_profile(), tiny_config()).unwrap()
}

fn tracked_patch(owner: u64, sequence_raw: u64, tracked_raw: u64) -> ExperiencePatch {
    let organism_id = OrganismId(owner);
    let sequence = ExperienceSequenceId(sequence_raw);
    let tick = Tick::new(sequence_raw);
    let slot = GroundedObjectSlotV1 {
        slot_index: 0,
        tracked_object_id: TrackedObjectId(tracked_raw),
        bearing: [0.2, 0.0],
        distance: 0.5,
        relative_velocity: [0.0; 3],
        color: [0.2, 0.5, 0.8],
        material: [0.3, 0.4, 0.5],
        shape: [0.1, 0.6, 0.9],
        chemical: [0.0, 0.2, 0.0],
        contact: 0.0,
        proprioception: [0.0; 2],
        temperature: 0.1,
        terrain: [0.5, 0.25],
        confidence: Confidence::new(0.9).unwrap(),
    };
    let candidate = ActionCandidate::new(
        0,
        ActionId(200),
        ActionKind::Interact,
        CandidateActionFamily::Ingest,
        CandidateObservationRef::ObjectSlot(0),
        ActionTarget::new(
            Some(WorldEntityId(9_000 + tracked_raw)),
            Some(Vec3f::new(1.0, 0.0, 0.0)),
        ),
        slot.candidate_features().unwrap(),
        Confidence::new(0.9).unwrap(),
        NormalizedScalar::new(0.2).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(2),
    )
    .unwrap();
    let draft = PerceptionFrameDraft::new(
        organism_id,
        tick,
        SensorProfile::GroundedObjectSlotsV1,
        SensorySnapshot::new(
            organism_id,
            tick,
            Vec3f::ZERO,
            SensoryChannels::ZERO,
            Default::default(),
        )
        .unwrap(),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![candidate],
        SensorProfileProvenance::new(
            SensorProfile::GroundedObjectSlotsV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        vec![slot],
    )
    .unwrap();
    let memory = MemoryBank::new(
        MemoryBankConfig::new(4, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let (frame, recall) = memory
        .recall_frame(&draft)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let command = frame.candidates()[0]
        .to_command(organism_id, Confidence::new(0.8).unwrap())
        .unwrap();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence,
        PhenotypeHash([1, 2, 3, 4]),
        sequence_raw,
        (sequence_raw & 1) as u8,
        &frame,
        NeuralActionSelection {
            candidate_index: 0,
            logit: 0.7,
            confidence: Confidence::new(0.8).unwrap(),
            active_tiles: 8,
            active_synapses: 64,
        },
        command,
    )
    .unwrap()
    .with_finalized_memory_recall(&frame, &recall, 0)
    .unwrap();
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let genome = BrainGenome::scaffold(321 ^ owner, spec.id);
    let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.5).unwrap())
        .with_enabled_lobes([
            LobeKind::SensoryGrounding,
            LobeKind::CoreAssociation,
            LobeKind::MotorArbitration,
        ]);
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence,
        spec.id,
        PhenotypeHash([1, 2, 3, 4]),
        genome.id,
        genome.schema_version,
        development,
        frame,
    )
    .unwrap();
    let outcome = PostActionOutcome::new(
        organism_id,
        sequence,
        Tick::new(sequence_raw + 1),
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Touch,
            target_entity: Some(WorldEntityId(9_000 + tracked_raw)),
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta::zero(),
        SignedValence::new(0.2).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        SignedValence::new(-0.1).unwrap(),
        NormalizedScalar::new(0.2).unwrap(),
    )
    .unwrap();
    ExperiencePatchBuilder::new(sequence)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}

#[test]
fn ten_thousand_topology_observations_stay_bounded_and_never_return_capacity_error() {
    let mut sidecar = grounded_sidecar(7);
    let mut degraded = 0_u64;
    for index in 1..=10_240_u64 {
        let receipt = sidecar.observe_sealed_patch(&tracked_patch(7, index, 10_000 + index));
        degraded += u64::from(!receipt.degradations.is_empty());
        assert!(receipt.after_counts.within(sidecar.config()));
        assert!(!receipt.rejected_invalid);
    }
    assert!(degraded > 0);
    assert_eq!(sidecar.diagnostics().terminal_errors, 0);
}

#[test]
fn topology_receipt_has_no_action_or_score_output() {
    let source = include_str!("../src/topology.rs");
    assert!(!source.contains(concat!("Action", "Command")));
    assert!(!source.contains("score_delta"));
    assert!(!source.contains("candidate_logits"));
}

#[test]
fn topology_sidecars_are_owned_and_isolated_by_organism() {
    let mut sidecars = BTreeMap::<u64, TopologySidecar>::new();
    sidecars.insert(7, grounded_sidecar(7));
    sidecars.insert(9, grounded_sidecar(9));
    let before_nine = sidecars[&9].diagnostics().canonical_digest;
    let receipt = sidecars
        .get_mut(&7)
        .unwrap()
        .observe_sealed_patch(&tracked_patch(7, 1, 71));
    assert_eq!(receipt.organism_id_raw, 7);
    assert_eq!(sidecars[&9].diagnostics().canonical_digest, before_nine);
    let rejected = sidecars
        .get_mut(&7)
        .unwrap()
        .observe_sealed_patch(&tracked_patch(9, 2, 72));
    assert!(rejected.rejected_invalid);
    assert_eq!(rejected.before_digest, rejected.after_digest);
}

#[test]
fn topology_rejects_duplicate_and_out_of_order_sealed_sequences_atomically() {
    let mut sidecar = grounded_sidecar(7);
    let patch = tracked_patch(7, 9, 79);
    let first = sidecar.observe_sealed_patch(&patch);
    let duplicate = sidecar.observe_sealed_patch(&patch);
    let stale = sidecar.observe_sealed_patch(&tracked_patch(7, 8, 78));
    assert!(!first.replay_rejected);
    assert!(duplicate.replay_rejected);
    assert!(stale.replay_rejected);
    assert_eq!(duplicate.before_digest, duplicate.after_digest);
    assert_eq!(stale.before_digest, stale.after_digest);
    assert_eq!(duplicate.before_counts, duplicate.after_counts);
    assert_eq!(stale.before_counts, stale.after_counts);
}

#[test]
fn topology_uses_only_sealed_tracked_bindings_for_object_identity() {
    let source = include_str!("../src/topology.rs");
    assert!(!source.contains("ConceptSignature::Object"));
    assert!(!source.contains("objects: Vec<WorldEntityId>"));
    assert!(source.contains("ConceptSignature::TrackedObject"));
    assert!(source.contains("episodic_key"));
}

#[test]
fn invalid_observation_is_atomic() {
    let mut sidecar = grounded_sidecar(7);
    let before = sidecar.diagnostics();
    let rejected = sidecar.observe_sealed_patch(&tracked_patch(9, 1, 71));
    assert!(rejected
        .degradations
        .contains(&TopologyDegradationKind::InvalidObservationRejected));
    assert!(rejected.rejected_invalid);
    assert_eq!(rejected.before_digest, rejected.after_digest);
    assert_eq!(rejected.before_counts, rejected.after_counts);
    assert_eq!(
        sidecar.diagnostics().canonical_digest,
        before.canonical_digest
    );
}

#[test]
fn portable_topology_asset_roundtrips_replay_guard_and_rejects_tampering() {
    let profile = grounded_profile();
    let mut sidecar = TopologySidecar::new_profiled(OrganismId(7), profile, tiny_config()).unwrap();
    sidecar.observe_sealed_patch(&tracked_patch(7, 1, 71));
    let asset = sidecar.export_portable().unwrap();

    assert_eq!(asset.organism_id_raw, 7);
    assert_eq!(asset.profile, profile);
    assert_eq!(asset.last_observed_sequence_id_raw, 1);
    assert!(!asset.concepts.is_empty());
    let restored = TopologySidecar::restore_portable(asset.clone()).unwrap();
    assert_eq!(restored.counts(), sidecar.counts());
    assert_eq!(restored.next_ids(), sidecar.next_ids());
    assert_eq!(restored.diagnostics(), sidecar.diagnostics());
    let replay = restored
        .clone()
        .observe_sealed_patch(&tracked_patch(7, 1, 71));
    assert!(replay.replay_rejected);

    let mut tampered = asset;
    tampered.concepts[0].bindings.tracked_object_ids_raw[0] ^= 1;
    assert_eq!(
        TopologySidecar::restore_portable(tampered).unwrap_err(),
        alife_core::ScaffoldContractError::InvalidMemoryQuery
    );
}
