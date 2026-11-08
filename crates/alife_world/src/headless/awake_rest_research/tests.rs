use super::*;
use crate::*;
use alife_core::*;

const SEED: u64 = 961_060;
const SUBJECT: OrganismId = OrganismId(1);

fn fresh(candidate: bool) -> CanonicalNewGame {
    fresh_at(SEED, candidate)
}

fn fresh_at(seed: u64, candidate: bool) -> CanonicalNewGame {
    let config = CanonicalNewGameConfig::phase3(seed, 1).unwrap();
    if candidate {
        create_canonical_new_game_with_awake_rest_research_v1(&config).unwrap()
    } else {
        let foundation = FoundationWeightAsset::builtin_nano512_v1(config.sensor_profile).unwrap();
        create_canonical_new_game(&config, &foundation).unwrap()
    }
}

fn subject(world: &HeadlessWorld) -> &WorldOrganismRecord {
    world.organism_registry.get(SUBJECT).unwrap()
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

fn authored_pair(world: &HeadlessWorld) -> HeadlessWorld {
    let mut result = world.clone();
    let graph = &subject(world).phenotype().chemistry.biochemical;
    let targets = [
        BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Fatigue),
        BiochemicalTargetLocus::Endocrine(EndocrineChannel::SleepPressure),
    ];
    let mut wire = serde_json::to_value(subject(world).biochemistry()).unwrap();
    for target in targets {
        let species = graph
            .receptors()
            .iter()
            .find(|r| r.target == target)
            .unwrap()
            .source;
        let index = graph
            .species()
            .iter()
            .position(|s| s.id == species)
            .unwrap();
        wire["graph_state"]["concentrations"][index] = serde_json::json!(0.65);
    }
    let mut biology: BiochemistryState = serde_json::from_value(wire).unwrap();
    biology.homeostasis = biology.graph_state().derive_homeostasis(graph).unwrap();
    biology
        .validate_against(subject(world).phenotype())
        .unwrap();
    result
        .organism_registry
        .with_biology_mut(SUBJECT, |current| {
            *current = biology;
            Ok(())
        })
        .unwrap();
    result
}

fn bundle(world: &HeadlessWorld, posture: ActionId, moving: bool) -> MotorCommandBundle {
    let channel = |channel, primitive, direction| {
        ChannelCommand::new(
            channel,
            primitive,
            None,
            direction,
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            0.0,
            Confidence::new(1.0).unwrap(),
            0,
        )
        .unwrap()
    };
    let mut channels = vec![channel(MotorChannel::Posture, posture, Vec3f::ZERO)];
    if moving {
        channels.push(channel(
            MotorChannel::Locomotion,
            HeadlessActionIds::STEP_FORWARD,
            Vec3f::new(1.0, 0.0, 0.0),
        ));
    }
    MotorCommandBundle::new(
        SUBJECT,
        ExperienceSequenceId(world.tick().raw() + 1),
        world.tick(),
        channels,
    )
    .unwrap()
}

fn assert_preserved(before: &WorldOrganismRecord, after: &WorldOrganismRecord) {
    assert_eq!(before.genome(), after.genome());
    assert_eq!(before.phenotype(), after.phenotype());
    assert_eq!(before.embodiment(), after.embodiment());
    assert_eq!(before.state_graph().brain, after.state_graph().brain);
    assert_eq!(before.state_graph().memory, after.state_graph().memory);
    assert_eq!(before.sleep_seal(), after.sleep_seal());
    assert_eq!(before.cognitive_work(), after.cognitive_work());
    assert_eq!(after.sleep_phase(), SleepPhase::Awake);
    assert_eq!(after.sleep_work_units(), 0);
}

fn act(
    world: &mut HeadlessWorld,
    posture: ActionId,
    moving: bool,
    candidate: bool,
    label: &str,
) -> AwakeRestReceiptV1 {
    let before = subject(world).clone();
    let bundle = bundle(world, posture, moving);
    let receipt = if candidate {
        world
            .apply_awake_rest_research_v1(
                &AwakeRestRequestV1::new(bundle).unwrap(),
                before.world_entity_id(),
            )
            .unwrap()
    } else {
        AwakeRestReceiptV1 {
            motor: world
                .apply_registered_motor_bundle(&bundle, before.world_entity_id())
                .unwrap(),
            recovery: AwakeRestEventV1::NONE,
        }
    };
    let m = &receipt.motor;
    assert_eq!(m.biology_before, *before.biochemistry());
    assert_eq!(m.biology_after, *subject(world).biochemistry());
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
                receipt.recovery,
            )
            .unwrap()
    );
    assert_preserved(&before, subject(world));
    assert_eq!(m.body_event.nutrition, 0.0);
    assert_eq!(m.body_event.player_reward, 0.0);
    let sleep_species = before
        .phenotype()
        .chemistry
        .biochemical
        .receptors()
        .iter()
        .find(|r| r.target == BiochemicalTargetLocus::SleepMetabolicRate)
        .unwrap()
        .source;
    let a = m.biology_after;
    let b = m.biology_before;
    let sleep_state = a
        .graph_state()
        .concentration(&before.phenotype().chemistry.biochemical, sleep_species)
        .unwrap();
    println!("AWAKE_REST {label} age={} candidate={} posture={} moving={} expression={:.9} dose={:.9} sleep_dose={:.9} body_sleep={} sleep_state={:.9} fatigue={:.9}->{:.9} pressure={:.9}->{:.9} energy={:.9}->{:.9} atp={:.9}->{:.9} event_energy={:.9} work={} effectors={:.9} displacement={:?}",
        b.development.age_ticks.raw(), candidate, posture.raw(), moving, a.development.biochemical_expression,
        receipt.recovery.dose(), m.body_event.sleep_recovery, a.body.sleeping, sleep_state,
        b.homeostasis.drives.fatigue, a.homeostasis.drives.fatigue,
        b.homeostasis.hormones.sleep_pressure, a.homeostasis.hormones.sleep_pressure,
        b.body.energy, a.body.energy, b.homeostasis.drives.brain_atp, a.homeostasis.drives.brain_atp,
        m.body_event.energy, a.biochemical_work().total(), before.embodiment().effector_gain(EffectorCapability::Rest),
        m.joint.execution.displacement);
    if candidate {
        let repeated = AwakeRestRequestV1::new(m.bundle.clone()).unwrap();
        let saved = serde_json::to_vec(&world.save_state()).unwrap();
        assert!(world
            .apply_awake_rest_research_v1(&repeated, before.world_entity_id())
            .is_err());
        assert_eq!(serde_json::to_vec(&world.save_state()).unwrap(), saved);
    }
    assert_eq!(world.try_advance_tick().unwrap(), m.outcome_tick);
    assert_eq!(
        *subject(world).biochemistry(),
        m.biology_after,
        "enclosing tick must not advance biology twice"
    );
    receipt
}

#[test]
fn awake_rest_development_matrix_preserves_sleep_need_and_legacy_rest() {
    let mut prepared = fresh(true).world;
    for age in [0, 60, 240] {
        prepare(&mut prepared, age);
        let authored = authored_pair(&prepared);
        for (label, initial) in [
            ("fresh", &prepared),
            ("authored-fatigue-and-pressure-0.65", &authored),
        ] {
            let awake = act(
                &mut initial.clone(),
                HeadlessActionIds::NO_POSTURE,
                false,
                true,
                label,
            );
            let neutral = act(
                &mut initial.clone(),
                HeadlessActionIds::NO_POSTURE,
                false,
                false,
                label,
            );
            let rest = act(
                &mut initial.clone(),
                ActionKind::Rest.canonical_id(),
                false,
                false,
                label,
            );
            let idle = act(
                &mut initial.clone(),
                ActionKind::Idle.canonical_id(),
                false,
                false,
                label,
            );
            let moving = act(
                &mut initial.clone(),
                HeadlessActionIds::NO_POSTURE,
                true,
                true,
                label,
            );
            let move_only = act(
                &mut initial.clone(),
                HeadlessActionIds::NO_POSTURE,
                true,
                false,
                label,
            );
            let moving_rest = act(
                &mut initial.clone(),
                ActionKind::Rest.canonical_id(),
                true,
                false,
                label,
            );
            assert_eq!(awake.recovery.dose(), 1.0);
            assert_eq!(awake.motor.body_event, neutral.motor.body_event);
            assert_eq!(
                awake.motor.biology_after.body,
                neutral.motor.biology_after.body
            );
            assert_eq!(
                awake.motor.biology_after.homeostasis.hormones,
                neutral.motor.biology_after.homeostasis.hormones
            );
            assert_eq!(
                awake
                    .motor
                    .biology_after
                    .homeostasis
                    .hormones
                    .sleep_pressure,
                idle.motor.biology_after.homeostasis.hormones.sleep_pressure
            );
            assert!(!awake.motor.biology_after.body.sleeping);
            assert_eq!(awake.motor.body_event.sleep_recovery, 0.0);
            assert!(rest.motor.biology_after.body.sleeping);
            assert_eq!(rest.motor.body_event.sleep_recovery, 1.0);
            assert_eq!(awake.motor.body_event.energy, 0.0);
            assert_eq!(rest.motor.body_event.energy, 0.0);
            assert_eq!(idle.motor.body_event.energy, -0.01);
            assert!(
                awake.motor.biology_after.body.energy <= awake.motor.biology_before.body.energy
            );
            assert!(
                awake.motor.biology_after.homeostasis.drives.brain_atp
                    <= awake.motor.biology_before.homeostasis.drives.brain_atp
            );
            let expected_fatigue = (neutral.motor.biology_after.homeostasis.drives.fatigue
                - 0.70 * awake.motor.biology_after.development.biochemical_expression)
                .max(0.0);
            assert!(
                (awake.motor.biology_after.homeostasis.drives.fatigue - expected_fatigue).abs()
                    < 1e-6
            );
            if age == 0 {
                assert_eq!(awake.motor.biology_after, neutral.motor.biology_after);
            }
            if age > 0 && label.starts_with("authored") {
                assert!(
                    rest.motor.biology_after.homeostasis.hormones.sleep_pressure
                        < awake
                            .motor
                            .biology_after
                            .homeostasis
                            .hormones
                            .sleep_pressure
                );
                assert!(
                    awake.motor.biology_after.homeostasis.drives.fatigue
                        < neutral.motor.biology_after.homeostasis.drives.fatigue
                );
            }
            assert_eq!(moving.recovery.dose(), 0.0);
            assert_ne!(moving.motor.joint.execution.displacement, Vec3f::ZERO);
            assert_eq!(moving.motor, move_only.motor);
            assert_eq!(moving.motor.body_event, moving_rest.motor.body_event);
            assert_eq!(moving.motor.biology_after, moving_rest.motor.biology_after);
        }
    }
}

fn physiology_without_work(value: &BiochemistryState) -> serde_json::Value {
    let mut wire = serde_json::to_value(value).unwrap();
    wire.as_object_mut().unwrap().remove("biochemical_work");
    wire
}

#[test]
fn awake_rest_inherited_emitter_alone_preserves_passive_and_ordinary_outcomes() {
    let mut old = fresh(false).world;
    let mut candidate = fresh(true).world;
    for age in [0, 60, 240] {
        prepare(&mut old, age);
        prepare(&mut candidate, age);
        assert_eq!(
            physiology_without_work(subject(&old).biochemistry()),
            physiology_without_work(subject(&candidate).biochemistry())
        );
        for posture in [
            HeadlessActionIds::NO_POSTURE,
            ActionKind::Rest.canonical_id(),
            ActionKind::Idle.canonical_id(),
        ] {
            let a = act(&mut old.clone(), posture, false, false, "ordinary-genotype");
            let b = act(
                &mut candidate.clone(),
                posture,
                false,
                false,
                "candidate-emitter-inactive",
            );
            assert_eq!(a.motor.body_event, b.motor.body_event);
            assert_eq!(a.motor.joint, b.motor.joint);
            assert_eq!(
                physiology_without_work(&a.motor.biology_after),
                physiology_without_work(&b.motor.biology_after)
            );
            assert_eq!(
                b.motor.biology_after.biochemical_work().emitter_evaluations,
                a.motor.biology_after.biochemical_work().emitter_evaluations + 1
            );
        }
    }
}

#[test]
fn awake_rest_damage_cancels_only_recovery_and_failed_transaction_rolls_back() {
    let mut base = fresh(true).world;
    prepare(&mut base, 60);
    base = authored_pair(&base);
    let own = subject(&base).world_entity_id();
    let position = base.objects[&own.raw()].position;
    base.editor_spawn_object(WorldEditorSpawnSpec {
        label: "fresh-injury".into(),
        kind: WorldObjectKind::Hazard,
        organism_id: None,
        position,
        nutrition: 0.0,
        hazard_pain: 0.2,
        radius: 0.1,
        token_id: None,
    })
    .unwrap();
    let awake = act(
        &mut base.clone(),
        HeadlessActionIds::NO_POSTURE,
        false,
        true,
        "fresh-damage",
    );
    let neutral = act(
        &mut base.clone(),
        HeadlessActionIds::NO_POSTURE,
        false,
        false,
        "damage-control",
    );
    assert_eq!(awake.recovery.dose(), 0.0);
    assert!(awake.motor.body_event.damage > 0.0);
    assert!(awake.motor.biology_after.body.health < awake.motor.biology_before.body.health);
    assert_eq!(awake.motor, neutral.motor);
    let request =
        AwakeRestRequestV1::new(bundle(&base, HeadlessActionIds::NO_POSTURE, true)).unwrap();
    let saved = serde_json::to_vec(&base.save_state()).unwrap();
    let prior_action = base.last_action_result.clone();
    base.inject_post_action_failure();
    assert!(base.apply_awake_rest_research_v1(&request, own).is_err());
    assert_eq!(serde_json::to_vec(&base.save_state()).unwrap(), saved);
    assert_eq!(base.last_action_result, prior_action);
}

#[test]
fn awake_rest_admission_rejects_ordinary_recipe_bad_posture_stale_interval_and_missing_effector() {
    let mut old = fresh(false).world;
    let request =
        AwakeRestRequestV1::new(bundle(&old, HeadlessActionIds::NO_POSTURE, false)).unwrap();
    let saved = serde_json::to_vec(&old.save_state()).unwrap();
    let own = subject(&old).world_entity_id();
    assert!(old.apply_awake_rest_research_v1(&request, own).is_err());
    assert_eq!(serde_json::to_vec(&old.save_state()).unwrap(), saved);
    for posture in [
        ActionKind::Rest.canonical_id(),
        ActionKind::Idle.canonical_id(),
        ActionId(u32::MAX),
    ] {
        assert!(AwakeRestRequestV1::new(bundle(&old, posture, false)).is_err());
    }
    let mut candidate = fresh(true).world;
    let mut record = subject(&candidate).clone();
    let mut wire = serde_json::to_value(record.embodiment()).unwrap();
    wire["revision"] = serde_json::json!(record.embodiment().revision() + 1);
    let effectors = wire["effectors"].as_array_mut().unwrap();
    assert_eq!(
        effectors.last().unwrap()["capability"],
        serde_json::json!("Rest")
    );
    effectors.pop();
    wire["effector_controllability"]
        .as_array_mut()
        .unwrap()
        .pop();
    record
        .replace_embodiment_state(serde_json::from_value(wire).unwrap())
        .unwrap();
    candidate.replace_organism_registry_exact([record]).unwrap();
    let saved = serde_json::to_vec(&candidate.save_state()).unwrap();
    assert!(matches!(
        candidate.apply_awake_rest_research_v1(&request, own),
        Err(HeadlessMotorTransactionError::UnsupportedChannel(
            MotorChannel::Posture
        ))
    ));
    assert_eq!(serde_json::to_vec(&candidate.save_state()).unwrap(), saved);
    let mut candidate = fresh(true).world;
    candidate.try_advance_tick().unwrap();
    let saved = serde_json::to_vec(&candidate.save_state()).unwrap();
    assert!(candidate
        .apply_awake_rest_research_v1(&request, own)
        .is_err());
    assert_eq!(serde_json::to_vec(&candidate.save_state()).unwrap(), saved);
}

#[test]
fn awake_rest_duration_and_effector_intensity_are_bounded_not_free_repeated_doses() {
    let mut initial = fresh(true).world;
    prepare(&mut initial, 60);
    initial = authored_pair(&initial);
    let own = subject(&initial).world_entity_id();
    for intensity in [0.0, 0.25, 1.0] {
        let mut short = bundle(&initial, HeadlessActionIds::NO_POSTURE, false);
        short.channels[0].intensity = Intensity::new(intensity).unwrap();
        let mut long = short.clone();
        long.channels[0].duration_ticks = DurationTicks::new(64);
        let a = initial
            .clone()
            .apply_awake_rest_research_v1(&AwakeRestRequestV1::new(short).unwrap(), own)
            .unwrap();
        let b = initial
            .clone()
            .apply_awake_rest_research_v1(&AwakeRestRequestV1::new(long).unwrap(), own)
            .unwrap();
        assert_eq!(a.recovery, b.recovery);
        assert_eq!(a.motor.biology_after, b.motor.biology_after);
        assert_eq!(
            a.recovery.dose(),
            (intensity
                * subject(&initial)
                    .embodiment()
                    .effector_gain(EffectorCapability::Rest))
            .clamp(0.0, 1.0)
        );
    }
    let b = subject(&initial).biochemistry();
    assert!(b
        .advance_with_body_research_events_v1(
            Tick(61),
            Tick(61),
            BodyEventDelta {
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
            None,
            subject(&initial).phenotype(),
            ThermalBalanceCandidate::default(),
            AwakeRestEventV1::new(1.0).unwrap()
        )
        .is_err());
    assert!(AwakeRestEventV1::new(f32::NAN).is_err());
    assert!(AwakeRestEventV1::new(1.01).is_err());
}

#[test]
fn awake_rest_portable_roundtrip_keeps_recipe_state_and_exact_continuation() {
    for authored in [false, true] {
        let game = fresh(true);
        let mut initial = game.world;
        prepare(&mut initial, 60);
        if authored {
            initial = authored_pair(&initial);
        }
        let mut creatures = game.creatures;
        let biology = subject(&initial).biochemistry();
        creatures[0].mind.tick = biology.tick;
        creatures[0].mind.homeostasis = biology.homeostasis;
        creatures[0].development_tick = biology.development.last_update_tick;
        let save = PortableSaveFile::from_headless_world(
            "disposable-awake-rest-v1",
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
        assert_eq!(subject(&initial), subject(&restored));
        assert_eq!(
            initial.canonical_signature_digest().unwrap(),
            restored.canonical_signature_digest().unwrap()
        );
        assert!(subject(&restored)
            .phenotype()
            .chemistry
            .biochemical
            .supports_awake_rest_recovery_v1());
        for (posture, candidate) in [
            (HeadlessActionIds::NO_POSTURE, true),
            (ActionKind::Rest.canonical_id(), false),
            (ActionKind::Idle.canonical_id(), false),
        ] {
            let mut a = initial.clone();
            let mut b = restored.clone();
            assert_eq!(
                act(&mut a, posture, false, candidate, "before-save"),
                act(&mut b, posture, false, candidate, "after-restore")
            );
            assert_eq!(subject(&a), subject(&b));
            assert_eq!(
                a.canonical_signature_digest().unwrap(),
                b.canonical_signature_digest().unwrap()
            );
        }
    }
}

#[test]
fn awake_rest_failed_attempt_retains_cost_and_actual_body_sleep_remains_available() {
    let mut initial = fresh(true).world;
    prepare(&mut initial, 60);
    initial = authored_pair(&initial);
    let legacy = act(
        &mut initial.clone(),
        ActionKind::Rest.canonical_id(),
        false,
        false,
        "actual-rest-control",
    );
    let mut sleeping = initial.clone();
    sleeping
        .try_advance_tick_with_body_events(&BTreeMap::from([(
            SUBJECT.raw(),
            BodyEventDelta {
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
        )]))
        .unwrap();
    assert_eq!(
        *subject(&sleeping).biochemistry(),
        legacy.motor.biology_after
    );
    assert_eq!(subject(&sleeping).sleep_phase(), SleepPhase::Awake);
    assert!(subject(&sleeping).biochemistry().body.sleeping);
    let awake = act(
        &mut sleeping.clone(),
        HeadlessActionIds::NO_POSTURE,
        false,
        true,
        "wake-from-body-sleep",
    );
    let neutral = act(
        &mut sleeping.clone(),
        HeadlessActionIds::NO_POSTURE,
        false,
        false,
        "wake-control",
    );
    assert!(!awake.motor.biology_after.body.sleeping);
    assert_eq!(
        awake
            .motor
            .biology_after
            .homeostasis
            .hormones
            .sleep_pressure,
        neutral
            .motor
            .biology_after
            .homeostasis
            .hormones
            .sleep_pressure
    );

    let own = subject(&initial).world_entity_id();
    let position = initial.objects[&own.raw()].position;
    initial
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "blocked-attempt".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(position.x + 0.8, position.y, position.z),
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.8,
            token_id: None,
        })
        .unwrap();
    let failed = act(
        &mut initial.clone(),
        HeadlessActionIds::NO_POSTURE,
        true,
        true,
        "blocked-attempt",
    );
    let control = act(
        &mut initial.clone(),
        HeadlessActionIds::NO_POSTURE,
        true,
        false,
        "blocked-control",
    );
    assert!(!failed.motor.succeeded);
    assert_eq!(failed.motor.joint.execution.displacement, Vec3f::ZERO);
    assert_eq!(failed.recovery.dose(), 0.0);
    assert!(failed.motor.body_event.energy < 0.0);
    assert_eq!(failed.motor, control.motor);
}

#[test]
fn awake_rest_inheritance_retains_structural_alleles_and_seeded_mutated_gametes() {
    let ordinary = subject(&fresh(false).world).genome().clone();
    let candidate = subject(&fresh(true).world).genome().clone();
    let father = subject(&fresh_at(SEED + 1, true).world).genome().clone();
    let old_graph = ordinary.chemistry.graph.expressed().clone();
    let new_graph = candidate.chemistry.graph.expressed().clone();
    for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
        let mut heterozygous = candidate.clone();
        heterozygous.chemistry.graph =
            BiochemicalGraphChromosome::new(new_graph.clone(), old_graph.clone(), side).unwrap();
        let phenotype = heterozygous.express().unwrap();
        assert_eq!(
            phenotype
                .chemistry
                .biochemical
                .supports_awake_rest_recovery_v1(),
            side == AlleleSide::Maternal
        );
        let restored: CreatureGenome =
            serde_json::from_slice(&serde_json::to_vec(&heterozygous).unwrap()).unwrap();
        assert_eq!(restored, heterozygous);
        assert_eq!(restored.express().unwrap(), phenotype);
    }
    // Fixed construction fixtures, not a breeding population or mutation study.
    // The ordinary inherited rate remains untouched in the first case; the
    // other two cases explicitly author zero and maximal point-mutation rates.
    for (seed, mutation_rate) in [(901, None), (902, Some(0.0)), (903, Some(1.0))] {
        let mut mother = candidate.clone();
        let mut father = father.clone();
        if let Some(rate) = mutation_rate {
            for parent in [&mut mother, &mut father] {
                parent.reproduction.mutation_rate = ContinuousLocus::mean(rate, rate).unwrap();
                parent.reproduction.discrete_mutation_rate =
                    ContinuousLocus::mean(0.0, 0.0).unwrap();
            }
        }
        let parents_before = serde_json::to_vec(&(&mother, &father)).unwrap();
        let child = CreatureGenome::reproduce(&mother, &father, seed).unwrap();
        assert_eq!(
            child,
            CreatureGenome::reproduce(&mother, &father, seed).unwrap()
        );
        assert_eq!(
            parents_before,
            serde_json::to_vec(&(&mother, &father)).unwrap()
        );
        let phenotype = child.express().unwrap();
        let restored: CreatureGenome =
            serde_json::from_slice(&serde_json::to_vec(&child).unwrap()).unwrap();
        assert_eq!(restored, child);
        assert_eq!(restored.express().unwrap(), phenotype);
        let rows = phenotype
            .chemistry
            .biochemical
            .emitters()
            .iter()
            .filter(|row| row.source == BiochemicalSourceLocus::AwakeRestRecoveryV1)
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        let admitted = phenotype
            .chemistry
            .biochemical
            .supports_awake_rest_recovery_v1();
        if mutation_rate == Some(0.0) {
            assert!(admitted);
        }
        if mutation_rate == Some(1.0) {
            assert!(!admitted);
            assert!(!child.provenance.mutations.is_empty());
        }
        let biology = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        let ordinary_advance = biology
            .advance_with_age(Tick(1), Tick(1), BodyEventDelta::zero(), &phenotype)
            .unwrap();
        ordinary_advance.validate_against(&phenotype).unwrap();
        // Keep every other expressed mutant parameter intact. Removing only
        // the event-backed rows must give the identical newborn graph update.
        let mut wire = serde_json::to_value(&phenotype.chemistry.biochemical).unwrap();
        wire["emitters"]
            .as_array_mut()
            .unwrap()
            .retain(|row| row["source"] != serde_json::json!("AwakeRestRecoveryV1"));
        let without: BiochemicalPhenotype = serde_json::from_value(wire).unwrap();
        let control = biology
            .graph_state()
            .advance(
                Tick(1),
                ordinary_advance.body,
                BodyEventDelta::zero(),
                None,
                &without,
                ordinary_advance.development.biochemical_expression,
            )
            .unwrap()
            .0;
        assert_eq!(*ordinary_advance.graph_state(), control);
        if !admitted {
            assert!(biology
                .advance_with_body_research_events_v1(
                    Tick(1),
                    Tick(1),
                    BodyEventDelta::zero(),
                    None,
                    &phenotype,
                    ThermalBalanceCandidate::default(),
                    AwakeRestEventV1::new(1.0).unwrap()
                )
                .is_err());
        }
        println!("AWAKE_REST_INHERITANCE seed={seed} authored_rate={mutation_rate:?} mutations={} threshold={:.9} gain={:.9} floor={:.9} exact_event_admission={admitted} newborn_advance=true roundtrip=true", child.provenance.mutations.len(), rows[0].threshold, rows[0].gain, rows[0].developmental_expression_floor);
    }
}
