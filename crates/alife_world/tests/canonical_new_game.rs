use alife_core::{
    BrainCapacityClass, FoundationGeneticIdentity, FoundationWeightAsset,
    N512FounderFoundationProjection, PhysicalContactKind, SensorProfile, Tick, Validate,
};
use alife_world::{
    create_canonical_new_game, CanonicalNewGameConfig, HeadlessWorldCommand, WorldObjectKind,
};

fn phase3_game(population: u16) -> alife_world::CanonicalNewGame {
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    create_canonical_new_game(
        &CanonicalNewGameConfig::phase3(240_824, population).unwrap(),
        &foundation,
    )
    .unwrap()
}

#[test]
fn player_care_is_local_gene_controlled_and_survives_pending_save() {
    let game = phase3_game(1);
    let mut world = game.world;
    let (organism, entity) = world.organism_entity_ids()[0];
    let position = world.entity(entity).unwrap().position;
    let before_signature = world.canonical_signature_digest().unwrap();
    let mut control = world.clone();
    assert!(world
        .queue_player_care(organism, alife_core::Vec3f::new(100.0, 0.0, 0.0), true)
        .is_err());
    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        before_signature
    );
    world.queue_player_care(organism, position, true).unwrap();
    world.queue_player_care(organism, position, true).unwrap();
    world.queue_player_care(organism, position, false).unwrap();
    let mut acting = world.clone();
    let action = HeadlessWorldCommand::idle(organism).unwrap();
    let action_receipt = acting
        .apply_registered_command(&action, entity, Tick(1))
        .unwrap();
    assert_eq!(action_receipt.action_result.body_event.player_reward, 1.0);
    assert!(action_receipt.biology_after.homeostasis.hormones.extension[0] > 0.0);
    acting.try_advance_tick().unwrap();
    let next = acting
        .apply_registered_command(&action, entity, Tick(2))
        .unwrap();
    assert_eq!(next.action_result.body_event.player_reward, 0.0);
    let before = *world
        .organism_registry()
        .get(organism)
        .unwrap()
        .biochemistry();
    let save = alife_world::PortableSaveFile::from_headless_world(
        "care",
        &world,
        alife_world::RuntimeConfig::deterministic_default(
            world.seed(),
            alife_core::BrainScaleTier::Nano512,
        ),
        alife_world::AssetManifest::empty(),
        game.creatures,
    )
    .unwrap();
    let mut restored =
        alife_world::PortableSaveFile::from_json_str(&serde_json::to_string(&save).unwrap())
            .unwrap()
            .restore_headless_world()
            .unwrap();
    world.try_advance_tick().unwrap();
    control.try_advance_tick().unwrap();
    restored.try_advance_tick().unwrap();
    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        restored.canonical_signature_digest().unwrap()
    );
    let after = *world
        .organism_registry()
        .get(organism)
        .unwrap()
        .biochemistry();
    assert!(after.homeostasis.hormones.extension[0] > before.homeostasis.hormones.extension[0]);
    let untouched = control
        .organism_registry()
        .get(organism)
        .unwrap()
        .biochemistry();
    assert!(after.homeostasis.drives.loneliness <= untouched.homeostasis.drives.loneliness);
    assert!(after.homeostasis.hormones.oxytocin > untouched.homeostasis.hormones.oxytocin);
    assert!(
        alife_core::MeasuredPhysiologyTransition::new(before, after)
            .unwrap()
            .homeostatic_improvement()
            > 0.0
    );
    world.try_advance_tick().unwrap();
    let faded = *world
        .organism_registry()
        .get(organism)
        .unwrap()
        .biochemistry();
    assert!(faded.homeostasis.hormones.extension[0] < after.homeostasis.hormones.extension[0]);
    let mut neutral = alife_core::HomeostaticDelta::zero();
    neutral.hormones.extension[0] = -0.5;
    assert_eq!(after.value_profile().value_change(neutral, 0.0), 0.0);
}

#[test]
fn food_and_toy_variants_have_physical_gene_controlled_and_persistent_effects() {
    use alife_core::{
        ActionCommand, ActionKind, ActionTarget, AlleleSide, BiochemicalSourceLocus,
        BodyEventDelta, Confidence, DurationTicks, Intensity, Vec3f,
    };
    use alife_world::{FoodVariety, HeadlessActionIds};
    let game = phase3_game(1);
    let mut world = game.world;
    let (organism, entity) = world.organism_entity_ids()[0];
    let position = world.entity(entity).unwrap().position;
    let food = world.entity_id("food-01").unwrap();
    let mut foods = Vec::new();
    for variety in [FoodVariety::Root, FoodVariety::Fruit, FoodVariety::Seed] {
        let mut trial = world.clone();
        trial.set_food_variety(food, variety).unwrap();
        trial.editor_move_object(food, position).unwrap();
        let object = trial.entity(food).unwrap().clone();
        let command = ActionCommand::structured(
            organism,
            HeadlessActionIds::EAT,
            ActionKind::Interact,
            ActionTarget::new(Some(food), None),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(1.0).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap();
        let receipt = trial
            .apply_registered_command(&command, entity, Tick(1))
            .unwrap();
        assert!(receipt.action_result.execution.succeeded);
        assert_eq!(
            receipt.action_result.body_event.nutrition,
            variety.nutrition()
        );
        assert!(trial.entity(food).unwrap().consumed);
        foods.push((object.nutrition, object.grounded_physical));
    }
    assert!(foods.windows(2).all(|pair| pair[0].0 != pair[1].0
        && pair[0].1.color != pair[1].1.color
        && pair[0].1.chemical != pair[1].1.chemical));
    let ball = world.spawn_toy("test-ball", position, true).unwrap();
    let station = world
        .spawn_toy(
            "test-station",
            Vec3f::new(position.x + 0.1, position.y, position.z),
            false,
        )
        .unwrap();
    let command = |action, target| {
        ActionCommand::structured(
            organism,
            action,
            ActionKind::Interact,
            ActionTarget::new(Some(target), None),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(1.0).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap()
    };
    let baseline = world.clone();
    let get = world
        .apply_registered_command(&command(HeadlessActionIds::GRAB, ball), entity, Tick(1))
        .unwrap();
    assert!(get.action_result.execution.succeeded);
    assert_eq!(world.entity(ball).unwrap().carried_by, Some(organism));
    world.try_advance_tick().unwrap();
    let release = world
        .apply_registered_command(&command(HeadlessActionIds::GRAB, ball), entity, Tick(2))
        .unwrap();
    assert!(release.action_result.execution.succeeded);
    assert_eq!(world.entity(ball).unwrap().carried_by, None);
    world.try_advance_tick().unwrap();
    for toy in [ball, station] {
        let mut trial = baseline.clone();
        let play = trial
            .apply_registered_command(&command(HeadlessActionIds::PLAY, toy), entity, Tick(1))
            .unwrap();
        assert!(play.action_result.execution.succeeded);
        assert_eq!(play.action_result.body_event.play_stimulation, 1.0);
        assert_eq!(
            trial.entity(toy).unwrap().position,
            baseline.entity(toy).unwrap().position
        );
        assert!(!trial.entity(toy).unwrap().consumed);
        let mut trial = baseline.clone();
        assert!(
            !trial
                .apply_registered_command(&command(HeadlessActionIds::EAT, toy), entity, Tick(1))
                .unwrap()
                .action_result
                .execution
                .succeeded
        );
    }
    let mut trial = baseline.clone();
    assert!(
        !trial
            .apply_registered_command(&command(HeadlessActionIds::GRAB, station), entity, Tick(1))
            .unwrap()
            .action_result
            .execution
            .succeeded
    );
    trial.try_advance_tick().unwrap();
    trial
        .editor_move_object(
            station,
            Vec3f::new(position.x + 10.0, position.y, position.z),
        )
        .unwrap();
    assert!(
        !trial
            .apply_registered_command(&command(HeadlessActionIds::PLAY, station), entity, Tick(2))
            .unwrap()
            .action_result
            .execution
            .succeeded
    );
    // Same physical play: new toys satisfy curiosity, familiar toys relieve boredom.
    let channel = alife_core::ChannelCommand::new(
        alife_core::MotorChannel::Manipulation,
        HeadlessActionIds::PLAY,
        Some(ActionTarget::new(Some(ball), None)),
        Vec3f::ZERO,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(1.0).unwrap(),
        0,
    )
    .unwrap();
    let bundle = alife_core::MotorCommandBundle::new(
        organism,
        alife_core::ExperienceSequenceId(1),
        baseline.tick(),
        vec![channel],
    )
    .unwrap();
    let neural = alife_core::NeuralEmissionFrame::new(
        baseline.tick(),
        1,
        vec![alife_core::NeuralEmission::new(
            alife_core::NeuralEmissionClass::ExecutiveSustain,
            0.0,
            1.0,
        )
        .unwrap()],
    )
    .unwrap();
    let mut new_toy = baseline.clone();
    let mut familiar_toy = baseline.clone();
    let new_play = new_toy
        .apply_registered_motor_bundle_with_perceived_novelty_in_staged_tick(
            &bundle,
            entity,
            &neural,
            1.0,
            &[(ball, 1.0)],
        )
        .unwrap();
    let familiar_play = familiar_toy
        .apply_registered_motor_bundle_with_perceived_novelty_in_staged_tick(
            &bundle,
            entity,
            &neural,
            0.0,
            &[(ball, 0.0)],
        )
        .unwrap();
    assert_eq!(new_play.body_event.play_stimulation, 0.0);
    assert_eq!(new_play.body_event.investigation, 1.0);
    assert_eq!(familiar_play.body_event.play_stimulation, 1.0);
    assert_eq!(
        new_toy.entity(ball).unwrap().position,
        familiar_toy.entity(ball).unwrap().position
    );
    // Looking at a familiar ball must not make a different, new station
    // relieve boredom. Each channel uses its own physical interaction target.
    let inspect = alife_core::ChannelCommand::new(
        alife_core::MotorChannel::Posture,
        ActionKind::Inspect.canonical_id(),
        Some(ActionTarget::new(Some(ball), None)),
        Vec3f::ZERO,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(1.0).unwrap(),
        0,
    )
    .unwrap();
    let mut play = bundle.channels[0].clone();
    play.target = Some(ActionTarget::new(Some(station), None));
    let mixed = alife_core::MotorCommandBundle::new(
        organism,
        alife_core::ExperienceSequenceId(1),
        baseline.tick(),
        vec![inspect, play],
    )
    .unwrap();
    let mut mixed_world = baseline.clone();
    let mixed_play = mixed_world
        .apply_registered_motor_bundle_with_perceived_novelty_in_staged_tick(
            &mixed,
            entity,
            &neural,
            0.0,
            &[(ball, 0.0), (station, 1.0)],
        )
        .unwrap();
    assert!(mixed_play.succeeded);
    assert_eq!(mixed_play.body_event.play_stimulation, 0.0);
    assert_eq!(mixed_play.body_event.investigation, 1.0);
    // The chemical response is inherited and can be silenced by its gene.
    let record = baseline.organism_registry().get(organism).unwrap();
    let mut genome = record.genome().clone();
    let index = genome
        .chemistry
        .graph
        .expressed()
        .emitters()
        .iter()
        .position(|row| row.source == BiochemicalSourceLocus::PlayStimulation)
        .unwrap();
    for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
        genome.chemistry.graph = genome
            .chemistry
            .graph
            .clone()
            .with_emitter_gain(side, index, 0.0)
            .unwrap();
    }
    let no_play = genome.express().unwrap();
    let mut chemistry = *record.biochemistry();
    for tick in 1..=600 {
        chemistry = chemistry
            .advance(Tick(tick), BodyEventDelta::zero(), record.phenotype())
            .unwrap();
    }
    let stimulus = BodyEventDelta {
        play_stimulation: 1.0,
        ..BodyEventDelta::zero()
    };
    let pleased = chemistry
        .advance(Tick(601), stimulus, record.phenotype())
        .unwrap();
    let unchanged = chemistry.advance(Tick(601), stimulus, &no_play).unwrap();
    assert!(pleased.homeostasis.drives.extension[0] < unchanged.homeostasis.drives.extension[0]);
    let relief = alife_core::MeasuredPhysiologyTransition::new(chemistry, pleased)
        .unwrap()
        .homeostatic_improvement();
    assert!(relief > 0.0);
    // Novel stimuli raise curiosity; investigation relieves it through genes.
    let novelty = BodyEventDelta {
        perceived_novelty: 1.0,
        ..BodyEventDelta::zero()
    };
    let curious = chemistry
        .advance(Tick(601), novelty, record.phenotype())
        .unwrap();
    let neutral = chemistry
        .advance(Tick(601), BodyEventDelta::zero(), record.phenotype())
        .unwrap();
    assert!(curious.homeostasis.drives.curiosity > neutral.homeostasis.drives.curiosity);
    let investigated = curious
        .advance(
            Tick(602),
            BodyEventDelta {
                investigation: 1.0,
                ..BodyEventDelta::zero()
            },
            record.phenotype(),
        )
        .unwrap();
    let uninvolved = curious
        .advance(Tick(602), BodyEventDelta::zero(), record.phenotype())
        .unwrap();
    assert!(investigated.homeostasis.drives.curiosity < uninvolved.homeostasis.drives.curiosity);
    let mut genome = record.genome().clone();
    let index = genome
        .chemistry
        .graph
        .expressed()
        .emitters()
        .iter()
        .position(|row| row.source == BiochemicalSourceLocus::PerceivedNovelty)
        .unwrap();
    for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
        genome.chemistry.graph = genome
            .chemistry
            .graph
            .clone()
            .with_emitter_gain(side, index, 0.0)
            .unwrap();
    }
    let silenced = chemistry
        .advance(Tick(601), novelty, &genome.express().unwrap())
        .unwrap();
    assert_eq!(
        silenced.homeostasis.drives.curiosity,
        neutral.homeostasis.drives.curiosity
    );
    let save = alife_world::PortableSaveFile::from_headless_world(
        "variety",
        &world,
        alife_world::RuntimeConfig::deterministic_default(
            world.seed(),
            alife_core::BrainScaleTier::Nano512,
        ),
        alife_world::AssetManifest::empty(),
        game.creatures,
    )
    .unwrap();
    let restored =
        alife_world::PortableSaveFile::from_json_str(&serde_json::to_string(&save).unwrap())
            .unwrap()
            .restore_headless_world()
            .unwrap();
    assert_eq!(world.object_snapshots(), restored.object_snapshots());
    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        restored.canonical_signature_digest().unwrap()
    );
}

#[test]
fn canonical_new_game_creates_exact_requested_population() {
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();

    for population in [1, 4, 6, 8] {
        let game = create_canonical_new_game(
            &CanonicalNewGameConfig::phase3(240_824, population).unwrap(),
            &foundation,
        )
        .unwrap();

        assert_eq!(game.receipt.requested_population, population);
        assert_eq!(
            game.world.organism_registry().len(),
            usize::from(population)
        );
        assert_eq!(game.creatures.len(), usize::from(population));
        assert_eq!(game.receipt.founders.len(), usize::from(population));
    }
}

#[test]
fn canonical_new_game_rejects_population_outside_phase3_bounds() {
    assert!(CanonicalNewGameConfig::phase3(1, 0).is_err());
    assert!(CanonicalNewGameConfig::phase3(1, 9).is_err());
    assert!(CanonicalNewGameConfig::phase3(0, 6).is_err());
}

#[test]
fn canonical_new_game_binds_every_subsystem_before_admission() {
    let game = phase3_game(6);

    for founder in &game.receipt.founders {
        let record = game
            .world
            .organism_registry()
            .get(founder.organism_id)
            .unwrap();
        record.validate_contract().unwrap();
        assert_eq!(record.genome().id, founder.genome_id);
        assert_eq!(record.phenotype().source_genome_id, founder.genome_id);
        assert!(record
            .phenotype()
            .chemistry
            .biochemical
            .emitters()
            .iter()
            .any(|emitter| emitter.source == alife_core::BiochemicalSourceLocus::HealthDeficit));
        assert_eq!(record.state_graph().organism_id, founder.organism_id);
        assert_eq!(record.embodiment().entity_id(), founder.world_entity_id);
        assert!(game
            .world
            .habitat_authority()
            .membership(founder.organism_id)
            .is_some());
    }
}

#[test]
fn canonical_new_game_contains_live_ecology_not_frontend_fixtures() {
    let game = phase3_game(6);
    let objects = game.world.object_snapshots();

    assert_eq!(
        objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Food)
            .count(),
        1
    );
    assert_eq!(
        objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Hazard)
            .count(),
        1
    );
    assert_eq!(
        objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Obstacle)
            .count(),
        2
    );
    assert_eq!(game.world.ecology().resources.len(), 1);
    assert!(!game.world.ecology().zones.is_empty());
}

#[test]
fn canonical_new_game_meadow_is_safe_until_actual_hazard_contact() {
    // Every supported founder slot starts clear of danger, on both the flat
    // fixture and the terrain used by ordinary New Game. Food has a clear route.
    for elevated in [false, true] {
        let mut nursery = phase3_game(8);
        if elevated {
            nursery.world.enable_highlands_for_new_game().unwrap();
        }
        let hazard_id = nursery.world.entity_id("hazard-01").unwrap();
        let hazard = nursery.world.entity(hazard_id).unwrap().position;
        let food_id = nursery.world.entity_id("food-01").unwrap();
        let food = nursery.world.entity(food_id).unwrap().position;
        for founder in &nursery.receipt.founders {
            let position = nursery
                .world
                .entity(founder.world_entity_id)
                .unwrap()
                .position;
            let separation = ((position.x - hazard.x).powi(2)
                + (position.y - hazard.y).powi(2)
                + (position.z - hazard.z).powi(2))
            .sqrt();
            assert!(separation > 6.0, "hazard encroaches on a founder spawn");
            if let Some(terrain) = nursery.world.terrain() {
                assert!(terrain.walkable(position.x, position.z));
                assert!(
                    terrain.resolve_move(position, food).is_some(),
                    "food route blocked"
                );
            }
            let idle = HeadlessWorldCommand::idle(founder.organism_id).unwrap();
            let receipt = nursery
                .world
                .apply_registered_command(&idle, founder.world_entity_id, Tick(1))
                .unwrap();
            assert_eq!(receipt.action_result.body_event.damage, 0.0);
            assert_eq!(receipt.biology_after.body.health, 1.0);
        }
    }

    let mut game = phase3_game(1);
    let founder = &game.receipt.founders[0];
    let hazard = game.world.entity_id("hazard-01").unwrap();
    let command = HeadlessWorldCommand::approach(founder.organism_id, hazard).unwrap();

    let meadow_step = game
        .world
        .apply_registered_command(&command, founder.world_entity_id, Tick(1))
        .unwrap();
    assert!(meadow_step.action_result.execution.succeeded);
    assert_eq!(
        meadow_step.action_result.execution.physical.contact,
        PhysicalContactKind::Moved
    );
    assert_eq!(meadow_step.action_result.body_event.damage, 0.0);
    assert_eq!(meadow_step.biology_after.body.health, 1.0);
    game.world.try_advance_tick().unwrap();

    let mut hazard_step = None;
    for tick in 2..=32 {
        let step = game
            .world
            .apply_registered_command(&command, founder.world_entity_id, Tick(tick))
            .unwrap();
        assert!(step.action_result.execution.succeeded);
        if step.action_result.body_event.damage > 0.0 {
            hazard_step = Some(step);
            break;
        }
        assert_eq!(step.biology_after.body.health, 1.0);
        game.world.try_advance_tick().unwrap();
    }
    let hazard_step = hazard_step.expect("the remote hazard remains reachable");
    assert!(hazard_step.action_result.execution.succeeded);
    assert_eq!(
        hazard_step.action_result.execution.physical.contact,
        PhysicalContactKind::Collision
    );
    assert_eq!(hazard_step.action_result.body_event.damage, 0.12);
    assert!(hazard_step.biology_after.body.health < hazard_step.biology_before.body.health);
    game.world.try_advance_tick().unwrap();

    // Contact has a real consequence, but it does not trap the creature there.
    let food = game.world.entity_id("food-01").unwrap();
    let retreat = HeadlessWorldCommand::approach(founder.organism_id, food).unwrap();
    for _ in 0..3 {
        let next_tick = Tick(game.world.tick().raw() + 1);
        let step = game
            .world
            .apply_registered_command(&retreat, founder.world_entity_id, next_tick)
            .unwrap();
        assert!(step.action_result.execution.succeeded);
        game.world.try_advance_tick().unwrap();
    }
    let idle = HeadlessWorldCommand::idle(founder.organism_id).unwrap();
    let next_tick = Tick(game.world.tick().raw() + 1);
    let recovered = game
        .world
        .apply_registered_command(&idle, founder.world_entity_id, next_tick)
        .unwrap();
    assert_eq!(recovered.action_result.body_event.damage, 0.0);
}

#[test]
fn canonical_new_game_is_deterministic_and_rejects_the_wrong_foundation() {
    let first = phase3_game(6);
    let second = phase3_game(6);
    assert_eq!(first.receipt, second.receipt);
    assert_eq!(first.creatures, second.creatures);
    assert_eq!(
        first.world.canonical_signature_digest().unwrap(),
        second.world.canonical_signature_digest().unwrap()
    );

    let privileged =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::PrivilegedAffordanceV1).unwrap();
    let config = CanonicalNewGameConfig::phase3(240_824, 6).unwrap();
    assert!(create_canonical_new_game(&config, &privileged).is_err());
}

#[test]
fn canonical_new_game_founders_compile_against_the_checked_nano512_foundation() {
    let game = phase3_game(6);
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();

    for founder in &game.receipt.founders {
        let record = game
            .world
            .organism_registry()
            .get(founder.organism_id)
            .unwrap();
        assert_eq!(
            record.phenotype().foundation,
            FoundationGeneticIdentity::new(
                0x004E_3531_325F_5631,
                1,
                0x4E35_3132_5F00_FA11,
                BrainCapacityClass::N512_ID,
            )
            .unwrap()
        );
        assert_eq!(
            record.phenotype().source_genome_id,
            record.phenotype().brain_genome.id
        );
        assert_eq!(
            record.phenotype().brain_genome.lineage_id,
            Some(record.phenotype().lineage_id)
        );
        record.phenotype().brain_genome.validate_contract().unwrap();
        record
            .phenotype()
            .development_state_at(alife_core::Tick::ZERO)
            .unwrap();
        let projection = N512FounderFoundationProjection::compile(
            record.phenotype(),
            SensorProfile::GroundedObjectSlotsV1,
            &foundation,
        )
        .unwrap();
        assert_eq!(projection.source_genome_id(), founder.genome_id);
        assert_eq!(projection.lineage_id(), founder.lineage_id);
    }
}
