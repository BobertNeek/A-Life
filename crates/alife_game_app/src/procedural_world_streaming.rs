//! CA44A procedural world travel smoke.
//!
//! This module exposes headless evidence that the GPU alpha world is generated
//! from a deterministic seed around creature anchors over time. It does not
//! render, issue actions, alter simulation authority, or rewrite weights.

use crate::prelude::*;
use crate::{AppShellLaunchConfig, GameAppShellError};
use alife_world::{
    simulate_procedural_world_travel, ProceduralTileCoord, ProceduralWorldConfig,
    ProceduralWorldTravelReport,
};

pub const CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_SCHEMA: &str =
    "alife.ca44a.procedural_world_travel_smoke.v1";
pub const CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_SCHEMA_VERSION: u16 = 1;
pub const CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_ROUTE_STEPS: usize = 6;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralWorldTravelSmokeSummary {
    pub schema: String,
    pub schema_version: u16,
    pub seed: u64,
    pub stable_id: WorldEntityId,
    pub route_steps: usize,
    pub total_unique_materialized_chunks: usize,
    pub max_active_chunk_count: usize,
    pub total_content_candidates_seen: usize,
    pub generated_without_rendering: bool,
    pub rendering_required: bool,
    pub chunks_exist_without_creature_presence: bool,
    pub materialized_only_near_creature_anchors: bool,
    pub bounded_for_creature_context: bool,
    pub can_emit_actions: bool,
    pub can_rewrite_weights: bool,
    pub world_generation_claim: String,
    pub travel_report: ProceduralWorldTravelReport,
}

impl ProceduralWorldTravelSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_SCHEMA
            || self.schema_version != CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_SCHEMA_VERSION
            || self.route_steps < CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_ROUTE_STEPS
            || self.total_unique_materialized_chunks <= self.max_active_chunk_count
            || self.total_content_candidates_seen == 0
            || !self.generated_without_rendering
            || self.rendering_required
            || self.chunks_exist_without_creature_presence
            || !self.materialized_only_near_creature_anchors
            || !self.bounded_for_creature_context
            || self.can_emit_actions
            || self.can_rewrite_weights
            || self.world_generation_claim != "SeededCreatureAnchoredNoRenderChunks"
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA44A procedural world travel smoke violates streaming contract",
            });
        }
        self.travel_report
            .validate(ProceduralWorldConfig::with_seed(self.seed))?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:seed={}:stable={}:steps={}:unique_chunks={}:max_active={}:content_seen={}:no_render={}:rendering_required={}:anchor_only={}:action_authority={}:weight_authority={}:claim={}",
            self.schema,
            self.schema_version,
            self.seed,
            self.stable_id.raw(),
            self.route_steps,
            self.total_unique_materialized_chunks,
            self.max_active_chunk_count,
            self.total_content_candidates_seen,
            self.generated_without_rendering,
            self.rendering_required,
            self.materialized_only_near_creature_anchors,
            self.can_emit_actions,
            self.can_rewrite_weights,
            self.world_generation_claim
        )
    }
}

pub fn run_procedural_world_travel_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<ProceduralWorldTravelSmokeSummary, GameAppShellError> {
    let save = PortableSaveFile::from_json_file(&launch.save_path)?;
    save.validate_with_asset_root(&launch.asset_root)?;
    let creature = save
        .world
        .objects
        .iter()
        .find(|object| object.kind == WorldObjectKind::Agent)
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "CA44A procedural travel smoke requires a creature anchor",
        })?;
    let base_tile = ProceduralTileCoord::new(
        creature.position.x.round() as i32,
        creature.position.z.round() as i32,
    );
    let route = ca44a_default_procedural_travel_route(base_tile);
    let config = ProceduralWorldConfig::with_seed(save.deterministic_seed);
    let travel_report = simulate_procedural_world_travel(config, creature.id, &route)?;
    let summary = ProceduralWorldTravelSmokeSummary {
        schema: CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_SCHEMA.to_string(),
        schema_version: CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_SCHEMA_VERSION,
        seed: save.deterministic_seed,
        stable_id: creature.id,
        route_steps: route.len(),
        total_unique_materialized_chunks: travel_report.total_unique_materialized_chunks,
        max_active_chunk_count: travel_report.max_active_chunk_count,
        total_content_candidates_seen: travel_report.total_content_candidates_seen,
        generated_without_rendering: travel_report.generated_without_rendering,
        rendering_required: travel_report.rendering_required,
        chunks_exist_without_creature_presence: travel_report
            .chunks_exist_without_creature_presence,
        materialized_only_near_creature_anchors: travel_report
            .materialized_only_near_creature_anchors,
        bounded_for_creature_context: travel_report.bounded_for_creature_context,
        can_emit_actions: travel_report.can_emit_actions,
        can_rewrite_weights: travel_report.can_rewrite_weights,
        world_generation_claim: "SeededCreatureAnchoredNoRenderChunks".to_string(),
        travel_report,
    };
    summary.validate()?;
    Ok(summary)
}

fn ca44a_default_procedural_travel_route(base: ProceduralTileCoord) -> Vec<ProceduralTileCoord> {
    [
        (0, 0),
        (48, 0),
        (96, -64),
        (144, 32),
        (192, -96),
        (240, -16),
    ]
    .into_iter()
    .map(|(x, z)| ProceduralTileCoord::new(base.x + x, base.z + z))
    .collect()
}
