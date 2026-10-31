//! Disposable shared world/perception work: no neural engine, trainer or checkpoint.
use alife_core::{
    BrainCapacityClass, BrainGenome, BrainScaleTier, DevelopmentState, FoundationWeightAsset,
    NormalizedScalar, PerceptionContextBlock, PhenotypeCompiler, SensorProfile, Tick,
    TrainingStageManifest,
};
use alife_world::{create_canonical_new_game_with_n2048_candidate, CanonicalNewGameConfig};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let ticks: u64 = arguments.next().unwrap_or_else(|| "32".into()).parse()?;
    let population: u16 = arguments.next().unwrap_or_else(|| "1".into()).parse()?;
    if !(1..=512).contains(&ticks) || !(1..=8).contains(&population) || arguments.next().is_some() {
        return Err("profile_shared_host requires ticks1..512 and population1..8".into());
    }
    let setup = Instant::now();
    // Same synthetic native graph/canonical-world construction used by the
    // existing policy_weight_comparison_signature integration fixture.
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(539_363_617, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0)?);
    let phenotype = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )?;
    let weights = phenotype
        .synapses()
        .iter()
        .map(|synapse| synapse.genetic_weight())
        .collect();
    let asset = FoundationWeightAsset::from_trained_weights(
        &phenotype,
        weights,
        TrainingStageManifest::bootstrap(),
    )?;
    let mut config = CanonicalNewGameConfig::phase3(71_001, population)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = 539_363_617;
    config.sensor_profile = asset.manifest().sensor_profile();
    let mut world = create_canonical_new_game_with_n2048_candidate(&config, &asset)?.world;
    let setup_seconds = setup.elapsed().as_secs_f64();
    let initial_signature = world.canonical_signature_digest()?;
    let organisms = world
        .organism_entity_ids()
        .into_iter()
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    let mut frames = Vec::with_capacity(ticks as usize * organisms.len());
    let (mut index_seconds, mut perception_seconds, mut world_tick_seconds) = (0.0, 0.0, 0.0);
    let started = Instant::now();
    for _ in 0..ticks {
        let tick = world.tick();
        let clock = Instant::now();
        let index = world.build_perception_batch_index()?;
        index_seconds += clock.elapsed().as_secs_f64();
        for id in &organisms {
            let clock = Instant::now();
            // Use the world's actual current physiology, never a fabricated
            // hunger/homeostasis baseline. Perception also updates tracking.
            let homeostasis = world
                .organism_registry()
                .get(*id)
                .ok_or("missing fixture organism")?
                .biochemistry()
                .homeostasis;
            let frame = world
                .perception_frame_draft_indexed(
                    *id,
                    tick,
                    config.sensor_profile,
                    homeostasis,
                    &index,
                )?
                .finalize(PerceptionContextBlock::empty())?;
            let row = (
                tick.raw(),
                id.raw(),
                frame.frame_digest().0,
                frame.candidates().len(),
            );
            perception_seconds += clock.elapsed().as_secs_f64();
            frames.push(row);
        }
        let clock = Instant::now();
        // The public transaction includes the rollback clone and performs
        // ordinary shared physics/ecology/biology with no neural commands.
        world.try_advance_tick()?;
        world_tick_seconds += clock.elapsed().as_secs_f64();
    }
    let shared_host_seconds = started.elapsed().as_secs_f64();
    let final_signature = world.canonical_signature_digest()?;
    println!(
        "{}",
        serde_json::json!({
            "schema":1,"kind":"disposable-shared-host-profile-fixture",
            "debug_assertions":cfg!(debug_assertions),"ticks":ticks,"population":population,
            "setup_seconds":setup_seconds,"shared_host_seconds":shared_host_seconds,
            "index_seconds":index_seconds,"perception_seconds":perception_seconds,
            "world_tick_seconds":world_tick_seconds,"initial_signature":initial_signature,
            "final_signature":final_signature,"frame_signatures":frames,
            "neural_engine_executed":false,"training_executed":false,
            "scope":"shared perception index, grounded draft/finalize with empty context, safe passive world tick; excludes episodic recall, neural inference, PPO and persistence"
        })
    );
    Ok(())
}
