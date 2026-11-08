//! Shared fresh-candidate admission and fixed-context helpers, copied from the measured operating-point assay.
use super::assay;
use alife_core::*;
use alife_training::TrainingSequence;
use std::{collections::BTreeSet, error::Error};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub fn bind_candidate(seed: u64, asset: &FoundationWeightAsset) -> Result<BrainPhenotype> {
    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let builtin = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile)?;
    let game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &builtin)?;
    let record = game.world.organism_registry().iter().next().unwrap();
    let genome = record.phenotype().brain_genome.clone();
    let development = alife_runtime::canonical_preparation::foundation_construction_development(
        &genome,
        &BrainCapacityClass::n2048(),
        &record.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    Ok(
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            genome,
            development,
            asset.clone(),
        )?
        .0,
    )
}
pub const CONTEXTS: [&str; 3] = ["nutritious_contact", "aversive_contact", "distant"];
pub fn combined_sequence(
    p: &BrainPhenotype,
    base: [f32; 11],
    changed: [f32; 11],
    d: &DendriticBranchSet,
    cut: bool,
) -> TrainingSequence {
    let mut q = assay::sequence(p, base, changed, CONTEXTS[0], d, cut);
    let mut candidates = vec![];
    for context in CONTEXTS {
        candidates.extend(
            assay::sequence(p, base, base, context, d, false).ticks[0]
                .candidates
                .clone(),
        );
    }
    for tick in &mut q.ticks {
        tick.candidates = candidates.clone();
    }
    q
}
pub fn motor_ids(p: &BrainPhenotype) -> BTreeSet<usize> {
    p.synapses()
        .iter()
        .filter_map(|s| match s.kind() {
            CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate => {
                Some(s.source() as usize)
            }
            _ => None,
        })
        .collect()
}
