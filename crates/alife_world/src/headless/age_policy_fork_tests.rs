//! Synthetic world-native sensing tests, not efficacy bodies.
use super::*;

const OBSERVER: OrganismId = OrganismId(1);

fn fixture(enabled: bool) -> (HeadlessWorld, WorldEntityId) {
    let mut world = HeadlessScenarioBuilder::new(2306)
        .agent("observer", OBSERVER, Vec3f::ZERO)
        .food("destination", Vec3f::new(3.0, 0.0, 0.0), 0.1)
        .build()
        .unwrap();
    let target = world
        .spawn_toy("carry", Vec3f::new(0.7, 0.0, 0.0), true)
        .unwrap();
    let mut genome = alife_core::CreatureGenome::early_mammal_founder(
        2306,
        alife_core::FoundationGeneticIdentity::new(
            10,
            1,
            7,
            alife_core::BrainCapacityClass::N512_ID,
        )
        .unwrap(),
    )
    .unwrap();
    if enabled {
        genome.brain.construction.maternal_architecture = genome
            .brain
            .construction
            .maternal_architecture
            .with_grip_proprioception_v5()
            .unwrap();
        genome.brain.construction.paternal_architecture = genome
            .brain
            .construction
            .paternal_architecture
            .with_grip_proprioception_v5()
            .unwrap();
    }
    let phenotype = genome.express().unwrap();
    assert_eq!(
        phenotype
            .brain_genome
            .cognitive_architecture()
            .grip_proprioception(),
        enabled
    );
    let biology = BiochemistryState::new(&phenotype, world.tick()).unwrap();
    let record = WorldOrganismRecord::new(
        OBSERVER,
        world.agent_entity_id(OBSERVER).unwrap(),
        genome,
        phenotype,
        biology,
        world.tick(),
    )
    .unwrap();
    world.register_organism_record(record).unwrap();
    (world, target)
}

fn raw(w: &HeadlessWorld) -> Vec<u8> {
    serde_json::to_vec(&w.save_state()).unwrap()
}
#[test]
fn age_policy_fork_changes_only_flag_and_original_newgame_guard_stays_strict() {
    let (mut world, _) = fixture(false);
    world.try_advance_tick().unwrap();
    let before = raw(&world);
    assert!(world.set_age_death_disabled_for_new_game(true).is_err());
    assert_eq!(raw(&world), before);
    let fork = world.fork_without_age_death_for_research().unwrap();
    assert!(fork.age_death_disabled());
    assert!(!world.age_death_disabled());
    let mut normalized = fork.save_state();
    normalized.disable_age_death = false;
    assert_eq!(serde_json::to_vec(&normalized).unwrap(), before);
    assert_ne!(
        world.canonical_signature_digest().unwrap(),
        fork.canonical_signature_digest().unwrap()
    );
    assert_eq!(raw(&world), before);
    assert!(fork.fork_without_age_death_for_research().is_err());
}
#[test]
fn age_policy_fork_rejects_empty_dead_and_currently_terminal_without_mutation() {
    let empty = HeadlessScenarioBuilder::new(2321).build().unwrap();
    assert!(empty.fork_without_age_death_for_research().is_err());
    for kind in 0..3 {
        let (mut world, _) = fixture(false);
        if kind == 0 {
            world
                .organism_registry
                .mark_dead(OBSERVER, Tick::ZERO)
                .unwrap();
        } else {
            world
                .organism_registry
                .with_biology_mut(OBSERVER, |b| {
                    if kind == 1 {
                        b.body.set_health(0.)?;
                    } else {
                        b.body.set_energy(0.)?;
                    }
                    Ok(())
                })
                .unwrap();
        }
        let before = raw(&world);
        assert!(world.fork_without_age_death_for_research().is_err());
        assert_eq!(raw(&world), before);
    }
}
#[test]
fn age_disabled_still_ages_and_health_energy_death_remain_native() {
    let (mut source, _) = fixture(false);
    let p = source
        .organism_registry
        .get(OBSERVER)
        .unwrap()
        .phenotype()
        .clone();
    let maximum = PassiveBodyUpkeepPolicy::maximum_lifespan_ticks(&p);
    // Synthetic native boundary setup only: no retained actor is edited.
    source.tick = Tick::new(maximum - 1);
    source
        .organism_registry
        .with_biology_mut(OBSERVER, |b| {
            *b = BiochemistryState::new_with_age(
                &p,
                Tick::new(maximum - 1),
                Tick::new(maximum - 1),
            )?;
            Ok(())
        })
        .unwrap();
    let mut fork = source.fork_without_age_death_for_research().unwrap();
    let original = raw(&source);
    source.try_advance_tick().unwrap();
    fork.try_advance_tick().unwrap();
    assert!(!source
        .organism_registry
        .get(OBSERVER)
        .unwrap()
        .lifecycle()
        .is_alive());
    assert!(fork
        .organism_registry
        .get(OBSERVER)
        .unwrap()
        .lifecycle()
        .is_alive());
    assert_eq!(
        fork.organism_registry
            .get(OBSERVER)
            .unwrap()
            .age_at(fork.tick())
            .unwrap()
            .raw(),
        maximum
    );
    assert_ne!(raw(&source), original);
    assert!(source.fork_without_age_death_for_research().is_err());
    for energy in [false, true] {
        let mut terminal = fork.clone();
        terminal
            .organism_registry
            .with_biology_mut(OBSERVER, |b| {
                if energy {
                    b.body.set_energy(0.)?;
                } else {
                    b.body.set_health(0.)?;
                }
                Ok(())
            })
            .unwrap();
        terminal.try_advance_tick().unwrap();
        assert!(!terminal
            .organism_registry
            .get(OBSERVER)
            .unwrap()
            .lifecycle()
            .is_alive());
    }
}

#[test]
fn alive_at_age_cutoff_is_terminal_even_before_native_retirement() {
    let (mut world, _) = fixture(false);
    let p = world
        .organism_registry
        .get(OBSERVER)
        .unwrap()
        .phenotype()
        .clone();
    let maximum = PassiveBodyUpkeepPolicy::maximum_lifespan_ticks(&p);
    world.tick = Tick::new(maximum);
    world
        .organism_registry
        .with_biology_mut(OBSERVER, |b| {
            *b = BiochemistryState::new_with_age(&p, Tick::new(maximum), Tick::new(maximum))?;
            Ok(())
        })
        .unwrap();
    assert!(world
        .organism_registry
        .get(OBSERVER)
        .unwrap()
        .lifecycle()
        .is_alive());
    let before = raw(&world);
    assert!(world.fork_without_age_death_for_research().is_err());
    assert_eq!(raw(&world), before);
}
#[test]
fn any_dead_or_terminal_peer_rejects_whole_world_fork() {
    for dead in [false, true] {
        let (mut world, _) = fixture(false);
        let record = world.organism_registry.get(OBSERVER).unwrap().clone();
        let peer = OrganismId(2);
        let entity = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "peer".into(),
                kind: WorldObjectKind::Agent,
                organism_id: Some(peer),
                position: Vec3f::new(5., 0., 0.),
                nutrition: 0.,
                hazard_pain: 0.,
                radius: 0.3,
                token_id: None,
            })
            .unwrap();
        let biology = BiochemistryState::new(record.phenotype(), world.tick()).unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::new(
                    peer,
                    entity,
                    record.genome().clone(),
                    record.phenotype().clone(),
                    biology,
                    world.tick(),
                )
                .unwrap(),
            )
            .unwrap();
        if dead {
            world
                .organism_registry
                .mark_dead(peer, world.tick())
                .unwrap();
        } else {
            world
                .organism_registry
                .with_biology_mut(peer, |b| {
                    b.body.set_energy(0.)?;
                    Ok(())
                })
                .unwrap();
        }
        assert!(world
            .organism_registry
            .get(OBSERVER)
            .unwrap()
            .lifecycle()
            .is_alive());
        let before = raw(&world);
        assert!(world.fork_without_age_death_for_research().is_err());
        assert_eq!(raw(&world), before);
    }
}
