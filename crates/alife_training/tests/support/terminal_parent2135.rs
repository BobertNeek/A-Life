use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
use alife_world::*;
use sha2::{Digest, Sha256};

const ASSET: &[u8] = include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-bearing4.alife-foundation");
const GENOME: &[u8] = include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-bearing4.alife-foundation.genome.json");

// The same exported pair and native construction as the research example.
// Kept independent of the optional head so the pre-feature executable can use it.
pub fn fixture(atp: Option<f32>) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    assert_eq!(
        format!("{:x}", Sha256::digest(ASSET)),
        "ec710f2176ae4cfccdd342953c7778385f1880efc7a0216ca5412aad68d6628c"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(GENOME)),
        "5c63a675c89be9195de6b2bdedfc51a0f6f95fa11917ef8db6b75492bba025f7"
    );
    let asset = FoundationWeightAsset::decode_canonical(ASSET).unwrap();
    let genome: CreatureGenome = serde_json::from_slice(GENOME).unwrap();
    assert!(
        genome
            .n2048_foundation_candidate
            .as_ref()
            .unwrap()
            .encode_canonical()
            .unwrap()
            == ASSET
    );
    let mut config = CanonicalNewGameConfig::phase3(2026111701, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = asset.manifest().sensor_profile();
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset).unwrap();
    let old = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let expressed = genome.express().unwrap();
    let development = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome.clone(),
        development,
        asset.clone(),
    )
    .unwrap();
    assert_eq!(
        p.candidate_decoder().factorized_motor_channels(&p).unwrap(),
        [MotorChannel::Locomotion, MotorChannel::Manipulation]
    );
    assert!(p
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
    assert_eq!(
        p.candidate_decoder()
            .memory_channel()
            .unwrap()
            .max_candidate_gain(),
        0.5
    );
    let mut chemistry = *old.biochemistry();
    if let Some(atp) = atp {
        chemistry.homeostasis.drives.brain_atp = atp;
    }
    game.world
        .replace_organism_record_exact(
            WorldOrganismRecord::new(
                old.organism_id(),
                old.world_entity_id(),
                genome.clone(),
                expressed,
                chemistry,
                old.birth_tick(),
            )
            .unwrap(),
        )
        .unwrap();
    game.creatures[0].weights.genetic_fixed_digest =
        PortableAssetDigest::for_bytes(&serde_json::to_vec(&genome).unwrap()).0;
    let mask = StageTrainableMask::recurrent_only(&p).unwrap();
    let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
    (
        CpuCanonicalHost::new_cohort(game, &trainer, 42).unwrap(),
        trainer,
    )
}

pub fn local_output() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(std::env::var_os("ALIFE_NATIVE_QUALIFY_OUTPUT")?);
    std::fs::create_dir(&root).unwrap();
    // Every create-only run protects itself before writing any acquired arrays.
    std::fs::write(root.join(".gitignore"), b"*\n").unwrap();
    Some(root)
}
