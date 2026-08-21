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
    raw_reward: f32,
    expected_value: f32,
    reward_prediction_error: f32,
    pain: f32,
    injury: f32,
    homeostatic_improvement: f32,
    frustration: f32,
    novelty: f32,
    sensory_prediction_residual: f32,
    social: f32,
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
        let values = [
            reward_prediction_error,
            pain,
            homeostatic_improvement,
            frustration,
            novelty,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if !(-2.0..=2.0).contains(&reward_prediction_error)
            || !(-1.0..=1.0).contains(&homeostatic_improvement)
            || [pain, frustration, novelty]
                .iter()
                .any(|value| !(0.0..=1.0).contains(value))
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let value = (reward_prediction_error - pain + 0.75 * homeostatic_improvement
            - 0.5 * frustration
            + 0.2 * novelty)
            .clamp(-1.0, 1.0);
        Ok(Self {
            raw_reward: reward_prediction_error,
            expected_value: 0.0,
            reward_prediction_error,
            pain,
            injury: 0.0,
            homeostatic_improvement,
            frustration,
            novelty,
            sensory_prediction_residual: 0.0,
            social: 0.0,
            value,
        })
    }

    /// Construct credit from measured reward and a bounded learned expectation.
    pub fn from_measured_components(
        raw_reward: f32,
        expected_value: f32,
        pain: f32,
        injury: f32,
        homeostatic_improvement: f32,
        frustration: f32,
        novelty: f32,
        sensory_prediction_residual: f32,
        social: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let values = [
            raw_reward,
            expected_value,
            pain,
            injury,
            homeostatic_improvement,
            frustration,
            novelty,
            sensory_prediction_residual,
            social,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if !(-1.0..=1.0).contains(&raw_reward)
            || !(-1.0..=1.0).contains(&expected_value)
            || !(-1.0..=1.0).contains(&homeostatic_improvement)
            || !(-1.0..=1.0).contains(&social)
            || [
                pain,
                injury,
                frustration,
                novelty,
                sensory_prediction_residual,
            ]
            .iter()
            .any(|value| !(0.0..=1.0).contains(value))
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let reward_prediction_error = raw_reward - expected_value;
        if !(-2.0..=2.0).contains(&reward_prediction_error) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let value = (reward_prediction_error - pain - injury + 0.75 * homeostatic_improvement
            - 0.5 * frustration
            + 0.2 * novelty
            + 0.2 * social)
            .clamp(-1.0, 1.0);
        Ok(Self {
            raw_reward,
            expected_value,
            reward_prediction_error,
            pain,
            injury,
            homeostatic_improvement,
            frustration,
            novelty,
            sensory_prediction_residual,
            social,
            value,
        })
    }

    pub const fn raw_reward(self) -> f32 {
        self.raw_reward
    }

    pub const fn expected_value(self) -> f32 {
        self.expected_value
    }

    pub const fn reward_prediction_error(self) -> f32 {
        self.reward_prediction_error
    }

    pub const fn pain(self) -> f32 {
        self.pain
    }

    pub const fn injury(self) -> f32 {
        self.injury
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

    pub const fn sensory_prediction_residual(self) -> f32 {
        self.sensory_prediction_residual
    }

    pub const fn social(self) -> f32 {
        self.social
    }

    pub const fn value(self) -> f32 {
        self.value
    }
}

#[derive(Deserialize)]
struct NeuromodulatorSampleWire {
    #[serde(default)]
    raw_reward: f32,
    #[serde(default)]
    expected_value: f32,
    reward_prediction_error: f32,
    pain: f32,
    #[serde(default)]
    injury: f32,
    homeostatic_improvement: f32,
    frustration: f32,
    novelty: f32,
    #[serde(default)]
    sensory_prediction_residual: f32,
    #[serde(default)]
    social: f32,
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
        let legacy = wire.raw_reward == 0.0
            && wire.expected_value == 0.0
            && wire.injury == 0.0
            && wire.sensory_prediction_residual == 0.0
            && wire.social == 0.0;
        let recomputed = if legacy {
            Self::from_components(
                wire.reward_prediction_error,
                wire.pain,
                wire.homeostatic_improvement,
                wire.frustration,
                wire.novelty,
            )
        } else {
            Self::from_measured_components(
                wire.raw_reward,
                wire.expected_value,
                wire.pain,
                wire.injury,
                wire.homeostatic_improvement,
                wire.frustration,
                wire.novelty,
                wire.sensory_prediction_residual,
                wire.social,
            )
        }
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
        if outcome.measured_biology.is_none() {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        let modulator = NeuromodulatorSample::from_measured_components(
            outcome.raw_reward().raw(),
            outcome.expected_value.raw(),
            outcome.pain_delta.raw(),
            outcome.injury_delta.raw(),
            homeostatic_improvement(outcome),
            outcome.frustration_delta.raw(),
            outcome.novelty.raw(),
            outcome.sensory_prediction_residual().raw(),
            outcome.social_outcome.raw(),
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
    let body = outcome.body_delta;
    // Lower aversive drives and improved canonical body state are improvements.
    // Pain and injury remain separate negative factors in the joint modulator.
    let drive_signal =
        -drives.hunger - drives.fatigue - drives.fear - drives.pain - drives.loneliness
            + drives.brain_atp
            - drives.temperature_stress;
    let body_signal =
        body.energy.raw() + body.health.raw() - body.injury.raw() - body.temperature_stress.raw();
    ((drive_signal + body_signal) / 11.0).clamp(-1.0, 1.0)
}

/// Validate a packet's learning ABI before backend upload.
pub fn validate_outcome_credit_schema(
    packet: &OutcomeCreditPacket,
) -> Result<(), ScaffoldContractError> {
    require_current_version(SchemaKind::Learning, packet.schema_version)
}
