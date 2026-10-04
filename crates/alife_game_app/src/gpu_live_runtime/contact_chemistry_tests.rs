//! CPU tests of production sealing and personal recall; no neural execution.
use super::*;
use alife_core::*;
use alife_world::{
    create_canonical_new_game, CanonicalNewGameConfig, HeadlessActionIds, WorldEditorSpawnSpec,
    WorldObjectKind,
};

const OWN: OrganismId = OrganismId(1);

fn founders() -> HeadlessWorld {
    let asset =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let mut world =
        create_canonical_new_game(&CanonicalNewGameConfig::phase3(92_108, 3).unwrap(), &asset)
            .unwrap()
            .world;
    for (id, position) in [
        (1, Vec3f::ZERO),
        (2, Vec3f::new(2.0, 0.0, 0.0)),
        (3, Vec3f::new(3.0, 0.0, 0.0)),
    ] {
        let entity = world
            .organism_registry()
            .get(OrganismId(id))
            .unwrap()
            .world_entity_id();
        assert_eq!(world.entity(entity).unwrap().social_affinity, 0.0);
        world.editor_move_object(entity, position).unwrap();
    }
    // Let the actual newborn drives accumulate and development stabilize. No
    // drive, chemical, value weight, or remembered valence is assigned here.
    for _ in 0..600 {
        world.try_advance_tick().unwrap();
    }
    world
}

fn empty_memory() -> MemoryBank {
    MemoryBank::new(
        MemoryBankConfig::new(
            LIVE_MEMORY_CAPACITY,
            LIVE_MEMORY_MAX_FEATURE_LEN,
            LIVE_MEMORY_MAX_MATCH_COUNT,
            LIVE_MEMORY_MIN_MATCH_SCORE,
            Confidence::new(0.0).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn recall(
    world: &mut HeadlessWorld,
    bank: &MemoryBank,
    target: WorldEntityId,
) -> (PerceptionFrame, FinalizedMemoryRecall) {
    recall_at(world, bank, target, ExperienceSequenceId(1))
}

fn recall_at(
    world: &mut HeadlessWorld,
    bank: &MemoryBank,
    target: WorldEntityId,
    sequence: ExperienceSequenceId,
) -> (PerceptionFrame, FinalizedMemoryRecall) {
    let tick = world.tick();
    let homeostasis = world
        .organism_registry()
        .get(OWN)
        .unwrap()
        .biochemistry()
        .homeostasis;
    let draft = world
        .perception_frame_draft(OWN, tick, SensorProfile::GroundedObjectSlotsV1, homeostasis)
        .unwrap();
    let candidate = draft
        .candidates()
        .iter()
        .find(|c| c.target.entity == Some(target))
        .unwrap();
    let CandidateObservationRef::ObjectSlot(slot) = candidate.observation else {
        panic!("physical target slot")
    };
    let tracked = draft.grounded_object_slots()[usize::from(slot)].tracked_object_id;
    let mut attention = AttentionFrame::empty(OWN, sequence, tick).unwrap();
    attention.focal_targets = vec![StableFocusIdentity::TrackedObject(tracked)];
    attention.budget_receipt.requested_focal_count = 1;
    attention.budget_receipt.granted_focal_count = 1;
    let mut context = CognitiveContextFrame::empty(OWN, sequence, tick).unwrap();
    context.focal.identities = attention.focal_targets.clone();
    context.budget.focal_capacity = attention.budget_receipt.focal_capacity;
    context.attention = attention;
    bank.recall_frame(&draft)
        .unwrap()
        .with_cognitive_context(context)
        .unwrap()
        .finalize(draft)
        .unwrap()
}

fn seal_selection(
    world: &mut HeadlessWorld,
    target: WorldEntityId,
    bank: &MemoryBank,
    action_id: ActionId,
    sequence: ExperienceSequenceId,
) -> ExperiencePatch {
    let tick = world.tick();
    let entity = world
        .organism_registry()
        .get(OWN)
        .unwrap()
        .world_entity_id();
    let (phenotype, mut resident) = GpuLiveBrainRuntime::compile_birth(
        world,
        BrainScaleTier::Nano512,
        SensorProfile::GroundedObjectSlotsV1,
        OWN,
    )
    .unwrap();
    let (frame, recalled) = recall_at(world, bank, target, sequence);
    let chosen = frame
        .candidates()
        .iter()
        .find(|c| {
            c.action_id == action_id
                && (c.kind == ActionKind::Idle || c.target.entity == Some(target))
        })
        .unwrap();
    let confidence = Confidence::new(1.0).unwrap();
    resident.next_sequence = sequence.raw();
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence,
        phenotype.brain_class_id(),
        phenotype.phenotype_hash(),
        resident.genome.id,
        resident.genome.schema_version,
        resident.development.clone(),
        frame.clone(),
    )
    .unwrap();
    // A validated selection receipt enters the real sealer. This does not claim
    // that a GPU chose Approach or that learned neural behavior was exercised.
    let command = chosen.to_command(OWN, confidence).unwrap();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence,
        phenotype.phenotype_hash(),
        1,
        0,
        &frame,
        NeuralActionSelection {
            candidate_index: chosen.candidate_index,
            logit: 0.0,
            confidence,
            active_tiles: 1,
            active_synapses: 1,
        },
        command,
    )
    .unwrap()
    .with_finalized_memory_recall(&frame, &recalled, chosen.candidate_index)
    .unwrap();
    let bundle = MotorCommandBundle::new(
        OWN,
        sequence,
        tick,
        vec![channel_command_for_action(
            if command.kind == ActionKind::Idle {
                MotorChannel::Posture
            } else {
                MotorChannel::Locomotion
            },
            &command,
        )
        .unwrap()],
    )
    .unwrap();
    let work = BrainWorkReceipt {
        schema_version: 1,
        class_id_raw: 0,
        organism_id_raw: OWN.raw(),
        tick: tick.raw(),
        handle_slot: 0,
        handle_generation: 0,
        dispatch_generation: 1,
        frame_digest: frame.frame_digest().0,
        sequence_cursor: sequence.raw(),
        counters: Default::default(),
        route_schedule_digest: [0; 4],
        neural_cost_q24: 0,
        atp_before_q16: 0,
        atp_debit_q16: 0,
        atp_after_q16: 0,
        receipt_digest: [0; 4],
    };
    let mut residents = BTreeMap::from([(OWN.raw(), resident)]);
    let context = recalled.cognitive_context().unwrap().clone();
    let sealed = seal_prepared_selection_core(
        world,
        &mut residents,
        0,
        CognitiveWorkCostPolicy::disabled(),
        false,
        WorldMutationRollback::Local,
        PreparedSealInput {
            organism_id: OWN,
            world_entity_id: entity,
            frame,
            memory: recalled.receipt().clone(),
            social_recognition: recalled.recognized_social_emission(),
            sequence_id: sequence,
            outcome_tick: Tick(tick.raw() + 1),
            cognitive_context: context,
            work,
            v11_work: GpuV11WorkReceipt::default(),
            pre_action,
            decision,
            motor_bundle: bundle,
            frozen_prediction: None,
            speech_payload: None,
            speech_prompted: false,
        },
    )
    .unwrap();
    assert_eq!(
        sealed
            .patch
            .outcome()
            .measured_physiology
            .unwrap()
            .after
            .homeostasis,
        world
            .organism_registry()
            .get(OWN)
            .unwrap()
            .biochemistry()
            .homeostasis
    );
    sealed.patch
}

#[test]
fn contact_chemistry_bootstraps_signed_personal_memory_from_real_founder_experience() {
    let base = founders();
    let peer = base
        .organism_registry()
        .get(OrganismId(2))
        .unwrap()
        .world_entity_id();
    let stranger = base
        .organism_registry()
        .get(OrganismId(3))
        .unwrap()
        .world_entity_id();
    let mut touching = base.clone();
    touching
        .editor_move_object(peer, Vec3f::new(0.4, 0.0, 0.0))
        .unwrap();
    let mut memory = empty_memory();
    assert!(recall(&mut touching, &memory, peer)
        .1
        .recognized_social_emission()
        .is_none());
    let mut separated = base.clone();
    let control = seal_selection(
        &mut separated,
        peer,
        &memory,
        HeadlessActionIds::APPROACH,
        ExperienceSequenceId(1),
    );
    let positive = seal_selection(
        &mut touching,
        peer,
        &memory,
        HeadlessActionIds::APPROACH,
        ExperienceSequenceId(1),
    );
    let physiology = positive.outcome().measured_physiology.unwrap();
    assert_eq!(physiology.before.value_profile().hormones[..9], [0.0; 9]);
    assert!(physiology.homeostatic_delta.drives.loneliness < 0.0);
    assert!(physiology.homeostatic_improvement() > 0.0, "{physiology:?}");
    assert!(
        positive.outcome().experienced_valence().raw() > 0.0,
        "{:?}",
        positive.outcome()
    );
    assert!(
        control
            .outcome()
            .measured_physiology
            .unwrap()
            .homeostatic_improvement()
            < physiology.homeostatic_improvement()
    );
    assert_eq!(
        positive.decision().selected_action.target_entity,
        Some(peer)
    );
    assert_eq!(positive.outcome().reward_valence, SignedValence::ZERO);
    memory.observe_sealed_patch(&positive).unwrap();
    touching.try_advance_tick().unwrap();
    let (frame, familiar) = recall(&mut touching, &memory, peer);
    assert!(
        familiar.recognized_social_emission().is_some(),
        "sensory={:?}; recalled consequences={:?}",
        frame.sensory().social_context,
        familiar.context()
    );
    assert!(frame
        .candidates()
        .iter()
        .any(|c| c.kind == ActionKind::Idle));
    assert!(recall(&mut touching, &memory, stranger)
        .1
        .recognized_social_emission()
        .is_none());
    let bytes = serde_json::to_vec(&memory).unwrap();
    let loaded: MemoryBank = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        recall(&mut touching, &loaded, peer)
            .1
            .recognized_social_emission(),
        familiar.recognized_social_emission()
    );

    // Reunion at sight range can release learned recognition while Idle wins,
    // with physical contact removed. Compare the same body and action with an
    // empty personal bank; the hormone difference cannot come from peer touch.
    let own = touching
        .organism_registry()
        .get(OWN)
        .unwrap()
        .world_entity_id();
    let position = touching.entity(own).unwrap().position;
    touching
        .editor_move_object(peer, Vec3f::new(position.x + 1.0, position.y, position.z))
        .unwrap();
    let mut unknown = touching.clone();
    let idle = ActionKind::Idle.canonical_id();
    let familiar_idle = seal_selection(&mut touching, peer, &loaded, idle, ExperienceSequenceId(2));
    let unfamiliar_idle = seal_selection(
        &mut unknown,
        peer,
        &empty_memory(),
        idle,
        ExperienceSequenceId(2),
    );
    assert_eq!(
        familiar_idle.decision().selected_action.kind,
        ActionKind::Idle
    );
    assert!(
        familiar_idle
            .outcome()
            .measured_physiology
            .unwrap()
            .after
            .homeostasis
            .hormones
            .oxytocin
            > unfamiliar_idle
                .outcome()
                .measured_physiology
                .unwrap()
                .after
                .homeostasis
                .hormones
                .oxytocin
    );

    // Matched initial biology, same peer, actual simultaneous injury. No negative
    // static affinity or synthetic negative memory is needed to make it aversive.
    let mut hurt = base;
    hurt.editor_move_object(peer, Vec3f::new(0.4, 0.0, 0.0))
        .unwrap();
    hurt.editor_spawn_object(WorldEditorSpawnSpec {
        label: "injury".into(),
        kind: WorldObjectKind::Hazard,
        organism_id: None,
        position: Vec3f::ZERO,
        nutrition: 0.0,
        hazard_pain: 0.4,
        radius: 0.75,
        token_id: None,
    })
    .unwrap();
    let mut aversive = empty_memory();
    let negative = seal_selection(
        &mut hurt,
        peer,
        &aversive,
        HeadlessActionIds::APPROACH,
        ExperienceSequenceId(3),
    );
    let tracked_peer = positive
        .decision()
        .episodic_key()
        .unwrap()
        .query()
        .tracked_object_id()
        .unwrap();
    assert_eq!(
        negative
            .decision()
            .episodic_key()
            .unwrap()
            .query()
            .tracked_object_id(),
        Some(tracked_peer),
        "mixed signed history must refer to the same personally tracked peer"
    );
    assert!(
        negative
            .outcome()
            .measured_physiology
            .unwrap()
            .homeostatic_improvement()
            < 0.0
    );
    aversive.observe_sealed_patch(&negative).unwrap();
    hurt.try_advance_tick().unwrap();
    assert!(recall(&mut hurt, &aversive, peer)
        .1
        .recognized_social_emission()
        .is_none());
    memory.observe_sealed_patch(&negative).unwrap();
    assert!(
        recall(&mut hurt, &memory, peer)
            .1
            .recognized_social_emission()
            .is_none(),
        "signed injury must outweigh small relief"
    );
}
