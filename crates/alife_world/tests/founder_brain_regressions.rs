//! Source-to-behavior CPU reproductions. These evaluate the compiled founder
//! arithmetic with fresh zero mutable banks, neutral receptor gains and all
//! recurrent routes enabled; they are not physical-GPU acceptance evidence.
use alife_core::*;
use alife_world::*;

const SUBJECT: OrganismId = OrganismId(1);
const PROFILE: SensorProfile = SensorProfile::GroundedTerrainVisionV1;

fn brain(seed: u64, capacity: &BrainCapacityClass) -> BrainPhenotype {
    let creature = CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(10, 1, 7, capacity.id()).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let genome = creature.brain_genome;
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    PhenotypeCompiler::compile(&genome, capacity, &development, PROFILE).unwrap()
}

fn draft(world: &mut HeadlessWorld, drives: DriveSnapshot) -> PerceptionFrameDraft {
    draft_profile(world, drives, PROFILE)
}

fn draft_profile(
    world: &mut HeadlessWorld,
    drives: DriveSnapshot,
    profile: SensorProfile,
) -> PerceptionFrameDraft {
    let mut homeostasis = HomeostaticSnapshot::baseline(Tick::ZERO);
    homeostasis.drives = drives;
    world
        .perception_frame_draft(SUBJECT, Tick::ZERO, profile, homeostasis)
        .unwrap()
}

fn candidate(
    frame: &PerceptionFrameDraft,
    action: ActionId,
    target: WorldEntityId,
) -> ActionCandidate {
    *frame
        .candidates()
        .iter()
        .find(|c| c.action_id == action && c.target.entity == Some(target))
        .unwrap()
}

fn fresh_activations(brain: &BrainPhenotype, frame: &PerceptionFrameDraft) -> Vec<f32> {
    // Same source packing as closed_loop_buffers/perception.rs and same
    // encode/recurrent equations as the production WGSL. No learned banks,
    // structural branches or memory context exist in this fresh fixture.
    let body = frame.body().neural_projection(frame.sensor_profile());
    let body = [
        body.pose.translation.x,
        body.pose.translation.y,
        body.pose.translation.z,
        body.pose.rotation.x,
        body.pose.rotation.y,
        body.pose.rotation.z,
        body.pose.rotation.w,
        body.velocity.linear.x,
        body.velocity.linear.y,
        body.velocity.linear.z,
        body.velocity.angular.x,
        body.velocity.angular.y,
        body.velocity.angular.z,
    ];
    let homeostasis: Vec<_> = frame
        .homeostasis()
        .drives
        .to_array()
        .into_iter()
        .chain(frame.homeostasis().hormones.to_array())
        .collect();
    let sensory = frame.sensory().channels.as_flat_array();
    let n = brain.neuron_count() as usize;
    let mut encoded = vec![0.0_f32; n];
    for a in brain.sensor_encoder().assignments() {
        let lane = usize::from(a.source_index());
        let source = match a.source_group() {
            SensorEncoderSourceGroup::SensoryChannel => sensory[lane],
            SensorEncoderSourceGroup::Body => body[lane],
            SensorEncoderSourceGroup::Homeostasis => homeostasis[lane],
            SensorEncoderSourceGroup::HeardLanguage | SensorEncoderSourceGroup::SemanticPrior => {
                0.0
            }
        };
        let (lo, hi) = a.clamp_range();
        encoded[a.target_neuron() as usize] += (source * a.scale() + a.bias()).clamp(lo, hi);
    }
    encoded.iter_mut().for_each(|v| *v = v.clamp(-1.0, 1.0));
    let mut activations = vec![0.0_f32; n];
    let mut metabolic = vec![0.0_f32; n];
    for step in 0..brain.microstep_count() {
        let mut sum = vec![0.0_f32; n];
        for s in brain
            .synapses()
            .iter()
            .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
        {
            let route = &brain.projections()[usize::from(s.route_index())];
            let fires = match route.update_cadence() {
                UpdateCadence::Hot60Hz => true,
                UpdateCadence::Hot15To60Hz | UpdateCadence::Hot10To30Hz => step % 2 == 0,
                UpdateCadence::Hot5To15Hz | UpdateCadence::Hot1To5Hz => step == 0,
                _ => false,
            };
            if fires {
                sum[s.target() as usize] += activations[s.source() as usize] * s.genetic_weight();
            }
        }
        let mut next = vec![0.0; n];
        for (i, d) in brain.neuron_dynamics().iter().enumerate() {
            let pre = d.bias() + encoded[i] + sum[i] - d.homeostatic_gain() * metabolic[i];
            assert_eq!(d.activation(), ActivationFunction::Tanh);
            next[i] = (1.0 - d.leak()) * activations[i] + d.leak() * pre.tanh();
            metabolic[i] =
                d.metabolic_decay() * metabolic[i] + (1.0 - d.metabolic_decay()) * next[i].powi(2);
        }
        activations = next;
    }
    activations
}

fn decode(
    brain: &BrainPhenotype,
    motor: &[f32],
    c: ActionCandidate,
    drives: DriveSnapshot,
) -> (f32, f32) {
    let family = brain
        .candidate_decoder()
        .families()
        .iter()
        .find(|f| f.family() == c.family)
        .unwrap();
    let mut score = family.bias() + family.innate_contribution(drives, c.features);
    let mut distance_derivative = 0.0;
    for s in &brain.synapses()[family.decoder_synapse_start() as usize
        ..(family.decoder_synapse_start() + family.decoder_synapse_count()) as usize]
    {
        let CompiledSynapseKind::Decoder(coordinate) = s.kind() else {
            panic!("decoder slice")
        };
        let contribution = motor[s.source() as usize] * s.genetic_weight();
        score += contribution * c.features.0[usize::from(coordinate.input_lane())];
        if coordinate.input_lane() == 2 {
            distance_derivative += contribution;
        }
    }
    (score, distance_derivative)
}

#[test]
fn fresh_procedural_founders_prefer_nearer_identical_observations() {
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        for seed in [41, 96001, 96002] {
            let brain = brain(seed, &capacity);
            let properties =
                GroundedPhysicalProperties::deterministic_for_kind(WorldObjectKind::Food, 2);
            let mut world = HeadlessScenarioBuilder::new(41)
                .agent("subject", SUBJECT, Vec3f::ZERO)
                .food("object", Vec3f::new(4.0, 0.0, 0.0), 0.6)
                .grounded_physical("object", properties)
                .build()
                .unwrap();
            let target = world.entity_id("object").unwrap();
            let frame = draft(&mut world, DriveSnapshot::baseline());
            let mut near_world = world.clone();
            near_world
                .editor_move_object(target, Vec3f::new(2.0, 0.0, 0.0))
                .unwrap();
            near_world
                .set_grounded_physical_properties(target, properties)
                .unwrap();
            let near_frame = draft(&mut near_world, DriveSnapshot::baseline());
            let motor = fresh_activations(&brain, &frame);
            let near_motor = fresh_activations(&brain, &near_frame);
            let range = brain
                .lobe_layout()
                .region(LobeKind::ActionPlanning)
                .unwrap();
            let (min, max) = motor[range.start as usize..range.end() as usize]
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &x| {
                    (lo.min(x), hi.max(x))
                });
            if capacity.id() != BrainCapacityClass::N2048_ID {
                assert!(min > 0.0, "measured positive motor path: {min}..{max}");
            }
            for action in [
                HeadlessActionIds::APPROACH,
                HeadlessActionIds::FLEE,
                HeadlessActionIds::GRAB,
            ] {
                let far = candidate(&frame, action, target);
                let near = candidate(&near_frame, action, target);
                assert_eq!(near.features.0[2], 0.25);
                let mut normalized = far.features;
                normalized.0[2] = near.features.0[2];
                assert_eq!(normalized, near.features);
                let (far_score, derivative) =
                    decode(&brain, &motor, far, frame.homeostasis().drives);
                let (near_score, _) = decode(&brain, &motor, near, frame.homeostasis().drives);
                let (actual_near_score, _) =
                    decode(&brain, &near_motor, near, near_frame.homeostasis().drives);
                assert!(
                    actual_near_score > far_score,
                    "the complete near observation and recurrent path must also prefer proximity"
                );
                let finite_difference =
                    (far_score - near_score) / (far.features.0[2] - near.features.0[2]);
                assert!((finite_difference - derivative).abs() < 1e-4);
                println!("class={:?} seed={seed} family={:?} motor={min:.6}..{max:.6} near={near_score:.6} far={far_score:.6} dscore/ddistance={derivative:.6}", capacity.id(), far.family);
                assert!(
                    derivative < 0.0 && near_score > far_score,
                    "farther observations must not gain founder score"
                );
                // Decoder full sign path: dscore/dweight = motor*distance > 0,
                // while dscore/ddistance sums motor*signed_weight < 0.
                let row = &brain.candidate_decoder().families()[far.family.raw() as usize];
                let distance_rows: Vec<_> = brain.synapses()[row.decoder_synapse_start() as usize..
                    (row.decoder_synapse_start() + row.decoder_synapse_count()) as usize].iter()
                    .filter(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.input_lane() == 2)).collect();
                assert!(!distance_rows.is_empty());
                for s in distance_rows {
                    if capacity.id() != BrainCapacityClass::N2048_ID {
                        assert!(motor[s.source() as usize] * far.features.0[2] > 0.0);
                    }
                    assert!(s.genetic_weight() < 0.0);
                }
                if action == HeadlessActionIds::APPROACH {
                    let mut trial = world.clone();
                    trial
                        .apply_command(
                            &near
                                .to_command(SUBJECT, Confidence::new(1.0).unwrap())
                                .unwrap(),
                        )
                        .unwrap();
                    assert!(
                        trial
                            .entity(trial.entity_id("subject").unwrap())
                            .unwrap()
                            .position
                            .x
                            > 0.0
                    );
                }
            }
            for s in brain
                .synapses()
                .iter()
                .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
            {
                let route = &brain.projections()[usize::from(s.route_index())];
                if matches!(
                    route.projection_type(),
                    ProjectionType::MotorProposal | ProjectionType::Homeostatic
                ) {
                    assert!(s.genetic_weight() > 0.0);
                }
            }
        }
    }
}

#[test]
fn distant_visible_objects_support_urgency_driven_approach_and_avoid_without_chemical_gps() {
    let brain = brain(96001, &BrainCapacityClass::n512());
    let mut world = HeadlessScenarioBuilder::new(42)
        .agent("subject", SUBJECT, Vec3f::ZERO)
        .hazard("object", Vec3f::new(4.0, 0.0, 0.0), 0.15)
        .build()
        .unwrap();
    let target = world.entity_id("object").unwrap();
    let mut danger = DriveSnapshot::baseline();
    danger.pain = 0.98;
    let frame = draft(&mut world, danger);
    let slot = frame.grounded_object_slots()[0];
    assert_eq!(slot.contact, 0.0);
    assert_eq!(slot.chemical, [0.0; 3]);
    assert_eq!(slot.temperature, 0.0);
    let approach = candidate(&frame, HeadlessActionIds::APPROACH, target);
    let avoid = candidate(&frame, HeadlessActionIds::FLEE, target);
    let family = &brain.candidate_decoder().families()[CandidateActionFamily::Avoid.raw() as usize];
    let prior = family.innate_contribution(danger, avoid.features);
    println!("distant Avoid prior={prior} chemical={:?}", slot.chemical);
    assert!(prior > 0.0, "danger must permit avoidance before contact");
    assert_eq!(
        family.innate_contribution(DriveSnapshot::baseline(), avoid.features),
        0.0
    );
    let motor = fresh_activations(&brain, &frame);
    let select_move = |frame: &PerceptionFrameDraft, motor: &[f32], drives| {
        frame
            .candidates()
            .iter()
            .filter(|c| c.kind == ActionKind::Move)
            .max_by(|a, b| {
                decode(&brain, motor, **a, drives)
                    .0
                    .total_cmp(&decode(&brain, motor, **b, drives).0)
            })
            .unwrap()
            .action_id
    };
    assert_eq!(select_move(&frame, &motor, danger), HeadlessActionIds::FLEE);
    assert!(decode(&brain, &motor, avoid, danger).0 > decode(&brain, &motor, approach, danger).0);
    let before = world
        .entity(world.entity_id("subject").unwrap())
        .unwrap()
        .position
        .x;
    let mut retreat = world.clone();
    retreat
        .apply_command(
            &avoid
                .to_command(SUBJECT, Confidence::new(1.0).unwrap())
                .unwrap(),
        )
        .unwrap();
    assert!(
        retreat
            .entity(retreat.entity_id("subject").unwrap())
            .unwrap()
            .position
            .x
            < before
    );
    let mut hungry = DriveSnapshot::baseline();
    hungry.hunger = 0.98;
    let hungry_frame = draft(&mut world, hungry);
    let motor = fresh_activations(&brain, &hungry_frame);
    assert_eq!(
        select_move(&hungry_frame, &motor, hungry),
        HeadlessActionIds::APPROACH
    );
    assert!(decode(&brain, &motor, approach, hungry).0 > decode(&brain, &motor, avoid, hungry).0);
    world
        .apply_command(
            &approach
                .to_command(SUBJECT, Confidence::new(1.0).unwrap())
                .unwrap(),
        )
        .unwrap();
    assert!(
        world
            .entity(world.entity_id("subject").unwrap())
            .unwrap()
            .position
            .x
            > before
    );

    // Change only hidden source chemistry/temperature. Visible slots and candidate
    // decoder features stay identical; anonymous smell is allowed to change.
    let mut control = HeadlessScenarioBuilder::new(42)
        .agent("subject", SUBJECT, Vec3f::ZERO)
        .hazard("object", Vec3f::new(4.0, 0.0, 0.0), 0.15)
        .build()
        .unwrap();
    let mut props = GroundedPhysicalProperties::deterministic_for_kind(WorldObjectKind::Hazard, 2);
    props.chemical = [0.9, -0.4, 0.7];
    props.surface_temperature = 0.8;
    control
        .set_grounded_physical_properties(target, props)
        .unwrap();
    let changed = draft(&mut control, danger);
    assert_eq!(
        candidate(&changed, HeadlessActionIds::FLEE, target).features,
        avoid.features
    );
    assert_eq!(
        family.innate_contribution(
            danger,
            candidate(&changed, HeadlessActionIds::FLEE, target).features
        ),
        prior,
        "unavailable object valence cannot influence the danger cue"
    );
    let ingest =
        &brain.candidate_decoder().families()[CandidateActionFamily::Ingest.raw() as usize];
    let distant_eat = candidate(&frame, HeadlessActionIds::EAT, target);
    assert_eq!(
        ingest.innate_contribution(hungry, distant_eat.features),
        0.0
    );
    control
        .editor_move_object(target, Vec3f::new(0.1, 0.0, 0.0))
        .unwrap();
    let contact = draft(&mut control, hungry);
    assert_eq!(contact.grounded_object_slots()[0].contact, 1.0);
    assert_eq!(contact.grounded_object_slots()[0].chemical, props.chemical);
    assert_eq!(
        family.innate_contribution(
            danger,
            candidate(&contact, HeadlessActionIds::FLEE, target).features
        ),
        0.0,
        "contact must retain the original positive chemical cue interpretation"
    );
    assert!(
        ingest.innate_contribution(
            hungry,
            candidate(&contact, HeadlessActionIds::EAT, target).features
        ) > 0.0
    );
    props.chemical[0] = -0.9;
    control
        .set_grounded_physical_properties(target, props)
        .unwrap();
    let bitter_contact = draft(&mut control, danger);
    assert!(
        family.innate_contribution(
            danger,
            candidate(&bitter_contact, HeadlessActionIds::FLEE, target).features
        ) > 0.0
    );
    assert_eq!(
        ingest.innate_contribution(
            hungry,
            candidate(&bitter_contact, HeadlessActionIds::EAT, target).features
        ),
        0.0
    );
}

#[test]
fn play_and_grab_keep_all_physical_features_and_have_distinct_readouts() {
    let mut world = HeadlessScenarioBuilder::new(43)
        .agent("subject", SUBJECT, Vec3f::ZERO)
        .food("object", Vec3f::new(0.1, 0.0, 0.0), 0.6)
        .build()
        .unwrap();
    let target = world.entity_id("object").unwrap();
    // Exercise nonzero angular speed too: the ordinary world currently reports
    // zero angular velocity, but the sensor ABI permits the full physical range.
    for angular in [1.0, 0.5, 0.0] {
        let mut snapshot = world
            .physical_observation_snapshot(SUBJECT, Tick::ZERO)
            .unwrap();
        snapshot.observer_velocity.angular =
            Vec3f::new(0.0, angular * GROUNDED_VELOCITY_CEILING, 0.0);
        let mut tracker = TrackedObjectRegistry::new(world.seed(), 64).unwrap();
        let sensed = GroundedSensorExtractor::extract(&snapshot, &mut tracker).unwrap();
        let choices = GroundedCandidateEnumerator
            .enumerate_candidates(&sensed, PROFILE)
            .unwrap();
        let grab = choices
            .iter()
            .find(|c| c.action_id == HeadlessActionIds::GRAB)
            .unwrap();
        let play = choices
            .iter()
            .find(|c| c.action_id == HeadlessActionIds::PLAY)
            .unwrap();
        println!(
            "angular={angular} grab_family={:?} play_family={:?} identical_features={}",
            grab.family,
            play.family,
            grab.features == play.features
        );
        assert_ne!(grab.family, play.family);
        for action in [HeadlessActionIds::GRAB, HeadlessActionIds::PLAY] {
            let observed = choices.iter().find(|c| c.action_id == action).unwrap();
            assert_eq!(
                observed.features,
                sensed.slots()[0].candidate_features().unwrap()
            );
            assert_eq!(observed.features.0[20], angular);
        }
    }
    let frame = draft(&mut world, DriveSnapshot::baseline());
    let grab = candidate(&frame, HeadlessActionIds::GRAB, target);
    let play = candidate(&frame, HeadlessActionIds::PLAY, target);
    let physical = frame.grounded_object_slots()[0]
        .candidate_features()
        .unwrap();
    assert_eq!(
        play.features, physical,
        "Play must preserve angular speed and every physical lane"
    );
    assert_eq!(grab.features, physical);
    assert_ne!(
        play.family, grab.family,
        "identical physical evidence needs distinct action identity"
    );
    let brain = brain(96001, &BrainCapacityClass::n512());
    let motor = fresh_activations(&brain, &frame);
    let play_score = decode(&brain, &motor, play, frame.homeostasis().drives).0;
    let grab_score = decode(&brain, &motor, grab, frame.homeostasis().drives).0;
    assert_ne!(play_score, grab_score);
    println!(
        "distinct Play={play_score} Grab={grab_score} angular={}",
        play.features.0[20]
    );
}

#[test]
fn play_other_stays_manipulation_in_joint_commands_memory_and_decision_history() {
    let mut world = HeadlessScenarioBuilder::new(96003)
        .agent("subject", SUBJECT, Vec3f::ZERO)
        .build()
        .unwrap();
    let entity = world.entity_id("subject").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        96003,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::newborn(SUBJECT, entity, genome, phenotype, Tick::ZERO).unwrap(),
        )
        .unwrap();
    let toy = world
        .spawn_toy("fixed", Vec3f::new(0.1, 0.0, 0.0), false)
        .unwrap();
    let draft = draft(&mut world, DriveSnapshot::baseline());
    let play = candidate(&draft, HeadlessActionIds::PLAY, toy);
    let grab = candidate(&draft, HeadlessActionIds::GRAB, toy);
    let vocal = *draft
        .candidates()
        .iter()
        .find(|c| c.kind == ActionKind::Vocalize)
        .unwrap();
    let posture = *draft
        .candidates()
        .iter()
        .find(|c| c.action_id == HeadlessActionIds::NO_POSTURE)
        .unwrap();
    assert_eq!(play.family, CandidateActionFamily::Other);
    assert_eq!(play.kind, ActionKind::Interact);
    for other in [vocal, posture] {
        assert_eq!(other.family, CandidateActionFamily::Other);
        assert_ne!(other.features, play.features);
    }
    let query = MemoryQueryEncoderV2::encode_candidate(&draft, &play).unwrap();
    let grip_query = MemoryQueryEncoderV2::encode_candidate(&draft, &grab).unwrap();
    assert_eq!(query.action_family(), CandidateActionFamily::Other);
    assert_eq!(grip_query.action_family(), CandidateActionFamily::Contact);
    assert_ne!(query.features(), grip_query.features());
    let frame = draft.finalize(PerceptionContextBlock::empty()).unwrap();
    let command = play
        .to_command(SUBJECT, Confidence::new(1.0).unwrap())
        .unwrap();
    let mut slots = [0; 6];
    slots[2] = play.candidate_index + 1;
    slots[3] = vocal.candidate_index + 1;
    slots[4] = posture.candidate_index + 1;
    let bundle = factorized_motor_bundle_for_candidates(
        SUBJECT,
        ExperienceSequenceId(1),
        Tick::ZERO,
        &frame,
        slots,
        &[
            MotorChannel::Manipulation,
            MotorChannel::Vocal,
            MotorChannel::Posture,
        ],
        &command,
        play.candidate_index,
        None,
        false,
    )
    .unwrap();
    assert!(
        bundle
            .channels
            .iter()
            .any(|c| c.channel == MotorChannel::Manipulation
                && c.primitive == HeadlessActionIds::PLAY)
    );
    assert!(bundle
        .channels
        .iter()
        .any(|c| c.channel == MotorChannel::Vocal && c.primitive == vocal.action_id));
    assert!(bundle
        .channels
        .iter()
        .any(|c| c.channel == MotorChannel::Posture && c.primitive == posture.action_id));
    JointActionSelectionV1::new(slots, &bundle)
        .unwrap()
        .validate_bundle(&bundle)
        .unwrap();
    let brain = brain(96003, &BrainCapacityClass::n512());
    let decision = DecisionSnapshot::from_neural_selection(
        ExperienceSequenceId(1),
        brain.phenotype_hash(),
        1,
        0,
        &frame,
        NeuralActionSelection {
            candidate_index: play.candidate_index,
            logit: 1.0,
            confidence: Confidence::new(1.0).unwrap(),
            active_tiles: 1,
            active_synapses: 1,
        },
        command,
    )
    .unwrap();
    let restored: DecisionSnapshot =
        serde_json::from_slice(&serde_json::to_vec(&decision).unwrap()).unwrap();
    assert_eq!(restored, decision);
    assert_eq!(
        restored.neural_evidence().unwrap().action_family,
        CandidateActionFamily::Other
    );
    assert_eq!(restored.selected_action.action_id, HeadlessActionIds::PLAY);
    let receipt = world
        .apply_registered_motor_bundle(&bundle, entity)
        .unwrap();
    let activation = receipt
        .channel_receipts
        .iter()
        .find(|r| r.command.channel == MotorChannel::Manipulation)
        .unwrap();
    assert_eq!(activation.command.primitive, HeadlessActionIds::PLAY);
    assert_eq!(activation.phase, ActionPhase::Completed);
    assert!(activation.play_stimulation > 0.0);
    assert_eq!(activation.physical.target_entity, Some(toy));
}

#[test]
fn ordinary_default_nano_newborn_damage_reaches_pain_without_maturing_other_chemistry() {
    let config = CanonicalNewGameConfig::phase3(240824, 1).unwrap();
    assert_eq!(config.brain_class, BrainScaleTier::Nano512);
    let foundation = FoundationWeightAsset::builtin_nano512_v1(config.sensor_profile).unwrap();
    let game = create_canonical_new_game(&config, &foundation).unwrap();
    let record = game.world.organism_registry().get(SUBJECT).unwrap();
    let newborn = record.biochemistry();
    let hurt = newborn
        .advance_with_age(
            Tick(1),
            Tick(1),
            BodyEventDelta {
                damage: 0.15,
                ..BodyEventDelta::zero()
            },
            record.phenotype(),
        )
        .unwrap();
    let quiet = newborn
        .advance_with_age(Tick(1), Tick(1), BodyEventDelta::zero(), record.phenotype())
        .unwrap();
    println!(
        "ordinary Nano damage=.15 injury={} pain={} expression={}",
        hurt.body.injury, hurt.homeostasis.drives.pain, hurt.development.biochemical_expression
    );
    assert!(hurt.body.injury > newborn.body.injury);
    assert!((hurt.homeostasis.drives.pain - 0.12).abs() < 1e-6);
    assert_eq!(quiet.homeostasis.drives.pain, 0.0);
    assert_eq!(hurt.development.biochemical_expression, 0.0);
    let fed_and_hurt = newborn
        .advance_with_age(
            Tick(1),
            Tick(1),
            BodyEventDelta {
                nutrition: 1.0,
                damage: 0.15,
                ..BodyEventDelta::zero()
            },
            record.phenotype(),
        )
        .unwrap();
    assert_eq!(
        fed_and_hurt.graph_state(),
        hurt.graph_state(),
        "the pain floor must not mature nutrition chemistry"
    );
    let mut world = game.world;
    let (organism, entity) = world.organism_entity_ids()[0];
    let hazard = world.entity_id("hazard-01").unwrap();
    let position = world.entity(entity).unwrap().position;
    world.editor_move_object(hazard, position).unwrap();
    let command = HeadlessWorldCommand::approach(organism, hazard).unwrap();
    let collision = world
        .apply_registered_command(&command, entity, Tick(1))
        .unwrap();
    assert!(collision.action_result.body_event.damage > 0.0);
    assert!(collision.biology_after.homeostasis.drives.pain > 0.0);
    println!(
        "ordinary Nano real collision damage={} pain={}",
        collision.action_result.body_event.damage, collision.biology_after.homeostasis.drives.pain
    );
}

#[test]
fn combined_hunger_and_danger_use_measured_cues_and_actual_movement() {
    let brain = brain(96001, &BrainCapacityClass::n512());
    for (name, danger_lanes) in [("fear", 1), ("pain", 2), ("temperature", 4), ("all", 7)] {
        let mut drives = DriveSnapshot::baseline();
        drives.hunger = 0.98;
        drives.fear = if danger_lanes & 1 != 0 {
            0.98
        } else {
            drives.fear
        };
        drives.pain = if danger_lanes & 2 != 0 { 0.98 } else { 0.0 };
        drives.temperature_stress = if danger_lanes & 4 != 0 { 0.98 } else { 0.0 };
        for distance in [4.0, 0.1] {
            for chemical in [-0.9, 0.9] {
                let mut props =
                    GroundedPhysicalProperties::deterministic_for_kind(WorldObjectKind::Hazard, 2);
                props.chemical = [chemical, 0.0, 0.0];
                let mut world = HeadlessScenarioBuilder::new(42)
                    .agent("subject", SUBJECT, Vec3f::ZERO)
                    .hazard("object", Vec3f::new(distance, 0.0, 0.0), 0.15)
                    .grounded_physical("object", props)
                    .build()
                    .unwrap();
                let target = world.entity_id("object").unwrap();
                let frame = draft(&mut world, drives);
                let approach = candidate(&frame, HeadlessActionIds::APPROACH, target);
                let avoid = candidate(&frame, HeadlessActionIds::FLEE, target);
                let ingest = candidate(&frame, HeadlessActionIds::EAT, target);
                let families = brain.candidate_decoder().families();
                let approach_plan = &families[CandidateActionFamily::Approach.raw() as usize];
                let avoid_plan = &families[CandidateActionFamily::Avoid.raw() as usize];
                let ingest_plan = &families[CandidateActionFamily::Ingest.raw() as usize];
                let approach_cue = approach_plan.innate_contribution(drives, approach.features)
                    / ((drives.hunger - 0.55) * approach_plan.innate_gain());
                let avoid_cue = avoid_plan.innate_contribution(drives, avoid.features)
                    / ((0.98 - 0.55) * avoid_plan.innate_gain());
                let contact = distance < 1.0;
                let expected_avoid_cue = if !contact {
                    1.0
                } else if chemical < 0.0 {
                    (0.9 - 0.4) / 0.6
                } else {
                    0.0
                };
                assert!((avoid_cue - expected_avoid_cue).abs() < 1e-5);
                assert!((approach_cue - if contact { 0.0 } else { 1.0 }).abs() < 1e-5);
                assert_eq!(
                    frame.grounded_object_slots()[0].chemical[0],
                    if contact { chemical } else { 0.0 }
                );
                let taste = ingest_plan.innate_contribution(drives, ingest.features);
                assert_eq!(taste > 0.0, contact && chemical > 0.0);
                let motor = fresh_activations(&brain, &frame);
                let selected = *frame
                    .candidates()
                    .iter()
                    .filter(|c| c.kind == ActionKind::Move)
                    .max_by(|a, b| {
                        decode(&brain, &motor, **a, drives)
                            .0
                            .total_cmp(&decode(&brain, &motor, **b, drives).0)
                    })
                    .unwrap();
                assert_eq!(
                    selected.family,
                    if contact && chemical > 0.0 {
                        CandidateActionFamily::Approach
                    } else {
                        CandidateActionFamily::Avoid
                    },
                    "{name} distance={distance} chemical={chemical}"
                );
                let approach_score = decode(&brain, &motor, approach, drives).0;
                let avoid_score = decode(&brain, &motor, avoid, drives).0;
                world
                    .apply_command(
                        &selected
                            .to_command(SUBJECT, Confidence::new(1.0).unwrap())
                            .unwrap(),
                    )
                    .unwrap();
                let dx = world
                    .entity(world.entity_id("subject").unwrap())
                    .unwrap()
                    .position
                    .x;
                if selected.family == CandidateActionFamily::Avoid {
                    assert!(dx < 0.0);
                }
                if selected.family == CandidateActionFamily::Approach {
                    assert!(dx > 0.0);
                }
                println!("combined={name} distance={distance} chemical={chemical} normalized_cues={approach_cue:.6}/{avoid_cue:.6} scores={approach_score:.6}/{avoid_score:.6} selected={:?} dx={dx:.6} taste_prior={taste:.6}", selected.family);
            }
        }
    }
}

#[test]
fn fresh_builtin_foundations_prefer_nearer_identical_observations() {
    let mut proximity_holds = true;
    let profile = SensorProfile::GroundedObjectSlotsV1;
    for capacity in [BrainCapacityClass::n512(), BrainCapacityClass::n2048()] {
        for seed in [41, 96001, 96002] {
            let source = if capacity.id() == BrainCapacityClass::N512_ID {
                FoundationWeightAsset::builtin_nano512_v1(profile).unwrap()
            } else {
                FoundationWeightAsset::builtin_n2048_v1(profile).unwrap()
            };
            let manifest = source.manifest();
            let creature = CreatureGenome::early_mammal_founder(
                seed,
                FoundationGeneticIdentity::new(
                    manifest.foundation_id().raw(),
                    u16::try_from(manifest.foundation_version().raw()).unwrap(),
                    manifest.compatibility_family_id().raw(),
                    capacity.id(),
                )
                .unwrap(),
            )
            .unwrap()
            .express()
            .unwrap();
            let brain = if capacity.id() == BrainCapacityClass::N512_ID {
                N512FounderFoundationProjection::compile(&creature, profile, &source)
                    .unwrap()
                    .compiled_phenotype()
                    .clone()
            } else {
                let genome = &creature.brain_genome;
                let development = DevelopmentState::new(
                    genome.id,
                    Tick::ZERO,
                    NormalizedScalar::new(1.0).unwrap(),
                );
                PhenotypeCompiler::compile_from_foundation_asset(
                    genome,
                    &capacity,
                    &development,
                    profile,
                    &source,
                )
                .unwrap()
            };
            let props =
                GroundedPhysicalProperties::deterministic_for_kind(WorldObjectKind::Food, 2);
            let mut world = HeadlessScenarioBuilder::new(41)
                .agent("subject", SUBJECT, Vec3f::ZERO)
                .food("object", Vec3f::new(4.0, 0.0, 0.0), 0.6)
                .grounded_physical("object", props)
                .build()
                .unwrap();
            let target = world.entity_id("object").unwrap();
            let far_frame = draft_profile(&mut world, DriveSnapshot::baseline(), profile);
            world
                .editor_move_object(target, Vec3f::new(2.0, 0.0, 0.0))
                .unwrap();
            world
                .set_grounded_physical_properties(target, props)
                .unwrap();
            let near_frame = draft_profile(&mut world, DriveSnapshot::baseline(), profile);
            let motor = fresh_activations(&brain, &far_frame);
            let near_motor = fresh_activations(&brain, &near_frame);
            let motor_lobe = brain
                .lobe_layout()
                .region(LobeKind::ActionPlanning)
                .unwrap();
            let (min, max) = motor[motor_lobe.start as usize..motor_lobe.end() as usize]
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &value| {
                    (lo.min(value), hi.max(value))
                });
            println!(
                "builtin class={:?} seed={seed} motor={min:.8}..{max:.8}",
                capacity.id()
            );
            for action in [
                HeadlessActionIds::APPROACH,
                HeadlessActionIds::FLEE,
                HeadlessActionIds::GRAB,
            ] {
                let near = candidate(&near_frame, action, target);
                let far = candidate(&far_frame, action, target);
                let mut normalized = far.features;
                normalized.0[2] = near.features.0[2];
                assert_eq!(normalized, near.features);
                let (far_score, derivative) =
                    decode(&brain, &motor, far, far_frame.homeostasis().drives);
                let near_score = decode(&brain, &motor, near, near_frame.homeostasis().drives).0;
                let actual_near =
                    decode(&brain, &near_motor, near, near_frame.homeostasis().drives).0;
                let fd = (far_score - near_score) / (far.features.0[2] - near.features.0[2]);
                println!("builtin class={:?} seed={seed} family={:?} near={near_score:.8} far={far_score:.8} actual_near={actual_near:.8} derivative={derivative:.8}", capacity.id(), far.family);
                assert!((fd - derivative).abs() < 1e-4);
                proximity_holds &=
                    derivative < 0.0 && near_score > far_score && actual_near > far_score;
            }
        }
    }
    assert!(
        proximity_holds,
        "fresh builtin replacement must retain a proximity disposition"
    );
}
