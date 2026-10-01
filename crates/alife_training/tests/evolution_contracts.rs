use alife_core::{
    ActionCandidateCreditProfileV1, BrainCapacityClass, BrainGenome, DevelopmentState,
    FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler, PlasticityGenomeParameters,
    SensorProfile, Tick,
};
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
fn descendant_mutations_preserve_the_finalists_inherited_genome_changes() {
    let founder = BrainGenome::scaffold(77, BrainCapacityClass::N2048_ID);
    let finalist =
        mutate_hardening_genome(&founder, HardeningMutationKind::AlphaPlasticity, 991).unwrap();
    assert_ne!(
        finalist.alpha_mask.default_alpha,
        founder.alpha_mask.default_alpha
    );

    let descendant =
        mutate_hardening_genome(&finalist, HardeningMutationKind::SparseGeneticDelta, 992).unwrap();
    assert_eq!(
        descendant.alpha_mask.default_alpha,
        finalist.alpha_mask.default_alpha
    );
    assert_eq!(descendant.parent_genome_ids, vec![finalist.id]);
    assert_eq!(descendant.lineage_id, finalist.lineage_id);
}

#[test]
fn receptor_mutations_preserve_inherited_action_credit_scope() {
    let founder = BrainGenome::scaffold(77, BrainCapacityClass::N2048_ID);
    let source = *founder.plasticity_parameters();
    let unscoped = PlasticityGenomeParameters::try_new(
        source.eligibility_decay(),
        source.base_learning_rate(),
        source.normalization_rate(),
        source.sleep_replay_rate(),
        source.receptor_profile(),
        source.fast_bounds().0,
        source.fast_bounds().1,
        source.sleep_staging_rate(),
        source.sleep_weight_limit(),
        source.sleep_fast_decay_rate(),
    )
    .unwrap();

    for profile in [
        None,
        Some(ActionCandidateCreditProfileV1::SignedConsequences),
        Some(ActionCandidateCreditProfileV1::SignedChoiceReadouts),
    ] {
        let parameters = match profile {
            Some(profile) => unscoped
                .with_action_candidate_credit_profile(profile)
                .unwrap(),
            None => unscoped,
        };
        let parent = founder
            .clone()
            .with_plasticity_parameters(parameters)
            .unwrap();
        for mutation in [
            HardeningMutationKind::PlasticityReceptor,
            HardeningMutationKind::BiochemicalSensitivity,
        ] {
            let child = mutate_hardening_genome(&parent, mutation, 993).unwrap();
            let child_parameters = child.plasticity_parameters();
            assert_eq!(
                child_parameters.action_candidate_credit_profile(),
                profile,
                "{mutation:?} discarded inherited {profile:?}",
            );
            assert_ne!(
                child_parameters.receptor_profile(),
                parameters.receptor_profile()
            );
            assert_eq!(
                child_parameters.eligibility_decay(),
                parameters.eligibility_decay()
            );
            assert_eq!(child_parameters.fast_bounds(), parameters.fast_bounds());
            assert_eq!(parent.plasticity_parameters(), &parameters);
        }
    }
}

#[test]
fn sparse_genetic_reseeding_changes_inherited_weights_without_changing_the_frozen_graph() {
    let capacity = BrainCapacityClass::n2048();
    let parent = BrainGenome::scaffold(77, capacity.id());
    let child =
        mutate_hardening_genome(&parent, HardeningMutationKind::SparseGeneticDelta, 992).unwrap();
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let compile = |genome: &BrainGenome| {
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        PhenotypeCompiler::compile_from_foundation_asset(
            genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &foundation,
        )
        .unwrap()
    };
    let parent = compile(&parent);
    let child = compile(&child);
    assert_eq!(parent.synapses().len(), child.synapses().len());
    assert!(parent
        .synapses()
        .iter()
        .zip(child.synapses())
        .all(|(a, b)| { a.source() == b.source() && a.target() == b.target() }));
    assert!(parent
        .synapses()
        .iter()
        .zip(child.synapses())
        .any(|(a, b)| a.genetic_weight().to_bits() != b.genetic_weight().to_bits()));
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
