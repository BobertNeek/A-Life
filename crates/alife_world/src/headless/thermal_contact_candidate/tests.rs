use super::*;
use crate::{PortableSaveFile, RuntimeConfig, WorldOrganismRecord};
use alife_core::{BrainCapacityClass, BrainScaleTier, CreatureGenome, FoundationGeneticIdentity};
const OWN: OrganismId = OrganismId(71);

fn fixture(temperature: f32) -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
    fixture_at(temperature, None, Tick::ZERO)
}
fn fixture_at(
    temperature: f32,
    tolerance: Option<f32>,
    tick: Tick,
) -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
    let mut world = HeadlessScenarioBuilder::new(71512026)
        .agent("body", OWN, Vec3f::ZERO)
        .token("surface", Vec3f::new(0.2, 0.0, 0.0), 1)
        .build()
        .unwrap();
    let own = world.entity_id("body").unwrap();
    let surface = world.entity_id("surface").unwrap();
    world
        .objects
        .get_mut(&surface.raw())
        .unwrap()
        .grounded_physical
        .surface_temperature = temperature;
    let mut genome = CreatureGenome::early_mammal_founder(
        71512026,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    if let Some(tolerance) = tolerance {
        genome.body.temperature_tolerance =
            alife_core::ContinuousLocus::mean(tolerance, tolerance).unwrap();
    }
    world.tick = tick;
    let phenotype = genome.express().unwrap();
    let bio = BiochemistryState::new(&phenotype, tick).unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::new(OWN, own, genome, phenotype, bio, Tick::ZERO).unwrap(),
        )
        .unwrap();
    (world, own, surface)
}
fn config(surface: WorldEntityId) -> ThermalContactCandidate {
    ThermalContactCandidate::research(BTreeMap::from([(surface.raw(), 1.0)]))
}
fn biology(world: &HeadlessWorld) -> BiochemistryState {
    *world.organism_registry.get(OWN).unwrap().biochemistry()
}
fn advance(world: &mut HeadlessWorld, parameters: ThermalContactCandidate) {
    world
        .with_thermal_contact_candidate(parameters, |world| world.try_advance_tick())
        .unwrap();
}
fn bytes(world: &HeadlessWorld) -> String {
    serde_json::to_string(
        &PortableSaveFile::from_headless_world(
            "thermal-research",
            world,
            RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
            crate::AssetManifest::empty(),
            Vec::new(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn neutral_boundaries_and_default_are_byte_identical() {
    for temperature in [-0.25, 0.0, 0.25] {
        let (base, _, surface) = fixture(temperature);
        let mut ordinary = base.clone();
        ordinary.try_advance_tick().unwrap();
        let mut candidate = base;
        advance(&mut candidate, config(surface));
        assert_eq!(bytes(&ordinary), bytes(&candidate));
    }
    let (mut ordinary, _, _) = fixture(1.0);
    ordinary.try_advance_tick().unwrap();
    assert_eq!(biology(&ordinary).body.temperature_stress, 0.0);
}
#[test]
fn hot_cold_are_symmetric_tolerance_scaled_and_conductance_insulates() {
    for temperature in [-1.0, -0.2501, 0.2501, 1.0] {
        let (mut world, _, surface) = fixture(temperature);
        let tolerance = world
            .organism_registry
            .get(OWN)
            .unwrap()
            .phenotype()
            .body
            .temperature_tolerance;
        let mut insulated = world.clone();
        let mut p = config(surface);
        p.surface_conductance.insert(surface.raw(), 0.0);
        advance(&mut insulated, p);
        assert_eq!(biology(&insulated).body.temperature_stress, 0.0);
        advance(&mut world, config(surface));
        let expected = ((temperature.abs() - 0.25) / 0.75) * 0.10 / WORLD_TICKS_PER_SECOND as f32
            * (1.0 - tolerance);
        assert!((biology(&world).body.temperature_stress - expected).abs() < 1e-7);
    }
}
#[test]
fn no_contact_and_intervening_solid_block_exposure() {
    let (mut distant, _, surface) = fixture(1.0);
    distant
        .editor_move_object(surface, Vec3f::new(10.0, 0.0, 0.0))
        .unwrap();
    advance(&mut distant, config(surface));
    assert_eq!(biology(&distant).body.temperature_stress, 0.0);
    let (mut blocked, _, surface) = fixture(1.0);
    let obstacle = blocked
        .insert_object(SpawnSpec {
            label: "wall",
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(0.1, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: None,
            social_affinity: 0.0,
            teacher_channel: None,
        })
        .unwrap();
    blocked.objects.get_mut(&obstacle.raw()).unwrap().radius = 0.025;
    advance(&mut blocked, config(surface));
    assert_eq!(biology(&blocked).body.temperature_stress, 0.0);
}
#[test]
fn overlapping_hot_and_cold_use_max_not_sum_or_cancellation() {
    let (mut world, _, surface) = fixture(1.0);
    let mut single = world.clone();
    let cold = world
        .insert_object(SpawnSpec {
            label: "cold",
            kind: WorldObjectKind::Token,
            organism_id: None,
            position: Vec3f::new(-0.2, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: Some(2),
            social_affinity: 0.0,
            teacher_channel: None,
        })
        .unwrap();
    world
        .objects
        .get_mut(&cold.raw())
        .unwrap()
        .grounded_physical
        .surface_temperature = -1.0;
    let mut p = config(surface);
    p.surface_conductance.insert(cold.raw(), 1.0);
    advance(&mut world, p);
    advance(&mut single, config(surface));
    assert_eq!(
        biology(&world).body.temperature_stress,
        biology(&single).body.temperature_stress
    );
}
#[test]
fn stationary_single_and_joint_actions_receive_one_interval_not_command_duration() {
    let (base, own, surface) = fixture(1.0);
    let mut stationary = base.clone();
    advance(&mut stationary, config(surface));
    let mut single = base.clone();
    single
        .with_thermal_contact_candidate(config(surface), |world| {
            let mut command = HeadlessWorldCommand::idle(OWN).unwrap();
            command.duration_ticks = alife_core::DurationTicks::new(99);
            world.apply_registered_command(&command, own, Tick(1))?;
            world.try_advance_tick()
        })
        .unwrap();
    let mut joint = base;
    joint
        .with_thermal_contact_candidate(config(surface), |world| {
            let command = HeadlessWorldCommand::idle(OWN).unwrap();
            let channel = alife_core::channel_command_for_action(MotorChannel::Posture, &command)?;
            let bundle = MotorCommandBundle::new(
                OWN,
                alife_core::ExperienceSequenceId(1),
                Tick::ZERO,
                vec![channel],
            )?;
            world.apply_registered_motor_bundle(&bundle, own).unwrap();
            world.try_advance_tick()
        })
        .unwrap();
    assert_eq!(
        biology(&stationary).body.temperature_stress,
        biology(&single).body.temperature_stress
    );
    assert_eq!(
        biology(&stationary).body.temperature_stress,
        biology(&joint).body.temperature_stress
    );
    assert_eq!(biology(&joint).tick, Tick(1));
}
#[test]
fn relocation_recovers_without_sleep_atp_or_fatigue_benefit_and_save_continues() {
    let (mut world, _, surface) = fixture(1.0);
    for _ in 0..4 {
        advance(&mut world, config(surface));
    }
    world
        .editor_move_object(surface, Vec3f::new(10.0, 0.0, 0.0))
        .unwrap();
    let before = biology(&world);
    let mut no_relief = world.clone();
    let mut off = config(surface);
    off.relief_per_second = 0.0;
    advance(&mut no_relief, off);
    let mut restored = PortableSaveFile::from_json_str(&bytes(&world))
        .unwrap()
        .restore_headless_world()
        .unwrap();
    advance(&mut restored, config(surface));
    advance(&mut world, config(surface));
    assert_eq!(bytes(&world), bytes(&restored));
    let after = biology(&world);
    assert!(after.body.temperature_stress < before.body.temperature_stress);
    assert_eq!(
        after.homeostasis.drives.brain_atp,
        biology(&no_relief).homeostasis.drives.brain_atp
    );
    assert_eq!(
        after.homeostasis.drives.fatigue,
        biology(&no_relief).homeostasis.drives.fatigue
    );
    assert_eq!(
        after.body.sleeping,
        no_relief
            .organism_registry
            .get(OWN)
            .unwrap()
            .biochemistry()
            .body
            .sleeping
    );
}
#[test]
fn invalid_parameters_and_late_failures_roll_back_complete_world() {
    let (mut world, own, surface) = fixture(1.0);
    let original = bytes(&world);
    for bad in [f32::NAN, -0.1, 1.1] {
        let mut p = config(surface);
        p.surface_conductance.insert(surface.raw(), bad);
        assert!(world
            .with_thermal_contact_candidate(p, |w| w.try_advance_tick())
            .is_err());
        let mut p = config(surface);
        p.exposure_per_second = bad;
        assert!(world
            .with_thermal_contact_candidate(p, |w| w.try_advance_tick())
            .is_err());
        let mut p = config(surface);
        p.relief_per_second = bad;
        assert!(world
            .with_thermal_contact_candidate(p, |w| w.try_advance_tick())
            .is_err());
        assert_eq!(bytes(&world), original);
    }
    let mut invalid_band = config(surface);
    invalid_band.neutral_band = 1.0;
    assert!(world
        .with_thermal_contact_candidate(invalid_band, |w| w.try_advance_tick())
        .is_err());
    let result: Result<(), _> = world.with_thermal_contact_candidate(config(surface), |w| {
        w.apply_registered_command(&HeadlessWorldCommand::idle(OWN)?, own, Tick(1))?;
        w.try_advance_tick()?;
        Err(ScaffoldContractError::InvalidDecisionEvidence)
    });
    assert!(result.is_err());
    assert_eq!(bytes(&world), original);
    world.inject_post_action_failure();
    assert!(world
        .with_thermal_contact_candidate(config(surface), |w| w.apply_registered_command(
            &HeadlessWorldCommand::idle(OWN)?,
            own,
            Tick(1)
        ))
        .is_err());
    assert_eq!(bytes(&world), original);
}

#[test]
fn inherited_tolerance_variants_and_mature_chemical_projection() {
    let mut loads = Vec::new();
    for tolerance in [0.0, 0.5, 1.0] {
        let (mut world, _, surface) = fixture_at(1.0, Some(tolerance), Tick(600));
        let mut neutral = world.clone();
        neutral
            .objects
            .get_mut(&surface.raw())
            .unwrap()
            .grounded_physical
            .surface_temperature = 0.0;
        advance(&mut neutral, config(surface));
        advance(&mut world, config(surface));
        let actual = biology(&world);
        loads.push(actual.body.temperature_stress);
        if tolerance < 1.0 {
            assert!(
                actual.homeostasis.drives.temperature_stress
                    > biology(&neutral).homeostasis.drives.temperature_stress
            );
        }
    }
    assert!(loads[0] > loads[1]);
    assert_eq!(loads[2], 0.0);
}

#[test]
fn locomotion_departure_gets_one_endpoint_dose_then_neutral_recovery() {
    let (mut world, own, surface) = fixture(1.0);
    // Start 0.70 units from surface; one 0.10-unit step crosses tactile radius0.75.
    world
        .editor_move_object(own, Vec3f::new(-0.5, 0.0, 0.0))
        .unwrap();
    world
        .with_thermal_contact_candidate(config(surface), |w| {
            let command = HeadlessWorldCommand::structured(
                OWN,
                ActionKind::Move.canonical_id(),
                ActionKind::Move,
                None,
                Some(Vec3f::new(-10.0, 0.0, 0.0)),
            )?;
            let receipt = w.apply_registered_command(&command, own, Tick(1))?;
            assert!(receipt.action_result.execution.succeeded);
            assert!((receipt.action_result.body_event.temperature_stress - 0.005).abs() < 1e-7);
            assert!(
                distance(
                    w.objects[&own.raw()].position,
                    w.objects[&surface.raw()].position
                ) > HEADLESS_CONTACT_RADIUS
            );
            w.try_advance_tick()
        })
        .unwrap();
    let exposed = biology(&world).body.temperature_stress;
    advance(&mut world, config(surface));
    assert!(biology(&world).body.temperature_stress < exposed);
}

#[test]
fn neural_bundle_and_stationary_late_failure_share_atomic_boundary() {
    let (base, own, surface) = fixture(1.0);
    let mut ordinary_bundle = base.clone();
    let mut neural_bundle = base.clone();
    let channel = alife_core::channel_command_for_action(
        MotorChannel::Posture,
        &HeadlessWorldCommand::idle(OWN).unwrap(),
    )
    .unwrap();
    let bundle = MotorCommandBundle::new(
        OWN,
        alife_core::ExperienceSequenceId(1),
        Tick::ZERO,
        vec![channel],
    )
    .unwrap();
    let neural = NeuralEmissionFrame::new(Tick::ZERO, 1, Vec::new()).unwrap();
    ordinary_bundle
        .with_thermal_contact_candidate(config(surface), |w| {
            w.apply_registered_motor_bundle(&bundle, own).unwrap();
            w.try_advance_tick()
        })
        .unwrap();
    neural_bundle
        .with_thermal_contact_candidate(config(surface), |w| {
            w.apply_registered_motor_bundle_with_neural_emission(&bundle, own, &neural)
                .unwrap();
            w.try_advance_tick()
        })
        .unwrap();
    // Supplying even an empty neural frame visits neuroemitters and records
    // that work. Compare physiology, not legitimately different work receipts.
    assert_eq!(biology(&ordinary_bundle).body, biology(&neural_bundle).body);
    assert_eq!(
        biology(&ordinary_bundle).homeostasis,
        biology(&neural_bundle).homeostasis
    );
    assert_eq!(biology(&ordinary_bundle).tick, biology(&neural_bundle).tick);
    assert!(
        biology(&neural_bundle)
            .biochemical_work()
            .neural_emitter_evaluations
            > biology(&ordinary_bundle)
                .biochemical_work()
                .neural_emitter_evaluations
    );
    let mut failing = base;
    let before = bytes(&failing);
    failing.injected_tick_late_failure_after_first_organism = true;
    assert!(failing
        .with_thermal_contact_candidate(config(surface), |w| w.try_advance_tick())
        .is_err());
    assert_eq!(bytes(&failing), before);
}

#[test]
fn zero_exchange_preserves_signed_zero_and_wrapper_count_is_not_time() {
    let (mut world, _, surface) = fixture(1.0);
    let original = bytes(&world);
    let mut zero = config(surface);
    zero.exposure_per_second = -0.0;
    zero.relief_per_second = -0.0;
    for _ in 0..3 {
        world
            .with_thermal_contact_candidate(zero.clone(), |w| {
                let event = BodyEventDelta {
                    temperature_stress: -0.0,
                    ..BodyEventDelta::zero()
                };
                event.validate_contract()?;
                let (unchanged, relief) = w.thermal_interval_event(event, 1.0);
                assert_eq!(unchanged.temperature_stress.to_bits(), (-0.0_f32).to_bits());
                let p = w.organism_registry.get(OWN).unwrap().phenotype();
                let b = biology(w);
                let ordinary = b.advance(Tick(1), event, p)?;
                let candidate = b.advance_with_thermal_balance_candidate(
                    Tick(1),
                    Tick(1),
                    event,
                    None,
                    p,
                    relief,
                )?;
                assert_eq!(
                    serde_json::to_string(&ordinary).unwrap(),
                    serde_json::to_string(&candidate).unwrap()
                );
                Ok(())
            })
            .unwrap();
    }
    assert_eq!(bytes(&world), original);
    let mut separate = world.clone();
    for _ in 0..4 {
        advance(&mut separate, config(surface));
    }
    world
        .with_thermal_contact_candidate(config(surface), |w| {
            for _ in 0..4 {
                w.try_advance_tick()?;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(bytes(&world), bytes(&separate));
}

#[test]
fn v2_continued_hot_cold_rest_preserves_exposure_and_other_sleep_benefits() {
    for temperature in [-1.0, 1.0] {
        let (mut base, own, surface) = fixture_at(temperature, None, Tick(600));
        for _ in 0..10 {
            advance(&mut base, config(surface));
        }
        // A real shared injury makes the repair comparison nontrivial.
        let injury = BTreeMap::from([(
            OWN.raw(),
            BodyEventDelta {
                damage: 0.4,
                ..BodyEventDelta::zero()
            },
        )]);
        base.with_thermal_contact_candidate(config(surface), |w| {
            w.try_advance_tick_with_body_events(&injury)
        })
        .unwrap();
        let before = biology(&base);
        assert!(before.body.injury > 0.0);
        let initial = before.body.temperature_stress;
        let mut neural_rest = base.clone();
        let mut idle = base.clone();
        let mut rest = base.clone();
        let mut neutral_rest = base.clone();
        let mut left_rest = base;
        neutral_rest
            .objects
            .get_mut(&surface.raw())
            .unwrap()
            .grounded_physical
            .surface_temperature = 0.0;
        left_rest
            .editor_move_object(surface, Vec3f::new(10.0, 0.0, 0.0))
            .unwrap();
        for (world, kind) in [
            (&mut idle, ActionKind::Idle),
            (&mut rest, ActionKind::Rest),
            (&mut neutral_rest, ActionKind::Rest),
            (&mut left_rest, ActionKind::Rest),
        ] {
            let next = Tick(world.tick.raw() + 1);
            world
                .with_thermal_contact_candidate(config(surface), |w| {
                    let command = HeadlessWorldCommand::structured(
                        OWN,
                        kind.canonical_id(),
                        kind,
                        None,
                        None,
                    )?;
                    let receipt = w.apply_registered_command(&command, own, next)?;
                    if kind == ActionKind::Rest {
                        assert_eq!(receipt.action_result.body_event.sleep_recovery, 1.0);
                    }
                    w.try_advance_tick()
                })
                .unwrap();
        }
        neural_rest
            .with_thermal_contact_candidate(config(surface), |w| {
                let command = HeadlessWorldCommand::structured(
                    OWN,
                    ActionKind::Rest.canonical_id(),
                    ActionKind::Rest,
                    None,
                    None,
                )?;
                let channel =
                    alife_core::channel_command_for_action(MotorChannel::Posture, &command)?;
                let bundle = MotorCommandBundle::new(
                    OWN,
                    alife_core::ExperienceSequenceId(1),
                    w.tick,
                    vec![channel],
                )?;
                let neural = NeuralEmissionFrame::new(w.tick, 1, Vec::new())?;
                w.apply_registered_motor_bundle_with_neural_emission(&bundle, own, &neural)
                    .unwrap();
                w.try_advance_tick()
            })
            .unwrap();
        let hot_rest = biology(&rest);
        let clear_rest = biology(&neutral_rest);
        assert_eq!(hot_rest.body, biology(&neural_rest).body);
        assert_eq!(hot_rest.homeostasis, biology(&neural_rest).homeostasis);
        assert!(
            biology(&neural_rest)
                .biochemical_work()
                .neural_emitter_evaluations
                > hot_rest.biochemical_work().neural_emitter_evaluations
        );
        assert_eq!(
            rest.last_action_result.as_ref().unwrap().execution,
            neutral_rest.last_action_result.as_ref().unwrap().execution
        );
        assert!(hot_rest.body.health > before.body.health);
        assert!(hot_rest.body.temperature_stress > initial);
        assert_eq!(
            hot_rest.body.temperature_stress,
            biology(&idle).body.temperature_stress
        );
        assert_eq!(clear_rest.body.temperature_stress, 0.0);
        assert_eq!(biology(&left_rest), clear_rest);
        assert_eq!(
            hot_rest.homeostasis.drives.brain_atp,
            clear_rest.homeostasis.drives.brain_atp
        );
        assert_eq!(
            hot_rest.homeostasis.drives.fatigue,
            clear_rest.homeostasis.drives.fatigue
        );
        assert!(
            hot_rest.homeostasis.drives.brain_atp > biology(&idle).homeostasis.drives.brain_atp
        );
        assert_eq!(hot_rest.body.sleeping, clear_rest.body.sleeping);
        assert!(hot_rest.body.sleeping);
        assert_eq!(hot_rest.body.energy, clear_rest.body.energy);
        assert_eq!(hot_rest.body.health, clear_rest.body.health);
        assert_eq!(hot_rest.body.injury, clear_rest.body.injury);
        assert_eq!(hot_rest.reproduction, clear_rest.reproduction);
        assert_eq!(hot_rest.biochemical_work(), clear_rest.biochemical_work());
        for (a, b) in hot_rest
            .body
            .organs()
            .iter()
            .zip(clear_rest.body.organs().iter())
        {
            assert_eq!(a.energy, b.energy);
            assert_eq!(a.damage, b.damage);
            assert_eq!(a.integrity, b.integrity);
            assert_eq!(a.repair_capacity, b.repair_capacity);
        }
        let mut saved = PortableSaveFile::from_json_str(&bytes(&rest))
            .unwrap()
            .restore_headless_world()
            .unwrap();
        for w in [&mut rest, &mut saved] {
            let next = Tick(w.tick.raw() + 1);
            w.with_thermal_contact_candidate(config(surface), |w| {
                let command = HeadlessWorldCommand::structured(
                    OWN,
                    ActionKind::Rest.canonical_id(),
                    ActionKind::Rest,
                    None,
                    None,
                )?;
                w.apply_registered_command(&command, own, next)?;
                w.try_advance_tick()
            })
            .unwrap();
        }
        assert_eq!(bytes(&rest), bytes(&saved));
        println!("V2 temperature={temperature} prior={initial} hot_rest={} clear_rest={} ATP_hot={} ATP_clear={}",hot_rest.body.temperature_stress,clear_rest.body.temperature_stress,hot_rest.homeostasis.drives.brain_atp,clear_rest.homeostasis.drives.brain_atp);
    }
}

#[test]
fn v2_zero_external_dose_does_not_suppress_legacy_or_v1_sleep_cooling() {
    let (base, _, surface) = fixture_at(1.0, None, Tick(600));
    let record = base.organism_registry.get(OWN).unwrap();
    let p = record.phenotype();
    let before = biology(&base);
    let event = BodyEventDelta {
        sleep_recovery: 1.0,
        temperature_stress: 0.2,
        ..BodyEventDelta::zero()
    };
    let legacy = before.advance(Tick(601), event, p).unwrap();
    let v1 = before
        .advance_with_thermal_relief_candidate(Tick(601), Tick(601), event, None, p, 0.0)
        .unwrap();
    assert_eq!(
        serde_json::to_string(&legacy).unwrap(),
        serde_json::to_string(&v1).unwrap()
    );
    let mut candidate = base.clone();
    let mut p = config(surface);
    p.exposure_per_second = 0.0;
    let events = BTreeMap::from([(OWN.raw(), event)]);
    let mut ordinary = base;
    ordinary.try_advance_tick_with_body_events(&events).unwrap();
    candidate
        .with_thermal_contact_candidate(p, |w| w.try_advance_tick_with_body_events(&events))
        .unwrap();
    assert_eq!(bytes(&ordinary), bytes(&candidate));
}

#[test]
fn physical_withdrawal_uses_both_thermal_endpoints_and_truthful_once_only_receipt() {
    for entering in [true, false] {
        let (mut base, own, surface) = fixture(1.0);
        let x = if entering { -0.750001 } else { 0.749999 };
        base.editor_move_object(surface, Vec3f::new(x, 0.0, 0.0))
            .unwrap();
        let mut no_thermal = base.clone();
        let events = BTreeMap::from([(
            OWN.raw(),
            BodyEventDelta {
                damage: 0.4,
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
        )]);
        let plain = no_thermal
            .try_advance_tick_with_body_contact_candidate(&events)
            .unwrap();
        let mut combined = base;
        let receipt = combined
            .with_thermal_contact_candidate(config(surface), |world| {
                world.try_advance_tick_with_body_contact_candidate(&events)
            })
            .unwrap();
        assert_eq!(receipt.outcome_tick, Tick(1));
        assert_eq!(receipt.responses.len(), 1);
        let response = &receipt.responses[0];
        let reflex = response.withdrawal.as_ref().unwrap();
        assert!(!reflex.blocked);
        assert!(reflex.displacement.x < -0.00001);
        assert_eq!(response.withdrawal, plain.responses[0].withdrawal);
        let final_position = combined.objects[&own.raw()].position;
        let final_contact = distance(final_position, combined.objects[&surface.raw()].position)
            <= HEADLESS_CONTACT_RADIUS;
        assert_eq!(final_contact, entering);
        let tolerance = combined
            .organism_registry
            .get(OWN)
            .unwrap()
            .phenotype()
            .body
            .temperature_tolerance;
        let dose = 0.10 * (1.0 / WORLD_TICKS_PER_SECOND as f32);
        assert_eq!(response.applied_body_event.temperature_stress, dose);
        assert_eq!(response.applied_body_event.sleep_recovery, 0.0);
        assert!(
            (biology(&combined).body.temperature_stress - dose * (1.0 - tolerance)).abs() < 1e-7
        );
        assert_eq!(biology(&combined).tick, Tick(1));
        assert_eq!(
            biology(&combined).body.injury,
            biology(&no_thermal).body.injury
        );
        assert_eq!(
            biology(&combined).body.energy,
            biology(&no_thermal).body.energy
        );
        assert_eq!(biology(&no_thermal).body.temperature_stress, 0.0);
        // Chronic injury does not renew the peripheral movement. Continued
        // configured thermal load is an independent physical input, not pain.
        let quiet = combined
            .with_thermal_contact_candidate(config(surface), |world| {
                world.try_advance_tick_with_body_contact_candidate(&BTreeMap::new())
            })
            .unwrap();
        assert!(quiet.responses.is_empty());
        assert_eq!(combined.objects[&own.raw()].position, final_position);
        assert_eq!(biology(&combined).tick, Tick(2));
    }
}

#[test]
fn selected_physical_response_does_not_gate_configured_thermal_dose() {
    for selected in [false, true] {
        let (mut world, own, surface) = fixture(1.0);
        let before_position = world.objects[&own.raw()].position;
        let selected_ids = if selected {
            BTreeSet::from([OWN.raw()])
        } else {
            BTreeSet::new()
        };
        let events = BTreeMap::from([(
            OWN.raw(),
            BodyEventDelta {
                damage: 0.4,
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
        )]);
        let receipt = world
            .with_thermal_contact_candidate(config(surface), |world| {
                world.try_advance_tick_with_selected_body_contacts_in_staged_tick(
                    &events,
                    &selected_ids,
                )
            })
            .unwrap();
        assert_eq!(receipt.outcome_tick, Tick(1));
        assert_eq!(biology(&world).tick, Tick(1));
        let dose = 0.10 * (1.0 / WORLD_TICKS_PER_SECOND as f32);
        let tolerance = world
            .organism_registry
            .get(OWN)
            .unwrap()
            .phenotype()
            .body
            .temperature_tolerance;
        assert!((biology(&world).body.temperature_stress - dose * (1.0 - tolerance)).abs() < 1e-7);
        if selected {
            assert_eq!(receipt.responses.len(), 1);
            let response = &receipt.responses[0];
            assert_eq!(response.applied_body_event.temperature_stress, dose);
            assert_eq!(response.applied_body_event.sleep_recovery, 0.0);
            assert!(response.sleep_recovery_interrupted);
            assert!(!response.withdrawal.as_ref().unwrap().blocked);
            assert_ne!(world.objects[&own.raw()].position, before_position);
        } else {
            assert!(receipt.responses.is_empty());
            assert_eq!(world.objects[&own.raw()].position, before_position);
        }
    }
}
