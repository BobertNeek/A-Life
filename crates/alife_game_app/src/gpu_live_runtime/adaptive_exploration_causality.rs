//! CPU-only causal input-chain regression (AOA-CON-003/004/005, AOA-EVAL-006).
//! No adapter, neural execution, or hand-injected concept/gap state.
use super::*;
use alife_core::{
    ActionKind, CandidateActionFamily, ExperiencePatchBuilder, GapResolutionStatus,
    HomeostaticDelta, PhysicalActionOutcome, PhysicalContactKind, TrackedObjectId,
};
use alife_world::HeadlessScenarioBuilder;

// The backend tests bind this bounded mathematical rule to the actual WGSL.
// This reuse proves the upload-to-rule link; it is not GPU behavioral parity.
#[allow(dead_code)]
#[path = "../../../alife_gpu_backend/tests/support/exploration_reference.rs"]
mod reference;

const OWN: OrganismId = OrganismId(1);
const PROFILE: SensorProfile = SensorProfile::GroundedObjectSlotsV1;

fn draft_at(world: &mut HeadlessWorld, tick: Tick) -> PerceptionFrameDraft {
    world
        .perception_frame_draft(OWN, tick, PROFILE, HomeostaticSnapshot::baseline(tick))
        .unwrap()
}

fn sealed_contact(
    draft: PerceptionFrameDraft,
    memory: &MemorySidecarState,
    resident: &ResidentCognition,
    sequence: ExperienceSequenceId,
    object: TrackedObjectId,
    success: bool,
    require_prior_success: bool,
) -> ExperiencePatch {
    let (frame, recall) = memory
        .recall_frame(&draft)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let chosen = frame
        .candidates()
        .iter()
        .find(|candidate| {
            candidate.kind == ActionKind::Interact
                && candidate.family == CandidateActionFamily::Contact
                && matches!(candidate.observation, CandidateObservationRef::ObjectSlot(slot)
                    if frame.grounded_object_slots()[usize::from(slot)].tracked_object_id == object)
        })
        .unwrap();
    if !success {
        let remembered = &recall.context().candidates[usize::from(chosen.candidate_index)];
        assert!(remembered.family_source_count > 0);
        // The first mismatch contradicts successful recall. After observing
        // it, bounded recall may prefer the newer failed contact; the second
        // mismatch still reinforces the same observed prediction-error gap.
        if require_prior_success {
            assert!(
                remembered.family_value[1] > 0.0,
                "prior successful contact evidence"
            );
        }
    }
    let confidence = chosen.sensor_confidence;
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence,
        resident.phenotype.brain_class_id(),
        resident.phenotype.phenotype_hash(),
        resident.genome.id,
        resident.genome.schema_version,
        resident.development.clone(),
        frame.clone(),
    )
    .unwrap();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence,
        resident.phenotype.phenotype_hash(),
        sequence.raw(),
        0,
        &frame,
        NeuralActionSelection {
            candidate_index: chosen.candidate_index,
            logit: 0.0,
            confidence,
            active_tiles: 1,
            active_synapses: 1,
        },
        chosen.to_command(OWN, confidence).unwrap(),
    )
    .unwrap()
    .with_finalized_memory_recall(&frame, &recall, chosen.candidate_index)
    .unwrap();
    // Exercise the real supported predictor/error producer at a controlled
    // CPU boundary. Its thirteen lanes are position, velocity and drives,
    // never a fabricated contact-success lane. The zero/high successor values
    // are deliberate fixture inputs, not a claimed physical world transition.
    let source = grounded_semantic_state_from_frame(&frame).unwrap();
    assert_eq!(source.len(), 13);
    let channel =
        channel_command_for_action(MotorChannel::Manipulation, &decision.selected_action).unwrap();
    let condition =
        alife_core::JointMotorCondition::new(vec![alife_core::MotorChannelFactor::from_command(
            &channel,
        )
        .unwrap()])
        .unwrap();
    let target = |values| {
        PredictionTargetReceipt::for_successor(
            OWN,
            sequence,
            chosen.action_id,
            frame.tick(),
            frame.frame_digest().0,
            source.clone(),
            condition.clone(),
            SemanticStateVector::new(values).unwrap(),
        )
        .unwrap()
    };
    let matched = target(vec![0.0; source.len()]);
    let mut predictor = GroundedSuccessorPredictor::default();
    assert_eq!(
        predictor.observe(&matched).unwrap().mean_absolute_error,
        0.0
    );
    assert!(predictor.has_acquired_state());
    let mut frozen = predictor.predict(&source, &condition).unwrap();
    frozen.source_digest = frame.frame_digest().0;
    let measured = target(vec![if success { 0.0 } else { 0.75 }; source.len()]);
    let update = predictor.observe_frozen(&measured, &frozen).unwrap();
    let mut context = CognitiveContextFrame::empty(OWN, sequence, frame.tick()).unwrap();
    let prediction_error = apply_prediction_evidence(
        &mut context,
        &measured,
        &update.error,
        update.prediction.category_coverage,
    )
    .unwrap();
    assert_eq!(prediction_error, update.mean_absolute_error);
    assert_eq!(prediction_error, if success { 0.0 } else { 0.75 });
    // The measured contact is still supplied at the sealed-patch fixture
    // boundary. No GPU decision or live-world execution is claimed here.
    let outcome = PostActionOutcome::new(
        OWN,
        sequence,
        Tick(frame.tick().raw() + 1),
        success,
        PhysicalActionOutcome {
            contact: if success {
                PhysicalContactKind::Touch
            } else {
                PhysicalContactKind::Blocked
            },
            target_entity: chosen.target.entity,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar(0.1),
        },
        HomeostaticDelta::zero(),
        SignedValence(if success { 0.25 } else { -0.6 }),
        NormalizedScalar(if success { 0.05 } else { 0.6 }),
        NormalizedScalar(0.0),
        SignedValence(-0.1),
        NormalizedScalar(prediction_error),
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

fn observe(
    memory: &mut MemorySidecarState,
    topology: &mut TopologySidecar,
    patch: &ExperiencePatch,
) -> alife_core::TopologyUpdate {
    patch.validate_contract().unwrap();
    memory.observe_sealed_patch(patch).unwrap();
    let receipt = topology.observe_sealed_patch(patch);
    assert!(!receipt.rejected_invalid);
    assert!(!receipt.replay_rejected);
    receipt.update.unwrap()
}

#[test]
fn repeated_sealed_mismatch_keeps_familiar_object_gap_evidence_target_local() {
    let mut world = HeadlessScenarioBuilder::new(77_113)
        .agent("agent", OWN, Vec3f::ZERO)
        .food("food-a", Vec3f::new(1.0, 0.0, 0.0), 0.8)
        .food("food-b", Vec3f::new(-1.0, 0.0, 0.0), 0.8)
        .build()
        .unwrap();
    super::tests::register_sealing_test_organism(&mut world, OWN);
    let (_, resident) =
        GpuLiveBrainRuntime::compile_birth(&world, BrainScaleTier::Nano512, PROFILE, OWN).unwrap();
    let initial = draft_at(&mut world, Tick::ZERO);
    assert_eq!(initial.grounded_object_slots().len(), 2);
    let object_a = initial.grounded_object_slots()[0].tracked_object_id;
    let object_b = initial.grounded_object_slots()[1].tracked_object_id;
    assert_ne!(object_a, object_b);
    let mut memory = GpuLiveBrainRuntime::new_memory_sidecar(OWN, PROFILE).unwrap();
    let cold_memory = memory.clone();
    let mut topology = TopologySidecar::new_profiled(
        OWN,
        initial.profile_provenance().identity(),
        TopologicalMapConfig::default(),
    )
    .unwrap();

    // Familiarity is acquired from sealed, target-keyed contacts, not an
    // assigned confidence, synthetic interaction counter, or patched JSON.
    for raw in 1..=16 {
        let object = if raw % 2 == 1 { object_a } else { object_b };
        let patch = sealed_contact(
            draft_at(&mut world, Tick(raw * 2)),
            &memory,
            &resident,
            ExperienceSequenceId(raw),
            object,
            true,
            false,
        );
        assert!(observe(&mut memory, &mut topology, &patch)
            .gap_ids
            .is_empty());
    }
    let familiar_a = memory
        .bank()
        .object_familiarity(OWN, object_a, memory.profile());
    let familiar_b = memory
        .bank()
        .object_familiarity(OWN, object_b, memory.profile());
    assert!(familiar_a > 0.85 && familiar_b > 0.85);
    let stable_topology = topology.clone();
    assert!(stable_topology.map().unresolved_gaps().is_empty());

    let mut first_gap = None;
    let mut previous_voltage = 0.0;
    for raw in 17..=18 {
        let patch = sealed_contact(
            draft_at(&mut world, Tick(raw * 2)),
            &memory,
            &resident,
            ExperienceSequenceId(raw),
            object_a,
            false,
            raw == 17,
        );
        assert!(!patch.outcome().contradiction_observed);
        let update = observe(&mut memory, &mut topology, &patch);
        assert_eq!(update.gap_ids.len(), 1);
        let gap_id = update.gap_ids[0];
        assert_eq!(*first_gap.get_or_insert(gap_id), gap_id);
        let gap = topology.gap(gap_id).unwrap();
        assert_eq!(gap.status, GapResolutionStatus::Open);
        assert_eq!(
            gap.contradiction_type,
            alife_core::ContradictionType::PredictionError
        );
        assert!(gap.curiosity_voltage.raw() > previous_voltage);
        previous_voltage = gap.curiosity_voltage.raw();
        assert!(gap.source_concepts.iter().any(|id| {
            topology
                .concept(*id)
                .unwrap()
                .bindings
                .objects
                .contains(&object_a)
        }));
        assert!(!gap.source_concepts.iter().any(|id| {
            topology
                .concept(*id)
                .unwrap()
                .bindings
                .objects
                .contains(&object_b)
        }));
    }
    assert_eq!(topology.map().unresolved_gaps().len(), 1);
    assert_eq!(
        memory
            .bank()
            .object_familiarity(OWN, object_b, memory.profile()),
        familiar_b
    );

    // Build the existing 54-lane candidate ABI entirely on the CPU.
    let asset = FoundationWeightAsset::decode_canonical(include_bytes!(
        "../../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
    ))
    .unwrap();
    let manifest = asset.manifest();
    let identity = FoundationGeneticIdentity::new(
        manifest.foundation_id().raw(),
        manifest.foundation_version().raw() as u16,
        manifest.compatibility_family_id().raw(),
        manifest.capacity_class_id(),
    )
    .unwrap();
    let extension = alife_core::CognitiveChannelExtensionV1::try_new_v1(
        identity,
        alife_core::ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )
    .unwrap();
    let candidate =
        alife_core::Nano512ActionCreditCandidateV2::new_with_cognitive_extension(&asset, extension)
            .unwrap();
    let (phenotype, _) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&candidate).unwrap();
    let mut bucket =
        alife_gpu_backend::GpuClassBucketPlan::new(BrainCapacityClass::n512(), 1).unwrap();
    let slot = bucket.insert_phenotype(0, 1, &phenotype).unwrap();
    assert_eq!(slot.decoder_input_stride(), 54);
    let final_draft = draft_at(&mut world, Tick(40));
    let prepare = |memory: &MemorySidecarState, topology: &TopologySidecar| {
        let familiarity = memory
            .bank()
            .object_familiarity(OWN, object_a, memory.profile());
        let draft = final_draft
            .clone()
            .with_remembered_novelty(1.0 - familiarity)
            .unwrap();
        let recall = memory.recall_frame(&draft).unwrap();
        let evidence = topology_evidence_for_draft(&draft, topology).unwrap();
        let projection = cognitive_projection_for_draft(
            &draft,
            &recall,
            ExperienceSequenceId(20),
            &GroundedSuccessorPredictor::default(),
            &evidence,
        )
        .unwrap();
        let context =
            cognitive_context_for_recall(OWN, ExperienceSequenceId(20), &recall, topology).unwrap();
        let context = cognitive_context_with_projection(context, projection.clone()).unwrap();
        let (frame, finalized) = recall
            .with_cognitive_context(context)
            .unwrap()
            .finalize(draft)
            .unwrap();
        finalized.validate_for_frame(&frame).unwrap();
        let perception =
            alife_gpu_backend::GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
        let upload = GpuMemoryContextUpload::try_from_finalized(
            &frame,
            &finalized,
            perception.frame_binding,
            &slot,
        )
        .unwrap();
        (evidence, projection, frame, upload)
    };
    // Keep final memory/world fixed: only the observed topology differs.
    let (stable, stable_projection, _, stable_upload) = prepare(&memory, &stable_topology);
    let (changed, projection, frame, upload) = prepare(&memory, &topology);
    let (_, cold_projection, cold_frame, cold_upload) = prepare(&cold_memory, &topology);
    assert!(frame.sensory().channels.novelty_signal.raw() < 0.1);
    assert_eq!(cold_frame.sensory().channels.novelty_signal.raw(), 1.0);
    assert!(changed[&object_a].1.raw() > 0.0);
    assert_eq!(stable[&object_a].1.raw(), 0.0);
    assert_eq!(changed[&object_b], stable[&object_b]);
    assert_eq!(changed[&object_b].1.raw(), 0.0);
    assert_eq!(upload.cognitive_records.len(), frame.candidates().len());
    let mut a_count = 0;
    let mut b_count = 0;
    for (index, candidate) in projection.candidates.iter().enumerate() {
        let row = &upload.cognitive_records[index];
        assert_eq!(row.schema_version, 1);
        assert_eq!(row.candidate_index, u32::from(candidate.candidate_index));
        assert_eq!(row.values[14], candidate.gap_match.raw());
        // Familiarity lowers novelty but cannot erase real unresolved gaps.
        assert_eq!(
            candidate.gap_match,
            cold_projection.candidates[index].gap_match
        );
        assert_eq!(
            row.values[14],
            cold_upload.cognitive_records[index].values[14]
        );
        if candidate.tracked_object_id == Some(object_a) {
            a_count += 1;
            assert_eq!(candidate.gap_match, changed[&object_a].1);
            assert!(row.values[14] > frame.sensory().channels.novelty_signal.raw());
            assert_eq!(stable_upload.cognitive_records[index].values[14], 0.0);
            assert_eq!(
                row.values[15],
                stable_upload.cognitive_records[index].values[15]
            );
        } else {
            assert_eq!(row, &stable_upload.cognitive_records[index]);
            assert_eq!(candidate, &stable_projection.candidates[index]);
            assert_eq!(row.values[14], 0.0);
            b_count += usize::from(candidate.tracked_object_id == Some(object_b));
        }
    }
    assert!(a_count > 0 && b_count > 0);

    let a_index = projection
        .candidates
        .iter()
        .position(|candidate| candidate.tracked_object_id == Some(object_a))
        .unwrap();
    let b_index = projection
        .candidates
        .iter()
        .position(|candidate| candidate.tracked_object_id == Some(object_b))
        .unwrap();
    let drives = frame.homeostasis().drives;
    let state = reference::State {
        organism_id: OWN.raw(),
        tick: frame.tick().raw(),
        novelty: frame.sensory().channels.novelty_signal.raw(),
        curiosity: drives.curiosity,
        hunger: drives.hunger,
        fatigue: drives.fatigue,
        temperature_stress: drives.temperature_stress,
        fear: drives.fear,
        pain: drives.pain,
    };
    let mut stable_choices = reference::pair();
    stable_choices[0].gap_match = stable_upload.cognitive_records[b_index].values[14];
    stable_choices[1].gap_match = stable_upload.cognitive_records[a_index].values[14];
    // Exceed the complete cold pair's perturbation bound, so the matched
    // no-gap baseline cannot change its learned winner even at extreme draws.
    let cold_pair_bound = 2.0
        * (reference::scale(state, stable_choices[0]) + reference::scale(state, stable_choices[1]));
    stable_choices[1].logit = -(cold_pair_bound + 0.01);
    let mut changed_choices = stable_choices;
    changed_choices[0].gap_match = upload.cognitive_records[b_index].values[14];
    changed_choices[1].gap_match = upload.cognitive_records[a_index].values[14];
    assert_eq!(
        reference::scale(state, stable_choices[0]),
        reference::scale(state, changed_choices[0])
    );
    assert!(
        reference::scale(state, changed_choices[1]) > reference::scale(state, stable_choices[1])
    );
    let warm_pair_bound = 2.0
        * (reference::scale(state, changed_choices[0])
            + reference::scale(state, changed_choices[1]));
    assert!(-changed_choices[1].logit < warm_pair_bound);
    let stable_count = reference::alternate_count(state, &stable_choices, 4_096);
    let changed_count = reference::alternate_count(state, &changed_choices, 4_096);
    assert_eq!(stable_count, 0);
    assert!(changed_count > 512);
    eprintln!(
        "controlled predictor mismatch -> target gap -> upload -> bounded rule: stable={stable_count}, unresolved={changed_count} alternate choices / 4096"
    );
}
