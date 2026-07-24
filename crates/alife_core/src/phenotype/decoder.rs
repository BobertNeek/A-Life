//! Contract-only compiler-owned candidate-conditioned decoder plan.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    CandidateActionFamily, CanonicalDigestBuilder, LobeKind, ScaffoldContractError,
    CANDIDATE_FEATURE_COUNT,
};

use super::{BrainPhenotype, CompiledSynapseKind, DecoderHeadKind, MemoryChannelPlan};

const DECODER_SCHEMA_VERSION: u16 = 1;
const DECODER_DOMAIN: &[u8] = b"alife.phenotype.candidate-decoder.v1";
const FAMILY_COUNT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CandidateDecoderFamilyPlan {
    family: CandidateActionFamily,
    bias: f32,
    decoder_synapse_start: u32,
    decoder_synapse_count: u32,
}

impl CandidateDecoderFamilyPlan {
    pub const fn family(&self) -> CandidateActionFamily {
        self.family
    }
    pub const fn bias(&self) -> f32 {
        self.bias
    }
    pub const fn decoder_synapse_start(&self) -> u32 {
        self.decoder_synapse_start
    }
    pub const fn decoder_synapse_count(&self) -> u32 {
        self.decoder_synapse_count
    }
    pub(super) const fn new(
        family: CandidateActionFamily,
        bias: f32,
        decoder_synapse_start: u32,
        decoder_synapse_count: u32,
    ) -> Self {
        Self {
            family,
            bias,
            decoder_synapse_start,
            decoder_synapse_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CandidateDecoderPlan {
    schema_version: u16,
    motor_start: u32,
    motor_width: u16,
    feature_count: u16,
    flattened_input_lane_count: u16,
    memory_channel: Option<MemoryChannelPlan>,
    families: Vec<CandidateDecoderFamilyPlan>,
    canonical_digest: [u64; 4],
}

impl CandidateDecoderPlan {
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn motor_start(&self) -> u32 {
        self.motor_start
    }
    pub const fn motor_width(&self) -> u16 {
        self.motor_width
    }
    pub const fn feature_count(&self) -> u16 {
        self.feature_count
    }
    pub const fn flattened_input_lane_count(&self) -> u16 {
        self.flattened_input_lane_count
    }
    pub const fn memory_channel(&self) -> Option<&MemoryChannelPlan> {
        self.memory_channel.as_ref()
    }
    pub fn families(&self) -> &[CandidateDecoderFamilyPlan] {
        &self.families
    }
    pub fn decoder_synapse_count(&self) -> u32 {
        self.families
            .iter()
            .try_fold(0_u32, |sum, row| sum.checked_add(row.decoder_synapse_count))
            .unwrap_or(u32::MAX)
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }

    pub(super) fn try_new(
        motor_start: u32,
        motor_width: u16,
        feature_count: u16,
        flattened_input_lane_count: u16,
        memory_channel: Option<MemoryChannelPlan>,
        families: Vec<CandidateDecoderFamilyPlan>,
    ) -> Result<Self, ScaffoldContractError> {
        let mut value = Self {
            schema_version: DECODER_SCHEMA_VERSION,
            motor_start,
            motor_width,
            feature_count,
            flattened_input_lane_count,
            memory_channel,
            families,
            canonical_digest: [0; 4],
        };
        value.validate_shape()?;
        value.canonical_digest = value.recompute_digest()?;
        Ok(value)
    }

    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_local()?;
        let capacity = crate::BrainCapacityClass::production_for_id(phenotype.brain_class_id())?;
        let execution = capacity.execution();
        let expected_stride = self
            .memory_channel
            .map_or(u32::from(execution.candidate_feature_count()), |plan| {
                plan.decoder_input_stride()
            });
        if self.feature_count != execution.candidate_feature_count()
            || u32::from(self.flattened_input_lane_count) != expected_stride
            || self.flattened_input_lane_count != phenotype.budgets().global.decoder_input_lanes
            || self.flattened_input_lane_count > execution.max_decoder_input_lanes()
            || self.decoder_synapse_count() > phenotype.budgets().global.action_decoder_synapses
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let motor = phenotype
            .lobe_layout()
            .region(LobeKind::MotorArbitration)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let motor_end = self
            .motor_start
            .checked_add(u32::from(self.motor_width))
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        if !motor.enabled
            || self.motor_width == 0
            || self.motor_start < motor.start
            || motor_end > motor.end()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        for row in &self.families {
            let end = row
                .decoder_synapse_start
                .checked_add(row.decoder_synapse_count)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let slice = phenotype
                .synapses()
                .get(row.decoder_synapse_start as usize..end as usize)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            for synapse in slice {
                let CompiledSynapseKind::Decoder(coordinate) = synapse.kind() else {
                    return Err(ScaffoldContractError::PhenotypeCompile);
                };
                if coordinate.head() != DecoderHeadKind::ActionCandidate
                    || coordinate.family() != row.family
                    || coordinate.input_lane() >= self.feature_count
                    || coordinate.motor_index() >= self.motor_width
                    || synapse.source() != self.motor_start + u32::from(coordinate.motor_index())
                    || synapse.target() != self.motor_start + u32::from(coordinate.motor_index())
                {
                    return Err(ScaffoldContractError::PhenotypeCompile);
                }
            }
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != DECODER_SCHEMA_VERSION
            || self.motor_width == 0
            || self.feature_count != CANDIDATE_FEATURE_COUNT as u16
            || self.flattened_input_lane_count
                != self
                    .memory_channel
                    .map_or(CANDIDATE_FEATURE_COUNT as u16, |plan| {
                        u16::try_from(plan.decoder_input_stride()).unwrap_or(u16::MAX)
                    })
            || self.families.len() != FAMILY_COUNT
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        if let Some(plan) = self.memory_channel {
            plan.validate_contract()?;
        }
        let mut cursor = self
            .families
            .first()
            .map_or(0, |row| row.decoder_synapse_start);
        for (raw, row) in self.families.iter().enumerate() {
            let raw = u8::try_from(raw).map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
            if row.family.raw() != raw
                || row.bias.to_bits() != 0.0_f32.to_bits()
                || row.decoder_synapse_start != cursor
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            CandidateActionFamily::try_from_raw(raw)?;
            cursor = cursor
                .checked_add(row.decoder_synapse_count)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        }
        Ok(())
    }

    pub(super) fn validate_local(&self) -> Result<(), ScaffoldContractError> {
        self.validate_shape()?;
        if self.recompute_digest()? != self.canonical_digest {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(DECODER_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u32(self.motor_start);
        digest.write_u16(self.motor_width);
        digest.write_u16(self.feature_count);
        digest.write_u16(self.flattened_input_lane_count);
        digest.write_bool(self.memory_channel.is_some());
        if let Some(plan) = self.memory_channel {
            for word in plan.canonical_digest() {
                digest.write_u64(word);
            }
        }
        digest.write_sequence_len(self.families.len());
        for row in &self.families {
            digest.write_u8(row.family.raw());
            digest.write_f32(row.bias)?;
            digest.write_u32(row.decoder_synapse_start);
            digest.write_u32(row.decoder_synapse_count);
        }
        Ok(digest.finish256())
    }
}

const AUXILIARY_DECODER_SCHEMA_VERSION: u16 = 1;
const AUXILIARY_DECODER_DOMAIN: &[u8] = b"alife.phenotype.auxiliary-decoder.v1";

/// A bounded decoder head reserved inside the immutable phenotype ABI.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuxiliaryDecoderPlan {
    schema_version: u16,
    head: DecoderHeadKind,
    input_width: u16,
    output_width: u16,
    decoder_synapse_start: u32,
    decoder_synapse_count: u32,
    canonical_digest: [u64; 4],
}

impl AuxiliaryDecoderPlan {
    pub(super) fn try_new(
        head: DecoderHeadKind,
        input_width: u16,
        output_width: u16,
        decoder_synapse_start: u32,
        decoder_synapse_count: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let mut value = Self {
            schema_version: AUXILIARY_DECODER_SCHEMA_VERSION,
            head,
            input_width,
            output_width,
            decoder_synapse_start,
            decoder_synapse_count,
            canonical_digest: [0; 4],
        };
        value.validate_shape()?;
        value.canonical_digest = value.recompute_digest();
        Ok(value)
    }

    pub const fn head(&self) -> DecoderHeadKind {
        self.head
    }
    pub const fn input_width(&self) -> u16 {
        self.input_width
    }
    pub const fn output_width(&self) -> u16 {
        self.output_width
    }
    pub const fn decoder_synapse_start(&self) -> u32 {
        self.decoder_synapse_start
    }
    pub const fn decoder_synapse_count(&self) -> u32 {
        self.decoder_synapse_count
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }

    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_local()?;
        let end = self
            .decoder_synapse_start
            .checked_add(self.decoder_synapse_count)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let slice = phenotype
            .synapses()
            .get(self.decoder_synapse_start as usize..end as usize)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        if slice.iter().any(|synapse| {
            !matches!(
                synapse.kind(),
                CompiledSynapseKind::Decoder(coordinate) if coordinate.head() == self.head
            )
        }) {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let total_for_head = phenotype
            .synapses()
            .iter()
            .filter(|synapse| {
                matches!(
                    synapse.kind(),
                    CompiledSynapseKind::Decoder(coordinate) if coordinate.head() == self.head
                )
            })
            .count();
        if total_for_head
            != usize::try_from(self.decoder_synapse_count)
                .map_err(|_| ScaffoldContractError::PhenotypeCompile)?
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    pub(super) fn validate_local(&self) -> Result<(), ScaffoldContractError> {
        self.validate_shape()?;
        if self.canonical_digest != self.recompute_digest() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != AUXILIARY_DECODER_SCHEMA_VERSION
            || !matches!(
                self.head,
                DecoderHeadKind::SpeechPayload | DecoderHeadKind::MemoryContext
            )
            || self.input_width == 0
            || self.output_width == 0
            || self.decoder_synapse_count
                != u32::from(self.input_width) * u32::from(self.output_width)
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
        let mut d = CanonicalDigestBuilder::new(AUXILIARY_DECODER_DOMAIN);
        d.write_u16(self.schema_version);
        d.write_u32(self.head.raw());
        d.write_u16(self.input_width);
        d.write_u16(self.output_width);
        d.write_u32(self.decoder_synapse_start);
        d.write_u32(self.decoder_synapse_count);
        d.finish256()
    }
}

impl<'de> Deserialize<'de> for AuxiliaryDecoderPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            head: DecoderHeadKind,
            input_width: u16,
            output_width: u16,
            decoder_synapse_start: u32,
            decoder_synapse_count: u32,
            canonical_digest: [u64; 4],
        }
        let w = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: w.schema_version,
            head: w.head,
            input_width: w.input_width,
            output_width: w.output_width,
            decoder_synapse_start: w.decoder_synapse_start,
            decoder_synapse_count: w.decoder_synapse_count,
            canonical_digest: w.canonical_digest,
        };
        value.validate_local().map_err(D::Error::custom)?;
        Ok(value)
    }
}

impl<'de> Deserialize<'de> for CandidateDecoderPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            motor_start: u32,
            motor_width: u16,
            feature_count: u16,
            flattened_input_lane_count: u16,
            memory_channel: Option<MemoryChannelPlan>,
            families: Vec<CandidateDecoderFamilyPlan>,
            canonical_digest: [u64; 4],
        }
        let w = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: w.schema_version,
            motor_start: w.motor_start,
            motor_width: w.motor_width,
            feature_count: w.feature_count,
            flattened_input_lane_count: w.flattened_input_lane_count,
            memory_channel: w.memory_channel,
            families: w.families,
            canonical_digest: w.canonical_digest,
        };
        value.validate_local().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn families(bias: f32) -> Vec<CandidateDecoderFamilyPlan> {
        [
            CandidateActionFamily::Idle,
            CandidateActionFamily::Rest,
            CandidateActionFamily::Inspect,
            CandidateActionFamily::Approach,
            CandidateActionFamily::Avoid,
            CandidateActionFamily::Contact,
            CandidateActionFamily::Ingest,
            CandidateActionFamily::Other,
        ]
        .into_iter()
        .map(|family| CandidateDecoderFamilyPlan::new(family, bias, 100, 0))
        .collect()
    }

    #[test]
    fn slice_a_decoder_shape_requires_exact_widths_and_positive_zero_biases() {
        assert!(CandidateDecoderPlan::try_new(0, 16, 24, 24, None, families(0.0)).is_ok());
        assert!(CandidateDecoderPlan::try_new(0, 16, 23, 24, None, families(0.0)).is_err());
        assert!(CandidateDecoderPlan::try_new(0, 16, 24, 23, None, families(0.0)).is_err());
        assert!(CandidateDecoderPlan::try_new(0, 16, 24, 24, None, families(0.25)).is_err());
        assert!(CandidateDecoderPlan::try_new(0, 16, 24, 24, None, families(-0.0)).is_err());
        let memory = MemoryChannelPlan::try_new_v1(96).unwrap();
        assert!(CandidateDecoderPlan::try_new(0, 16, 24, 36, Some(memory), families(0.0)).is_ok());
    }
}
