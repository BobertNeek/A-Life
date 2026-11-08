mod support;
use alife_core::*;
use alife_gpu_backend::*;
fn phenotype(profile: SensorProfile) -> BrainPhenotype {
    let g = BrainGenome::scaffold(
        0x2048_2026,
        BrainCapacityClass::N2048_GROUNDED_DRIVE_RESEARCH_ID,
    );
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    PhenotypeCompiler::compile_n2048_grounded_drive_basis(g, d, profile)
        .unwrap()
        .0
}
fn rebuild(
    f: &PerceptionFrame,
    c: Vec<ActionCandidate>,
    slots: Vec<GroundedObjectSlotV1>,
) -> PerceptionFrame {
    PerceptionFrame::new(
        f.organism_id(),
        f.tick(),
        f.sensor_profile(),
        f.sensory().clone(),
        f.body(),
        *f.homeostasis(),
        c,
        SensorProfileProvenance::new(f.sensor_profile(), SensoryAbiVersion::CURRENT, f.tick())
            .unwrap(),
        slots,
    )
    .unwrap()
}
fn frame() -> PerceptionFrame {
    let f =
        support::perception_frame_for_profile(812, SensorProfile::GroundedObjectSlotsV1, true, 2);
    let mut slots = f.grounded_object_slots().to_vec();
    slots[0].chemical[0] = 0.7;
    slots[0].temperature = 0.8;
    slots[0].contact = 1.;
    slots[1].chemical[0] = -0.7;
    slots[1].temperature = -0.8;
    let mut c = f.candidates().to_vec();
    for (i, c) in c.iter_mut().enumerate() {
        c.target = ActionTarget::new(Some(WorldEntityId(200 + i as u64)), Some(Vec3f::ZERO));
        c.features = slots[i].candidate_features().unwrap();
    }
    rebuild(&f, c, slots)
}
#[test]
fn runtime_projection_matches_gpu_packing_and_rejects_tampering() {
    let p = phenotype(SensorProfile::GroundedObjectSlotsV1);
    let mut bucket =
        GpuClassBucketPlan::new(BrainCapacityClass::n2048_grounded_drive_research(), 1).unwrap();
    let slot = bucket.insert_phenotype(0, 7, &p).unwrap();
    let f = frame();
    let features = prepare_grounded_drive_features(&f).unwrap();
    assert_eq!(features[0][1], features[1][2]);
    assert_eq!(features[0][3], features[1][4]);
    assert_eq!(&features[0][8..], &[0., 0.]);
    assert_eq!(features[0][5], 0.); // No social body evidence, regardless of appearance.
    let upload = GpuPerceptionUpload::try_from_frame(&f, &slot, 0).unwrap();
    upload.validate_against(&f, &slot).unwrap();
    for (i, c) in upload.candidates.iter().enumerate() {
        let at = c.feature_offset as usize;
        assert_eq!(
            &upload.frame_payload_words[at..at + 24],
            &f.candidates()[i].features.0.map(f32::to_bits)
        );
        assert_eq!(
            &upload.frame_payload_words[at + 36..at + 46],
            &features[i].map(f32::to_bits)
        );
        assert_eq!(upload.frame_payload_words[at + 53], 0);
    }
    let mut bad = upload.clone();
    bad.frame_payload_words[upload.candidates[0].feature_offset as usize + 36] = 0;
    assert!(bad.validate_against(&f, &slot).is_err());
    let other = phenotype(SensorProfile::GroundedTerrainVisionV1);
    let mut bucket2 =
        GpuClassBucketPlan::new(BrainCapacityClass::n2048_grounded_drive_research(), 1).unwrap();
    let other_slot = bucket2.insert_phenotype(0, 7, &other).unwrap();
    assert!(GpuPerceptionUpload::try_from_frame(&f, &other_slot, 0).is_err());
}
#[test]
fn runtime_projection_rejects_cross_slot_entity_and_same_slot_target_aliases() {
    let f = frame();
    let mut c = f.candidates().to_vec();
    c[1].target = c[0].target;
    let alias = rebuild(&f, c, f.grounded_object_slots().to_vec());
    assert!(prepare_grounded_drive_features(&alias).is_err());
    let mut c = f.candidates().to_vec();
    c[1].observation = c[0].observation;
    c[1].features = c[0].features;
    let alias = rebuild(&f, c, f.grounded_object_slots().to_vec());
    assert!(prepare_grounded_drive_features(&alias).is_err());
}
#[test]
fn modified_shaders_parse_and_validate() {
    for source in [
        CLOSED_LOOP_DECODE_WGSL,
        CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
        CLOSED_LOOP_ELIGIBILITY_WGSL,
    ] {
        let module = naga::front::wgsl::parse_str(source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
}
