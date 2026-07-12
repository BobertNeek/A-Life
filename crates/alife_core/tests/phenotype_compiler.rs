//! Contract tests for deterministic, compiler-owned production phenotypes.

use alife_core::{
    BrainCapacityClass, BrainClassId, BrainGenome, BrainScaleTier, DevelopmentState,
    LegacyBrainClassAdapter, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
    CANDIDATE_FEATURE_COUNT,
};

fn compile(class_id: BrainClassId, seed: u64) -> alife_core::BrainPhenotype {
    let capacity = BrainCapacityClass::production_for_id(class_id).unwrap();
    let genome = BrainGenome::scaffold(seed, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap()
}

#[test]
fn production_classes_compile_nonempty_with_stable_hashes() {
    for class_id in [
        BrainCapacityClass::N512_ID,
        BrainCapacityClass::N1024_ID,
        BrainCapacityClass::N2048_ID,
    ] {
        let one = compile(class_id, 41);
        let two = compile(class_id, 41);
        assert!(!one.projections().is_empty());
        assert!(one.synapses().len() >= 128);
        assert_eq!(one.phenotype_hash(), two.phenotype_hash());
        assert_eq!(
            one.budgets().global.total_synapses as usize,
            one.synapses().len()
        );
        assert!(
            one.budgets().global.total_synapses
                <= BrainCapacityClass::production_for_id(class_id)
                    .unwrap()
                    .execution()
                    .max_total_synapses()
        );
    }
}

#[test]
fn connectome_and_density_mutations_change_phenotype() {
    let capacity = BrainCapacityClass::n512();
    let mut genome = BrainGenome::scaffold(9, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let before = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    genome.sparse_density_priors[0].density = NormalizedScalar::new(0.08).unwrap();
    let after = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    assert_ne!(before.phenotype_hash(), after.phenotype_hash());
    assert_ne!(before.synapses().len(), after.synapses().len());
}

#[test]
fn large_named_tiers_are_research_gated() {
    for tier in [
        BrainScaleTier::Large4096,
        BrainScaleTier::Cognitive32768,
        BrainScaleTier::Student131k,
        BrainScaleTier::Ascended1M,
        BrainScaleTier::Ascended5M,
        BrainScaleTier::ResearchCustom,
    ] {
        let legacy_id = LegacyBrainClassAdapter::capacity_id_for_tier(tier);
        assert!(BrainCapacityClass::production_for_id(legacy_id).is_err());
    }
}

#[test]
fn same_id_with_forged_capacity_limits_is_rejected() {
    let mut json = serde_json::to_value(BrainCapacityClass::n512()).unwrap();
    json["execution"]["max_total_synapses"] = serde_json::json!(u32::MAX);
    assert!(serde_json::from_value::<BrainCapacityClass>(json).is_err());
}

#[test]
fn serialized_phenotype_is_rehashed_and_cannot_carry_stale_content() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 41);
    let mut json = serde_json::to_value(&phenotype).unwrap();
    json["microstep_count"] = serde_json::json!(4);
    assert!(serde_json::from_value::<alife_core::BrainPhenotype>(json).is_err());
}

#[test]
fn candidate_decoder_plan_covers_exactly_the_action_decoder_synapses() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 41);
    let decoder = phenotype.candidate_decoder();
    decoder.validate_against(&phenotype).unwrap();
    assert_eq!(
        decoder.decoder_synapse_count(),
        phenotype.budgets().global.action_decoder_synapses,
    );
    assert_eq!(decoder.feature_count(), CANDIDATE_FEATURE_COUNT as u16);
}

#[test]
fn maturation_compiles_the_immutable_two_three_four_microstep_schedule() {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(0x5ced_u64, capacity.id());
    let mut hashes = Vec::new();
    for (maturation, expected) in [(0.2_f32, 2_u8), (0.5, 3), (0.8, 4)] {
        let development = DevelopmentState::new(
            genome.id,
            Tick::ZERO,
            NormalizedScalar::new(maturation).unwrap(),
        );
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::PrivilegedAffordanceV1,
        )
        .unwrap();
        let replay = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::PrivilegedAffordanceV1,
        )
        .unwrap();
        assert_eq!(
            phenotype.microstep_count(),
            expected,
            "maturation={maturation}"
        );
        assert_eq!(phenotype.phenotype_hash(), replay.phenotype_hash());
        assert_eq!(phenotype, replay);
        hashes.push(phenotype.phenotype_hash());
    }
    assert_ne!(hashes[0], hashes[1]);
    assert_ne!(hashes[1], hashes[2]);
    assert_ne!(hashes[0], hashes[2]);
}

#[path = "phenotype_compiler/capacity.rs"]
mod capacity;
#[path = "phenotype_compiler/causal_routing.rs"]
mod causal_routing;

#[path = "phenotype_compiler/plans_persistence.rs"]
mod plans_persistence;
