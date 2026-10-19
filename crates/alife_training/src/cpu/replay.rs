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
    incoming: Vec<Vec<usize>>,
    decoders: [Vec<usize>; 8],
}
pub(super) struct Tape {
    pub evaluation: TrainingReplayEvaluation,
    pub a: Vec<Vec<f32>>,
    pub m: Vec<Vec<f32>>,
    activated: Vec<Vec<f32>>,
    memory: Vec<Vec<f32>>,
    cognitive: Vec<Vec<f32>>,
}
impl SparseGraph {
    pub fn new(p: &BrainPhenotype) -> Self {
        let mut incoming = vec![Vec::new(); p.neuron_count() as usize];
        let mut decoders: [Vec<usize>; 8] = Default::default();
        for (i, s) in p.synapses().iter().enumerate() {
            match s.kind() {
                CompiledSynapseKind::Recurrent => incoming[s.target() as usize].push(i),
                CompiledSynapseKind::Decoder(c) if c.head() != DecoderHeadKind::SpeechPayload => {
                    decoders[c.family().raw() as usize].push(i)
                }
                _ => {}
            }
        }
        Self { incoming, decoders }
    }
    pub fn forward(
        &self,
        p: &BrainPhenotype,
        weights: &[f32],
        q: &TrainingSequence,
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
                    let mut sum = 0.;
                    for &id in &self.incoming[target] {
                        let s = &p.synapses()[id];
                        if tick.enabled_routes[s.route_index() as usize]
                            && route_fires(
                                p.projections()[s.route_index() as usize]
                                    .update_cadence()
                                    .raw(),
                                step,
                            )
                        {
                            sum += previous[s.source() as usize]
                                * (weights[id] + tick.effective_weight_offsets[id]);
                        }
                    }
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
                for &id in &self.decoders[c.family.raw() as usize] {
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

    pub fn backward(
        &self,
        p: &BrainPhenotype,
        weights: &[f32],
        q: &TrainingSequence,
        tape: &Tape,
        adjoints: &[Vec<f32>],
        speech_targets: &[Option<crate::ReplaySpeechTarget>],
    ) -> Result<TrainingGradientProbe, TrainingError> {
        if adjoints.len() != q.ticks.len() || speech_targets.len() != q.ticks.len() {
            return Err(invalid());
        }
        let n = p.neuron_count() as usize;
        let steps = p.microstep_count() as usize;
        let mut ga = vec![vec![0.; n]; tape.a.len()];
        let mut gm = vec![vec![0.; n]; tape.m.len()];
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
        for (t, (tick, adjoint)) in q.ticks.iter().zip(adjoints).enumerate() {
            if adjoint.len() != tick.candidates.len() {
                return Err(invalid());
            }
            finite(adjoint)?;
            if t < q.burn_in_ticks {
                if adjoint.iter().any(|g| *g != 0.) {
                    return Err(invalid());
                }
                continue;
            }
            let state = (t + 1) * steps;
            for (c, (candidate, &g)) in tick.candidates.iter().zip(adjoint).enumerate() {
                objective += f64::from(g) * f64::from(tape.evaluation.candidate_logits[t][c]);
                for &id in &self.decoders[candidate.family.raw() as usize] {
                    let s = &p.synapses()[id];
                    let CompiledSynapseKind::Decoder(coord) = s.kind() else {
                        unreachable!()
                    };
                    let input = candidate.decoder_inputs[coord.input_lane() as usize];
                    match coord.head() {
                        DecoderHeadKind::ActionCandidate => {
                            gradients[id] += g * tape.a[state][s.source() as usize] * input;
                            ga[state][s.source() as usize] +=
                                g * input * (weights[id] + tick.effective_weight_offsets[id]);
                        }
                        DecoderHeadKind::MemoryContext | DecoderHeadKind::CognitiveContext => {
                            let raw = if coord.head() == DecoderHeadKind::MemoryContext {
                                tape.memory[t][c]
                            } else {
                                tape.cognitive[t][c]
                            };
                            if raw > -q.memory_candidate_gain && raw < q.memory_candidate_gain {
                                gradients[id] += g * input;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        for step in (q.burn_in_ticks * steps..q.ticks.len() * steps).rev() {
            let tick = &q.ticks[step / steps];
            let local = step % steps;
            if local >= tick.microstep_count as usize {
                for i in 0..n {
                    ga[step][i] += ga[step + 1][i];
                    gm[step][i] += gm[step + 1][i];
                }
                continue;
            }
            let mut delta = vec![0.; n];
            for i in 0..n {
                let d = &p.neuron_dynamics()[i];
                let output = tape.a[step + 1][i];
                let raw = d.metabolic_decay() * tape.m[step][i]
                    + (1. - d.metabolic_decay()) * output * output;
                let gate = f32::from((raw > 0. && raw < 1.) as u8);
                let total = ga[step + 1][i]
                    + gm[step + 1][i] * gate * 2. * (1. - d.metabolic_decay()) * output;
                delta[i] = total * d.leak() * derivative(tape.activated[step][i], d.activation());
                ga[step][i] += total * (1. - d.leak());
                gm[step][i] +=
                    gm[step + 1][i] * gate * d.metabolic_decay() - delta[i] * d.homeostatic_gain();
            }
            for (target, _) in delta.iter().enumerate() {
                for &id in &self.incoming[target] {
                    let s = &p.synapses()[id];
                    if tick.enabled_routes[s.route_index() as usize]
                        && route_fires(
                            p.projections()[s.route_index() as usize]
                                .update_cadence()
                                .raw(),
                            local,
                        )
                    {
                        gradients[id] += delta[target]
                            * tape.a[step][s.source() as usize]
                            * tick.projection_gain;
                        ga[step][s.source() as usize] += delta[target]
                            * (weights[id] + tick.effective_weight_offsets[id])
                            * tick.projection_gain;
                    }
                }
            }
            for s in &tick.structural_synapses {
                if tick.enabled_routes[s.route as usize] && route_fires(s.cadence as u8, local) {
                    ga[step][s.source as usize] +=
                        delta[s.target as usize] * s.effective_weight * tick.projection_gain;
                }
            }
            for b in q.initial.dendrites.branches() {
                let raw = b
                    .inputs
                    .iter()
                    .map(|i| tape.a[step][i.source as usize] * i.weight)
                    .sum::<f32>()
                    - b.threshold;
                if raw > 0. {
                    let v = raw.tanh();
                    for input in &b.inputs {
                        ga[step][input.source as usize] += delta[b.target as usize]
                            * tick.projection_gain
                            * b.output_gain
                            * (1. - v * v)
                            * input.weight;
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
        ga: &mut [Vec<f32>],
        gradients: &mut [f32],
        objective: &mut f64,
    ) -> Result<(), TrainingError> {
        let scales = crate::replay_speech_loss_scales(targets);
        let target_start = p.candidate_decoder().motor_start() as usize
            + alife_core::SpeechDecoderLayoutV1::MOTOR_TARGET_OFFSET as usize;
        let source_start = target_start - 32;
        let steps = p.microstep_count() as usize;
        let ids = p
            .synapses()
            .iter()
            .enumerate()
            .filter_map(|(i, s)| match s.kind() {
                CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload => {
                    Some(i)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for (tick, target) in targets.iter().enumerate() {
            let Some(target) = target else { continue };
            target.validate()?;
            if tick < q.burn_in_ticks {
                return Err(invalid());
            }
            let state = (tick + 1) * steps;
            let context = &q.ticks[tick];
            let mut raw = [[0_f32; 32]; 6];
            for step in 0..6 {
                for &id in &ids {
                    let s = &p.synapses()[id];
                    let output = s.target() as usize - target_start;
                    let lane = s.source() as usize - source_start;
                    let input = if step > 0 && lane >= 18 {
                        raw[step - 1][lane].tanh()
                    } else {
                        tape.a[state][s.source() as usize]
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
                for &id in &ids {
                    let s = &p.synapses()[id];
                    let output = s.target() as usize - target_start;
                    let lane = s.source() as usize - source_start;
                    let input = if step > 0 && lane >= 18 {
                        raw[step - 1][lane].tanh()
                    } else {
                        tape.a[state][s.source() as usize]
                    };
                    gradients[id] += deltas[output] * input;
                    input_gradient[lane] +=
                        deltas[output] * (weights[id] + context.effective_weight_offsets[id]);
                }
                for lane in 0..32 {
                    if step == 0 || lane < 18 {
                        ga[state][source_start + lane] += input_gradient[lane];
                    }
                }
                future = input_gradient;
            }
        }
        Ok(())
    }
}
