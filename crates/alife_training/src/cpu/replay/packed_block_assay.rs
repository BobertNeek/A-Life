//! Disposable FP32 layout oracle. Test-only: no execution flag or save field.
//! CUDA/Tensor Core kernels are separate component experiments, not this code.
use super::*;
use crate::cpu::{hotpath_bench, CpuFoundationTrainer};
use std::{collections::BTreeMap, hint::black_box, time::Instant};

#[derive(serde::Serialize)]
struct Block {
    source_start: usize,
    target_start: usize,
    route: usize,
    cadence: u8,
    // Target-major/source-minor slots contain original global edge identities.
    ids: Vec<usize>,
}
pub(super) struct PackedBlocks {
    graph: SparseGraph,
    blocks: Vec<Block>,
    ids: Vec<usize>,
    // Canonical global-ID order within each target; never sum tile partials here.
    rows: Vec<Vec<usize>>,
}
impl PackedBlocks {
    fn new(p: &BrainPhenotype) -> Self {
        let graph = SparseGraph::new(p);
        let mut groups = BTreeMap::<(usize, usize, usize), Vec<(usize, usize)>>::new();
        for (id, s) in p.synapses().iter().enumerate() {
            if s.kind() == CompiledSynapseKind::Recurrent {
                groups
                    .entry((
                        s.route_index() as usize,
                        s.source() as usize / 16,
                        s.target() as usize / 16,
                    ))
                    .or_default()
                    .push((
                        id,
                        (s.target() as usize % 16) * 16 + s.source() as usize % 16,
                    ));
            }
        }
        let mut blocks = Vec::new();
        for ((route, source, target), entries) in groups {
            assert_eq!(
                entries.len(),
                256,
                "partial tiles are unsupported, never padded silently"
            );
            let mut ids = vec![usize::MAX; 256];
            for (id, slot) in entries {
                assert_eq!(ids[slot], usize::MAX);
                ids[slot] = id;
            }
            assert!(!ids.contains(&usize::MAX));
            blocks.push(Block {
                source_start: source * 16,
                target_start: target * 16,
                route,
                cadence: p.projections()[route].update_cadence().raw(),
                ids,
            });
        }
        let ids = blocks
            .iter()
            .flat_map(|b| b.ids.iter().copied())
            .collect::<Vec<_>>();
        let mut reverse = vec![usize::MAX; p.synapses().len()];
        for (packed, &id) in ids.iter().enumerate() {
            assert_eq!(reverse[id], usize::MAX);
            reverse[id] = packed;
        }
        let rows = graph
            .incoming
            .iter()
            .map(|r| {
                r.iter()
                    .map(|&id| {
                        assert_ne!(reverse[id], usize::MAX);
                        reverse[id]
                    })
                    .collect()
            })
            .collect();
        Self {
            graph,
            blocks,
            ids,
            rows,
        }
    }
    pub(super) fn pack(&self, canonical: &[f32]) -> Vec<f32> {
        self.ids.iter().map(|&id| canonical[id]).collect()
    }
    fn unpack(&self, packed: &[f32], original: &[f32]) -> Vec<f32> {
        assert_eq!(packed.len(), self.ids.len());
        let mut result = original.to_vec();
        for (&id, &v) in self.ids.iter().zip(packed) {
            result[id] = v;
        }
        result
    }
    fn effective(&self, genetic: &[f32], offsets: &[f32]) -> Vec<f32> {
        genetic
            .iter()
            .zip(&self.ids)
            .map(|(&g, &id)| g + offsets[id])
            .collect()
    }
    fn dot(
        &self,
        target: usize,
        step: usize,
        tick: &crate::TrainingReplayTick,
        previous: &[f32],
        effective: &[f32],
    ) -> f32 {
        let mut sum = 0.;
        for &index in &self.rows[target] {
            let block = &self.blocks[index / 256];
            if tick.enabled_routes[block.route] && route_fires(block.cadence, step) {
                sum += previous[block.source_start + index % 16] * effective[index];
            }
        }
        sum
    }
    fn forward_batch(
        &self,
        p: &BrainPhenotype,
        weights: &[f32],
        windows: &[TrainingSequence],
    ) -> Vec<Tape> {
        // Independent windows; serial CPU oracle. No cross-tick or cross-window state.
        let genetic = self.pack(weights);
        windows
            .iter()
            .map(|q| {
                q.validate_for(p).unwrap();
                self.forward_one(p, q, &genetic, weights).unwrap()
            })
            .collect()
    }
}

fn bits(t: &Tape) -> Vec<u32> {
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
    .flat_map(|rows| rows.iter().flatten().map(|x| x.to_bits()))
    .collect()
}

fn sequences(t: &CpuFoundationTrainer, count: usize) -> Vec<TrainingSequence> {
    (0..count)
        .map(|variant| {
            let mut q = hotpath_bench::window(t, 8 + variant % 3, variant % 3)
                .actor
                .sequence;
            for (i, tick) in q.ticks.iter_mut().enumerate() {
                tick.microstep_count = ((i + variant) % 5) as u32;
                for (route, enabled) in tick.enabled_routes.iter_mut().enumerate() {
                    *enabled = (route + i + variant) % 4 != 0;
                }
                for (id, offset) in tick.effective_weight_offsets.iter_mut().enumerate() {
                    *offset += ((id + i + variant) % 5) as f32 * 0.003;
                }
                tick.structural_synapses.push(crate::TrainingFrozenSynapse {
                    source: 6,
                    target: 35,
                    route: 0,
                    cadence: t.phenotype().projections()[0].update_cadence().raw() as u32,
                    effective_weight: -0.17,
                });
            }
            q.validate_for(t.phenotype()).unwrap();
            q
        })
        .collect()
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

#[test]
fn exact_layout_full_recurrence_gradients_and_adam_continuity() {
    let (p, source) = hotpath_bench::fixture();
    let plan = PackedBlocks::new(&p);
    assert_eq!(plan.blocks.len(), 96);
    assert_eq!(plan.ids.len(), 24_576);
    let mut native = hotpath_bench::learner(&p, &source);
    let mut packed = hotpath_bench::learner(&p, &source);
    let original = native.checkpoint();
    assert_eq!(
        plan.unpack(&plan.pack(&original.weights), &original.weights),
        original.weights
    );
    assert_eq!(
        plan.unpack(&plan.pack(&original.first_moment), &original.first_moment),
        original.first_moment
    );
    assert_eq!(
        plan.unpack(&plan.pack(&original.second_moment), &original.second_moment),
        original.second_moment
    );
    // Integer ages never enter a float packing path (including >2^24).
    let mut ages = original.update_ages.clone();
    for &id in &plan.ids {
        ages[id] = original.update_ages[id];
    }
    assert_eq!(ages, original.update_ages);
    for _ in 0..2 {
        let qs = sequences(&native, 4);
        let tapes = plan.forward_batch(&p, packed.weights(), &qs);
        let mut total_a = vec![0.; native.weights().len()];
        let mut total_b = total_a.clone();
        for (q, b) in qs.iter().zip(&tapes) {
            let a = native.forward(q).unwrap();
            assert_eq!(bits(&a), bits(b));
            let targets = vec![None; q.ticks.len()];
            let adj = adjoints(q);
            let ga = plan
                .graph
                .backward(&p, native.weights(), q, &a, &adj, &targets)
                .unwrap();
            let gb = plan
                .graph
                .backward(&p, packed.weights(), q, b, &adj, &targets)
                .unwrap();
            assert_eq!(ga.objective.to_bits(), gb.objective.to_bits());
            assert_eq!(
                ga.gradients.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                gb.gradients.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            );
            assert!(ga
                .gradients
                .iter()
                .zip(p.synapses())
                .any(|(g, s)| s.kind() == CompiledSynapseKind::Recurrent && *g != 0.));
            for i in 0..total_a.len() {
                total_a[i] += ga.gradients[i];
                total_b[i] += gb.gradients[i];
            }
        }
        native.apply_gradients(&total_a).unwrap();
        packed.apply_gradients(&total_b).unwrap();
        assert_eq!(
            serde_json::to_vec(&native.checkpoint()).unwrap(),
            serde_json::to_vec(&packed.checkpoint()).unwrap()
        );
        let saved = packed.checkpoint();
        packed.restore_checkpoint(saved.clone()).unwrap();
        assert_eq!(saved, packed.checkpoint());
    }
    // Falsifier: a wrong lane map must visibly alter the full recurrent tape.
    let q = sequences(&native, 1);
    let mut bad = plan.pack(native.weights());
    bad[0] += 0.5;
    assert_ne!(
        bits(&native.forward(&q[0]).unwrap()),
        bits(&plan.forward_one(&p, &q[0], &bad, native.weights()).unwrap())
    );
    let frozen_checkpoint = packed.checkpoint();
    for i in 0..original.weights.len() {
        if !original.stage_mask.is_trainable(i) {
            let ck = &frozen_checkpoint;
            assert_eq!(ck.weights[i].to_bits(), original.weights[i].to_bits());
            assert_eq!(ck.update_ages[i], original.update_ages[i]);
            assert_eq!(
                ck.first_moment[i].to_bits(),
                original.first_moment[i].to_bits()
            );
            assert_eq!(
                ck.second_moment[i].to_bits(),
                original.second_moment[i].to_bits()
            );
        }
    }
}

#[test]
#[ignore = "explicit CPU-only packed layout/tape/BPTT/Adam timing; no Tensor Core claim"]
fn packed_block_cpu_ledger() {
    let (p, s) = hotpath_bench::fixture();
    let plan = PackedBlocks::new(&p);
    let mut rows = Vec::new();
    for batch in [1, 8, 16] {
        let initial = hotpath_bench::learner(&p, &s);
        let qs = sequences(&initial, batch);
        for packed in [false, true] {
            let mut samples = Vec::new();
            for run in 0..6 {
                let mut t = hotpath_bench::learner(&p, &s);
                let start = Instant::now();
                let tapes = if packed {
                    plan.forward_batch(&p, t.weights(), &qs)
                } else {
                    qs.iter().map(|q| t.forward(q).unwrap()).collect()
                };
                let mut total = vec![0.; t.weights().len()];
                for (q, tape) in qs.iter().zip(&tapes) {
                    let g = plan
                        .graph
                        .backward(
                            &p,
                            t.weights(),
                            q,
                            tape,
                            &adjoints(q),
                            &vec![None; q.ticks.len()],
                        )
                        .unwrap();
                    for (dst, src) in total.iter_mut().zip(g.gradients) {
                        *dst += src;
                    }
                }
                black_box(t.apply_gradients(&total).unwrap());
                let ms = start.elapsed().as_secs_f64() * 1000.;
                if run > 0 {
                    samples.push(ms);
                }
            }
            rows.push(
                serde_json::json!({"batch":batch,"packed":packed,"forward_bptt_adam_ms":samples}),
            );
        }
    }
    let t = hotpath_bench::learner(&p, &s);
    let q = sequences(&t, 1);
    let mut pack = Vec::new();
    for _ in 0..21 {
        let at = Instant::now();
        let g = plan.pack(t.weights());
        black_box(plan.effective(&g, &q[0].ticks[0].effective_weight_offsets));
        pack.push(at.elapsed().as_secs_f64() * 1000.);
    }
    let mut plan_build_ms = Vec::new();
    for _ in 0..21 {
        let at = Instant::now();
        black_box(PackedBlocks::new(&p));
        plan_build_ms.push(at.elapsed().as_secs_f64() * 1000.);
    }
    let mut cohort_rows = Vec::new();
    for batch in [1, 8, 16] {
        let mut expected = None;
        for packed in [false, true] {
            let mut samples = Vec::new();
            for run in 0..6 {
                let (ms, state) = full_cohort(&p, &s, batch, packed);
                if run > 0 {
                    samples.push(ms);
                }
                if run == 1 {
                    if packed {
                        assert_eq!(expected.as_ref().unwrap(), &state);
                    } else {
                        expected = Some(state);
                    }
                }
            }
            cohort_rows.push(serde_json::json!({"batch":batch,"packed":packed,"full_native_residual_ppo_ms":samples}));
        }
    }
    let result = serde_json::json!({"backend":"scalar-cpu-fp32","tensor_core_execution":false,"blocks":plan.blocks.len(),"recurrent_edges":plan.ids.len(),"plan_packed_bytes":plan.ids.len()*4,"pack_genetic_plus_one_tick_ms":pack,"plan_build_ms":plan_build_ms,"trials":rows,"full_learner_trials":cohort_rows,"limits":"synthetic independent replay windows; separate unchanged full residual-PPO cohort includes admission/critic/update receipts; plan/fixture setup excluded; sequential arms, no statistical speed claim; includes full recurrence/dendrites/structural edges/metabolism/EMA/decoders/native BPTT/Adam; frozen per-tick lifetime/fast context; no collection, disk IO, GPU transfer or CUDA kernel"});
    if let Some(dir) = std::env::var_os("ALIFE_PACKED_OUTPUT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            std::path::PathBuf::from(dir).join("packed-cpu-ledger.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }
    eprintln!("PACKED_CPU {result}");
}

fn full_cohort(
    p: &BrainPhenotype,
    source: &alife_core::FoundationWeightAsset,
    batch: usize,
    packed: bool,
) -> (f64, Vec<u8>) {
    let mut actor = hotpath_bench::learner(p, source);
    let mut critic = hotpath_bench::value(p);
    // Authoritative old-policy data always comes from the native reference.
    let windows = (0..batch)
        .map(|i| hotpath_bench::window(&actor, 8 + i % 3, i % 3))
        .collect::<Vec<_>>();
    if packed {
        actor.graph.experimental_blocks = Some(Box::new(PackedBlocks::new(p)));
    }
    let start = Instant::now();
    let receipt = crate::cpu::train_cpu_residual_cohort(
        &mut actor,
        &mut critic,
        batch,
        batch,
        |i| Ok(windows[i].clone()),
        hotpath_bench::configuration(),
        7400,
    )
    .unwrap();
    let ms = start.elapsed().as_secs_f64() * 1000.;
    (
        ms,
        serde_json::to_vec(&(actor.checkpoint(), critic.checkpoint(), receipt)).unwrap(),
    )
}

#[test]
fn unchanged_full_native_ppo_admission_and_both_optimizers_match() {
    let (p, s) = hotpath_bench::fixture();
    assert_eq!(
        full_cohort(&p, &s, 3, false).1,
        full_cohort(&p, &s, 3, true).1
    );
    let mut actor = hotpath_bench::learner(&p, &s);
    let mut critic = hotpath_bench::value(&p);
    let config = hotpath_bench::configuration();
    let mut window = hotpath_bench::window(&actor, 8, 0);
    let mut transitions = window.actor.batch.transitions().to_vec();
    transitions[0].action.old_joint_log_probability += 0.2;
    window.actor.batch = crate::PpoBatch::from_rollout(7400, transitions, config).unwrap();
    actor.graph.experimental_blocks = Some(Box::new(PackedBlocks::new(&p)));
    let before = serde_json::to_vec(&(actor.checkpoint(), critic.checkpoint())).unwrap();
    assert!(crate::cpu::train_cpu_residual_cohort(
        &mut actor,
        &mut critic,
        1,
        1,
        |_| Ok(window.clone()),
        config,
        7400
    )
    .is_err());
    assert_eq!(
        before,
        serde_json::to_vec(&(actor.checkpoint(), critic.checkpoint())).unwrap()
    );
}

// This disposable forward mirrors the native equations; production uses replay.rs.
impl PackedBlocks {
    pub(super) fn forward_one(
        &self,
        p: &BrainPhenotype,
        q: &TrainingSequence,
        genetic: &[f32],
        weights: &[f32],
    ) -> Result<Tape, TrainingError> {
        let n = p.neuron_count() as usize;
        let steps = p.microstep_count() as usize;
        let mut a = vec![q.initial.activations.clone()];
        let mut m = vec![q.initial.metabolic_load.clone()];
        let mut ema = q.initial.activity_ema.clone();
        let mut activated = Vec::new();
        let mut evaluation = TrainingReplayEvaluation {
            candidate_logits: Vec::new(),
            final_activations: Vec::new(),
            final_activity_ema: Vec::new(),
            final_metabolic_load: Vec::new(),
        };
        let mut memory = Vec::new();
        let mut cognitive = Vec::new();
        for tick in &q.ticks {
            let effective = self.effective(genetic, &tick.effective_weight_offsets);
            for step in 0..steps {
                let previous = a.last().unwrap();
                let previous_m = m.last().unwrap();
                if step >= tick.microstep_count as usize {
                    a.push(previous.clone());
                    m.push(previous_m.clone());
                    activated.push(vec![0.; n]);
                    continue;
                }
                let mut next = vec![0.; n];
                let mut next_m = vec![0.; n];
                let mut act = vec![0.; n];
                for target in 0..n {
                    let mut sum = self.dot(target, step, tick, previous, &effective);
                    for s in tick
                        .structural_synapses
                        .iter()
                        .filter(|s| s.target as usize == target)
                    {
                        if tick.enabled_routes[s.route as usize]
                            && route_fires(s.cadence as u8, step)
                        {
                            sum += previous[s.source as usize] * s.effective_weight;
                        }
                    }
                    let mut dendritic = 0.;
                    for b in &q.initial.dendrites.branches()
                        [q.initial.dendrites.target_span(target as u32)]
                    {
                        let sum = b
                            .inputs
                            .iter()
                            .map(|i| previous[i.source as usize] * i.weight)
                            .sum::<f32>();
                        dendritic += (sum - b.threshold).max(0.).tanh() * b.output_gain;
                    }
                    let d = &p.neuron_dynamics()[target];
                    act[target] = activate(
                        d.bias()
                            + tick.encoded_inputs[target]
                            + tick.projection_gain * (sum + dendritic)
                            - d.homeostatic_gain() * previous_m[target]
                            - tick.local_threshold_shift,
                        d.activation(),
                    );
                    next[target] = (1. - d.leak()) * previous[target] + d.leak() * act[target];
                    next_m[target] = (d.metabolic_decay() * previous_m[target]
                        + (1. - d.metabolic_decay()) * next[target] * next[target])
                        .clamp(0., 1.);
                    ema[target] = (d.activity_ema_decay() * ema[target]
                        + (1. - d.activity_ema_decay()) * next[target].abs())
                    .clamp(0., 1.);
                }
                finite(&next)?;
                finite(&next_m)?;
                finite(&ema)?;
                a.push(next);
                m.push(next_m);
                activated.push(act);
            }
            let last = a.last().unwrap();
            let mut logits = Vec::new();
            let mut mem = Vec::new();
            let mut cog = Vec::new();
            for c in &tick.candidates {
                let mut logit = p
                    .candidate_decoder()
                    .families()
                    .iter()
                    .find(|f| f.family() == c.family)
                    .ok_or_else(invalid)?
                    .bias()
                    + c.innate_bias;
                let mut memory = 0.;
                let mut cognitive = 0.;
                for &id in &self.graph.decoders[c.family.raw() as usize] {
                    let s = &p.synapses()[id];
                    let CompiledSynapseKind::Decoder(coord) = s.kind() else {
                        unreachable!()
                    };
                    let v = c.decoder_inputs[coord.input_lane() as usize]
                        * (weights[id] + tick.effective_weight_offsets[id]);
                    match coord.head() {
                        DecoderHeadKind::ActionCandidate => logit += last[s.source() as usize] * v,
                        DecoderHeadKind::MemoryContext => memory += v,
                        DecoderHeadKind::CognitiveContext => cognitive += v,
                        _ => {}
                    }
                }
                logit += memory.clamp(-q.memory_candidate_gain, q.memory_candidate_gain)
                    + cognitive.clamp(-q.memory_candidate_gain, q.memory_candidate_gain);
                logits.push(logit);
                mem.push(memory);
                cog.push(cognitive);
            }
            finite(&logits)?;
            evaluation.candidate_logits.push(logits);
            evaluation.final_activations.push(last.clone());
            evaluation.final_activity_ema.push(ema.clone());
            evaluation
                .final_metabolic_load
                .push(m.last().unwrap().clone());
            memory.push(mem);
            cognitive.push(cog);
        }
        Ok(Tape {
            evaluation,
            a,
            m,
            activated,
            memory,
            cognitive,
        })
    }
}
