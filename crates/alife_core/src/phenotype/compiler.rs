//! Contract-only compiler entry point; Stage 3 policy fills the validated construction path.

use crate::{
    BrainGenome, DevelopmentState, FoundationAbiBinding, FoundationWeightAsset,
    LegacyNano512CompatibilityAbiDescriptor, LegacyNano512CompatibilityAdmission,
    LegacyNano512CompatibilityReceipt, MigratedN2048FoundationV1Descriptor, ScaffoldContractError,
    SensorProfile,
};

use super::FoundationWeightApplication;
use super::{BrainCapacityClass, BrainPhenotype, PhenotypeCompilerInputs};

pub struct PhenotypeCompiler;

impl PhenotypeCompiler {
    pub fn compile_nano512_action_credit_candidate(
        candidate: &crate::Nano512ActionCreditCandidateV2,
    ) -> Result<(BrainPhenotype, PhenotypeCompilerInputs), ScaffoldContractError> {
        let result = Self::compile_nano512_action_credit_candidate_unchecked(candidate)?;
        result.0.validate_against(&BrainCapacityClass::n512())?;
        Ok(result)
    }

    pub(crate) fn compile_nano512_action_credit_candidate_unchecked(
        candidate: &crate::Nano512ActionCreditCandidateV2,
    ) -> Result<(BrainPhenotype, PhenotypeCompilerInputs), ScaffoldContractError> {
        let asset = candidate.asset()?;
        let (baseline, source_inputs) = Self::compile_nano512_readout_candidate_unchecked(&asset)?;
        let parameters = source_inputs
            .genome()
            .plasticity_parameters()
            .with_action_candidate_credit_profile(candidate.action_profile())?;
        let genome = source_inputs
            .genome()
            .clone()
            .with_plasticity_parameters(parameters)?;
        let inputs = PhenotypeCompilerInputs::try_new_with_foundation_selection(
            genome,
            &BrainCapacityClass::n512(),
            source_inputs.development().clone(),
            candidate.source().sensor_profile(),
            crate::FoundationAbiSelection::Nano512ActionCreditCandidateV2(candidate.clone()),
        )?;
        Ok((baseline.with_action_credit_candidate(&inputs)?, inputs))
    }

    /// Explicit opt-in admission; this does not change New Game defaults.
    pub fn compile_nano512_readout_candidate(
        foundation: &FoundationWeightAsset,
    ) -> Result<(BrainPhenotype, PhenotypeCompilerInputs), ScaffoldContractError> {
        let result = Self::compile_nano512_readout_candidate_unchecked(foundation)?;
        result.0.validate_against(&BrainCapacityClass::n512())?;
        Ok(result)
    }

    pub(crate) fn compile_nano512_readout_candidate_unchecked(
        foundation: &FoundationWeightAsset,
    ) -> Result<(BrainPhenotype, PhenotypeCompilerInputs), ScaffoldContractError> {
        let selection = crate::FoundationAbiSelection::Nano512ReadoutCandidateV1(
            crate::Nano512ReadoutCandidateV1::new(foundation)?,
        );
        let profile = foundation.manifest().sensor_profile();
        let builtin = FoundationWeightAsset::builtin_nano512_v1(profile)?;
        let (baseline, source_inputs, _) =
            Self::compile_fixed_legacy_nano512_compatibility_asset(profile, &builtin)?
                .into_runtime_parts();
        let inputs = PhenotypeCompilerInputs::try_new_with_foundation_selection(
            source_inputs.genome().clone(),
            &BrainCapacityClass::n512(),
            source_inputs.development().clone(),
            profile,
            selection,
        )?;
        Ok((
            baseline.with_nano512_readout_candidate(&inputs, foundation)?,
            inputs,
        ))
    }

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
        if let crate::FoundationAbiSelection::Nano512ActionCreditCandidateV2(candidate) =
            inputs.foundation_abi()
        {
            inputs.validate_against(capacity)?;
            let (phenotype, expected_inputs) =
                Self::compile_nano512_action_credit_candidate(candidate)?;
            if inputs != &expected_inputs {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            return Ok(phenotype);
        }
        if let crate::FoundationAbiSelection::Nano512ReadoutCandidateV1(candidate) =
            inputs.foundation_abi()
        {
            inputs.validate_against(capacity)?;
            let (phenotype, expected_inputs) =
                Self::compile_nano512_readout_candidate(&candidate.asset()?)?;
            if inputs != &expected_inputs {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            return Ok(phenotype);
        }
        if let Some(descriptor) = inputs.legacy_foundation_compatibility_abi() {
            let foundation =
                FoundationWeightAsset::builtin_nano512_v1(descriptor.sensor_profile())?;
            if descriptor.source_weight_asset() != foundation.asset_ref() {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            return match inputs.foundation_weight_application() {
                None => super::construction::compile_with_foundation_asset(
                    inputs,
                    capacity,
                    &foundation,
                ),
                Some(FoundationWeightApplication::Nano512FounderOverlayV1 { seed }) => {
                    super::construction::compile_with_foundation_asset_and_overlay_seed(
                        inputs,
                        capacity,
                        &foundation,
                        seed,
                    )
                }
            };
        }
        if let Some(descriptor) = inputs.migrated_n2048_foundation_v1() {
            let foundation = FoundationWeightAsset::builtin_n2048_v1(descriptor.sensor_profile())?;
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
        if capacity.id() == BrainCapacityClass::N2048_ID
            && MigratedN2048FoundationV1Descriptor::recognizes_source_asset(
                sensor_profile,
                foundation,
            )?
        {
            let descriptor = MigratedN2048FoundationV1Descriptor::for_asset(
                capacity,
                sensor_profile,
                foundation,
            )?;
            let inputs = PhenotypeCompilerInputs::try_new_with_migrated_n2048_foundation_v1(
                genome.clone(),
                capacity,
                development.clone(),
                sensor_profile,
                descriptor,
            )?;
            return super::construction::compile_with_foundation_asset(
                &inputs, capacity, foundation,
            );
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
}
