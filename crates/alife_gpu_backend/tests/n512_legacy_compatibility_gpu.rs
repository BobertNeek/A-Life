#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::{
    BiochemistryState, BodyEventDelta, BrainCapacityClass, BrainGenome, Confidence,
    DecisionSnapshot, DevelopmentState, EndocrineDelta, ExperiencePatch, ExperiencePatchBuilder,
    ExperienceSequenceId, FoundationWeightAsset, HomeostaticDelta,
    LegacyNano512CompatibilityReceipt, MeasuredPhysiologyTransition, NeuralActionSelection,
    NormalizedScalar, PhenotypeCompiler, PhysicalActionOutcome, PhysicalContactKind,
    PostActionOutcome, PreActionSnapshot, SensorProfile, SignedValence, Tick, Vec3f,
};
use alife_gpu_backend::{
    GpuBrainRestoreRequest, GpuClassBucketPlan, GpuClosedLoopBackend, GpuRuntimeProfile,
};

const FOUNDATION_SEED: u64 = 0x4E35_3132_5F00_0001;

fn compatibility_phenotype() -> (
    alife_core::BrainPhenotype,
    LegacyNano512CompatibilityReceipt,
    FoundationWeightAsset,
) {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let capacity = BrainCapacityClass::n512();
    let asset = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let genome = BrainGenome::scaffold(FOUNDATION_SEED, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let (phenotype, receipt) = PhenotypeCompiler::compile_from_legacy_nano512_compatibility_asset(
        &genome,
        &capacity,
        &development,
        profile,
        &asset,
    )
    .unwrap()
    .into_parts();
    (phenotype, receipt, asset)
}

fn sealed_measured_outcome(
    handle: alife_gpu_backend::GpuBrainHandle,
    frame: &alife_core::PerceptionFrame,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    physiology: &alife_core::CreaturePhenotype,
) -> (ExperiencePatch, alife_core::NeuralReceptorFrame) {
    let sequence_id = ExperienceSequenceId(1);
    let genome = BrainGenome::scaffold(FOUNDATION_SEED, handle.class_id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let selection = NeuralActionSelection {
        candidate_index: tick.selection.candidate_index,
        logit: tick.selection.logit,
        confidence: tick.selection.confidence,
        active_tiles: tick.selection.active_tiles,
        active_synapses: tick.selection.active_synapses,
    };
    let command = frame.candidates()[usize::from(selection.candidate_index)]
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
    let before = BiochemistryState::new(physiology, frame.tick()).unwrap();
    let receptors = before.neural_receptor_frame(physiology).unwrap();
    let after = before
        .advance(
            Tick::new(frame.tick().raw() + 1),
            BodyEventDelta::zero(),
            physiology,
        )
        .unwrap();
    let measured = MeasuredPhysiologyTransition::new(before, after).unwrap();
    let outcome = PostActionOutcome::new(
        handle.organism_id(),
        sequence_id,
        Tick::new(frame.tick().raw() + 1),
        true,
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
        SignedValence::ZERO,
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        SignedValence::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
    )
    .unwrap()
    .with_measured_physiology(measured)
    .unwrap();
    let patch = ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap();
    (patch, receptors)
}

#[test]
fn canonical_v2_control_ticks_under_the_same_profile() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(FOUNDATION_SEED, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let phenotype = PhenotypeCompiler::compile(&genome, &capacity, &development, profile).unwrap();
    let organism = alife_core::OrganismId(51_200_003);
    let physiology = support::test_physiology(51_200_003, &phenotype).unwrap();
    let mut backend =
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let handle = backend.insert_brain(organism, phenotype.clone()).unwrap();
    let source =
        support::perception_frame_for_profile_at_tick(organism.raw(), 51_200, profile, true, 2);
    support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &source).unwrap();
}

#[test]
fn explicit_legacy_selector_uses_ordinary_gpu_learning_checkpoint_and_restore() {
    let (phenotype, receipt, asset) = compatibility_phenotype();
    let selector_digest = phenotype.foundation_abi().selector_digest();
    let receipt_digest = receipt.canonical_digest();
    let plan = GpuClassBucketPlan::for_phenotype(&phenotype).unwrap();
    plan.slot_allocation_receipt()
        .unwrap()
        .validate_contract()
        .unwrap();

    let organism = alife_core::OrganismId(51_200_001);
    let mut source =
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let hardware = source.hardware_receipt();
    println!("BACKEND_API={:?}", hardware.backend_api);
    println!("ADAPTER={}", hardware.adapter_name);
    let handle = source.insert_brain(organism, phenotype.clone()).unwrap();
    let physiology = support::test_physiology(42, &phenotype).unwrap();

    let source_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        51_200,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        2,
    );
    let (frame, tick) =
        support::tick_with_receptors(&mut source, handle, &phenotype, &physiology, &source_frame)
            .unwrap();
    let (patch, receptors) = sealed_measured_outcome(handle, &frame, &tick, &physiology);
    assert_eq!(
        source
            .sealed_outcome_credit_mismatch_receipt(handle, &patch)
            .unwrap(),
        None
    );
    source
        .apply_sealed_outcome(handle, &patch, &receptors)
        .unwrap();

    let snapshot = source.snapshot_brain(handle, Tick::new(51_201)).unwrap();
    let checkpoint_digest = snapshot.canonical_digest();
    let restored_phenotype = phenotype.clone();
    assert_eq!(
        restored_phenotype.foundation_abi().selector_digest(),
        selector_digest
    );
    assert_eq!(receipt.canonical_digest(), receipt_digest);
    receipt
        .validate_against(&restored_phenotype, &asset)
        .unwrap();

    let mut restored =
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let restore = restored
        .restore_brain(
            organism,
            restored_phenotype.clone(),
            GpuBrainRestoreRequest::try_new(snapshot).unwrap(),
        )
        .unwrap();
    assert!(restore.pending_eligibility.is_none());
    assert_eq!(
        restored
            .snapshot_brain(restore.handle, Tick::new(51_201))
            .unwrap()
            .canonical_digest(),
        checkpoint_digest
    );

    let probe = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        51_202,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        2,
    );
    let (_probe, restored_tick) = support::tick_with_receptors(
        &mut restored,
        restore.handle,
        &restored_phenotype,
        &physiology,
        &probe,
    )
    .unwrap();
    restored
        .discard_pending_eligibility(restore.handle, restored_tick.pending_eligibility.identity())
        .unwrap();
}
