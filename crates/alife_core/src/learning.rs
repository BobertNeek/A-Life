//! Patch-gated three-factor learning contracts.
//!
//! This module owns only engine-independent evidence and replay contracts. GPU
//! storage and WGSL updates live in `alife_gpu_backend`; no CPU neural update is
//! performed here.

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use crate::{
    require_current_version, ActionId, CandidateActionFamily, CandidateFeatureDigest,
    ExperiencePatch, ExperiencePatchPhase, ExperienceSequenceId, NeuralReceptorClass,
    NeuralReceptorFrame, OrganismId, PerceptionFrameDigest, PhenotypeHash, PreActionBrainEvidence,
    ScaffoldContractError, SchemaKind, SchemaVersions, Tick, Validate,
};

pub const NEUROMODULATORY_LANE_COUNT: usize = 8;

/// Bounded biological and predictive evidence. No lane is a finished reward.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(transparent)]
pub struct NeuromodulatoryFrame {
    lanes: [f32; NEUROMODULATORY_LANE_COUNT],
}

impl NeuromodulatoryFrame {
    pub fn try_new(
        lanes: [f32; NEUROMODULATORY_LANE_COUNT],
    ) -> Result<Self, ScaffoldContractError> {
        if lanes.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if lanes.iter().any(|value| !(-1.0..=1.0).contains(value)) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self { lanes })
    }

    pub const fn lanes(&self) -> &[f32; NEUROMODULATORY_LANE_COUNT] {
        &self.lanes
    }
}

impl<'de> Deserialize<'de> for NeuromodulatoryFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let lanes = <[f32; NEUROMODULATORY_LANE_COUNT]>::deserialize(deserializer)?;
        Self::try_new(lanes).map_err(D::Error::custom)
    }
}

/// Heritable local projection from the shared lane frame into one third factor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PlasticityReceptorProfile {
    weights: [f32; NEUROMODULATORY_LANE_COUNT],
}

impl PlasticityReceptorProfile {
    pub fn try_new(
        weights: [f32; NEUROMODULATORY_LANE_COUNT],
    ) -> Result<Self, ScaffoldContractError> {
        if weights.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if weights.iter().any(|value| !(-2.0..=2.0).contains(value)) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self { weights })
    }

    pub const fn weights(&self) -> &[f32; NEUROMODULATORY_LANE_COUNT] {
        &self.weights
    }

    pub fn project(&self, frame: &NeuromodulatoryFrame) -> Result<f32, ScaffoldContractError> {
        let scale = self.weights.iter().map(|weight| weight.abs()).sum::<f32>();
        if !scale.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if scale == 0.0 {
            return Ok(0.0);
        }
        let value = self
            .weights
            .iter()
            .zip(frame.lanes)
            .map(|(weight, lane)| weight * lane)
            .sum::<f32>()
            / scale;
        if !value.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        Ok(value.clamp(-1.0, 1.0))
    }
}

impl<'de> Deserialize<'de> for PlasticityReceptorProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let weights = <[f32; NEUROMODULATORY_LANE_COUNT]>::deserialize(deserializer)?;
        Self::try_new(weights).map_err(D::Error::custom)
    }
}

/// Compatibility name for the bounded outcome lane frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct NeuromodulatorSample {
    prediction_residual: f32,
    pain: f32,
    homeostatic_improvement: f32,
    frustration: f32,
    novelty: f32,
    frame: NeuromodulatoryFrame,
}

impl NeuromodulatorSample {
    pub fn from_frame(frame: NeuromodulatoryFrame) -> Self {
        let lanes = frame.lanes();
        Self {
            prediction_residual: lanes[0],
            pain: lanes[1],
            homeostatic_improvement: lanes[2],
            frustration: lanes[3],
            novelty: lanes[4],
            frame,
        }
    }

    /// Construct a sample with the canonical bounded three-factor formula.
    pub fn from_components(
        prediction_residual: f32,
        pain: f32,
        homeostatic_improvement: f32,
        frustration: f32,
        novelty: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let components = [
            prediction_residual,
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
        let frame = NeuromodulatoryFrame::try_new([
            prediction_residual,
            pain,
            homeostatic_improvement,
            frustration,
            novelty,
            0.0,
            0.0,
            0.0,
        ])?;
        Ok(Self::from_frame(frame))
    }

    pub const fn prediction_residual(self) -> f32 {
        self.prediction_residual
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

    pub const fn frame(self) -> NeuromodulatoryFrame {
        self.frame
    }

    pub fn with_biochemical_receptors(
        mut self,
        receptors: &NeuralReceptorFrame,
    ) -> Result<Self, ScaffoldContractError> {
        receptors.validate_contract()?;
        let mut lanes = *self.frame.lanes();
        lanes[6] = receptors.activation_for(NeuralReceptorClass::PlasticityAppetitive);
        lanes[7] = receptors.activation_for(NeuralReceptorClass::PlasticityAversive);
        self.frame = NeuromodulatoryFrame::try_new(lanes)?;
        Ok(self)
    }
}

#[derive(Deserialize)]
struct NeuromodulatorSampleWire {
    #[serde(alias = "reward_prediction_error")]
    prediction_residual: f32,
    pain: f32,
    homeostatic_improvement: f32,
    frustration: f32,
    novelty: f32,
    frame: NeuromodulatoryFrame,
}

impl<'de> Deserialize<'de> for NeuromodulatorSample {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = NeuromodulatorSampleWire::deserialize(deserializer)?;
        let sample = Self::from_frame(wire.frame);
        if [
            sample.prediction_residual.to_bits() == wire.prediction_residual.to_bits(),
            sample.pain.to_bits() == wire.pain.to_bits(),
            sample.homeostatic_improvement.to_bits() == wire.homeostatic_improvement.to_bits(),
            sample.frustration.to_bits() == wire.frustration.to_bits(),
            sample.novelty.to_bits() == wire.novelty.to_bits(),
        ]
        .into_iter()
        .all(|matches| matches)
        {
            Ok(sample)
        } else {
            Err(D::Error::custom(
                "neuromodulatory component projections do not match the authoritative frame",
            ))
        }
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
        let physiology = outcome
            .measured_physiology
            .as_ref()
            .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
        let modulator = NeuromodulatorSample::from_components(
            outcome.prediction_error.raw(),
            physiology.pain_delta.raw().max(0.0),
            homeostatic_improvement(physiology),
            outcome.frustration_delta.raw(),
            0.0,
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

    pub fn with_biochemical_receptors(
        mut self,
        receptors: &NeuralReceptorFrame,
    ) -> Result<Self, ScaffoldContractError> {
        if receptors.source_tick.raw() < self.originating_tick.raw()
            || receptors.source_tick.raw() > self.outcome_tick.raw()
        {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        self.modulator = self.modulator.with_biochemical_receptors(receptors)?;
        Ok(self)
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

fn homeostatic_improvement(physiology: &crate::MeasuredPhysiologyTransition) -> f32 {
    let drives = physiology.homeostatic_delta.drives;
    // Lower aversive drives and higher ATP/energy are improvements. Curiosity,
    // reproductive drive, pain, and extension channels are excluded here:
    // curiosity is represented by novelty, pain has its own negative factor,
    // and the remaining channels have no universal good direction.
    let oriented_sum = -drives.hunger - drives.fatigue - drives.fear - drives.loneliness
        + drives.brain_atp
        - drives.temperature_stress
        + physiology.energy_delta.raw();
    (oriented_sum / 7.0).clamp(-1.0, 1.0)
}

/// Validate a packet's learning ABI before backend upload.
pub fn validate_outcome_credit_schema(
    packet: &OutcomeCreditPacket,
) -> Result<(), ScaffoldContractError> {
    require_current_version(SchemaKind::Learning, packet.schema_version)
}
