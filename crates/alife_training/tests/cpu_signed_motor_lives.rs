#![cfg(feature = "cpu-backend")]
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
use alife_world::*;

fn fixture(signed: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let mut config = CanonicalNewGameConfig::phase3(0x2048_2026, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    let builtin = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile).unwrap();
    let first = create_canonical_new_game_with_n2048_candidate(&config, &builtin).unwrap();
    let record = first.world.organism_registry().iter().next().unwrap();
    let genome = record.phenotype().brain_genome.clone();
    let capacity = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        &genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let compile = if signed {
        PhenotypeCompiler::compile_n2048_signed_motor_v3_graph
    } else {
        PhenotypeCompiler::compile_n2048_anchored_v2_graph
    };
    let base = compile(genome.clone(), development.clone(), config.sensor_profile)
        .unwrap()
        .0;
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&base).unwrap();
    assert!(create_canonical_new_game_with_n2048_candidate(&config, &asset).is_err());
    let game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset).unwrap();
    // Reconstruct the actual newborn's genome/development, not a surrogate.
    let record = game.world.organism_registry().iter().next().unwrap();
    let genome = record.phenotype().brain_genome.clone();
    let development = foundation_construction_development(
        &genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let p = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        genome,
        development,
        asset.clone(),
    )
    .unwrap()
    .0;
    let mut trainer = CpuFoundationTrainer::new(
        p.clone(),
        asset,
        StageTrainableMask::recurrent_only(&p).unwrap(),
        AdamWConfig::default(),
    )
    .unwrap();
    let mut checkpoint = trainer.checkpoint();
    checkpoint.optimizer_step = 7;
    checkpoint.first_moment[0] = 0.125;
    checkpoint.second_moment[0] = 0.25;
    checkpoint.update_ages[0] = 7;
    let before = serde_json::to_vec(&checkpoint).unwrap();
    trainer.restore_checkpoint(checkpoint).unwrap();
    assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), before);
    let host = CpuCanonicalHost::new_cohort(game, &trainer, 42).unwrap();
    (host, trainer)
}
#[test]
fn acquired_cpu_lives_cannot_cross_signed_motor_policy_boundary() {
    let (mut old, old_trainer) = fixture(false);
    let (mut new, new_trainer) = fixture(true);
    let mut decisions = 0;
    for _ in 0..4 {
        old.step(&old_trainer, 0.7).unwrap();
        decisions += new.step(&new_trainer, 0.7).unwrap().decisions.len();
    }
    assert!(
        decisions > 0,
        "checkpoint fixture must execute actual decisions"
    );
    let old_save = old.checkpoint(&old_trainer).unwrap();
    let new_save = new.checkpoint(&new_trainer).unwrap();
    assert!(
        new_save
            .residents
            .iter()
            .any(|r| r.neural.last_outcome_tick.is_some()
                && r.neural
                    .eligibility
                    .iter()
                    .chain(&r.neural.fast)
                    .chain(&r.neural.lifetime)
                    .any(|v| *v != 0.0)),
        "roundtrip must include nontrivial acquired neural state"
    );
    assert!(CpuCanonicalHost::from_checkpoint(old_save.clone(), &new_trainer).is_err());
    assert!(CpuCanonicalHost::from_checkpoint(new_save.clone(), &old_trainer).is_err());
    assert!(old_trainer
        .checkpoint()
        .validate_for(
            new_trainer.phenotype(),
            new_trainer.checkpoint().source_foundation_digest
        )
        .is_err());
    assert!(new_trainer
        .checkpoint()
        .validate_for(
            old_trainer.phenotype(),
            old_trainer.checkpoint().source_foundation_digest
        )
        .is_err());
    let bytes = serde_json::to_vec(&new_save).unwrap();
    let restored_save = serde_json::from_slice(&bytes).unwrap();
    let mut restored = CpuCanonicalHost::from_checkpoint(restored_save, &new_trainer).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.checkpoint(&new_trainer).unwrap()).unwrap(),
        bytes
    );
    new.step(&new_trainer, 0.7).unwrap();
    restored.step(&new_trainer, 0.7).unwrap();
    assert_eq!(
        serde_json::to_vec(&new.checkpoint(&new_trainer).unwrap()).unwrap(),
        serde_json::to_vec(&restored.checkpoint(&new_trainer).unwrap()).unwrap()
    );
}
