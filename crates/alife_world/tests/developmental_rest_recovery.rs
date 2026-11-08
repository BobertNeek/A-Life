//! Bounded canonical body fixtures, not neural experience or trained behavior.
//! Only the explicitly named authored-fatigue arm changes acquired chemistry.
use alife_core::*;
use alife_world::*;

const SEED: u64 = 961_060;
const SUBJECT: OrganismId = OrganismId(1);
const AUTHORED_FATIGUE: f32 = 0.65;

fn fresh_game() -> CanonicalNewGame {
    let config = CanonicalNewGameConfig::phase3(SEED, 1).unwrap();
    let foundation = FoundationWeightAsset::builtin_nano512_v1(config.sensor_profile).unwrap();
    create_canonical_new_game(&config, &foundation).unwrap()
}

fn record(world: &HeadlessWorld) -> &WorldOrganismRecord {
    world.organism_registry().get(SUBJECT).unwrap()
}

fn prepare_to(world: &mut HeadlessWorld, age: u64) {
    while world.tick().raw() < age {
        world.try_advance_tick().unwrap();
        let subject = record(world);
        assert_eq!(subject.biochemistry().development.age_ticks, world.tick());
        assert_eq!(subject.sleep_phase(), SleepPhase::Awake);
        assert_eq!(subject.sleep_work_units(), 0);
        if world.tick().raw() < 60 {
            assert_eq!(subject.biochemistry().homeostasis.drives.fatigue, 0.0);
            assert_eq!(
                subject.biochemistry().development.biochemical_expression,
                0.0
            );
        }
    }
}

fn authored_fatigue(world: &HeadlessWorld) -> HeadlessWorld {
    let mut result = world.clone();
    let before = record(world);
    let graph = &before.phenotype().chemistry.biochemical;
    let fatigue_species = graph
        .receptors()
        .iter()
        .find(|r| r.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Fatigue))
        .unwrap()
        .source;
    let fatigue_index = graph
        .species()
        .iter()
        .position(|s| s.id == fatigue_species)
        .unwrap();
    // Deserialize only this disposable test's chemistry, then derive its public
    // projection from the changed graph. Do not edit the inherited recipe,
    // developmental clocks, body, embodiment, hormones or other concentrations.
    let mut wire = serde_json::to_value(before.biochemistry()).unwrap();
    wire["graph_state"]["concentrations"][fatigue_index] = serde_json::json!(AUTHORED_FATIGUE);
    let mut biology: BiochemistryState = serde_json::from_value(wire).unwrap();
    biology.homeostasis = biology.graph_state().derive_homeostasis(graph).unwrap();
    biology.validate_against(before.phenotype()).unwrap();
    let mut registry = world.organism_registry().clone();
    registry
        .with_biology_mut(SUBJECT, |current| {
            *current = biology;
            Ok(())
        })
        .unwrap();
    result
        .replace_organism_registry_exact(registry.iter().cloned())
        .unwrap();
    let after = record(&result);
    assert_eq!(after.genome(), before.genome());
    assert_eq!(after.phenotype(), before.phenotype());
    assert_eq!(after.embodiment(), before.embodiment());
    assert_eq!(after.biochemistry().body, before.biochemistry().body);
    assert_eq!(
        after.biochemistry().development,
        before.biochemistry().development
    );
    assert_eq!(
        after.biochemistry().homeostasis.hormones,
        before.biochemistry().homeostasis.hormones
    );
    assert_eq!(after.state_graph().brain, before.state_graph().brain);
    assert_eq!(after.state_graph().memory, before.state_graph().memory);
    assert_eq!(
        after.biochemistry().homeostasis.drives.fatigue,
        AUTHORED_FATIGUE
    );
    for species in graph.species().iter().filter(|s| s.id != fatigue_species) {
        assert_eq!(
            after
                .biochemistry()
                .graph_state()
                .concentration(graph, species.id)
                .unwrap(),
            before
                .biochemistry()
                .graph_state()
                .concentration(graph, species.id)
                .unwrap()
        );
    }
    result
}

fn bundle(world: &HeadlessWorld, rest: bool, moving: bool) -> MotorCommandBundle {
    let mut channels = vec![ChannelCommand::new(
        MotorChannel::Posture,
        if rest {
            ActionKind::Rest
        } else {
            ActionKind::Idle
        }
        .canonical_id(),
        None,
        Vec3f::ZERO,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(1.0).unwrap(),
        0,
    )
    .unwrap()];
    if moving {
        // Idle itself spends energy. The exact control for moving Rest is the
        // same locomotion without a posture command, not locomotion plus Idle.
        if !rest {
            channels.clear();
        }
        channels.push(
            ChannelCommand::new(
                MotorChannel::Locomotion,
                HeadlessActionIds::STEP_FORWARD,
                None,
                Vec3f::new(1.0, 0.0, 0.0),
                Intensity::new(1.0).unwrap(),
                DurationTicks::new(1),
                0.0,
                Confidence::new(1.0).unwrap(),
                0,
            )
            .unwrap(),
        );
    }
    MotorCommandBundle::new(
        SUBJECT,
        ExperienceSequenceId(world.tick().raw() + 1),
        world.tick(),
        channels,
    )
    .unwrap()
}

fn act(
    world: &mut HeadlessWorld,
    rest: bool,
    moving: bool,
    label: &str,
) -> HeadlessMotorTransactionReceipt {
    let before = record(world).clone();
    let action = bundle(world, rest, moving);
    let receipt = world
        .apply_registered_motor_bundle(&action, before.world_entity_id())
        .unwrap();
    assert!(receipt.succeeded);
    assert_eq!(receipt.biology_before, *before.biochemistry());
    assert_eq!(receipt.biology_after, *record(world).biochemistry());
    assert_eq!(
        receipt.body_event.sleep_recovery,
        f32::from(rest && !moving)
    );
    assert_eq!(receipt.biology_after.body.sleeping, rest && !moving);
    assert_eq!(receipt.joint.execution.displacement != Vec3f::ZERO, moving);
    assert_eq!(receipt.body_event.nutrition, 0.0);
    assert_eq!(receipt.body_event.damage, 0.0);
    assert_eq!(
        receipt.biology_after,
        receipt
            .biology_before
            .advance_with_age(
                receipt.outcome_tick,
                receipt.outcome_tick,
                receipt.body_event,
                before.phenotype()
            )
            .unwrap()
    );
    assert_eq!(record(world).genome(), before.genome());
    assert_eq!(record(world).phenotype(), before.phenotype());
    assert_eq!(
        record(world).state_graph().brain,
        before.state_graph().brain
    );
    assert_eq!(
        record(world).state_graph().memory,
        before.state_graph().memory
    );
    assert_eq!(record(world).sleep_seal(), before.sleep_seal());
    assert_eq!(record(world).sleep_phase(), SleepPhase::Awake);
    assert_eq!(record(world).cognitive_work(), before.cognitive_work());
    let b = receipt.biology_before;
    let a = receipt.biology_after;
    let graph = &before.phenotype().chemistry.biochemical;
    let sleep_species = graph
        .receptors()
        .iter()
        .find(|r| r.target == BiochemicalTargetLocus::SleepMetabolicRate)
        .unwrap()
        .source;
    let sleep_state = a.graph_state().concentration(graph, sleep_species).unwrap();
    println!("ROW {label} age={}->{} action={} expression={:.9}->{:.9} maturation={:.9} development_last={} dose={:.1} body_sleep={} sleep_state={:.9} fatigue={:.9}->{:.9} energy={:.9}->{:.9} atp={:.9}->{:.9} displacement=({:.9},{:.9},{:.9}) event_energy={:.9} sleep_phase={:?} sleep_work={}",
        b.development.age_ticks.raw(), a.development.age_ticks.raw(),
        match (rest, moving) { (true, false) => "Rest", (false, false) => "Idle", (true, true) => "MovingRest", (false, true) => "MovementOnly" },
        b.development.biochemical_expression, a.development.biochemical_expression,
        a.development.maturation, a.development.last_update_tick.raw(),
        receipt.body_event.sleep_recovery, a.body.sleeping, sleep_state,
        b.homeostasis.drives.fatigue, a.homeostasis.drives.fatigue,
        b.body.energy, a.body.energy, b.homeostasis.drives.brain_atp, a.homeostasis.drives.brain_atp,
        receipt.joint.execution.displacement.x, receipt.joint.execution.displacement.y, receipt.joint.execution.displacement.z,
        receipt.body_event.energy, record(world).sleep_phase(), record(world).sleep_work_units());
    // The world closes the same interval, rather than advancing biology twice.
    assert_eq!(world.try_advance_tick().unwrap(), receipt.outcome_tick);
    assert_eq!(*record(world).biochemistry(), receipt.biology_after);
    receipt
}

#[test]
fn canonical_development_rest_idle_and_moving_controls() {
    let mut prepared = fresh_game().world;
    let genome = record(&prepared).genome().clone();
    let phenotype = record(&prepared).phenotype().clone();
    for age in [0, 59, 60, 240] {
        prepare_to(&mut prepared, age);
        assert_eq!(record(&prepared).genome(), &genome);
        assert_eq!(record(&prepared).phenotype(), &phenotype);
        assert_eq!(
            record(&prepared).biochemistry().cadence,
            BiochemistryCadence::early_mammal()
        );
        let authored = authored_fatigue(&prepared);
        for (label, initial) in [
            ("canonical", &prepared),
            ("authored-fatigue-0.65", &authored),
        ] {
            let rest = act(&mut initial.clone(), true, false, label);
            let idle = act(&mut initial.clone(), false, false, label);
            let moving_rest = act(&mut initial.clone(), true, true, label);
            let moving_only = act(&mut initial.clone(), false, true, label);
            assert_eq!(rest.biology_before, idle.biology_before);
            assert_eq!(moving_rest.biology_before, moving_only.biology_before);
            assert_eq!(moving_rest.body_event, moving_only.body_event);
            assert_eq!(moving_rest.biology_after, moving_only.biology_after);
            assert_eq!(
                moving_rest.joint.execution.displacement,
                moving_only.joint.execution.displacement
            );
            assert!(moving_rest.body_event.energy < 0.0);
            assert_eq!(rest.body_event.energy, 0.0);
            assert!(rest.biology_after.body.energy <= rest.biology_before.body.energy);
            assert_eq!(idle.body_event.energy, -0.01);
            let graph = &phenotype.chemistry.biochemical;
            let atp = graph
                .receptors()
                .iter()
                .find(|r| {
                    r.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::BrainAtp)
                })
                .unwrap()
                .source;
            assert!(!graph.emitters().iter().any(|emitter| emitter.source
                == BiochemicalSourceLocus::SleepRecovery
                && emitter.target == atp));
            let e = rest.biology_after.development.biochemical_expression;
            if age == 0 {
                assert_eq!(e, 0.0);
                assert_eq!(
                    rest.biology_after.homeostasis.drives.fatigue,
                    idle.biology_after.homeostasis.drives.fatigue
                );
                assert_eq!(
                    rest.biology_after.homeostasis.drives.fatigue,
                    rest.biology_before.homeostasis.drives.fatigue * 0.99995
                );
            } else {
                assert!(e > 0.0);
                assert!(
                    rest.biology_after.homeostasis.drives.fatigue
                        < idle.biology_after.homeostasis.drives.fatigue
                );
                if label == "authored-fatigue-0.65" && age < 240 {
                    let relief = idle.biology_after.homeostasis.drives.fatigue
                        - rest.biology_after.homeostasis.drives.fatigue;
                    assert!((relief - e * (0.70 + 0.00007)).abs() < 1e-6);
                }
            }
            if age == 59 {
                assert_eq!(rest.biology_before.development.biochemical_expression, 0.0);
                assert_eq!(rest.biology_after.development.last_update_tick, Tick(60));
            }
            if age == 240 {
                assert_eq!(e, 1.0);
                assert_eq!(rest.biology_after.homeostasis.drives.fatigue, 0.0);
            }
        }
    }
}

#[test]
fn exact_portable_continuation_across_development_boundary() {
    let game = fresh_game();
    let mut canonical = game.world;
    prepare_to(&mut canonical, 59);
    let authored = authored_fatigue(&canonical);
    for (label, initial) in [
        ("canonical-boundary", canonical),
        ("authored-boundary", authored),
    ] {
        // Encode only in memory. No lives, saves, or neural arrays are written.
        let mut creatures = game.creatures.clone();
        let biology = record(&initial).biochemistry();
        creatures[0].mind.tick = biology.tick;
        creatures[0].mind.homeostasis = biology.homeostasis;
        creatures[0].development_tick = biology.development.last_update_tick;
        let save = PortableSaveFile::from_headless_world(
            "disposable-development-boundary",
            &initial,
            RuntimeConfig::deterministic_default(SEED, BrainScaleTier::Nano512),
            AssetManifest::empty(),
            creatures,
        )
        .unwrap();
        let encoded = serde_json::to_string(&save).unwrap();
        let restored = PortableSaveFile::from_json_str(&encoded)
            .unwrap()
            .restore_headless_world()
            .unwrap();
        assert_eq!(record(&initial), record(&restored));
        assert_eq!(
            initial.canonical_signature_digest().unwrap(),
            restored.canonical_signature_digest().unwrap()
        );
        for rest in [false, true] {
            let mut uninterrupted = initial.clone();
            let mut resumed = restored.clone();
            let expected = act(&mut uninterrupted, rest, false, label);
            let actual = act(&mut resumed, rest, false, "restored-boundary");
            assert_eq!(expected, actual);
            assert_eq!(record(&uninterrupted), record(&resumed));
            assert_eq!(
                uninterrupted.canonical_signature_digest().unwrap(),
                resumed.canonical_signature_digest().unwrap()
            );
            println!("CONTINUATION {label} rest={rest} age=59->60 exact_receipt=true exact_record=true exact_world=true");
        }
    }
}
