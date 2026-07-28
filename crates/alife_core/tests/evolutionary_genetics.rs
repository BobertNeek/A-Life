//! EI0 diploid genome, reproduction, and causal expression contracts.

use alife_core::{
    AlleleDominance, BodyFrame, BrainCapacityClass, ChromosomeKind, ContinuousLocus,
    CreatureGenome, DiscreteAllele, DiscreteExpression, DiscreteLocus, FoundationGeneticIdentity,
    MatePreference, MutationRecord, ScaffoldContractError, StarterVocabularyProfile, Validate,
    MAX_CROSSOVER_SEGMENTS,
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

#[test]
fn seeded_reproduction_is_byte_deterministic_bounded_and_records_both_parents() {
    let maternal = early_mammal(0xE10_0101);
    let paternal = early_mammal(0xE10_0102);

    let one = CreatureGenome::reproduce(&maternal, &paternal, 0xC0FF_EE01).unwrap();
    let two = CreatureGenome::reproduce(&maternal, &paternal, 0xC0FF_EE01).unwrap();

    assert_eq!(
        serde_json::to_vec(&one).unwrap(),
        serde_json::to_vec(&two).unwrap()
    );
    assert_eq!(one.parent_genome_ids, vec![maternal.id, paternal.id]);
    assert!(one.provenance.ordinary_birth);
    assert_eq!(one.provenance.conception_seed, 0xC0FF_EE01);
    assert_eq!(one.provenance.recombination.len(), 6);
    assert!(one.provenance.recombination.iter().all(|record| {
        record.maternal_segments <= MAX_CROSSOVER_SEGMENTS
            && record.paternal_segments <= MAX_CROSSOVER_SEGMENTS
            && record.maternal_segments > 0
            && record.paternal_segments > 0
    }));
    assert!(one.provenance.mutations.iter().all(|record| match record {
        MutationRecord::Continuous {
            after,
            lower,
            upper,
            ..
        } => (lower..=upper).contains(&after),
        MutationRecord::Discrete { before, after, .. } => before != after,
    }));
    one.validate_contract().unwrap();
}

#[test]
fn seeded_discrete_mutation_changes_only_declared_domains_and_records_provenance() {
    let mut maternal = early_mammal(0xE10_0201);
    let mut paternal = early_mammal(0xE10_0202);
    for parent in [&mut maternal, &mut paternal] {
        parent.reproduction.discrete_mutation_rate = ContinuousLocus::mean(1.0, 1.0).unwrap();
    }

    let one = CreatureGenome::reproduce(&maternal, &paternal, 0xD15C_0001).unwrap();
    let two = CreatureGenome::reproduce(&maternal, &paternal, 0xD15C_0001).unwrap();
    assert_eq!(one, two);

    assert!(matches!(
        one.body.frame.maternal.value,
        BodyFrame::Light | BodyFrame::Balanced | BodyFrame::Sturdy
    ));
    assert!(matches!(
        one.body.frame.paternal.value,
        BodyFrame::Light | BodyFrame::Balanced | BodyFrame::Sturdy
    ));
    assert!(matches!(
        one.reproduction.mate_preference.maternal.value,
        MatePreference::Novelty | MatePreference::Similarity | MatePreference::Health
    ));
    assert!(matches!(
        one.predisposition.starter_vocabulary.paternal.value,
        StarterVocabularyProfile::Minimal
            | StarterVocabularyProfile::Foraging
            | StarterVocabularyProfile::Social
    ));

    let discrete_chromosomes = one
        .provenance
        .mutations
        .iter()
        .filter_map(|record| match record {
            MutationRecord::Discrete { chromosome, .. } => Some(*chromosome),
            MutationRecord::Continuous { .. } => None,
        })
        .collect::<Vec<_>>();
    for expected in [
        ChromosomeKind::Body,
        ChromosomeKind::Reproduction,
        ChromosomeKind::Predisposition,
    ] {
        assert!(discrete_chromosomes.contains(&expected));
    }
}

#[test]
fn reproduction_rejects_incompatible_brain_classes_and_foundation_families() {
    let maternal = early_mammal(0xE10_0301);
    let different_class = CreatureGenome::early_mammal_founder(
        0xE10_0302,
        foundation(11, 7, BrainCapacityClass::N1024_ID),
    )
    .unwrap();
    assert_eq!(
        CreatureGenome::reproduce(&maternal, &different_class, 9).unwrap_err(),
        ScaffoldContractError::IncompatibleGeneticClass
    );

    let different_family = CreatureGenome::early_mammal_founder(
        0xE10_0303,
        foundation(12, 99, BrainCapacityClass::N512_ID),
    )
    .unwrap();
    assert_eq!(
        CreatureGenome::reproduce(&maternal, &different_family, 9).unwrap_err(),
        ScaffoldContractError::IncompatibleGeneticClass
    );
}

#[test]
fn offspring_serialization_round_trip_keeps_dna_and_excludes_acquired_mind_state() {
    let maternal = early_mammal(0xE10_0401);
    let paternal = early_mammal(0xE10_0402);
    let offspring = CreatureGenome::reproduce(&maternal, &paternal, 0x5A9E_0001).unwrap();
    let wire = serde_json::to_value(&offspring).unwrap();
    let decoded: CreatureGenome = serde_json::from_value(wire.clone()).unwrap();
    decoded.validate_contract().unwrap();
    assert_eq!(decoded, offspring);

    let keys = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        keys,
        std::collections::BTreeSet::from([
            "body",
            "brain",
            "chemistry",
            "conception_seed",
            "development",
            "foundation",
            "id",
            "lineage_id",
            "parent_genome_ids",
            "predisposition",
            "provenance",
            "reproduction",
            "schema_version",
        ])
    );
}
