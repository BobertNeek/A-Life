#[cfg(feature = "gpu-tests")]
use alife_core::{
    ActionKind, BrainCapacityClass, CandidateActionFamily, CreatureGenome,
    FoundationGeneticIdentity, GenomeId, OrganismId, Validate,
};
use alife_core::{ActiveChallengeKind, ACTIVE_CHALLENGE_COUNT};
use alife_training::ActiveBatteryChallengeSpec;
#[cfg(feature = "gpu-tests")]
use alife_training::N2048ActiveBatteryRunner;

#[test]
fn every_active_challenge_has_a_bounded_production_world_spec() {
    let specs = ActiveBatteryChallengeSpec::all();
    assert_eq!(specs.len(), ACTIVE_CHALLENGE_COUNT);
    for (expected, spec) in ActiveChallengeKind::ALL.into_iter().zip(specs) {
        assert_eq!(spec.kind, expected);
        assert!(spec.tick_budget > 0 && spec.tick_budget <= 64);
        assert!(spec.world_object_count >= 2);
        assert!(spec.uses_grounded_sensing);
        assert!(!spec.slm_enabled);
    }
}

#[cfg(feature = "gpu-tests")]
#[test]
fn real_gpu_active_battery_measures_all_fifteen_challenges() {
    let mut runner = N2048ActiveBatteryRunner::new_required().unwrap();
    let evidence = runner
        .run_genetic_founder(OrganismId(7), 0xA11F_E4404)
        .unwrap();

    assert_eq!(evidence.receipt.completed_count(), ACTIVE_CHALLENGE_COUNT);
    assert_eq!(evidence.challenge_worlds, ACTIVE_CHALLENGE_COUNT as u32);
    assert!(evidence.gpu_dispatches >= ACTIVE_CHALLENGE_COUNT as u64);
    assert_eq!(evidence.gpu_dispatches, evidence.sealed_outcomes);
    assert!(!evidence.slm_enabled);
    assert!(!evidence.adapter_name.trim().is_empty());
    assert!(!evidence.backend_api.trim().is_empty());
}

#[cfg(feature = "gpu-tests")]
#[test]
fn real_gpu_battery_binds_the_exact_second_generation_creature_genome() {
    let foundation = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let founders = [11_u64, 12, 13, 14]
        .map(|seed| CreatureGenome::early_mammal_founder(seed, foundation).unwrap());
    let first = CreatureGenome::reproduce(&founders[0], &founders[1], 101).unwrap();
    let second = CreatureGenome::reproduce(&founders[2], &founders[3], 102).unwrap();
    let child = CreatureGenome::reproduce(&first, &second, 201).unwrap();
    let expressed = child.express().unwrap();

    let mut runner = N2048ActiveBatteryRunner::new_required().unwrap();
    let evidence = runner.run_creature_genome(OrganismId(17), &child).unwrap();

    assert_eq!(evidence.source_creature_genome_id, Some(child.id));
    assert_eq!(evidence.brain_genome_id, expressed.brain_genome.id);
    assert_eq!(evidence.parent_genome_ids, child.parent_genome_ids);
    assert_eq!(evidence.lineage_id, Some(child.lineage_id));
    assert_eq!(evidence.foundation_id, child.foundation.foundation_id);
    assert_eq!(
        evidence.foundation_version,
        u32::from(child.foundation.version)
    );
    assert_eq!(
        evidence.compatibility_family_id,
        child.foundation.compatibility_family_id
    );
    assert_ne!(evidence.brain_genome_id, GenomeId(0));
    assert_eq!(evidence.receipt.completed_count(), ACTIVE_CHALLENGE_COUNT);
    assert_eq!(evidence.gpu_dispatches, evidence.sealed_outcomes);
    assert!(evidence.sleep_consolidations >= 1);
}

#[cfg(feature = "gpu-tests")]
#[test]
fn real_gpu_reproduction_intent_targets_a_legal_mate_and_seals_the_outcome() {
    let foundation = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let initiator = CreatureGenome::early_mammal_founder(0xE10_901, foundation).unwrap();
    let mut runner = N2048ActiveBatteryRunner::new_required().unwrap();
    let receipt = runner
        .run_reproduction_intent(OrganismId(91), &initiator, OrganismId(92), 256)
        .unwrap();

    receipt.patch.validate_contract().unwrap();
    assert_eq!(receipt.initiator_organism_id, OrganismId(91));
    assert_eq!(receipt.mate_organism_id, OrganismId(92));
    assert_eq!(receipt.patch.pre_action().genome_id, initiator.id);
    assert_eq!(
        receipt.patch.decision().policy_backend(),
        alife_core::PolicyBackend::NeuralClosedLoopGpu
    );
    assert_eq!(
        receipt.patch.decision().selected_action.kind,
        ActionKind::Interact
    );
    assert_eq!(
        receipt
            .patch
            .decision()
            .neural_evidence()
            .unwrap()
            .action_family,
        CandidateActionFamily::Contact
    );
    assert_eq!(
        receipt.patch.decision().selected_action.target_entity,
        Some(receipt.mate_entity_id)
    );
    assert!(receipt.patch.outcome().success);
    assert!(receipt.observed_ticks > 0 && receipt.observed_ticks <= 256);
}

#[cfg(feature = "gpu-tests")]
#[test]
fn real_gpu_reproduction_intent_rejects_a_mismatched_foundation_identity() {
    let wrong_foundation = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5632,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let initiator = CreatureGenome::early_mammal_founder(0xE10_902, wrong_foundation).unwrap();
    let mut runner = N2048ActiveBatteryRunner::new_required().unwrap();

    let result = runner.run_reproduction_intent(OrganismId(93), &initiator, OrganismId(94), 8);

    assert!(result.is_err());
}
