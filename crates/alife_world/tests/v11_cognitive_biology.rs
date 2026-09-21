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

#[test]
fn new_game_inherited_turnover_preserves_legacy_identity_and_food_reserves() {
    use alife_core::{
        BiochemistryState, BodyEventDelta, FoundationWeightAsset, PassiveBodyUpkeepPolicy,
        SensorProfile,
    };
    use alife_world::{create_canonical_new_game, CanonicalNewGameConfig};

    // Missing fields decode to the exact old genotype/phenotype serialization.
    let legacy = record(1, 101);
    let legacy_bytes = serde_json::to_vec(legacy.genome()).unwrap();
    assert!(!String::from_utf8_lossy(&legacy_bytes).contains("metabolic_turnover"));
    let restored: CreatureGenome = serde_json::from_slice(&legacy_bytes).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), legacy_bytes);
    let legacy_phenotype_bytes = serde_json::to_vec(legacy.phenotype()).unwrap();
    assert!(!String::from_utf8_lossy(&legacy_phenotype_bytes).contains("metabolic_turnover"));
    let restored_phenotype: alife_core::CreaturePhenotype =
        serde_json::from_slice(&legacy_phenotype_bytes).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored_phenotype).unwrap(),
        legacy_phenotype_bytes
    );
    assert_eq!(restored_phenotype.body.metabolic_turnover, 1.0);

    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let game =
        create_canonical_new_game(&CanonicalNewGameConfig::phase3(37, 2).unwrap(), &foundation)
            .unwrap();
    let mut founder = game
        .world
        .organism_registry()
        .get(OrganismId(1))
        .unwrap()
        .clone();
    let other = game.world.organism_registry().get(OrganismId(2)).unwrap();
    let phenotype = founder.phenotype().clone();
    let rate = phenotype.body.metabolic_turnover;
    assert!((rate - 1.0 / 400.0).abs() < 1e-8);
    let child = CreatureGenome::reproduce(founder.genome(), other.genome(), 0xC0FF_EE01).unwrap();
    let child_rate = child.express().unwrap().body.metabolic_turnover;
    assert!((1.0 / 1024.0..=1.0).contains(&child_rate));
    assert!((0.5 * rate..2.0 * rate).contains(&child_rate));
    let founder_bytes = serde_json::to_vec(&founder).unwrap();
    let restored_founder: WorldOrganismRecord = serde_json::from_slice(&founder_bytes).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored_founder).unwrap(),
        founder_bytes
    );

    // Synthetic receipt totaling 2,899 work units, matching the observed debit
    // magnitude but not claiming the same GPU operation mix. Charge it through
    // the real world accounting entrypoint; this is never CPU cognition.
    let receipt = CognitiveWorkCounters::new(0, 2899, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
        .unwrap()
        .into_receipt()
        .unwrap();
    assert_eq!(receipt.weighted_total, 2899);
    let policy = CognitiveWorkCostPolicy::enabled(0.000001).unwrap();
    let cognitive_debit = founder.account_cognitive_work(receipt, policy).unwrap();
    assert!(cognitive_debit > 0.0);
    assert!((cognitive_debit - policy.energy_debit(&receipt).unwrap() * rate).abs() < 1e-8);

    // Ten minutes at the production normal 20 ticks/s, continuously moving,
    // unfed and charging active cognitive work every tick. No GPU is needed.
    let mut state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let initial_energy = state.body.energy;
    let mut estimated_active_minutes = 0.0;
    for tick in 1..=12_000 {
        state.body.debit_energy_pro_rata(cognitive_debit).unwrap();
        state = state
            .advance(
                Tick(tick),
                BodyEventDelta {
                    energy: -0.04,
                    ..BodyEventDelta::zero()
                },
                &phenotype,
            )
            .unwrap();
        assert!(state.body.energy > 0.0, "starvation at tick {tick}");
        if tick == 120 {
            let per_tick = (initial_energy - state.body.energy) / 120.0;
            estimated_active_minutes = initial_energy / per_tick / 20.0 / 60.0;
        }
    }
    assert!(
        (10.0..30.0).contains(&estimated_active_minutes),
        "active accounting estimate: {estimated_active_minutes} minutes"
    );
    assert!(state.body.energy < initial_energy - 0.1);
    assert_eq!(state.development.age_ticks, Tick(12_000));

    // Material food still replenishes reserve; starvation remains lethal.
    let fed = state
        .advance(
            Tick(12_001),
            BodyEventDelta {
                energy: 0.325,
                nutrition: 0.65,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    assert!(fed.body.energy > state.body.energy + 0.1);
    let moving_fed = state
        .advance(
            Tick(12_001),
            BodyEventDelta {
                energy: 0.325 - 0.04,
                nutrition: 0.65,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    let movement_cost = fed.body.energy - moving_fed.body.energy;
    assert!((movement_cost - 0.04 * rate * 1.1 / 6.0).abs() < 1e-6);
    state.body.set_energy(0.0).unwrap();
    assert!(PassiveBodyUpkeepPolicy::is_terminal(
        &state.body,
        0,
        &phenotype
    ));
}
