use alife_core::{BrainCapacityClass, BrainGenome, FoundationWeightAsset, SensorProfile};
use alife_training::{
    mutate_hardening_genome, pareto_front, HardeningEvaluation, HardeningFitness,
    HardeningMutationKind, HARDENING_DESCENDANTS_PER_FINALIST, HARDENING_NEWBORNS_PER_GENOME,
    HARDENING_WORLD_COUNT,
};

fn evaluation(seed: u64, survival: f32, learning: f32) -> HardeningEvaluation {
    HardeningEvaluation {
        genome: BrainGenome::scaffold(seed, BrainCapacityClass::N2048_ID),
        mutation: HardeningMutationKind::SparseGeneticDelta,
        viable: true,
        nonviable_reason: None,
        newborn_count: HARDENING_NEWBORNS_PER_GENOME,
        world_count: HARDENING_WORLD_COUNT,
        neural_ticks: 32,
        fitness: Some(
            HardeningFitness {
                survival,
                learning,
                language_acquisition: 0.5,
                narration_fidelity: 0.5,
                mutation_robustness: 0.5,
                compute_efficiency: 0.5,
            }
            .validate()
            .unwrap(),
        ),
    }
}

#[test]
fn hardening_mutations_are_deterministic_distinct_and_parent_bound() {
    let parent = BrainGenome::scaffold(77, BrainCapacityClass::N2048_ID);
    let mut child_ids = std::collections::BTreeSet::new();
    for mutation in HardeningMutationKind::ALL_MUTATIONS {
        let first = mutate_hardening_genome(&parent, mutation, 991).unwrap();
        let replay = mutate_hardening_genome(&parent, mutation, 991).unwrap();
        assert_eq!(first, replay);
        assert_ne!(first.id, parent.id);
        assert_eq!(first.parent_genome_ids, vec![parent.id]);
        assert!(child_ids.insert(first.id.raw()));
    }
}

#[test]
fn pareto_selection_keeps_tradeoffs_instead_of_collapsing_to_one_iq_scalar() {
    let evaluations = vec![
        evaluation(1, 0.9, 0.4),
        evaluation(2, 0.4, 0.9),
        evaluation(3, 0.3, 0.3),
    ];
    assert_eq!(pareto_front(&evaluations), vec![0, 1]);
}

#[test]
fn hardening_population_contract_is_four_world_distinct_newborns_and_eight_descendants() {
    assert_eq!(HARDENING_NEWBORNS_PER_GENOME, 4);
    assert_eq!(HARDENING_WORLD_COUNT, 4);
    assert_eq!(HARDENING_DESCENDANTS_PER_FINALIST, 8);
}

#[test]
fn shipped_grounded_foundation_has_explicit_evolutionary_promotion_evidence() {
    let source =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    assert!(source.manifest().promotion_receipt().is_promoted());
}
