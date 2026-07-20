//! Patch-gated three-factor learning contracts.
//!
//! This module owns only engine-independent evidence and replay contracts. GPU
//! storage and WGSL updates live in `alife_gpu_backend`; no CPU neural update is
//! performed here.

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use crate::{
    require_current_version, ActionId, CandidateActionFamily, CandidateFeatureDigest,
    ExperiencePatch, ExperiencePatchPhase, ExperienceSequenceId, OrganismId, PerceptionFrameDigest,
    PhenotypeHash, PreActionBrainEvidence, ScaffoldContractError, SchemaKind, SchemaVersions, Tick,
    Validate,
};

/// Bounded, auditable components of the third factor applied after an outcome.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct NeuromodulatorSample {
    reward_prediction_error: f32,
    pain: f32,
    homeostatic_improvement: f32,
    frustration: f32,
    novelty: f32,
    value: f32,
}

impl NeuromodulatorSample {
    /// Construct a sample with the canonical bounded three-factor formula.
    pub fn from_components(
        reward_prediction_error: f32,
        pain: f32,
        homeostatic_improvement: f32,
        frustration: f32,
        novelty: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let components = [
            reward_prediction_error,
            pain,
            homeostatic_improvement,
            frustration,
            novelty,
        ];
        if components.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if components.iter().any(|value| !(-1.0..=1.0).contains(value)) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let value = (reward_prediction_error - pain + 0.75 * homeostatic_improvement
            - 0.5 * frustration
            + 0.2 * novelty)
            .clamp(-1.0, 1.0);
        Ok(Self {
            reward_prediction_error,
            pain,
            homeostatic_improvement,
            frustration,
            novelty,
            value,
        })
    }

    pub const fn reward_prediction_error(self) -> f32 {
        self.reward_prediction_error
    }

    pub const fn pain(self) -> f32 {
        self.pain
    }

    pub const fn homeostatic_improvement(self) -> f32 {
        self.homeostatic_improvement
    }

    pub const fn frustration(self) -> f32 {
        self.frustration
    }

    pub const fn novelty(self) -> f32 {
        self.novelty
    }

    pub const fn value(self) -> f32 {
        self.value
    }
}

#[derive(Deserialize)]
struct NeuromodulatorSampleWire {
    reward_prediction_error: f32,
    pain: f32,
    homeostatic_improvement: f32,
    frustration: f32,
    novelty: f32,
    value: f32,
}

impl<'de> Deserialize<'de> for NeuromodulatorSample {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = NeuromodulatorSampleWire::deserialize(deserializer)?;
        if !wire.value.is_finite() || !(-1.0..=1.0).contains(&wire.value) {
            return Err(D::Error::custom("invalid serialized neuromodulator value"));
        }
        let recomputed = Self::from_components(
            wire.reward_prediction_error,
            wire.pain,
            wire.homeostatic_improvement,
            wire.frustration,
            wire.novelty,
        )
        .map_err(D::Error::custom)?;
        if recomputed.value.to_bits() != wire.value.to_bits() {
            return Err(D::Error::custom(
                "serialized neuromodulator value does not match its components",
            ));
        }
        Ok(recomputed)
    }
}

/// Compact outcome credit derived exclusively from one sealed neural patch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutcomeCreditPacket {
    schema_version: u16,
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
    sequence_id: ExperienceSequenceId,
    originating_tick: Tick,
    outcome_tick: Tick,
    frame_digest: PerceptionFrameDigest,
    active_activation_side: u8,
    selected_candidate: u16,
    selected_family: CandidateActionFamily,
    selected_action: ActionId,
    candidate_feature_digest: CandidateFeatureDigest,
    dispatch_generation: u64,
    modulator: NeuromodulatorSample,
}

impl OutcomeCreditPacket {
    /// Derive outcome credit from a validated, sealed GPU decision patch.
    pub fn from_sealed_patch(patch: &ExperiencePatch) -> Result<Self, ScaffoldContractError> {
        patch
            .validate_contract()
            .map_err(|_| ScaffoldContractError::LearningEvidenceMismatch)?;
        if patch.header().phase != ExperiencePatchPhase::Sealed {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        let evidence = patch
            .decision()
            .neural_evidence()
            .map_err(|_| ScaffoldContractError::LearningEvidenceMismatch)?;
        let pre_action_hash = match patch.pre_action().brain_evidence {
            PreActionBrainEvidence::NeuralClosedLoopGpu {
                phenotype_hash,
                frame_digest,
                ..
            } if frame_digest == evidence.frame_digest => phenotype_hash,
            _ => return Err(ScaffoldContractError::LearningEvidenceMismatch),
        };
        if pre_action_hash != evidence.phenotype_hash
            || patch.header().organism_id != patch.decision().organism_id
            || patch.header().organism_id != patch.outcome().organism_id
            || patch.header().sequence_id != patch.decision().sequence_id
            || patch.header().sequence_id != patch.outcome().sequence_id
            || patch.decision().selected_action.action_id != evidence.action_id
            || evidence.active_activation_side > 1
            || evidence.dispatch_generation == 0
        {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }

        let outcome = patch.outcome();
        let modulator = NeuromodulatorSample::from_components(
            outcome.reward_valence.raw(),
            outcome.pain_delta.raw(),
            homeostatic_improvement(outcome),
            outcome.frustration_delta.raw(),
            outcome.prediction_error.raw(),
        )?;
        Ok(Self {
            schema_version: SchemaVersions::CURRENT.learning.raw(),
            organism_id: patch.header().organism_id,
            phenotype_hash: evidence.phenotype_hash,
            sequence_id: patch.header().sequence_id,
            originating_tick: patch.header().world_tick,
            outcome_tick: outcome.outcome_tick,
            frame_digest: evidence.frame_digest,
            active_activation_side: evidence.active_activation_side,
            selected_candidate: evidence.candidate_index,
            selected_family: evidence.action_family,
            selected_action: evidence.action_id,
            candidate_feature_digest: evidence.candidate_feature_digest,
            dispatch_generation: evidence.dispatch_generation,
            modulator,
        })
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn phenotype_hash(&self) -> PhenotypeHash {
        self.phenotype_hash
    }

    pub const fn sequence_id(&self) -> ExperienceSequenceId {
        self.sequence_id
    }

    pub const fn originating_tick(&self) -> Tick {
        self.originating_tick
    }

    pub const fn outcome_tick(&self) -> Tick {
        self.outcome_tick
    }

    pub const fn frame_digest(&self) -> PerceptionFrameDigest {
        self.frame_digest
    }

    pub const fn active_activation_side(&self) -> u8 {
        self.active_activation_side
    }

    pub const fn selected_candidate(&self) -> u16 {
        self.selected_candidate
    }

    pub const fn selected_family(&self) -> CandidateActionFamily {
        self.selected_family
    }

    pub const fn selected_action(&self) -> ActionId {
        self.selected_action
    }

    pub const fn candidate_feature_digest(&self) -> CandidateFeatureDigest {
        self.candidate_feature_digest
    }

    pub const fn dispatch_generation(&self) -> u64 {
        self.dispatch_generation
    }

    pub const fn modulator(&self) -> NeuromodulatorSample {
        self.modulator
    }

    pub const fn replay_key(&self) -> OutcomeCreditReplayKey {
        OutcomeCreditReplayKey {
            organism_id: self.organism_id,
            phenotype_hash: self.phenotype_hash,
            sequence_id: self.sequence_id,
        }
    }
}

/// Stable replay identity for an outcome-credit application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutcomeCreditReplayKey {
    pub organism_id: OrganismId,
    pub phenotype_hash: PhenotypeHash,
    pub sequence_id: ExperienceSequenceId,
}

/// Single-use authorization returned by a read-only sequence preflight.
#[derive(Debug, PartialEq, Eq)]
pub struct LearningCommitToken {
    expected_previous: Option<OutcomeCreditReplayKey>,
    next: OutcomeCreditReplayKey,
}

/// Organism- and phenotype-bound replay guard for committed GPU learning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningSequenceGuard {
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
    last_committed: Option<OutcomeCreditReplayKey>,
}

impl LearningSequenceGuard {
    pub const fn new(organism_id: OrganismId, phenotype_hash: PhenotypeHash) -> Self {
        Self {
            organism_id,
            phenotype_hash,
            last_committed: None,
        }
    }

    pub fn restore_validated(
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
        last_committed: Option<OutcomeCreditReplayKey>,
    ) -> Result<Self, ScaffoldContractError> {
        organism_id.validate()?;
        if let Some(last) = last_committed {
            last.sequence_id.validate()?;
            if last.organism_id != organism_id || last.phenotype_hash != phenotype_hash {
                return Err(ScaffoldContractError::LearningEvidenceMismatch);
            }
        }
        Ok(Self {
            organism_id,
            phenotype_hash,
            last_committed,
        })
    }

    pub const fn last_committed(&self) -> Option<OutcomeCreditReplayKey> {
        self.last_committed
    }

    pub fn validate_next(
        &self,
        next: OutcomeCreditReplayKey,
    ) -> Result<LearningCommitToken, ScaffoldContractError> {
        self.organism_id.validate()?;
        next.organism_id.validate()?;
        next.sequence_id.validate()?;
        if next.organism_id != self.organism_id || next.phenotype_hash != self.phenotype_hash {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        if self
            .last_committed
            .is_some_and(|last| next.sequence_id.raw() <= last.sequence_id.raw())
        {
            return Err(ScaffoldContractError::LearningReplayRejected);
        }
        Ok(LearningCommitToken {
            expected_previous: self.last_committed,
            next,
        })
    }

    pub fn commit_validated(
        &mut self,
        token: LearningCommitToken,
    ) -> Result<(), ScaffoldContractError> {
        if token.expected_previous != self.last_committed
            || token.next.organism_id != self.organism_id
            || token.next.phenotype_hash != self.phenotype_hash
            || self
                .last_committed
                .is_some_and(|last| token.next.sequence_id.raw() <= last.sequence_id.raw())
        {
            return Err(ScaffoldContractError::LearningReplayRejected);
        }
        token.next.sequence_id.validate()?;
        self.last_committed = Some(token.next);
        Ok(())
    }
}

/// Production waking fast weights are immediately effective three-factor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FastWeightSemantics {
    ImmediateThreeFactor,
}

fn homeostatic_improvement(outcome: &crate::PostActionOutcome) -> f32 {
    let drives = outcome.homeostatic_delta.drives;
    // Lower aversive drives and higher ATP/energy are improvements. Curiosity,
    // reproductive drive, pain, and extension channels are excluded here:
    // curiosity is represented by novelty, pain has its own negative factor,
    // and the remaining channels have no universal good direction.
    let oriented_sum = -drives.hunger - drives.fatigue - drives.fear - drives.loneliness
        + drives.brain_atp
        - drives.temperature_stress
        + outcome.energy_delta.raw();
    (oriented_sum / 7.0).clamp(-1.0, 1.0)
}

/// Validate a packet's learning ABI before backend upload.
pub fn validate_outcome_credit_schema(
    packet: &OutcomeCreditPacket,
) -> Result<(), ScaffoldContractError> {
    require_current_version(SchemaKind::Learning, packet.schema_version)
}
