//! Optional terminal observations from an authenticated committed motor channel.
//!
//! This value does not alter an experience patch, ordinary episodic observation,
//! action credit, or rewards. Physiology is the measured joint transaction; it
//! is not an estimate of an isolated channel's reward.

use crate::{
    channel_command_for_action, ActionKind, CandidateActionFamily, CandidateMemoryQueryV2,
    CanonicalDigestBuilder, EpisodicDecisionKeyV2, ExperiencePatch, ExperienceSequenceId,
    FinalizedMemoryRecall, MeasuredChannelObservation, MotorChannel, PhysicalActionOutcome,
    PhysicalContactKind, ScaffoldContractError, Tick, Validate, WorldEntityId,
};
use serde::{Deserialize, Serialize};

const SCHEMA: u16 = 1;
const DOMAIN: &[u8] = b"ALIFE-CHANNEL-TERMINAL-OBSERVATION-V1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Wire")]
pub struct ChannelTerminalObservationV1 {
    schema_version: u16,
    key: EpisodicDecisionKeyV2,
    sequence_id: ExperienceSequenceId,
    channel: MotorChannel,
    target_entity: WorldEntityId,
    outcome_tick: Tick,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    channel_observation: Option<MeasuredChannelObservation>,
    physical: PhysicalActionOutcome,
    measured_hunger_delta: f32,
    danger: f32,
    confidence: f32,
    bundle_digest: [u64; 4],
    patch_digest: [u64; 4],
    canonical_digest: [u64; 4],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema_version: u16,
    key: EpisodicDecisionKeyV2,
    sequence_id: ExperienceSequenceId,
    channel: MotorChannel,
    target_entity: WorldEntityId,
    outcome_tick: Tick,
    #[serde(default)]
    channel_observation: Option<MeasuredChannelObservation>,
    physical: PhysicalActionOutcome,
    measured_hunger_delta: f32,
    danger: f32,
    confidence: f32,
    bundle_digest: [u64; 4],
    patch_digest: [u64; 4],
    canonical_digest: [u64; 4],
}

impl TryFrom<Wire> for ChannelTerminalObservationV1 {
    type Error = ScaffoldContractError;

    fn try_from(wire: Wire) -> Result<Self, Self::Error> {
        let value = Self {
            schema_version: wire.schema_version,
            key: wire.key,
            sequence_id: wire.sequence_id,
            channel: wire.channel,
            target_entity: wire.target_entity,
            outcome_tick: wire.outcome_tick,
            channel_observation: wire.channel_observation,
            physical: wire.physical,
            measured_hunger_delta: wire.measured_hunger_delta,
            danger: wire.danger,
            confidence: wire.confidence,
            bundle_digest: wire.bundle_digest,
            patch_digest: wire.patch_digest,
            canonical_digest: wire.canonical_digest,
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl ChannelTerminalObservationV1 {
    /// Derive one actual consumed observation without changing the representative
    /// decision. The key must be the selected channel candidate's original key
    /// in the supplied finalized native recall, not a reconstructed query.
    ///
    /// Legitimate missing commands or nonconsumed channel outcomes abstain.
    /// A matching committed channel's Consumed physical outcome proves execution;
    /// optional kinematic measurements are retained only when World emits them.
    /// A supplied key/command/target mismatch is invalid evidence. Duplicate
    /// delivery is the owning collection's responsibility, keyed by this sealed
    /// sequence, channel and target; this constructor does not mutate memory.
    pub fn try_from_sealed_patch(
        patch: &ExperiencePatch,
        recall: &FinalizedMemoryRecall,
        key: &EpisodicDecisionKeyV2,
        channel: MotorChannel,
    ) -> Result<Option<Self>, ScaffoldContractError> {
        patch.validate_contract()?;
        let frame = patch.pre_action().perception();
        recall.validate_for_frame(frame)?;
        key.validate_contract()?;
        let query = key.query();
        let index = usize::from(query.candidate_index());
        let candidate = frame
            .candidates()
            .get(index)
            .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
        if recall.candidate_keys().get(index) != Some(key) {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        query.validate_against_frame(frame, candidate)?;

        let bundle = patch
            .selected_bundle()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let Some(command) = bundle.channels.iter().find(|c| c.channel == channel) else {
            return Ok(None);
        };
        let expected = channel_command_for_action(
            channel,
            &candidate.to_command(query.organism_id(), command.confidence)?,
        )?;
        if command != &expected
            || frame
                .candidates()
                .iter()
                .filter(|c| c.action_id == command.primitive && c.target == candidate.target)
                .count()
                != 1
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if query.action_family() != CandidateActionFamily::Ingest {
            return Ok(None);
        }
        if channel != MotorChannel::Manipulation
            || query.action_kind() != ActionKind::Interact
            || query.tracked_object_id().is_none()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let target_entity = candidate
            .target
            .entity
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let joint = patch
            .outcome()
            .joint
            .as_ref()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let Some(physical) = joint.channel_outcomes.iter().find(|p| p.channel == channel) else {
            return Ok(None);
        };
        if physical.physical.contact != PhysicalContactKind::Consumed {
            return Ok(None);
        }
        if physical.physical.target_entity != Some(target_entity) {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let observed = joint
            .channel_observations
            .iter()
            .find(|o| o.channel == channel)
            .copied();
        if observed.is_some_and(|o| !o.executed) {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let physiology = patch
            .outcome()
            .measured_physiology
            .as_ref()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        physiology.validate_contract()?;
        let (valence, pain, disappointment) = super::memory_consequence(patch.outcome());
        let mut observation = Self {
            schema_version: SCHEMA,
            key: key.clone(),
            sequence_id: patch.header().sequence_id,
            channel,
            target_entity,
            outcome_tick: patch.outcome().outcome_tick,
            channel_observation: observed,
            physical: physical.physical,
            measured_hunger_delta: physiology.homeostatic_delta.drives.hunger,
            danger: pain.max(disappointment).max((-valence).max(0.0)),
            confidence: command.confidence.raw(),
            bundle_digest: bundle.canonical_digest()?,
            patch_digest: patch.causal_digest()?,
            canonical_digest: [0; 4],
        };
        observation.canonical_digest = observation.recompute_canonical_digest()?;
        observation.validate_contract()?;
        Ok(Some(observation))
    }

    /// Validate the retained cold proof shape and digest. Native source
    /// authentication occurs in `try_from_sealed_patch`; no sealed patch is
    /// reconstructed or replayed during restoration.
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.key.validate_contract()?;
        self.sequence_id.validate()?;
        self.target_entity.validate()?;
        if let Some(observed) = &self.channel_observation {
            observed.validate_contract()?;
        }
        self.physical.validate_contract()?;
        let query = self.query();
        if self.schema_version != SCHEMA
            || self.channel != MotorChannel::Manipulation
            || self
                .channel_observation
                .is_some_and(|o| o.channel != self.channel || !o.executed)
            || self.physical.contact != PhysicalContactKind::Consumed
            || self.physical.target_entity != Some(self.target_entity)
            || query.action_family() != CandidateActionFamily::Ingest
            || query.action_kind() != ActionKind::Interact
            || query.tracked_object_id().is_none()
            || self.outcome_tick.raw() < query.tick().raw()
            || !self.measured_hunger_delta.is_finite()
            || !(-1.0..=1.0).contains(&self.measured_hunger_delta)
            || !self.danger.is_finite()
            || !(0.0..=1.0).contains(&self.danger)
            || !self.confidence.is_finite()
            || !(0.0..=1.0).contains(&self.confidence)
            || self.bundle_digest == [0; 4]
            || self.patch_digest == [0; 4]
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }

    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(DOMAIN);
        digest.write_u16(self.schema_version);
        for word in self.key.canonical_digest() {
            digest.write_u64(word);
        }
        digest.write_u64(self.sequence_id.raw());
        digest.write_u16(self.channel.canonical_key());
        digest.write_u64(self.target_entity.raw());
        digest.write_u64(self.outcome_tick.raw());
        match self.channel_observation {
            Some(observed) => {
                digest.write_some();
                digest.write_u16(observed.channel.canonical_key());
                digest.write_bool(observed.executed);
                digest.write_f32(observed.measured_intensity.raw())?;
                for v in [
                    observed.displacement.x,
                    observed.displacement.y,
                    observed.displacement.z,
                ] {
                    digest.write_f32(v)?;
                }
            }
            None => digest.write_none(),
        }
        digest.write_u8(match self.physical.contact {
            PhysicalContactKind::None => 0,
            PhysicalContactKind::Touch => 1,
            PhysicalContactKind::Collision => 2,
            PhysicalContactKind::Blocked => 3,
            PhysicalContactKind::Consumed => 4,
            PhysicalContactKind::Moved => 5,
        });
        match self.physical.target_entity {
            Some(target) => {
                digest.write_some();
                digest.write_u64(target.raw());
            }
            None => digest.write_none(),
        }
        for v in [
            self.physical.displacement.x,
            self.physical.displacement.y,
            self.physical.displacement.z,
        ] {
            digest.write_f32(v)?;
        }
        match self.physical.collision_normal {
            Some(normal) => {
                digest.write_some();
                for v in [normal.x, normal.y, normal.z] {
                    digest.write_f32(v)?;
                }
            }
            None => digest.write_none(),
        }
        digest.write_f32(self.physical.energy_cost.raw())?;
        digest.write_f32(self.measured_hunger_delta)?;
        digest.write_f32(self.danger)?;
        digest.write_f32(self.confidence)?;
        for word in self.bundle_digest {
            digest.write_u64(word);
        }
        for word in self.patch_digest {
            digest.write_u64(word);
        }
        Ok(digest.finish256())
    }

    pub const fn key(&self) -> &EpisodicDecisionKeyV2 {
        &self.key
    }
    pub fn query(&self) -> &CandidateMemoryQueryV2 {
        self.key.query()
    }
    pub const fn sequence_id(&self) -> ExperienceSequenceId {
        self.sequence_id
    }
    pub const fn channel(&self) -> MotorChannel {
        self.channel
    }
    pub const fn target_entity(&self) -> WorldEntityId {
        self.target_entity
    }
    pub const fn outcome_tick(&self) -> Tick {
        self.outcome_tick
    }
    pub fn channel_observation(&self) -> Option<&MeasuredChannelObservation> {
        self.channel_observation.as_ref()
    }
    pub const fn physical(&self) -> &PhysicalActionOutcome {
        &self.physical
    }
    pub const fn measured_hunger_delta(&self) -> f32 {
        self.measured_hunger_delta
    }
    pub const fn danger(&self) -> f32 {
        self.danger
    }
    pub const fn confidence(&self) -> f32 {
        self.confidence
    }
    pub const fn bundle_digest(&self) -> [u64; 4] {
        self.bundle_digest
    }
    pub const fn patch_digest(&self) -> [u64; 4] {
        self.patch_digest
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }
}

impl Validate for ChannelTerminalObservationV1 {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        Self::validate_contract(self)
    }
}
