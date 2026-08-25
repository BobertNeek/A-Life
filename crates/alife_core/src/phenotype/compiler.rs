//! Contract-only compiler entry point; Stage 3 policy fills the validated construction path.

use crate::{
    BrainGenome, DevelopmentState, FoundationAbiBinding, FoundationWeightAsset,
    LegacyNano512CompatibilityAbiDescriptor, LegacyNano512CompatibilityAdmission,
    LegacyNano512CompatibilityReceipt, ScaffoldContractError, SensorProfile,
};

use super::{BrainCapacityClass, BrainPhenotype, PhenotypeCompilerInputs};

pub struct PhenotypeCompiler;

impl PhenotypeCompiler {
    pub fn compile_fixed_legacy_nano512_compatibility_asset(
        sensor_profile: SensorProfile,
        foundation: &FoundationWeightAsset,
    ) -> Result<LegacyNano512CompatibilityAdmission, ScaffoldContractError> {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(crate::LEGACY_NANO512_V1_COORDINATE_SEED, capacity.id());
        let development = DevelopmentState::new(
            genome.id,
            crate::Tick::ZERO,
            crate::NormalizedScalar::new(1.0)?,
        );
        Self::compile_from_legacy_nano512_compatibility_asset(
            &genome,
            &capacity,
            &development,
            sensor_profile,
            foundation,
        )
    }

    pub fn compile_validated(
        inputs: &PhenotypeCompilerInputs,
        capacity: &BrainCapacityClass,
    ) -> Result<BrainPhenotype, ScaffoldContractError> {
        if let Some(descriptor) = inputs.legacy_foundation_compatibility_abi() {
            let foundation =
                FoundationWeightAsset::builtin_nano512_v1(descriptor.sensor_profile())?;
            if descriptor.source_weight_asset() != foundation.asset_ref() {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            return super::construction::compile_with_foundation_asset(
                inputs,
                capacity,
                &foundation,
            );
        }
        let foundation_abi = inputs
            .foundation_abi()
            .canonical_v2()
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        if let Some(expected_digest) = foundation_abi.foundation_payload_digest() {
            let foundation = match capacity.id() {
                BrainCapacityClass::N512_ID => {
                    FoundationWeightAsset::builtin_nano512_v1(inputs.sensor_profile())?
                }
                BrainCapacityClass::N2048_ID => {
                    FoundationWeightAsset::builtin_n2048_v1(inputs.sensor_profile())?
                }
                _ => return Err(ScaffoldContractError::PhenotypeCompile),
            };
            if foundation.digest() != expected_digest
                || foundation_abi.foundation_weight_asset() != Some(foundation.asset_ref())
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

    pub fn compile_from_legacy_nano512_compatibility_asset(
        genome: &BrainGenome,
        capacity: &BrainCapacityClass,
        development: &DevelopmentState,
        sensor_profile: SensorProfile,
        foundation: &FoundationWeightAsset,
    ) -> Result<LegacyNano512CompatibilityAdmission, ScaffoldContractError> {
        let descriptor = LegacyNano512CompatibilityAbiDescriptor::for_asset(
            capacity,
            sensor_profile,
            foundation,
        )?;
        let inputs = PhenotypeCompilerInputs::try_new_with_legacy_foundation_compatibility_abi(
            genome.clone(),
            capacity,
            development.clone(),
            sensor_profile,
            descriptor,
        )?;
        let phenotype =
            super::construction::compile_with_foundation_asset(&inputs, capacity, foundation)?;
        let descriptor = phenotype
            .legacy_foundation_compatibility_abi()
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let receipt = LegacyNano512CompatibilityReceipt::new(descriptor, &phenotype)?;
        Ok(LegacyNano512CompatibilityAdmission::new(
            phenotype, inputs, receipt,
        ))
    }

    pub(super) fn compile_from_foundation_asset_with_overlay_seed(
        genome: &BrainGenome,
        capacity: &BrainCapacityClass,
        development: &DevelopmentState,
        sensor_profile: SensorProfile,
        foundation: &FoundationWeightAsset,
        overlay_seed: u64,
    ) -> Result<BrainPhenotype, ScaffoldContractError> {
        if overlay_seed == 0 {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let foundation_abi =
            FoundationAbiBinding::canonical_for_foundation_asset(capacity, foundation)?;
        let inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
            genome.clone(),
            capacity,
            development.clone(),
            sensor_profile,
            foundation_abi,
        )?;
        super::construction::compile_with_foundation_asset_and_overlay_seed(
            &inputs,
            capacity,
            foundation,
            overlay_seed,
        )
    }
}
