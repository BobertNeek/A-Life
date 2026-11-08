use super::*;
use crate::{AdamWConfig, StageTrainableMask};
fn fixture(bounded: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let mut config = alife_world::CanonicalNewGameConfig::phase3(7, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    let builtin = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile).unwrap();
    let source =
        alife_world::create_canonical_new_game_with_n2048_candidate(&config, &builtin).unwrap();
    let record = source.world.organism_registry().iter().next().unwrap();
    let g = record.phenotype().brain_genome.clone();
    let d = alife_runtime::canonical_preparation::foundation_construction_development(
        &g,
        &BrainCapacityClass::n2048(),
        &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let p = if bounded {
        PhenotypeCompiler::compile_n2048_bounded_founder(
            g.clone(),
            d.clone(),
            config.sensor_profile,
        )
    } else {
        PhenotypeCompiler::compile_n2048_experimental_graph(
            g.clone(),
            d.clone(),
            config.sensor_profile,
        )
    }
    .unwrap()
    .0;
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&p).unwrap();
    let p = if bounded {
        PhenotypeCompiler::compile_n2048_bounded_founder_candidate(g, d, asset.clone())
    } else {
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(g, d, asset.clone())
    }
    .unwrap()
    .0;
    let mask = StageTrainableMask::from_synapse_indices(
        &p,
        &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
    )
    .unwrap();
    let trainer =
        CpuFoundationTrainer::new(p, asset.clone(), mask, AdamWConfig::default()).unwrap();
    let game =
        alife_world::create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset)
            .unwrap();
    (
        CpuCanonicalHost::new_cohort(game, &trainer, 42).unwrap(),
        trainer,
    )
}
#[test]
fn bounded_host_admits_restores_acquired_fixture_and_continues_exactly() {
    let (mut host, trainer) = fixture(true);
    let (old, old_trainer) = fixture(false);
    let sentinel = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    host.step(&trainer, 1.).unwrap();
    host.step(&trainer, 1.).unwrap();
    let resident = host.residents.values_mut().next().unwrap();
    let mut state = resident.neural.checkpoint().unwrap();
    assert!(state.neural.activations.iter().any(|x| *x != 0.));
    // Prepare acquired-state fixture while owned by host, then use its
    // ordinary authority seal. Never patch a saved checkpoint's digest.
    let mut branches = state.neural.dendrites.branches().to_vec();
    branches[0].output_gain = 0.73;
    branches[0].threshold = 0.31;
    let acquired = DendriticBranchSet::new(branches).unwrap();
    state.neural.dendrites = acquired.clone();
    resident.neural = CpuOrganismState::from_checkpoint(trainer.phenotype(), state).unwrap();
    resident
        .v11
        .set_dendritic_branches(acquired.clone())
        .unwrap();
    host.seal_all_residents().unwrap();
    let cp = host.checkpoint(&trainer).unwrap();
    let bytes = serde_json::to_vec(&cp).unwrap();
    let mut a = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
    let mut b =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes).unwrap(), &trainer)
            .unwrap();
    assert_eq!(
        a.checkpoint(&trainer).unwrap().residents[0]
            .neural
            .neural
            .dendrites,
        acquired
    );
    assert!(CpuCanonicalHost::from_checkpoint(cp, &old_trainer).is_err());
    assert!(
        CpuCanonicalHost::from_checkpoint(old.checkpoint(&old_trainer).unwrap(), &trainer).is_err()
    );
    let x = a.step(&trainer, 1.).unwrap();
    let y = b.step(&trainer, 1.).unwrap();
    assert_eq!(
        serde_json::to_vec(&x.decisions).unwrap(),
        serde_json::to_vec(&y.decisions).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&a.checkpoint(&trainer).unwrap()).unwrap(),
        serde_json::to_vec(&b.checkpoint(&trainer).unwrap()).unwrap()
    );
    assert_eq!(sentinel, serde_json::to_vec(&trainer.checkpoint()).unwrap());
}
