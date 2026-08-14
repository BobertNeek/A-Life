use alife_core::cognitive_work::{CognitiveWorkCostPolicy, CognitiveWorkCounters};
use alife_core::{
    BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, OrganismId, Tick, WorldEntityId,
};
use alife_world::WorldOrganismRecord;

fn record(organism_id: u64, world_entity_id: u64) -> WorldOrganismRecord {
    let genome = CreatureGenome::early_mammal_founder(
        0xE10_3200 + organism_id,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    WorldOrganismRecord::newborn(
        OrganismId(organism_id),
        WorldEntityId(world_entity_id),
        genome,
        phenotype,
        Tick::ZERO,
    )
    .unwrap()
}

#[test]
fn cognitive_work_is_deterministic_and_policy_cost_reaches_authoritative_biology() {
    let low_counters = CognitiveWorkCounters::new(2, 3, 1, 2, 1, 0, 1, 2, 0, 1, 1, 0).unwrap();
    let high_counters = CognitiveWorkCounters::new(3, 5, 2, 4, 2, 1, 1, 3, 1, 2, 2, 1).unwrap();
    let low_receipt = low_counters.into_receipt().unwrap();
    let high_receipt = high_counters.into_receipt().unwrap();
    let repeated_low_receipt = low_counters.into_receipt().unwrap();

    assert_ne!(low_receipt, high_receipt);
    assert_eq!(low_receipt, repeated_low_receipt);
    assert_eq!(
        low_receipt.canonical_digest().unwrap(),
        repeated_low_receipt.canonical_digest().unwrap()
    );

    let disabled = CognitiveWorkCostPolicy::disabled();
    let mut free_low = record(1, 101);
    let mut free_high = record(2, 102);
    let low_energy_before = free_low.biochemistry().body.energy;
    let high_energy_before = free_high.biochemistry().body.energy;
    assert_eq!(
        free_low
            .account_cognitive_work(low_receipt, disabled)
            .unwrap(),
        0.0
    );
    assert_eq!(
        free_high
            .account_cognitive_work(high_receipt, disabled)
            .unwrap(),
        0.0
    );
    assert_eq!(free_low.biochemistry().body.energy, low_energy_before);
    assert_eq!(free_high.biochemistry().body.energy, high_energy_before);
    assert_eq!(free_low.cognitive_energy_debit(), 0.0);
    assert_eq!(free_high.cognitive_energy_debit(), 0.0);

    let enabled = CognitiveWorkCostPolicy::enabled(0.001).unwrap();
    let mut charged_low = record(3, 103);
    let mut charged_high = record(4, 104);
    let low_energy_before = charged_low.biochemistry().body.energy;
    let high_energy_before = charged_high.biochemistry().body.energy;
    let low_debit = charged_low
        .account_cognitive_work(low_receipt, enabled)
        .unwrap();
    let high_debit = charged_high
        .account_cognitive_work(high_receipt, enabled)
        .unwrap();

    assert!(high_debit > low_debit);
    assert_eq!(charged_low.cognitive_energy_debit(), low_debit);
    assert_eq!(charged_high.cognitive_energy_debit(), high_debit);
    assert!((low_energy_before - charged_low.biochemistry().body.energy - low_debit).abs() < 1e-6);
    assert!(
        (high_energy_before - charged_high.biochemistry().body.energy - high_debit).abs() < 1e-6
    );
    assert_eq!(charged_low.cognitive_work(), &low_receipt);
    assert_eq!(charged_high.cognitive_work(), &high_receipt);
    charged_low.validate_contract().unwrap();
    charged_high.validate_contract().unwrap();
}
