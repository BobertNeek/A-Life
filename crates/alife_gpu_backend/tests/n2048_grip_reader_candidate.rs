use alife_core::*;
use alife_gpu_backend::{
    GpuClassBucketPlan, GpuMemoryChannelPlan, GpuPhenotypeUpload, CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
};
#[test]
fn grip_identity_uses_the_same_memory_upload_coordinates_as_cpu() {
    let g = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let p = PhenotypeCompiler::compile_n2048_grip_reader_graph(
        g,
        d,
        SensorProfile::GroundedGripObjectSlotsV1,
    )
    .unwrap()
    .0;
    let upload = GpuPhenotypeUpload::try_from(&p).unwrap();
    // CPU SparseGraph accumulates each family's memory in phenotype order.
    // The schema3 GPU map must retain that exact f32 summation order too.
    let expected_memory = p.synapses().iter().enumerate().filter_map(|(index, s)| {
        matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::MemoryContext)
            .then_some(upload.canonical_to_local_synapse[index])
    }).collect::<Vec<_>>();
    assert_eq!(upload.memory_weight_indices, expected_memory);
    let plan = upload.memory_channel_plans[0];
    assert_eq!(plan.schema_version, 3);
    assert_eq!(plan.decoder_input_stride, 54);
    assert_eq!(plan.max_candidate_gain, 12.);
    assert_eq!(plan.memory_decoder_synapse_count, 4096);
    assert_eq!(
        plan.max_candidate_gain,
        p.candidate_decoder()
            .memory_channel()
            .unwrap()
            .max_candidate_gain()
    );
    let mut bucket = GpuClassBucketPlan::new(BrainCapacityClass::n2048(), 1).unwrap();
    bucket.insert_phenotype(0, 1, &p).unwrap();
    let copy =
        GpuMemoryChannelPlan::try_from(p.candidate_decoder().memory_channel().unwrap()).unwrap();
    assert_eq!(copy.max_candidate_gain, 12.);
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_MEMORY_CONTEXT_WGSL).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
fn grip_slots_reject_legacy_frames_even_when_numeric_action_matches_release() {
    use alife_gpu_backend::GpuPerceptionUpload;
    fn phenotype(grip: bool) -> BrainPhenotype {
        let g = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
        let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
        if grip {
            PhenotypeCompiler::compile_n2048_grip_reader_graph(
                g,
                d,
                SensorProfile::GroundedGripObjectSlotsV1,
            )
            .unwrap()
            .0
        } else {
            PhenotypeCompiler::compile_n2048_memory_gain_graph(
                g,
                d,
                SensorProfile::GroundedObjectSlotsV1,
            )
            .unwrap()
            .0
        }
    }
    fn frame(profile: SensorProfile) -> PerceptionFrame {
        let tick = Tick::ZERO;
        let (action, kind, family) = if profile == SensorProfile::GroundedObjectSlotsV1 {
            (
                ActionId(2202),
                ActionKind::Interact,
                CandidateActionFamily::Other,
            )
        } else {
            (
                ActionKind::Idle.canonical_id(),
                ActionKind::Idle,
                CandidateActionFamily::Idle,
            )
        };
        PerceptionFrameDraft::new(
            OrganismId(1),
            tick,
            profile,
            SensorySnapshot::new(
                OrganismId(1),
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
            vec![ActionCandidate::new(
                0,
                action,
                kind,
                family,
                CandidateObservationRef::None,
                ActionTarget::NONE,
                CandidateFeatureVector::zero(),
                Confidence::new(1.).unwrap(),
                NormalizedScalar::new(0.).unwrap(),
                DurationTicks::new(1),
                DurationTicks::new(1),
            )
            .unwrap()],
            SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
            Vec::new(),
        )
        .unwrap()
        .finalize(PerceptionContextBlock::empty())
        .unwrap()
    }
    let mut old_bucket = GpuClassBucketPlan::new(BrainCapacityClass::n2048(), 1).unwrap();
    let old_slot = old_bucket
        .insert_phenotype(0, 1, &phenotype(false))
        .unwrap();
    let mut new_bucket = GpuClassBucketPlan::new(BrainCapacityClass::n2048(), 1).unwrap();
    let new_slot = new_bucket.insert_phenotype(0, 1, &phenotype(true)).unwrap();
    let old_frame = frame(SensorProfile::GroundedObjectSlotsV1);
    let new_frame = frame(SensorProfile::GroundedGripObjectSlotsV1);
    assert!(GpuPerceptionUpload::try_from_frame(&old_frame, &old_slot, 0).is_ok());
    assert!(GpuPerceptionUpload::try_from_frame(&new_frame, &new_slot, 0).is_ok());
    assert!(GpuPerceptionUpload::try_from_frame(&old_frame, &new_slot, 0).is_err());
    assert!(GpuPerceptionUpload::try_from_frame(&new_frame, &old_slot, 0).is_err());
}
