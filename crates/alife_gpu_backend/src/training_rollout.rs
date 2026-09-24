//! Training-only behavior-policy evidence. Neural logits and samples originate on GPU.
use alife_core::{ActionKind, PerceptionFrame, PerceptionFrameDigest, ScaffoldContractError};
use serde::{Deserialize, Serialize};

pub(crate) const TRAINING_PAYLOAD_TAG: u32 = 0x8000_0000;

/// Exact GPU state at an explicitly requested training boundary. Offsets in
/// brain_slot.word_ranges() are absolute; subtract mutable_word_base to index words.
/// Kept outside portable saves and obtained through the existing GPU readback.
#[derive(Debug, Clone)]
pub struct GpuTrainingStateSnapshot {
    pub handle: crate::GpuBrainHandle,
    pub tick: u64,
    pub logical_dispatch_generation: u64,
    pub active_activation_side: u8,
    pub active_weight_generation: u64,
    pub active_weight_bank: u8,
    pub phenotype: alife_core::BrainPhenotype,
    pub brain_slot: crate::GpuBrainSlot,
    pub v11: crate::GpuV11Checkpoint,
    pub mutable_word_base: u32,
    pub mutable_words: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpuTrainingSamplingConfig {
    pub seed: u32,
    /// Caller-owned counter, never inferred from wall time or frame ordering.
    pub counter: u32,
    pub temperature: f32,
    pub demonstrator: Option<GpuTrainingDemonstratorAction>,
}
impl GpuTrainingSamplingConfig {
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if !self.temperature.is_finite() || self.temperature < 1.0e-4 {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if let Some(action) = self.demonstrator {
            if action.representative_index >= 32
                || action
                    .motor_indices
                    .iter()
                    .any(|index| *index != u16::MAX && *index >= 32)
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        }
        Ok(())
    }
}

/// Offline teacher intent only. The GPU still checks the dispatch's legal logits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuTrainingDemonstratorAction {
    pub representative_index: u16,
    /// Zero-based indexes; u16::MAX for an empty channel. Forced slot equals representative.
    pub motor_indices: [u16; 6],
}

impl GpuTrainingDemonstratorAction {
    pub(crate) fn validate_for_frame(
        self,
        frame: &PerceptionFrame,
        enabled_channels: u32,
    ) -> Result<(), ScaffoldContractError> {
        let representative = frame
            .candidates()
            .get(self.representative_index as usize)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let forced = slot(representative.kind).unwrap_or(4);
        for (channel, index) in self.motor_indices.iter().copied().enumerate() {
            if channel == forced {
                if index != self.representative_index {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            } else if index != u16::MAX {
                let candidate = frame
                    .candidates()
                    .get(index as usize)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                if slot(candidate.kind) != Some(channel) || enabled_channels & (1 << channel) == 0 {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
        }
        // Only GPU-produced logits can prove a requested empty channel is empty.
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuTrainingBehaviorKind {
    OnPolicy,
    Demonstrator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuTrainingRolloutReceipt {
    pub organism_id: u64,
    pub tick: u64,
    pub dispatch_generation: u64,
    pub frame_digest: PerceptionFrameDigest,
    pub active_weight_generation: u64,
    pub sampling: GpuTrainingSamplingConfig,
    pub logits: Vec<f32>,
    /// Exact GPU-produced flattened candidate inputs, candidate-major.
    pub decoder_inputs: Vec<f32>,
    pub decoder_input_stride: usize,
    pub representative_mask: u32,
    pub motor_masks: [u32; 6],
    pub forced_motor_slots: Vec<u8>,
    pub representative_index: u16,
    /// u16::MAX means an empty channel. The forced channel contains the representative.
    pub motor_indices: [u16; 6],
    /// Representative then six channels; forced/empty channels contribute zero.
    pub factor_log_probabilities: [f32; 7],
    pub behavior: GpuTrainingBehaviorKind,
    /// Behavior probability exists only for sampled on-policy actions. Never PPO-replay demonstrations.
    pub joint_log_probability: Option<f32>,
    /// Current neural policy likelihood, including for supervised demonstration targets.
    pub policy_joint_log_probability: f32,
}

fn behavior_probability(
    demonstrator: Option<GpuTrainingDemonstratorAction>,
    representative_index: u16,
    motor_indices: [u16; 6],
    policy_log_probability: f32,
) -> Result<(GpuTrainingBehaviorKind, Option<f32>), ScaffoldContractError> {
    if let Some(demonstrator) = demonstrator {
        if demonstrator.representative_index != representative_index
            || demonstrator.motor_indices != motor_indices
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok((GpuTrainingBehaviorKind::Demonstrator, None))
    } else {
        Ok((
            GpuTrainingBehaviorKind::OnPolicy,
            Some(policy_log_probability),
        ))
    }
}

impl GpuTrainingRolloutReceipt {
    /// PPO callers must use this gate; teacher likelihood is never behavior likelihood.
    pub fn on_policy_log_probability(&self) -> Result<f32, ScaffoldContractError> {
        if self.behavior != GpuTrainingBehaviorKind::OnPolicy
            || self.sampling.demonstrator.is_some()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        self.joint_log_probability
            .filter(|value| value.is_finite() && *value == self.policy_joint_log_probability)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)
    }
}

fn slot(kind: ActionKind) -> Option<usize> {
    match kind {
        ActionKind::Move => Some(0),
        ActionKind::Interact | ActionKind::Write => Some(2),
        ActionKind::Vocalize => Some(3),
        ActionKind::Hold | ActionKind::Rest | ActionKind::Inspect => Some(4),
        ActionKind::Idle | ActionKind::Gesture => None,
    }
}

fn log_probability(
    logits: &[f32],
    mask: u32,
    chosen: u16,
    temperature: f32,
) -> Result<f32, ScaffoldContractError> {
    if chosen as usize >= logits.len() || mask & (1u32 << chosen) == 0 {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let maximum = logits
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1u32 << i) != 0)
        .map(|(_, v)| *v as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = logits
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1u32 << i) != 0)
        .map(|(_, v)| ((*v as f64 - maximum) / temperature as f64).exp())
        .sum();
    let result =
        ((logits[chosen as usize] as f64 - maximum) / temperature as f64 - sum.ln()) as f32;
    if !result.is_finite() {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    Ok(result)
}

pub(crate) fn build_receipt(
    frame: &PerceptionFrame,
    dispatch_generation: u64,
    active_weight_generation: u64,
    enabled_channels: u32,
    sampling: GpuTrainingSamplingConfig,
    bits: &[u32],
    decoder_input_bits: &[u32],
    representative_index: u16,
    encoded_motor_indices: [u16; 6],
) -> Result<GpuTrainingRolloutReceipt, ScaffoldContractError> {
    sampling.validate()?;
    if bits.len() != frame.candidates().len()
        || bits.is_empty()
        || bits.len() > 32
        || enabled_channels == 0
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    if decoder_input_bits.len() % bits.len() != 0
        || !(24..=64).contains(&(decoder_input_bits.len() / bits.len()))
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let decoder_inputs: Vec<f32> = decoder_input_bits
        .iter()
        .map(|v| f32::from_bits(*v))
        .collect();
    if decoder_inputs.iter().any(|v| !v.is_finite()) {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let decoder_input_stride = decoder_inputs.len() / bits.len();
    let logits: Vec<f32> = bits.iter().map(|bits| f32::from_bits(*bits)).collect();
    let mut representative_mask = 0;
    let mut motor_masks = [0; 6];
    let mut forced_motor_slots = Vec::with_capacity(bits.len());
    for (index, candidate) in frame.candidates().iter().enumerate() {
        let motor = slot(candidate.kind);
        forced_motor_slots.push(motor.unwrap_or(4) as u8);
        if !logits[index].is_finite() {
            continue;
        }
        representative_mask |= 1u32 << index;
        if let Some(motor) = motor {
            if enabled_channels & (1u32 << motor) != 0 {
                motor_masks[motor] |= 1u32 << index;
            }
        }
    }
    let mut factor_log_probabilities = [0.0; 7];
    factor_log_probabilities[0] = log_probability(
        &logits,
        representative_mask,
        representative_index,
        sampling.temperature,
    )?;
    let forced = forced_motor_slots[representative_index as usize] as usize;
    let mut motor_indices = [u16::MAX; 6];
    for motor in 0..6 {
        motor_indices[motor] = encoded_motor_indices[motor]
            .checked_sub(1)
            .unwrap_or(u16::MAX);
        if motor == forced {
            if motor_indices[motor] != representative_index {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        } else if motor_masks[motor] == 0 {
            if motor_indices[motor] != u16::MAX {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        } else {
            factor_log_probabilities[motor + 1] = log_probability(
                &logits,
                motor_masks[motor],
                motor_indices[motor],
                sampling.temperature,
            )?;
        }
    }
    let policy_joint_log_probability = factor_log_probabilities.iter().sum();
    let (behavior, joint_log_probability) = behavior_probability(
        sampling.demonstrator,
        representative_index,
        motor_indices,
        policy_joint_log_probability,
    )?;
    Ok(GpuTrainingRolloutReceipt {
        organism_id: frame.organism_id().raw(),
        tick: frame.tick().raw(),
        dispatch_generation,
        frame_digest: frame.frame_digest(),
        active_weight_generation,
        sampling,
        logits,
        decoder_inputs,
        decoder_input_stride,
        representative_mask,
        motor_masks,
        forced_motor_slots,
        representative_index,
        motor_indices,
        factor_log_probabilities,
        behavior,
        joint_log_probability,
        policy_joint_log_probability,
    })
}

/// Production source strings stay byte-for-byte unchanged without this feature.
pub(crate) fn shader_source(source: &str) -> String {
    assert_eq!(
        source
            .matches("fn sparse_selector_request_spans_valid(header:GpuPerceptionHeader) -> bool {")
            .count(),
        1,
        "training source must retain the checked common ABI hook"
    );
    let mut source = source.replace("fn sparse_selector_request_spans_valid(header:GpuPerceptionHeader) -> bool {",
        "fn sparse_selector_request_spans_valid(header:GpuPerceptionHeader) -> bool {\n  if ((header.reserved & 0x80000000u) != 0u) {\n    let offset = header.reserved & 0x7fffffffu;\n    let count = arrayLength(&frame_payload_words);\n    if (offset >= count || count - offset < 11u || header.candidate_count > 32u) { return false; }\n    let temperature = bitcast<f32>(frame_payload_words[offset + 2u]);\n    return temperature >= 0.0001 && temperature <= 3.402823e38 && frame_payload_words[offset + 3u] <= 1u;\n  }");
    if source.contains("fn select_candidate(") {
        assert_eq!(
            source
                .matches("  let base = brain.selection_offset;")
                .count(),
            1
        );
        assert_eq!(
            source
                .matches("  let selection = load_speech_selection(brain.selection_offset);")
                .count(),
            1
        );
        source = source.replace(
            "if (header.reserved != 0u)",
            "if (header.reserved != 0u && (header.reserved & 0x80000000u) == 0u)",
        );
        source = source.replace("  let base = brain.selection_offset;", "  if ((header.reserved & 0x80000000u) != 0u) {\n    selected_candidate = training_sample(header, brain, 0u);\n    found = selected_candidate < header.candidate_count;\n    if (found) {\n      selected_logit = load_state_f32(brain.candidate_logit_offset + selected_candidate);\n      selected_confidence = load_candidate(header.candidate_offset + selected_candidate * 8u).confidence_q16;\n    }\n  }\n  let base = brain.selection_offset;");
        source = source.replace("  let selection = load_speech_selection(brain.selection_offset);", "  if ((header.reserved & 0x80000000u) != 0u) {\n    for (var slot=0u; slot<6u; slot++) {\n      motor_candidates[slot] = training_sample(header, brain, slot + 1u);\n      motor_found[slot] = motor_candidates[slot] < header.candidate_count;\n    }\n  }\n  let selection = load_speech_selection(brain.selection_offset);");
        source.push_str(include_str!("../shaders/training_rollout.wgsl"));
    }
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn training_policy_shader_and_probability_contract() {
        // Validate the actual feature-composed modules, not a disconnected sampler.
        for source in [
            crate::CLOSED_LOOP_ENCODE_WGSL,
            crate::CLOSED_LOOP_RECURRENT_WGSL,
            crate::CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
            crate::CLOSED_LOOP_DECODE_WGSL,
            crate::CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
            crate::CLOSED_LOOP_ELIGIBILITY_WGSL,
            crate::CLOSED_LOOP_PLASTICITY_WGSL,
            crate::CLOSED_LOOP_CONSOLIDATE_WGSL,
            crate::CLOSED_LOOP_REPLAY_LEARNING_WGSL,
        ] {
            let source = shader_source(source);
            let module = naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .unwrap();
        }
        assert!(GpuTrainingSamplingConfig {
            seed: 1,
            counter: 0,
            temperature: f32::NAN,
            demonstrator: None,
        }
        .validate()
        .is_err());
        let teacher = GpuTrainingDemonstratorAction {
            representative_index: 0,
            motor_indices: [u16::MAX, u16::MAX, u16::MAX, u16::MAX, 0, u16::MAX],
        };
        assert_eq!(
            behavior_probability(Some(teacher), 0, teacher.motor_indices, -2.0).unwrap(),
            (GpuTrainingBehaviorKind::Demonstrator, None)
        );
        assert_eq!(
            behavior_probability(None, 0, teacher.motor_indices, -2.0).unwrap(),
            (GpuTrainingBehaviorKind::OnPolicy, Some(-2.0))
        );
        assert!(behavior_probability(Some(teacher), 1, teacher.motor_indices, -2.0).is_err());
        let logits = [1000.0, 1000.0, f32::NAN];
        assert!((log_probability(&logits, 3, 0, 1.0).unwrap() + 2.0_f32.ln()).abs() < 1.0e-6);
        assert_eq!(log_probability(&logits, 1, 0, 0.5).unwrap(), 0.0);
        assert!(log_probability(&logits, 1, 1, 1.0).is_err());
        assert!(log_probability(&logits, 1, u16::MAX, 1.0).is_err());
        // Idle/Gesture force posture but do not enter its independent distribution.
        assert_eq!(slot(ActionKind::Idle), None);
        assert_eq!(slot(ActionKind::Gesture), None);
        assert_eq!(slot(ActionKind::Rest), Some(4));
    }
}
