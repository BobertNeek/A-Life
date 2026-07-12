use alife_core::{
    BrainScaleTier, GenomeId, HomeostaticSnapshot, OrganismId, Tick, Vec3f, WorldEntityId,
};
use alife_world::{
    AssetManifest, CreatureMindSaveSummary, CreatureSaveState, CreatureWorldAnchor,
    LearningTraceSaveSummary, PersistenceError, PersistentVoxelProfileId,
    PersistentVoxelWorldBackend, PortableSaveFile, RuntimeConfig, VoxelBiomeId, VoxelChunkCoord,
    VoxelTerrainMaterialId, VoxelTileCoord, VoxelTileEdit, WeightLayerSaveSummary,
    FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA,
};

fn anchor(id: u64, x: f32, z: f32) -> CreatureWorldAnchor {
    CreatureWorldAnchor::new(WorldEntityId(id), Vec3f::new(x, 0.0, z)).unwrap()
}

fn fixture_world() -> alife_world::HeadlessWorld {
    alife_world::HeadlessScenarioBuilder::new(4242)
        .agent("agent", OrganismId(1), Vec3f::ZERO)
        .food("berry", Vec3f::new(1.0, 0.0, 0.0), 0.75)
        .hazard("thorn", Vec3f::new(3.0, 0.0, 0.0), 0.4)
        .build()
        .unwrap()
}

fn fixture_creature() -> CreatureSaveState {
    CreatureSaveState {
        organism_id: OrganismId(1),
        genome_id: GenomeId(17),
        brain_class: BrainScaleTier::Nano512,
        development_tick: Tick::new(3),
        appearance: alife_world::CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick: Tick::new(3),
            homeostasis: HomeostaticSnapshot::baseline(Tick::new(3)),
            memory_record_count: 1,
            memory_source_ids: Vec::new(),
            concept_count: 1,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".to_string(),
            diagnostics: vec!["fvr02".to_string()],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest: "fnv1a64:0000000000000001".to_string(),
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: 0,
            h_operational_entries: 0,
            h_shadow_entries: 0,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: true,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: Some(Tick::new(2)),
        },
    }
}

#[test]
fn fvr02_persistent_chunks_roundtrip_saved_edits_and_materialized_metadata() {
    let anchors = [anchor(1, 0.0, 0.0), anchor(6, 34.0, -18.0)];
    let mut backend =
        PersistentVoxelWorldBackend::new(4242, PersistentVoxelProfileId::MinimumSettings30x30)
            .unwrap();

    let initial = backend.snapshot_for_anchors(&anchors).unwrap();
    assert_eq!(initial.schema, FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA);
    assert_eq!(
        initial.profile_id,
        PersistentVoxelProfileId::MinimumSettings30x30
    );
    assert!(!initial.visible_chunks.is_empty());
    assert!(initial.visible_chunks.len() <= initial.profile_budget.active_chunk_cap as usize);
    assert_eq!(initial.creatures.len(), anchors.len());
    assert!(initial
        .resources_and_hazards
        .iter()
        .any(|entry| entry.is_resource()));
    assert!(initial
        .resources_and_hazards
        .iter()
        .any(|entry| entry.is_hazard()));

    let edit = VoxelTileEdit {
        tile: VoxelTileCoord::new(2, -3),
        material: VoxelTerrainMaterialId::CultivatedResource,
        biome: VoxelBiomeId::ResourceGrove,
        elevation_delta: 2,
        resource_bias_override: Some(0.95),
        hazard_pressure_override: Some(0.0),
        author_stable_id: Some(WorldEntityId(1)),
        reason: "fvr02-test-cultivated-resource".to_string(),
    };
    let edited_chunk = VoxelChunkCoord::for_tile(initial.profile_budget.chunk_tile_size, edit.tile);
    backend.apply_tile_edit(edit.clone()).unwrap();
    let edited_signature = backend.chunk_signature(edited_chunk).unwrap();

    let save_state = backend.to_save_state().unwrap();
    assert_eq!(save_state.schema, FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA);
    assert_eq!(save_state.world_seed, 4242);
    assert_eq!(save_state.saved_edits, vec![edit.clone()]);
    assert!(save_state.generator.output_digest.0.starts_with("fnv1a64:"));
    assert!(save_state
        .materialized_chunks
        .iter()
        .any(|chunk| chunk.coord == edited_chunk && chunk.saved_edit_count == 1));
    assert!(
        save_state.materialized_chunk_count <= save_state.profile_budget.active_chunk_cap as usize
    );

    let json = serde_json::to_string_pretty(&save_state).unwrap();
    assert!(!json.contains("Bevy"));
    assert!(!json.contains("wgpu"));
    assert!(!json.contains("Entity("));
    let restored_state = serde_json::from_str(&json).unwrap();
    let restored = PersistentVoxelWorldBackend::from_save_state(restored_state).unwrap();
    let restored_snapshot = restored.snapshot_for_anchors(&anchors).unwrap();
    assert_eq!(
        restored.chunk_signature(edited_chunk).unwrap(),
        edited_signature
    );
    assert!(restored_snapshot
        .dirty_regions
        .iter()
        .any(|region| region.chunk == edited_chunk));
    assert_eq!(
        restored_snapshot.lookup_tile(edit.tile).unwrap().tile,
        Some(edit.tile)
    );
}

#[test]
fn fvr02_snapshot_streams_are_compact_dirty_and_renderer_independent() {
    let anchors = [anchor(1, 0.0, 0.0)];
    let mut backend =
        PersistentVoxelWorldBackend::new(4242, PersistentVoxelProfileId::MinSpecComfort1080p)
            .unwrap();
    let before = backend.snapshot_for_anchors(&anchors).unwrap();

    backend
        .apply_tile_edit(VoxelTileEdit {
            tile: VoxelTileCoord::new(3, 3),
            material: VoxelTerrainMaterialId::HazardCrystal,
            biome: VoxelBiomeId::HazardPressure,
            elevation_delta: 1,
            resource_bias_override: Some(0.0),
            hazard_pressure_override: Some(0.9),
            author_stable_id: Some(WorldEntityId(1)),
            reason: "fvr02-test-hazard".to_string(),
        })
        .unwrap();
    let after = backend.snapshot_for_anchors(&anchors).unwrap();

    assert!(after.visible_chunks.len() <= after.profile_budget.active_chunk_cap as usize);
    assert!(after.visible_chunks.len() < after.virtual_chunk_count as usize);
    assert!(after.dirty_regions.len() <= after.visible_chunks.len());
    assert!(after.chunk_signature_digest != before.chunk_signature_digest);
    assert!(after
        .field_overlays
        .iter()
        .any(|overlay| overlay.kind.is_resource()));
    assert!(after
        .field_overlays
        .iter()
        .any(|overlay| overlay.kind.is_hazard()));
    assert!(after
        .selection_refs
        .iter()
        .all(|reference| reference.is_stable()));

    let json = serde_json::to_string(&after).unwrap();
    for forbidden in ["bevy", "wgpu", "renderer", "windowhandle", "Entity("] {
        assert!(
            !json
                .to_ascii_lowercase()
                .contains(&forbidden.to_ascii_lowercase()),
            "snapshot leaked renderer token {forbidden}: {json}"
        );
    }
}

#[test]
fn fvr02_profile_residency_budgets_scale_without_minimum_ceiling() {
    let minimum = PersistentVoxelProfileId::MinimumSettings30x30.budget();
    assert_eq!(minimum.chunk_tile_size, 16);
    assert_eq!(minimum.activation_radius_chunks, 2);
    assert_eq!(minimum.active_chunk_cap, 128);
    assert_eq!(minimum.hot_brain_slots, 4);
    assert_eq!(minimum.warm_brain_slots, 12);
    assert_eq!(minimum.cold_brain_slots, 14);

    let comfort = PersistentVoxelProfileId::MinSpecComfort1080p.budget();
    assert_eq!(comfort.activation_radius_chunks, 4);
    assert_eq!(comfort.active_chunk_cap, 256);
    assert!(comfort.active_chunk_cap > minimum.active_chunk_cap);

    let high = PersistentVoxelProfileId::HighSpecScaleUp.budget();
    let research = PersistentVoxelProfileId::ResearchScale.budget();
    assert!(high.active_chunk_cap > comfort.active_chunk_cap);
    assert!(research.active_chunk_cap >= high.active_chunk_cap);
    assert!(research.virtual_half_extent_chunks > minimum.activation_radius_chunks as i32);

    let backend =
        PersistentVoxelWorldBackend::new(4242, PersistentVoxelProfileId::MinimumSettings30x30)
            .unwrap();
    assert!(backend.virtual_chunk_count() > minimum.active_chunk_cap as u64);
    assert!(!backend.allocates_far_chunks());
}

#[test]
fn fvr02_portable_save_migrates_legacy_p34_and_preserves_voxel_roundtrip() {
    let world = fixture_world();
    let save = PortableSaveFile::from_headless_world(
        "fvr02-save",
        &world,
        RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![fixture_creature()],
    )
    .unwrap();
    assert!(save.world.voxel_backend.is_some());
    save.validate_with_asset_root(std::env::temp_dir()).unwrap();

    let loaded = PortableSaveFile::from_json_str(&save.to_json_string_pretty().unwrap()).unwrap();
    let before = save
        .world
        .voxel_backend
        .as_ref()
        .unwrap()
        .visible_chunk_signatures();
    let after = loaded
        .world
        .voxel_backend
        .as_ref()
        .unwrap()
        .visible_chunk_signatures();
    assert_eq!(before, after);

    let mut legacy_json = serde_json::to_value(&save).unwrap();
    legacy_json
        .get_mut("world")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("voxel_backend");
    let legacy = PortableSaveFile::from_json_str(&legacy_json.to_string()).unwrap();
    assert!(legacy.world.voxel_backend.is_none());

    let migrated = legacy
        .with_migrated_voxel_backend(PersistentVoxelProfileId::MinimumSettings30x30)
        .unwrap();
    assert!(migrated.world.voxel_backend.is_some());
    assert!(matches!(
        legacy.require_voxel_backend(),
        Err(PersistenceError::MigrationUnsupported { .. })
    ));
}
