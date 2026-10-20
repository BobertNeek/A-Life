//! CPU spelling of the production encoder, conditional sampling and speech ABI.
use super::{finite, invalid, CpuFoundationTrainer};
use crate::{PpoJointAction, TrainingError, TrainingReplayCandidate, TrainingReplayTick};
use alife_core::*;

/// The input is the finalized canonical memory/attention frame, never a teacher
/// label. Route/structural context is supplied by the organism-owned host.
pub fn encode_cpu_frame(
    p: &BrainPhenotype,
    frame: &PerceptionFrame,
    recall: &FinalizedMemoryRecall,
    effects: &NeuralReceptorEffects,
    enabled_routes: Vec<bool>,
    structural_synapses: Vec<crate::TrainingFrozenSynapse>,
) -> Result<TrainingReplayTick, TrainingError> {
    recall.validate_for_frame(frame)?;
    effects.validate_contract()?;
    if effects.source_tick != frame.tick()
        || frame.sensor_profile() != p.sensor_profile()
        || enabled_routes.len() != p.projections().len()
    {
        return Err(invalid());
    }
    let sensory = frame.sensory().channels.as_flat_array();
    let body = frame.body().neural_projection(frame.sensor_profile());
    let t = body.pose.translation;
    let r = body.pose.rotation;
    let v = body.velocity.linear;
    let a = body.velocity.angular;
    let body = [
        t.x, t.y, t.z, r.x, r.y, r.z, r.w, v.x, v.y, v.z, a.x, a.y, a.z,
    ];
    let homeostasis = frame.homeostasis();
    let home = homeostasis
        .drives
        .to_array()
        .into_iter()
        .chain(homeostasis.hormones.to_array())
        .collect::<Vec<_>>();
    let language = frame.sensory().language_prior_neural_lanes();
    let mut encoded = vec![0.; p.neuron_count() as usize];
    for assignment in p.sensor_encoder().assignments() {
        let index = assignment.source_index() as usize;
        let source = match assignment.source_group() {
            SensorEncoderSourceGroup::SensoryChannel => {
                sensory.get(index).map(|x| x * effects.attention_gain)
            }
            SensorEncoderSourceGroup::Body => body.get(index).copied(),
            SensorEncoderSourceGroup::Homeostasis => {
                home.get(index).map(|x| x * effects.interoceptive_gain)
            }
            SensorEncoderSourceGroup::HeardLanguage => language.get(index).copied(),
            SensorEncoderSourceGroup::SemanticPrior => language.get(128 + index).copied(),
        }
        .ok_or_else(invalid)?;
        let (min, max) = assignment.clamp_range();
        encoded[assignment.target_neuron() as usize] +=
            (source * assignment.scale() + assignment.bias()).clamp(min, max);
    }
    let gate =
        (effects.structural_growth_gate + effects.sleep_gate + effects.consolidation_gate) / 3.;
    for value in &mut encoded {
        *value = (*value * effects.regional_excitability + (gate - 0.5) * 0.05).clamp(-1., 1.);
    }
    let projection = recall
        .cognitive_context()
        .and_then(|c| c.cognitive_projection.as_ref())
        .ok_or_else(invalid)?;
    if projection.base_frame_digest != frame.base_digest()
        || projection.candidates.len() != frame.candidates().len()
    {
        return Err(invalid());
    }
    let candidates = frame
        .candidates()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let m = recall.context().candidates.get(i).ok_or_else(invalid)?;
            let cog = &projection.candidates[i];
            if m.candidate_index as usize != i
                || cog.candidate_index as usize != i
                || cog.prediction.predicted_successor.len() > 13
            {
                return Err(invalid());
            }
            let mut inputs = [0.; 54];
            inputs[..24].copy_from_slice(&c.features.0);
            for (dst, src) in inputs[24..32].iter_mut().zip(m.target_latent) {
                *dst = src * m.target_confidence.raw().clamp(0., 1.);
            }
            for (dst, src) in inputs[32..36].iter_mut().zip(m.family_value) {
                *dst = src * m.family_confidence.raw().clamp(0., 1.);
            }
            for (dst, src) in inputs[36..49]
                .iter_mut()
                .zip(&cog.prediction.predicted_successor)
            {
                *dst = *src;
            }
            inputs[49] = cog.concept_match.raw();
            inputs[50] = cog.gap_match.raw();
            inputs[51] = cog.prior_residual.raw();
            inputs[52] = f32::from(cog.forecast_available);
            finite(&inputs)?;
            let family = p
                .candidate_decoder()
                .families()
                .iter()
                .find(|f| f.family() == c.family)
                .ok_or_else(invalid)?;
            Ok(TrainingReplayCandidate {
                family: c.family,
                decoder_inputs: inputs,
                innate_bias: family.innate_contribution(homeostasis.drives, c.features),
            })
        })
        .collect::<Result<Vec<_>, TrainingError>>()?;
    finite(&encoded)?;
    Ok(TrainingReplayTick {
        encoded_inputs: encoded,
        projection_gain: effects.projection_gain,
        local_threshold_shift: effects.local_threshold_shift,
        microstep_count: u32::from(p.microstep_count()),
        enabled_routes,
        effective_weight_offsets: vec![0.; p.synapses().len()],
        structural_synapses,
        candidates,
    })
}
fn motor_slot(kind: ActionKind) -> Option<u8> {
    match kind {
        ActionKind::Move => Some(0),
        ActionKind::Look => Some(1),
        ActionKind::Interact | ActionKind::Write => Some(2),
        ActionKind::Vocalize => Some(3),
        ActionKind::Hold | ActionKind::Rest | ActionKind::Inspect => Some(4),
        _ => None,
    }
}
fn draw(seed: u32, counter: u32, factor: u32) -> f32 {
    let mut v = seed ^ counter.wrapping_mul(0x9e3779b9) ^ factor.wrapping_mul(0x85ebca6b);
    v = (v ^ (v >> 16)).wrapping_mul(0x7feb352d);
    v = (v ^ (v >> 15)).wrapping_mul(0x846ca68b);
    v ^= v >> 16;
    ((v >> 9) as f32 + 0.5) / 8388608.
}
fn sample(logits: &[f32], mask: u32, temp: f32, random: f32) -> Result<(u16, f32), TrainingError> {
    let max = logits
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, x)| *x)
        .fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.;
    let mut values = Vec::new();
    for (i, &l) in logits.iter().enumerate() {
        if mask & (1 << i) != 0 {
            let e = ((l - max) / temp).exp();
            sum += e;
            values.push((i, e));
        }
    }
    if !sum.is_finite() || sum <= 0. {
        return Err(invalid());
    }
    let mut cumulative = 0.;
    let mut selected = values.last().ok_or_else(invalid)?.0;
    for &(i, e) in &values {
        cumulative += e;
        if random * sum < cumulative {
            selected = i;
            break;
        }
    }
    Ok((selected as u16, (logits[selected] - max) / temp - sum.ln()))
}
/// The masks come from finite current-policy logits and native channel metadata,
/// before execution. World failure never changes an old-policy probability.
pub fn sample_cpu_joint_action(
    p: &BrainPhenotype,
    frame: &PerceptionFrame,
    logits: &[f32],
    seed: u32,
    counter: u32,
    temperature: f32,
) -> Result<PpoJointAction, TrainingError> {
    let count = logits.len();
    if count == 0
        || count > 32
        || frame.candidates().len() != count
        || !temperature.is_finite()
        || temperature <= 0.
    {
        return Err(invalid());
    }
    finite(logits)?;
    let channels = p.candidate_decoder().factorized_motor_channels(p)?;
    let enabled = |slot: usize| {
        channels.iter().any(|c| match c {
            MotorChannel::Locomotion => slot == 0,
            MotorChannel::Orientation => slot == 1,
            MotorChannel::Manipulation => slot == 2,
            MotorChannel::Vocal => slot == 3,
            MotorChannel::Posture => slot == 4,
            MotorChannel::SpeciesSpecific(_) => slot == 5,
        })
    };
    let representative_mask = u32::MAX >> (32 - count);
    let mut motor_masks = [0; 6];
    let mut forced_slots = Vec::with_capacity(count);
    for (i, c) in frame.candidates().iter().enumerate() {
        if let Some(slot) = motor_slot(c.kind) {
            if enabled(slot as usize) {
                motor_masks[slot as usize] |= 1 << i;
            }
        }
        forced_slots.push(
            if matches!(c.kind, ActionKind::Idle | ActionKind::Gesture) {
                Some(4)
            } else {
                motor_slot(c.kind)
            },
        );
    }
    let (representative, mut logp) = sample(
        logits,
        representative_mask,
        temperature,
        draw(seed, counter, 0),
    )?;
    let forced = forced_slots[representative as usize];
    let mut motor_candidates = [None; 6];
    for slot in 0..6 {
        if forced == Some(slot as u8) {
            motor_candidates[slot] = Some(representative);
        } else if motor_masks[slot] != 0 {
            let (index, lp) = sample(
                logits,
                motor_masks[slot],
                temperature,
                draw(seed, counter, slot as u32 + 1),
            )?;
            motor_candidates[slot] = Some(index);
            logp += lp;
        }
    }
    let action = PpoJointAction {
        candidate_count: count as u16,
        representative_mask,
        motor_masks,
        representative,
        forced_slots,
        motor_candidates,
        old_joint_log_probability: logp,
        temperature,
    };
    action.validate(temperature)?;
    Ok(action)
}
/// Six recurrent speech steps, including the byte confidence quantization in
/// closed_loop_decode.wgsl. Silence is a valid absence of a payload receipt.
pub fn decode_cpu_speech(
    founder: &CpuFoundationTrainer,
    activations: &[f32],
    offsets: &[f32],
) -> Result<Option<SpeechMotorPayload>, TrainingError> {
    let p = founder.phenotype();
    if activations.len() != p.neuron_count() as usize || offsets.len() != p.synapses().len() {
        return Err(invalid());
    }
    let source = p.candidate_decoder().motor_start() as usize + 128;
    let target = source + 32;
    let ids=p.synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload)).collect::<Vec<_>>();
    if ids.len() != 1024 {
        return Err(invalid());
    }
    let mut inputs = [0.; 32];
    inputs.copy_from_slice(&activations[source..source + 32]);
    let mut tokens = Vec::new();
    let mut act = 0;
    let mut confidence = 0.;
    for step in 0..6 {
        let mut outputs = [0_f32; 32];
        for &(i, s) in &ids {
            let output = (s.target() as usize)
                .checked_sub(target)
                .filter(|x| *x < 32)
                .ok_or_else(invalid)?;
            let lane = (s.source() as usize)
                .checked_sub(source)
                .filter(|x| *x < 32)
                .ok_or_else(invalid)?;
            outputs[output] += inputs[lane] * (founder.weights()[i] + offsets[i]);
        }
        finite(&outputs)?;
        for x in &mut outputs {
            *x = x.tanh();
        }
        if step == 0 {
            for i in 1..8 {
                if outputs[i] > outputs[act] {
                    act = i;
                }
            }
            confidence = (outputs[act].abs().clamp(0., 1.) * 255.).round() / 255.;
        }
        if outputs[16] > 0. {
            let mut token = 0;
            for bit in 0..8 {
                if outputs[8 + bit] > 0. {
                    token |= 1 << bit;
                }
            }
            if token != 0 {
                tokens.push(LanguageTokenId::new(token)?);
            }
        }
        if outputs[17] > 0. {
            break;
        }
        inputs[18..32].copy_from_slice(&outputs[18..32]);
    }
    if tokens.is_empty() {
        Ok(None)
    } else {
        Ok(Some(SpeechMotorPayload::try_new(
            SpeechActKind::try_from_raw(act as u8)?,
            tokens,
            Confidence::new(confidence)?,
        )?))
    }
}
