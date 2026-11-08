//! Candidate-local episodic context lanes owned by the compiled phenotype.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{CanonicalDigestBuilder, ScaffoldContractError, CANDIDATE_FEATURE_COUNT};

const MEMORY_CHANNEL_SCHEMA_VERSION: u16 = 1;
const MEMORY_CHANNEL_DOMAIN: &[u8] = b"alife.phenotype.memory-channel.v1";

/// Immutable mapping from candidate memory rows into decoder-learning lanes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MemoryChannelPlan {
    schema_version: u16,
    target_latent_lane_start: u32,
    family_value_lane_start: u32,
    decoder_input_stride: u32,
    max_candidate_gain: f32,
    memory_decoder_synapse_count: u32,
    canonical_digest: [u64; 4],
}

impl MemoryChannelPlan {
    pub const TARGET_LATENT_WIDTH: u32 = 8;
    pub const FAMILY_VALUE_WIDTH: u32 = 4;
    pub const FAMILY_COUNT: u32 = 8;
    pub const MINIMUM_SYNAPSE_COUNT: u32 =
        Self::FAMILY_COUNT * (Self::TARGET_LATENT_WIDTH + Self::FAMILY_VALUE_WIDTH);

    pub(super) fn try_new_v1(
        memory_decoder_synapse_count: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let mut value = Self {
            schema_version: MEMORY_CHANNEL_SCHEMA_VERSION,
            target_latent_lane_start: CANDIDATE_FEATURE_COUNT as u32,
            family_value_lane_start: CANDIDATE_FEATURE_COUNT as u32 + Self::TARGET_LATENT_WIDTH,
            decoder_input_stride: CANDIDATE_FEATURE_COUNT as u32
                + Self::TARGET_LATENT_WIDTH
                + Self::FAMILY_VALUE_WIDTH,
            max_candidate_gain: 0.5,
            memory_decoder_synapse_count,
            canonical_digest: [0; 4],
        };
        value.validate_shape()?;
        value.canonical_digest = value.recompute_digest()?;
        Ok(value)
    }

    pub(super) fn try_new_memory_gain_candidate(count: u32) -> Result<Self, ScaffoldContractError> {
        let mut value = Self::try_new_v1(count)?;
        value.schema_version = 2;
        value.max_candidate_gain = crate::N2048MemoryGainCandidateV1::MAX_CANDIDATE_GAIN;
        value.validate_shape()?;
        value.canonical_digest = value.recompute_digest()?;
        Ok(value)
    }
    pub(super) fn try_new_grip_reader(count: u32) -> Result<Self, ScaffoldContractError> {
        let mut value = Self::try_new_memory_gain_candidate(count)?;
        value.schema_version = 3;
        value.decoder_input_stride = 54;
        value.validate_shape()?;
        value.canonical_digest = value.recompute_digest()?;
        Ok(value)
    }

    pub const fn is_grip_reader(&self) -> bool {
        self.schema_version == 3
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn target_latent_lane_start(&self) -> u32 {
        self.target_latent_lane_start
    }
    pub const fn family_value_lane_start(&self) -> u32 {
        self.family_value_lane_start
    }
    pub const fn decoder_input_stride(&self) -> u32 {
        self.decoder_input_stride
    }
    pub const fn max_candidate_gain(&self) -> f32 {
        self.max_candidate_gain
    }
    pub const fn memory_decoder_synapse_count(&self) -> u32 {
        self.memory_decoder_synapse_count
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }
    pub const fn synapses_per_family(&self) -> u32 {
        self.memory_decoder_synapse_count / Self::FAMILY_COUNT
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.validate_shape()?;
        if self.canonical_digest != self.recompute_digest()? {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), ScaffoldContractError> {
        let lane_width = Self::TARGET_LATENT_WIDTH + Self::FAMILY_VALUE_WIDTH;
        if !matches!(self.schema_version, MEMORY_CHANNEL_SCHEMA_VERSION | 2 | 3)
            || self.target_latent_lane_start != CANDIDATE_FEATURE_COUNT as u32
            || self.family_value_lane_start
                != self.target_latent_lane_start + Self::TARGET_LATENT_WIDTH
            || self.decoder_input_stride
                != if self.schema_version == 3 {
                    54
                } else {
                    self.target_latent_lane_start + lane_width
                }
            || self.max_candidate_gain.to_bits()
                != (if self.schema_version >= 2 {
                    12.0_f32
                } else {
                    0.5_f32
                })
                .to_bits()
            || (self.schema_version >= 2 && self.memory_decoder_synapse_count != 4096)
            || self.memory_decoder_synapse_count < Self::MINIMUM_SYNAPSE_COUNT
            || !self
                .memory_decoder_synapse_count
                .is_multiple_of(Self::FAMILY_COUNT)
            || self.synapses_per_family() < lane_width
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(MEMORY_CHANNEL_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u32(self.target_latent_lane_start);
        digest.write_u32(self.family_value_lane_start);
        digest.write_u32(self.decoder_input_stride);
        digest.write_f32(self.max_candidate_gain)?;
        digest.write_u32(self.memory_decoder_synapse_count);
        Ok(digest.finish256())
    }
}

impl<'de> Deserialize<'de> for MemoryChannelPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            target_latent_lane_start: u32,
            family_value_lane_start: u32,
            decoder_input_stride: u32,
            max_candidate_gain: f32,
            memory_decoder_synapse_count: u32,
            canonical_digest: [u64; 4],
        }

        let wire = Wire::deserialize(deserializer)?;
        let expected = match wire.schema_version {
            1 => Self::try_new_v1(wire.memory_decoder_synapse_count),
            2 => Self::try_new_memory_gain_candidate(wire.memory_decoder_synapse_count),
            3 => Self::try_new_grip_reader(wire.memory_decoder_synapse_count),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
        .map_err(D::Error::custom)?;
        if wire.schema_version != expected.schema_version
            || wire.target_latent_lane_start != expected.target_latent_lane_start
            || wire.family_value_lane_start != expected.family_value_lane_start
            || wire.decoder_input_stride != expected.decoder_input_stride
            || wire.max_candidate_gain.to_bits() != expected.max_candidate_gain.to_bits()
            || wire.canonical_digest != expected.canonical_digest
        {
            return Err(D::Error::custom(
                "memory-channel ABI does not match its explicit version",
            ));
        }
        Ok(expected)
    }
}
