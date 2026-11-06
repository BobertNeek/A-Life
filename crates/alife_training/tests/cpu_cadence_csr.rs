#![cfg(feature = "cpu-backend")]

use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, CandidateActionFamily, DendriticBranch,
    DendriticBranchSet, DendriticInputRef, DevelopmentState, FoundationWeightAsset,
    NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick, TrainingStageManifest,
};
use alife_training::cpu::{CpuFoundationTrainer, CpuReplayExecution};
use alife_training::{
    AdamWConfig, StageTrainableMask, TrainingFrozenSynapse, TrainingInitialState,
    TrainingReplayCandidate, TrainingReplayEvaluation, TrainingReplayTick, TrainingSequence,
};

fn fixture() -> (
    BrainPhenotype,
    FoundationWeightAsset,
    BrainGenome,
    DevelopmentState,
) {
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
    (p, source, genome, development)
}

fn learner(p: &BrainPhenotype, s: &FoundationWeightAsset) -> CpuFoundationTrainer {
    let mask = StageTrainableMask::from_synapse_indices(
        p,
        &(0..p.synapses().len() as u32)
            .filter(|i| i % 11 != 0)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut t =
        CpuFoundationTrainer::new(p.clone(), s.clone(), mask, AdamWConfig::default()).unwrap();
    let mut c = t.checkpoint();
    c.optimizer_step = 16_777_220;
    for i in 0..c.weights.len() {
        c.first_moment[i] = (i % 7) as f32 * 1e-7;
        c.second_moment[i] = 1e-8 + (i % 5) as f32 * 1e-8;
        c.update_ages[i] = [321, 19800, 16_777_217, 29][i % 4];
    }
    t.restore_checkpoint(c).unwrap();
    t
}

fn sequence(
    t: &CpuFoundationTrainer,
    ticks: usize,
    burn: usize,
    variant: usize,
) -> TrainingSequence {
    let p = t.phenotype();
    let n = p.neuron_count() as usize;
    let families = [
        CandidateActionFamily::Approach,
        CandidateActionFamily::Ingest,
        CandidateActionFamily::Idle,
    ];
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
    let frames = (0..ticks)
        .map(|i| {
            let mut offsets = (0..p.synapses().len())
                .map(|j| (j % 7) as f32 * 0.002)
                .collect::<Vec<_>>();
            offsets[0] = -t.weights()[0];
            TrainingReplayTick {
                encoded_inputs: (0..n).map(|j| (j % 11) as f32 * 0.015).collect(),
                projection_gain: 0.85,
                local_threshold_shift: 0.017,
                microstep_count: if variant & 2 != 0 {
                    (i % (p.microstep_count() as usize + 1)) as u32
                } else {
                    u32::from(p.microstep_count())
                },
                enabled_routes: (0..p.projections().len())
                    .map(|route| variant & 1 == 0 || (route + i) % 3 != 0)
                    .collect(),
                effective_weight_offsets: offsets,
                structural_synapses: vec![TrainingFrozenSynapse {
                    source: 6,
                    target: 35,
                    route: 0,
                    cadence: u32::from(p.projections()[0].update_cadence().raw()),
                    effective_weight: -0.17,
                }],
                candidates: (0..16)
                    .map(|j| TrainingReplayCandidate {
                        family: families[j % 3],
                        decoder_inputs: [0.03; 54],
                        innate_bias: 0.04 + j as f32 * 0.003,
                    })
                    .collect(),
            }
        })
        .collect();
    let q = TrainingSequence {
        phenotype_hash: p.phenotype_hash(),
        initial,
        ticks: frames,
        burn_in_ticks: burn,
        memory_candidate_gain: p
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    };
    q.validate_for(p).unwrap();
    q
}
fn adjoints(q: &TrainingSequence) -> Vec<Vec<f32>> {
    q.ticks[q.burn_in_ticks..]
        .iter()
        .enumerate()
        .map(|(i, t)| {
            t.candidates
                .iter()
                .enumerate()
                .map(|(j, _)| ((i + j) % 7) as f32 * 0.03 - 0.07)
                .collect()
        })
        .collect()
}
fn bits(e: &TrainingReplayEvaluation) -> Vec<Vec<u32>> {
    [
        &e.candidate_logits,
        &e.final_activations,
        &e.final_activity_ema,
        &e.final_metabolic_load,
    ]
    .into_iter()
    .flat_map(|rows| rows.iter().map(|r| r.iter().map(|v| v.to_bits()).collect()))
    .collect()
}
fn checkpoint_bytes(t: &CpuFoundationTrainer) -> Vec<u8> {
    serde_json::to_vec(&t.checkpoint()).unwrap()
}

#[test]
fn public_opt_in_fallback_restore_and_adam_are_exact() {
    let (p, s, _, _) = fixture();
    for variant in 0..4 {
        let mut b = learner(&p, &s);
        let mut c = learner(&p, &s);
        assert_eq!(c.replay_execution(), CpuReplayExecution::Reference);
        let initial = checkpoint_bytes(&c);
        c.set_replay_execution(CpuReplayExecution::CadenceCsr);
        assert_eq!(c.replay_execution(), CpuReplayExecution::CadenceCsr);
        assert_eq!(initial, checkpoint_bytes(&c));
        let q = sequence(&b, 6, variant % 3, variant);
        let a = adjoints(&q);
        assert_eq!(
            bits(&b.evaluate_replay(&q).unwrap()),
            bits(&c.evaluate_replay(&q).unwrap())
        );
        let bg = b.probe_replay_gradients(&q, &a).unwrap();
        let cg = c.probe_replay_gradients(&q, &a).unwrap();
        assert_eq!(bg.objective.to_bits(), cg.objective.to_bits());
        assert_eq!(
            bg.gradients.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            cg.gradients.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        b.apply_gradients(&bg.gradients).unwrap();
        c.apply_gradients(&cg.gradients).unwrap();
        assert_eq!(checkpoint_bytes(&b), checkpoint_bytes(&c));
        let saved = c.checkpoint();
        c.set_replay_execution(CpuReplayExecution::Reference);
        assert_eq!(checkpoint_bytes(&b), checkpoint_bytes(&c));
        assert_eq!(
            bits(&b.evaluate_replay(&q).unwrap()),
            bits(&c.evaluate_replay(&q).unwrap())
        );
        let mut restored =
            CpuFoundationTrainer::from_checkpoint(p.clone(), s.clone(), saved.clone()).unwrap();
        assert_eq!(restored.replay_execution(), CpuReplayExecution::Reference);
        restored.set_replay_execution(CpuReplayExecution::CadenceCsr);
        restored.restore_checkpoint(saved).unwrap();
        assert_eq!(restored.replay_execution(), CpuReplayExecution::CadenceCsr);
        assert_eq!(checkpoint_bytes(&b), checkpoint_bytes(&restored));
    }
}

#[test]
fn genetic_rebind_rebuilds_selected_indices_and_preserves_adam() {
    let (p, s, g, d) = fixture();
    let mut c = learner(&p, &s);
    c.set_replay_execution(CpuReplayExecution::CadenceCsr);
    let mut gradients = vec![0.; p.synapses().len()];
    gradients[17] = 0.2;
    c.apply_gradients(&gradients).unwrap();
    let before = c.checkpoint();
    let asset = c
        .export_candidate(TrainingStageManifest::new(1, 1, 1))
        .unwrap();
    let (next, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(g, d, asset.clone()).unwrap();
    c.rebind_for_next_cohort(next.clone(), asset.clone())
        .unwrap();
    assert_eq!(c.replay_execution(), CpuReplayExecution::CadenceCsr);
    let after = c.checkpoint();
    assert_eq!(before.weights, after.weights);
    assert_eq!(before.first_moment, after.first_moment);
    assert_eq!(before.second_moment, after.second_moment);
    assert_eq!(before.update_ages, after.update_ages);
    assert_eq!(before.optimizer_step, after.optimizer_step);
    let reference = CpuFoundationTrainer::from_checkpoint(next, asset, after).unwrap();
    let q = sequence(&c, 5, 1, 3);
    assert_eq!(
        bits(&reference.evaluate_replay(&q).unwrap()),
        bits(&c.evaluate_replay(&q).unwrap())
    );
    let a = adjoints(&q);
    assert_eq!(
        reference.probe_replay_gradients(&q, &a).unwrap(),
        c.probe_replay_gradients(&q, &a).unwrap()
    );
}

#[test]
#[ignore = "bounded actual non-libtest CPU trainer API timing on disposable replay"]
fn paired_public_probe_measurement() {
    use std::{hint::black_box, time::Instant};
    let (p, s, _, _) = fixture();
    let b = learner(&p, &s);
    let mut c = learner(&p, &s);
    let at = Instant::now();
    c.set_replay_execution(CpuReplayExecution::CadenceCsr);
    let build_ms = at.elapsed().as_secs_f64() * 1000.;
    let mut profiles = Vec::new();
    for (ticks, burn) in [(256, 0), (384, 128)] {
        let q = sequence(&b, ticks, burn, 3);
        let a = adjoints(&q);
        assert_eq!(
            bits(&b.evaluate_replay(&q).unwrap()),
            bits(&c.evaluate_replay(&q).unwrap())
        );
        let before = checkpoint_bytes(&b);
        let mut trials = Vec::new();
        for pair in 0..7 {
            let mut ms = [0.; 2];
            let mut outputs = [None, None];
            for which in if pair % 2 == 0 { [0, 1] } else { [1, 0] } {
                let t = black_box(if which == 0 { &b } else { &c });
                let at = Instant::now();
                let output = t
                    .probe_replay_gradients(black_box(&q), black_box(&a))
                    .unwrap();
                black_box(&output);
                ms[which] = at.elapsed().as_secs_f64() * 1000.;
                outputs[which] = Some(output);
            }
            let bg = outputs[0].take().unwrap();
            let cg = outputs[1].take().unwrap();
            assert_eq!(bg.objective.to_bits(), cg.objective.to_bits());
            assert_eq!(
                bg.gradients.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                cg.gradients.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
            trials.push(serde_json::json!({"pair":pair,"first":if pair%2==0 {"reference"} else {"cadence"},"reference_ms":ms[0],"cadence_ms":ms[1]}));
        }
        assert_eq!(before, checkpoint_bytes(&b));
        assert_eq!(before, checkpoint_bytes(&c));
        profiles.push(serde_json::json!({"ticks":ticks,"burn_in":burn,"trials":trials}));
    }
    let result = serde_json::json!({"scope":"actual public CpuFoundationTrainer probe: validation, forward, BPTT and tape destruction; returned gradients destroyed outside timer; no Adam/PPO/world/GPU; warm preparation; first gradients retained during second call; alternating order","plan_build_ms":build_ms,"neurons":p.neuron_count(),"synapses":p.synapses().len(),"microsteps":p.microstep_count(),"profiles":profiles});
    eprintln!("cpu_cadence_public_probe={result}");
    if let Some(path) = std::env::var_os("ALIFE_CPU_CADENCE_RESULT_DIR") {
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            std::path::Path::new(&path).join("trials.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }
}
