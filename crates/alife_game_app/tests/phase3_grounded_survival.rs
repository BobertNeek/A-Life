#![cfg(feature = "gpu-tests")]

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::{
    ActionId, ActionKind, ActionTarget, BodyEventDelta, BrainScaleTier, ChannelCommand, Confidence,
    DurationTicks, ExperienceSequenceId, Intensity, MotorChannel, MotorCommandBundle, OrganKind,
    OrganismId, Tick, Validate, Vec3f, WorldEntityId,
};
use alife_game_app::{
    create_canonical_new_game_runtime, stage_phase3_new_game, CanonicalNewGameLaunchRequest,
    LiveBrainCausalStage,
};
use alife_world::{AssetManifest, HeadlessActionIds, RuntimeConfig, WorldObjectKind};

#[path = "../src/factorized_arbitration.rs"]
mod factorized_arbitration;
use factorized_arbitration::{
    arbitrate_gpu_selected_command_into_factorized_bundle, channel_command_for_action,
    factorized_motor_channel_order,
};

const MAX_SLEEP_TO_ORDINARY_TICKS: usize = 384;
static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

fn new_game_request(label: &str) -> CanonicalNewGameLaunchRequest {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must follow the Unix epoch")
        .as_nanos();
    let ordinal = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "alife-phase3-grounded-{label}-{}-{nonce}-{ordinal}",
        std::process::id()
    ));
    let mut config = RuntimeConfig::deterministic_default(240_825, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    std::fs::create_dir_all(root.join("assets"))
        .expect("isolated canonical New Game asset root must be creatable");
    CanonicalNewGameLaunchRequest {
        world_seed: 240_825,
        population: 4,
        save_path: root.join("phase3-save.json"),
        asset_root: root.join("assets"),
        config,
        assets: AssetManifest::empty(),
    }
}

fn planar_distance(left: Vec3f, right: Vec3f) -> f32 {
    (left.x - right.x).hypot(left.y - right.y)
}

#[test]
fn gpu_selected_action_arbitrates_only_its_factorized_channel_deterministically() {
    let organism_id = OrganismId(41);
    let sequence_id = ExperienceSequenceId::new(7).unwrap();
    let tick = Tick(12);
    let command = |id, kind, target| {
        alife_core::ActionCommand::structured(
            organism_id,
            ActionId(id),
            kind,
            target,
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(0.8).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap()
    };
    let locomotion = channel_command_for_action(
        MotorChannel::Locomotion,
        &command(
            901,
            ActionKind::Move,
            ActionTarget::new(None, Some(Vec3f::new(1.0, 0.0, 0.0))),
        ),
    )
    .unwrap();
    let displaced_manipulation = channel_command_for_action(
        MotorChannel::Manipulation,
        &command(
            902,
            ActionKind::Interact,
            ActionTarget::new(Some(WorldEntityId(91)), None),
        ),
    )
    .unwrap();
    let vocal = channel_command_for_action(
        MotorChannel::Vocal,
        &command(903, ActionKind::Vocalize, ActionTarget::NONE),
    )
    .unwrap();
    let selected = command(
        904,
        ActionKind::Interact,
        ActionTarget::new(Some(WorldEntityId(92)), None),
    );
    let heads = vec![vocal.clone(), displaced_manipulation, locomotion.clone()];

    let first = arbitrate_gpu_selected_command_into_factorized_bundle(
        organism_id,
        sequence_id,
        tick,
        heads.clone(),
        &selected,
        None,
        false,
    )
    .unwrap();
    let second = arbitrate_gpu_selected_command_into_factorized_bundle(
        organism_id,
        sequence_id,
        tick,
        heads,
        &selected,
        None,
        false,
    )
    .unwrap();

    first.validate_contract().unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.canonical_digest().unwrap(),
        second.canonical_digest().unwrap()
    );
    let selected_channels = first
        .channels
        .iter()
        .filter(|channel| channel.primitive == selected.action_id)
        .collect::<Vec<_>>();
    assert_eq!(selected_channels.len(), 1);
    assert_eq!(selected_channels[0].channel, MotorChannel::Manipulation);
    assert_eq!(
        selected_channels[0].target,
        Some(ActionTarget::new(Some(WorldEntityId(92)), None))
    );
    assert_eq!(
        first
            .channels
            .iter()
            .find(|channel| channel.channel == MotorChannel::Locomotion),
        Some(&locomotion)
    );
    assert_eq!(
        first
            .channels
            .iter()
            .find(|channel| channel.channel == MotorChannel::Vocal),
        Some(&vocal)
    );
    assert!(first.channels.windows(2).all(|pair| {
        factorized_motor_channel_order(pair[0].channel)
            < factorized_motor_channel_order(pair[1].channel)
    }));
    assert_eq!(first.coordination.groups.len(), 1);
    assert_eq!(
        first.coordination.groups[0].channels,
        first
            .channels
            .iter()
            .map(|channel| channel.channel)
            .collect::<Vec<_>>()
    );
    assert!(first
        .channels
        .iter()
        .any(|channel| channel.primitive == selected.action_id));
}

#[test]
fn canonical_contact_resource_depletes_through_factorized_registered_transaction() {
    let staged = stage_phase3_new_game(new_game_request("resource"))
        .expect("canonical New Game world must stage");
    let mut world = staged.world;
    let founder = staged.receipt.founders[0].clone();
    let food = world.entity_id("food-01").expect("one canonical resource");
    let food_before = world.entity(food).expect("world-owned resource").clone();
    let agent_before = world
        .entity(founder.world_entity_id)
        .expect("canonical founder")
        .clone();
    assert_eq!(
        world
            .object_snapshots()
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Food)
            .count(),
        1
    );
    assert!(planar_distance(agent_before.position, food_before.position) <= food_before.radius);

    let biology_before = *world
        .organism_registry()
        .get(founder.organism_id)
        .expect("canonical founder biology")
        .biochemistry();
    let phenotype = world
        .organism_registry()
        .get(founder.organism_id)
        .expect("canonical founder phenotype")
        .phenotype()
        .clone();
    let intent = ChannelCommand::new(
        MotorChannel::Manipulation,
        HeadlessActionIds::EAT,
        Some(ActionTarget::new(Some(food), None)),
        Vec3f::ZERO,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(1.0).unwrap(),
        0,
    )
    .unwrap();
    let bundle = MotorCommandBundle::new(
        founder.organism_id,
        ExperienceSequenceId::new(1).unwrap(),
        Tick::ZERO,
        vec![intent],
    )
    .unwrap();

    let receipt = world
        .apply_registered_motor_bundle(&bundle, founder.world_entity_id)
        .expect("deterministic factorized ingest must use the registered world/body transaction");

    assert!(receipt.succeeded);
    assert!(
        world
            .entity(food)
            .expect("depleted resource remains world-owned")
            .consumed
    );
    assert!(receipt.body_event.nutrition > 0.0);
    assert!(receipt.biology_after.body.energy > biology_before.body.energy);
    let without_nutrition = biology_before
        .advance(
            Tick(1),
            BodyEventDelta {
                nutrition: 0.0,
                ..receipt.body_event
            },
            &phenotype,
        )
        .unwrap();
    assert_ne!(
        receipt.biology_after.body.organ(OrganKind::Digestive),
        without_nutrition.body.organ(OrganKind::Digestive),
        "ingested nutrition must reach phenotype-routed digestive physiology"
    );
    assert_ne!(
        receipt
            .biology_after
            .body
            .organ(OrganKind::MetabolicReserve),
        without_nutrition.body.organ(OrganKind::MetabolicReserve),
        "ingested nutrition must reach phenotype-routed metabolic reserve physiology"
    );
}

#[test]
fn ordinary_gpu_tick_reaches_registered_transaction_and_applies_contact_hazard() {
    let mut launched = create_canonical_new_game_runtime(new_game_request("hazard"))
        .expect("canonical New Game must launch on the required production GPU");
    let initial = launched.runtime.world_snapshot();
    let resource_founder = launched.receipt.founders[0].clone();
    let hazard_founder = launched.receipt.founders[1].clone();
    let food = initial
        .entity_id("food-01")
        .expect("one canonical resource");
    let hazard = initial
        .entity_id("hazard-01")
        .expect("one canonical hazard");
    assert_eq!(
        initial
            .object_snapshots()
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Hazard)
            .count(),
        1
    );
    assert!(
        planar_distance(
            initial
                .entity(resource_founder.world_entity_id)
                .unwrap()
                .position,
            initial.entity(food).unwrap().position,
        ) <= initial.entity(food).unwrap().radius
    );
    assert!(
        planar_distance(
            initial
                .entity(hazard_founder.world_entity_id)
                .unwrap()
                .position,
            initial.entity(hazard).unwrap().position,
        ) <= initial.entity(hazard).unwrap().radius
    );

    for _ in 0..MAX_SLEEP_TO_ORDINARY_TICKS {
        let before = launched.runtime.world_snapshot();
        let summaries = launched
            .runtime
            .tick()
            .expect("ordinary production GPU tick");
        let Some(hazard_summary) = summaries.iter().find(|summary| {
            summary.organism_id == hazard_founder.organism_id
                && summary
                    .causal_stages
                    .contains(&LiveBrainCausalStage::ExecuteAction)
        }) else {
            continue;
        };
        let resource_summary = summaries
            .iter()
            .find(|summary| summary.organism_id == resource_founder.organism_id)
            .expect("resource-contacted founder must share the ordinary GPU batch");
        assert!(resource_summary
            .causal_stages
            .contains(&LiveBrainCausalStage::ExecuteAction));
        assert!(resource_summary.patch_sealed);
        assert!(hazard_summary.patch_sealed);
        let after = launched.runtime.world_snapshot();
        let biology_before = before
            .organism_registry()
            .get(hazard_founder.organism_id)
            .expect("hazard-contacted founder before tick")
            .biochemistry();
        let biology_after = after
            .organism_registry()
            .get(hazard_founder.organism_id)
            .expect("hazard-contacted founder after tick")
            .biochemistry();
        assert!(biology_after.body.health < biology_before.body.health);
        assert!(biology_after.body.injury > biology_before.body.injury);
        assert!(biology_after.homeostasis.drives.pain > biology_before.homeostasis.drives.pain);
        assert!(
            biology_after.homeostasis.hormones.cortisol
                > biology_before.homeostasis.hormones.cortisol
        );
        assert!(after
            .organism_registry()
            .get(hazard_founder.organism_id)
            .expect("contacted founder remains registered")
            .lifecycle()
            .is_alive());
        return;
    }

    panic!("canonical newborns did not reach one ordinary production GPU action transaction");
}
