//! Contract-only compiler entry point; Stage 3 policy fills the validated construction path.

use crate::{
    BrainGenome, DevelopmentState, FoundationAbiBinding, FoundationWeightAsset,
    ScaffoldContractError, SensorProfile,
};

use super::{BrainCapacityClass, BrainPhenotype, PhenotypeCompilerInputs};

pub struct PhenotypeCompiler;

impl PhenotypeCompiler {
    pub fn compile_validated(
        inputs: &PhenotypeCompilerInputs,
        capacity: &BrainCapacityClass,
    ) -> Result<BrainPhenotype, ScaffoldContractError> {
        if let Some(expected_digest) = inputs.foundation_abi().foundation_payload_digest() {
            let foundation = FoundationWeightAsset::builtin_n2048_v1(inputs.sensor_profile())?;
            if foundation.digest() != expected_digest
                || inputs.foundation_abi().foundation_weight_asset() != Some(foundation.asset_ref())
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            super::construction::compile_with_foundation_asset(inputs, capacity, &foundation)
        } else {
            super::construction::compile(inputs, capacity)
        }
    }

    pub fn compile(
        genome: &BrainGenome,
        capacity: &BrainCapacityClass,
        development: &DevelopmentState,
        sensor_profile: SensorProfile,
    ) -> Result<BrainPhenotype, ScaffoldContractError> {
        let inputs = PhenotypeCompilerInputs::try_new(
            genome.clone(),
            capacity,
            development.clone(),
            sensor_profile,
        )?;
        Self::compile_validated(&inputs, capacity)
    }

    pub fn compile_testing_procedural_baseline(
        genome: &BrainGenome,
        capacity: &BrainCapacityClass,
        development: &DevelopmentState,
        sensor_profile: SensorProfile,
    ) -> Result<BrainPhenotype, ScaffoldContractError> {
        Self::compile(genome, capacity, development, sensor_profile)
    }

    pub fn compile_from_foundation_asset(
        genome: &BrainGenome,
        capacity: &BrainCapacityClass,
        development: &DevelopmentState,
        sensor_profile: SensorProfile,
        foundation: &FoundationWeightAsset,
    ) -> Result<BrainPhenotype, ScaffoldContractError> {
        let foundation_abi =
            FoundationAbiBinding::canonical_for_foundation_asset(capacity, foundation)?;
        let inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
            genome.clone(),
            capacity,
            development.clone(),
            sensor_profile,
            foundation_abi,
        )?;
        super::construction::compile_with_foundation_asset(&inputs, capacity, foundation)
    }
}
