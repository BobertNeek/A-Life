//! Contract tests for deterministic, compiler-owned production phenotypes.

use alife_core::{
    BrainCapacityClass, BrainClassId, BrainGenome, BrainScaleTier, ContinuousLocus, CreatureGenome,
    DevelopmentState, FoundationGeneticIdentity, LegacyBrainClassAdapter, NormalizedScalar,
    PhenotypeCompiler, SensorProfile, Tick, CANDIDATE_FEATURE_COUNT,
};
use alife_core::genome::CognitiveArchitectureGenomeParameters;

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

fn creature(seed: u64) -> CreatureGenome {
    creature_for_class(seed, BrainCapacityClass::N512_ID)
}

fn creature_for_class(seed: u64, class_id: BrainClassId) -> CreatureGenome {
    CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(10, 1, 7, class_id).unwrap(),
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
fn heritable_cognitive_architecture_compiles_into_phenotype_plan() {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(12, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let before = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let parameters = CognitiveArchitectureGenomeParameters::try_new_v1(
        1, 24, 6, 32, 0.08, 2, 16, 768, 64, 2, 0.45, 0.4, 0.7, 0.08, 0.06, 0.05, 0.04,
    )
    .unwrap();
    let changed_genome = genome.with_cognitive_architecture(parameters).unwrap();
    let after = PhenotypeCompiler::compile(
        &changed_genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();

    assert_eq!(after.cognitive_architecture().active_concept_limit(), 24);
    assert_eq!(after.cognitive_architecture().predictor_capacity(), 32);
    assert_eq!(after.cognitive_architecture().structural_edit_budget(), 2);
    assert_ne!(before.compiler_inputs_digest(), after.compiler_inputs_digest());
    assert_ne!(before.phenotype_hash(), after.phenotype_hash());
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

#[test]
fn reproduced_creature_genome_compiles_deterministically_with_non_founder_identity() {
    let maternal = creature(0xE10_0601);
    let paternal = creature(0xE10_0602);
    let child = CreatureGenome::reproduce(&maternal, &paternal, 0xE10_0603).unwrap();
    let expressed = child.express().unwrap();
    assert_ne!(expressed.brain_genome.id, maternal.id);
    assert_eq!(expressed.brain_genome.parent_genome_ids.len(), 2);

    let capacity =
        BrainCapacityClass::production_for_id(expressed.foundation.brain_class_id).unwrap();
    let development = expressed.development_state_at(Tick(4_000)).unwrap();
    let one = PhenotypeCompiler::compile(
        &expressed.brain_genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let two = PhenotypeCompiler::compile(
        &expressed.brain_genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    assert_eq!(one, two);
}

#[test]
fn creature_founders_and_offspring_compile_for_every_promoted_brain_class() {
    for (index, class_id) in [
        BrainCapacityClass::N512_ID,
        BrainCapacityClass::N1024_ID,
        BrainCapacityClass::N2048_ID,
    ]
    .into_iter()
    .enumerate()
    {
        let maternal = creature_for_class(0xE10_0651 + index as u64 * 2, class_id);
        let paternal = creature_for_class(0xE10_0652 + index as u64 * 2, class_id);
        let child =
            CreatureGenome::reproduce(&maternal, &paternal, 0xE10_0660 + index as u64).unwrap();
        for genome in [maternal, child] {
            let expressed = genome.express().unwrap();
            let capacity = BrainCapacityClass::production_for_id(class_id).unwrap();
            let development = expressed.development_state_at(Tick(4_000)).unwrap();
            PhenotypeCompiler::compile(
                &expressed.brain_genome,
                &capacity,
                &development,
                SensorProfile::PrivilegedAffordanceV1,
            )
            .unwrap();
        }
    }
}

#[test]
fn expressed_plasticity_changes_compiled_synapse_alpha() {
    let mut low = creature(0xE10_0701);
    low.brain.plasticity = ContinuousLocus::mean(0.20, 0.20).unwrap();
    let mut high = low.clone();
    high.brain.plasticity = ContinuousLocus::mean(0.80, 0.80).unwrap();
    let low = low.express().unwrap();
    let high = high.express().unwrap();
    let capacity = BrainCapacityClass::n512();
    let low_development = low.development_state_at(Tick(4_000)).unwrap();
    let high_development = high.development_state_at(Tick(4_000)).unwrap();

    let low_brain = PhenotypeCompiler::compile(
        &low.brain_genome,
        &capacity,
        &low_development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let high_brain = PhenotypeCompiler::compile(
        &high.brain_genome,
        &capacity,
        &high_development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();

    assert_eq!(low_brain.synapses().len(), high_brain.synapses().len());
    assert!(low_brain
        .synapses()
        .iter()
        .zip(high_brain.synapses())
        .any(|(left, right)| left.alpha() != right.alpha()));
}

#[path = "phenotype_compiler/capacity.rs"]
mod capacity;
#[path = "phenotype_compiler/causal_routing.rs"]
mod causal_routing;

#[path = "phenotype_compiler/learning.rs"]
mod learning;
#[path = "phenotype_compiler/plans_persistence.rs"]
mod plans_persistence;
