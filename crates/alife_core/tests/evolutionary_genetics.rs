//! EI0 diploid genome, reproduction, and causal expression contracts.

use alife_core::{
    AlleleDominance, BodyFrame, BrainCapacityClass, ContinuousLocus, CreatureGenome,
    DiscreteAllele, DiscreteExpression, DiscreteLocus, FoundationGeneticIdentity,
    ScaffoldContractError, Validate,
};

fn foundation(
    foundation_id: u64,
    family_id: u64,
    class_id: alife_core::BrainClassId,
) -> FoundationGeneticIdentity {
    FoundationGeneticIdentity::new(foundation_id, 1, family_id, class_id).unwrap()
}

fn early_mammal(seed: u64) -> CreatureGenome {
    CreatureGenome::early_mammal_founder(seed, foundation(10, 7, BrainCapacityClass::N512_ID))
        .unwrap()
}

#[test]
fn continuous_loci_blend_both_alleles() {
    let mean = ContinuousLocus::mean(0.2, 0.8).unwrap();
    assert_eq!(mean.expressed().unwrap(), 0.5);

    let maternal_bias = ContinuousLocus::with_bounds(0.2, 0.8, 0.0, 1.0, 0.6).unwrap();
    assert!((maternal_bias.expressed().unwrap() - 0.44).abs() < f32::EPSILON);
}

#[test]
fn discrete_loci_apply_dominant_recessive_and_codominant_rules() {
    let dominant = DiscreteLocus::new(
        DiscreteAllele::new(BodyFrame::Light, AlleleDominance::Recessive),
        DiscreteAllele::new(BodyFrame::Sturdy, AlleleDominance::Dominant),
    );
    assert_eq!(
        dominant.expressed(),
        DiscreteExpression::Single(BodyFrame::Sturdy)
    );

    let codominant = DiscreteLocus::new(
        DiscreteAllele::new(BodyFrame::Light, AlleleDominance::Codominant),
        DiscreteAllele::new(BodyFrame::Sturdy, AlleleDominance::Dominant),
    );
    assert_eq!(
        codominant.expressed(),
        DiscreteExpression::Codominant(BodyFrame::Light, BodyFrame::Sturdy)
    );
}

#[test]
fn founder_contains_two_valid_alleles_in_every_chromosome_family() {
    let genome = early_mammal(0xE10_0001);
    genome.validate_contract().unwrap();

    assert_ne!(genome.body.size.maternal, genome.body.size.paternal);
    assert_ne!(
        genome.brain.connectivity_density.maternal,
        genome.brain.connectivity_density.paternal
    );
    assert_ne!(
        genome.chemistry.reward_sensitivity.maternal,
        genome.chemistry.reward_sensitivity.paternal
    );
    assert_ne!(
        genome.development.maturation_rate.maternal,
        genome.development.maturation_rate.paternal
    );
    assert_ne!(
        genome.reproduction.fertility.maternal,
        genome.reproduction.fertility.paternal
    );
    assert_ne!(
        genome.predisposition.social_attention.maternal,
        genome.predisposition.social_attention.paternal
    );
}

#[test]
fn malformed_locus_bounds_are_rejected() {
    assert_eq!(
        ContinuousLocus::with_bounds(0.5, 0.5, 1.0, 0.0, 0.5).unwrap_err(),
        ScaffoldContractError::InvalidGeneticBounds
    );
    assert_eq!(
        ContinuousLocus::with_bounds(0.5, 1.5, 0.0, 1.0, 0.5).unwrap_err(),
        ScaffoldContractError::InvalidGeneticBounds
    );
    assert_eq!(
        ContinuousLocus::with_bounds(0.5, 0.5, 0.0, 1.0, 1.0).unwrap_err(),
        ScaffoldContractError::InvalidGeneticBounds
    );
}

#[test]
fn mutation_delta_above_the_ei0_limit_is_rejected_as_overflow() {
    let mut genome = early_mammal(0xE10_0002);
    genome.reproduction.max_mutation_delta = ContinuousLocus::mean(0.30, 0.30).unwrap();
    assert_eq!(
        genome.validate_contract().unwrap_err(),
        ScaffoldContractError::MutationOverflow
    );
}
