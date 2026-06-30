use alife_core::{Vec3f, WorldEntityId};
use alife_world::{
    activate_procedural_chunks_around_creatures, generate_procedural_world_content,
    procedural_chunk_summary, procedural_world_scale_report,
    sample_creature_procedural_content_neighborhood, sample_creature_procedural_neighborhood,
    sample_procedural_terrain_tile, CreatureWorldAnchor, ProceduralChunkCoord,
    ProceduralTerrainMaterial, ProceduralTileCoord, ProceduralWorldConfig,
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
    assert!(origin_report.generated_without_rendering);
    assert!(traveled_report.generated_without_rendering);
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
    let scale = procedural_world_scale_report(config, &report, 0).unwrap();

    assert_eq!(report.creature_anchor_count, 0);
    assert!(report.active_chunks.is_empty());
    assert_eq!(report.skipped_due_to_cap, 0);
    assert!(report.generated_without_rendering);
    assert_eq!(scale.creature_anchor_count, 0);
    assert_eq!(scale.active_chunk_count, 0);
    assert_eq!(scale.materialized_chunk_count, 0);
    assert!(!scale.chunks_exist_without_creature_anchors);
    assert!(scale.generated_without_rendering);
    assert!(!scale.rendering_required);
    assert!(!scale.can_emit_actions);
    assert!(!scale.can_rewrite_weights);
    scale.validate(config).unwrap();
}

#[test]
fn procedural_world_scale_is_large_virtual_and_creature_anchored() {
    let config = ProceduralWorldConfig::with_seed(4242);
    let anchors = [anchor(1, 0.0, 0.0), anchor(6, 34.0, -18.0)];
    let activation = activate_procedural_chunks_around_creatures(config, &anchors).unwrap();

    let report = procedural_world_scale_report(config, &activation, anchors.len()).unwrap();

    report.validate(config).unwrap();
    assert!(report.virtual_width_tiles >= 4096);
    assert!(report.virtual_height_tiles >= 4096);
    assert!(report.virtual_tile_count > 16_000_000);
    assert!(report.potential_chunk_count > 60_000);
    assert_eq!(report.creature_anchor_count, anchors.len());
    assert!(report.active_chunk_count > 0);
    assert!(report.active_chunk_count <= config.max_active_chunks);
    assert!(report.materialized_chunk_count <= report.active_chunk_count);
    assert!(report.active_fraction_of_virtual_world < 0.001);
    assert!(report.generated_without_rendering);
    assert!(!report.rendering_required);
    assert!(!report.chunks_exist_without_creature_anchors);
    assert!(report.bounded_active_chunk_window);
    assert!(report.materialized_only_near_creature_anchors);
    assert!(report.bounded_for_creature_context);
    assert!(!report.can_emit_actions);
    assert!(!report.can_rewrite_weights);
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
    assert!(first.generated_without_rendering);
    assert!(!first.rendering_required);
    assert!(first.bounded_for_creature_context);
    assert!(!first.can_emit_actions);
    assert!(!first.can_rewrite_weights);
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
    assert!(report.candidates.iter().all(|candidate| {
        candidate.alpha_art_role == candidate.kind.alpha_art_role()
            && !candidate.can_emit_actions
            && !candidate.can_rewrite_weights
            && !candidate.rendering_required
    }));
}

#[test]
fn procedural_world_content_does_not_exist_without_active_creature_chunks() {
    let config = ProceduralWorldConfig::with_seed(4242);
    let activation = activate_procedural_chunks_around_creatures(config, &[]).unwrap();

    let report = generate_procedural_world_content(config, &activation).unwrap();

    assert_eq!(report.candidate_count, 0);
    assert!(report.candidates.is_empty());
    assert!(report.generated_without_rendering);
    assert!(!report.rendering_required);
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
fn procedural_creature_content_neighborhood_is_bounded_context_only() {
    let config = ProceduralWorldConfig::with_seed(4242);
    let creature = anchor(1, 0.0, 0.0);
    let activation = activate_procedural_chunks_around_creatures(config, &[creature]).unwrap();
    let report = generate_procedural_world_content(config, &activation).unwrap();

    let neighborhood =
        sample_creature_procedural_content_neighborhood(config, creature, &report).unwrap();

    neighborhood.validate(config).unwrap();
    assert_eq!(neighborhood.stable_id, WorldEntityId(1));
    assert!(neighborhood.candidate_count <= config.max_neighborhood_samples);
    assert!(neighborhood.bounded_for_sensory);
    assert!(!neighborhood.can_emit_actions);
    assert!(!neighborhood.can_rewrite_weights);
}
