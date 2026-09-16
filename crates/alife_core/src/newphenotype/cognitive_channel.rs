//! Optional, explicitly enabled cognitive-context decoder extension.

use serde::{Deserialize, Serialize};

use crate::{
    ActionCandidateCreditProfileV1, CanonicalDigestBuilder, FoundationGeneticIdentity,
    ScaffoldContractError,
};

pub const COGNITIVE_CHANNEL_EXTENSION_SCHEMA_VERSION: u16 = 1;
pub const COGNITIVE_CHANNEL_LANE_START: u16 = 36;
pub const COGNITIVE_CHANNEL_LANE_COUNT: u16 = 18;
pub const COGNITIVE_CHANNEL_LANE_END: u16 =
    COGNITIVE_CHANNEL_LANE_START + COGNITIVE_CHANNEL_LANE_COUNT;
pub const COGNITIVE_CHANNEL_FAMILY_COUNT: u8 = 8;
pub const COGNITIVE_CHANNEL_TOTAL_SYNAPSES: u32 = 144;
pub const COGNITIVE_CONTEXT_DECODER_HEAD_RAW: u32 = 4;
pub const COGNITIVE_DECODER_ROLE_RAW: u8 = 3;
pub const COGNITIVE_CHANNEL_REPLAY_CAPTURE_LIMIT: u32 = 64;

const PLAN_DOMAIN: &[u8] = b"alife.phenotype.cognitive-channel-plan.v1";

/// Genetic opt-in for the bounded cognitive-context decoder extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CognitiveChannelExtensionV1 {
    schema_version: u16,
    source_identity: FoundationGeneticIdentity,
    action_candidate_credit_profile: ActionCandidateCreditProfileV1,
}

impl CognitiveChannelExtensionV1 {
    pub fn try_new_v1(
        source_identity: FoundationGeneticIdentity,
        action_candidate_credit_profile: ActionCandidateCreditProfileV1,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            schema_version: COGNITIVE_CHANNEL_EXTENSION_SCHEMA_VERSION,
            source_identity,
            action_candidate_credit_profile,
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn source_identity(&self) -> FoundationGeneticIdentity {
        self.source_identity
    }

    pub const fn action_candidate_credit_profile(&self) -> ActionCandidateCreditProfileV1 {
        self.action_candidate_credit_profile
    }

    pub(crate) fn write_canonical(&self, digest: &mut CanonicalDigestBuilder) {
        digest.write_u16(self.schema_version);
        digest.write_u64(self.source_identity.foundation_id);
        digest.write_u16(self.source_identity.version);
        digest.write_u64(self.source_identity.compatibility_family_id);
        digest.write_u16(self.source_identity.brain_class_id.raw());
        digest.write_u8(self.action_candidate_credit_profile.raw());
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != COGNITIVE_CHANNEL_EXTENSION_SCHEMA_VERSION
            || self.source_identity.foundation_id == 0
            || self.source_identity.version == 0
            || self.source_identity.compatibility_family_id == 0
            || self.source_identity.brain_class_id != crate::BrainCapacityClass::N512_ID
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

/// Compiled layout for the optional 8-family cognitive decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CognitiveChannelPlanV1 {
    schema_version: u16,
    decoder_synapse_start: u32,
    decoder_synapse_count: u32,
    input_lane_start: u16,
    input_lane_count: u16,
    family_count: u8,
    decoder_head_raw: u32,
    projection_role_raw: u8,
    canonical_digest: [u64; 4],
}

impl CognitiveChannelPlanV1 {
    pub(crate) fn try_new_v1(decoder_synapse_start: u32) -> Result<Self, ScaffoldContractError> {
        let mut value = Self {
            schema_version: COGNITIVE_CHANNEL_EXTENSION_SCHEMA_VERSION,
            decoder_synapse_start,
            decoder_synapse_count: COGNITIVE_CHANNEL_TOTAL_SYNAPSES,
            input_lane_start: COGNITIVE_CHANNEL_LANE_START,
            input_lane_count: COGNITIVE_CHANNEL_LANE_COUNT,
            family_count: COGNITIVE_CHANNEL_FAMILY_COUNT,
            decoder_head_raw: COGNITIVE_CONTEXT_DECODER_HEAD_RAW,
            projection_role_raw: COGNITIVE_DECODER_ROLE_RAW,
            canonical_digest: [0; 4],
        };
        value.validate_shape()?;
        value.canonical_digest = value.recompute_digest();
        Ok(value)
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn decoder_synapse_start(&self) -> u32 {
        self.decoder_synapse_start
    }
    pub const fn decoder_synapse_count(&self) -> u32 {
        self.decoder_synapse_count
    }
    pub const fn input_lane_start(&self) -> u16 {
        self.input_lane_start
    }
    pub const fn input_lane_count(&self) -> u16 {
        self.input_lane_count
    }
    pub const fn input_lane_end(&self) -> u16 {
        self.input_lane_start + self.input_lane_count
    }
    pub const fn family_count(&self) -> u8 {
        self.family_count
    }
    pub const fn decoder_head_raw(&self) -> u32 {
        self.decoder_head_raw
    }
    pub const fn projection_role_raw(&self) -> u8 {
        self.projection_role_raw
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.validate_shape()?;
        if self.canonical_digest != self.recompute_digest() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != COGNITIVE_CHANNEL_EXTENSION_SCHEMA_VERSION
            || self.decoder_synapse_count != COGNITIVE_CHANNEL_TOTAL_SYNAPSES
            || self.input_lane_start != COGNITIVE_CHANNEL_LANE_START
            || self.input_lane_count != COGNITIVE_CHANNEL_LANE_COUNT
            || self.family_count != COGNITIVE_CHANNEL_FAMILY_COUNT
            || self.decoder_head_raw != COGNITIVE_CONTEXT_DECODER_HEAD_RAW
            || self.projection_role_raw != COGNITIVE_DECODER_ROLE_RAW
            || self
                .decoder_synapse_start
                .checked_add(self.decoder_synapse_count)
                .is_none()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(PLAN_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u32(self.decoder_synapse_start);
        digest.write_u32(self.decoder_synapse_count);
        digest.write_u16(self.input_lane_start);
        digest.write_u16(self.input_lane_count);
        digest.write_u8(self.family_count);
        digest.write_u32(self.decoder_head_raw);
        digest.write_u8(self.projection_role_raw);
        digest.finish256()
    }
}
