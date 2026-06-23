use alife_core::{
    ActionKind, ActionProposal, ActionTarget, BrainScaleTier, Confidence, ContextStreams, CooEntry,
    CooTile, CreatureMind, ExperiencePatch, HomeostaticDelta, Intensity, NeuralProjectionSchema,
    NormalizedScalar, OrganismId, PhysicalActionOutcome, PhysicalContactKind,
    PostSealHShadowDeltaTarget, PostSealLearningToken, PostSealLifetimeDeltaBatch,
    PostSealLifetimeDeltaRecord, PostSealLifetimeDeltaRejectionReason,
    PostSealLifetimeDeltaSchemaVersion, PostSealLifetimeDeltaSourceKind, PostSealLifetimeLayer,
    ProjectionTile, ReferenceActionExecution, ReferenceActionExecutor, ReferenceOutcomeObservation,
    ReferenceOutcomeObserver, ReferenceOutcomeRequest, ReferenceSensoryAdapter,
    ReferenceSensoryRequest, ScaffoldContractError, SensoryChannels, SensorySnapshot,
    SparseTileCoord, SynapseWeightSplit, Tick, Validate, Vec3f,
};

#[test]
fn valid_post_seal_hshadow_delta_applies_after_sealed_patch() {
    let (mut mind, patch) = prepared_mind_and_patch();
    let batch = batch_for_patch(&mind, &patch, 0.1, 0.25, 0).unwrap();

    let receipt = mind.apply_post_seal_lifetime_deltas(&patch, batch).unwrap();

    assert_eq!(receipt.applied_records, 1);
    assert_eq!(receipt.changed_records, 1);
    assert!(receipt.h_shadow_changed);
    assert!(receipt.genetic_fixed_unchanged);
    assert!(receipt.lifetime_consolidated_unchanged);
    assert!(receipt.h_operational_unchanged);
    assert!(receipt.post_seal_only);
    assert!(receipt.replay_protected);
    let weights = first_weight(&mind);
    assert!((weights.h_shadow - 0.25).abs() < 1.0e-6);
    assert_eq!(weights.genetic_fixed, 1.0);
    assert_eq!(weights.lifetime_consolidated, 0.0);
    assert_eq!(weights.h_operational, 0.0);
}

#[test]
fn missing_sealed_patch_token_rejects() {
    assert!(matches!(
        PostSealLearningToken::from_optional_sealed_patch(None),
        Err(ScaffoldContractError::MissingPhaseData)
    ));
}

#[test]
fn unsealed_patch_rejects() {
    let (_mind, patch) = prepared_mind_and_patch();
    let mut value = serde_json::to_value(&patch).unwrap();
    value["header"]["phase"] = serde_json::Value::String("PostActionOutcome".to_string());
    let unsealed: ExperiencePatch = serde_json::from_value(value).unwrap();

    assert!(matches!(
        PostSealLearningToken::from_sealed_patch(&unsealed),
        Err(ScaffoldContractError::UnorderedExperiencePhase)
    ));
}

#[test]
fn wrong_organism_rejects() {
    let (mut mind, patch) = prepared_mind_and_patch();
    let mut batch = batch_for_patch(&mind, &patch, 0.1, 0.2, 0).unwrap();
    batch.organism_id = OrganismId(99);

    assert!(matches!(
        mind.apply_post_seal_lifetime_deltas(&patch, batch),
        Err(ScaffoldContractError::InvalidId)
    ));
}

#[test]
fn wrong_tick_or_sequence_rejects() {
    let (mut mind, patch) = prepared_mind_and_patch();
    let mut wrong_tick = batch_for_patch(&mind, &patch, 0.1, 0.2, 0).unwrap();
    wrong_tick.originating_tick = Tick::new(99);
    assert!(matches!(
        mind.apply_post_seal_lifetime_deltas(&patch, wrong_tick),
        Err(ScaffoldContractError::InvalidId)
    ));

    let mut wrong_sequence = batch_for_patch(&mind, &patch, 0.1, 0.2, 0).unwrap();
    wrong_sequence.sealed_sequence_id = alife_core::ExperienceSequenceId(99);
    assert!(matches!(
        mind.apply_post_seal_lifetime_deltas(&patch, wrong_sequence),
        Err(ScaffoldContractError::InvalidId)
    ));
}

#[test]
fn duplicate_or_replayed_application_rejects() {
    let (mut mind, patch) = prepared_mind_and_patch();
    let batch = batch_for_patch(&mind, &patch, 0.1, 0.2, 0).unwrap();
    mind.apply_post_seal_lifetime_deltas(&patch, batch).unwrap();

    let replay = batch_for_patch(&mind, &patch, 0.2, 0.3, 0).unwrap();
    assert!(matches!(
        mind.apply_post_seal_lifetime_deltas(&patch, replay),
        Err(ScaffoldContractError::NonMonotonicTick)
    ));
}

#[test]
fn nan_inf_and_out_of_range_values_reject() {
    let (mind, patch) = prepared_mind_and_patch();
    assert!(PostSealLifetimeDeltaRecord::h_shadow(
        PostSealHShadowDeltaTarget::new(0, 0, 0),
        f32::NAN,
        0.2,
        -1.0,
        1.0,
    )
    .is_err());
    assert!(PostSealLifetimeDeltaRecord::h_shadow(
        PostSealHShadowDeltaTarget::new(0, 0, 0),
        0.1,
        f32::INFINITY,
        -1.0,
        1.0,
    )
    .is_err());
    assert!(batch_for_patch(&mind, &patch, 0.1, 5.0, 0).is_err());
}

#[test]
fn duplicate_target_index_rejects() {
    let (mind, patch) = prepared_mind_and_patch();
    let one = PostSealLifetimeDeltaRecord::h_shadow(
        PostSealHShadowDeltaTarget::new(0, 0, 0),
        0.1,
        0.2,
        -1.0,
        1.0,
    )
    .unwrap();
    let two = PostSealLifetimeDeltaRecord::h_shadow(
        PostSealHShadowDeltaTarget::new(0, 0, 0),
        0.1,
        0.3,
        -1.0,
        1.0,
    )
    .unwrap();
    assert!(PostSealLifetimeDeltaBatch::new(
        OrganismId(7),
        mind.brain_class().id,
        mind.brain_class().neuron_count,
        mind.brain_class().max_active_synapses,
        patch.header().world_tick,
        patch.header().sequence_id,
        PostSealLifetimeDeltaSourceKind::GpuCpuShadowGuarded,
        true,
        true,
        true,
        true,
        vec![one, two],
    )
    .is_err());
}

#[test]
fn parity_failed_or_wrong_layer_batch_rejects() {
    let (mind, patch) = prepared_mind_and_patch();
    let mut batch = batch_for_patch(&mind, &patch, 0.1, 0.2, 0).unwrap();
    batch.cpu_shadow_parity_passed = false;
    assert!(batch.validate_contract().is_err());

    let wrong_layer = PostSealLifetimeDeltaRecord {
        layer: PostSealLifetimeLayer::LifetimePlastic,
        target: PostSealHShadowDeltaTarget::new(0, 0, 0),
        before_value: 0.1,
        after_value: 0.2,
        min_value: -1.0,
        max_value: 1.0,
    };
    assert!(wrong_layer.validate_contract().is_err());
}

#[test]
fn batch_size_cap_is_enforced() {
    let (mind, patch) = prepared_mind_and_patch();
    let record = PostSealLifetimeDeltaRecord::h_shadow(
        PostSealHShadowDeltaTarget::new(0, 0, 0),
        0.1,
        0.2,
        -1.0,
        1.0,
    )
    .unwrap();
    let mut records = vec![record; alife_core::POST_SEAL_LIFETIME_DELTA_MAX_RECORDS + 1];
    for (index, record) in records.iter_mut().enumerate() {
        record.target.synapse_index = index as u16;
    }
    assert!(PostSealLifetimeDeltaBatch::new(
        OrganismId(7),
        mind.brain_class().id,
        mind.brain_class().neuron_count,
        mind.brain_class().max_active_synapses,
        patch.header().world_tick,
        patch.header().sequence_id,
        PostSealLifetimeDeltaSourceKind::GpuCpuShadowGuarded,
        true,
        true,
        true,
        true,
        records,
    )
    .is_err());
}

#[test]
fn schema_version_type_is_current_and_no_engine_types_are_required() {
    assert_eq!(
        PostSealLifetimeDeltaSchemaVersion::CURRENT.raw(),
        alife_core::POST_SEAL_LIFETIME_DELTA_SCHEMA_VERSION
    );
    let _reason = PostSealLifetimeDeltaRejectionReason::MissingSealedPatch;
}

fn prepared_mind_and_patch() -> (CreatureMind, ExperiencePatch) {
    let mut mind =
        CreatureMind::scaffold(OrganismId(7), BrainScaleTier::Nano512, 42, Tick::ZERO).unwrap();
    mind.initialize_neural_projection_schema(test_schema(0.1))
        .unwrap();
    let output = mind.tick(
        alife_core::BrainTickInput::new(Tick::ZERO, vec![proposal()]),
        &mut TestSensory,
        &mut TestExecutor,
        &mut TestObserver,
    );
    (mind, output.experience_patch.unwrap())
}

fn test_schema(h_shadow: f32) -> NeuralProjectionSchema {
    let spec = alife_core::BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 1).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(1.0, 0.0, 0.5, 0.0, h_shadow)).unwrap(),
            CooEntry::new(1, 1, weights(1.0, 0.0, 0.5, 0.0, h_shadow)).unwrap(),
        ])
        .unwrap(),
    ));
    schema.rebuild_supertile_masks();
    schema
}

fn first_weight(mind: &CreatureMind) -> SynapseWeightSplit {
    let tile = &mind.neural_projection_schema().projections[0].tiles[0];
    match &tile.payload {
        alife_core::SparseTilePayload::Coo(coo) => coo.entries[0].weights,
        _ => panic!("test fixture uses COO"),
    }
}

fn batch_for_patch(
    mind: &CreatureMind,
    patch: &ExperiencePatch,
    before: f32,
    after: f32,
    synapse_index: u16,
) -> Result<PostSealLifetimeDeltaBatch, ScaffoldContractError> {
    PostSealLifetimeDeltaBatch::new(
        OrganismId(7),
        mind.brain_class().id,
        mind.brain_class().neuron_count,
        mind.brain_class().max_active_synapses,
        patch.header().world_tick,
        patch.header().sequence_id,
        PostSealLifetimeDeltaSourceKind::GpuCpuShadowGuarded,
        true,
        true,
        true,
        true,
        vec![PostSealLifetimeDeltaRecord::h_shadow(
            PostSealHShadowDeltaTarget::new(0, 0, synapse_index),
            before,
            after,
            -1.0,
            1.0,
        )?],
    )
}

fn weights(
    genetic_fixed: f32,
    lifetime_consolidated: f32,
    alpha: f32,
    h_operational: f32,
    h_shadow: f32,
) -> SynapseWeightSplit {
    SynapseWeightSplit::new(
        genetic_fixed,
        lifetime_consolidated,
        alpha,
        h_operational,
        h_shadow,
    )
    .unwrap()
}

fn proposal() -> ActionProposal {
    ActionProposal::new(
        ActionKind::Inspect.canonical_id(),
        ActionKind::Inspect,
        0.8,
        Confidence::new(0.9).unwrap(),
        None,
        0,
        ActionTarget::NONE,
        NormalizedScalar::new(0.5).unwrap(),
    )
    .unwrap()
    .with_intensity(Intensity::new(0.25).unwrap())
}

struct TestSensory;

impl ReferenceSensoryAdapter for TestSensory {
    fn gather_sensory(
        &mut self,
        request: ReferenceSensoryRequest,
    ) -> Result<SensorySnapshot, ScaffoldContractError> {
        SensorySnapshot::new(
            request.organism_id,
            request.tick,
            Vec3f::ZERO,
            SensoryChannels::default(),
            ContextStreams::default(),
        )
    }
}

struct TestExecutor;

impl ReferenceActionExecutor for TestExecutor {
    fn execute_action(
        &mut self,
        _command: &alife_core::ActionCommand,
    ) -> Result<ReferenceActionExecution, ScaffoldContractError> {
        ReferenceActionExecution::succeeded(PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.0).unwrap(),
        })
    }
}

struct TestObserver;

impl ReferenceOutcomeObserver for TestObserver {
    fn observe_outcome(
        &mut self,
        _request: ReferenceOutcomeRequest<'_>,
    ) -> Result<ReferenceOutcomeObservation, ScaffoldContractError> {
        ReferenceOutcomeObservation::new(
            true,
            HomeostaticDelta::zero(),
            alife_core::SignedValence::new(0.1).unwrap(),
            NormalizedScalar::new(0.0).unwrap(),
            NormalizedScalar::new(0.0).unwrap(),
            alife_core::SignedValence::new(0.0).unwrap(),
            NormalizedScalar::new(0.0).unwrap(),
        )
    }
}
