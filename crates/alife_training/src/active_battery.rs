//! Phenotype reconstruction for retained N2048 evidence.

use alife_core::{
    BrainCapacityClass, CreatureGenome, FoundationWeightAsset, PhenotypeCompiler, PhenotypeHash,
    SensorProfile, Tick, Validate,
};

use crate::TrainingError;

pub fn expected_n2048_creature_phenotype_hash(
    creature_genome: &CreatureGenome,
) -> Result<PhenotypeHash, TrainingError> {
    creature_genome.validate_contract()?;
    let foundation = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let expressed = creature_genome.express()?;
    let development = expressed.development_state_at(Tick::new(u64::from(
        expressed.development.maturation_duration_ticks,
    )))?;
    Ok(PhenotypeCompiler::compile_from_foundation_asset(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &foundation,
    )?
    .phenotype_hash())
}
