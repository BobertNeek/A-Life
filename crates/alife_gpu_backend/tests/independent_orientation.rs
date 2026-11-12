//! Explicit channel-policy fixtures, not evidence of learned navigation.
use alife_core::*;
use alife_gpu_backend::*;
fn phenotype(v2: bool, heads: u8) -> BrainPhenotype {
    let c = BrainCapacityClass::n512();
    let mut g = BrainGenome::scaffold(42, c.id());
    // Test-only zero innate bias: zero candidate features then give an exact tie.
    g.innate_priority.reflex_strength = 0.0;
    let mut a = serde_json::to_value(g.cognitive_architecture()).unwrap();
    a["motor_head_count"] = heads.into();
    let mut a: alife_core::genome::CognitiveArchitectureGenomeParameters =
        serde_json::from_value(a).unwrap();
    if v2 {
        a = a.with_independent_orientation_v2(heads).unwrap();
    }
    let g = g.with_cognitive_architecture(a).unwrap();
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    PhenotypeCompiler::compile(&g, &c, &d, SensorProfile::PrivilegedAffordanceV1).unwrap()
}
#[test]
fn versioned_mode_masks_preserve_legacy_and_enable_orientation() {
    for (v2, heads, mask) in [
        (false, 1, 1),
        (false, 2, 5),
        (true, 1, 1),
        (true, 2, 3),
        (true, 3, 7),
        (true, 4, 15),
        (true, 5, 31),
    ] {
        let p = phenotype(v2, heads);
        let u = GpuPhenotypeUpload::try_from(&p).unwrap();
        let marker = if v2 {
            GPU_ORIENTATION_LAYOUT_V2_MARKER
        } else {
            GPU_JOINT_SELECTION_V1_MARKER
        };
        assert_eq!(u.joint_motor_mode, marker | mask);
        assert!(valid_joint_motor_mode(u.joint_motor_mode));
        u.validate_against(&p).unwrap();
    }
    assert!(valid_joint_motor_mode(0));
    for m in [3, 7, 15, 31] {
        assert!(!valid_joint_motor_mode(GPU_JOINT_SELECTION_V1_MARKER | m));
    }
    for m in [0, 5, 13, 29, 63, 128] {
        assert!(!valid_joint_motor_mode(
            GPU_ORIENTATION_LAYOUT_V2_MARKER | m
        ));
    }
    // The six-slot selection receipt ABI is unchanged, including Orientation slot1.
    let slots = [1, 2, 0, 0, 0, 0];
    let packed = pack_joint_motor_candidates(slots);
    assert_eq!(packed[1] & 0xffff0000, GPU_JOINT_SELECTION_V1_MARKER);
}
#[cfg(feature = "gpu-tests")]
#[path = "support/mod.rs"]
mod support;
#[cfg(feature = "gpu-tests")]
mod device {
    use super::*;
    #[test]
    #[ignore = "explicit software Vulkan runtime fixture; not physical GPU qualification"]
    fn software_orientation_slot_is_independent_and_bound_in_native_joint_receipt() {
        let b = GpuClosedLoopBackend::new_software_vulkan_runtime_test(
            GpuRuntimeProfile::production_v1(),
        )
        .unwrap();
        let info = b.software_runtime_test_adapter_info().unwrap();
        eprintln!("orientation_test_adapter={info:?}");
        assert_eq!(info.device_type, wgpu::DeviceType::Cpu);
        check_orientation_slots(b);
    }

    #[test]
    fn hardware_orientation_slot_is_independent_and_bound_in_native_joint_receipt() {
        let b = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
        check_orientation_slots(b);
    }

    fn check_orientation_slots(mut b: GpuClosedLoopBackend) {
        for (case, (v2, look_first, look_id)) in [
            (false, false, 601),
            (true, false, 601),
            (false, true, 601),
            (true, true, 601),
            (true, false, 606),
            (true, false, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let p = phenotype(v2, 2);
            let id = OrganismId(75100 + case as u64);
            let h = b.insert_brain(id, p.clone()).unwrap();
            let template = support::perception_frame_for_profile_at_tick(
                id.raw(),
                4100,
                SensorProfile::PrivilegedAffordanceV1,
                false,
                2,
            );
            // Zero novelty and synthetic features with compatible families make the native deterministic representative
            // tie choose index0. These are sampler/receipt fixtures, not a diet or behavior test.
            let mut candidates = template.candidates().to_vec();
            let same = candidates[0];
            for (i, c) in candidates.iter_mut().enumerate() {
                *c = same;
                c.candidate_index = i as u16;
                let look = look_id != 0 && (i == 0) == look_first;
                c.kind = if look {
                    ActionKind::Look
                } else {
                    ActionKind::Move
                };
                c.family = if look {
                    CandidateActionFamily::Inspect
                } else {
                    CandidateActionFamily::Approach
                };
                c.features = CandidateFeatureVector::zero();
                c.action_id = ActionId(if look { look_id } else { 105 });
            }
            let frame = PerceptionFrame::new(
                id,
                template.tick(),
                template.sensor_profile(),
                template.sensory().clone(),
                template.body(),
                *template.homeostasis(),
                candidates,
                template.profile_provenance(),
                template.grounded_object_slots().to_vec(),
            )
            .unwrap();
            let body = CreatureGenome::early_mammal_founder(
                42,
                FoundationGeneticIdentity::new(22, 1, 1, BrainCapacityClass::n512().id()).unwrap(),
            )
            .unwrap()
            .express()
            .unwrap();
            let state = BiochemistryState::new(&body, frame.tick()).unwrap();
            let _receptors = state.neural_receptor_frame(&body).unwrap();
            let (frame, tick) = support::tick_with_receptors(&mut b, h, &p, &body, &frame).unwrap();
            assert_eq!(tick.selection.candidate_index, 0);
            let slots = tick.factorized_motor_candidates;
            assert_ne!(slots[0], 0);
            assert_eq!(slots[1] != 0, (v2 || look_first) && look_id != 0);
            let selected = frame.candidates()[0];
            let command = selected.to_command(id, tick.selection.confidence).unwrap();
            let bundle = factorized_motor_bundle_for_candidates(
                id,
                ExperienceSequenceId(1),
                frame.tick(),
                &frame,
                slots,
                &p.candidate_decoder().factorized_motor_channels(&p).unwrap(),
                &command,
                0,
                tick.speech_payload.as_ref(),
                false,
            )
            .unwrap();
            let identity = tick.pending_eligibility.identity();
            identity
                .joint_selection()
                .unwrap()
                .validate_bundle(&bundle)
                .unwrap();
            assert_eq!(
                bundle
                    .channels
                    .iter()
                    .any(|c| c.channel == MotorChannel::Orientation),
                (v2 || look_first) && look_id != 0
            );
            let mut tampered = bundle.clone();
            tampered.channels[0].primitive = ActionId(999);
            assert!(identity
                .joint_selection()
                .unwrap()
                .validate_bundle(&tampered)
                .is_err());
            b.discard_pending_eligibility(h, identity).unwrap();
        }
    }
}
