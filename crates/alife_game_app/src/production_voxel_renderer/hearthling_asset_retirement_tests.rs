//! CPU/ECS regressions for the shared initial, restored and newborn presentation paths.
use super::*;
use alife_core::BrainScaleTier;
use alife_world::{AssetManifest, CreaturePartFamilyId, CreaturePartSources, RuntimeConfig};

fn staged() -> crate::StagedCanonicalNewGame {
    let mut config = RuntimeConfig::deterministic_default(240_824, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    crate::stage_phase3_new_game(crate::CanonicalNewGameLaunchRequest {
        world_seed: 240_824,
        population: 2,
        disable_age_death: false,
        save_path: std::env::temp_dir().join("alife-hearthling-retirement-save.json"),
        asset_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        config,
        assets: AssetManifest::empty(),
    })
    .unwrap()
}

fn context(profile: ProductionFrontendProfileId, population: u16) -> Fvr04CreatureSpawnContext {
    Fvr04CreatureSpawnContext {
        settings: Fvr04ProductionCreatureRendererSettings::for_profile(profile, population),
    }
}

fn records(save: &PortableSaveFile) -> Vec<Fvr04CreatureVisualRecord> {
    load_fvr04_runtime_state_from_save(
        save,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        ProductionFrontendProfileId::MinSpecComfort1080p,
        2,
    )
    .unwrap()
    .creatures
}

#[test]
fn new_game_and_restored_save_prepare_hearthling_roots_without_generated_parts() {
    let save = staged().save;
    let save_path = std::env::temp_dir().join(format!(
        "alife-hearthling-restored-{}.json",
        std::process::id()
    ));
    std::fs::write(&save_path, serde_json::to_vec(&save).unwrap()).unwrap();
    let restored = PortableSaveFile::from_json_file(&save_path).unwrap();
    std::fs::remove_file(save_path).unwrap();
    let expected = records(&save);
    let loaded = records(&restored);
    let context = context(ProductionFrontendProfileId::MinSpecComfort1080p, 2);
    let mut world = World::new();
    world.insert_resource(BevyEntityMap::default());
    // No mesh/image/material resources or AssetServer: preparation must use no retired pack.
    let batch = prepare_fvr04_creature_batch(&loaded, &BTreeMap::new(), &context).unwrap();
    for (prepared, expected) in batch.creatures.iter().zip(&expected) {
        assert_eq!(
            prepared.record.visual.appearance,
            expected.visual.appearance
        );
        assert_eq!(prepared.root_visual.local_bounds, hearthling::LOCAL_BOUNDS);
        assert_eq!(
            prepared.root_transform.scale,
            hearthling::scale(expected.visual.appearance)
        );
        assert_eq!(prepared.root_transform.rotation, Quat::IDENTITY);
        assert!((prepared.root_transform.translation.y - 0.48).abs() < 1e-5);
    }
    let scene = spawn_fvr04_prepared_creature_batch(&mut world, batch);
    assert_eq!(scene.creature_root_count, 2);
    assert_eq!(scene.creature_part_entity_count, 0);
    assert_eq!(scene.visual_profile, "approved-hearthling-skinned-v2");
    for record in loaded {
        let root = world
            .resource::<BevyEntityMap>()
            .bevy_entity(record.visual.stable_id)
            .unwrap();
        assert_eq!(
            world
                .get::<ProductionCreatureAssemblyRoot>(root)
                .unwrap()
                .organism_id,
            record.visual.organism_id
        );
    }
}

#[test]
fn high_and_mixed_family_save_fields_survive_every_presentation_lod() {
    let mut save = staged().save;
    let sources = CreaturePartSources {
        head: CreaturePartFamilyId(11),
        torso: CreaturePartFamilyId(9),
        arms: CreaturePartFamilyId(8),
        legs: CreaturePartFamilyId(10),
        tail: CreaturePartFamilyId(6),
    };
    save.creatures[0].appearance.part_sources = sources;
    save.creatures[1].appearance.part_sources =
        CreaturePartSources::coherent(CreaturePartFamilyId(11));
    let before = serde_json::to_vec(&save).unwrap();
    let restored: PortableSaveFile = serde_json::from_slice(&before).unwrap();
    let records = records(&restored);
    for (profile, population) in [
        (ProductionFrontendProfileId::MinSpecComfort1080p, 2),
        (ProductionFrontendProfileId::Balanced1080p, 2),
        (ProductionFrontendProfileId::ResearchScale, 250),
    ] {
        let batch =
            prepare_fvr04_creature_batch(&records, &BTreeMap::new(), &context(profile, population))
                .unwrap();
        assert_eq!(
            batch.creatures[0].record.visual.appearance.part_sources,
            sources
        );
        assert_eq!(
            batch.creatures[0].root_visual.local_bounds,
            hearthling::LOCAL_BOUNDS
        );
        let mut world = World::new();
        world.insert_resource(BevyEntityMap::default());
        let scene = spawn_fvr04_prepared_creature_batch(&mut world, batch);
        assert_eq!(scene.creature_part_family_count, 5);
        assert_eq!(scene.creature_mixed_assembly_count, 1);
    }
    assert_eq!(serde_json::to_vec(&restored).unwrap(), before);
}

#[test]
fn live_newborn_record_and_incremental_root_use_the_same_asset_free_preparation() {
    let staged = staged();
    let frame =
        LiveBrainPresentationFrame::from_authoritative_world(Vec::new(), &staged.world).unwrap();
    let mut newborns = frame
        .objects()
        .filter(|object| object.kind == WorldObjectKind::Agent)
        .map(|object| {
            let tile = VoxelTileCoord::new(
                object.position.x.floor() as i32,
                object.position.z.floor() as i32,
            );
            fvr04_live_creature_visual_record(
                &frame,
                240_824,
                object,
                tile,
                VoxelChunkCoord::for_tile(16, tile),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    newborns.sort_by_key(|record| record.visual.stable_id.raw());
    assert_eq!(newborns.len(), 2);
    let mut world = World::new();
    world.insert_resource(BevyEntityMap::default());
    let context = context(ProductionFrontendProfileId::Balanced1080p, 2);
    let first = prepare_fvr04_creature_batch(&newborns[..1], &BTreeMap::new(), &context).unwrap();
    let mut scene = spawn_fvr04_prepared_creature_batch(&mut world, first);
    let first_root = world
        .resource::<BevyEntityMap>()
        .bevy_entity(newborns[0].visual.stable_id)
        .unwrap();
    let first_transform = *world.get::<Transform>(first_root).unwrap();
    let added = prepare_fvr04_creature_batch(&newborns[1..], &BTreeMap::new(), &context).unwrap();
    let added = spawn_fvr04_prepared_creature_batch(&mut world, added);
    append_fvr04_creature_scene_resource(&mut scene, added);
    assert_eq!(scene.rendered_creature_count, 2);
    assert_eq!(
        scene.stable_lookup_by_raw_id[&newborns[1].visual.stable_id.raw()],
        1
    );
    assert_eq!(
        *world.get::<Transform>(first_root).unwrap(),
        first_transform
    );
    let child_root = world
        .resource::<BevyEntityMap>()
        .bevy_entity(newborns[1].visual.stable_id)
        .unwrap();
    let marker = world
        .get::<Fvr04ProductionCreatureVisualMarker>(child_root)
        .unwrap();
    assert_eq!(
        marker.base_scale,
        hearthling::scale(newborns[1].visual.appearance)
    );
    assert_eq!(marker.local_bounds, hearthling::LOCAL_BOUNDS);
    assert_eq!(
        world
            .query::<&ProductionCreaturePartMarker>()
            .iter(&world)
            .count(),
        0
    );
}
