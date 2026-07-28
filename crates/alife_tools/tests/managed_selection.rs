use alife_core::{
    BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, GenomeId, Validate,
};
use alife_tools::p33_evaluation::{ObjectiveVector, ScoreEstimate};
use alife_tools::p33_selection::{
    run_managed_selection, CandidateRejectionReason, ManagedSelectionConfig, PairingLane,
    PopulationLane, ProbationCheck, SelectionCandidate, SpecialistRole,
};

fn genome(seed: u64) -> CreatureGenome {
    CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(0xE10, 1, 0x5E1E_C710, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
}

fn objectives(values: [f32; 7]) -> ObjectiveVector {
    let known = |value| ScoreEstimate::known(value, 4);
    ObjectiveVector {
        ecological: known(values[0]),
        cognitive: known(values[1]),
        social: known(values[2]),
        group: known(values[3]),
        stability: known(values[4]),
        efficiency: known(values[5]),
        diversity: known(values[6]),
    }
}

fn managed(seed: u64, values: [f32; 7]) -> SelectionCandidate {
    SelectionCandidate {
        genome: genome(seed),
        objectives: objectives(values),
        known_ancestor_genome_ids: Vec::new(),
        population_share: 0.25,
        lane: PopulationLane::Managed,
        specialist_roles: Vec::new(),
    }
}

fn config(seed: u64, max_pairings: usize) -> ManagedSelectionConfig {
    ManagedSelectionConfig {
        selection_seed: seed,
        max_pairings,
        minority_lineage_share_max: 0.10,
        fragile_ecology_max: 0.40,
        high_cognition_min: 0.75,
        robust_ecology_min: 0.65,
        introgression_sibling_count: 2,
    }
}

fn pairing_contains(pair: &alife_tools::p33_selection::ManagedPairing, id: GenomeId) -> bool {
    pair.maternal_genome_id == id || pair.paternal_genome_id == id
}

#[test]
fn selection_is_deterministic_and_preserves_wild_minority_and_specialists() {
    let mut wild = managed(0xE10_1001, [0.8; 7]);
    wild.lane = PopulationLane::Wild;

    let mut minority = managed(0xE10_1002, [0.95, 0.50, 0.45, 0.40, 0.80, 0.55, 0.99]);
    minority.population_share = 0.03;
    let minority_lineage = minority.genome.lineage_id;

    let mut teacher = managed(0xE10_1003, [0.70, 0.82, 0.96, 0.85, 0.76, 0.50, 0.40]);
    teacher.specialist_roles = vec![SpecialistRole::Teacher];
    let teacher_id = teacher.genome.id;

    let mut efficient = managed(0xE10_1004, [0.68, 0.78, 0.55, 0.52, 0.79, 0.99, 0.45]);
    efficient.specialist_roles = vec![SpecialistRole::EfficientSolver];
    let efficient_id = efficient.genome.id;

    let robust = managed(0xE10_1005, [0.88, 0.74, 0.74, 0.75, 0.90, 0.80, 0.60]);
    let candidates = vec![wild.clone(), minority, teacher, efficient, robust];

    let first = run_managed_selection(&candidates, &config(11, 2)).unwrap();
    let second = run_managed_selection(&candidates, &config(11, 2)).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.preserved_wild_genomes, vec![wild.genome.clone()]);
    assert!(first.retained_minority_lineages.contains(&minority_lineage));
    assert!(first
        .retained_specialists
        .iter()
        .any(
            |retained| retained.role == SpecialistRole::Teacher && retained.genome_id == teacher_id
        ));
    assert!(first
        .retained_specialists
        .iter()
        .any(|retained| retained.role == SpecialistRole::EfficientSolver
            && retained.genome_id == efficient_id));
    assert!(first.pareto_frontier.len() >= 3);
    assert!(first.pairings.iter().all(|pair| {
        !pairing_contains(pair, wild.genome.id) && !pair.offspring_genome_ids.is_empty()
    }));
    assert!(first.offspring.iter().all(|offspring| {
        offspring.genome.validate_contract().is_ok()
            && offspring.genetic_provenance == offspring.genome.provenance
            && offspring.viability.neuron_count > 0
            && offspring.viability.synapse_count > 0
    }));
}

#[test]
fn cognitive_introgression_requires_robust_unrelated_mate_and_elevated_probation() {
    let fragile = managed(0xE10_2001, [0.20, 0.97, 0.70, 0.68, 0.62, 0.75, 0.82]);
    let fragile_id = fragile.genome.id;
    let another_fragile = managed(0xE10_2002, [0.25, 0.96, 0.72, 0.70, 0.60, 0.74, 0.80]);
    let another_fragile_id = another_fragile.genome.id;
    let robust = managed(0xE10_2003, [0.92, 0.62, 0.65, 0.66, 0.93, 0.77, 0.58]);
    let robust_id = robust.genome.id;
    let population_control = managed(0xE10_2004, [0.86, 0.60, 0.63, 0.62, 0.90, 0.74, 0.56]);
    let control_id = population_control.genome.id;

    let plan = run_managed_selection(
        &[fragile, another_fragile, robust, population_control],
        &config(1, 1),
    )
    .unwrap();

    assert_eq!(plan.pairings.len(), 1);
    let pairing = &plan.pairings[0];
    assert_eq!(pairing.lane, PairingLane::CognitiveIntrogression);
    assert!(pairing_contains(pairing, fragile_id) || pairing_contains(pairing, another_fragile_id));
    assert!(
        !(pairing_contains(pairing, fragile_id) && pairing_contains(pairing, another_fragile_id))
    );
    assert!(pairing_contains(pairing, robust_id) || pairing_contains(pairing, control_id));
    assert_eq!(pairing.offspring_genome_ids.len(), 2);

    for offspring in &plan.offspring {
        offspring.genome.validate_contract().unwrap();
        offspring.genetic_provenance.validate_contract().unwrap();
        assert!(offspring.genetic_provenance.ordinary_birth);
        assert_eq!(offspring.genetic_provenance, offspring.genome.provenance);
        assert_eq!(offspring.genome.parent_genome_ids.len(), 2);
        assert_eq!(offspring.viability.neuron_count, 512);
        assert!(offspring.viability.synapse_count > 0);

        let probation = offspring.probation.as_ref().unwrap();
        assert_eq!(probation.scrutiny_multiplier, 2);
        assert_eq!(
            probation.required_checks,
            vec![
                ProbationCheck::Cognition,
                ProbationCheck::Ecology,
                ProbationCheck::Transfer,
                ProbationCheck::StabilityHealth,
                ProbationCheck::Development,
            ]
        );
        assert_eq!(probation.sibling_controls.len(), 1);
        assert!(!probation.population_controls.is_empty());
        assert!(probation
            .population_controls
            .iter()
            .all(|id| !offspring.genome.parent_genome_ids.contains(id)));
    }
}

#[test]
fn shared_lineage_parent_and_known_ancestry_pairs_never_reproduce() {
    let same_lineage_left = managed(0xE10_3001, [0.8; 7]);
    let mut same_lineage_right = managed(0xE10_3002, [0.8; 7]);
    same_lineage_right.genome.lineage_id = same_lineage_left.genome.lineage_id;

    let shared_ancestor = GenomeId(0xA11C_E570);
    let mut ancestor_left = managed(0xE10_3003, [0.8; 7]);
    let mut ancestor_right = managed(0xE10_3004, [0.8; 7]);
    ancestor_left.known_ancestor_genome_ids = vec![shared_ancestor];
    ancestor_right.known_ancestor_genome_ids = vec![shared_ancestor];

    let parent_a = genome(0xE10_3010);
    let parent_b = genome(0xE10_3011);
    let sibling_left_genome = CreatureGenome::reproduce(&parent_a, &parent_b, 0xE10_3012).unwrap();
    let sibling_right_genome = CreatureGenome::reproduce(&parent_a, &parent_b, 0xE10_3013).unwrap();
    let mut sibling_left = managed(0xE10_3014, [0.8; 7]);
    let mut sibling_right = managed(0xE10_3015, [0.8; 7]);
    sibling_left.genome = sibling_left_genome;
    sibling_right.genome = sibling_right_genome;

    let unrelated_left = managed(0xE10_3020, [0.8; 7]);
    let unrelated_right = managed(0xE10_3021, [0.8; 7]);

    let forbidden = [
        (same_lineage_left.genome.id, same_lineage_right.genome.id),
        (ancestor_left.genome.id, ancestor_right.genome.id),
        (sibling_left.genome.id, sibling_right.genome.id),
    ];
    let plan = run_managed_selection(
        &[
            same_lineage_left,
            same_lineage_right,
            ancestor_left,
            ancestor_right,
            sibling_left,
            sibling_right,
            unrelated_left,
            unrelated_right,
        ],
        &config(9, 4),
    )
    .unwrap();

    assert!(!plan.pairings.is_empty());
    for pairing in &plan.pairings {
        assert!(forbidden.iter().all(|(left, right)| {
            !(pairing_contains(pairing, *left) && pairing_contains(pairing, *right))
        }));
    }
}

#[test]
fn fragile_fragile_pair_has_no_managed_reproduction_lane() {
    let fragile_left = managed(0xE10_4001, [0.20, 0.95, 0.7, 0.7, 0.6, 0.7, 0.8]);
    let fragile_right = managed(0xE10_4002, [0.25, 0.96, 0.7, 0.7, 0.6, 0.7, 0.8]);

    let plan = run_managed_selection(&[fragile_left, fragile_right], &config(1, 1)).unwrap();

    assert!(plan.pairings.is_empty());
    assert!(plan.offspring.is_empty());
}

#[test]
fn missing_objectives_and_corrupt_genomes_are_disqualified_without_poisoning_valid_pairs() {
    let mut missing = managed(0xE10_5001, [0.8; 7]);
    missing.objectives.cognitive = ScoreEstimate::UNKNOWN;
    let missing_id = missing.genome.id;

    let mut corrupt = managed(0xE10_5002, [0.8; 7]);
    corrupt.genome.schema_version = 0;
    let corrupt_id = corrupt.genome.id;

    let valid_left = managed(0xE10_5003, [0.8; 7]);
    let valid_right = managed(0xE10_5004, [0.8; 7]);
    let plan =
        run_managed_selection(&[missing, corrupt, valid_left, valid_right], &config(7, 1)).unwrap();

    assert!(plan.rejected_candidates.iter().any(|rejection| {
        rejection.genome_id == missing_id
            && rejection.reason == CandidateRejectionReason::MissingObjectiveEvidence
    }));
    assert!(plan.rejected_candidates.iter().any(|rejection| {
        rejection.genome_id == corrupt_id
            && rejection.reason == CandidateRejectionReason::InvalidGenome
    }));
    assert_eq!(plan.pairings.len(), 1);
    assert_eq!(plan.offspring.len(), 1);
}
