use alife_core::{
    BrainCapacityClass, CreatureGenome, Era1Control, FoundationGeneticIdentity, LanguageTokenId,
};
use alife_tools::{
    era1_evolution::{
        run_era1_evolution, Era1EvolutionConfig, Era1EvolutionError, Era1SelectionProfile,
    },
    p33_evaluation::{ObjectiveVector, ScoreEstimate},
    p33_selection::{PairingLane, SpecialistRole},
};

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

fn objectives(ecological: f32, cognitive: f32, index: usize) -> ObjectiveVector {
    ObjectiveVector {
        ecological: ScoreEstimate::known(ecological, 12),
        cognitive: ScoreEstimate::known(cognitive, 12),
        social: ScoreEstimate::known(0.60 + index as f32 * 0.02, 12),
        group: ScoreEstimate::known(0.68 - index as f32 * 0.01, 12),
        stability: ScoreEstimate::known(0.78 - index as f32 * 0.01, 12),
        efficiency: ScoreEstimate::known(0.66 + index as f32 * 0.02, 12),
        diversity: ScoreEstimate::known(0.55 + index as f32 * 0.05, 12),
    }
}

fn profiles(founders: &[CreatureGenome]) -> Vec<Era1SelectionProfile> {
    founders
        .iter()
        .enumerate()
        .map(|(index, founder)| Era1SelectionProfile {
            founder_genome_id: founder.id,
            objectives: objectives(
                0.82 - index as f32 * 0.03,
                0.62 + index as f32 * 0.04,
                index,
            ),
            known_ancestor_genome_ids: Vec::new(),
            population_share: 0.25,
            specialist_roles: vec![match index {
                0 => SpecialistRole::EcologicalSurvivor,
                1 => SpecialistRole::Teacher,
                2 => SpecialistRole::Coordinator,
                _ => SpecialistRole::TransferSpecialist,
            }],
        })
        .collect()
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

    let profiles = profiles(&founders);
    let root = tempfile::tempdir().unwrap();
    let run = run_era1_evolution(&config, &founders, &profiles, root.path()).unwrap();
    let repeated = run_era1_evolution(&config, &founders, &profiles, root.path()).unwrap();

    assert_eq!(run, repeated);
    assert_eq!(run.wild_reservoir, founders);
    assert_eq!(config.lineage_count, 4);
    assert_eq!(config.evaluation_seeds.len(), 3);
    assert_eq!(config.held_out_world_transforms.len(), 2);
    assert_eq!(config.controls, Era1Control::ALL);
    assert_eq!(config.ordinary_birth_generations, 2);
    assert_eq!(run.generations.len(), 3);
    assert_eq!(run.lineages.len(), 4);
    assert!(run.generations.iter().all(|generation| {
        !generation.archives.is_empty()
            && !generation.portable_save.digest_hex.is_empty()
            && generation.preserved_wild_genome_ids
                == founders.iter().map(|genome| genome.id).collect::<Vec<_>>()
    }));

    for generation_index in 1..run.generations.len() {
        let parents = &run.generations[generation_index - 1].births;
        let generation = &run.generations[generation_index];
        let plan = generation.selection_plan.as_ref().unwrap();
        assert!(!plan.pairings.is_empty());
        assert_eq!(generation.habitat_breeding.len(), plan.pairings.len());
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
    let root = tempfile::tempdir().unwrap();
    let run = run_era1_evolution(&config, &founders, &profiles(&founders), root.path()).unwrap();

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
    let root = tempfile::tempdir().unwrap();
    let run = run_era1_evolution(&config, &founders, &profiles(&founders), root.path()).unwrap();
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

#[test]
fn unknown_objectives_and_related_pairings_are_rejected() {
    let founders = vec![
        founder(54_001),
        founder(54_002),
        founder(54_003),
        founder(54_004),
    ];
    let config = Era1EvolutionConfig::bounded_default(0xE1_5004).unwrap();
    let root = tempfile::tempdir().unwrap();

    let mut unknown = profiles(&founders);
    unknown[0].objectives.cognitive = ScoreEstimate::UNKNOWN;
    assert!(matches!(
        run_era1_evolution(&config, &founders, &unknown, root.path()),
        Err(Era1EvolutionError::UnknownSelectionObjective(id)) if id == founders[0].id
    ));

    let mut related = profiles(&founders);
    for (index, profile) in related.iter_mut().enumerate() {
        profile.known_ancestor_genome_ids = founders
            .iter()
            .enumerate()
            .filter_map(|(other, genome)| (other != index).then_some(genome.id))
            .collect();
    }
    assert!(run_era1_evolution(&config, &founders, &related, root.path()).is_err());
}

#[test]
fn fragile_high_cognition_lineage_uses_managed_introgression_probation() {
    let founders = vec![
        founder(55_001),
        founder(55_002),
        founder(55_003),
        founder(55_004),
    ];
    let mut profiles = profiles(&founders);
    profiles[0].objectives = objectives(0.20, 0.95, 0);
    profiles[1].objectives = objectives(0.90, 0.55, 1);
    let config = Era1EvolutionConfig::bounded_default(0xE1_5005).unwrap();
    let root = tempfile::tempdir().unwrap();
    let run = run_era1_evolution(&config, &founders, &profiles, root.path()).unwrap();
    let plan = run.generations[1].selection_plan.as_ref().unwrap();
    assert!(plan
        .pairings
        .iter()
        .any(|pairing| pairing.lane == PairingLane::CognitiveIntrogression));
    assert!(plan
        .offspring
        .iter()
        .filter(|offspring| offspring.probation.is_some())
        .all(|offspring| offspring
            .probation
            .as_ref()
            .is_some_and(|probation| !probation.sibling_controls.is_empty()
                && !probation.population_controls.is_empty())));
}
