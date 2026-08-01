use alife_core::{
    BrainCapacityClass, CreatureGenome, Era1Control, FoundationGeneticIdentity, LanguageTokenId,
};
use alife_tools::era1_evolution::{run_era1_evolution, Era1EvolutionConfig};

fn founder(seed: u64) -> CreatureGenome {
    let foundation = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    CreatureGenome::early_mammal_founder(seed, foundation).unwrap()
}

#[test]
fn bounded_default_runs_exact_seeded_two_generation_reproduction() {
    let founders = vec![
        founder(51_001),
        founder(51_002),
        founder(51_003),
        founder(51_004),
    ];
    let config = Era1EvolutionConfig::bounded_default(0xE1_5001).unwrap();

    let run = run_era1_evolution(&config, &founders).unwrap();
    let repeated = run_era1_evolution(&config, &founders).unwrap();

    assert_eq!(run, repeated);
    assert_eq!(run.wild_reservoir, founders);
    assert_eq!(config.lineage_count, 4);
    assert_eq!(config.evaluation_seeds.len(), 3);
    assert_eq!(config.held_out_world_transforms.len(), 2);
    assert_eq!(config.controls, Era1Control::ALL);
    assert_eq!(config.ordinary_birth_generations, 2);
    assert_eq!(run.generations.len(), 3);
    assert_eq!(run.lineages.len(), 4);

    for generation_index in 1..run.generations.len() {
        let parents = &run.generations[generation_index - 1].births;
        let generation = &run.generations[generation_index];
        assert_eq!(generation.births.len(), 4);
        for birth in &generation.births {
            assert!(birth.genome.provenance.ordinary_birth);
            assert_eq!(birth.genome.parent_genome_ids.len(), 2);
            let maternal = parents
                .iter()
                .find(|candidate| candidate.genome.id == birth.genome.parent_genome_ids[0])
                .unwrap();
            let paternal = parents
                .iter()
                .find(|candidate| candidate.genome.id == birth.genome.parent_genome_ids[1])
                .unwrap();
            assert_eq!(
                birth.genome,
                CreatureGenome::reproduce(
                    &maternal.genome,
                    &paternal.genome,
                    birth.genome.conception_seed,
                )
                .unwrap()
            );
        }
    }
}

#[test]
fn ordinary_children_inherit_only_dna_starter_words_and_empty_lifetime_state() {
    let founders = vec![
        founder(52_001),
        founder(52_002),
        founder(52_003),
        founder(52_004),
    ];
    let config = Era1EvolutionConfig::bounded_default(0xE1_5002).unwrap();
    let run = run_era1_evolution(&config, &founders).unwrap();

    for birth in run
        .generations
        .iter()
        .skip(1)
        .flat_map(|generation| &generation.births)
    {
        assert!(birth.acquired_state.is_empty());
        let expressed = birth.genome.express().unwrap();
        assert_eq!(
            birth.inherited_starter_tokens,
            expressed.predisposition.starter_tokens
        );
        assert!(!birth.inherited_starter_tokens.is_empty());
        assert!(birth
            .inherited_starter_tokens
            .iter()
            .all(|token| *token != LanguageTokenId::new(0).unwrap()));
    }
}

#[test]
fn copied_learning_or_fabricated_inheritance_invalidates_evolution_receipts() {
    let founders = vec![
        founder(53_001),
        founder(53_002),
        founder(53_003),
        founder(53_004),
    ];
    let config = Era1EvolutionConfig::bounded_default(0xE1_5003).unwrap();
    let run = run_era1_evolution(&config, &founders).unwrap();
    run.validate_contract().unwrap();

    let mut copied_learning = run.clone();
    copied_learning.generations[1].births[0]
        .acquired_state
        .learned_vocabulary
        .push(LanguageTokenId::new(41).unwrap());
    assert!(copied_learning.validate_contract().is_err());

    let mut injected_silence = run.clone();
    injected_silence.generations[1].births[0]
        .inherited_starter_tokens
        .push(LanguageTokenId::new(0).unwrap());
    assert!(injected_silence.validate_contract().is_err());

    let mut fabricated_parent = run;
    fabricated_parent.generations[1].births[0]
        .genome
        .parent_genome_ids[0] = fabricated_parent.generations[0].births[2].genome.id;
    assert!(fabricated_parent.validate_contract().is_err());
}
