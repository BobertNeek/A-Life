//! Opt-in scalar reconstruction of captured GPU selector diagnostics.

use super::{
    GpuMemoryContextUpload, GpuSelectorCandidateDiagnostic, GpuSelectorDiagnosticReceipt,
    GpuSelectorExplorationMode, PerceptionFrame, ScaffoldContractError,
};
use alife_core::{
    ActionCandidate, ActionKind, CandidateFeatureVector, CandidateObservationRef, Confidence,
    DurationTicks, NormalizedScalar,
};
use serde::{Deserialize, Serialize};

/// Primitive details missing from the raw-logit diagnostic but included in the
/// canonical, target-bound candidate identity. The digest is verified against
/// these details and the diagnostic's existing action ID, family, and target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSelectorExplorationCandidateIdentity {
    pub digest: [u32; 4],
    pub kind: ActionKind,
    pub min_duration: DurationTicks,
    pub max_duration: DurationTicks,
}

impl GpuSelectorExplorationCandidateIdentity {
    fn from_candidate(candidate: &ActionCandidate) -> Result<Self, ScaffoldContractError> {
        Ok(Self {
            digest: crate::game_candidate_exploration_identity(candidate)?,
            kind: candidate.kind,
            min_duration: candidate.min_duration,
            max_duration: candidate.max_duration,
        })
    }

    fn validate(
        &self,
        candidate: &GpuSelectorCandidateDiagnostic,
    ) -> Result<(), ScaffoldContractError> {
        let source = ActionCandidate::new(
            candidate.candidate_index,
            candidate.action_id,
            self.kind,
            candidate.family,
            CandidateObservationRef::None,
            candidate.target,
            CandidateFeatureVector::zero(),
            Confidence(1.0),
            NormalizedScalar(0.0),
            self.min_duration,
            self.max_duration,
        )?;
        if crate::game_candidate_exploration_identity(&source)? != self.digest {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

/// Bounded source cues and durable draw identity for an opt-in diagnostic.
/// Candidate gaps are the already-uploaded cognitive gap-match lane, not a
/// second memory query or the legacy remembered-outcome-magnitude lane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSelectorAdaptiveExplorationDiagnostic {
    pub organism_id_raw: u64,
    pub world_tick: u64,
    pub focal_novelty: f32,
    pub curiosity: f32,
    pub urgency: f32,
    pub candidate_gap_matches: Vec<f32>,
    pub candidate_identities: Vec<GpuSelectorExplorationCandidateIdentity>,
}

impl GpuSelectorAdaptiveExplorationDiagnostic {
    pub(super) fn from_frame(
        frame: &PerceptionFrame,
        memory_upload: Option<&GpuMemoryContextUpload>,
        decoder_input_stride: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let drives = frame.homeostasis().drives;
        let need = drives
            .hunger
            .max(drives.fatigue.max(drives.temperature_stress));
        let critical_need = ((need - 0.65) / (1.0 - 0.65)).clamp(0.0, 1.0);
        let candidate_gap_matches = (0..frame.candidates().len())
            .map(|index| {
                if decoder_input_stride < 54 {
                    return Ok(0.0);
                }
                let Some(upload) = memory_upload else {
                    return Ok(0.0);
                };
                let row = upload
                    .cognitive_records
                    .get(index)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                if row.candidate_index as usize != index {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
                match row.schema_version {
                    0 => Ok(0.0),
                    1 => Ok(row.values[14]),
                    _ => Err(ScaffoldContractError::InvalidDecisionEvidence),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let diagnostic = Self {
            organism_id_raw: frame.organism_id().raw(),
            world_tick: frame.tick().raw(),
            focal_novelty: frame.sensory().channels.novelty_signal.raw(),
            curiosity: drives.curiosity,
            urgency: drives.fear.max(drives.pain).max(critical_need),
            candidate_gap_matches,
            candidate_identities: frame
                .candidates()
                .iter()
                .map(GpuSelectorExplorationCandidateIdentity::from_candidate)
                .collect::<Result<Vec<_>, _>>()?,
        };
        diagnostic.validate(frame.candidates().len())?;
        Ok(diagnostic)
    }

    fn validate(&self, candidate_count: usize) -> Result<(), ScaffoldContractError> {
        if self.organism_id_raw == 0
            || self.candidate_gap_matches.len() != candidate_count
            || self.candidate_identities.len() != candidate_count
            || [self.focal_novelty, self.curiosity, self.urgency]
                .iter()
                .chain(&self.candidate_gap_matches)
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }

    fn scale(&self, candidate_index: usize) -> f32 {
        0.5 * self
            .focal_novelty
            .max(self.candidate_gap_matches[candidate_index])
            * (0.6 + 0.4 * self.curiosity)
            * (1.0 - self.urgency)
    }

    fn score(&self, candidate: &GpuSelectorCandidateDiagnostic, logit: f32) -> f32 {
        let scale = self.scale(usize::from(candidate.candidate_index));
        if scale == 0.0 {
            return logit;
        }
        logit
            + scale
                * bounded_noise(
                    self.organism_id_raw,
                    self.world_tick,
                    self.candidate_identities[usize::from(candidate.candidate_index)].digest,
                )
    }
}

fn mix(mut value: u32) -> u32 {
    value = (value ^ (value >> 16)).wrapping_mul(0x7feb_352d);
    value = (value ^ (value >> 15)).wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

fn bounded_noise(organism: u64, tick: u64, candidate_identity: [u32; 4]) -> f32 {
    let mut value = 0x6d2b_79f5;
    for word in [
        organism as u32,
        (organism >> 32) as u32,
        tick as u32,
        (tick >> 32) as u32,
        candidate_identity[0],
        candidate_identity[1],
        candidate_identity[2],
        candidate_identity[3],
    ] {
        value = mix(value ^ word);
    }
    let uniform = ((value >> 9) as f32 + 0.5) / 8_388_608.0;
    2.0 * (2.0 * uniform - 1.0)
}

pub(super) fn validate_arbitration(
    receipt: &GpuSelectorDiagnosticReceipt,
) -> Result<(), ScaffoldContractError> {
    match receipt.policy.exploration_mode {
        GpuSelectorExplorationMode::Disabled => {
            if receipt.adaptive_exploration.is_some()
                || receipt.chosen_candidate_index != receipt.argmax_candidate_index
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
            return Ok(());
        }
        GpuSelectorExplorationMode::AdaptiveBoundedUniformV1 => {}
    }
    let evidence = receipt
        .adaptive_exploration
        .as_ref()
        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
    evidence.validate(receipt.candidates.len())?;
    let mut best = None::<(u16, f32)>;
    let mut chosen_score = None;
    for candidate in &receipt.candidates {
        evidence.candidate_identities[usize::from(candidate.candidate_index)]
            .validate(candidate)?;
        let Some(logit) = candidate.final_logit else {
            continue;
        };
        let score = evidence.score(candidate, logit);
        if !score.is_finite() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if candidate.candidate_index == receipt.chosen_candidate_index {
            chosen_score = Some(score);
        }
        if best.is_none_or(|(index, maximum)| {
            score > maximum || (score == maximum && candidate.candidate_index < index)
        }) {
            best = Some((candidate.candidate_index, score));
        }
    }
    let (best_index, best_score) = best.ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
    let chosen_score = chosen_score.ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
    if best_index != receipt.chosen_candidate_index {
        // Preserve exact no-exploration and lowest-index tie semantics. The
        // narrow tolerance covers a GPU's permitted fused multiply/add only.
        let both_cold = evidence.scale(usize::from(best_index)) == 0.0
            && evidence.scale(usize::from(receipt.chosen_candidate_index)) == 0.0;
        let tolerance = 8.0 * f32::EPSILON * best_score.abs().max(chosen_score.abs()).max(1.0);
        if both_cold || best_score == chosen_score || best_score - chosen_score > tolerance {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GpuSelectorBindingIdentity, GpuSelectorCandidateValidity, GpuSelectorPolicyIdentity,
        GPU_SELECTOR_DIAGNOSTIC_SCHEMA_VERSION,
    };
    use alife_core::{
        ActionId, ActionTarget, CandidateActionFamily, PerceptionFrameDigest, PhenotypeHash,
    };

    fn candidate(index: u16, logit: f32) -> GpuSelectorCandidateDiagnostic {
        GpuSelectorCandidateDiagnostic {
            candidate_index: index,
            action_id: ActionId(100 + u32::from(index)),
            family: CandidateActionFamily::Inspect,
            target: ActionTarget::NONE,
            validity: GpuSelectorCandidateValidity::Valid,
            decoder_family_bias: logit,
            binding: (index == 0).then_some(GpuSelectorBindingIdentity {
                decoder_plan_offset: 1,
                decoder_family_offset: 2,
                decoder_family_start: 2,
                decoder_family_count: 1,
                weight_index_start: 3,
                weight_index_count: 0,
                activation_side: 0,
                activation_offset: 4,
                motor_start: 5,
                feature_offset: 6,
                genetic_weight_offset: 7,
                alpha_offset: 8,
                lifetime_weight_offset: 9,
                fast_weight_offset: 10,
            }),
            contributions: Vec::new(),
            pre_context_logit: Some(logit),
            memory_context_delta: Some(0.0),
            final_logit: Some(logit),
        }
    }

    fn receipt(novelty: f32, gap: f32) -> GpuSelectorDiagnosticReceipt {
        let candidates = vec![candidate(0, 0.1), candidate(1, 0.0)];
        let candidate_identities = candidates
            .iter()
            .map(|candidate| {
                let source = ActionCandidate::new(
                    candidate.candidate_index,
                    candidate.action_id,
                    ActionKind::Inspect,
                    candidate.family,
                    CandidateObservationRef::None,
                    candidate.target,
                    CandidateFeatureVector::zero(),
                    Confidence(1.0),
                    NormalizedScalar(0.0),
                    DurationTicks(1),
                    DurationTicks(1),
                )
                .unwrap();
                GpuSelectorExplorationCandidateIdentity::from_candidate(&source).unwrap()
            })
            .collect();
        GpuSelectorDiagnosticReceipt {
            schema_version: GPU_SELECTOR_DIAGNOSTIC_SCHEMA_VERSION,
            frame_digest: PerceptionFrameDigest([1, 2, 3, 4]),
            phenotype_hash: PhenotypeHash([5, 6, 7, 8]),
            dispatch_generation: 9,
            policy: GpuSelectorPolicyIdentity::PRODUCTION_ADAPTIVE_V1,
            adaptive_exploration: Some(GpuSelectorAdaptiveExplorationDiagnostic {
                organism_id_raw: 7,
                world_tick: 12,
                focal_novelty: novelty,
                curiosity: 0.5,
                urgency: 0.0,
                candidate_gap_matches: vec![0.0, gap],
                candidate_identities,
            }),
            requested_candidate_indices: vec![0],
            candidates,
            argmax_candidate_index: 0,
            equal_max_candidate_indices: vec![0],
            chosen_candidate_index: 0,
        }
    }

    #[test]
    fn adaptive_diagnostic_preserves_raw_argmax_and_checks_actual_arbitration() {
        let mut receipt = receipt(1.0, 0.0);
        for tick in 0..100 {
            let evidence = receipt.adaptive_exploration.as_mut().unwrap();
            evidence.world_tick = tick;
            if evidence.score(&receipt.candidates[1], 0.0)
                > evidence.score(&receipt.candidates[0], 0.1)
            {
                receipt.chosen_candidate_index = 1;
                break;
            }
        }
        assert_eq!(receipt.chosen_candidate_index, 1);
        receipt.validate_contract().unwrap();
        assert_eq!(receipt.argmax_candidate_index, 0);
        let mut wrong_winner = receipt.clone();
        wrong_winner.chosen_candidate_index = 0;
        assert!(wrong_winner.validate_contract().is_err());
        let mut wrong_raw_argmax = receipt;
        wrong_raw_argmax.argmax_candidate_index = 1;
        assert!(wrong_raw_argmax.validate_contract().is_err());
    }

    #[test]
    fn cold_urgent_and_legacy_diagnostics_keep_exact_argmax() {
        let mut cold = receipt(0.0, 0.0);
        cold.validate_contract().unwrap();
        cold.candidates[1] = candidate(1, 0.1);
        cold.equal_max_candidate_indices.push(1);
        cold.validate_contract().unwrap();
        cold.chosen_candidate_index = 1;
        assert!(cold.validate_contract().is_err());
        let mut urgent = receipt(1.0, 1.0);
        urgent.adaptive_exploration.as_mut().unwrap().urgency = 1.0;
        urgent.validate_contract().unwrap();
        urgent.chosen_candidate_index = 1;
        assert!(urgent.validate_contract().is_err());
        let mut legacy = receipt(0.0, 0.0);
        legacy.policy = GpuSelectorPolicyIdentity::PRODUCTION_V1;
        legacy.schema_version = legacy.policy.schema_version;
        legacy.adaptive_exploration = None;
        legacy.validate_contract().unwrap();
        legacy.chosen_candidate_index = 1;
        assert!(legacy.validate_contract().is_err());
    }

    #[test]
    fn adaptive_cues_are_bounded_and_required_by_their_policy() {
        let mut missing = receipt(1.0, 0.0);
        missing.adaptive_exploration = None;
        assert!(missing.validate_contract().is_err());
        for value in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            let mut invalid = receipt(0.0, 0.0);
            invalid
                .adaptive_exploration
                .as_mut()
                .unwrap()
                .candidate_gap_matches[1] = value;
            assert!(invalid.validate_contract().is_err());
        }
        let mut invalid_shape = receipt(0.0, 0.0);
        invalid_shape
            .adaptive_exploration
            .as_mut()
            .unwrap()
            .candidate_gap_matches
            .clear();
        assert!(invalid_shape.validate_contract().is_err());
        let mut wrong_policy = receipt(0.0, 0.0);
        wrong_policy.policy = GpuSelectorPolicyIdentity::PRODUCTION_V1;
        wrong_policy.schema_version = wrong_policy.policy.schema_version;
        assert!(wrong_policy.validate_contract().is_err());
    }

    #[test]
    fn adaptive_identity_digest_is_bound_to_target_and_primitive() {
        let valid = receipt(0.0, 0.0);
        valid.validate_contract().unwrap();
        let mut wrong_digest = valid.clone();
        wrong_digest
            .adaptive_exploration
            .as_mut()
            .unwrap()
            .candidate_identities[1]
            .digest[0] ^= 1;
        assert!(wrong_digest.validate_contract().is_err());
        let mut wrong_target = valid.clone();
        wrong_target.candidates[1].target.entity = Some(alife_core::WorldEntityId(42));
        assert!(wrong_target.validate_contract().is_err());
        let mut wrong_duration = valid.clone();
        wrong_duration
            .adaptive_exploration
            .as_mut()
            .unwrap()
            .candidate_identities[1]
            .max_duration = DurationTicks(2);
        assert!(wrong_duration.validate_contract().is_err());
        let mut wrong_kind = valid;
        wrong_kind
            .adaptive_exploration
            .as_mut()
            .unwrap()
            .candidate_identities[1]
            .kind = ActionKind::Look;
        assert!(wrong_kind.validate_contract().is_err());
    }

    #[test]
    fn uniform_draws_use_all_identity_halves_and_stay_strictly_bounded() {
        let identity = [10, 11, 12, 13];
        let base = bounded_noise(7, 12, identity);
        for changed in [
            bounded_noise(7 | (1 << 32), 12, identity),
            bounded_noise(7, 12 | (1 << 32), identity),
            bounded_noise(7, 12, [11, 11, 12, 13]),
            bounded_noise(7, 12, [10, 12, 12, 13]),
            bounded_noise(7, 12, [10, 11, 13, 13]),
            bounded_noise(7, 12, [10, 11, 12, 14]),
        ] {
            assert_ne!(base, changed);
        }
        let mut sum = 0.0;
        for tick in 0..10_000 {
            let sample = bounded_noise(7, tick, identity);
            assert!(sample > -2.0 && sample < 2.0);
            sum += sample;
        }
        assert!(sum.abs() / 10_000.0 < 0.05);
        let low = receipt(0.0, 0.0);
        let high = receipt(1.0, 1.0);
        assert_eq!(low.adaptive_exploration.unwrap().scale(1), 0.0);
        assert!(high.adaptive_exploration.unwrap().scale(1) <= 0.5);
    }
}
