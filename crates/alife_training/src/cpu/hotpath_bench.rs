use super::*;
use crate::{
    AdamWConfig, CriticResidualRow, PpoConfig, PpoJointAction, PpoValueHeadCheckpoint,
    StageTrainableMask, TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick,
    TrainingSequence,
};
use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, CandidateActionFamily, DendriticBranch,
    DendriticBranchSet, DendriticInputRef, DevelopmentState, FoundationWeightAsset,
    NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
};

pub(super) fn fixture() -> (BrainPhenotype, FoundationWeightAsset) {
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

use crate::{PpoBatch, PpoBoundary, PpoTrainingWindow, PpoTransition};
use std::{hint::black_box, time::Instant};
fn timed(mut f: impl FnMut(), repetitions: usize) -> Vec<f64> {
    f();
    (0..repetitions)
        .map(|_| {
            let at = Instant::now();
            f();
            at.elapsed().as_secs_f64() * 1000.
        })
        .collect()
}
pub(super) fn window(
    t: &CpuFoundationTrainer,
    ticks: usize,
    burn: usize,
) -> CpuResidualTrainingWindow {
    let mut q = sequence(t.phenotype(), ticks, burn);
    for (k, tick) in q.ticks.iter_mut().enumerate() {
        let base = tick.candidates.clone();
        tick.candidates = (0..16)
            .map(|i| {
                let mut c = base[i % base.len()].clone();
                c.innate_bias += i as f32 * 0.003 + k as f32 * 0.0001;
                c
            })
            .collect();
        if k % 7 == 0 {
            tick.microstep_count = 1;
        }
    }
    let replay = t.evaluate_replay(&q).unwrap();
    let config = configuration();
    let mut rows = Vec::new();
    let transitions = (burn..ticks)
        .map(|i| {
            let mut motor_masks = [0u32; 6];
            for j in 0..16 {
                motor_masks[j % 6] |= 1 << j;
            }
            let mut a = PpoJointAction {
                candidate_count: 16,
                representative_mask: 65535,
                motor_masks,
                representative: 0,
                forced_slots: (0..16).map(|i| Some((i % 6) as u8)).collect(),
                motor_candidates: [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)],
                old_joint_log_probability: -1.,
                temperature: config.temperature,
            };
            a.old_joint_log_probability =
                cpu_ppo_objective(&replay.candidate_logits[i], &a, 0., config)
                    .unwrap()
                    .joint_log_probability;
            let final_row = i + 1 == ticks;
            let reward = if i % 2 == 0 { 0.17 } else { -0.11 };
            let discount = if final_row {
                0.
            } else {
                (-std::f32::consts::LN_2 * 0.05 / config.discount_half_life_seconds).exp()
            };
            rows.push(CriticResidualRow {
                current: (i - burn) as u32,
                next: (i - burn + 1) as u32,
                reward,
                discount,
            });
            PpoTransition {
                policy_version: 7400,
                trajectory_id: 1,
                step: (i - burn) as u64,
                action: a,
                reward,
                old_value: 0.,
                next_value: 0.,
                elapsed_seconds: 0.05,
                boundary: if final_row {
                    PpoBoundary::Terminated
                } else {
                    PpoBoundary::Continuing
                },
            }
        })
        .collect();
    let mut features = replay.final_activations[burn..].to_vec();
    features.push(features.last().unwrap().clone());
    CpuResidualTrainingWindow {
        actor: PpoTrainingWindow {
            speech_targets: vec![None; ticks],
            sequence: q,
            batch: PpoBatch::from_rollout(7400, transitions, config).unwrap(),
            auxiliary: None,
        },
        critic_features: features,
        critic_rows: rows,
    }
}
pub(super) fn configuration() -> PpoConfig {
    PpoConfig {
        epochs: 2,
        target_kl: 100.,
        temperature: 0.7,
        ..Default::default()
    }
}
pub(super) fn learner(p: &BrainPhenotype, s: &FoundationWeightAsset) -> CpuFoundationTrainer {
    let mut t = trainer(p, s);
    let mut ck = t.checkpoint();
    ck.optimizer_step = 16_777_220;
    ck.stage_mask = StageTrainableMask::from_synapse_indices(
        p,
        &(0..p.synapses().len() as u32)
            .filter(|i| i % 4 != 0)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    for i in 0..ck.weights.len() {
        ck.first_moment[i] = (i % 7) as f32 * 1e-7;
        ck.second_moment[i] = (i % 11) as f32 * 1e-9;
        ck.update_ages[i] = [321, 19_800, 16_777_217, 29][i % 4];
    }
    t.restore_checkpoint(ck).unwrap();
    t
}
pub(super) fn value(p: &BrainPhenotype) -> CpuValueHead {
    let width = p.neuron_count() as usize + 1;
    let mut parameters = vec![0.; width * 3];
    for i in 0..width {
        parameters[i] = (i % 7) as f32 * 0.0001;
        parameters[width + i] = (i % 5) as f32 * 1e-6;
        parameters[width * 2 + i] = (i % 11) as f32 * 1e-8;
    }
    CpuValueHead::from_checkpoint(PpoValueHeadCheckpoint {
        feature_count: p.neuron_count(),
        optimizer_step: 19_800,
        parameters,
        last_updated_policy_version: Some(7399),
    })
    .unwrap()
}
#[test]
#[ignore = "synthetic production-code CPU benchmark; explicitly run in cloud"]
fn synthetic_cpu_hotpath_ledger() {
    let (p, s) = fixture();
    let mut t = learner(&p, &s);
    let v = value(&p);
    let config = configuration();
    let w = window(&t, 40, 8);
    let q = &w.actor.sequence;
    let tape = t.forward(q).unwrap();
    let (adjoints, _, _, _) = objective::evaluate_window(&t, &w.actor, config, 7400).unwrap();
    let mut full = vec![vec![0.; 16]; 8];
    full.extend_from_slice(&adjoints);
    let probe = t
        .graph
        .backward(&p, t.weights(), q, &tape, &full, &w.actor.speech_targets)
        .unwrap();
    let mut ledger = serde_json::Map::new();
    ledger.insert(
        "frozen_residual_preflight_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(cohort::validate_frozen_residual_window(
                    &t, &v, &w, config, 7400,
                ))
                .unwrap();
            },
            11
        )),
    );
    ledger.insert(
        "sequence_validation_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(q.validate_for(&p)).unwrap();
            },
            21
        )),
    );
    ledger.insert(
        "forward_graph_alloc_drop_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(t.graph.forward(&p, t.weights(), q).unwrap());
            },
            11
        )),
    );
    let tape_drop_only_ms = (0..11)
        .map(|_| {
            let tape = t.forward(q).unwrap();
            let started = Instant::now();
            drop(tape);
            started.elapsed().as_secs_f64() * 1000.
        })
        .collect::<Vec<_>>();
    ledger.insert(
        "tape_drop_only_ms".into(),
        serde_json::json!(tape_drop_only_ms),
    );
    ledger.insert(
        "backward_existing_tape_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(
                    t.graph
                        .backward(&p, t.weights(), q, &tape, &full, &w.actor.speech_targets)
                        .unwrap(),
                );
            },
            11
        )),
    );
    ledger.insert(
        "joint_ppo_32_rows_ms".into(),
        serde_json::json!(timed(
            || {
                for (i, tr) in w.actor.batch.transitions().iter().enumerate() {
                    black_box(
                        cpu_ppo_objective(
                            &tape.evaluation.candidate_logits[i + 8],
                            &tr.action,
                            w.actor.batch.actor_advantage(i).unwrap(),
                            config,
                        )
                        .unwrap(),
                    );
                }
            },
            21
        )),
    );
    ledger.insert(
        "evaluate_window_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(objective::evaluate_window(&t, &w.actor, config, 7400).unwrap());
            },
            11
        )),
    );
    ledger.insert(
        "public_probe_recompute_forward_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(
                    t.probe_replay_gradients_with_speech(q, &adjoints, &w.actor.speech_targets)
                        .unwrap(),
                );
            },
            11
        )),
    );
    ledger.insert(
        "residual_critic_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(
                    v.residual_gradients(
                        &w.critic_features,
                        &w.critic_rows,
                        config.value_coefficient,
                    )
                    .unwrap(),
                );
            },
            21
        )),
    );
    ledger.insert(
        "window_owned_clone_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(w.clone());
            },
            21
        )),
    );
    ledger.insert(
        "actor_full_adam_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(t.apply_gradients(&probe.gradients).unwrap());
            },
            21
        )),
    );
    ledger.insert(
        "critic_adam_proposal_ms".into(),
        serde_json::json!(timed(
            || {
                black_box(
                    v.proposal(
                        &vec![0.00001; p.neuron_count() as usize + 1],
                        config.value_optimizer,
                        7400,
                    )
                    .unwrap(),
                );
            },
            21
        )),
    );
    let mut checkpoints = Vec::new();
    for residual in [false, true] {
        let mut times = Vec::new();
        for run in 0..8 {
            let mut actor = learner(&p, &s);
            let mut critic = value(&p);
            let windows = [window(&actor, 40, 8), window(&actor, 40, 8)];
            let at = Instant::now();
            let receipt = if residual {
                train_cpu_residual_cohort(
                    &mut actor,
                    &mut critic,
                    2,
                    2,
                    |i| Ok(windows[i].clone()),
                    config,
                    7400,
                )
                .unwrap()
            } else {
                train_cpu_ppo_cohort(
                    &mut actor,
                    &mut critic,
                    2,
                    2,
                    |i| Ok(windows[i].actor.clone()),
                    config,
                    7400,
                )
                .unwrap()
            };
            if run > 0 {
                times.push(at.elapsed().as_secs_f64() * 1000.);
            }
            if run == 1 {
                checkpoints.push(serde_json::json!({"residual":residual,"actor":actor.checkpoint(),"critic":critic.checkpoint(),"receipt":receipt}));
            }
        }
        ledger.insert(
            if residual {
                "residual_cohort_64_rows_2_epochs_ms"
            } else {
                "ppo_cohort_64_rows_2_epochs_ms"
            }
            .into(),
            serde_json::json!(times),
        );
    }
    let tag = std::env::var("ALIFE_BENCH_TAG").unwrap_or_else(|_| "candidate".into());
    let revision = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".into());
    let result = serde_json::json!({"tag":tag,"source":revision.trim(),"neurons":p.neuron_count(),"synapses":p.synapses().len(),"routes":p.projections().len(),"microsteps":p.microstep_count(),"trainable":learner(&p,&s).checkpoint().stage_mask.trainable_count(),"conditions":"synthetic CPU numerical workload; caller must record compiler/profile; 40 ticks, 8 detached burn-in, 16 candidates, 6 motor factors; no speech targets; one dendritic branch; inactive microsteps every seventh tick; not simulation or learning competence","ledger":ledger});
    if let Some(directory) = std::env::var_os("ALIFE_BENCH_OUTPUT_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(format!("{tag}.json")),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join(format!("{tag}-checkpoints.json")),
            serde_json::to_vec(&checkpoints).unwrap(),
        )
        .unwrap();
    }
    eprintln!("HOTPATH {}", result);
}
