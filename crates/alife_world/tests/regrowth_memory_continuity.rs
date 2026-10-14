//! Portable-component continuation, not a complete GPU checkpoint or UI Save/Load test.
//! The selection receipt below is controlled fixture data; no neural backend runs.

use alife_core::{
    ActionCandidate, ActionCommand, ActionKind, ActionTarget, BiochemistryState,
    BrainCapacityClass, BrainScaleTier, CandidateActionFamily, CandidateMemoryContextV1,
    CandidateObservationRef, Confidence, CreatureGenome, DecisionSnapshot, DevelopmentState,
    DurationTicks, ExperiencePatchBuilder, ExperienceSequenceId, FinalizedMemoryRecall,
    FoundationGeneticIdentity, Intensity, LobeKind, MeasuredPhysiologyTransition, MemoryBankConfig,
    MemoryCompactionPhase, MemoryId, MemorySidecarState, NeuralActionSelection, NormalizedScalar,
    OrganismId, PerceptionFrameDraft, PhenotypeHash, PhysicalContactKind,
    PortableMemoryBankAssetV2, PostActionOutcome, PreActionSnapshot, SensorProfile,
    SensorProfileIdentity, SensoryAbiVersion, SignedValence, Tick, TrackedObjectId, Vec3f,
    WorldEntityId, MEMORY_MIN_SIMILARITY, MEMORY_TARGET_RANGE,
};
use alife_world::{
    persistence::{
        AssetManifest, CreatureMindSaveSummary, CreatureSaveState, GpuBrainAssetRef,
        LearningTraceSaveSummary, MemorySidecarSaveState, PortableAssetDigest, PortableSaveFile,
        RuntimeConfig, WeightLayerSaveSummary,
    },
    EcologyZoneId, GroundedPhysicalProperties, HabitatAuthority, HabitatId, HeadlessActionIds,
    HeadlessScenarioBuilder, HeadlessWorld, TerrainZoneKind, TrackedObjectRegistrySaveState,
    WorldEditorSpawnSpec, WorldObjectKind, WorldOrganismRecord,
};

const ORGANISM: OrganismId = OrganismId(707);
const SEED: u64 = 7_070;
const SEQUENCE: ExperienceSequenceId = ExperienceSequenceId(1);
const PROFILE: SensorProfile = SensorProfile::GroundedObjectSlotsV1;

fn profile() -> SensorProfileIdentity {
    SensorProfileIdentity {
        profile_id: PROFILE.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    }
}

fn world_with_hungry_organism() -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(SEED)
        .agent("creature", ORGANISM, Vec3f::ZERO)
        .food("decoy", Vec3f::new(0.75, 0.0, 0.0), 0.2)
        .terrain_zone(
            1,
            "meadow",
            TerrainZoneKind::Meadow,
            Vec3f::ZERO,
            3.0,
            0.8,
            0.0,
        )
        .build()
        .unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        321,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let mut biology = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    biology.body.set_energy(0.1).unwrap();
    let record = WorldOrganismRecord::new(
        ORGANISM,
        world.entity_id("creature").unwrap(),
        genome,
        phenotype,
        biology,
        Tick::ZERO,
    )
    .unwrap();
    world.register_organism_record(record).unwrap();
    // Match the actual current save contract before branching: the save constructor
    // otherwise populates a previously unassigned default habitat membership.
    let mut habitats = HabitatAuthority::default();
    habitats
        .register_creature(ORGANISM, HabitatId::DEFAULT_WILD, Tick::ZERO)
        .unwrap();
    world.replace_habitat_authority(habitats).unwrap();
    // Let actual biology express low reserves in its homeostasis before the meal.
    for _ in 0..3 {
        world.try_advance_tick().unwrap();
    }
    assert_alive(&world);
    world
}

fn assert_alive(world: &HeadlessWorld) {
    let record = world.organism_registry().get(ORGANISM).unwrap();
    assert!(record.lifecycle().is_alive());
    assert!(record.biochemistry().body.health > 0.0);
    assert_eq!(record.biochemistry().tick, world.tick());
}

fn sense(world: &mut HeadlessWorld) -> PerceptionFrameDraft {
    let tick = world.tick();
    let homeostasis = world
        .organism_registry()
        .get(ORGANISM)
        .unwrap()
        .biochemistry()
        .homeostasis;
    world
        .perception_frame_draft(ORGANISM, tick, PROFILE, homeostasis)
        .unwrap()
}

fn ingest_candidate(draft: &PerceptionFrameDraft, food: WorldEntityId) -> ActionCandidate {
    *draft
        .candidates()
        .iter()
        .find(|candidate| {
            candidate.family == CandidateActionFamily::Ingest
                && candidate.target.entity == Some(food)
        })
        .expect("food must have its own grounded Ingest candidate")
}

fn candidate_tracked_id(
    draft: &PerceptionFrameDraft,
    candidate: ActionCandidate,
) -> TrackedObjectId {
    let CandidateObservationRef::ObjectSlot(slot_index) = candidate.observation else {
        panic!("food must bind to an actual object slot")
    };
    draft
        .grounded_object_slots()
        .iter()
        .find(|slot| slot.slot_index == slot_index)
        .unwrap()
        .tracked_object_id
}

fn tracked_id(world: &HeadlessWorld, food: WorldEntityId) -> TrackedObjectId {
    let key = world.entity(food).unwrap().tracking_key;
    world
        .tracked_objects()
        .save_state(ORGANISM)
        .unwrap()
        .records
        .iter()
        .find(|record| record.tracking_key == key)
        .unwrap()
        .tracked_object_id
}

fn meal_cues() -> GroundedPhysicalProperties {
    GroundedPhysicalProperties {
        velocity: Vec3f::ZERO,
        color: [1.0, 0.0, 0.0],
        material: [0.3, 0.4, 0.5],
        shape: [0.6, 0.2, 0.4],
        chemical: [0.8, 0.1, 0.2],
        surface_temperature: 0.1,
        terrain: [0.5, 0.25],
    }
}

fn spawn_food(
    world: &mut HeadlessWorld,
    label: &str,
    harmful: bool,
    position: Vec3f,
    radius: f32,
) -> WorldEntityId {
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: label.to_string(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position,
            nutrition: if harmful { 0.05 } else { 0.6 },
            hazard_pain: if harmful { 0.35 } else { 0.0 },
            radius,
            token_id: None,
        })
        .unwrap()
}

fn creature_summary(
    world: &HeadlessWorld,
    active: &PortableMemoryBankAssetV2,
) -> CreatureSaveState {
    let record = world.organism_registry().get(ORGANISM).unwrap();
    let biology = record.biochemistry();
    CreatureSaveState {
        organism_id: ORGANISM,
        genome_id: record.genome().id,
        brain_class: BrainScaleTier::Nano512,
        development_tick: biology.development.last_update_tick,
        appearance: alife_world::CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick: biology.tick,
            homeostasis: biology.homeostasis,
            memory_record_count: u32::try_from(active.records.len()).unwrap(),
            memory_source_ids: active
                .records
                .iter()
                .map(|record| MemoryId(record.memory_id_raw))
                .collect(),
            concept_count: 0,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".to_string(),
            diagnostics: vec![
                "test-owned portable memory components; no GPU brain envelope".to_string(),
            ],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest: "fnv1a64:0000000000000001".to_string(),
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: 0,
            h_operational_entries: 0,
            h_shadow_entries: 0,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: true,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: None,
        },
        composite_genetics: None,
        lifetime_state_asset: None,
        gpu_brain: None,
    }
}

fn recall_context(
    recall: &FinalizedMemoryRecall,
    candidate: ActionCandidate,
) -> CandidateMemoryContextV1 {
    let context = recall
        .context()
        .candidates
        .iter()
        .find(|context| context.candidate_index == candidate.candidate_index)
        .unwrap();
    *context
}

fn isolated_ingest_recall(
    memory: &MemorySidecarState,
    draft: &PerceptionFrameDraft,
    mut candidate: ActionCandidate,
    category_reads: u32,
) -> FinalizedMemoryRecall {
    assert_eq!(candidate.family, CandidateActionFamily::Ingest);
    assert!(draft.candidates().contains(&candidate));
    // Keep the real sensed state and candidate; isolate only frame-local receipt accounting.
    candidate.candidate_index = 0;
    let diagnostic = PerceptionFrameDraft::new(
        draft.organism_id(),
        draft.tick(),
        draft.sensor_profile(),
        draft.sensory().clone(),
        draft.body(),
        *draft.homeostasis(),
        vec![candidate],
        draft.profile_provenance(),
        draft.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let (_, recall) = memory
        .recall_frame(&diagnostic)
        .unwrap()
        .finalize(diagnostic)
        .unwrap();
    let receipt = recall.receipt();
    assert_eq!(
        recall.candidate_keys()[0].query().tracked_object_id(),
        Some(candidate_tracked_id(draft, candidate))
    );
    assert_eq!(receipt.candidate_count, 1);
    assert_eq!(receipt.exact_bucket_reads, 2);
    // Current tracked namespaces have no bin neighbors: these reads are category fallbacks.
    assert_eq!(receipt.neighbor_bucket_reads, category_reads);
    assert_eq!(receipt.candidates[0].target_reused_from, None);
    assert_eq!(receipt.candidates[0].family_reused_from, None);
    recall
}

fn assert_same_bin_guard_preconditions(query: &[f32], episode: &[f32]) {
    for lane in (6..18).chain(std::iter::once(21)) {
        let index = MEMORY_TARGET_RANGE.start + lane;
        assert_eq!((query[index] * 4.0).round(), (episode[index] * 4.0).round());
    }
    let chemical_lane = MEMORY_TARGET_RANGE.start + 16;
    assert_eq!(episode[chemical_lane], 0.1);
    assert_eq!(query[chemical_lane], -0.12);
    assert!((query[chemical_lane] - episode[chemical_lane]).abs() > 0.2);
    // Mirror the frozen recall policy only to establish this negative probe's reachability.
    let cosine = |start: usize, end: usize| {
        let (mut dot, mut query_norm, mut episode_norm) = (0.0_f32, 0.0_f32, 0.0_f32);
        for index in start..end {
            dot += query[index] * episode[index];
            query_norm += query[index] * query[index];
            episode_norm += episode[index] * episode[index];
        }
        if query_norm == 0.0 && episode_norm == 0.0 {
            1.0
        } else if query_norm == 0.0 || episode_norm == 0.0 {
            0.0
        } else {
            (dot / (query_norm.sqrt() * episode_norm.sqrt())).clamp(-1.0, 1.0)
        }
    };
    for lane in (6..18).step_by(3) {
        let start = MEMORY_TARGET_RANGE.start + lane;
        assert!(cosine(start, start + 3) >= 0.9);
    }
    let target_similarity = 0.45 * cosine(0, 40) + 0.50 * cosine(57, 81) + 0.05 * cosine(81, 83);
    let family_similarity = 0.30 * cosine(0, 40)
        + 0.10 * cosine(40, 49)
        + 0.20 * cosine(49, 57)
        + 0.35 * cosine(57, 81)
        + 0.05 * cosine(81, 83);
    assert!(target_similarity >= MEMORY_MIN_SIMILARITY);
    assert!(family_similarity >= MEMORY_MIN_SIMILARITY);
}

fn assert_recalled_meal(
    context: CandidateMemoryContextV1,
    source: MemoryId,
    harmful: bool,
    expected_target_latent: &[f32],
) {
    assert_eq!(context.target_source_count, 1);
    assert_eq!(context.family_source_count, 1);
    assert_eq!(context.best_target_source, Some(source));
    assert_eq!(context.best_family_source, Some(source));
    assert_eq!(context.target_latent.as_slice(), expected_target_latent);
    for confidence in [context.target_confidence, context.family_confidence] {
        assert!(confidence.raw().is_finite());
        assert!((0.0..=1.0).contains(&confidence.raw()));
        assert!(confidence.raw() > 0.0);
    }
    if harmful {
        assert!(context.family_value[0] < 0.0);
        assert!(context.family_value[2] > 0.0);
    } else {
        assert!(context.family_value[0] > 0.0);
    }
}

#[test]
fn regrown_food_retains_individual_memory_after_consumed_portable_component_save() {
    for harmful in [false, true] {
        run_meal_continuation(harmful);
    }
}

fn run_meal_continuation(harmful: bool) {
    let mut control = world_with_hungry_organism();
    let decoy = control.entity_id("decoy").unwrap();
    let _decoy_draft = sense(&mut control);
    let decoy_tracked = tracked_id(&control, decoy);
    control
        .editor_move_object(decoy, Vec3f::new(40.0, 0.0, 1.0))
        .unwrap();
    let food = spawn_food(
        &mut control,
        "episode-food",
        harmful,
        Vec3f::new(0.25, 0.0, 0.0),
        0.5,
    );
    control
        .set_grounded_physical_properties(food, meal_cues())
        .unwrap();
    control
        .track_resource_lifecycle(food, EcologyZoneId(1), 4, 12)
        .unwrap();
    let initial_draft = sense(&mut control);
    let original_tracked =
        candidate_tracked_id(&initial_draft, ingest_candidate(&initial_draft, food));
    assert_eq!(original_tracked, tracked_id(&control, food));
    assert_ne!(original_tracked, decoy_tracked);
    let original_provenance = control.entity(food).unwrap().tracking_provenance;
    let original_key = control.entity(food).unwrap().tracking_key;
    let agent = control.entity_id("creature").unwrap();
    let grab = ActionCommand::structured(
        ORGANISM,
        HeadlessActionIds::GRAB,
        ActionKind::Hold,
        ActionTarget::new(Some(food), None),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(1.0).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    let grab_tick = Tick::new(control.tick().raw() + 1);
    let grabbed = control
        .apply_registered_command(&grab, agent, grab_tick)
        .unwrap();
    assert!(grabbed.action_result.execution.succeeded);
    assert_eq!(control.entity(food).unwrap().carried_by, Some(ORGANISM));
    assert_eq!(control.try_advance_tick().unwrap(), grab_tick);

    let mut memory = MemorySidecarState::new_profiled(
        ORGANISM,
        profile(),
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let draft = sense(&mut control);
    let selected = ingest_candidate(&draft, food);
    assert_eq!(candidate_tracked_id(&draft, selected), original_tracked);
    let (frame, finalized) = memory
        .recall_frame(&draft)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let episode_query_features = finalized.candidate_keys()[usize::from(selected.candidate_index)]
        .query()
        .features()
        .to_vec();
    let confidence = Confidence::new(0.8).unwrap();
    let eat = selected.to_command(ORGANISM, confidence).unwrap();
    // Controlled receipt binds the real candidate; it is not evidence of a GPU choice.
    let phenotype_hash = PhenotypeHash([1, 2, 3, 4]);
    let decision = DecisionSnapshot::from_neural_selection(
        SEQUENCE,
        phenotype_hash,
        1,
        0,
        &frame,
        NeuralActionSelection {
            candidate_index: selected.candidate_index,
            logit: 0.7,
            confidence,
            active_tiles: 8,
            active_synapses: 64,
        },
        eat,
    )
    .unwrap()
    .with_finalized_memory_recall(&frame, &finalized, selected.candidate_index)
    .unwrap();
    let brain_genome = control
        .organism_registry()
        .get(ORGANISM)
        .unwrap()
        .phenotype()
        .brain_genome
        .clone();
    let development = DevelopmentState::new(
        brain_genome.id,
        frame.tick(),
        NormalizedScalar::new(0.5).unwrap(),
    )
    .with_enabled_lobes([
        LobeKind::PerceptualIntegration,
        LobeKind::TemporalPredictive,
        LobeKind::ActionPlanning,
    ]);
    let pre_action = PreActionSnapshot::from_neural_frame(
        SEQUENCE,
        BrainCapacityClass::N512_ID,
        phenotype_hash,
        brain_genome.id,
        brain_genome.schema_version,
        development,
        frame,
    )
    .unwrap();
    let outcome_tick = Tick::new(control.tick().raw() + 1);
    let receipt = control
        .apply_registered_command(&eat, agent, outcome_tick)
        .unwrap();
    let result = &receipt.action_result;
    assert!(result.execution.succeeded);
    assert_eq!(
        result.execution.physical.contact,
        PhysicalContactKind::Consumed
    );
    assert_eq!(result.execution.physical.target_entity, Some(food));
    assert!(result.body_event.nutrition > 0.0);
    assert_eq!(result.observation.reward_valence.raw(), 0.0);
    assert!(control.entity(food).unwrap().is_consumed());
    assert_eq!(control.entity(food).unwrap().carried_by, None);
    let physiology =
        MeasuredPhysiologyTransition::new(receipt.biology_before, receipt.biology_after).unwrap();
    let outcome = PostActionOutcome::new(
        ORGANISM,
        SEQUENCE,
        outcome_tick,
        result.execution.succeeded,
        result.execution.physical,
        physiology.homeostatic_delta,
        SignedValence::ZERO,
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(physiology.aversive_harm()).unwrap(),
        physiology.energy_delta,
        NormalizedScalar::new(0.0).unwrap(),
    )
    .unwrap()
    .with_measured_physiology(physiology)
    .unwrap();
    assert_eq!(outcome.reward_valence.raw(), 0.0);
    if harmful {
        assert!(
            physiology.aversive_harm() > 0.0,
            "fixture must measure real nonfatal harm"
        );
        assert!(
            outcome.experienced_valence().raw() < 0.0,
            "fixture must measure negative value"
        );
    } else {
        assert!(
            outcome.experienced_valence().raw() > 0.0,
            "fixture must measure positive value"
        );
    }
    let patch = ExperiencePatchBuilder::new(SEQUENCE)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap();
    memory.observe_sealed_patch(&patch).unwrap();
    // Consumption belongs to the action's current world tick, not biology's outcome tick.
    let resource = control
        .ecology()
        .resources
        .iter()
        .find(|resource| resource.object_id == food)
        .unwrap()
        .clone();
    let consumed_at = resource.consumed_at_tick.unwrap();
    assert_eq!(consumed_at, control.tick());
    assert_eq!(outcome_tick.raw(), consumed_at.raw() + 1);
    let deadline = Tick::new(consumed_at.raw() + u64::from(resource.regrow_after_ticks));
    assert_eq!(control.try_advance_tick().unwrap(), outcome_tick);
    assert_alive(&control);
    assert!(control.tick().raw() < deadline.raw() - 1);
    assert!(control.entity(food).unwrap().is_consumed());
    assert!(control
        .physical_observation_snapshot(ORGANISM, control.tick())
        .unwrap()
        .visible
        .iter()
        .all(|object| object.transport_entity != food));

    let saved_tracker = control.tracked_objects().save_state(ORGANISM).unwrap();
    assert!(
        saved_tracker.records.len() >= 2,
        "decoy allocation must precede food"
    );
    let active = memory.export_active_bank().unwrap();
    assert_eq!(
        active.records.len(),
        1,
        "one source keeps attribution unambiguous"
    );
    let source_record = active
        .records
        .iter()
        .find(|record| {
            record.sealed_sequence_id_raw == SEQUENCE.raw()
                && record.tracked_object_id_raw == original_tracked.raw()
        })
        .unwrap();
    let source = MemoryId(source_record.memory_id_raw);
    let source_target_latent = source_record
        .target_latent_bits
        .iter()
        .map(|bits| f32::from_bits(*bits))
        .collect::<Vec<_>>();
    assert!(source_target_latent.iter().any(|value| *value != 0.0));
    assert!(
        memory
            .bank()
            .object_familiarity(ORGANISM, original_tracked, profile())
            > 0.0
    );
    // Exact checkpoint equality below is justified only for this deliberately Idle case.
    assert!(matches!(
        memory.compaction_checkpoint().phase,
        MemoryCompactionPhase::Idle
    ));
    let staged = memory.export_staged_bank().unwrap();
    assert!(staged.is_none());
    let active_bytes = serde_json::to_vec(&active).unwrap();
    let staged_bytes = staged
        .as_ref()
        .map(|asset| serde_json::to_vec(asset).unwrap());
    let active_ref = GpuBrainAssetRef {
        asset_id: "test-owned-memory-active".to_string(),
        digest: PortableAssetDigest::for_bytes(&active_bytes),
    };
    let staged_ref = staged_bytes.as_ref().map(|bytes| GpuBrainAssetRef {
        asset_id: "test-owned-memory-staged".to_string(),
        digest: PortableAssetDigest::for_bytes(bytes),
    });
    let saved_memory =
        MemorySidecarSaveState::from_sidecar(&memory, active_ref, staged_ref, None).unwrap();
    assert!(saved_memory.retained_learning.is_none());
    let memory_bytes = serde_json::to_vec(&saved_memory).unwrap();
    let tracker_bytes = serde_json::to_vec(&saved_tracker).unwrap();
    let save = PortableSaveFile::from_headless_world(
        "regrowth-memory-components",
        &control,
        RuntimeConfig::deterministic_default(SEED, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![creature_summary(&control, &active)],
    )
    .unwrap();
    save.validate_with_asset_root(std::env::temp_dir()).unwrap();
    // The real registered genome/biology exceeds the inline-string convenience limit.
    // Use the supported complete file-save API without weakening the production cap.
    let world_path = std::env::temp_dir().join(format!(
        "regrowth-memory-components-{}-{}-{}.json",
        std::process::id(),
        harmful,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    save.to_json_file(&world_path).unwrap();

    // Separate test-owned bytes exercise real component DTOs. They are not runtime content-store I/O.
    let loaded = PortableSaveFile::from_json_file(&world_path).unwrap();
    loaded
        .validate_with_asset_root(std::env::temp_dir())
        .unwrap();
    let decoded_memory: MemorySidecarSaveState = serde_json::from_slice(&memory_bytes).unwrap();
    let decoded_tracker: TrackedObjectRegistrySaveState =
        serde_json::from_slice(&tracker_bytes).unwrap();
    let decoded_active: PortableMemoryBankAssetV2 = serde_json::from_slice(&active_bytes).unwrap();
    let decoded_staged: Option<PortableMemoryBankAssetV2> = staged_bytes
        .as_ref()
        .map(|bytes| serde_json::from_slice(bytes).unwrap());
    decoded_memory.validate_for(ORGANISM, profile()).unwrap();
    assert_eq!(decoded_memory, saved_memory);
    assert_eq!(decoded_tracker, saved_tracker);
    assert_eq!(decoded_active, active);
    assert_eq!(
        decoded_memory.compaction.active_bank_asset.digest,
        PortableAssetDigest::for_bytes(&active_bytes)
    );
    let restored_memory = MemorySidecarState::restore_portable(
        profile(),
        decoded_memory.compaction.checkpoint,
        decoded_active,
        decoded_staged,
    )
    .unwrap();
    let mut restored = loaded.restore_headless_world().unwrap();
    assert!(restored
        .tracked_objects()
        .save_state(ORGANISM)
        .unwrap()
        .records
        .is_empty());
    restored
        .restore_tracked_object_states([decoded_tracker])
        .unwrap();
    // Assert the complete tracker, including next_id, BEFORE any fresh sensing can hide a reset.
    assert_eq!(
        restored.tracked_objects().save_state(ORGANISM).unwrap(),
        saved_tracker
    );
    assert_eq!(restored_memory.export_active_bank().unwrap(), active);
    assert_eq!(
        restored_memory.compaction_checkpoint(),
        memory.compaction_checkpoint()
    );
    assert_eq!(restored.tick(), control.tick());
    assert_eq!(restored.ecology(), control.ecology());
    assert_eq!(
        restored.organism_registry().get(ORGANISM),
        control.organism_registry().get(ORGANISM)
    );
    assert_eq!(
        restored.canonical_signature_digest().unwrap(),
        control.canonical_signature_digest().unwrap()
    );

    let persisted_resource = restored
        .ecology()
        .resources
        .iter()
        .find(|resource| resource.object_id == food)
        .unwrap();
    let persisted_deadline = Tick::new(
        persisted_resource.consumed_at_tick.unwrap().raw()
            + u64::from(persisted_resource.regrow_after_ticks),
    );
    assert_eq!(persisted_deadline, deadline);
    while control.tick().raw() < persisted_deadline.raw() - 1 {
        control.try_advance_tick().unwrap();
        restored.try_advance_tick().unwrap();
    }
    assert_eq!(control.tick().raw(), persisted_deadline.raw() - 1);
    assert_eq!(restored.tick(), control.tick());
    assert!(control.entity(food).unwrap().is_consumed());
    assert!(restored.entity(food).unwrap().is_consumed());
    assert_eq!(control.try_advance_tick().unwrap(), persisted_deadline);
    assert_eq!(restored.try_advance_tick().unwrap(), persisted_deadline);
    assert!(!control.entity(food).unwrap().is_consumed());
    assert!(!restored.entity(food).unwrap().is_consumed());
    assert_alive(&control);
    assert_alive(&restored);
    assert_eq!(
        restored.canonical_signature_digest().unwrap(),
        control.canonical_signature_digest().unwrap()
    );

    let control_draft = sense(&mut control);
    let restored_draft = sense(&mut restored);
    let return_candidate = ingest_candidate(&restored_draft, food);
    assert_eq!(
        candidate_tracked_id(&restored_draft, return_candidate),
        original_tracked
    );
    let returned = restored.entity(food).unwrap().clone();
    assert_eq!(returned.id, food);
    assert_eq!(returned.tracking_provenance, original_provenance);
    assert_eq!(returned.tracking_key, original_key);
    assert_eq!(tracked_id(&restored, food), original_tracked);
    let individual_recall =
        isolated_ingest_recall(&restored_memory, &restored_draft, return_candidate, 0);
    let individual_receipt = &individual_recall.receipt().candidates[0];
    assert_eq!(individual_receipt.target_searched, 1);
    assert_eq!(individual_receipt.family_searched, 1);
    assert_eq!(individual_receipt.target_matches, 1);
    assert_eq!(individual_receipt.family_matches, 1);
    let (_, control_recall) = memory
        .recall_frame(&control_draft)
        .unwrap()
        .finalize(control_draft)
        .unwrap();
    let (_, return_recall) = restored_memory
        .recall_frame(&restored_draft)
        .unwrap()
        .finalize(restored_draft)
        .unwrap();
    assert_eq!(return_recall.context(), control_recall.context());
    assert_eq!(return_recall.receipt(), control_recall.receipt());
    assert_eq!(
        individual_recall.candidate_keys()[0].query().features(),
        return_recall.candidate_keys()[usize::from(return_candidate.candidate_index)]
            .query()
            .features()
    );
    let mut individual_context = individual_recall.context().candidates[0];
    individual_context.candidate_index = return_candidate.candidate_index;
    assert_eq!(
        individual_context,
        recall_context(&return_recall, return_candidate)
    );
    assert_recalled_meal(
        recall_context(&return_recall, return_candidate),
        source,
        harmful,
        &source_target_latent,
    );
    assert!(
        restored_memory
            .bank()
            .object_familiarity(ORGANISM, original_tracked, profile())
            > 0.0
    );
    assert_eq!(
        restored.canonical_signature_digest().unwrap(),
        control.canonical_signature_digest().unwrap()
    );

    // Independent branches keep matched, same-bin negative and gross-mismatch probes separate.
    for (cue_matched, same_category_bin) in [(true, true), (false, true), (false, false)] {
        let mut probe = restored.clone();
        probe
            .editor_move_object(food, Vec3f::new(40.0, 0.0, 0.0))
            .unwrap();
        let fresh = spawn_food(
            &mut probe,
            "fresh-food",
            harmful,
            returned.position,
            returned.radius,
        );
        let mut cues = returned.grounded_physical;
        if !cue_matched && same_category_bin {
            // Both 0.1 and -0.12 quantize to bin 0; the 0.22 gap fails per-record equivalence.
            cues.chemical[1] = -0.12;
        } else if !cue_matched {
            cues.color = [0.0, 0.0, 1.0];
            cues.material = [0.0, 0.0, 1.0];
            cues.shape = [0.0, 1.0, 0.0];
            cues.chemical = [-0.8, -0.1, -0.2];
            cues.surface_temperature = -0.7;
        }
        probe.set_grounded_physical_properties(fresh, cues).unwrap();
        let object = probe.entity(fresh).unwrap();
        assert_ne!(object.id, returned.id);
        assert_ne!(
            object.tracking_provenance.spawn_sequence,
            returned.tracking_provenance.spawn_sequence
        );
        assert_ne!(object.tracking_key, returned.tracking_key);
        assert_eq!(
            object.tracking_key,
            object.tracking_provenance.canonical_key()
        );
        assert_eq!(object.position, returned.position);
        let fresh_draft = sense(&mut probe);
        let fresh_candidate = ingest_candidate(&fresh_draft, fresh);
        let fresh_tracked = candidate_tracked_id(&fresh_draft, fresh_candidate);
        assert_eq!(fresh_tracked, tracked_id(&probe, fresh));
        assert_ne!(fresh_tracked, original_tracked);
        assert_eq!(
            restored_memory
                .bank()
                .object_familiarity(ORGANISM, fresh_tracked, profile()),
            0.0
        );
        let category_recall =
            isolated_ingest_recall(&restored_memory, &fresh_draft, fresh_candidate, 2);
        let (_, fresh_recall) = restored_memory
            .recall_frame(&fresh_draft)
            .unwrap()
            .finalize(fresh_draft)
            .unwrap();
        let fresh_query =
            fresh_recall.candidate_keys()[usize::from(fresh_candidate.candidate_index)].query();
        assert_eq!(fresh_query.tracked_object_id(), Some(fresh_tracked));
        let context = recall_context(&fresh_recall, fresh_candidate);
        assert_eq!(
            category_recall.candidate_keys()[0].query().features(),
            fresh_query.features()
        );
        let mut category_context = category_recall.context().candidates[0];
        category_context.candidate_index = fresh_candidate.candidate_index;
        assert_eq!(category_context, context);
        let category_receipt = &category_recall.receipt().candidates[0];
        if cue_matched {
            // These are the actual public query lanes used by the private category-cue predicate.
            for lane in (6..18).chain(std::iter::once(21)) {
                let index = MEMORY_TARGET_RANGE.start + lane;
                assert_eq!(fresh_query.features()[index], episode_query_features[index]);
            }
            assert_eq!(category_receipt.target_searched, 1);
            assert_eq!(category_receipt.family_searched, 1);
            assert_eq!(category_receipt.target_matches, 1);
            assert_eq!(category_receipt.family_matches, 1);
            assert_recalled_meal(context, source, harmful, &source_target_latent);
        } else {
            if same_category_bin {
                assert_same_bin_guard_preconditions(
                    fresh_query.features(),
                    &episode_query_features,
                );
                assert_eq!(category_receipt.target_searched, 1);
                assert_eq!(category_receipt.family_searched, 1);
            } else {
                let color_lane = MEMORY_TARGET_RANGE.start + 6;
                assert!(
                    (fresh_query.features()[color_lane] - episode_query_features[color_lane]).abs()
                        > 0.2
                );
            }
            assert_eq!(category_receipt.target_matches, 0);
            assert_eq!(category_receipt.family_matches, 0);
            assert_eq!(context.target_source_count, 0);
            assert_eq!(context.family_source_count, 0);
            assert_eq!(context.best_target_source, None);
            assert_eq!(context.best_family_source, None);
        }
        assert_eq!(
            restored_memory
                .bank()
                .object_familiarity(ORGANISM, fresh_tracked, profile()),
            0.0,
            "category recall must not create personal familiarity"
        );
        assert!(
            restored_memory
                .bank()
                .object_familiarity(ORGANISM, original_tracked, profile())
                > 0.0
        );
        assert_eq!(
            restored_memory.export_active_bank().unwrap(),
            active,
            "read-only recall must not add episodes"
        );
    }
}
