//! Contract-only compiler entry point; Stage 3 policy fills the validated construction path.

use crate::{BrainGenome, DevelopmentState, ScaffoldContractError, SensorProfile};

use super::{BrainCapacityClass, BrainPhenotype, PhenotypeCompilerInputs};

pub struct PhenotypeCompiler;

impl PhenotypeCompiler {
    pub fn compile_validated(
        inputs: &PhenotypeCompilerInputs,
        capacity: &BrainCapacityClass,
    ) -> Result<BrainPhenotype, ScaffoldContractError> {
        super::construction::compile(inputs, capacity)
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
}
