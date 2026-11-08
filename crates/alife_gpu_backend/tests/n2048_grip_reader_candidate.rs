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
    // Resolve the GPU map through its public packed learning/eligibility records,
    // independently of the training-only canonical_to_local_synapse convenience
    // field. Compare coordinates and weight bits in order, not sorted sets.
    let expected_memory = p
        .synapses()
        .iter()
        .filter_map(|s| match s.kind() {
            CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::MemoryContext => {
                Some([
                    c.head().raw(),
                    u32::from(c.family().raw()),
                    u32::from(c.input_lane()),
                    s.source(),
                    s.target(),
                    u32::from(s.receptor_index()),
                    s.genetic_weight().to_bits(),
                    s.alpha().to_bits(),
                ])
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let actual_memory = upload
        .memory_weight_indices
        .iter()
        .map(|&local_id| {
            let local = local_id as usize;
            let learning = &upload.synapse_learning_metadata[local];
            assert_eq!(learning.global_synapse_id, local_id);
            assert_eq!(learning.kind, CompiledSynapseKind::DECODER_RAW);
            let decoder = &upload.decoder_eligibility_metadata
                [learning.decoder_metadata_local_or_max as usize];
            assert_eq!(decoder.global_synapse_id, local_id);
            assert_eq!(
                decoder.eligibility_local_index,
                learning.eligibility_local_index
            );
            assert_eq!(decoder.receptor_index, learning.receptor_index);
            // The GPU eligibility ABI's motor_index stores the absolute source
            // neuron, not the phenotype's lobe-local output ordinal.
            assert_eq!(decoder.motor_index, learning.source_neuron);
            [
                decoder.decoder_head,
                decoder.family,
                decoder.input_lane,
                learning.source_neuron,
                learning.target_neuron,
                learning.receptor_index,
                upload.genetic_weights[local].to_bits(),
                upload.alpha[local].to_bits(),
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_memory, expected_memory);
    assert_eq!(expected_memory.len(), 4096);
    assert_eq!(
        expected_memory
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4096
    );
    // Sensitivity check: naive lane53-last ordering must differ from the
    // intended interleaving of the 64 retargeted source55 coordinates.
    let mut naive_lane_order = expected_memory.clone();
    naive_lane_order.sort_by_key(|row| (row[0], row[1], row[2], row[4], row[3]));
    assert_ne!(actual_memory, naive_lane_order);
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
        // The legacy raw action ID is permitted, but Other/Interact still needs
        // ordinary object grounding. This must be a valid legacy frame before
        // testing whether the new-profile slot rejects its ABI.
        let grounded_slots = if profile == SensorProfile::GroundedObjectSlotsV1 {
            vec![GroundedObjectSlotV1 {
                slot_index: 0,
                tracked_object_id: TrackedObjectId(77),
                bearing: [0., 1.],
                distance: 0.02,
                relative_velocity: [0.; 3],
                color: [0.2; 3],
                material: [0.1; 3],
                shape: [0.3; 3],
                chemical: [0.; 3],
                contact: 1.,
                proprioception: [0.; 2],
                temperature: 0.,
                terrain: [0.; 2],
                confidence: Confidence::new(1.).unwrap(),
            }]
        } else {
            Vec::new()
        };
        let (observation, target, features) = if let Some(slot) = grounded_slots.first() {
            (
                CandidateObservationRef::ObjectSlot(slot.slot_index),
                ActionTarget::new(Some(WorldEntityId(9)), None),
                slot.candidate_features().unwrap(),
            )
        } else {
            (
                CandidateObservationRef::None,
                ActionTarget::NONE,
                CandidateFeatureVector::zero(),
            )
        };
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
                observation,
                target,
                features,
                Confidence::new(1.).unwrap(),
                NormalizedScalar::new(0.).unwrap(),
                DurationTicks::new(1),
                DurationTicks::new(1),
            )
            .unwrap()],
            SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
            grounded_slots,
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
    old_frame.validate().unwrap();
    new_frame.validate().unwrap();
    assert!(GpuPerceptionUpload::try_from_frame(&old_frame, &old_slot, 0).is_ok());
    assert!(GpuPerceptionUpload::try_from_frame(&new_frame, &new_slot, 0).is_ok());
    assert!(GpuPerceptionUpload::try_from_frame(&old_frame, &new_slot, 0).is_err());
    assert!(GpuPerceptionUpload::try_from_frame(&new_frame, &old_slot, 0).is_err());
}
