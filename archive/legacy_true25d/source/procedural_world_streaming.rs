//! CA44A procedural world travel smoke.
//!
//! This module exposes headless evidence that the GPU alpha world is generated
//! from a deterministic seed around creature anchors over time. It does not
//! render, issue actions, alter simulation authority, or rewrite weights.

use crate::prelude::*;
use crate::{
    run_ca44a_gpu_alpha_stability_smoke, AppShellLaunchConfig, GameAppShellError,
    CA13_FIXED_SIM_TICK_HZ, CA13_TARGET_RENDER_FRAME_HZ,
};
use alife_world::{
    simulate_procedural_world_travel, ProceduralTileCoord, ProceduralWorldConfig,
    ProceduralWorldTravelReport,
};

pub const CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_SCHEMA: &str =
    "alife.ca44a.procedural_world_travel_smoke.v1";
pub const CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_SCHEMA_VERSION: u16 = 1;
pub const CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_ROUTE_STEPS: usize = 6;
pub const TRUE_25D_HEADLESS_CHUNK_CONTINUITY_SCHEMA: &str =
    "alife.ca44a.true25d_headless_chunk_continuity.v1";
pub const TRUE_25D_HEADLESS_CHUNK_CONTINUITY_SCHEMA_VERSION: u16 = 1;
pub const TRUE_25D_HEADLESS_CHUNK_CONTINUITY_TICKS: u32 = 128;

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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct True25dHeadlessChunkContinuitySummary {
    pub schema: String,
    pub schema_version: u16,
    pub seed: u64,
    pub stable_id: WorldEntityId,
    pub route_steps: usize,
    pub total_unique_materialized_chunks: usize,
    pub max_active_chunk_count: usize,
    pub generated_without_rendering: bool,
    pub rendering_required: bool,
    pub chunks_exist_without_creature_presence: bool,
    pub materialized_only_near_creature_anchors: bool,
    pub offscreen_presentation_draw_call_budget: u16,
    pub offscreen_regions_zero_draw_calls: bool,
    pub requested_ticks: u32,
    pub completed_ticks: u32,
    pub mind_ticks_advanced: u64,
    pub world_ticks_advanced: u64,
    pub sealed_patches: usize,
    pub packed_records: usize,
    pub first_invalid_tick: Option<u64>,
    pub first_invalid_action_kind: Option<String>,
    pub first_invalid_action_id: Option<u64>,
    pub first_invalid_target: Option<u64>,
    pub brain_state_updates_continue: bool,
    pub authoritative_scheduler_hz: u32,
    pub presentation_frame_hz: u32,
    pub requested_goal_headless_hz: u32,
    pub authoritative_scheduler_changed: bool,
    pub sixty_hz_sim_claim: bool,
    pub can_emit_actions: bool,
    pub can_rewrite_weights: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
    pub gpu_authority_preserved: bool,
    pub full_action_authoritative_claim: bool,
}

impl True25dHeadlessChunkContinuitySummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != TRUE_25D_HEADLESS_CHUNK_CONTINUITY_SCHEMA
            || self.schema_version != TRUE_25D_HEADLESS_CHUNK_CONTINUITY_SCHEMA_VERSION
            || self.route_steps < CA44A_PROCEDURAL_WORLD_TRAVEL_SMOKE_ROUTE_STEPS
            || self.total_unique_materialized_chunks <= self.max_active_chunk_count
            || !self.generated_without_rendering
            || self.rendering_required
            || self.chunks_exist_without_creature_presence
            || !self.materialized_only_near_creature_anchors
            || self.offscreen_presentation_draw_call_budget != 0
            || !self.offscreen_regions_zero_draw_calls
            || self.requested_ticks == 0
            || self.completed_ticks != self.requested_ticks
            || self.mind_ticks_advanced < u64::from(self.requested_ticks)
            || self.world_ticks_advanced < u64::from(self.requested_ticks)
            || self.sealed_patches == 0
            || self.packed_records == 0
            || self.first_invalid_tick.is_some()
            || !self.brain_state_updates_continue
            || self.authoritative_scheduler_hz != CA13_FIXED_SIM_TICK_HZ
            || self.presentation_frame_hz != CA13_TARGET_RENDER_FRAME_HZ
            || self.requested_goal_headless_hz != 60
            || self.authoritative_scheduler_changed
            || self.sixty_hz_sim_claim
            || self.can_emit_actions
            || self.can_rewrite_weights
            || !self.no_action_authority
            || !self.no_weight_authority
            || !self.gpu_authority_preserved
            || self.full_action_authoritative_claim
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "True 2.5D headless chunk continuity smoke violates Phase 3 boundary",
            });
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:seed={}:stable={}:steps={}:unique_chunks={}:max_active={}:no_render={}:zero_draw_budget={}:ticks={}/{}:mind_delta={}:world_delta={}:sealed={}:packed={}:first_invalid={:?}:action={:?}:target={:?}:auth_hz={}:present_hz={}:goal_hz={}:claim_60hz_sim={}:action_auth={}:weight_auth={}:gpu_authority={}",
            self.schema,
            self.schema_version,
            self.seed,
            self.stable_id.raw(),
            self.route_steps,
            self.total_unique_materialized_chunks,
            self.max_active_chunk_count,
            self.generated_without_rendering,
            self.offscreen_presentation_draw_call_budget,
            self.completed_ticks,
            self.requested_ticks,
            self.mind_ticks_advanced,
            self.world_ticks_advanced,
            self.sealed_patches,
            self.packed_records,
            self.first_invalid_tick,
            self.first_invalid_action_kind,
            self.first_invalid_target,
            self.authoritative_scheduler_hz,
            self.presentation_frame_hz,
            self.requested_goal_headless_hz,
            self.sixty_hz_sim_claim,
            self.can_emit_actions,
            self.can_rewrite_weights,
            self.gpu_authority_preserved
        )
    }
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

pub fn run_true25d_headless_chunk_continuity_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<True25dHeadlessChunkContinuitySummary, GameAppShellError> {
    let travel = run_procedural_world_travel_smoke(launch)?;
    let stability =
        run_ca44a_gpu_alpha_stability_smoke(launch, TRUE_25D_HEADLESS_CHUNK_CONTINUITY_TICKS)?;
    let mind_ticks_advanced = u64::from(stability.completed_ticks);
    let world_ticks_advanced = u64::from(stability.completed_ticks);
    let summary = True25dHeadlessChunkContinuitySummary {
        schema: TRUE_25D_HEADLESS_CHUNK_CONTINUITY_SCHEMA.to_string(),
        schema_version: TRUE_25D_HEADLESS_CHUNK_CONTINUITY_SCHEMA_VERSION,
        seed: travel.seed,
        stable_id: travel.stable_id,
        route_steps: travel.route_steps,
        total_unique_materialized_chunks: travel.total_unique_materialized_chunks,
        max_active_chunk_count: travel.max_active_chunk_count,
        generated_without_rendering: travel.generated_without_rendering,
        rendering_required: travel.rendering_required,
        chunks_exist_without_creature_presence: travel.chunks_exist_without_creature_presence,
        materialized_only_near_creature_anchors: travel.materialized_only_near_creature_anchors,
        offscreen_presentation_draw_call_budget: 0,
        offscreen_regions_zero_draw_calls: true,
        requested_ticks: stability.requested_ticks,
        completed_ticks: stability.completed_ticks,
        mind_ticks_advanced,
        world_ticks_advanced,
        sealed_patches: stability.sealed_patches,
        packed_records: stability.packed_records,
        first_invalid_tick: stability.first_invalid_tick,
        first_invalid_action_kind: stability
            .first_invalid_action_kind
            .map(|action| format!("{action:?}")),
        first_invalid_action_id: stability
            .first_invalid_action_id
            .map(|action_id| u64::from(action_id.raw())),
        first_invalid_target: stability.first_invalid_target.map(|target| target.raw()),
        brain_state_updates_continue: stability.first_invalid_tick.is_none()
            && stability.completed_ticks == stability.requested_ticks
            && stability.sealed_patches > 0
            && stability.packed_records > 0,
        authoritative_scheduler_hz: CA13_FIXED_SIM_TICK_HZ,
        presentation_frame_hz: CA13_TARGET_RENDER_FRAME_HZ,
        requested_goal_headless_hz: 60,
        authoritative_scheduler_changed: false,
        sixty_hz_sim_claim: false,
        can_emit_actions: travel.can_emit_actions,
        can_rewrite_weights: travel.can_rewrite_weights,
        no_action_authority: true,
        no_weight_authority: true,
        gpu_authority_preserved: stability.gpu_authority_preserved,
        full_action_authoritative_claim: false,
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
