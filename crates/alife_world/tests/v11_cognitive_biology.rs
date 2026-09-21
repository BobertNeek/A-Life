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
        AlleleSide, BiochemicalDriveChannel, BiochemicalSourceLocus, BiochemicalTargetLocus,
        BiochemistryState, BodyEventDelta, FoundationWeightAsset, PassiveBodyUpkeepPolicy,
        SensorProfile,
    };
    use alife_world::{create_canonical_new_game, CanonicalNewGameConfig, HeadlessWorldCommand};

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
    let hunger_gain = |genome: &CreatureGenome| {
        let graph = genome.chemistry.graph.expressed();
        let hunger = graph
            .receptors()
            .iter()
            .find(|r| r.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Hunger))
            .unwrap()
            .source;
        graph
            .emitters()
            .iter()
            .find(|e| e.source == BiochemicalSourceLocus::EnergyDeficit && e.target == hunger)
            .unwrap()
            .gain
    };
    assert_eq!(hunger_gain(&restored), 0.18);

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
    assert_eq!(hunger_gain(founder.genome()), 0.03);
    assert_eq!(hunger_gain(&child), 0.03);
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

    // Mature, unfed biology: reserve loss strengthens hunger without saturating
    // a well provisioned founder. Regulatory chemistry retains its fast cadence.
    let mut reserves = [0.7925, 0.2, 0.01].map(|energy| {
        let mut body = BiochemistryState::new(&phenotype, Tick(240)).unwrap();
        body.body.set_energy(energy).unwrap();
        body
    });
    for tick in 241..=480 {
        for body in &mut reserves {
            *body = body
                .advance(Tick(tick), BodyEventDelta::zero(), &phenotype)
                .unwrap();
        }
    }
    let [provisioned, low, depleted] = reserves;
    assert!((0.3..0.55).contains(&provisioned.homeostasis.drives.hunger));
    assert!(provisioned.homeostasis.drives.hunger < low.homeostasis.drives.hunger);
    assert!(low.homeostasis.drives.hunger < depleted.homeostasis.drives.hunger);
    assert!(low.homeostasis.drives.fatigue > provisioned.homeostasis.drives.fatigue);
    assert_eq!(provisioned.body.health, 1.0);
    assert!(
        provisioned.body.energy < 0.7925,
        "healthy passive biology must still spend reserve"
    );
    let mut rest_world = game.world.clone();
    let rest_receipt = rest_world
        .apply_registered_command(
            &HeadlessWorldCommand::rest(OrganismId(1)).unwrap(),
            game.receipt.founders[0].world_entity_id,
            Tick(1),
        )
        .unwrap();
    assert!(rest_receipt.biology_after.body.energy < rest_receipt.biology_before.body.energy);
    let rest = low
        .advance(Tick(481), rest_receipt.action_result.body_event, &phenotype)
        .unwrap();
    assert!(rest.homeostasis.drives.fatigue < low.homeostasis.drives.fatigue);
    assert!(rest.body.energy < low.body.energy);
    assert!(
        low.body.energy - rest.body.energy < 0.0001,
        "Rest effort must use inherited turnover"
    );

    // Damage stays physical (not divided by metabolic turnover). Once acute
    // pain subsides, inherited basal repair still heals existing damage and
    // spends exactly the additional reserve; it cannot heal an empty body.
    let injured = provisioned
        .advance(
            Tick(481),
            BodyEventDelta {
                damage: 0.12,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    assert!((provisioned.body.health - injured.body.health - 0.03248).abs() < 0.0001);
    let resting_injured = injured
        .advance(
            Tick(482),
            BodyEventDelta {
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    assert_eq!(resting_injured.homeostasis.drives.pain, 0.0);
    let mut no_basal_repair = founder.genome().clone();
    let repair_receptor = no_basal_repair
        .chemistry
        .graph
        .expressed()
        .receptors()
        .iter()
        .position(|r| r.target == BiochemicalTargetLocus::OrganRepair)
        .unwrap();
    for allele in [AlleleSide::Maternal, AlleleSide::Paternal] {
        no_basal_repair.chemistry.graph = no_basal_repair
            .chemistry
            .graph
            .clone()
            .with_receptor_nominal(allele, repair_receptor, 0.0)
            .unwrap();
    }
    let unrepaired = resting_injured
        .advance(
            Tick(494),
            BodyEventDelta::zero(),
            &no_basal_repair.express().unwrap(),
        )
        .unwrap();
    let repaired = resting_injured
        .advance(Tick(494), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    let healing = repaired.body.health - unrepaired.body.health;
    let reserve_cost = unrepaired.body.energy - repaired.body.energy;
    assert!(healing > 0.0 && reserve_cost > 0.0);
    assert!((healing - reserve_cost).abs() < 1e-6);
    let mut exhausted_injured = resting_injured;
    exhausted_injured.body.set_energy(0.0).unwrap();
    let exhausted_after = exhausted_injured
        .advance(Tick(494), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert_eq!(exhausted_after.body.health, exhausted_injured.body.health);

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
    assert!(fed.homeostasis.drives.hunger < state.homeostasis.drives.hunger);
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
