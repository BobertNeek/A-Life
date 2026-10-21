use alife_core::{Vec3f, WorldEntityId};
use alife_world::{
    activate_procedural_chunks_around_creatures, generate_procedural_world_content,
    procedural_chunk_summary, sample_procedural_terrain_tile, CreatureWorldAnchor,
    ProceduralChunkCoord, ProceduralTerrainMaterial, ProceduralTileCoord, ProceduralWorldConfig,
    ProceduralWorldContentKind, DEFAULT_ACTIVATION_RADIUS_CHUNKS, PROCEDURAL_CONTENT_ID_BASE,
    PROCEDURAL_WORLD_CHUNKS_SCHEMA, PROCEDURAL_WORLD_CONTENT_SCHEMA,
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
fn procedural_chunks_follow_creature_travel_from_same_seed() {
    let config = ProceduralWorldConfig::with_seed(4242);

    let origin_report =
        activate_procedural_chunks_around_creatures(config, &[anchor(1, 0.0, 0.0)]).unwrap();
    let traveled_report =
        activate_procedural_chunks_around_creatures(config, &[anchor(1, 96.0, -64.0)]).unwrap();
    let repeated_traveled_report =
        activate_procedural_chunks_around_creatures(config, &[anchor(1, 96.0, -64.0)]).unwrap();

    assert_eq!(traveled_report, repeated_traveled_report);
    assert_eq!(origin_report.seed, traveled_report.seed);
    assert_ne!(
        origin_report.active_chunks, traveled_report.active_chunks,
        "creature travel should materialize a new deterministic active chunk window instead of reusing a single screen"
    );
    assert!(origin_report
        .active_chunks
        .iter()
        .all(|chunk| chunk.anchor_stable_id == WorldEntityId(1)));
    assert!(traveled_report
        .active_chunks
        .iter()
        .all(|chunk| chunk.anchor_stable_id == WorldEntityId(1)));
    assert!(traveled_report.active_chunks.iter().any(|chunk| {
        chunk.anchor_tile == ProceduralTileCoord::new(96, -64)
            && chunk.anchor_stable_id == WorldEntityId(1)
    }));
    origin_report.validate(config).unwrap();
    traveled_report.validate(config).unwrap();
}

#[test]
fn procedural_chunks_do_not_exist_without_creature_anchors() {
    let config = ProceduralWorldConfig::with_seed(4242);

    let report = activate_procedural_chunks_around_creatures(config, &[]).unwrap();

    assert_eq!(report.creature_anchor_count, 0);
    assert!(report.active_chunks.is_empty());
    assert_eq!(report.skipped_due_to_cap, 0);
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
    assert_eq!(ProceduralTerrainMaterial::Water.material_id(), "water");
    assert_eq!(ProceduralTerrainMaterial::Sand.material_id(), "sand");
}

#[test]
fn procedural_world_content_is_deterministic_and_creature_anchored() {
    let config = ProceduralWorldConfig::with_seed(4242);
    let anchors = [anchor(1, 0.0, 0.0), anchor(6, 34.0, -18.0)];
    let activation = activate_procedural_chunks_around_creatures(config, &anchors).unwrap();

    let first = generate_procedural_world_content(config, &activation).unwrap();
    let second = generate_procedural_world_content(config, &activation).unwrap();

    first.validate(config).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.schema, PROCEDURAL_WORLD_CONTENT_SCHEMA);
    assert!(first.candidate_count > 0);
    assert!(first
        .candidates
        .iter()
        .all(|candidate| candidate.stable_id.raw() >= PROCEDURAL_CONTENT_ID_BASE));
    assert!(first.candidates.iter().all(|candidate| {
        anchors
            .iter()
            .any(|anchor| anchor.stable_id == candidate.anchor_stable_id)
    }));
}

#[test]
fn procedural_world_content_includes_readable_ecology_roles() {
    let config = ProceduralWorldConfig::with_seed(4242);
    let activation =
        activate_procedural_chunks_around_creatures(config, &[anchor(1, 0.0, 0.0)]).unwrap();

    let report = generate_procedural_world_content(config, &activation).unwrap();

    assert!(report.count_kind(ProceduralWorldContentKind::Food) > 0);
    assert!(report.count_kind(ProceduralWorldContentKind::Hazard) > 0);
    assert!(report.count_kind(ProceduralWorldContentKind::Obstacle) > 0);
    assert!(report.count_kind(ProceduralWorldContentKind::DressingProp) > 0);
    assert!(report
        .candidates
        .iter()
        .all(|candidate| { candidate.alpha_art_role == candidate.kind.alpha_art_role() }));
}

#[test]
fn procedural_world_content_does_not_exist_without_active_creature_chunks() {
    let config = ProceduralWorldConfig::with_seed(4242);
    let activation = activate_procedural_chunks_around_creatures(config, &[]).unwrap();

    let report = generate_procedural_world_content(config, &activation).unwrap();

    assert_eq!(report.candidate_count, 0);
    assert!(report.candidates.is_empty());
}

#[test]
fn procedural_world_content_respects_capacity() {
    let config = ProceduralWorldConfig {
        max_active_content_candidates: 5,
        ..ProceduralWorldConfig::with_seed(4242)
    };
    let activation =
        activate_procedural_chunks_around_creatures(config, &[anchor(1, 0.0, 0.0)]).unwrap();

    let report = generate_procedural_world_content(config, &activation).unwrap();

    assert_eq!(report.candidate_count, 5);
    assert!(report.skipped_due_to_cap > 0);
    report.validate(config).unwrap();
}

#[test]
fn valid_far_virtual_tiles_keep_biome_distance_arithmetic_in_range() {
    let config = ProceduralWorldConfig {
        chunk_tile_size: 64,
        virtual_half_extent_chunks: 8192,
        ..ProceduralWorldConfig::with_seed(4242)
    };
    for tile in [
        ProceduralTileCoord::new(200_000, -200_000),
        ProceduralTileCoord::new(-500_000, 500_000),
    ] {
        let sample = sample_procedural_terrain_tile(config, tile).unwrap();
        assert_eq!(sample.tile, tile);
        sample.validate().unwrap();
    }
}
