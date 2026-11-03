//! CPU-only experiment: skip edges that the unchanged cadence predicate rejects.
//! No weight-based pruning, route-mask caching, topology mutation or GPU claims.

use super::*;
use crate::cpu::{hotpath_bench, CpuFoundationTrainer};
use crate::{AdamWConfig, StageTrainableMask, TrainingFrozenSynapse};
use std::{hint::black_box, time::Instant};

pub(super) struct CadenceRows {
    offsets: [Vec<usize>; 3],
    ids: [Vec<usize>; 3],
}

fn class(step: usize) -> usize {
    if step == 0 {
        0
    } else if step.is_multiple_of(2) {
        2
    } else {
        1
    }
}

impl CadenceRows {
    fn new(incoming: &[Vec<usize>], cadence: impl Fn(usize) -> u8) -> Self {
        let mut offsets: [Vec<usize>; 3] = Default::default();
        let mut ids: [Vec<usize>; 3] = Default::default();
        for step in 0..3 {
            offsets[step].push(0);
            for row in incoming {
                ids[step].extend(
                    row.iter()
                        .copied()
                        .filter(|&id| route_fires(cadence(id), step)),
                );
                offsets[step].push(ids[step].len());
            }
        }
        Self { offsets, ids }
    }
    fn row(&self, target: usize, step: usize) -> &[usize] {
        let c = class(step);
        &self.ids[c][self.offsets[c][target]..self.offsets[c][target + 1]]
    }
    fn payload_bytes(&self) -> usize {
        self.offsets
            .iter()
            .chain(&self.ids)
            .map(|v| v.len() * size_of::<usize>())
            .sum()
    }
}

impl SparseGraph {
    fn enable_cadence_assay(&mut self, p: &BrainPhenotype) {
        self.cadence_rows = Some(CadenceRows::new(&self.incoming, |id| {
            p.projections()[p.synapses()[id].route_index() as usize]
                .update_cadence()
                .raw()
        }));
    }
    pub(super) fn assay_incoming(&self, target: usize, step: usize) -> &[usize] {
        self.cadence_rows
            .as_ref()
            .map_or(&self.incoming[target], |p| p.row(target, step))
    }
}

fn learner(p: &BrainPhenotype, s: &alife_core::FoundationWeightAsset) -> CpuFoundationTrainer {
    let ids = (0..p.synapses().len() as u32)
        .filter(|i| i % 11 != 0)
        .collect::<Vec<_>>();
    let mut t = CpuFoundationTrainer::new(
        p.clone(),
        s.clone(),
        StageTrainableMask::from_synapse_indices(p, &ids).unwrap(),
        AdamWConfig::default(),
    )
    .unwrap();
    let mut c = t.checkpoint();
    c.optimizer_step = 16_777_220;
    for i in 0..c.weights.len() {
        c.first_moment[i] = (i % 7) as f32 * 1e-7;
        c.second_moment[i] = 1e-8 + (i % 5) as f32 * 1e-8;
        c.update_ages[i] = [321, 19_800, 16_777_217, 29][i % 4];
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
    let mut q = hotpath_bench::window(t, ticks, burn).actor.sequence;
    for (i, tick) in q.ticks.iter_mut().enumerate() {
        if variant & 1 != 0 {
            for (route, flag) in tick.enabled_routes.iter_mut().enumerate() {
                *flag = (route + i) % 3 != 0;
            }
        }
        if variant & 2 != 0 {
            tick.microstep_count = (i % (t.phenotype().microstep_count() as usize + 1)) as u32;
        }
        if variant & 4 != 0 {
            tick.structural_synapses.push(TrainingFrozenSynapse {
                source: 6,
                target: 35,
                route: 0,
                cadence: t.phenotype().projections()[0].update_cadence().raw() as u32,
                effective_weight: -0.17,
            });
            // An exactly cancelled effective weight remains an anatomical edge.
            tick.effective_weight_offsets[0] = -t.weights()[0];
        }
    }
    q.validate_for(t.phenotype()).unwrap();
    q
}

fn adjoints(q: &TrainingSequence) -> Vec<Vec<f32>> {
    q.ticks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            t.candidates
                .iter()
                .enumerate()
                .map(|(j, _)| {
                    if i < q.burn_in_ticks {
                        0.
                    } else {
                        ((i + j) % 7) as f32 * 0.03 - 0.07
                    }
                })
                .collect()
        })
        .collect()
}

fn tape_bits(t: &Tape) -> Vec<Vec<u32>> {
    [
        &t.a,
        &t.m,
        &t.activated,
        &t.memory,
        &t.cognitive,
        &t.evaluation.candidate_logits,
        &t.evaluation.final_activations,
        &t.evaluation.final_activity_ema,
        &t.evaluation.final_metabolic_load,
    ]
    .into_iter()
    .flat_map(|rows| {
        rows.iter()
            .map(|row| row.iter().map(|v| v.to_bits()).collect())
    })
    .collect()
}

fn sweep(
    g: &SparseGraph,
    p: &BrainPhenotype,
    weights: &[f32],
    q: &TrainingSequence,
    a: &[Vec<f32>],
) -> (Tape, TrainingGradientProbe) {
    let tape = g.forward(p, weights, q).unwrap();
    let gradient = g
        .backward(p, weights, q, &tape, a, &vec![None; q.ticks.len()])
        .unwrap();
    (tape, gradient)
}

#[test]
fn ordered_cadence_rows_match_all_raw_cadences_and_route_masks() {
    let incoming = vec![vec![], (0..8).collect(), vec![7, 3, 0, 6, 1]];
    let raw = [0, 1, 2, 3, 4, 5, 6, 255];
    let plan = CadenceRows::new(&incoming, |id| raw[id]);
    for mask in 0u16..256 {
        for step in 0..8 {
            for (target, row) in incoming.iter().enumerate() {
                let expected = row
                    .iter()
                    .copied()
                    .filter(|&id| mask & (1 << id) != 0 && route_fires(raw[id], step))
                    .collect::<Vec<_>>();
                let actual = plan
                    .row(target, step)
                    .iter()
                    .copied()
                    .filter(|&id| mask & (1 << id) != 0)
                    .collect::<Vec<_>>();
                assert_eq!(expected, actual);
            }
        }
    }
}

#[test]
fn complete_tapes_gradients_and_adam_remain_bit_exact() {
    let (p, s) = hotpath_bench::fixture();
    for variant in 0..8 {
        let mut baseline = learner(&p, &s);
        let mut candidate = learner(&p, &s);
        candidate.graph.enable_cadence_assay(&p);
        let q = sequence(&baseline, 6, variant % 3, variant);
        let baseline_work = work(&baseline.graph, &p, &q);
        let candidate_work = work(&candidate.graph, &p, &q);
        assert_eq!(baseline_work.1, candidate_work.1);
        assert!(candidate_work.0 <= baseline_work.0);
        if variant == 0 {
            assert!(candidate_work.0 < baseline_work.0);
        }
        assert_eq!(baseline.graph.incoming, candidate.graph.incoming);
        assert!(candidate.graph.cadence_rows.as_ref().unwrap().ids[0].contains(&0));
        let a = adjoints(&q);
        let b = sweep(&baseline.graph, &p, baseline.weights(), &q, &a);
        let c = sweep(&candidate.graph, &p, candidate.weights(), &q, &a);
        assert_eq!(tape_bits(&b.0), tape_bits(&c.0), "tape variant {variant}");
        assert_eq!(b.1.objective.to_bits(), c.1.objective.to_bits());
        assert_eq!(
            b.1.gradients
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            c.1.gradients
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>()
        );
        baseline.apply_gradients(&b.1.gradients).unwrap();
        candidate.apply_gradients(&c.1.gradients).unwrap();
        assert_eq!(
            serde_json::to_vec(&baseline.checkpoint()).unwrap(),
            serde_json::to_vec(&candidate.checkpoint()).unwrap()
        );
    }
    let t = learner(&p, &s);
    let q = sequence(&t, 3, 0, 0);
    let mut corrupted = SparseGraph::new(&p);
    corrupted.enable_cadence_assay(&p);
    // Falsifier: deleting an active contribution must break complete-state equality.
    let plan = corrupted.cadence_rows.as_mut().unwrap();
    let index = plan.ids[0]
        .iter()
        .position(|&id| t.weights()[id] != 0.)
        .unwrap();
    plan.ids[0].remove(index);
    for offset in &mut plan.offsets[0] {
        if *offset > index {
            *offset -= 1;
        }
    }
    assert_ne!(
        tape_bits(&t.graph.forward(&p, t.weights(), &q).unwrap()),
        tape_bits(&corrupted.forward(&p, t.weights(), &q).unwrap())
    );
}

fn work(g: &SparseGraph, p: &BrainPhenotype, q: &TrainingSequence) -> (usize, usize) {
    let mut visits = 0;
    let mut contributions = 0;
    for (tick_index, tick) in q.ticks.iter().enumerate() {
        for step in 0..tick.microstep_count as usize {
            let multiplicity = 1 + usize::from(tick_index >= q.burn_in_ticks);
            for target in 0..p.neuron_count() as usize {
                let row = g.assay_incoming(target, step);
                visits += row.len() * multiplicity;
                contributions += row
                    .iter()
                    .filter(|&&id| {
                        let route = p.synapses()[id].route_index() as usize;
                        tick.enabled_routes[route]
                            && route_fires(p.projections()[route].update_cadence().raw(), step)
                    })
                    .count()
                    * multiplicity;
            }
        }
    }
    (visits, contributions)
}

#[test]
#[ignore = "bounded CPU-only full replay timing; no adapter or private trajectory"]
fn paired_full_replay_measurement() {
    let (p, s) = hotpath_bench::fixture();
    let t = learner(&p, &s);
    let baseline = SparseGraph::new(&p);
    let mut candidate = SparseGraph::new(&p);
    let at = Instant::now();
    candidate.enable_cadence_assay(&p);
    let build_ms = at.elapsed().as_secs_f64() * 1000.;
    let mut results = Vec::new();
    for (ticks, burn) in [(40, 8), (256, 0), (384, 128)] {
        let q = sequence(&t, ticks, burn, 7);
        let a = adjoints(&q);
        let bwork = work(&baseline, &p, &q);
        let cwork = work(&candidate, &p, &q);
        assert_eq!(bwork.1, cwork.1);
        assert!(cwork.0 < bwork.0);
        let mut trials = Vec::new();
        for pair in 0..7 {
            let mut ms = [0.; 2];
            let mut observed: [Option<(Tape, TrainingGradientProbe)>; 2] = [None, None];
            for which in if pair % 2 == 0 { [0, 1] } else { [1, 0] } {
                let at = Instant::now();
                let output = sweep(
                    black_box(if which == 0 { &baseline } else { &candidate }),
                    &p,
                    black_box(t.weights()),
                    black_box(&q),
                    black_box(&a),
                );
                // Retain results outside the timer for complete bit comparisons.
                black_box(&output);
                ms[which] = at.elapsed().as_secs_f64() * 1000.;
                observed[which] = Some(output);
            }
            let b = observed[0].take().unwrap();
            let c = observed[1].take().unwrap();
            assert_eq!(tape_bits(&b.0), tape_bits(&c.0));
            assert_eq!(b.1.objective.to_bits(), c.1.objective.to_bits());
            assert_eq!(
                b.1.gradients
                    .iter()
                    .map(|v| v.to_bits())
                    .collect::<Vec<_>>(),
                c.1.gradients
                    .iter()
                    .map(|v| v.to_bits())
                    .collect::<Vec<_>>()
            );
            trials.push(serde_json::json!({"pair":pair,"first":if pair%2==0 {"baseline"} else {"candidate"},"baseline_ms":ms[0],"candidate_ms":ms[1]}));
        }
        results.push(serde_json::json!({"ticks":ticks,"burn_in":burn,"baseline_visits":bwork.0,"candidate_visits":cwork.0,"identical_contributions":bwork.1,"trials":trials}));
    }
    let mut degrees = std::collections::BTreeMap::<usize, usize>::new();
    for row in &baseline.incoming {
        *degrees.entry(row.len()).or_default() += 1;
    }
    let self_edges = p
        .synapses()
        .iter()
        .filter(|s| matches!(s.kind(), CompiledSynapseKind::Recurrent) && s.source() == s.target())
        .count();
    let result = serde_json::json!({"scope":"CPU synthetic full forward+BPTT; no PPO objective/Adam/world/GPU; output destruction outside timer; first result retained during second span in alternating order","preparation":"untimed fixture validation and forward; first timed pair retained, not a cold-process assay; plan built once","microsteps":p.microstep_count(),"neurons":p.neuron_count(),"synapses":p.synapses().len(),"recurrent_degree_histogram":degrees,"explicit_recurrent_self_edges":self_edges,"plan_build_ms":build_ms,"additional_index_payload_bytes":candidate.cadence_rows.as_ref().unwrap().payload_bytes(),"trials":results});
    eprintln!("cadence_assay={result}");
    if let Some(path) = std::env::var_os("ALIFE_CADENCE_ASSAY_DIR") {
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            std::path::Path::new(&path).join("trials.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }
}
