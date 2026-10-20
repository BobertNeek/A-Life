#![cfg(feature = "cpu-backend")]

use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, CandidateActionFamily, CompiledSynapseKind,
    DecoderHeadKind, DendriticBranch, DendriticBranchSet, DendriticInputRef, DevelopmentState,
    FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
};
use alife_training::cpu::*;
use alife_training::{
    AdamWConfig, CriticResidualRow, ImitationExample, ImitationTarget, PpoConfig, PpoJointAction,
    PpoValueHeadCheckpoint, ReplaySpeechTarget, StageTrainableMask, TrainingInitialState,
    TrainingReplayCandidate, TrainingReplayTick, TrainingSequence,
};

fn fixture() -> (BrainPhenotype, FoundationWeightAsset) {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let source =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let p = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &source,
    )
    .unwrap();
    (p, source)
}
fn trainer(p: &BrainPhenotype, source: &FoundationWeightAsset) -> CpuFoundationTrainer {
    CpuFoundationTrainer::new(
        p.clone(),
        source.clone(),
        StageTrainableMask::from_synapse_indices(
            p,
            &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
        )
        .unwrap(),
        AdamWConfig::default(),
    )
    .unwrap()
}
fn sequence(p: &BrainPhenotype, ticks: usize, burn_in: usize) -> TrainingSequence {
    let n = p.neuron_count() as usize;
    let initial = TrainingInitialState {
        activations: (0..n).map(|i| 0.1 + (i % 7) as f32 * 0.01).collect(),
        activity_ema: vec![0.2; n],
        metabolic_load: vec![0.03; n],
        dendrites: DendriticBranchSet::new(vec![DendriticBranch::new(
            35,
            0.02,
            0.4,
            vec![DendriticInputRef::new(6, 0.8).unwrap()],
        )
        .unwrap()])
        .unwrap(),
    };
    let mut inputs = [0.; 54];
    for (i, v) in inputs.iter_mut().enumerate() {
        *v = 0.03 * (i % 5) as f32;
    }
    let candidates = [
        CandidateActionFamily::Approach,
        CandidateActionFamily::Ingest,
        CandidateActionFamily::Idle,
    ]
    .into_iter()
    .map(|family| TrainingReplayCandidate {
        family,
        decoder_inputs: inputs,
        innate_bias: 0.04,
    })
    .collect::<Vec<_>>();
    let tick = TrainingReplayTick {
        encoded_inputs: (0..n).map(|i| (i % 11) as f32 * 0.015).collect(),
        projection_gain: 0.85,
        local_threshold_shift: 0.017,
        microstep_count: u32::from(p.microstep_count()),
        enabled_routes: vec![true; p.projections().len()],
        effective_weight_offsets: (0..p.synapses().len())
            .map(|i| (i % 7) as f32 * 0.002)
            .collect(),
        structural_synapses: vec![],
        candidates,
    };
    TrainingSequence {
        phenotype_hash: p.phenotype_hash(),
        initial,
        ticks: vec![tick; ticks],
        burn_in_ticks: burn_in,
        memory_candidate_gain: p
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    }
}
fn near(a: f32, b: f32, abs: f32, relative: f32) {
    assert!(
        (a - b).abs() <= abs + relative * a.abs().max(b.abs()),
        "{a} != {b}"
    );
}

#[test]
fn recurrent_formula_includes_leak_metabolic_and_context() {
    let (p, s) = fixture();
    let t = trainer(&p, &s);
    let mut q = sequence(&p, 1, 0);
    q.ticks[0].enabled_routes.fill(false);
    q.ticks[0].microstep_count = 1;
    let r = t.evaluate_replay(&q).unwrap();
    for i in [0, 35, 123, 2047] {
        let d = &p.neuron_dynamics()[i];
        let mut dendrite = 0_f32;
        if i == 35 {
            dendrite = (q.initial.activations[6] * 0.8 - 0.02).max(0.).tanh() * 0.4;
        }
        let x = d.bias() + q.ticks[0].encoded_inputs[i] + 0.85 * dendrite
            - d.homeostatic_gain() * 0.03
            - 0.017;
        let act = match d.activation().raw() {
            0 => x,
            1 => x.max(0.),
            2 => x.tanh(),
            3 => 1. / (1. + (-x).exp()),
            _ => panic!(),
        };
        let output = (1. - d.leak()) * q.initial.activations[i] + d.leak() * act;
        near(r.final_activations[0][i], output, 2e-6, 2e-5);
        near(
            r.final_metabolic_load[0][i],
            (d.metabolic_decay() * 0.03 + (1. - d.metabolic_decay()) * output * output)
                .clamp(0., 1.),
            2e-6,
            2e-5,
        );
        near(
            r.final_activity_ema[0][i],
            (d.activity_ema_decay() * 0.2 + (1. - d.activity_ema_decay()) * output.abs())
                .clamp(0., 1.),
            2e-6,
            2e-5,
        );
    }
    assert_eq!(t.hardware_receipt().backend, "native-cpu");
}

#[test]
fn bptt_matches_finite_differences_with_detached_burn_in_and_private_context() {
    let (p, s) = fixture();
    let t = trainer(&p, &s);
    let q = sequence(&p, 3, 1);
    let adjoints = vec![vec![0.3, -0.2, 0.1], vec![0.1, 0.2, -0.3]];
    let analytic = t.probe_replay_gradients(&q, &adjoints).unwrap();
    let eval = t.evaluate_replay(&q).unwrap();
    let mut suffix = q.clone();
    suffix.initial.activations = eval.final_activations[0].clone();
    suffix.initial.metabolic_load = eval.final_metabolic_load[0].clone();
    suffix.initial.activity_ema = eval.final_activity_ema[0].clone();
    suffix.ticks.remove(0);
    suffix.burn_in_ticks = 0;
    let suffix_adjoint = adjoints.clone();
    let mut selected = Vec::new();
    for kind in 0..4 {
        let best = p
            .synapses()
            .iter()
            .enumerate()
            .filter(|(_, s)| match (kind, s.kind()) {
                (0, CompiledSynapseKind::Recurrent) => true,
                (1, CompiledSynapseKind::Decoder(c)) => {
                    c.head() == DecoderHeadKind::ActionCandidate
                }
                (2, CompiledSynapseKind::Decoder(c)) => c.head() == DecoderHeadKind::MemoryContext,
                (3, CompiledSynapseKind::Decoder(c)) => {
                    c.head() == DecoderHeadKind::CognitiveContext
                }
                _ => false,
            })
            .max_by(|(a, _), (b, _)| {
                analytic.gradients[*a]
                    .abs()
                    .total_cmp(&analytic.gradients[*b].abs())
            });
        if let Some((id, _)) = best {
            if analytic.gradients[id].abs() > 1e-7 {
                selected.push(id);
            }
        }
    }
    assert!(selected.len() >= 2);
    for id in selected {
        let objective = |sign: f32| {
            let mut ck = t.checkpoint();
            ck.weights[id] += sign * 0.002;
            let perturbed =
                CpuFoundationTrainer::from_checkpoint(p.clone(), s.clone(), ck).unwrap();
            let r = perturbed.evaluate_replay(&suffix).unwrap();
            r.candidate_logits
                .iter()
                .zip(&suffix_adjoint)
                .flat_map(|(ls, gs)| ls.iter().zip(gs))
                .map(|(l, g)| f64::from(*l) * f64::from(*g))
                .sum::<f64>()
        };
        let numerical = ((objective(1.) - objective(-1.)) / 0.004) as f32;
        near(analytic.gradients[id], numerical, 3e-5, 0.035);
    }
    let mut invalid = adjoints.clone();
    invalid.insert(0, vec![0.; 3]);
    assert!(t.probe_replay_gradients(&q, &invalid).is_err());
    let mut masked = q.clone();
    masked.ticks[1].microstep_count = 0;
    let r = t.evaluate_replay(&masked).unwrap();
    assert_eq!(r.final_activations[0], r.final_activations[1]);
}

fn action() -> PpoJointAction {
    PpoJointAction {
        candidate_count: 4,
        representative_mask: 15,
        motor_masks: [3, 12, 0, 0, 0, 0],
        representative: 0,
        forced_slots: vec![Some(0), Some(0), Some(1), Some(1)],
        motor_candidates: [Some(0), Some(3), None, None, None, None],
        old_joint_log_probability: -1.,
        temperature: 0.7,
    }
}
#[test]
fn ppo_joint_probability_and_entropy_gradient_match_numerical_objective() {
    let logits = [0.2, -0.4, 0.7, 0.1];
    let mut a = action();
    let config = PpoConfig {
        temperature: a.temperature,
        entropy_coefficient: 0.1,
        ..Default::default()
    };
    a.old_joint_log_probability = cpu_ppo_objective(&logits, &a, 0.3, config)
        .unwrap()
        .joint_log_probability;
    let r = cpu_ppo_objective(&logits, &a, 0.3, config).unwrap();
    near(r.ratio, 1., 1e-6, 0.);
    let z = logits.iter().map(|x| (*x / 0.7_f32).exp()).sum::<f32>();
    let conditional =
        (logits[3] / 0.7_f32).exp() / ((logits[2] / 0.7_f32).exp() + (logits[3] / 0.7_f32).exp());
    near(
        r.joint_log_probability,
        (logits[0] / 0.7_f32).exp().ln() - z.ln() + conditional.ln(),
        1e-6,
        1e-5,
    );
    for i in 0..4 {
        let loss = |sign: f32| {
            let mut l = logits;
            l[i] += sign * 0.001;
            let o = cpu_ppo_objective(&l, &a, 0.3, config).unwrap();
            o.policy_loss - config.entropy_coefficient * o.entropy
        };
        near(
            r.logit_gradients[i],
            (loss(1.) - loss(-1.)) / 0.002,
            7e-5,
            0.005,
        );
    }
    let mut bad = a.clone();
    bad.temperature = 1.;
    assert!(cpu_ppo_objective(&logits, &bad, 0.3, config).is_err());
}

#[test]
fn imitation_marginalizes_representative_overrides() {
    let logits = [0.2, -0.4, 0.7, 0.1];
    let a = action();
    let e = ImitationExample {
        candidate_count: 4,
        representative_mask: a.representative_mask,
        motor_masks: a.motor_masks,
        forced_slots: a.forced_slots,
        target: ImitationTarget {
            representative: None,
            motors: [Some(2), Some(12), None, None, None, None],
        },
    };
    let r = cpu_imitation_objective(&logits, &e, 0.7, 0.4).unwrap();
    for i in 0..4 {
        let loss = |sign: f32| {
            let mut l = logits;
            l[i] += sign * 0.001;
            cpu_imitation_objective(&l, &e, 0.7, 0.4).unwrap().loss
        };
        near(
            r.logit_gradients[i],
            (loss(1.) - loss(-1.)) / 0.002,
            8e-5,
            0.005,
        );
    }
}

#[test]
fn imitation_rejects_overflowing_loss_even_with_finite_gradients() {
    let e = ImitationExample {
        candidate_count: 2,
        representative_mask: 3,
        motor_masks: [0; 6],
        forced_slots: vec![None; 2],
        target: ImitationTarget {
            representative: Some(2),
            motors: [None; 6],
        },
    };
    assert!(cpu_imitation_objective(&[0., -2e38], &e, 1., 2.).is_err());
}

#[test]
fn native_checkpoint_roundtrip_preserves_accepted_ages_and_frozen_coordinates() {
    let (p, s) = fixture();
    let mut t = trainer(&p, &s);
    let mask = StageTrainableMask::from_synapse_indices(&p, &[17]).unwrap();
    t.set_stage_mask(mask).unwrap();
    let mut c = t.checkpoint();
    c.optimizer_step = 16_684;
    c.update_ages[17] = 16_684;
    c.first_moment[17] = 0.03;
    c.second_moment[17] = 0.02;
    c.update_ages[18] = 321;
    c.first_moment[18] = -0.07;
    c.second_moment[18] = 0.04;
    let bytes = serde_json::to_vec(&c).unwrap();
    t.restore_checkpoint(serde_json::from_slice(&bytes).unwrap())
        .unwrap();
    assert_eq!(t.checkpoint(), c);
    let mut gradients = vec![0.; c.weights.len()];
    gradients[17] = 0.2;
    t.apply_gradients(&gradients).unwrap();
    let after = t.checkpoint();
    assert_eq!(after.optimizer_step, 16_685);
    assert_eq!(after.update_ages[17], 16_685);
    for i in 0..c.weights.len() {
        if i != 17 {
            assert_eq!(after.weights[i].to_bits(), c.weights[i].to_bits());
            assert_eq!(after.update_ages[i], c.update_ages[i]);
            assert_eq!(after.first_moment[i].to_bits(), c.first_moment[i].to_bits());
            assert_eq!(
                after.second_moment[i].to_bits(),
                c.second_moment[i].to_bits()
            );
        }
    }
    let mut large = after.clone();
    large.optimizer_step = 16_777_217;
    large.update_ages[17] = 16_777_217;
    t.restore_checkpoint(large.clone()).unwrap();
    assert_eq!(t.checkpoint().update_ages[17], 16_777_217);
    let mut overflow = large;
    overflow.optimizer_step = u32::MAX;
    t.restore_checkpoint(overflow.clone()).unwrap();
    assert!(t.apply_gradients(&gradients).is_err());
    assert_eq!(t.checkpoint(), overflow);
    let value = PpoValueHeadCheckpoint {
        feature_count: 2,
        optimizer_step: 19_800,
        parameters: vec![0.2, -0.3, 0.1, 0.03, 0.02, 0.01, 0.04, 0.02, 0.03],
        last_updated_policy_version: Some(7400),
    };
    let restored = CpuValueHead::from_checkpoint(
        serde_json::from_slice(&serde_json::to_vec(&value).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(restored.checkpoint(), value);
}

#[test]
fn ppo_clips_both_advantage_signs_without_extra_forced_probability() {
    let logits = [0.2, -0.4, 0.7, 0.1];
    let mut a = action();
    let config = PpoConfig {
        temperature: 0.7,
        entropy_coefficient: 0.,
        ..Default::default()
    };
    let current = cpu_ppo_objective(&logits, &a, 0., config)
        .unwrap()
        .joint_log_probability;
    for (advantage, ratio, clipped) in [(1_f32, 1.5_f32, 1.2_f32), (-1., 0.5, 0.8)] {
        a.old_joint_log_probability = current - ratio.ln();
        let o = cpu_ppo_objective(&logits, &a, advantage, config).unwrap();
        assert!(o.clipped);
        near(o.policy_loss, -clipped * advantage, 2e-6, 1e-5);
        assert!(o.logit_gradients.iter().all(|g| g.abs() < 1e-7));
    }
}

#[test]
fn structural_context_participates_in_recurrence_and_source_bptt() {
    let (p, s) = fixture();
    let t = trainer(&p, &s);
    let mut q = sequence(&p, 2, 0);
    let route = p
        .projections()
        .iter()
        .position(|r| r.update_cadence().raw() <= 4)
        .unwrap();
    for tick in &mut q.ticks {
        tick.structural_synapses
            .push(alife_training::TrainingFrozenSynapse {
                source: 6,
                target: 35,
                route: route as u32,
                cadence: u32::from(p.projections()[route].update_cadence().raw()),
                effective_weight: 0.7,
            });
    }
    let adjoints = vec![vec![0.3, -0.2, 0.1]; 2];
    let analytic = t.probe_replay_gradients(&q, &adjoints).unwrap();
    let id = p
        .synapses()
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.kind(), CompiledSynapseKind::Recurrent))
        .max_by(|(a, _), (b, _)| {
            analytic.gradients[*a]
                .abs()
                .total_cmp(&analytic.gradients[*b].abs())
        })
        .unwrap()
        .0;
    let objective = |sign: f32| {
        let mut ck = t.checkpoint();
        ck.weights[id] += sign * 0.002;
        let perturbed = CpuFoundationTrainer::from_checkpoint(p.clone(), s.clone(), ck).unwrap();
        let e = perturbed.evaluate_replay(&q).unwrap();
        e.candidate_logits
            .iter()
            .zip(&adjoints)
            .flat_map(|(l, g)| l.iter().zip(g))
            .map(|(l, g)| f64::from(*l) * f64::from(*g))
            .sum::<f64>()
    };
    near(
        analytic.gradients[id],
        ((objective(1.) - objective(-1.)) / 0.004) as f32,
        3e-5,
        0.035,
    );
    let with = t.evaluate_replay(&q).unwrap();
    for tick in &mut q.ticks {
        tick.structural_synapses.clear();
    }
    let without = t.evaluate_replay(&q).unwrap();
    assert_ne!(
        with.final_activations[0][35].to_bits(),
        without.final_activations[0][35].to_bits()
    );
}

#[test]
fn critic_residual_adam_differentiates_next_state_and_bias() {
    let c = PpoValueHeadCheckpoint {
        feature_count: 2,
        optimizer_step: 19_800,
        parameters: vec![0.2, -0.3, 0.1, 0., 0., 0., 0., 0., 0.],
        last_updated_policy_version: Some(7400),
    };
    let mut v = CpuValueHead::from_checkpoint(c.clone()).unwrap();
    let features = vec![vec![0.3, 0.8], vec![0.7, 0.2]];
    let residual = v.predict(&features[0]).unwrap() - 0.15 - 0.9 * v.predict(&features[1]).unwrap();
    let config = AdamWConfig {
        gradient_clip: 100.,
        ..Default::default()
    };
    v.residual_step(
        &features,
        &[CriticResidualRow {
            current: 0,
            next: 1,
            reward: 0.15,
            discount: 0.9,
        }],
        config,
        0.5,
        7401,
    )
    .unwrap();
    let after = v.checkpoint();
    assert_eq!(after.optimizer_step, 19_801);
    assert_eq!(after.last_updated_policy_version, Some(7401));
    for (i, delta) in [0.3 - 0.9 * 0.7, 0.8 - 0.9 * 0.2, 0.1]
        .into_iter()
        .enumerate()
    {
        near(
            after.parameters[3 + i],
            (1. - config.beta1) * 0.5 * residual * delta,
            2e-7,
            1e-5,
        );
    }
    let before = v.checkpoint();
    assert!(v
        .residual_step(
            &features,
            &[CriticResidualRow {
                current: 0,
                next: 9,
                reward: 0.15,
                discount: 0.9
            }],
            config,
            0.5,
            7402
        )
        .is_err());
    assert_eq!(v.checkpoint(), before);
}

#[test]
fn private_somatic_weights_and_neural_states_do_not_cross_organisms() {
    let (p, s) = fixture();
    let founder = trainer(&p, &s);
    let q = sequence(&p, 1, 0);
    let left = CpuOrganismState::new(&p, alife_core::OrganismId(1), q.initial.clone()).unwrap();
    let original = left.checkpoint().unwrap();
    let mut changed = original.clone();
    changed.organism_id = alife_core::OrganismId(2);
    changed.lifetime[0] = 0.3;
    changed.fast[0] = 0.4;
    changed.neural.activations[0] = 0.7;
    let right = CpuOrganismState::from_checkpoint(&p, changed.clone()).unwrap();
    assert_eq!(left.checkpoint().unwrap(), original);
    assert_eq!(right.checkpoint().unwrap(), changed);
    near(
        right.effective_offsets(&p).unwrap()[0],
        0.3 + p.synapses()[0].alpha() * 0.4,
        1e-7,
        0.,
    );
    assert_eq!(founder.optimizer_step(), 0);
    let mut bad = original.clone();
    bad.eligibility[0] = 0.1;
    assert!(CpuOrganismState::from_checkpoint(&p, bad).is_err());
    let mut bad = original;
    bad.last_decision_tick = Some(3);
    assert!(CpuOrganismState::from_checkpoint(&p, bad).is_err());
}

#[test]
fn rebind_fresh_cohort_retains_native_optimizer_state() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7411, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let base = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let source = FoundationWeightAsset::from_phenotype_for_genetic_birth(&base).unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development.clone(),
        source.clone(),
    )
    .unwrap();
    let mut t = trainer(&p, &source);
    let mut gradients = vec![0.; p.synapses().len()];
    gradients[17] = 0.2;
    t.apply_gradients(&gradients).unwrap();
    let before = t.checkpoint();
    let asset = t
        .export_candidate(alife_core::TrainingStageManifest::new(1, 1, 1))
        .unwrap();
    let (next, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())
            .unwrap();
    t.rebind_for_next_cohort(next.clone(), asset.clone())
        .unwrap();
    let after = t.checkpoint();
    assert_eq!(after.weights, before.weights);
    assert_eq!(after.first_moment, before.first_moment);
    assert_eq!(after.second_moment, before.second_moment);
    assert_eq!(after.update_ages, before.update_ages);
    assert_eq!(after.optimizer_step, before.optimizer_step);
    assert_eq!(after.stage_mask, before.stage_mask);
    assert_eq!(after.source_foundation_digest, asset.digest());
    assert_eq!(after.phenotype_hash, next.phenotype_hash());
}

#[test]
fn six_step_speech_bptt_matches_finite_differences() {
    let (p, s) = fixture();
    let t = trainer(&p, &s);
    let q = sequence(&p, 2, 0);
    let labels = vec![
        Some(ReplaySpeechTarget {
            token: Some(8),
            continuation: [1, 0, 0, 0, 0],
            act: alife_core::SpeechActKind::Declare,
            weight: 1.,
        }),
        Some(ReplaySpeechTarget {
            token: None,
            continuation: [0; 5],
            act: alife_core::SpeechActKind::Declare,
            weight: 1.,
        }),
    ];
    let adjoints = vec![vec![0.; 3]; 2];
    let analytic = t
        .probe_replay_gradients_with_speech(&q, &adjoints, &labels)
        .unwrap();
    let ids=p.synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload))
        .map(|(i,_)|i).collect::<Vec<_>>();
    let id = *ids
        .iter()
        .max_by(|a, b| {
            analytic.gradients[**a]
                .abs()
                .total_cmp(&analytic.gradients[**b].abs())
        })
        .unwrap();
    assert!(analytic.gradients[id].abs() > 1e-6);
    let objective = |sign: f32| {
        let mut ck = t.checkpoint();
        ck.weights[id] += sign * 0.002;
        let other = CpuFoundationTrainer::from_checkpoint(p.clone(), s.clone(), ck).unwrap();
        other
            .probe_replay_gradients_with_speech(&q, &adjoints, &labels)
            .unwrap()
            .objective
    };
    near(
        analytic.gradients[id],
        ((objective(1.) - objective(-1.)) / 0.004) as f32,
        6e-5,
        0.01,
    );
}

#[test]
fn cpu_cohort_updates_both_native_optimizers_and_rejects_stale_policy() {
    use alife_training::{PpoBatch, PpoBoundary, PpoTrainingWindow, PpoTransition};
    let (p, s) = fixture();
    let mut t = trainer(&p, &s);
    let q = sequence(&p, 2, 0);
    let eval = t.evaluate_replay(&q).unwrap();
    let config = PpoConfig {
        epochs: 1,
        ..Default::default()
    };
    let transitions = (0..2)
        .map(|i| {
            let mut a = PpoJointAction {
                candidate_count: 3,
                representative_mask: 7,
                motor_masks: [0; 6],
                representative: 0,
                forced_slots: vec![None; 3],
                motor_candidates: [None; 6],
                old_joint_log_probability: -1.,
                temperature: config.temperature,
            };
            a.old_joint_log_probability =
                cpu_ppo_objective(&eval.candidate_logits[i], &a, 0., config)
                    .unwrap()
                    .joint_log_probability;
            PpoTransition {
                policy_version: 7400,
                trajectory_id: 1,
                step: i as u64,
                action: a,
                reward: 0.3,
                old_value: 0.,
                next_value: 0.,
                elapsed_seconds: 0.05,
                boundary: if i == 1 {
                    PpoBoundary::Terminated
                } else {
                    PpoBoundary::Continuing
                },
            }
        })
        .collect();
    let batch = PpoBatch::from_rollout(7400, transitions, config).unwrap();
    let w = PpoTrainingWindow {
        sequence: q,
        batch,
        auxiliary: None,
        speech_targets: vec![None; 2],
    };
    let mut value = CpuValueHead::from_checkpoint(PpoValueHeadCheckpoint {
        feature_count: p.neuron_count(),
        optimizer_step: 19_800,
        parameters: vec![0.; (p.neuron_count() as usize + 1) * 3],
        last_updated_policy_version: Some(7399),
    })
    .unwrap();
    let baseline_actor = t.checkpoint();
    let baseline_value = value.checkpoint();
    let mut bad = w.clone();
    bad.speech_targets[1] = Some(ReplaySpeechTarget {
        token: Some(0),
        continuation: [0; 5],
        act: alife_core::SpeechActKind::Declare,
        weight: 1.,
    });
    assert!(train_cpu_ppo_cohort(
        &mut t,
        &mut value,
        2,
        1,
        |i| Ok(if i == 0 { w.clone() } else { bad.clone() }),
        config,
        7400
    )
    .is_err());
    assert_eq!(t.checkpoint(), baseline_actor);
    assert_eq!(value.checkpoint(), baseline_value);
    let started = std::time::Instant::now();
    let receipt =
        train_cpu_ppo_cohort(&mut t, &mut value, 2, 2, |_| Ok(w.clone()), config, 7400).unwrap();
    eprintln!(
        "CPU_TIMING {}",
        serde_json::json!({"wall_seconds":started.elapsed().as_secs_f64(),"numerical_ppo_decisions":4,
        "canonical_valid_decisions":0,"actor_updates":receipt.actor_optimizer_step,"critic_updates":receipt.value_optimizer_step-19_800,
        "hardware":t.hardware_receipt(),"profile":"unoptimized numerical fixture; no behavioral throughput claim"})
    );
    assert_eq!(receipt.completed_epochs, 1);
    assert_eq!(receipt.actor_optimizer_step, 1);
    assert_eq!(receipt.value_optimizer_step, 19_801);
    let before = t.checkpoint();
    let vbefore = value.checkpoint();
    assert!(
        train_cpu_ppo_cohort(&mut t, &mut value, 1, 1, |_| Ok(w.clone()), config, 7400).is_err()
    );
    assert_eq!(t.checkpoint(), before);
    assert_eq!(value.checkpoint(), vbefore);
}

#[cfg(feature = "gpu-tests")]
#[test]
fn cpu_differential_matches_required_hardware_sparse_replay_and_gradients() {
    let (p, s) = fixture();
    let cpu = trainer(&p, &s);
    let q = sequence(&p, 3, 1);
    let mask = cpu.checkpoint().stage_mask;
    let mut gpu =
        alife_training::FoundationTrainer::new_required(p, s, mask, AdamWConfig::default())
            .unwrap();
    let c = cpu.evaluate_replay(&q).unwrap();
    let g = gpu.evaluate_replay(&q).unwrap();
    for (c, g) in c
        .candidate_logits
        .iter()
        .flatten()
        .zip(g.candidate_logits.iter().flatten())
    {
        near(*c, *g, 2e-5, 3e-4);
    }
    for (c, g) in c
        .final_activations
        .iter()
        .flatten()
        .zip(g.final_activations.iter().flatten())
    {
        near(*c, *g, 2e-5, 3e-4);
    }
    let adjoints = vec![vec![0.3, -0.2, 0.1]; 2];
    let c = cpu.probe_replay_gradients(&q, &adjoints).unwrap();
    let g = gpu.probe_replay_gradients(&q, &adjoints).unwrap();
    for (c, g) in c.gradients.iter().zip(&g.gradients) {
        near(*c, *g, 3e-5, 8e-4);
    }
}

#[test]
fn native_signed_decoder_adam_keeps_negative_coefficients() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let p = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let mut weights = p
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    let selected = p
        .synapses()
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match s.kind() {
            CompiledSynapseKind::Decoder(c)
                if matches!(
                    c.head(),
                    DecoderHeadKind::ActionCandidate | DecoderHeadKind::SpeechPayload
                ) =>
            {
                Some(i)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!selected.is_empty());
    for &i in &selected {
        weights[i] = -0.3;
    }
    let asset = FoundationWeightAsset::from_trained_weights(
        &p,
        weights,
        alife_core::TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let genome = BrainGenome::scaffold(0x7A11, BrainCapacityClass::n2048().id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let (signed, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())
            .unwrap();
    let mut trainer = trainer(&signed, &asset);
    trainer
        .apply_gradients(&vec![0.; signed.synapses().len()])
        .unwrap();
    for i in selected {
        assert!(trainer.weights()[i] < 0.);
    }
    let exported = trainer
        .export_candidate(alife_core::TrainingStageManifest::new(1, 1, 1))
        .unwrap();
    let genome = BrainGenome::scaffold(0x7A11, BrainCapacityClass::n2048().id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, exported).unwrap();
}
