//! Run unchanged against qualified-original and additive-integration sources.
//! Prints hashes only. Never writes a checkpoint or acquired-state artifact.
use alife_core::*;
use alife_training::cpu::*;
use alife_training::*;
use alife_world::*;
use sha2::{Digest, Sha256};
#[allow(dead_code)]
#[path = "../src/cpu/grouped_host_fixture.rs"]
mod recipe;
fn main() {
    let seed = 0x004f_5054_564f_4331;
    let r = recipe::build(seed, true).unwrap();
    let founder = CpuFoundationTrainer::new(
        r.phenotype.clone(),
        r.asset,
        StageTrainableMask::recurrent_only(&r.phenotype).unwrap(),
        AdamWConfig::default(),
    )
    .unwrap();
    let mut game = create_grounded_drive_research_new_game(&r.config, &r.genotype).unwrap();
    let newborn = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let mut blank = HeadlessScenarioBuilder::new(seed)
        .agent("learner", newborn.organism_id(), Vec3f::ZERO)
        .build()
        .unwrap();
    blank
        .register_grounded_drive_research_record(newborn)
        .unwrap();
    game.world = blank;
    let (mut host, ()) = CpuOptionalVocalM2HostV1::new_prepared(
        game,
        founder,
        r.inputs,
        seed as u32,
        MediationModeV1::Full,
        |_| Ok::<_, String>(()),
    )
    .unwrap();
    for boundary in 0..=2 {
        let wire = host.checkpoint().unwrap().encode().unwrap();
        println!("{}", serde_json::json!({"schema":"m2-play-default-wire-v1", "boundary":boundary, "bytes":wire.len(), "sha256":format!("{:x}", Sha256::digest(&wire))}));
        if boundary != 2 {
            host.step(0.73).unwrap();
        }
    }
}
