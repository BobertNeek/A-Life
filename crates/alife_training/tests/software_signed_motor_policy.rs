//! Actual WGSL execution on explicit Cpu/Vulkan, never hardware qualification.
#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, CompiledSynapseKind, DecoderHeadKind,
    DevelopmentState, FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler, ProjectionType,
    SensorProfile, Tick, TrainingStageManifest,
};
use alife_runtime::GpuAuthoritativeSession;
use alife_training::{
    AdamWConfig, FoundationTrainer, FoundationTrainerCheckpoint, StageTrainableMask,
    TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick, TrainingSequence,
};
use wgpu::util::DeviceExt;

fn fixture(signed: bool, motor_weight: f32) -> (BrainPhenotype, FoundationWeightAsset, [usize; 3]) {
    let genome = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let compile = if signed {
        PhenotypeCompiler::compile_n2048_signed_motor_v3_graph
    } else {
        PhenotypeCompiler::compile_n2048_anchored_v2_graph
    };
    let (base, _) = compile(
        genome.clone(),
        development.clone(),
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let mut indices = [
        ProjectionType::MotorProposal,
        ProjectionType::Homeostatic,
        ProjectionType::LateralInhibition,
    ]
    .map(|projection| {
        base.synapses()
            .iter()
            .position(|s| {
                s.kind() == CompiledSynapseKind::Recurrent
                    && base.projections()[s.route_index() as usize].projection_type() == projection
            })
            .unwrap()
    });
    // Choose a motor target with an actual action readout, so the same fixture
    // can prove a nonzero recurrent gradient rather than only momentum decay.
    indices[0] = base.synapses().iter().position(|motor| {
        motor.kind() == CompiledSynapseKind::Recurrent
            && base.projections()[motor.route_index() as usize].projection_type() == ProjectionType::MotorProposal
            && base.synapses().iter().any(|decoder| decoder.source() == motor.target()
                && matches!(decoder.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate))
    }).unwrap();
    let motor_route = base.synapses()[indices[0]].route_index();
    let mut weights = base
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    for (index, synapse) in base.synapses().iter().enumerate() {
        if synapse.route_index() == motor_route {
            weights[index] = 0.0;
        }
    }
    weights[indices[0]] = motor_weight;
    weights[indices[1]] = 0.00001;
    weights[indices[2]] = -0.00001;
    let readout = base.synapses().iter().position(|s| s.source() == base.synapses()[indices[0]].target()
        && matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate)).unwrap();
    weights[readout] = 1.0;
    let asset = FoundationWeightAsset::from_trained_weights(
        &base,
        weights,
        TrainingStageManifest::bootstrap(),
    )
    .unwrap();
    let (phenotype, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        genome,
        development,
        asset.clone(),
    )
    .unwrap();
    assert_eq!(
        phenotype.synapses()[indices[0]].genetic_weight().to_bits(),
        motor_weight.to_bits()
    );
    (phenotype, asset, indices)
}

fn trainer(
    phenotype: BrainPhenotype,
    asset: FoundationWeightAsset,
    indices: &[usize],
) -> FoundationTrainer {
    // No optional adapter or early-return path: missing Cpu/Vulkan fails the test.
    let session = GpuAuthoritativeSession::new_software_vulkan_training_test().unwrap();
    assert!(session.backend().is_software_training_test());
    let info = session
        .backend()
        .software_training_test_adapter_info()
        .unwrap();
    assert_eq!(info.device_type, wgpu::DeviceType::Cpu);
    assert_eq!(info.backend, wgpu::Backend::Vulkan);
    eprintln!("signed_motor_actual_adapter={info:?}");
    let mask = StageTrainableMask::from_synapse_indices(
        &phenotype,
        &indices
            .iter()
            .map(|&index| index as u32)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    FoundationTrainer::from_session(session, phenotype, asset, mask, AdamWConfig::default())
        .unwrap()
}

fn sequence(p: &BrainPhenotype, motor: usize) -> TrainingSequence {
    let n = p.neuron_count() as usize;
    let synapse = &p.synapses()[motor];
    let readout = p
        .synapses()
        .iter()
        .find_map(|s| match s.kind() {
            CompiledSynapseKind::Decoder(c)
                if s.source() == synapse.target()
                    && c.head() == DecoderHeadKind::ActionCandidate =>
            {
                Some(c)
            }
            _ => None,
        })
        .unwrap();
    let mut decoder_inputs = [0.0; 54];
    decoder_inputs[usize::from(readout.input_lane())] = 1.0;
    let mut activations = vec![0.0; n];
    activations[synapse.source() as usize] = 1.0;
    let mut enabled_routes = vec![false; p.projections().len()];
    enabled_routes[synapse.route_index() as usize] = true;
    TrainingSequence {
        phenotype_hash: p.phenotype_hash(),
        initial: TrainingInitialState {
            activations,
            activity_ema: vec![0.0; n],
            metabolic_load: vec![0.0; n],
            dendrites: Default::default(),
        },
        ticks: vec![TrainingReplayTick {
            encoded_inputs: vec![0.0; n],
            projection_gain: 1.0,
            local_threshold_shift: 0.0,
            microstep_count: 1,
            enabled_routes,
            effective_weight_offsets: vec![0.0; p.synapses().len()],
            structural_synapses: vec![],
            candidates: vec![TrainingReplayCandidate {
                family: readout.family(),
                decoder_inputs,
                innate_bias: 0.0,
            }],
        }],
        burn_in_ticks: 0,
        memory_candidate_gain: p
            .candidate_decoder()
            .memory_channel()
            .map_or(0.0, |m| m.max_candidate_gain()),
    }
}

fn gpu_update(
    trainer: &mut FoundationTrainer,
    replay: &TrainingSequence,
    objective_adjoint: f32,
) -> FoundationTrainerCheckpoint {
    // Execute actual forward/backward and Adam pipelines. A zero adjoint isolates
    // stored momentum; a nonzero adjoint exercises fresh gradient-driven learning.
    trainer.evaluate_replay(replay).unwrap();
    let (device, _) = trainer
        .session()
        .backend()
        .offline_training_device_queue()
        .unwrap();
    let mut adjoint_values = [0.0_f32; alife_core::MAX_ACTION_CANDIDATES];
    adjoint_values[0] = objective_adjoint;
    let adjoints = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("signed-motor-zero-objective-adjoints"),
        contents: bytemuck::cast_slice(&adjoint_values),
        usage: wgpu::BufferUsages::COPY_SRC,
    });
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    trainer.submit_replay_update(encoder, &adjoints, 0).unwrap();
    trainer.checkpoint().unwrap()
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 2e-6 + expected.abs() * 2e-5,
        "actual={actual}, expected={expected}"
    );
}

#[test]
#[ignore = "explicit software Vulkan execution; no physical GPU qualification"]
fn signed_motor_v3_executes_negative_forward_and_unclamped_adam_on_software_vulkan() {
    let mut responses = Vec::new();
    for weight in [-0.3, 0.0, 0.3] {
        let (p, asset, indices) = fixture(true, weight);
        let target = p.synapses()[indices[0]].target() as usize;
        let replay = sequence(&p, indices[0]);
        let mut gpu = trainer(p, asset, &indices);
        responses.push(gpu.evaluate_replay(&replay).unwrap().final_activations[0][target]);
    }
    assert!(
        responses[0] < responses[1] && responses[1] < responses[2],
        "GPU forward responses={responses:?}"
    );
    eprintln!("signed_motor_gpu_forward_executed={responses:?}");

    let mut motor_moments = Vec::new();
    for signed in [false, true] {
        let (p, asset, indices) = fixture(signed, 0.00001);
        let replay = sequence(&p, indices[0]);
        let mut gpu = trainer(p, asset, &indices);
        let fresh = gpu.checkpoint().unwrap();
        let mut before = fresh.clone();
        before.optimizer_step = 16_777_219;
        before.first_moment.fill(0.05);
        before.second_moment.fill(0.1);
        before.update_ages.fill(7);
        before.update_ages[indices[0]] = 16_777_217;
        before.update_ages[indices[1]] = 13;
        before.update_ages[indices[2]] = 16_777_218;
        // The younger homeostatic coordinate needs stronger momentum to cross
        // zero after bias correction; it must still retain its nonnegative clamp.
        before.first_moment[indices[1]] = 0.5;
        before.first_moment[indices[2]] = -0.05;
        gpu.restore_checkpoint(&before).unwrap();
        assert_eq!(gpu.checkpoint().unwrap(), before);
        let after = gpu_update(&mut gpu, &replay, 0.0);
        assert_eq!(after.optimizer_step, before.optimizer_step + 1);
        for index in 0..before.weights.len() {
            if !indices.contains(&index) {
                assert_eq!(
                    after.weights[index].to_bits(),
                    before.weights[index].to_bits()
                );
                assert_eq!(
                    after.first_moment[index].to_bits(),
                    before.first_moment[index].to_bits()
                );
                assert_eq!(
                    after.second_moment[index].to_bits(),
                    before.second_moment[index].to_bits()
                );
                assert_eq!(after.update_ages[index], before.update_ages[index]);
            } else {
                near(
                    after.first_moment[index],
                    before.config.beta1 * before.first_moment[index],
                );
                near(
                    after.second_moment[index],
                    before.config.beta2 * before.second_moment[index],
                );
                assert_eq!(after.update_ages[index], before.update_ages[index] + 1);
            }
        }
        if signed {
            assert!(after.weights[indices[0]] < 0.0);
        } else {
            assert_eq!(after.weights[indices[0]].to_bits(), 0.0001_f32.to_bits());
        }
        assert_eq!(after.weights[indices[1]].to_bits(), 0.0001_f32.to_bits());
        assert_eq!(after.weights[indices[2]].to_bits(), (-0.0001_f32).to_bits());
        motor_moments.push((
            after.first_moment[indices[0]].to_bits(),
            after.second_moment[indices[0]].to_bits(),
            after.update_ages[indices[0]],
        ));
        let bytes = serde_json::to_vec(&after).unwrap();
        let restored = serde_json::from_slice(&bytes).unwrap();
        let uninterrupted = gpu_update(&mut gpu, &replay, 0.0);
        gpu.restore_checkpoint(&restored).unwrap();
        assert_eq!(gpu.checkpoint().unwrap(), after);
        assert_eq!(gpu_update(&mut gpu, &replay, 0.0), uninterrupted);
        eprintln!("signed_motor_gpu_adam_executed: v3={signed}; motor={}; homeostatic={}; inhibition={}; integer_age={}", after.weights[indices[0]], after.weights[indices[1]], after.weights[indices[2]], after.update_ages[indices[0]]);
        gpu.restore_checkpoint(&fresh).unwrap();
        let probe = gpu.probe_replay_gradients(&replay, &[vec![1.0]]).unwrap();
        assert!(
            probe.gradients[indices[0]] > 0.0,
            "motor gradient={}",
            probe.gradients[indices[0]]
        );
        let learned = gpu_update(&mut gpu, &replay, 1.0);
        assert_eq!(learned.weights[indices[0]] < 0.0, signed);
        assert_eq!(learned.update_ages[indices[0]], 1);
        assert!(learned.first_moment[indices[0]] > 0.0);
        assert!(learned.second_moment[indices[0]] > 0.0);
        eprintln!(
            "signed_motor_gpu_gradient_crossing_executed: v3={signed}; gradient={}; weight={}",
            probe.gradients[indices[0]], learned.weights[indices[0]]
        );
    }
    assert_eq!(motor_moments[0], motor_moments[1]);
}
