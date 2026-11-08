//! Fresh-only detached feasibility inspection. No world execution, source write,
//! live RNG advance, optimizer update or acquired-state reset occurs here.
use super::CpuHostError;
use alife_core::*;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedCandidatePreviewV4 {
    pub index: u16,
    pub action_id: ActionId,
    pub kind: ActionKind,
    pub family: CandidateActionFamily,
    pub intrinsic_basis: f32,
    pub effort: f32,
    pub logit: f32,
    pub representative_probability: f64,
    pub motor_probabilities: [Option<f64>; 6],
    pub inherited_bias: f32,
    pub action_readout: f32,
    pub memory_readout: f32,
    pub cognitive_readout: f32,
    pub motor_source_activation_min: f32,
    pub motor_source_activation_max: f32,
    pub lane23_genetic_aggregate: f32,
    pub lane23_activation_weighted_coefficient: f32,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedFeasibilityPreviewV4 {
    pub tick: Tick,
    pub organism: OrganismId,
    pub world_steps: u64,
    pub live_dispatches: u64,
    pub counterfactual_only: bool,
    pub enabled_motor_channels: Vec<MotorChannel>,
    pub candidates: Vec<CpuNeedCandidatePreviewV4>,
    pub pure_rest_selection_probability: f64,
    pub pure_supported_noop_selection_probability: f64,
    pub input_drives: [f32; 11],
    pub body_energy: f32,
    pub body_health: f32,
    pub biochemical_expression: f32,
    pub planned_brain_work: BrainWorkCounters,
    pub reader_work: Option<BrainWorkCounters>,
    pub binding_work: Option<NativeNeedBindingWorkV2>,
    pub binding_memory_ops: u64,
    pub context_memory_ops: u64,
    pub planned_atp_q16: u32,
    pub available_atp_q16: u32,
    pub detached_neural_activation_min: f32,
    pub detached_neural_activation_max: f32,
}
fn probabilities(logits: &[f32], mask: u32, temperature: f32) -> Vec<f64> {
    if mask == 0 {
        return vec![0.; logits.len()];
    }
    let maximum = logits
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, x)| f64::from(*x))
        .fold(f64::NEG_INFINITY, f64::max);
    let values = logits
        .iter()
        .enumerate()
        .map(|(i, x)| {
            if mask & (1 << i) == 0 {
                0.
            } else {
                ((f64::from(*x) - maximum) / f64::from(temperature)).exp()
            }
        })
        .collect::<Vec<_>>();
    let sum: f64 = values.iter().sum();
    values.into_iter().map(|x| x / sum).collect()
}
pub(super) fn preview_probabilities(
    phenotype: &BrainPhenotype,
    frame: &PerceptionFrame,
    logits: &[f32],
    temperature: f32,
) -> Result<(Vec<f64>, Vec<Vec<f64>>, f64, f64), CpuHostError> {
    // Masks and forced-channel semantics come from the real sampler. This draw
    // is discarded and does not use or advance the host's live sampling state.
    let shape = super::sample_cpu_joint_action(phenotype, frame, logits, 1, 0, temperature)?;
    let global = probabilities(logits, shape.representative_mask, temperature);
    let motor = shape
        .motor_masks
        .iter()
        .map(|&m| probabilities(logits, m, temperature))
        .collect::<Vec<_>>();
    let benign = |c: &ActionCandidate| {
        c.target == ActionTarget::NONE
            && matches!(
                c.action_id,
                alife_world::HeadlessActionIds::NO_LOCOMOTION
                    | alife_world::HeadlessActionIds::NO_MANIPULATION
                    | alife_world::HeadlessActionIds::NO_POSTURE
            )
    };
    let own_noop = |c: &ActionCandidate| {
        c.target == ActionTarget::NONE
            && ((c.kind == ActionKind::Move
                && c.action_id == alife_world::HeadlessActionIds::NO_LOCOMOTION)
                || (c.kind == ActionKind::Hold
                    && c.action_id == alife_world::HeadlessActionIds::NO_POSTURE))
    };
    let mut rest = 0.;
    let mut noop = 0.;
    for (i, c) in frame.candidates().iter().enumerate() {
        let is_rest = c.kind == ActionKind::Rest && c.target == ActionTarget::NONE;
        if !is_rest && !own_noop(c) {
            continue;
        }
        let mut p = global[i];
        for (slot, probabilities) in motor.iter().enumerate() {
            if shape.forced_slots[i] == Some(slot as u8) || shape.motor_masks[slot] == 0 {
                continue;
            }
            p *= frame
                .candidates()
                .iter()
                .enumerate()
                .filter(|(_, c)| benign(c))
                .map(|(j, _)| probabilities[j])
                .sum::<f64>();
        }
        if is_rest {
            rest += p;
        } else {
            noop += p;
        }
    }
    Ok((global, motor, rest, noop))
}
