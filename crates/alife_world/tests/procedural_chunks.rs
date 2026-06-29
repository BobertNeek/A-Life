use alife_core::{Vec3f, WorldEntityId};
use alife_world::{
    activate_procedural_chunks_around_creatures, procedural_chunk_summary,
    sample_creature_procedural_neighborhood, sample_procedural_terrain_tile, CreatureWorldAnchor,
    ProceduralChunkCoord, ProceduralTerrainMaterial, ProceduralTileCoord, ProceduralWorldConfig,
    DEFAULT_ACTIVATION_RADIUS_CHUNKS, PROCEDURAL_WORLD_CHUNKS_SCHEMA,
};

fn anchor(id: u64, x: f32, z: f32) -> CreatureWorldAnchor {
    CreatureWorldAnchor::new(WorldEntityId(id), Vec3f::new(x, 0.0, z)).unwrap()
}

#[test]
fn procedural_chunks_are_deterministic_for_seed_and_tile() {
    let config = ProceduralWorldConfig::with_seed(4242);
    let tile = ProceduralTileCoord::new(17, -23);

    let first = sample_procedural_terrain_tile(config, tile).unwrap();
    let second = sample_procedural_terrain_tile(config, tile).unwrap();
    assert_eq!(first, second);
    assert!(first.resource_bias.is_finite());
    assert!(first.hazard_pressure.is_finite());
    assert!((0.0..=1.0).contains(&first.traversal_cost));
}

#[test]
fn procedural_chunks_activate_around_creatures_without_rendering() {
    let config = ProceduralWorldConfig::with_seed(4242);
    let anchors = [anchor(1, 0.0, 0.0), anchor(6, 34.0, -18.0)];

    let report = activate_procedural_chunks_around_creatures(config, &anchors).unwrap();

    assert_eq!(report.schema, PROCEDURAL_WORLD_CHUNKS_SCHEMA);
    assert_eq!(report.creature_anchor_count, 2);
    assert!(report.generated_without_rendering);
    assert!(!report.rendering_required);
    assert!(report.active_chunks.iter().any(|chunk| {
        chunk.anchor_stable_id == WorldEntityId(1)
            && chunk.anchor_tile == ProceduralTileCoord::new(0, 0)
    }));
    assert!(report.active_chunks.iter().any(|chunk| {
        chunk.anchor_stable_id == WorldEntityId(6)
            && chunk.anchor_tile == ProceduralTileCoord::new(34, -18)
    }));
    let per_creature_square = (DEFAULT_ACTIVATION_RADIUS_CHUNKS * 2 + 1).pow(2) as usize;
    assert!(report.active_chunks.len() <= per_creature_square * anchors.len());
    report.validate(config).unwrap();
}

#[test]
fn procedural_chunks_do_not_exist_without_creature_anchors() {
    let config = ProceduralWorldConfig::with_seed(4242);

    let report = activate_procedural_chunks_around_creatures(config, &[]).unwrap();

    assert_eq!(report.creature_anchor_count, 0);
    assert!(report.active_chunks.is_empty());
    assert_eq!(report.skipped_due_to_cap, 0);
    assert!(report.generated_without_rendering);
}

#[test]
fn procedural_chunk_activation_respects_capacity() {
    let config = ProceduralWorldConfig {
        max_active_chunks: 4,
        ..ProceduralWorldConfig::with_seed(4242)
    };

    let report =
        activate_procedural_chunks_around_creatures(config, &[anchor(1, 0.0, 0.0)]).unwrap();

    assert_eq!(report.active_chunks.len(), 4);
    assert!(report.skipped_due_to_cap > 0);
    report.validate(config).unwrap();
}

#[test]
fn procedural_chunk_summary_reports_bounded_material_distribution() {
    let config = ProceduralWorldConfig::with_seed(4242);

    let summary = procedural_chunk_summary(config, ProceduralChunkCoord::new(0, 0)).unwrap();

    summary.validate(config).unwrap();
    assert_eq!(
        summary.tile_count,
        (config.chunk_tile_size * config.chunk_tile_size) as usize
    );
    assert!(summary.material_counts.iter().any(|entry| entry.count > 0));
    assert!((0.0..=1.0).contains(&summary.average_resource_bias));
    assert!((0.0..=1.0).contains(&summary.average_hazard_pressure));
}

#[test]
fn procedural_creature_neighborhood_is_bounded_context_not_action_authority() {
    let config = ProceduralWorldConfig::with_seed(4242);

    let neighborhood =
        sample_creature_procedural_neighborhood(config, anchor(1, 0.0, 0.0)).unwrap();

    neighborhood.validate(config).unwrap();
    assert_eq!(neighborhood.stable_id, WorldEntityId(1));
    assert!(neighborhood.sample_count <= config.max_neighborhood_samples);
    assert!(neighborhood.bounded_for_sensory);
    assert!(!neighborhood.can_emit_actions);
    assert!(!neighborhood.can_rewrite_weights);
}

#[test]
fn procedural_material_ids_match_alpha_art_roles() {
    assert_eq!(
        ProceduralTerrainMaterial::SafeGrass.material_id(),
        "safe-grass"
    );
    assert_eq!(
        ProceduralTerrainMaterial::NeutralSoil.material_id(),
        "neutral-soil"
    );
    assert_eq!(
        ProceduralTerrainMaterial::ResourceGrove.material_id(),
        "resource-grove"
    );
    assert_eq!(
        ProceduralTerrainMaterial::HazardPressure.material_id(),
        "hazard-pressure"
    );
    assert_eq!(
        ProceduralTerrainMaterial::StoneRough.material_id(),
        "stone-dressing"
    );
}
