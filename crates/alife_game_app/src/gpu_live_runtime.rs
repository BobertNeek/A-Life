//! Optional full GPU neural runtime product smoke.
//!
//! This bridge keeps the default product path CPU/headless-safe. When the
//! `gpu-runtime` feature is enabled, it can dispatch a compact static GPU
//! scorer and feed CPU-shadow-verified scores into the existing live tick.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FullGpuRuntimeSmokeMode {
    #[default]
    CpuReference,
    StaticShadow,
    StaticActionAuthoritative,
    StaticPlasticShadow,
    FullShadow,
    FullActionAuthoritative,
}

impl FullGpuRuntimeSmokeMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CpuReference => "cpu-reference",
            Self::StaticShadow => "static-shadow",
            Self::StaticActionAuthoritative => "static-action-authoritative",
            Self::StaticPlasticShadow => "static-plastic-shadow",
            Self::FullShadow => "full-shadow",
            Self::FullActionAuthoritative => "full-action-authoritative",
        }
    }

    pub const fn requests_plasticity(self) -> bool {
        matches!(
            self,
            Self::StaticPlasticShadow | Self::FullShadow | Self::FullActionAuthoritative
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullGpuRuntimeSmokeOptions {
    pub mode: FullGpuRuntimeSmokeMode,
    pub ticks: u32,
    pub json_path: Option<PathBuf>,
}

impl Default for FullGpuRuntimeSmokeOptions {
    fn default() -> Self {
        Self {
            mode: FullGpuRuntimeSmokeMode::default(),
            ticks: 1,
            json_path: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullGpuRuntimeSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_mode: String,
    pub selected_backend: String,
    pub fallback_reason: Option<String>,
    pub hardware_identifier: Option<String>,
    pub ticks_run: u32,
    pub actions_selected: Vec<String>,
    pub sealed_patches: usize,
    pub packed_logs: usize,
    pub gpu_static_dispatched: bool,
    pub gpu_output_used_for_proposals: bool,
    pub cpu_shadow_parity: bool,
    pub routing_total_tiles: u32,
    pub routing_active_tiles: u32,
    pub routing_skipped_tiles: u32,
    pub routing_active_synapses: u32,
    pub compact_readback_bytes: usize,
    pub bulk_readback_forbidden: bool,
    pub per_synapse_readback_forbidden: bool,
    pub per_lobe_readback_forbidden: bool,
    pub weight_readback_forbidden: bool,
    pub plasticity_dispatched: bool,
    pub plasticity_diagnostic_only: bool,
    pub plasticity_post_seal_only: bool,
    pub h_shadow_changed: bool,
    pub h_shadow_updated_values: u32,
    pub h_shadow_max_delta_q: i32,
    pub w_genetic_fixed_unchanged: bool,
    pub experience_patch_sealed_before_plasticity: bool,
    pub upload_ms: f32,
    pub gpu_submit_poll_ms: f32,
    pub compact_readback_ms: f32,
    pub cpu_shadow_ms: f32,
    pub total_gpu_runtime_ms: f32,
    pub product_runtime_claim: String,
    pub plasticity_live_gap: String,
}

impl FullGpuRuntimeSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != FULL_GPU_NEURAL_RUNTIME_SCHEMA
            || self.schema_version != FULL_GPU_NEURAL_RUNTIME_SCHEMA_VERSION
            || self.ticks_run == 0
            || self.ticks_run > FULL_GPU_NEURAL_RUNTIME_MAX_TICKS
            || !self.bulk_readback_forbidden
            || !self.per_synapse_readback_forbidden
            || !self.per_lobe_readback_forbidden
            || !self.weight_readback_forbidden
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "full GPU runtime summary violated product boundary",
            });
        }
        if self.gpu_output_used_for_proposals && !self.cpu_shadow_parity {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU proposal scoring requires CPU shadow parity",
            });
        }
        if self.plasticity_dispatched
            && (!self.plasticity_diagnostic_only
                || !self.plasticity_post_seal_only
                || !self.experience_patch_sealed_before_plasticity
                || !self.w_genetic_fixed_unchanged)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU plasticity must remain post-seal H_shadow-only diagnostic evidence",
            });
        }
        Ok(())
    }

    pub fn write_json(&self, path: impl AsRef<Path>) -> Result<(), GameAppShellError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

pub fn run_full_gpu_runtime_smoke(
    launch: &AppShellLaunchConfig,
    options: FullGpuRuntimeSmokeOptions,
) -> Result<FullGpuRuntimeSmokeSummary, GameAppShellError> {
    let ticks = options.ticks.clamp(1, FULL_GPU_NEURAL_RUNTIME_MAX_TICKS);
    let summary = run_full_gpu_runtime_smoke_inner(launch, options.mode, ticks)?;
    if let Some(path) = options.json_path {
        summary.write_json(path)?;
    }
    Ok(summary)
}

#[cfg(not(feature = "gpu-runtime"))]
fn run_full_gpu_runtime_smoke_inner(
    launch: &AppShellLaunchConfig,
    mode: FullGpuRuntimeSmokeMode,
    ticks: u32,
) -> Result<FullGpuRuntimeSmokeSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let summaries = live.update(LiveBrainTickControl::run_fixed(ticks))?;
    let summary = cpu_fallback_summary(mode, ticks, &summaries, "FeatureDisabled");
    summary.validate()?;
    Ok(summary)
}

#[cfg(feature = "gpu-runtime")]
fn run_full_gpu_runtime_smoke_inner(
    launch: &AppShellLaunchConfig,
    mode: FullGpuRuntimeSmokeMode,
    ticks: u32,
) -> Result<FullGpuRuntimeSmokeSummary, GameAppShellError> {
    use alife_gpu_backend::{
        run_full_gpu_runtime_post_seal_plasticity_diagnostic, run_full_gpu_runtime_static_tick,
        FullGpuRuntimeProductClaim,
    };

    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let backend_mode = backend_mode(mode);
    let mut tick_summaries = Vec::with_capacity(ticks as usize);
    let mut last_static = None;
    let mut plasticity = None;
    let mut gpu_output_used = false;

    for tick_index in 0..ticks {
        let report = live.current_sensory_report()?;
        let input = runtime_input_from_sensory(&report)?;
        let static_report = run_full_gpu_runtime_static_tick(input, backend_mode)?;
        let proposals = if static_report.action_summary.is_some()
            && static_report.cpu_shadow_parity_passed
            && matches!(
                static_report.product_runtime_claim,
                FullGpuRuntimeProductClaim::CpuShadowGuarded
                    | FullGpuRuntimeProductClaim::ActionAuthoritative
            ) {
            gpu_output_used = true;
            live.current_context_proposals_with_scores(scores_from_action_summary(
                static_report.action_summary.expect("checked above"),
            )?)?
        } else {
            live.current_context_proposals()?
        };
        let tick = live.tick_with_proposals(proposals);
        if tick.patch_sealed
            && tick_index == 0
            && mode.requests_plasticity()
            && static_report.backend.fallback_reason.is_none()
        {
            plasticity = Some(run_full_gpu_runtime_post_seal_plasticity_diagnostic(input)?);
        }
        last_static = Some(static_report);
        tick_summaries.push(tick);
    }

    let Some(static_report) = last_static else {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "full GPU runtime smoke must produce at least one tick",
        });
    };
    let actions_selected = selected_actions(&tick_summaries);
    let sealed_patches = tick_summaries
        .last()
        .map_or(0, |summary| summary.sealed_patch_count);
    let packed_logs = tick_summaries
        .last()
        .map_or(0, |summary| summary.packed_record_count);
    let plasticity_report = plasticity.as_ref();
    let summary = FullGpuRuntimeSmokeSummary {
        schema: FULL_GPU_NEURAL_RUNTIME_SCHEMA,
        schema_version: FULL_GPU_NEURAL_RUNTIME_SCHEMA_VERSION,
        requested_mode: mode.label().to_string(),
        selected_backend: format!("{:?}", static_report.backend.selected),
        fallback_reason: static_report
            .backend
            .fallback_reason
            .map(|reason| format!("{reason:?}")),
        hardware_identifier: static_report.hardware_identifier.clone(),
        ticks_run: ticks,
        actions_selected,
        sealed_patches,
        packed_logs,
        gpu_static_dispatched: static_report.action_summary.is_some(),
        gpu_output_used_for_proposals: gpu_output_used,
        cpu_shadow_parity: static_report.cpu_shadow_parity_passed,
        routing_total_tiles: static_report.routing.total_tiles,
        routing_active_tiles: static_report.routing.active_tiles,
        routing_skipped_tiles: static_report.routing.skipped_tiles,
        routing_active_synapses: static_report.routing.active_synapses,
        compact_readback_bytes: static_report.readback.compact_readback_bytes,
        bulk_readback_forbidden: static_report.readback.bulk_neural_readback_forbidden,
        per_synapse_readback_forbidden: static_report.readback.per_synapse_readback_forbidden,
        per_lobe_readback_forbidden: static_report.readback.per_lobe_readback_forbidden,
        weight_readback_forbidden: static_report.readback.weight_readback_forbidden,
        plasticity_dispatched: plasticity_report.is_some(),
        plasticity_diagnostic_only: plasticity_report.is_none_or(|report| report.diagnostic_only),
        plasticity_post_seal_only: plasticity_report.is_none_or(|report| report.post_seal_only),
        h_shadow_changed: plasticity_report.is_some_and(|report| report.h_shadow_changed),
        h_shadow_updated_values: plasticity_report.map_or(0, |report| report.updated_values_count),
        h_shadow_max_delta_q: plasticity_report.map_or(0, |report| report.max_delta_q),
        w_genetic_fixed_unchanged: plasticity_report
            .is_none_or(|report| report.genetic_fixed_unchanged),
        experience_patch_sealed_before_plasticity: plasticity_report
            .is_none_or(|_| sealed_patches > 0),
        upload_ms: static_report.timing.upload_ms,
        gpu_submit_poll_ms: static_report.timing.gpu_submit_poll_ms,
        compact_readback_ms: static_report.timing.compact_readback_ms,
        cpu_shadow_ms: static_report.timing.cpu_shadow_ms,
        total_gpu_runtime_ms: static_report.timing.total_gpu_runtime_ms,
        product_runtime_claim: format!("{:?}", static_report.product_runtime_claim),
        plasticity_live_gap: static_report.fallback_note.clone().unwrap_or_else(|| {
            "live H_shadow application remains diagnostic/shadow-only until alife_core owns a safe post-seal lifetime-state hook"
                .to_string()
        }),
    };
    summary.validate()?;
    Ok(summary)
}

#[cfg(not(feature = "gpu-runtime"))]
fn cpu_fallback_summary(
    mode: FullGpuRuntimeSmokeMode,
    ticks: u32,
    summaries: &[LiveBrainTickSummary],
    reason: &str,
) -> FullGpuRuntimeSmokeSummary {
    FullGpuRuntimeSmokeSummary {
        schema: FULL_GPU_NEURAL_RUNTIME_SCHEMA,
        schema_version: FULL_GPU_NEURAL_RUNTIME_SCHEMA_VERSION,
        requested_mode: mode.label().to_string(),
        selected_backend: "CpuReference".to_string(),
        fallback_reason: Some(reason.to_string()),
        hardware_identifier: None,
        ticks_run: ticks,
        actions_selected: selected_actions(summaries),
        sealed_patches: summaries
            .last()
            .map_or(0, |summary| summary.sealed_patch_count),
        packed_logs: summaries
            .last()
            .map_or(0, |summary| summary.packed_record_count),
        gpu_static_dispatched: false,
        gpu_output_used_for_proposals: false,
        cpu_shadow_parity: false,
        routing_total_tiles: 0,
        routing_active_tiles: 0,
        routing_skipped_tiles: 0,
        routing_active_synapses: 0,
        compact_readback_bytes: 0,
        bulk_readback_forbidden: true,
        per_synapse_readback_forbidden: true,
        per_lobe_readback_forbidden: true,
        weight_readback_forbidden: true,
        plasticity_dispatched: false,
        plasticity_diagnostic_only: true,
        plasticity_post_seal_only: true,
        h_shadow_changed: false,
        h_shadow_updated_values: 0,
        h_shadow_max_delta_q: 0,
        w_genetic_fixed_unchanged: true,
        experience_patch_sealed_before_plasticity: false,
        upload_ms: 0.0,
        gpu_submit_poll_ms: 0.0,
        compact_readback_ms: 0.0,
        cpu_shadow_ms: 0.0,
        total_gpu_runtime_ms: 0.0,
        product_runtime_claim: "None".to_string(),
        plasticity_live_gap:
            "GPU feature unavailable; CPU reference sealed patches remain authoritative".to_string(),
    }
}

#[cfg(feature = "gpu-runtime")]
fn backend_mode(mode: FullGpuRuntimeSmokeMode) -> alife_gpu_backend::FullGpuRuntimeMode {
    match mode {
        FullGpuRuntimeSmokeMode::CpuReference => {
            alife_gpu_backend::FullGpuRuntimeMode::CpuReference
        }
        FullGpuRuntimeSmokeMode::StaticShadow => {
            alife_gpu_backend::FullGpuRuntimeMode::GpuStaticShadow
        }
        FullGpuRuntimeSmokeMode::StaticActionAuthoritative => {
            alife_gpu_backend::FullGpuRuntimeMode::GpuStaticActionAuthoritative
        }
        FullGpuRuntimeSmokeMode::StaticPlasticShadow => {
            alife_gpu_backend::FullGpuRuntimeMode::GpuStaticPlasticShadow
        }
        FullGpuRuntimeSmokeMode::FullShadow => alife_gpu_backend::FullGpuRuntimeMode::GpuFullShadow,
        FullGpuRuntimeSmokeMode::FullActionAuthoritative => {
            alife_gpu_backend::FullGpuRuntimeMode::GpuFullActionAuthoritative
        }
    }
}

#[cfg(feature = "gpu-runtime")]
fn runtime_input_from_sensory(
    report: &HeadlessSensoryReport,
) -> Result<alife_gpu_backend::FullGpuRuntimeStaticTickInput, GameAppShellError> {
    let mut food_salience = 0.0_f32;
    let mut hazard_salience = 0.0_f32;
    let mut inspect_salience = 0.0_f32;
    for visible in &report.visible_entities {
        let salience = visible_salience(visible.distance);
        match visible.kind {
            WorldObjectKind::Food => food_salience = food_salience.max(salience),
            WorldObjectKind::Hazard => hazard_salience = hazard_salience.max(salience),
            WorldObjectKind::Obstacle | WorldObjectKind::Agent | WorldObjectKind::Token => {
                inspect_salience = inspect_salience.max(salience);
            }
        }
    }
    Ok(alife_gpu_backend::FullGpuRuntimeStaticTickInput {
        action_ids: [
            HeadlessActionIds::EAT.raw(),
            HeadlessActionIds::FLEE.raw(),
            ActionKind::Inspect.canonical_id().raw(),
            ActionKind::Idle.canonical_id().raw(),
        ],
        food_salience,
        hazard_salience,
        inspect_salience,
        idle_salience: 0.28,
        confidence: 0.95,
        drive_source_mask: 0b11,
    })
}

#[cfg(feature = "gpu-runtime")]
fn scores_from_action_summary(
    summary: alife_gpu_backend::GpuActionSummaryStagingRecord,
) -> Result<LiveBrainProposalScores, GameAppShellError> {
    Ok(LiveBrainProposalScores {
        food_score: dequantize_score(summary.reserved[2])?,
        hazard_score: dequantize_score(summary.reserved[3])?,
        inspect_score: dequantize_score(summary.reserved[4])?,
        idle_score: dequantize_score(summary.reserved[5])?,
        confidence: summary.confidence_q16 as f32 / u16::MAX as f32,
    })
}

#[cfg(feature = "gpu-runtime")]
fn dequantize_score(word: u32) -> Result<f32, GameAppShellError> {
    let signed = i32::from_ne_bytes(word.to_ne_bytes());
    let score = (signed as f32 / 32767.0).clamp(0.0, 1.0);
    alife_core::validate_finite(score)?;
    Ok(score)
}

#[cfg(feature = "gpu-runtime")]
fn visible_salience(distance: f32) -> f32 {
    if distance <= 0.0 {
        0.5
    } else {
        (1.0 / (1.0 + distance)).clamp(0.1, 1.0)
    }
}

fn selected_actions(summaries: &[LiveBrainTickSummary]) -> Vec<String> {
    summaries
        .iter()
        .map(|summary| {
            format!(
                "{:?}:{:?}",
                summary.selected_action_kind,
                summary.selected_action_id.map(ActionId::raw)
            )
        })
        .collect()
}
