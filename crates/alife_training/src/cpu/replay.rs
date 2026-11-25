use super::{finite, invalid};
use crate::{TrainingError, TrainingGradientProbe, TrainingReplayEvaluation, TrainingSequence};
use alife_core::{ActivationFunction, BrainPhenotype, CompiledSynapseKind, DecoderHeadKind};

pub(super) fn route_fires(raw: u8, step: usize) -> bool {
    match raw {
        0 => true,
        1 | 2 => step.is_multiple_of(2),
        3 | 4 => step == 0,
        _ => false,
    }
}
fn activate(x: f32, kind: ActivationFunction) -> f32 {
    match kind.raw() {
        0 => x,
        1 => x.max(0.),
        2 => x.tanh(),
        3 => 1. / (1. + (-x).exp()),
        _ => 0.,
    }
}
fn derivative(x: f32, kind: ActivationFunction) -> f32 {
    match kind.raw() {
        0 => 1.,
        1 => f32::from((x > 0.) as u8),
        2 => (1. - x * x).max(0.),
        3 => x * (1. - x),
        _ => 0.,
    }
}

/// Canonical edge order within each target/family, matching the WGSL summation.
pub(super) struct SparseGraph {
    incoming_offsets: Vec<u32>,
    incoming_synapse_id: Vec<u32>,
    incoming_source: Vec<u32>,
    incoming_route: Vec<u16>,
    incoming_cadence: Vec<u8>,
    decoder_offsets: [u32; 9],
    decoder_synapse_id: Vec<u32>,
    decoder_source: Vec<u32>,
    decoder_input_lane: Vec<u16>,
    decoder_head: Vec<DecoderHeadKind>,
    family_biases: [Option<f32>; 8],
    speech_synapse_id: Vec<u32>,
    speech_source: Vec<u32>,
    speech_target: Vec<u32>,
}
pub(super) struct Tape {
    pub evaluation: TrainingReplayEvaluation,
    a: Vec<f32>,
    m: Vec<f32>,
    activated: Vec<f32>,
    memory: Vec<f32>,
    cognitive: Vec<f32>,
    cand_offsets: Vec<usize>,
    neuron_count: usize,
}
impl Tape {
    pub(super) fn a_step(&self, step: usize) -> &[f32] {
        let start = step * self.neuron_count;
        &self.a[start..start + self.neuron_count]
    }
}
impl SparseGraph {
    pub fn new(p: &BrainPhenotype) -> Self {
        let n = p.neuron_count() as usize;
        let mut incoming_counts = vec![0u32; n];
        let mut decoder_counts = [0u32; 8];
        let mut speech_synapse_id = Vec::new();
        let mut speech_source = Vec::new();
        let mut speech_target = Vec::new();
        for (i, s) in p.synapses().iter().enumerate() {
            match s.kind() {
                CompiledSynapseKind::Recurrent => {
                    incoming_counts[s.target() as usize] += 1;
                }
                CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload => {
                    speech_synapse_id.push(i as u32);
                    speech_source.push(s.source());
                    speech_target.push(s.target());
                }
                CompiledSynapseKind::Decoder(c) => {
                    decoder_counts[c.family().raw() as usize] += 1;
                }
            }
        }
        let mut incoming_offsets = Vec::with_capacity(n + 1);
        let mut total_incoming = 0u32;
        for c in incoming_counts {
            incoming_offsets.push(total_incoming);
            total_incoming += c;
        }
        incoming_offsets.push(total_incoming);

        let mut decoder_offsets = [0u32; 9];
        let mut total_decoder = 0u32;
        for (f, c) in decoder_counts.into_iter().enumerate() {
            decoder_offsets[f] = total_decoder;
            total_decoder += c;
        }
        decoder_offsets[8] = total_decoder;

        let total_incoming_usize = total_incoming as usize;
        let mut incoming_synapse_id = vec![0u32; total_incoming_usize];
        let mut incoming_source = vec![0u32; total_incoming_usize];
        let mut incoming_route = vec![0u16; total_incoming_usize];
        let mut incoming_cadence = vec![0u8; total_incoming_usize];
        let mut incoming_cursor = incoming_offsets[..n].to_vec();

        let total_decoder_usize = total_decoder as usize;
        let mut decoder_synapse_id = vec![0u32; total_decoder_usize];
        let mut decoder_source = vec![0u32; total_decoder_usize];
        let mut decoder_input_lane = vec![0u16; total_decoder_usize];
        let mut decoder_head = vec![DecoderHeadKind::ActionCandidate; total_decoder_usize];
        let mut decoder_cursor: [u32; 8] = decoder_offsets[..8].try_into().unwrap();

        let projections = p.projections();
        for (i, s) in p.synapses().iter().enumerate() {
            match s.kind() {
                CompiledSynapseKind::Recurrent => {
                    let t = s.target() as usize;
                    let slot = incoming_cursor[t] as usize;
                    incoming_cursor[t] += 1;
                    incoming_synapse_id[slot] = i as u32;
                    incoming_source[slot] = s.source();
                    incoming_route[slot] = s.route_index();
                    incoming_cadence[slot] =
                        projections[s.route_index() as usize].update_cadence().raw();
                }
                CompiledSynapseKind::Decoder(c) if c.head() != DecoderHeadKind::SpeechPayload => {
                    let f = c.family().raw() as usize;
                    let slot = decoder_cursor[f] as usize;
                    decoder_cursor[f] += 1;
                    decoder_synapse_id[slot] = i as u32;
                    decoder_source[slot] = s.source();
                    decoder_input_lane[slot] = c.input_lane();
                    decoder_head[slot] = c.head();
                }
                _ => {}
            }
        }
        let mut family_biases = [None; 8];
        for f in p.candidate_decoder().families() {
            let idx = f.family().raw() as usize;
            if idx < 8 && family_biases[idx].is_none() {
                family_biases[idx] = Some(f.bias());
            }
        }
        Self {
            incoming_offsets,
            incoming_synapse_id,
            incoming_source,
            incoming_route,
            incoming_cadence,
            decoder_offsets,
            decoder_synapse_id,
            decoder_source,
            decoder_input_lane,
            decoder_head,
            family_biases,
            speech_synapse_id,
            speech_source,
            speech_target,
        }
    }
    pub fn forward(
        &self,
        p: &BrainPhenotype,
        weights: &[f32],
        q: &TrainingSequence,
    ) -> Result<Tape, TrainingError> {
        let n = p.neuron_count() as usize;
        let steps = p.microstep_count() as usize;
        let total_steps = q.ticks.len() * steps;
        let mut a = vec![0.; (total_steps + 1) * n];
        let mut m = vec![0.; (total_steps + 1) * n];
        a[..n].copy_from_slice(&q.initial.activations);
        m[..n].copy_from_slice(&q.initial.metabolic_load);
        let mut ema = q.initial.activity_ema.clone();
        let mut activated = vec![0.; total_steps * n];
        let mut evaluation = TrainingReplayEvaluation {
            candidate_logits: Vec::with_capacity(q.ticks.len()),
            final_activations: Vec::with_capacity(q.ticks.len()),
            final_activity_ema: Vec::with_capacity(q.ticks.len()),
            final_metabolic_load: Vec::with_capacity(q.ticks.len()),
        };
        let total_cands: usize = q.ticks.iter().map(|t| t.candidates.len()).sum();
        let mut memory = Vec::with_capacity(total_cands);
        let mut cognitive = Vec::with_capacity(total_cands);
        let mut cand_offsets = Vec::with_capacity(q.ticks.len() + 1);
        cand_offsets.push(0);

        let mut struct_counts = Vec::new();
        let mut struct_offsets = Vec::new();
        let mut struct_edges = Vec::new();
        let dynamics = p.neuron_dynamics();
        let branches = q.initial.dendrites.branches();

        for (tick_idx, tick) in q.ticks.iter().enumerate() {
            let has_structural = !tick.structural_synapses.is_empty();
            if has_structural {
                if struct_counts.is_empty() {
                    struct_counts.resize(n, 0u32);
                    struct_offsets.resize(n + 1, 0u32);
                } else {
                    struct_counts.fill(0);
                }
                for s in &tick.structural_synapses {
                    struct_counts[s.target as usize] += 1;
                }
                let mut offset = 0u32;
                for t in 0..n {
                    struct_offsets[t] = offset;
                    offset += struct_counts[t];
                }
                struct_offsets[n] = offset;
                struct_edges.resize(offset as usize, 0usize);
                struct_counts.copy_from_slice(&struct_offsets[..n]);
                for (idx, s) in tick.structural_synapses.iter().enumerate() {
                    let t = s.target as usize;
                    let slot = struct_counts[t] as usize;
                    struct_counts[t] += 1;
                    struct_edges[slot] = idx;
                }
            }
            for step in 0..steps {
                let global_step = tick_idx * steps + step;
                let prev_start = global_step * n;
                let next_start = (global_step + 1) * n;
                let (a_head, a_tail) = a.split_at_mut(next_start);
                let previous = &a_head[prev_start..];
                let next = &mut a_tail[..n];
                let (m_head, m_tail) = m.split_at_mut(next_start);
                let previous_m = &m_head[prev_start..];
                let next_m = &mut m_tail[..n];
                if step >= tick.microstep_count as usize {
                    next.copy_from_slice(previous);
                    next_m.copy_from_slice(previous_m);
                    continue;
                }
                let act = &mut activated[prev_start..next_start];
                for target in 0..n {
                    let mut sum = 0.;
                    let edge_start = self.incoming_offsets[target] as usize;
                    let edge_end = self.incoming_offsets[target + 1] as usize;
                    for idx in edge_start..edge_end {
                        let route = self.incoming_route[idx] as usize;
                        if tick.enabled_routes[route]
                            && route_fires(self.incoming_cadence[idx], step)
                        {
                            let id = self.incoming_synapse_id[idx] as usize;
                            let src = self.incoming_source[idx] as usize;
                            sum +=
                                previous[src] * (weights[id] + tick.effective_weight_offsets[id]);
                        }
                    }
                    if has_structural {
                        let s_start = struct_offsets[target] as usize;
                        let s_end = struct_offsets[target + 1] as usize;
                        for &idx in &struct_edges[s_start..s_end] {
                            let s = &tick.structural_synapses[idx];
                            if tick.enabled_routes[s.route as usize]
                                && route_fires(s.cadence as u8, step)
                            {
                                sum += previous[s.source as usize] * s.effective_weight;
                            }
                        }
                    }
                    let mut dendritic = 0.;
                    for b in &branches[q.initial.dendrites.target_span(target as u32)] {
                        let b_sum = b
                            .inputs
                            .iter()
                            .map(|i| previous[i.source as usize] * i.weight)
                            .sum::<f32>();
                        dendritic += (b_sum - b.threshold).max(0.).tanh() * b.output_gain;
                    }
                    let d = &dynamics[target];
                    let act_val = activate(
                        d.bias()
                            + tick.encoded_inputs[target]
                            + tick.projection_gain * (sum + dendritic)
                            - d.homeostatic_gain() * previous_m[target]
                            - tick.local_threshold_shift,
                        d.activation(),
                    );
                    act[target] = act_val;
                    let next_val = (1. - d.leak()) * previous[target] + d.leak() * act_val;
                    next[target] = next_val;
                    next_m[target] = (d.metabolic_decay() * previous_m[target]
                        + (1. - d.metabolic_decay()) * next_val * next_val)
                        .clamp(0., 1.);
                    ema[target] = (d.activity_ema_decay() * ema[target]
                        + (1. - d.activity_ema_decay()) * next_val.abs())
                    .clamp(0., 1.);
                }
                finite(next)?;
                finite(next_m)?;
                finite(&ema)?;
            }
            let last_start = (tick_idx + 1) * steps * n;
            let last = &a[last_start..last_start + n];
            let last_m = &m[last_start..last_start + n];
            let mut logits = Vec::with_capacity(tick.candidates.len());
            for c in &tick.candidates {
                let family_idx = c.family.raw() as usize;
                let base_bias = self
                    .family_biases
                    .get(family_idx)
                    .copied()
                    .flatten()
                    .ok_or_else(invalid)?;
                let mut logit = base_bias + c.innate_bias;
                let mut mem = 0.;
                let mut cog = 0.;
                let dec_start = self.decoder_offsets[family_idx] as usize;
                let dec_end = self.decoder_offsets[family_idx + 1] as usize;
                for idx in dec_start..dec_end {
                    let id = self.decoder_synapse_id[idx] as usize;
                    let v = c.decoder_inputs[self.decoder_input_lane[idx] as usize]
                        * (weights[id] + tick.effective_weight_offsets[id]);
                    match self.decoder_head[idx] {
                        DecoderHeadKind::ActionCandidate => {
                            logit += last[self.decoder_source[idx] as usize] * v;
                        }
                        DecoderHeadKind::MemoryContext => mem += v,
                        DecoderHeadKind::CognitiveContext => cog += v,
                        DecoderHeadKind::SpeechPayload => {}
                    }
                }
                logit += mem.clamp(-q.memory_candidate_gain, q.memory_candidate_gain)
                    + cog.clamp(-q.memory_candidate_gain, q.memory_candidate_gain);
                logits.push(logit);
                memory.push(mem);
                cognitive.push(cog);
            }
            finite(&logits)?;
            cand_offsets.push(memory.len());
            evaluation.candidate_logits.push(logits);
            evaluation.final_activations.push(last.to_vec());
            evaluation.final_activity_ema.push(ema.clone());
            evaluation.final_metabolic_load.push(last_m.to_vec());
        }
        Ok(Tape {
            evaluation,
            a,
            m,
            activated,
            memory,
            cognitive,
            cand_offsets,
            neuron_count: n,
        })
    }

    pub fn backward(
        &self,
        p: &BrainPhenotype,
        weights: &[f32],
        q: &TrainingSequence,
        tape: &Tape,
        adjoints: &[Vec<f32>],
        speech_targets: &[Option<crate::ReplaySpeechTarget>],
    ) -> Result<TrainingGradientProbe, TrainingError> {
        if adjoints.len() != q.ticks.len() - q.burn_in_ticks
            || speech_targets.len() != q.ticks.len()
        {
            return Err(invalid());
        }
        let n = p.neuron_count() as usize;
        let steps = p.microstep_count() as usize;
        let mut ga = vec![0.; tape.a.len()];
        let mut gm = vec![0.; tape.m.len()];
        let mut gradients = vec![0.; weights.len()];
        let mut objective = 0_f64;
        self.speech_backward(
            p,
            weights,
            q,
            tape,
            speech_targets,
            &mut ga,
            &mut gradients,
            &mut objective,
        )?;
        for (offset, (tick, adjoint)) in q.ticks[q.burn_in_ticks..].iter().zip(adjoints).enumerate()
        {
            let t = q.burn_in_ticks + offset;
            if adjoint.len() != tick.candidates.len() {
                return Err(invalid());
            }
            finite(adjoint)?;
            let state_base = (t + 1) * steps * n;
            let cand_base = tape.cand_offsets[t];
            for (c, (candidate, &g)) in tick.candidates.iter().zip(adjoint).enumerate() {
                objective += f64::from(g) * f64::from(tape.evaluation.candidate_logits[t][c]);
                let family_idx = candidate.family.raw() as usize;
                let dec_start = self.decoder_offsets[family_idx] as usize;
                let dec_end = self.decoder_offsets[family_idx + 1] as usize;
                for idx in dec_start..dec_end {
                    let id = self.decoder_synapse_id[idx] as usize;
                    let input = candidate.decoder_inputs[self.decoder_input_lane[idx] as usize];
                    match self.decoder_head[idx] {
                        DecoderHeadKind::ActionCandidate => {
                            let src = self.decoder_source[idx] as usize;
                            gradients[id] += g * tape.a[state_base + src] * input;
                            ga[state_base + src] +=
                                g * input * (weights[id] + tick.effective_weight_offsets[id]);
                        }
                        DecoderHeadKind::MemoryContext | DecoderHeadKind::CognitiveContext => {
                            let raw = if self.decoder_head[idx] == DecoderHeadKind::MemoryContext {
                                tape.memory[cand_base + c]
                            } else {
                                tape.cognitive[cand_base + c]
                            };
                            if raw > -q.memory_candidate_gain && raw < q.memory_candidate_gain {
                                gradients[id] += g * input;
                            }
                        }
                        DecoderHeadKind::SpeechPayload => {}
                    }
                }
            }
        }
        let dynamics = p.neuron_dynamics();
        let branches = q.initial.dendrites.branches();
        let mut delta = vec![0.; n];
        for step in (q.burn_in_ticks * steps..q.ticks.len() * steps).rev() {
            let tick = &q.ticks[step / steps];
            let local = step % steps;
            let step_base = step * n;
            let next_base = (step + 1) * n;
            if local >= tick.microstep_count as usize {
                for i in 0..n {
                    ga[step_base + i] += ga[next_base + i];
                    gm[step_base + i] += gm[next_base + i];
                }
                continue;
            }
            for i in 0..n {
                let d = &dynamics[i];
                let output = tape.a[next_base + i];
                let raw = d.metabolic_decay() * tape.m[step_base + i]
                    + (1. - d.metabolic_decay()) * output * output;
                let gate = f32::from((raw > 0. && raw < 1.) as u8);
                let total = ga[next_base + i]
                    + gm[next_base + i] * gate * 2. * (1. - d.metabolic_decay()) * output;
                let d_val =
                    total * d.leak() * derivative(tape.activated[step_base + i], d.activation());
                delta[i] = d_val;
                ga[step_base + i] += total * (1. - d.leak());
                gm[step_base + i] +=
                    gm[next_base + i] * gate * d.metabolic_decay() - d_val * d.homeostatic_gain();
            }
            for (target, &d_val) in delta.iter().enumerate() {
                if d_val == 0.0 {
                    continue;
                }
                let edge_start = self.incoming_offsets[target] as usize;
                let edge_end = self.incoming_offsets[target + 1] as usize;
                for idx in edge_start..edge_end {
                    let route = self.incoming_route[idx] as usize;
                    if tick.enabled_routes[route] && route_fires(self.incoming_cadence[idx], local)
                    {
                        let id = self.incoming_synapse_id[idx] as usize;
                        let src = self.incoming_source[idx] as usize;
                        gradients[id] += d_val * tape.a[step_base + src] * tick.projection_gain;
                        ga[step_base + src] += d_val
                            * (weights[id] + tick.effective_weight_offsets[id])
                            * tick.projection_gain;
                    }
                }
            }
            for s in &tick.structural_synapses {
                let d_val = delta[s.target as usize];
                if d_val != 0.0
                    && tick.enabled_routes[s.route as usize]
                    && route_fires(s.cadence as u8, local)
                {
                    ga[step_base + s.source as usize] +=
                        d_val * s.effective_weight * tick.projection_gain;
                }
            }
            for b in branches {
                let d_val = delta[b.target as usize];
                if d_val == 0.0 {
                    continue;
                }
                let raw = b
                    .inputs
                    .iter()
                    .map(|i| tape.a[step_base + i.source as usize] * i.weight)
                    .sum::<f32>()
                    - b.threshold;
                if raw > 0. {
                    let v = raw.tanh();
                    let factor = d_val * tick.projection_gain * b.output_gain * (1. - v * v);
                    for input in &b.inputs {
                        ga[step_base + input.source as usize] += factor * input.weight;
                    }
                }
            }
        }
        finite(&gradients)?;
        if !objective.is_finite() {
            return Err(invalid());
        }
        Ok(TrainingGradientProbe {
            objective,
            gradients,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn speech_backward(
        &self,
        p: &BrainPhenotype,
        weights: &[f32],
        q: &TrainingSequence,
        tape: &Tape,
        targets: &[Option<crate::ReplaySpeechTarget>],
        ga: &mut [f32],
        gradients: &mut [f32],
        objective: &mut f64,
    ) -> Result<(), TrainingError> {
        if targets.iter().all(Option::is_none) {
            return Ok(());
        }
        let scales = crate::replay_speech_loss_scales(targets);
        let target_start = p.candidate_decoder().motor_start() as usize
            + alife_core::SpeechDecoderLayoutV1::MOTOR_TARGET_OFFSET as usize;
        let source_start = target_start - 32;
        let n = tape.neuron_count;
        let steps = p.microstep_count() as usize;
        for (tick, target) in targets.iter().enumerate() {
            let Some(target) = target else { continue };
            target.validate()?;
            if tick < q.burn_in_ticks {
                return Err(invalid());
            }
            let state_base = (tick + 1) * steps * n;
            let context = &q.ticks[tick];
            let mut raw = [[0_f32; 32]; 6];
            for step in 0..6 {
                for idx in 0..self.speech_synapse_id.len() {
                    let id = self.speech_synapse_id[idx] as usize;
                    let src = self.speech_source[idx] as usize;
                    let tgt = self.speech_target[idx] as usize;
                    let output = tgt - target_start;
                    let lane = src - source_start;
                    let input = if step > 0 && lane >= 18 {
                        raw[step - 1][lane].tanh()
                    } else {
                        tape.a[state_base + src]
                    };
                    raw[step][output] +=
                        input * (weights[id] + context.effective_weight_offsets[id]);
                }
            }
            let mut future = [0.; 32];
            for step in (0..6).rev() {
                let expected = target.step_logits(step);
                let mut deltas = [0.; 32];
                for output in 0..32 {
                    let weight = if step < target.len() {
                        target.output_weight(output) * scales[usize::from(target.token.is_none())]
                    } else {
                        0.
                    };
                    let error = raw[step][output] - expected[output];
                    deltas[output] = 2. * weight * error;
                    *objective += f64::from(weight) * f64::from(error).powi(2);
                    if output >= 18 {
                        let activation = raw[step][output].tanh();
                        deltas[output] += future[output] * (1. - activation * activation);
                    }
                }
                let mut input_gradient = [0.; 32];
                for idx in 0..self.speech_synapse_id.len() {
                    let id = self.speech_synapse_id[idx] as usize;
                    let src = self.speech_source[idx] as usize;
                    let tgt = self.speech_target[idx] as usize;
                    let output = tgt - target_start;
                    let lane = src - source_start;
                    let input = if step > 0 && lane >= 18 {
                        raw[step - 1][lane].tanh()
                    } else {
                        tape.a[state_base + src]
                    };
                    gradients[id] += deltas[output] * input;
                    input_gradient[lane] +=
                        deltas[output] * (weights[id] + context.effective_weight_offsets[id]);
                }
                for lane in 0..32 {
                    if step == 0 || lane < 18 {
                        ga[state_base + source_start + lane] += input_gradient[lane];
                    }
                }
                future = input_gradient;
            }
        }
        Ok(())
    }
}
