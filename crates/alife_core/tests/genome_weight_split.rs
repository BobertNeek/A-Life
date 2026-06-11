use serde::{de::DeserializeOwned, Serialize};

use alife_core::{
    AlphaMask, AlphaStoragePolicy, BrainClassId, BrainClassSpec, BrainGenome, BrainScaleTier,
    CriticalPeriod, DevelopmentStage, DevelopmentState, DevelopmentalMilestone,
    DevelopmentalSchedule, EffectiveWeightSample, GenomeId, LifetimeConsolidationDelta, LobeKind,
    NormalizedScalar, ProjectionAlphaOverride, ProjectionKey, ScaffoldContractError, SchemaKind,
    SchemaVersions, SynapseAddress, Tick, Validate, WEffective, WeightLayerKind,
    WeightSplitContract,
};

fn assert_serde<T: Serialize + DeserializeOwned>() {}

#[test]
fn default_genome_is_schema_versioned_deterministic_and_non_lamarckian() {
    assert_serde::<BrainGenome>();

    let brain_class_id = BrainScaleTier::Small1024.default_class_id();
    let first = BrainGenome::scaffold(7, brain_class_id);
    let second = BrainGenome::scaffold(7, brain_class_id);

    assert_eq!(first, second);
    assert_eq!(first.schema_version, SchemaVersions::CURRENT.genome.raw());
    assert_eq!(first.species_seed, 7);
    assert_eq!(first.genetic_prior_seed, first.seeds.genetic_prior_seed);
    assert!(!first.inheritance.lamarckian_weights_enabled);
    assert!(!first.inheritance.inherit_lifetime_consolidation);
    assert!(first.validate_contract().is_ok());
}

#[test]
fn genome_validation_rejects_bad_version_unknown_class_and_invalid_ranges() {
    let mut genome = BrainGenome::scaffold(11, BrainScaleTier::Nano512.default_class_id());

    genome.schema_version = 99;
    assert!(matches!(
        genome.validate_contract(),
        Err(ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::Genome,
            expected: 1,
            actual: 99,
        })
    ));

    let mut genome = BrainGenome::scaffold(11, BrainScaleTier::Nano512.default_class_id());
    genome.brain_class_id = BrainClassId(99);
    assert!(matches!(
        genome.validate_contract(),
        Err(ScaffoldContractError::UnknownBrainClass)
    ));

    let mut genome = BrainGenome::scaffold(11, BrainScaleTier::Nano512.default_class_id());
    genome.sparse_density_priors[0].density = NormalizedScalar(1.25);
    assert!(matches!(
        genome.validate_contract(),
        Err(ScaffoldContractError::ScalarOutOfRange)
    ));

    let mut genome = BrainGenome::scaffold(11, BrainScaleTier::Nano512.default_class_id());
    genome
        .alpha_mask
        .projection_overrides
        .push(ProjectionAlphaOverride {
            projection: ProjectionKey::new(LobeKind::SensoryGrounding, LobeKind::CoreAssociation),
            alpha: NormalizedScalar(f32::NAN),
        });
    assert!(matches!(
        genome.validate_contract(),
        Err(ScaffoldContractError::NonFiniteFloat)
    ));

    let mut genome = BrainGenome::scaffold(11, BrainScaleTier::Nano512.default_class_id());
    genome.mutation_rates.point = NormalizedScalar(1.25);
    assert!(matches!(
        genome.validate_contract(),
        Err(ScaffoldContractError::ScalarOutOfRange)
    ));
}

#[test]
fn development_schedule_and_state_validate_monotonicity() {
    let schedule =
        DevelopmentalSchedule::standard(BrainScaleTier::Nano512.default_class_id()).unwrap();
    assert!(schedule.validate_contract().is_ok());

    let mut non_monotonic = schedule.clone();
    non_monotonic.milestones.push(DevelopmentalMilestone {
        stage: DevelopmentStage::Adult,
        begins_at: Tick(1),
        maturation: NormalizedScalar::new(1.0).unwrap(),
        target_brain_class_id: None,
    });
    assert!(matches!(
        non_monotonic.validate_contract(),
        Err(ScaffoldContractError::NonMonotonicTick)
    ));

    let critical_period = CriticalPeriod {
        lobe: LobeKind::CoreAssociation,
        opens_at: Tick(20),
        closes_at: Tick(10),
        plasticity_bias: NormalizedScalar::new(0.5).unwrap(),
    };
    let mut invalid_period = schedule;
    invalid_period.critical_periods.push(critical_period);
    assert!(matches!(
        invalid_period.validate_contract(),
        Err(ScaffoldContractError::NonMonotonicTick)
    ));

    let state = DevelopmentState::new(GenomeId(77), Tick(40), NormalizedScalar::new(0.6).unwrap())
        .with_enabled_lobes([LobeKind::SensoryGrounding, LobeKind::MotorArbitration]);
    assert_eq!(state.age_ticks, Tick(40));
    assert_eq!(state.enabled_lobes.len(), 2);
    assert!(state.validate_contract().is_ok());
}

#[test]
fn alpha_mask_defaults_to_hierarchical_sparse_policy() {
    let mut alpha = AlphaMask::default_for_projection(NormalizedScalar::new(0.25).unwrap());

    assert_eq!(alpha.storage_policy, AlphaStoragePolicy::HierarchicalSparse);
    assert!(alpha.per_synapse_overrides.is_empty());
    assert!(!alpha.dense_reference_opt_in);
    assert!(alpha.validate_contract().is_ok());

    alpha
        .per_synapse_overrides
        .push(alife_core::SynapseAlphaOverride {
            synapse: SynapseAddress::new(3, 9),
            alpha: NormalizedScalar::new(0.75).unwrap(),
            exceptional_reason: "injury recovery fixture".to_owned(),
        });
    assert!(alpha.validate_contract().is_ok());

    let mut dense_without_opt_in = alpha;
    dense_without_opt_in.storage_policy = AlphaStoragePolicy::DenseDebugReference;
    assert!(matches!(
        dense_without_opt_in.validate_contract(),
        Err(ScaffoldContractError::DenseAlphaRequiresOptIn)
    ));
}

#[test]
fn weight_split_consolidation_does_not_mutate_genetic_fixed() {
    let class = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut split = WeightSplitContract::for_brain_class(
        class.id,
        class.max_active_synapses,
        class.max_active_microtiles,
        123,
    )
    .unwrap();
    let genetic_before = split.genetic_fixed.clone();

    split
        .consolidate_lifetime(LifetimeConsolidationDelta {
            consolidated_synapses: 12,
            l2_norm_delta: 0.75,
        })
        .unwrap();

    assert_eq!(split.genetic_fixed, genetic_before);
    assert!(!split.genetic_fixed.mutable_during_lifetime());
    assert_eq!(split.lifetime_consolidated.consolidation_events, 1);
    assert_eq!(
        split.h_operational.descriptor.layer,
        WeightLayerKind::HOperational
    );
    assert_eq!(split.h_shadow.descriptor.layer, WeightLayerKind::HShadow);

    let effective = WEffective::from_components(EffectiveWeightSample {
        genetic_fixed: 0.25,
        lifetime_consolidated: -0.10,
        alpha: NormalizedScalar::new(0.5).unwrap(),
        h_operational: 0.20,
    })
    .unwrap();
    assert!((effective.value - 0.25).abs() < f32::EPSILON);
}
