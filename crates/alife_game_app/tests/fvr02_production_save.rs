use alife_game_app::{
    default_environment_manifest_path, run_production_voxel_frontend_dry_run,
    ProductionFrontendProfileId, ProductionVoxelLaunchConfig,
};
use alife_world::FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA;

#[test]
fn fvr02_validate_production_save_reports_real_persistent_voxel_backend_roundtrip() {
    let mut launch =
        ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
            .unwrap();
    launch.profile_id = ProductionFrontendProfileId::MinimumSettings30x30;
    launch.population = Some(30);
    launch.dry_run = true;

    let summary = run_production_voxel_frontend_dry_run(&launch).unwrap();

    assert_eq!(
        summary.save_metadata.voxel_backend_schema.as_deref(),
        Some(FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA)
    );
    assert!(summary.save_metadata.voxel_visible_chunk_signatures > 0);
    assert!(summary.save_metadata.voxel_materialized_chunks > 0);
    assert!(summary.save_metadata.voxel_resource_hazard_refs > 0);
    assert!(
        summary.save_metadata.voxel_stable_selection_refs >= summary.save_metadata.creature_count
    );
    assert!(
        summary.save_metadata.voxel_dirty_region_count
            <= summary.save_metadata.voxel_visible_chunk_signatures
    );
    assert!(summary.save_metadata.voxel_roundtrip_signatures_match);
    assert!(summary.save_metadata.no_renderer_tokens_in_voxel_save);
    assert_eq!(
        summary.save_metadata.selected_profile,
        "MinimumSettings30x30"
    );
}
