//! Real-hardware tests for sealed-outcome GPU fast plasticity.
//!
//! These tests build ordinary sealed neural experience patches and never run
//! CPU neural or plasticity math.
#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::{
    BrainGenome, Confidence, DecisionSnapshot, DevelopmentState, EndocrineDelta, ExperiencePatch,
    ExperiencePatchBuilder, ExperienceSequenceId, HomeostaticDelta, NeuralActionSelection,
    NormalizedScalar, OutcomeCreditPacket, PhysicalActionOutcome, PhysicalContactKind,
    PostActionOutcome, PreActionSnapshot, SignedValence, Tick, Vec3f,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuOutcomeCreditRecord};

const GPU_LEARNING_TOLERANCE: f32 = 1.0e-5;

#[test]
fn superseded_p26_oja_product_runtime_is_absent() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/plasticity.rs",
        "shaders/p26_plasticity.wgsl",
        "tests/plasticity_oja_parity.rs",
    ] {
        assert!(
            !crate_root.join(relative).exists(),
            "superseded product plasticity surface still exists: {relative}"
        );
    }

    let exports = std::fs::read_to_string(crate_root.join("src/lib.rs")).unwrap();
    for forbidden in [
        "pub mod plasticity;",
        "run_plasticity_gpu_diagnostic",
        "GpuOjaFixedPointConfig",
        "GpuPlasticityPlan",
        "P26_WGSL_PLASTICITY",
    ] {
        assert!(
            !exports.contains(forbidden),
            "superseded product export remains: {forbidden}"
        );
    }
}

fn sealed_outcome(
    handle: alife_gpu_backend::GpuBrainHandle,
    frame: &alife_core::PerceptionFrame,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    sequence_raw: u64,
    reward: f32,
    pain: f32,
) -> ExperiencePatch {
    let sequence_id = ExperienceSequenceId(sequence_raw);
    let genome = BrainGenome::scaffold(13, handle.class_id());
    let development = DevelopmentState::new(
        genome.id,
        frame.tick(),
        NormalizedScalar::new(0.35).unwrap(),
    );
    let selection = NeuralActionSelection {
        candidate_index: tick.selection.candidate_index,
        logit: tick.selection.logit,
        confidence: tick.selection.confidence,
        active_tiles: tick.selection.active_tiles,
        active_synapses: tick.selection.active_synapses,
    };
    let candidate = frame.candidates()[usize::from(selection.candidate_index)];
    let command = candidate
        .to_command(
            handle.organism_id(),
            Confidence::new(selection.confidence.raw()).unwrap(),
        )
        .unwrap();
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence_id,
        handle.class_id(),
        handle.phenotype_hash(),
        genome.id,
        genome.schema_version,
        development,
        frame.clone(),
    )
    .unwrap();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id,
        handle.phenotype_hash(),
        tick.dispatch_generation,
        tick.active_activation_side,
        frame,
        selection,
        command,
    )
    .unwrap();
    let outcome = PostActionOutcome::new(
        handle.organism_id(),
        sequence_id,
        Tick::new(frame.tick().raw() + 1),
        reward >= 0.0 && pain == 0.0,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.0).unwrap(),
        },
        HomeostaticDelta {
            drives: alife_core::DriveDelta::zero(),
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(reward).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(pain).unwrap(),
        SignedValence::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
    )
    .unwrap();
    ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}

fn assert_pending_matches_patch(
    handle: alife_gpu_backend::GpuBrainHandle,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    patch: &ExperiencePatch,
) {
    let packet = OutcomeCreditPacket::from_sealed_patch(patch).unwrap();
    let identity = tick.pending_eligibility.identity();
    assert_eq!(packet.organism_id(), handle.organism_id());
    assert_eq!(packet.phenotype_hash(), handle.phenotype_hash());
    assert_eq!(identity.handle_generation(), handle.generation());
    assert_eq!(identity.phenotype_hash(), packet.phenotype_hash());
    assert_eq!(identity.dispatch_generation(), packet.dispatch_generation());
    assert_eq!(identity.originating_tick(), packet.originating_tick());
    assert_eq!(identity.frame_digest(), packet.frame_digest());
    assert_eq!(
        identity.active_activation_side(),
        packet.active_activation_side()
    );
    assert_eq!(identity.candidate_index(), packet.selected_candidate());
    assert_eq!(identity.action_id(), packet.selected_action());
    assert_eq!(identity.action_family(), packet.selected_family());
    assert_eq!(
        identity.candidate_feature_digest(),
        packet.candidate_feature_digest()
    );
    GpuOutcomeCreditRecord::try_from(&packet).unwrap();
}

#[test]
fn outcome_credit_record_matches_the_frozen_abi() {
    assert_eq!(std::mem::align_of::<GpuOutcomeCreditRecord>(), 16);
    assert_eq!(std::mem::size_of::<GpuOutcomeCreditRecord>(), 160);
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, active_activation_side),
        76
    );
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, candidate_feature_digest),
        80
    );
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, frame_digest),
        96
    );
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, dispatch_generation),
        128
    );
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, modulator_value),
        156
    );
}

#[test]
fn plasticity_wgsl_parses_with_four_passes_and_exactly_seven_heap_bindings() {
    assert!(
        alife_gpu_backend::CLOSED_LOOP_PLASTICITY_WGSL.contains(&format!(
            "const PLASTICITY_DISPATCH_ROW_WORDS:u32 = {}u;",
            alife_gpu_backend::GPU_ACTIVE_DISPATCH_ROW_WORDS
        )),
        "plasticity row stepping must match the complete host dispatch row"
    );
    let module = naga::front::wgsl::parse_str(alife_gpu_backend::CLOSED_LOOP_PLASTICITY_WGSL)
        .expect("plasticity WGSL must parse");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .expect("plasticity WGSL must validate");
    let entries = module
        .entry_points
        .iter()
        .map(|entry| (entry.name.as_str(), entry.stage, entry.workgroup_size))
        .collect::<Vec<_>>();
    for expected in [
        (
            "initialize_fast_plasticity",
            naga::ShaderStage::Compute,
            [1, 1, 1],
        ),
        (
            "apply_fast_plasticity",
            naga::ShaderStage::Compute,
            [64, 1, 1],
        ),
        (
            "capture_fast_plasticity_replay",
            naga::ShaderStage::Compute,
            [64, 1, 1],
        ),
        (
            "finalize_fast_plasticity",
            naga::ShaderStage::Compute,
            [1, 1, 1],
        ),
    ] {
        assert!(
            entries.contains(&expected),
            "missing WGSL pass {expected:?}"
        );
    }
    let bindings = module
        .global_variables
        .iter()
        .filter_map(|(_, global)| global.binding.as_ref())
        .map(|binding| (binding.group, binding.binding))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        bindings,
        (0_u32..7).map(|binding| (0, binding)).collect(),
        "plasticity must reuse the exact seven production heaps"
    );
}

#[test]
fn rewarding_outcome_changes_next_encounter_before_sleep() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let organism = alife_core::OrganismId(4_101);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle = backend.insert_brain(organism, phenotype).unwrap();
    let first_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        900,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let first_tick = backend
        .tick_batch(&[(handle, first_frame.clone())])
        .unwrap()
        .remove(0);
    let patch = sealed_outcome(handle, &first_frame, &first_tick, 1, 0.8, 0.0);
    let receipt = backend.apply_sealed_outcome(handle, &patch).unwrap();

    assert_eq!(receipt.handle, handle);
    assert_eq!(receipt.sequence_id, ExperienceSequenceId(1));
    assert_eq!(receipt.dispatch_generation, first_tick.dispatch_generation);
    assert_eq!(
        receipt.active_activation_side,
        first_tick.active_activation_side
    );
    assert_eq!(
        receipt.output_fast_generation,
        receipt.input_fast_generation + 1
    );
    assert_eq!(
        receipt.output_eligibility_generation,
        first_tick
            .pending_eligibility
            .identity()
            .staging_eligibility_generation()
    );
    assert!(receipt.fast_weights_changed > 0);
    assert!(receipt.max_abs_delta > 0.0);

    let second_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        902,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let second = backend.tick_batch(&[(handle, second_frame)]).unwrap();
    assert_eq!(
        second.len(),
        1,
        "learning must unblock the next waking tick"
    );
}

#[test]
fn eligibility_contract_is_validated_once_per_row_before_synapse_work() {
    let source = alife_gpu_backend::CLOSED_LOOP_ELIGIBILITY_WGSL;
    let module = naga::front::wgsl::parse_str(source).expect("eligibility WGSL must parse");
    let entry = module
        .entry_points
        .iter()
        .find(|entry| entry.name == "prevalidate_eligibility")
        .expect("eligibility requires a once-per-row prevalidation pass");
    assert_eq!(entry.stage, naga::ShaderStage::Compute);
    assert_eq!(entry.workgroup_size, [1, 1, 1]);
    assert_eq!(
        source.matches("learning_contract_is_valid(").count(),
        2,
        "the full contract may appear only in its definition and row prepass"
    );
    assert_eq!(
        source.matches("pending_row_is_zero(").count(),
        2,
        "the 36-word pending scan may run only in the row prepass"
    );
    assert_eq!(
        source.matches("learning_contract_prevalidated(").count(),
        0,
        "parallel synapse work must consume the prevalidated selection sentinel directly"
    );
}

#[test]
fn parallel_learning_loops_trust_the_validated_row_and_immutable_upload() {
    let eligibility = alife_gpu_backend::CLOSED_LOOP_ELIGIBILITY_WGSL;
    for (entry, next) in [
        (
            "fn accumulate_recurrent_eligibility",
            "fn accumulate_decoder_eligibility",
        ),
        (
            "fn accumulate_decoder_eligibility",
            "fn finalize_pending_eligibility",
        ),
    ] {
        let body = eligibility
            .split_once(entry)
            .expect("parallel eligibility entry exists")
            .1
            .split_once(next)
            .expect("parallel eligibility entry is bounded")
            .0;
        for repeated_guard in [
            "immutable_span_within(",
            "receptor_is_valid(",
            "state_span_within(",
        ] {
            assert!(
                !body.contains(repeated_guard),
                "{entry} repeated immutable upload guard {repeated_guard}"
            );
        }
    }

    let plasticity = alife_gpu_backend::CLOSED_LOOP_PLASTICITY_WGSL;
    let apply_body = plasticity
        .split_once("fn apply_fast_plasticity")
        .expect("fast plasticity entry exists")
        .1
        .split_once("fn capture_fast_plasticity_replay")
        .expect("fast plasticity entry is bounded")
        .0;
    for repeated_guard in [
        "immutable_plan_span_within(",
        "receptor_valid_for_plasticity(",
        "state_span_within(",
    ] {
        assert!(
            !apply_body.contains(repeated_guard),
            "fast plasticity repeated immutable upload guard {repeated_guard}"
        );
    }
}

#[test]
fn recurrent_eligibility_obeys_the_validated_activity_route_mask() {
    let source = alife_gpu_backend::CLOSED_LOOP_ELIGIBILITY_WGSL;
    let body = source
        .split_once("fn accumulate_recurrent_eligibility")
        .expect("recurrent eligibility entry exists")
        .1
        .split_once("fn accumulate_decoder_eligibility")
        .expect("recurrent eligibility entry is bounded")
        .0;
    assert!(body.contains(
        "let route_index = immutable_plan_words[brain.route_indices_offset + local_synapse]"
    ));
    assert!(body.contains("if (!route_enabled_at(route_mask_base, route_index))"));
    assert!(body.contains("store_state_f32(staging_bases.recurrent + local_synapse, previous)"));
}

#[test]
fn same_class_learning_batch_spans_fixed_arenas_without_aliasing_slots() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let slot_bytes = alife_gpu_backend::GpuClassBucketPlan::for_phenotype(&phenotype)
        .unwrap()
        .slot_allocation_receipt()
        .unwrap()
        .logical_slot_commit_bytes;
    let profile = support::scaling::bounded_profile(slot_bytes * 5, 512 * 1024 * 1024, 5, 2);
    let mut backend = GpuClosedLoopBackend::new_required(profile).unwrap();
    let handles = (0_u64..5)
        .map(|index| {
            backend
                .insert_brain(alife_core::OrganismId(4_200 + index), phenotype.clone())
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(backend.allocated_class_arena_count_for_test(), 3);

    let frames = handles
        .iter()
        .map(|handle| {
            support::perception_frame_for_profile_at_tick(
                handle.organism_id().raw(),
                1_200,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            )
        })
        .collect::<Vec<_>>();
    let batch = handles
        .iter()
        .copied()
        .zip(frames.iter().cloned())
        .collect::<Vec<_>>();
    let ticks = backend.tick_batch(&batch).unwrap();
    assert_eq!(
        ticks.iter().map(|tick| tick.handle).collect::<Vec<_>>(),
        handles
    );
    assert!(ticks
        .iter()
        .all(|tick| tick.dispatch_generation == ticks[0].dispatch_generation));

    let patches = handles
        .iter()
        .zip(&frames)
        .zip(&ticks)
        .enumerate()
        .map(|(index, ((handle, frame), tick))| {
            sealed_outcome(*handle, frame, tick, 1, 0.5 + index as f32 * 0.05, 0.0)
        })
        .collect::<Vec<_>>();
    let learning_batch = handles
        .iter()
        .copied()
        .zip(patches.iter())
        .collect::<Vec<_>>();
    let receipts = backend.apply_sealed_outcome_batch(&learning_batch).unwrap();
    assert_eq!(receipts.len(), handles.len());
    assert!(receipts
        .iter()
        .all(|receipt| receipt.fast_weights_changed > 0));
    for handle in handles {
        assert!(backend
            .read_active_fast_weights_for_test(handle)
            .unwrap()
            .iter()
            .any(|weight| *weight != 0.0));
    }
}

#[test]
fn modulator_credit_changes_the_next_encounter_relative_to_a_sealed_neutral_outcome() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let organism_a = alife_core::OrganismId(4_111);
    let organism_b = alife_core::OrganismId(4_112);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle_a = backend.insert_brain(organism_a, phenotype.clone()).unwrap();
    let handle_b = backend.insert_brain(organism_b, phenotype).unwrap();
    for exposure in 0_u64..8 {
        let tick_raw = 1_000 + exposure * 2;
        let frame_a = support::perception_frame_for_profile_at_tick(
            organism_a.raw(),
            tick_raw,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let frame_b = support::perception_frame_for_profile_at_tick(
            organism_b.raw(),
            tick_raw,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let ticks = backend
            .tick_batch(&[(handle_a, frame_a.clone()), (handle_b, frame_b.clone())])
            .unwrap();
        let neutral_a = sealed_outcome(handle_a, &frame_a, &ticks[0], exposure + 1, 0.0, 0.0);
        let neutral_b = sealed_outcome(handle_b, &frame_b, &ticks[1], exposure + 1, 0.0, 0.0);
        assert_pending_matches_patch(handle_a, &ticks[0], &neutral_a);
        assert_pending_matches_patch(handle_b, &ticks[1], &neutral_b);
        assert_eq!(
            backend.pending_eligibility(handle_a).unwrap(),
            Some(ticks[0].pending_eligibility)
        );
        assert_eq!(
            backend.pending_eligibility(handle_b).unwrap(),
            Some(ticks[1].pending_eligibility)
        );
        let receipts = backend
            .apply_sealed_outcome_batch(&[(handle_a, &neutral_a), (handle_b, &neutral_b)])
            .unwrap_or_else(|error| panic!("neutral warmup exposure {exposure} failed: {error:?}"));
        assert_eq!(receipts[0].fast_weights_changed, 0);
        assert_eq!(receipts[1].fast_weights_changed, 0);
    }
    let frame_a = support::perception_frame_for_profile_at_tick(
        organism_a.raw(),
        1_100,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let frame_b = support::perception_frame_for_profile_at_tick(
        organism_b.raw(),
        1_100,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let first = backend
        .tick_batch(&[(handle_a, frame_a.clone()), (handle_b, frame_b.clone())])
        .unwrap();
    assert!((first[0].selection.logit - first[1].selection.logit).abs() <= f32::EPSILON);
    let rewarded = sealed_outcome(handle_a, &frame_a, &first[0], 9, 0.8, 0.0);
    let neutral = sealed_outcome(handle_b, &frame_b, &first[1], 9, 0.0, 0.0);
    let receipts = backend
        .apply_sealed_outcome_batch(&[(handle_a, &rewarded), (handle_b, &neutral)])
        .unwrap();
    assert_eq!(receipts.len(), 2);
    assert!(receipts[0].fast_weights_changed > 0);
    assert!(receipts[0].max_abs_delta > GPU_LEARNING_TOLERANCE);
    assert_eq!(receipts[1].fast_weights_changed, 0);
    assert_eq!(receipts[1].max_abs_delta, 0.0);
    let rewarded_fast = backend.read_active_fast_weights_for_test(handle_a).unwrap();
    let neutral_fast = backend.read_active_fast_weights_for_test(handle_b).unwrap();
    assert!(rewarded_fast.iter().any(|value| *value != 0.0));
    assert!(neutral_fast.iter().all(|value| *value == 0.0));

    let next_a = support::perception_frame_for_profile_at_tick(
        organism_a.raw(),
        1_102,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let next_b = support::perception_frame_for_profile_at_tick(
        organism_b.raw(),
        1_102,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let after = backend
        .tick_batch(&[(handle_a, next_a), (handle_b, next_b)])
        .unwrap();
    assert!(
        (after[0].selection.logit - after[1].selection.logit).abs() > GPU_LEARNING_TOLERANCE,
        "immediately active fast weights must causally alter the next decision: rewarded={}, neutral={}, delta={}, receipt_delta={}",
        after[0].selection.logit,
        after[1].selection.logit,
        (after[0].selection.logit - after[1].selection.logit).abs(),
        receipts[0].max_abs_delta,
    );
    for tick in &after {
        backend
            .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
            .unwrap();
    }
}

#[test]
fn reward_and_pain_change_the_next_decision_in_opposite_directions() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let organisms = [
        alife_core::OrganismId(4_131),
        alife_core::OrganismId(4_132),
        alife_core::OrganismId(4_133),
    ];
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handles =
        organisms.map(|organism| backend.insert_brain(organism, phenotype.clone()).unwrap());
    for exposure in 0_u64..8 {
        let tick_raw = 1_200 + exposure * 2;
        let frames = organisms.map(|organism| {
            support::perception_frame_for_profile_at_tick(
                organism.raw(),
                tick_raw,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            )
        });
        let ticks = backend
            .tick_batch(&[
                (handles[0], frames[0].clone()),
                (handles[1], frames[1].clone()),
                (handles[2], frames[2].clone()),
            ])
            .unwrap();
        let patches = std::array::from_fn::<_, 3, _>(|index| {
            sealed_outcome(
                handles[index],
                &frames[index],
                &ticks[index],
                exposure + 1,
                0.0,
                0.0,
            )
        });
        backend
            .apply_sealed_outcome_batch(&[
                (handles[0], &patches[0]),
                (handles[1], &patches[1]),
                (handles[2], &patches[2]),
            ])
            .unwrap_or_else(|error| {
                panic!("opposed-credit warmup exposure {exposure} failed: {error:?}")
            });
    }
    let frames = organisms.map(|organism| {
        support::perception_frame_for_profile_at_tick(
            organism.raw(),
            1_300,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        )
    });
    let before = backend
        .tick_batch(&[
            (handles[0], frames[0].clone()),
            (handles[1], frames[1].clone()),
            (handles[2], frames[2].clone()),
        ])
        .unwrap();
    let patches = [
        sealed_outcome(handles[0], &frames[0], &before[0], 9, 0.8, 0.0),
        sealed_outcome(handles[1], &frames[1], &before[1], 9, 0.0, 0.8),
        sealed_outcome(handles[2], &frames[2], &before[2], 9, 0.0, 0.0),
    ];
    let receipts = backend
        .apply_sealed_outcome_batch(&[
            (handles[0], &patches[0]),
            (handles[1], &patches[1]),
            (handles[2], &patches[2]),
        ])
        .unwrap();
    assert!(receipts[0].fast_weights_changed > 0);
    assert!(receipts[1].fast_weights_changed > 0);
    assert_eq!(receipts[2].fast_weights_changed, 0);

    let next_frames = organisms.map(|organism| {
        support::perception_frame_for_profile_at_tick(
            organism.raw(),
            1_302,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        )
    });
    let after = backend
        .tick_batch(&[
            (handles[0], next_frames[0].clone()),
            (handles[1], next_frames[1].clone()),
            (handles[2], next_frames[2].clone()),
        ])
        .unwrap();
    let reward_delta = after[0].selection.logit - after[2].selection.logit;
    let pain_delta = after[1].selection.logit - after[2].selection.logit;
    assert!(reward_delta.abs() > GPU_LEARNING_TOLERANCE);
    assert!(pain_delta.abs() > GPU_LEARNING_TOLERANCE);
    assert!(
        reward_delta * pain_delta < 0.0,
        "reward={reward_delta}, pain={pain_delta} must oppose around neutral"
    );
    for tick in &after {
        backend
            .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
            .unwrap();
    }
}

#[test]
fn decoder_credit_is_selected_family_and_selected_feature_specific() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let organism = alife_core::OrganismId(4_141);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle = backend.insert_brain(organism, phenotype.clone()).unwrap();
    for exposure in 0_u64..8 {
        let frame = support::perception_frame_for_profile_at_tick(
            organism.raw(),
            1_400 + exposure * 2,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let tick = backend
            .tick_batch(&[(handle, frame.clone())])
            .unwrap()
            .remove(0);
        let neutral = sealed_outcome(handle, &frame, &tick, exposure + 1, 0.0, 0.0);
        backend.apply_sealed_outcome(handle, &neutral).unwrap();
    }
    let frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        1_500,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let tick = backend
        .tick_batch(&[(handle, frame.clone())])
        .unwrap()
        .remove(0);
    let selected = frame.candidates()[usize::from(tick.selection.candidate_index)];
    let reward = sealed_outcome(handle, &frame, &tick, 9, 0.8, 0.0);
    backend.apply_sealed_outcome(handle, &reward).unwrap();
    let fast = backend.read_active_fast_weights_for_test(handle).unwrap();
    assert_eq!(fast.len(), phenotype.synapses().len());

    let mut credited_decoder_weights = 0_usize;
    for (index, synapse) in phenotype.synapses().iter().enumerate() {
        let alife_core::CompiledSynapseKind::Decoder(coordinate) = synapse.kind() else {
            continue;
        };
        let should_receive_credit = if coordinate.head()
            == alife_core::DecoderHeadKind::ActionCandidate
            && coordinate.family() == selected.family
        {
            *selected
                .features
                .0
                .get(usize::from(coordinate.input_lane()))
                .expect("action decoder lanes must stay inside the action feature ABI")
                != 0.0
        } else {
            false
        };
        if should_receive_credit {
            credited_decoder_weights += usize::from(fast[index] != 0.0);
        } else {
            assert_eq!(
                fast[index],
                0.0,
                "credit leaked to decoder {:?} lane {}",
                coordinate.family(),
                coordinate.input_lane()
            );
        }
    }
    assert!(credited_decoder_weights > 0);
}

#[test]
fn replayed_sealed_credit_is_rejected_without_blocking_a_later_tick() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let organism = alife_core::OrganismId(4_121);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle = backend.insert_brain(organism, phenotype).unwrap();
    let frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        1_100,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let tick = backend
        .tick_batch(&[(handle, frame.clone())])
        .unwrap()
        .remove(0);
    let patch = sealed_outcome(handle, &frame, &tick, 1, 0.8, 0.0);
    let committed = backend.apply_sealed_outcome(handle, &patch).unwrap();
    assert_eq!(
        backend.apply_sealed_outcome(handle, &patch),
        Err(alife_core::ScaffoldContractError::LearningReplayRejected)
    );

    let next_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        1_102,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let next_tick = backend
        .tick_batch(&[(handle, next_frame)])
        .unwrap()
        .remove(0);
    assert_eq!(committed.output_fast_generation, 2);
    backend
        .discard_pending_eligibility(handle, next_tick.pending_eligibility.identity())
        .unwrap();
}

#[test]
fn foreign_outcome_is_rejected_before_gpu_mutation_and_preserves_both_pending_rows() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let organisms = [alife_core::OrganismId(4_151), alife_core::OrganismId(4_152)];
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handles =
        organisms.map(|organism| backend.insert_brain(organism, phenotype.clone()).unwrap());
    let frames = organisms.map(|organism| {
        support::perception_frame_for_profile_at_tick(
            organism.raw(),
            1_600,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        )
    });
    let ticks = backend
        .tick_batch(&[
            (handles[0], frames[0].clone()),
            (handles[1], frames[1].clone()),
        ])
        .unwrap();
    let pending_before = [
        backend.pending_eligibility(handles[0]).unwrap().unwrap(),
        backend.pending_eligibility(handles[1]).unwrap().unwrap(),
    ];
    let patch_a = sealed_outcome(handles[0], &frames[0], &ticks[0], 1, 0.8, 0.0);
    assert_eq!(
        backend.apply_sealed_outcome(handles[1], &patch_a),
        Err(alife_core::ScaffoldContractError::LearningEvidenceMismatch)
    );
    assert_eq!(
        backend.pending_eligibility(handles[0]).unwrap(),
        Some(pending_before[0])
    );
    assert_eq!(
        backend.pending_eligibility(handles[1]).unwrap(),
        Some(pending_before[1])
    );
    assert!(backend
        .read_active_fast_weights_for_test(handles[0])
        .unwrap()
        .iter()
        .all(|value| *value == 0.0));
    assert!(backend
        .read_active_fast_weights_for_test(handles[1])
        .unwrap()
        .iter()
        .all(|value| *value == 0.0));
    for index in 0..2 {
        backend
            .discard_pending_eligibility(
                handles[index],
                ticks[index].pending_eligibility.identity(),
            )
            .unwrap();
    }
}

#[test]
fn abi_conversion_uses_only_validated_sealed_credit() {
    let phenotype = support::controlled_n512_phenotype_at_maturation(0.35);
    let organism = alife_core::OrganismId(4_102);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle = backend.insert_brain(organism, phenotype).unwrap();
    let frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        910,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        1,
    );
    let tick = backend
        .tick_batch(&[(handle, frame.clone())])
        .unwrap()
        .remove(0);
    let patch = sealed_outcome(handle, &frame, &tick, 1, 0.5, 0.0);
    let packet = OutcomeCreditPacket::from_sealed_patch(&patch).unwrap();
    let record = GpuOutcomeCreditRecord::try_from(&packet).unwrap();

    assert_eq!(record.schema_version, u32::from(packet.schema_version()));
    assert_eq!(
        record.active_activation_side,
        u32::from(tick.active_activation_side)
    );
    assert_eq!(
        record.dispatch_generation[0] as u64,
        tick.dispatch_generation & 0xffff_ffff
    );
    assert_eq!(record.modulator_value, packet.modulator().value());

    backend
        .discard_pending_eligibility(handle, tick.pending_eligibility.identity())
        .unwrap();
}
