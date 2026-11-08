//! Fixed biology-only fixtures. Commands are scripted, never neural choices.
use super::*;
use crate::*;
use alife_core::*;

const SEED: u64 = 0x0343_4D31_4658_0001;
const SUBJECT: OrganismId = OrganismId(1);
const HORIZON: u64 = 4514;

fn fresh(calibrated: bool) -> CanonicalNewGame {
    let config = CanonicalNewGameConfig::phase3(SEED, 1).unwrap();
    if calibrated {
        create_canonical_new_game_with_recovery_timescale_research_v1(&config).unwrap()
    } else {
        create_canonical_new_game_with_awake_rest_research_v1(&config).unwrap()
    }
}
fn subject(world: &HeadlessWorld) -> &WorldOrganismRecord {
    world.organism_registry().get(SUBJECT).unwrap()
}
fn prepare(world: &mut HeadlessWorld, age: u64) {
    while world.tick().raw() < age {
        world.try_advance_tick().unwrap();
    }
    assert_eq!(
        subject(world).biochemistry().development.age_ticks,
        Tick(age)
    );
}
fn levels(world: &HeadlessWorld) -> (f32, f32) {
    let h = subject(world).biochemistry().homeostasis;
    (h.drives.fatigue, h.hormones.sleep_pressure)
}
fn gain(graph: &BiochemicalPhenotype, id: u16) -> f32 {
    let rows: Vec<_> = graph
        .emitters()
        .iter()
        .filter(|e| e.target == ChemicalSpeciesId(id) && e.source == BiochemicalSourceLocus::Awake)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0].gain
}
fn digest<T: serde::Serialize>(value: &T) -> PortableAssetDigest {
    PortableAssetDigest::for_bytes(&serde_json::to_vec(value).unwrap())
}
fn bundle(world: &HeadlessWorld, posture: ActionId) -> MotorCommandBundle {
    MotorCommandBundle::new(
        SUBJECT,
        ExperienceSequenceId(world.tick().raw() + 1),
        world.tick(),
        vec![ChannelCommand::new(
            MotorChannel::Posture,
            posture,
            None,
            Vec3f::ZERO,
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            0.0,
            Confidence::new(1.0).unwrap(),
            0,
        )
        .unwrap()],
    )
    .unwrap()
}
#[derive(Clone, Copy, Debug)]
enum Scripted {
    Neutral,
    AwakeRest,
    BodySleep,
}
fn act(world: &mut HeadlessWorld, action: Scripted) -> AwakeRestReceiptV1 {
    let before = subject(world).clone();
    let posture = match action {
        Scripted::BodySleep => ActionKind::Rest.canonical_id(),
        _ => HeadlessActionIds::NO_POSTURE,
    };
    let command = bundle(world, posture);
    let result = match action {
        Scripted::AwakeRest => world
            .apply_awake_rest_research_v1(
                &AwakeRestRequestV1::new(command.clone()).unwrap(),
                before.world_entity_id(),
            )
            .unwrap(),
        _ => AwakeRestReceiptV1 {
            motor: world
                .apply_registered_motor_bundle(&command, before.world_entity_id())
                .unwrap(),
            recovery: AwakeRestEventV1::NONE,
        },
    };
    let m = &result.motor;
    assert_eq!(m.body_event.nutrition, 0.0);
    assert_eq!(m.body_event.player_reward, 0.0);
    assert_eq!(m.body_event.energy, 0.0);
    assert_eq!(m.biology_before, *before.biochemistry());
    assert_eq!(
        m.biology_after,
        before
            .biochemistry()
            .advance_with_body_research_events_v1(
                m.outcome_tick,
                Tick(before.biochemistry().development.age_ticks.raw() + 1),
                m.body_event,
                None,
                before.phenotype(),
                ThermalBalanceCandidate::default(),
                result.recovery,
            )
            .unwrap()
    );
    assert_eq!(subject(world).genome(), before.genome());
    assert_eq!(subject(world).phenotype(), before.phenotype());
    assert_eq!(
        subject(world).state_graph().brain,
        before.state_graph().brain
    );
    assert_eq!(
        subject(world).state_graph().memory,
        before.state_graph().memory
    );
    assert_eq!(subject(world).sleep_phase(), SleepPhase::Awake);
    assert_eq!(subject(world).sleep_work_units(), 0);
    let bytes = serde_json::to_vec(&world.save_state()).unwrap();
    assert!(world
        .apply_registered_motor_bundle(&command, before.world_entity_id())
        .is_err());
    assert_eq!(bytes, serde_json::to_vec(&world.save_state()).unwrap());
    assert_eq!(world.try_advance_tick().unwrap(), m.outcome_tick);
    assert_eq!(
        subject(world).biochemistry(),
        &m.biology_after,
        "biology must advance once"
    );
    println!("RECOVERY_ACTION scripted={action:?} age={} F={:.9}->{:.9} P={:.9}->{:.9} awake_dose={} sleep_dose={} sleeping={} energy={:.9}->{:.9} atp={:.9}->{:.9}",
        m.biology_before.development.age_ticks.raw(),
        m.biology_before.homeostasis.drives.fatigue, m.biology_after.homeostasis.drives.fatigue,
        m.biology_before.homeostasis.hormones.sleep_pressure, m.biology_after.homeostasis.hormones.sleep_pressure,
        result.recovery.dose(), m.body_event.sleep_recovery, m.biology_after.body.sleeping,
        m.biology_before.body.energy, m.biology_after.body.energy,
        m.biology_before.homeostasis.drives.brain_atp, m.biology_after.homeostasis.drives.brain_atp);
    result
}

#[test]
fn recovery_timescale_fresh_only_changes_two_inherited_rates_and_no_clock_or_value() {
    let control = fresh(false);
    let candidate = fresh(true);
    let old = subject(&control.world);
    let new = subject(&candidate.world);
    let mut expected = old.genome().clone();
    for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
        expected.chemistry.graph = expected
            .chemistry
            .graph
            .with_recovery_timescale_v1(side)
            .unwrap();
    }
    assert_eq!(&expected, new.genome());
    assert_eq!(old.phenotype().body, new.phenotype().body);
    assert_eq!(old.phenotype().development, new.phenotype().development);
    assert_eq!(old.phenotype().brain_genome, new.phenotype().brain_genome);
    let a = &old.phenotype().chemistry.biochemical;
    let b = &new.phenotype().chemistry.biochemical;
    let mut expected_graph = serde_json::to_value(a).unwrap();
    for e in expected_graph["emitters"].as_array_mut().unwrap() {
        if e["source"] == "Awake" {
            match e["target"].as_u64().unwrap() {
                2 => e["gain"] = serde_json::json!(0.0012_f32),
                18 => e["gain"] = serde_json::json!(0.0006_f32),
                _ => (),
            }
        }
    }
    assert_eq!(expected_graph, serde_json::to_value(b).unwrap());
    assert_eq!(a.value_profile(), b.value_profile());
    assert_eq!(gain(a, 2), 0.00007);
    assert_eq!(gain(a, 18), 0.00007);
    assert_eq!(gain(b, 2), 0.0012);
    assert_eq!(gain(b, 18), 0.0006);
    assert_eq!(old.biochemistry(), new.biochemistry());
    assert!(!candidate.world.age_death_disabled());
    let p = new.phenotype();
    assert_eq!(PassiveBodyUpkeepPolicy::maximum_lifespan_ticks(p), HORIZON);
    assert_eq!(p.development.maturation_duration_ticks, 1824);
    assert_eq!(p.development.puberty_tick, Tick(1113));
    for (id, target) in [
        (
            2,
            BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Fatigue),
        ),
        (
            18,
            BiochemicalTargetLocus::Endocrine(EndocrineChannel::SleepPressure),
        ),
    ] {
        let s = b
            .species()
            .iter()
            .find(|s| s.id == ChemicalSpeciesId(id))
            .unwrap();
        assert_eq!((s.baseline, s.decay_retention), (0.2, 0.99995));
        let r = b.receptors().iter().find(|r| r.target == target).unwrap();
        assert_eq!(
            (r.source, r.threshold, r.gain, r.nominal, r.digital),
            (ChemicalSpeciesId(id), 0.0, 1.0, 0.0, false)
        );
        println!(
            "RECOVERY_PARAMETERS id={id} baseline={} retention={} awake_gain={} receptor={r:?}",
            s.baseline,
            s.decay_retention,
            gain(b, id)
        );
    }
    println!("RECOVERY_IDENTITIES seed={SEED:#x} baseline_genotype={:?} candidate_genotype={:?} baseline_graph={:?} candidate_graph={:?} foundation={:?} clock={:?} lifespan={} turnover={} value={:?}",
        digest(old.genome()), digest(new.genome()), digest(a), digest(b), digest(&new.genome().foundation),
        new.biochemistry().cadence, HORIZON, p.body.metabolic_turnover, b.value_profile());
    // Identical canonical body recipe across brain classes, without executing a brain.
    let mut config = CanonicalNewGameConfig::phase3(SEED, 1).unwrap();
    let builtin = FoundationWeightAsset::builtin_nano512_v1(config.sensor_profile).unwrap();
    println!(
        "RECOVERY_ASSET canonical_nano512_bytes={:?}",
        PortableAssetDigest::for_bytes(&builtin.encode_canonical().unwrap())
    );
    let default_bytes = serde_json::to_vec(
        &create_canonical_new_game(&config, &builtin)
            .unwrap()
            .world
            .save_state(),
    )
    .unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    let n2048 = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile).unwrap();
    let n2048_game = create_canonical_new_game_with_n2048_candidate(&config, &n2048).unwrap();
    let n = subject(&n2048_game.world);
    assert_eq!(n.phenotype().body, old.phenotype().body);
    assert_eq!(n.phenotype().development, old.phenotype().development);
    for id in [2, 18] {
        assert_eq!(gain(&n.phenotype().chemistry.biochemical, id), gain(a, id));
    }
    config.brain_class = BrainScaleTier::Nano512;
    assert_eq!(
        default_bytes,
        serde_json::to_vec(
            &create_canonical_new_game(&config, &builtin)
                .unwrap()
                .world
                .save_state()
        )
        .unwrap()
    );
    assert_eq!(
        digest(&builtin.encode_canonical().unwrap()),
        digest(
            &FoundationWeightAsset::builtin_nano512_v1(config.sensor_profile)
                .unwrap()
                .encode_canonical()
                .unwrap()
        )
    );
}

#[test]
fn recovery_timescale_natural_buildup_respects_development_and_unchanged_age_death() {
    for calibrated in [false, true] {
        let mut world = fresh(calibrated).world;
        let mut first_f = None;
        let mut first_p = None;
        let mut full = None;
        let mut at240 = (0.0, 0.0);
        for age in 1..=HORIZON {
            world.try_advance_tick().unwrap();
            let record = subject(&world);
            let biology = record.biochemistry();
            let (f, p) = levels(&world);
            if f >= 0.35 {
                first_f.get_or_insert(age);
            }
            if p >= 0.6 {
                first_p.get_or_insert(age);
            }
            if biology.development.biochemical_expression == 1.0 {
                full.get_or_insert(age);
            }
            if age == 240 {
                at240 = (f, p);
            }
            if [59, 60, 179, 180, 239, 240, 4513, 4514].contains(&age) {
                println!("RECOVERY_BOUNDARY calibrated={calibrated} age={age} expression={} F={f} P={p} energy={} alive={}",biology.development.biochemical_expression,biology.body.energy,record.lifecycle().is_alive());
            }
            assert_eq!(record.lifecycle().is_alive(), age < HORIZON);
            assert_eq!(biology.tick, Tick(age));
            assert!(!biology.body.sleeping);
        }
        assert_eq!(full, Some(240));
        if calibrated {
            assert!((292..=532).contains(&first_f.unwrap()));
            assert!((1009..=1249).contains(&first_p.unwrap()));
        } else {
            assert_eq!((first_f, first_p), (None, None));
            assert_eq!(at240.0, at240.1);
            assert!((at240.1 - 0.008661905).abs() < 1e-8);
            assert!(levels(&world).1 < 0.323277);
        }
        println!("RECOVERY_NATURAL calibrated={calibrated} F_reference_first={first_f:?} P_reference_first={first_p:?} full_expression={full:?} at240={at240:?} age_death={HORIZON} final={:?}",levels(&world));
    }
}

#[test]
fn recovery_timescale_natural_need_real_awake_rest_and_sleep_have_distinct_effects() {
    for calibrated in [false, true] {
        let mut initial = fresh(calibrated).world;
        prepare(&mut initial, 1200);
        let awake = act(&mut initial.clone(), Scripted::AwakeRest);
        let neutral = act(&mut initial.clone(), Scripted::Neutral);
        let sleep = act(&mut initial.clone(), Scripted::BodySleep);
        assert_eq!(awake.recovery.dose(), 1.0);
        assert_eq!(awake.motor.body_event.sleep_recovery, 0.0);
        assert!(!awake.motor.biology_after.body.sleeping);
        assert_eq!(
            awake.motor.biology_after.body,
            neutral.motor.biology_after.body
        );
        assert_eq!(
            awake.motor.biology_after.homeostasis.hormones,
            neutral.motor.biology_after.homeostasis.hormones
        );
        assert!(
            awake.motor.biology_after.homeostasis.drives.fatigue
                < neutral.motor.biology_after.homeostasis.drives.fatigue
        );
        assert!(
            awake
                .motor
                .biology_after
                .homeostasis
                .hormones
                .sleep_pressure
                > awake
                    .motor
                    .biology_before
                    .homeostasis
                    .hormones
                    .sleep_pressure
        );
        assert_eq!(sleep.motor.body_event.sleep_recovery, 1.0);
        assert!(sleep.motor.biology_after.body.sleeping);
        assert!(
            sleep
                .motor
                .biology_after
                .homeostasis
                .hormones
                .sleep_pressure
                < awake
                    .motor
                    .biology_after
                    .homeostasis
                    .hormones
                    .sleep_pressure
        );
        assert!(awake.motor.biology_after.body.energy <= awake.motor.biology_before.body.energy);
        assert!(
            awake.motor.biology_after.homeostasis.drives.brain_atp
                <= awake.motor.biology_before.homeostasis.drives.brain_atp
        );
    }
}

#[test]
fn recovery_timescale_equal_scripted_cycles_allow_natural_rebuildup_within_life() {
    for calibrated in [false, true] {
        let mut world = fresh(calibrated).world;
        let mut sleeps = 0;
        let mut rests = 0;
        while world.tick().raw() < 4400 {
            let age = world.tick().raw();
            if [1200, 2200, 3200, 4200].contains(&age) {
                let (f, p) = levels(&world);
                if calibrated {
                    assert!(p >= 0.55, "age={age} pressure={p}");
                } else {
                    assert!(p < 0.1);
                }
                let receipt = act(&mut world, Scripted::BodySleep);
                assert!(
                    receipt
                        .motor
                        .biology_after
                        .homeostasis
                        .hormones
                        .sleep_pressure
                        < p
                );
                println!(
                    "RECOVERY_CYCLE calibrated={calibrated} cycle={} age={age} F={f} P={p}",
                    sleeps + 1
                );
                sleeps += 1;
            } else if [400, 700, 1450, 1750, 2450, 2750, 3450, 3750].contains(&age) {
                let receipt = act(&mut world, Scripted::AwakeRest);
                assert!(
                    receipt.motor.biology_after.homeostasis.drives.fatigue
                        < receipt.motor.biology_before.homeostasis.drives.fatigue
                );
                rests += 1;
            } else {
                world.try_advance_tick().unwrap();
            }
            assert!(subject(&world).lifecycle().is_alive());
        }
        assert_eq!((sleeps, rests), (4, 8));
        assert!(levels(&world).0 > 0.0 && levels(&world).1 > 0.0);
    }
}

#[test]
fn recovery_timescale_save_restore_preserves_accumulation_and_exact_continuation() {
    for calibrated in [false, true] {
        let mut game = fresh(calibrated);
        for age in [240, 1200, 2200] {
            prepare(&mut game.world, age);
            let b = subject(&game.world).biochemistry();
            game.creatures[0].mind.tick = b.tick;
            game.creatures[0].mind.homeostasis = b.homeostasis;
            game.creatures[0].development_tick = b.development.last_update_tick;
            let save = PortableSaveFile::from_headless_world(
                "fresh-recovery-timescale-v1",
                &game.world,
                RuntimeConfig::deterministic_default(SEED, BrainScaleTier::Nano512),
                AssetManifest::empty(),
                game.creatures.clone(),
            )
            .unwrap();
            let encoded = serde_json::to_string(&save).unwrap();
            let restored_save = PortableSaveFile::from_json_str(&encoded).unwrap();
            assert_eq!(encoded, serde_json::to_string(&restored_save).unwrap());
            let restored = restored_save.restore_headless_world().unwrap();
            assert_eq!(subject(&game.world), subject(&restored));
            assert_eq!(
                digest(&game.world.save_state()),
                digest(&restored.save_state())
            );
            for action in [Scripted::Neutral, Scripted::AwakeRest, Scripted::BodySleep] {
                let mut a = game.world.clone();
                let mut b = restored.clone();
                assert_eq!(act(&mut a, action), act(&mut b, action));
                assert_eq!(digest(&a.save_state()), digest(&b.save_state()));
            }
            act(&mut game.world, Scripted::BodySleep);
        }
    }
}

#[test]
fn recovery_timescale_inheritance_keeps_genes_and_valid_mutants_without_runtime_gate() {
    let control = subject(&fresh(false).world).genome().clone();
    let candidate = subject(&fresh(true).world).genome().clone();
    let mut heterozygous = candidate.clone();
    heterozygous.chemistry.graph = BiochemicalGraphChromosome::new(
        candidate.chemistry.graph.expressed().clone(),
        control.chemistry.graph.expressed().clone(),
        AlleleSide::Maternal,
    )
    .unwrap();
    let mixed = heterozygous.express().unwrap();
    assert_eq!(
        gain(&mixed.chemistry.biochemical, 2),
        (0.0012 + 0.00007) * 0.5
    );
    assert_eq!(
        gain(&mixed.chemistry.biochemical, 18),
        (0.0006 + 0.00007) * 0.5
    );
    for (seed, rate) in [(901, None), (902, Some(0.0)), (903, Some(1.0))] {
        let mut mother = candidate.clone();
        if let Some(rate) = rate {
            mother.reproduction.mutation_rate = ContinuousLocus::mean(rate, rate).unwrap();
            mother.reproduction.discrete_mutation_rate = ContinuousLocus::mean(0.0, 0.0).unwrap();
        }
        let before = serde_json::to_vec(&mother).unwrap();
        let child = CreatureGenome::reproduce(&mother, &mother, seed).unwrap();
        assert_eq!(before, serde_json::to_vec(&mother).unwrap());
        let phenotype = child.express().unwrap();
        let bytes = serde_json::to_vec(&child).unwrap();
        let restored: CreatureGenome = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
        assert_eq!(restored.express().unwrap(), phenotype);
        if rate == Some(0.0) {
            for side in ["maternal", "paternal"] {
                let wire = serde_json::to_value(&child.chemistry.graph).unwrap();
                let graph: BiochemicalPhenotype =
                    serde_json::from_value(wire[side].clone()).unwrap();
                assert_eq!(gain(&graph, 2), 0.0012);
                assert_eq!(gain(&graph, 18), 0.0006);
            }
        }
        if rate == Some(1.0) {
            assert!(!child.provenance.mutations.is_empty());
        }
        let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        state
            .advance_with_age(Tick(1), Tick(1), BodyEventDelta::zero(), &phenotype)
            .unwrap()
            .validate_against(&phenotype)
            .unwrap();
        println!("RECOVERY_INHERITANCE seed={seed} authored_mutation_rate={rate:?} mutations={} F_gain={} P_gain={} valid_ordinary_advance=true",child.provenance.mutations.len(),gain(&phenotype.chemistry.biochemical,2),gain(&phenotype.chemistry.biochemical,18));
    }
}

#[test]
fn recovery_timescale_separate_1100_script_records_four_real_pressure_crossings() {
    for calibrated in [false, true] {
        let mut world = fresh(calibrated).world;
        let mut crossings = Vec::new();
        let mut sleeps = 0;
        while world.tick().raw() < HORIZON {
            let age = world.tick().raw();
            let previous = levels(&world).1;
            if [1200, 2300, 3400, 4500].contains(&age) {
                if calibrated {
                    assert!(previous >= 0.6);
                    assert_eq!(crossings.len(), sleeps + 1);
                } else {
                    assert!(previous < 0.1);
                }
                act(&mut world, Scripted::BodySleep);
                sleeps += 1;
            } else if [400, 700, 1450, 1750, 2550, 2850, 3650, 3950].contains(&age) {
                act(&mut world, Scripted::AwakeRest);
            } else {
                world.try_advance_tick().unwrap();
            }
            let pressure = levels(&world).1;
            if previous < 0.6 && pressure >= 0.6 {
                crossings.push(world.tick().raw());
                println!("RECOVERY_PRESSURE_CROSSING schedule=separate1100 calibrated={calibrated} age={} pressure={pressure}",world.tick().raw());
            }
            assert_eq!(
                subject(&world).lifecycle().is_alive(),
                world.tick().raw() < HORIZON
            );
        }
        assert_eq!(sleeps, 4);
        assert_eq!(crossings.len(), if calibrated { 4 } else { 0 });
        println!("RECOVERY_SEPARATE_1100 calibrated={calibrated} natural_pressure_crossings={crossings:?} scripted_sleeps=[1200,2300,3400,4500] unchanged_age_death={HORIZON}");
    }
}
