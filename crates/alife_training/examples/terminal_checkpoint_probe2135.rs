// Local full-host checkpoint validation probe; no World stepping or state edits.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{AdamWConfig, StageTrainableMask, cpu::*};
use alife_world::*;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn config(asset: &FoundationWeightAsset) -> Result<CanonicalNewGameConfig> {
    let mut c = CanonicalNewGameConfig::phase3(2026111701, 1)?;
    c.brain_class = BrainScaleTier::Standard2048;
    c.sensor_profile = asset.manifest().sensor_profile();
    Ok(c)
}
// Load the exported construction genome directly. No new chromosome mutation,
// weight rebinding or reconstruction of a different founder is performed.
fn prepare_parent(
    asset: &FoundationWeightAsset,
    genome: &CreatureGenome,
) -> Result<(BrainPhenotype, CanonicalNewGame)> {
    let embedded = genome
        .n2048_foundation_candidate
        .as_ref()
        .ok_or("parent genome lacks foundation")?;
    if embedded.encode_canonical()? != asset.encode_canonical()? {
        return Err("genome/asset pair mismatch".into());
    }
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config(asset)?, asset)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let expressed = genome.express()?;
    let dev = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO)?,
    )?;
    let (compiled, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome.clone(),
        dev,
        asset.clone(),
    )?;
    if !compiled
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits())
    {
        return Err("compiled parent weights changed".into());
    }
    game.world
        .replace_organism_record_exact(WorldOrganismRecord::new(
            record.organism_id(),
            record.world_entity_id(),
            genome.clone(),
            expressed,
            *record.biochemistry(),
            record.birth_tick(),
        )?)?;
    game.creatures[0].weights.genetic_fixed_digest =
        PortableAssetDigest::for_bytes(&serde_json::to_vec(genome)?).0;
    Ok((compiled, game))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(&args[1])?)?;
    let genome: CreatureGenome = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    let (p, _) = prepare_parent(&asset, &genome)?;
    let mask = StageTrainableMask::recurrent_only(&p)?;
    let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default())?;
    let bytes = std::fs::read(&args[3])?;
    let cp: CpuCanonicalCheckpoint = serde_json::from_slice(&bytes)?;
    let schema = cp.schema_version;
    let host = CpuCanonicalHost::from_checkpoint(cp, &trainer)?;
    let roundtrip = serde_json::to_vec(&host.checkpoint(&trainer)?)?;
    assert_eq!(bytes, roundtrip);
    println!("{}", serde_json::json!({"schema":schema,"full_host_cold_exact":true}));
    Ok(())
}
