//! Contract-only compiler-owned candidate-conditioned decoder plan.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    CandidateActionFamily, CanonicalDigestBuilder, LobeKind, ScaffoldContractError,
    CANDIDATE_FEATURE_COUNT,
};

use super::{BrainPhenotype, CompiledSynapseKind, DecoderHeadKind};

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
        families: Vec<CandidateDecoderFamilyPlan>,
    ) -> Result<Self, ScaffoldContractError> {
        let mut value = Self {
            schema_version: DECODER_SCHEMA_VERSION,
            motor_start,
            motor_width,
            feature_count,
            flattened_input_lane_count,
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
        if self.feature_count != execution.candidate_feature_count()
            || self.flattened_input_lane_count != execution.candidate_feature_count()
            || self.flattened_input_lane_count != phenotype.budgets().global.decoder_input_lanes
            || self.flattened_input_lane_count > execution.max_decoder_input_lanes()
            || self.decoder_synapse_count() != phenotype.budgets().global.action_decoder_synapses
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
                    || coordinate.input_lane() >= self.flattened_input_lane_count
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
            || self.flattened_input_lane_count != CANDIDATE_FEATURE_COUNT as u16
            || self.families.len() != FAMILY_COUNT
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
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
        assert!(CandidateDecoderPlan::try_new(0, 16, 24, 24, families(0.0)).is_ok());
        assert!(CandidateDecoderPlan::try_new(0, 16, 23, 24, families(0.0)).is_err());
        assert!(CandidateDecoderPlan::try_new(0, 16, 24, 23, families(0.0)).is_err());
        assert!(CandidateDecoderPlan::try_new(0, 16, 24, 24, families(0.25)).is_err());
        assert!(CandidateDecoderPlan::try_new(0, 16, 24, 24, families(-0.0)).is_err());
    }
}
