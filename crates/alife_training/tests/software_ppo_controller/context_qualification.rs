//! Synthetic causal private-context qualification through the actual controller.
use super::*;
use alife_core::{
    DendriticBranch, DendriticBranchSet, DendriticInputRef, LobeKind, ProjectionType,
};
use alife_training::TrainingFrozenSynapse;

fn remove_context(sequence: &mut TrainingSequence) {
    sequence.initial.dendrites = Default::default();
    for row in &mut sequence.ticks {
        row.effective_weight_offsets.fill(0.0);
        row.structural_synapses.clear();
    }
}

fn recurrent_gradient_difference(
    trainer: &FoundationTrainer,
    left: &[f32],
    right: &[f32],
) -> usize {
    assert_eq!(left.len(), trainer.phenotype().synapses().len());
    assert_eq!(right.len(), left.len());
    let mask = trainer.checkpoint().unwrap().stage_mask;
    trainer
        .phenotype()
        .synapses()
        .iter()
        .enumerate()
        .filter(|(i, synapse)| {
            synapse.kind() == CompiledSynapseKind::Recurrent
                && mask.is_trainable(*i)
                && left[*i].is_finite()
                && right[*i].is_finite()
                && left[*i] != right[*i]
        })
        .count()
}

fn causal_ablations(trainer: &mut FoundationTrainer, sequence: &TrainingSequence) {
    let full = trainer.evaluate_replay(sequence).unwrap();
    let adjoints = vec![vec![0.2, -0.1, 0.05, 0.0]; sequence.ticks.len()];
    let full_gradient = trainer.probe_replay_gradients(sequence, &adjoints).unwrap();
    for component in ["dendrites", "structural", "offsets", "all"] {
        let mut without = sequence.clone();
        if component == "dendrites" || component == "all" {
            without.initial.dendrites = Default::default();
        }
        for row in &mut without.ticks {
            if component == "structural" || component == "all" {
                row.structural_synapses.clear();
            }
            if component == "offsets" || component == "all" {
                row.effective_weight_offsets.fill(0.0);
            }
        }
        let evaluation = trainer.evaluate_replay(&without).unwrap();
        assert!(
            full.candidate_logits
                .iter()
                .zip(&evaluation.candidate_logits)
                .any(|(a, b)| bits(a) != bits(b)),
            "{component} must causally affect GPU logits"
        );
        let gradient = trainer.probe_replay_gradients(&without, &adjoints).unwrap();
        let changed =
            recurrent_gradient_difference(trainer, &full_gradient.gradients, &gradient.gradients);
        assert!(
            changed > 0,
            "{component} must causally affect an actual trainable recurrent gradient"
        );
        eprintln!("private-context ablation {component}: GPU logits changed; {changed} trainable recurrent raw gradients differ");
    }
}

fn collect_split(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    sequence: TrainingSequence,
    config: PpoConfig,
) -> Vec<PpoTrainingWindow> {
    // Generate collection evidence only after the actual frozen context is set.
    let evaluation = trainer.evaluate_replay(&sequence).unwrap();
    let values = state.predict_values(trainer, &sequence).unwrap();
    let transitions = (0..7)
        .map(|index| {
            let logits = &evaluation.candidate_logits[index];
            let maximum = logits[..3]
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);
            let p = logits[..3]
                .iter()
                .map(|v| (f64::from(*v) - f64::from(maximum)).exp())
                .collect::<Vec<_>>();
            let z = p.iter().sum::<f64>();
            let old_logp = ((p[2] / z) * p[0] / (p[0] + p[1])).ln() as f32;
            PpoTransition {
                policy_version: 6,
                trajectory_id: 42,
                step: 100 + index as u64,
                action: PpoJointAction {
                    candidate_count: 4,
                    representative_mask: 7,
                    motor_masks: [3, 0, 0, 0, 0, 0],
                    representative: 2,
                    forced_slots: vec![Some(0), Some(0), Some(4), Some(4)],
                    motor_candidates: [Some(0), None, None, None, Some(2), None],
                    old_joint_log_probability: old_logp,
                    temperature: 1.0,
                },
                reward: if index % 2 == 0 { 0.6 } else { -0.15 } + index as f32 * 0.04,
                old_value: values[index],
                next_value: values[index + 1],
                elapsed_seconds: [0.04, 0.08, 0.05, 0.06, 0.03, 0.09, 0.07][index],
                boundary: if index == 6 {
                    PpoBoundary::Truncated
                } else {
                    PpoBoundary::Continuing
                },
            }
        })
        .collect::<Vec<_>>();
    assert_ne!(transitions[6].next_value, 0.0);
    let batch = PpoBatch::from_rollout(6, transitions, config).unwrap();
    let mut first = sequence.clone();
    first.ticks.truncate(3);
    let mut second = sequence.clone();
    second.ticks = sequence.ticks[1..7].to_vec();
    second.burn_in_ticks = 2;
    second.initial.activations = evaluation.final_activations[0].clone();
    second.initial.activity_ema = evaluation.final_activity_ema[0].clone();
    second.initial.metabolic_load = evaluation.final_metabolic_load[0].clone();
    // Dendrites are immutable descriptors. WGSL computes their output from the
    // previous activation; there is no additional dynamic branch register.
    assert_eq!(second.initial.dendrites, sequence.initial.dendrites);
    let replay = trainer.evaluate_replay(&second).unwrap();
    let replay_values = state.predict_values(trainer, &second).unwrap();
    for local in 0..6 {
        let global = local + 1;
        assert_eq!(
            bits(&replay.candidate_logits[local]),
            bits(&evaluation.candidate_logits[global])
        );
        assert_eq!(
            bits(&replay.final_activations[local]),
            bits(&evaluation.final_activations[global])
        );
        assert_eq!(
            bits(&replay.final_activity_ema[local]),
            bits(&evaluation.final_activity_ema[global])
        );
        assert_eq!(
            bits(&replay.final_metabolic_load[local]),
            bits(&evaluation.final_metabolic_load[global])
        );
        assert_eq!(replay_values[local].to_bits(), values[global].to_bits());
    }
    vec![
        PpoTrainingWindow {
            sequence: first,
            batch: batch.window(0..3).unwrap(),
            speech_targets: vec![None; 3],
            auxiliary: None,
        },
        PpoTrainingWindow {
            sequence: second,
            batch: batch.window(3..7).unwrap(),
            speech_targets: vec![None; 6],
            auxiliary: None,
        },
    ]
}

fn assert_progress(
    trainer: &FoundationTrainer,
    state: &mut PpoTrainingState,
    actor: &FoundationTrainerCheckpoint,
    value: &PpoValueHeadCheckpoint,
    updates: u32,
) -> usize {
    let after = trainer.checkpoint().unwrap();
    super::fallback_qualification::assert_actor_progress(actor, &after, updates);
    let recurrent = trainer
        .phenotype()
        .synapses()
        .iter()
        .enumerate()
        .filter(|(i, s)| {
            s.kind() == CompiledSynapseKind::Recurrent
                && actor.stage_mask.is_trainable(*i)
                && actor.weights[*i].to_bits() != after.weights[*i].to_bits()
                && actor.first_moment[*i].to_bits() != after.first_moment[*i].to_bits()
                && actor.second_moment[*i].to_bits() != after.second_moment[*i].to_bits()
        })
        .count();
    assert!(recurrent > 0);
    let after_value = state.checkpoint(trainer.session()).unwrap();
    assert_eq!(after_value.optimizer_step, value.optimizer_step + updates);
    let width = value.feature_count as usize + 1;
    for plane in 0..3 {
        let range = plane * width..(plane + 1) * width;
        assert_ne!(
            bits(&value.parameters[range.clone()]),
            bits(&after_value.parameters[range])
        );
    }
    recurrent
}

#[test]
#[ignore = "explicit synthetic software private-context and dendrite actual PPO qualification"]
fn nonempty_dendrites_and_changing_private_context_match_fresh_ppo() {
    let (mut trainer, weight, neuron, lane) = fixture_with_wide_mask(true);
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 2,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        value_coefficient: 0.5,
        ..Default::default()
    };
    let warm_config = PpoConfig {
        epochs: 1,
        ..config
    };
    let initial_actor = trainer.checkpoint().unwrap();
    let warm = windows_with_count(
        &mut trainer,
        &mut state,
        neuron,
        lane,
        warm_config,
        5,
        0.0,
        2,
    );
    let (receipt, _, _) = run_owned(&mut trainer, &mut state, &warm, warm_config, 5, false);
    assert_eq!(receipt.unwrap().completed_epochs, 1);
    let before_actor = trainer.checkpoint().unwrap();
    super::fallback_qualification::assert_actor_progress(&initial_actor, &before_actor, 1);
    let before_value = state.checkpoint(trainer.session()).unwrap();
    assert_eq!(before_value.optimizer_step, 1);
    let width = before_value.feature_count as usize + 1;
    for plane in 0..3 {
        assert!(before_value.parameters[plane * width..(plane + 1) * width]
            .iter()
            .any(|&v| v != 0.0));
    }
    let mut template = windows_with_shapes(
        &mut trainer,
        &mut state,
        neuron,
        lane,
        config,
        6,
        0.0,
        &[(8, 0)],
        true,
    )
    .remove(0)
    .sequence;
    let route = trainer
        .phenotype()
        .projections()
        .iter()
        .find(|p| {
            p.source_lobe() == LobeKind::ActionPlanning
                && p.target_lobe() == LobeKind::ActionPlanning
                && p.projection_type() == ProjectionType::LateralInhibition
        })
        .unwrap();
    let route_index = u32::from(route.route_index());
    let cadence = u32::from(route.update_cadence().raw());
    assert!(cadence <= 4);
    template.initial.dendrites = DendriticBranchSet::new(vec![DendriticBranch::new(
        neuron as u32,
        -0.2,
        0.25,
        vec![
            DendriticInputRef::new(neuron as u32, 0.6).unwrap(),
            DendriticInputRef::new(neuron as u32 + 1, 0.1).unwrap(),
        ],
    )
    .unwrap()])
    .unwrap();
    for (index, row) in template.ticks.iter_mut().enumerate() {
        row.effective_weight_offsets[weight] = 0.012 + 0.001 * index as f32;
        row.structural_synapses = (0..index % 3)
            .map(|edge| TrainingFrozenSynapse {
                source: neuron as u32 + edge as u32,
                target: neuron as u32,
                route: route_index,
                cadence,
                effective_weight: -0.08 - 0.01 * index as f32 - 0.02 * edge as f32,
            })
            .collect();
    }
    template.validate_for(trainer.phenotype()).unwrap();
    assert!(!template.initial.dendrites.branches().is_empty());
    assert_eq!(
        template
            .ticks
            .iter()
            .map(|r| r.structural_synapses.len())
            .collect::<Vec<_>>(),
        [0, 1, 2, 0, 1, 2, 0, 1]
    );
    causal_ablations(&mut trainer, &template);
    assert_persistent_equal(&before_actor, &before_value, &trainer, &mut state);
    let contextual = collect_split(&mut trainer, &mut state, template.clone(), config);
    let mut bare = template;
    remove_context(&mut bare);
    let ordinary = collect_split(&mut trainer, &mut state, bare, config);
    assert_persistent_equal(&before_actor, &before_value, &trainer, &mut state);
    let (control, _, loads) = run_owned(&mut trainer, &mut state, &ordinary, config, 6, false);
    assert_eq!(control.unwrap().completed_epochs, 2);
    assert_eq!(loads, [0, 1, 0, 1, 0, 1]);
    assert_progress(&trainer, &mut state, &before_actor, &before_value, 2);
    let control_actor = trainer.checkpoint().unwrap();
    trainer.restore_checkpoint(&before_actor).unwrap();
    state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
    let (probe, _, loads) = run_owned(&mut trainer, &mut state, &contextual, warm_config, 6, false);
    assert_eq!(probe.unwrap().completed_epochs, 1);
    assert_eq!(loads, [0, 1, 0, 1]);
    let first_recurrent = assert_progress(&trainer, &mut state, &before_actor, &before_value, 1);
    trainer.restore_checkpoint(&before_actor).unwrap();
    state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
    let (normal, _, loads) = run_owned(&mut trainer, &mut state, &contextual, config, 6, false);
    let normal = normal.unwrap();
    assert_eq!(normal.completed_epochs, 2);
    assert!(!normal.stopped_for_kl);
    assert_eq!(loads, [0, 1, 0, 1, 0, 1]);
    let recurrent = assert_progress(&trainer, &mut state, &before_actor, &before_value, 2);
    let after_actor = trainer.checkpoint().unwrap();
    let after_value = state.checkpoint(trainer.session()).unwrap();
    assert!(
        trainer
            .phenotype()
            .synapses()
            .iter()
            .enumerate()
            .any(|(i, s)| {
                s.kind() == CompiledSynapseKind::Recurrent
                    && before_actor.stage_mask.is_trainable(i)
                    && after_actor.first_moment[i].to_bits()
                        != control_actor.first_moment[i].to_bits()
            }),
        "actual context PPO must differ from its independently collected no-context control"
    );
    let expected = capture(&trainer, &mut state);
    trainer.restore_checkpoint(&before_actor).unwrap();
    state = PpoTrainingState::from_checkpoint(before_value).unwrap();
    let shared = contextual.into_iter().map(Arc::new).collect::<Vec<_>>();
    let mut stats = FrozenGradientCacheStats::default();
    let mut loads = Vec::new();
    let receipt = train_recurrent_ppo_cohort_with_frozen_gradients_for_test(
        &mut trainer,
        &mut state,
        2,
        8,
        |i| {
            loads.push(i);
            Ok(Arc::clone(&shared[i]))
        },
        config,
        6,
        &mut stats,
    )
    .unwrap();
    assert_active_cache(stats);
    assert_eq!(loads, [0, 1, 0, 1, 0, 1]);
    assert_eq!(receipt_words(&normal), receipt_words(&receipt));
    assert_actor_equal(&after_actor, &trainer.checkpoint().unwrap());
    assert_value_equal(&after_value, &state.checkpoint(trainer.session()).unwrap());
    assert_semantics_equal(&expected, &capture(&trainer, &mut state));
    eprintln!("private-context PPO passed: nonempty immutable dendrites; changing0/1/2structural edges and dense offsets; coherent slice ALLlogit/value/activation/EMA/metabolic words exact; firstupdate{first_recurrent}/twoepoch{recurrent} recurrent weights+m/v changed; contextual/no-context PPO causal; epochs2/reuse2/freshlater,fullnormal/cache state exact");
}
