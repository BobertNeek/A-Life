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
    StaticPlasticCpuShadowGuarded,
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
            Self::StaticPlasticCpuShadowGuarded => "static-plastic-cpu-shadow-guarded",
            Self::FullShadow => "full-shadow",
            Self::FullActionAuthoritative => "full-action-authoritative",
        }
    }

    pub const fn requests_plasticity(self) -> bool {
        matches!(
            self,
            Self::StaticPlasticShadow
                | Self::StaticPlasticCpuShadowGuarded
                | Self::FullShadow
                | Self::FullActionAuthoritative
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
    pub combined_mode: bool,
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
    pub plasticity_live_core_update_applied: bool,
    pub post_seal_hshadow_applied: bool,
    pub plasticity_post_seal_only: bool,
    pub post_seal_diagnostic_readback_bytes: usize,
    pub post_seal_diagnostic_readback_ms: f32,
    pub post_seal_diagnostic_readback_boundary_scoped: bool,
    pub h_shadow_changed: bool,
    pub h_shadow_updated_values: u32,
    pub h_shadow_max_delta_q: i32,
    pub post_seal_delta_applied_records: u32,
    pub post_seal_delta_changed_records: u32,
    pub post_seal_delta_max_abs_delta: f32,
    pub post_seal_delta_sequence_id: Option<u64>,
    pub post_seal_replay_protected: bool,
    pub w_genetic_fixed_unchanged: bool,
    pub lifetime_consolidated_unchanged: bool,
    pub h_operational_unchanged: bool,
    pub experience_patch_sealed_before_plasticity: bool,
    pub upload_ms: f32,
    pub gpu_submit_poll_ms: f32,
    pub compact_readback_ms: f32,
    pub cpu_shadow_ms: f32,
    pub total_gpu_runtime_ms: f32,
    pub product_runtime_claim: String,
    pub unsupported_full_runtime_gap_remaining: bool,
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
        alife_core::validate_finite(self.post_seal_delta_max_abs_delta)?;
        alife_core::validate_finite(self.post_seal_diagnostic_readback_ms)?;
        if self.post_seal_hshadow_applied != self.plasticity_live_core_update_applied {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "post-seal H_shadow application must match live core update status",
            });
        }
        if self.plasticity_dispatched
            && (self.post_seal_diagnostic_readback_bytes == 0
                || !self.post_seal_diagnostic_readback_boundary_scoped)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "post-seal plasticity readback must be explicit and boundary scoped",
            });
        }
        if self.plasticity_dispatched
            && (!self.plasticity_post_seal_only
                || !self.experience_patch_sealed_before_plasticity
                || !self.w_genetic_fixed_unchanged
                || !self.lifetime_consolidated_unchanged
                || !self.h_operational_unchanged)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU plasticity must remain post-seal H_shadow-only evidence",
            });
        }
        if self.plasticity_live_core_update_applied
            && (self.post_seal_delta_applied_records == 0
                || self.post_seal_delta_sequence_id.is_none()
                || !self.post_seal_hshadow_applied
                || !self.post_seal_replay_protected
                || self.plasticity_diagnostic_only)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "live H_shadow application requires a post-seal delta receipt",
            });
        }
        if self.combined_mode
            && self.product_runtime_claim == "CpuShadowGuardedStaticPlusLiveHShadow"
            && (!self.gpu_output_used_for_proposals || !self.post_seal_hshadow_applied)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "combined GPU runtime claim requires GPU proposals and H_shadow receipt",
            });
        }
        if self.combined_mode && self.gpu_static_dispatched && self.fallback_reason.is_none() {
            match self.product_runtime_claim.as_str() {
                "CpuShadowGuardedStaticPlusLiveHShadow" => {
                    if !self.gpu_output_used_for_proposals
                        || !self.post_seal_hshadow_applied
                        || !self.plasticity_live_core_update_applied
                        || self.post_seal_delta_applied_records == 0
                    {
                        return Err(GameAppShellError::VisibleWorldMismatch {
                            message:
                                "combined runtime claim requires GPU proposals and applied H_shadow deltas",
                        });
                    }
                }
                "CpuShadowGuarded" => {
                    if !self.gpu_output_used_for_proposals || self.post_seal_hshadow_applied {
                        return Err(GameAppShellError::VisibleWorldMismatch {
                            message:
                                "combined static-only degradation must use GPU proposals without H_shadow application",
                        });
                    }
                }
                _ => {
                    return Err(GameAppShellError::VisibleWorldMismatch {
                        message:
                            "combined GPU runtime must report a CPU-shadow-guarded claim or explicit fallback",
                    });
                }
            }
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

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicalGpuRuntimeTelemetry {
    pub requested_mode: GraphicalGpuRuntimeMode,
    pub selected_backend: String,
    pub fallback_reason: Option<String>,
    pub hardware_identifier: Option<String>,
    pub product_runtime_claim: String,
    pub gpu_static_dispatched_ticks: u32,
    pub gpu_scores_used_for_proposals: bool,
    pub cpu_shadow_parity: bool,
    pub parity_failures: u32,
    pub sealed_patches: usize,
    pub h_shadow_applications: u32,
    pub last_h_shadow_delta: f32,
    pub compact_readback_bytes: usize,
    pub post_seal_readback_bytes: usize,
    pub total_gpu_runtime_ms: f32,
    pub no_active_bulk_readback: bool,
    pub full_action_authoritative_claim: bool,
}

impl GraphicalGpuRuntimeTelemetry {
    pub fn pending(requested_mode: GraphicalGpuRuntimeMode) -> Self {
        if !requested_mode.requests_gpu() {
            return Self::cpu_reference(requested_mode, 0);
        }
        Self {
            requested_mode,
            selected_backend: "PendingFirstTick".to_string(),
            fallback_reason: None,
            hardware_identifier: None,
            product_runtime_claim: "PendingTick".to_string(),
            gpu_static_dispatched_ticks: 0,
            gpu_scores_used_for_proposals: false,
            cpu_shadow_parity: false,
            parity_failures: 0,
            sealed_patches: 0,
            h_shadow_applications: 0,
            last_h_shadow_delta: 0.0,
            compact_readback_bytes: 0,
            post_seal_readback_bytes: 0,
            total_gpu_runtime_ms: 0.0,
            no_active_bulk_readback: true,
            full_action_authoritative_claim: false,
        }
    }

    pub fn cpu_reference(requested_mode: GraphicalGpuRuntimeMode, sealed_patches: usize) -> Self {
        Self {
            requested_mode,
            selected_backend: "CpuReference".to_string(),
            fallback_reason: if requested_mode.requests_gpu() {
                Some("FeatureDisabledOrUnavailable".to_string())
            } else {
                None
            },
            hardware_identifier: None,
            product_runtime_claim: "None".to_string(),
            gpu_static_dispatched_ticks: 0,
            gpu_scores_used_for_proposals: false,
            cpu_shadow_parity: false,
            parity_failures: 0,
            sealed_patches,
            h_shadow_applications: 0,
            last_h_shadow_delta: 0.0,
            compact_readback_bytes: 0,
            post_seal_readback_bytes: 0,
            total_gpu_runtime_ms: 0.0,
            no_active_bulk_readback: true,
            full_action_authoritative_claim: false,
        }
    }

    pub fn backend_line(&self) -> String {
        let status = if self.fallback_reason.is_some() {
            "CpuFallback degraded"
        } else if self.requested_mode.requests_gpu() {
            "GpuPlastic"
        } else {
            "CpuReference"
        };
        format!(
            "GPU: {}  selected={}  fallback={}",
            status,
            self.selected_backend,
            self.fallback_reason.as_deref().unwrap_or("none")
        )
    }

    pub fn overlay_lines(&self) -> String {
        format!(
            "Details: claim={} scores={} parity={} fail={}\nLearning: H_shadow apps={} readback={}B\nBoundary: CPU shadow gate; not full action-authoritative; no bulk readback={}",
            self.product_runtime_claim,
            self.gpu_scores_used_for_proposals,
            self.cpu_shadow_parity,
            self.parity_failures,
            self.h_shadow_applications,
            self.compact_readback_bytes,
            self.no_active_bulk_readback,
        )
    }

    pub fn inspector_lines(&self) -> String {
        format!(
            concat!(
                "GPU Runtime\n",
                "Mode: {}\n",
                "Backend: {}\n",
                "Fallback/degraded: {}\n",
                "Claim:\n  {}\n",
                "Scores={} parity={} fail={}\n",
                "H_shadow apps={} last={:.5}\n",
                "Readback c={} post={}\n",
                "Gate: CPU shadow\n",
                "No full action-authoritative claim"
            ),
            self.requested_mode.label(),
            self.selected_backend,
            self.fallback_reason.as_deref().unwrap_or("none"),
            self.product_runtime_claim,
            self.gpu_scores_used_for_proposals,
            self.cpu_shadow_parity,
            self.parity_failures,
            self.h_shadow_applications,
            self.last_h_shadow_delta,
            self.compact_readback_bytes,
            self.post_seal_readback_bytes,
        )
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        alife_core::validate_finite(self.last_h_shadow_delta)?;
        alife_core::validate_finite(self.total_gpu_runtime_ms)?;
        if self.full_action_authoritative_claim {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical GPU telemetry must not claim full action-authoritative runtime",
            });
        }
        if self.gpu_scores_used_for_proposals && !self.cpu_shadow_parity {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical GPU proposals require CPU shadow parity",
            });
        }
        if !self.no_active_bulk_readback {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical GPU runtime must not allow active bulk neural readback",
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct GraphicalGpuRuntimeController {
    mode: GraphicalGpuRuntimeMode,
    #[allow(dead_code)]
    h_shadow_applied_once: bool,
    telemetry: GraphicalGpuRuntimeTelemetry,
}

impl GraphicalGpuRuntimeController {
    pub fn new(mode: GraphicalGpuRuntimeMode) -> Self {
        Self {
            mode,
            h_shadow_applied_once: false,
            telemetry: GraphicalGpuRuntimeTelemetry::pending(mode),
        }
    }

    pub const fn mode(&self) -> GraphicalGpuRuntimeMode {
        self.mode
    }

    pub fn telemetry(&self) -> &GraphicalGpuRuntimeTelemetry {
        &self.telemetry
    }

    pub fn tick(
        &mut self,
        live: &mut LiveBrainLoop,
    ) -> Result<LiveBrainTickSummary, GameAppShellError> {
        match self.mode {
            GraphicalGpuRuntimeMode::CpuReference => {
                let proposals = live.current_context_proposals()?;
                let tick = live.tick_with_proposals_detailed(proposals, true);
                self.telemetry = GraphicalGpuRuntimeTelemetry::cpu_reference(
                    self.mode,
                    tick.summary.sealed_patch_count,
                );
                Ok(tick.summary)
            }
            GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded
            | GraphicalGpuRuntimeMode::AutoWithCpuFallback => self.tick_gpu_requested(live),
        }
    }

    #[cfg(not(feature = "gpu-runtime"))]
    fn tick_gpu_requested(
        &mut self,
        live: &mut LiveBrainLoop,
    ) -> Result<LiveBrainTickSummary, GameAppShellError> {
        let proposals = live.current_context_proposals()?;
        let tick = live.tick_with_proposals_detailed(proposals, true);
        self.telemetry =
            GraphicalGpuRuntimeTelemetry::cpu_reference(self.mode, tick.summary.sealed_patch_count);
        Ok(tick.summary)
    }

    #[cfg(feature = "gpu-runtime")]
    fn tick_gpu_requested(
        &mut self,
        live: &mut LiveBrainLoop,
    ) -> Result<LiveBrainTickSummary, GameAppShellError> {
        use alife_gpu_backend::{
            full_gpu_runtime_live_plasticity_schema, post_seal_delta_batch_from_plasticity_report,
            run_full_gpu_runtime_post_seal_plasticity_diagnostic, run_full_gpu_runtime_static_tick,
            FullGpuRuntimeProductClaim,
        };

        let report = live.current_sensory_report()?;
        let input = runtime_input_from_sensory(&report)?;
        let static_report = run_full_gpu_runtime_static_tick(
            input,
            alife_gpu_backend::FullGpuRuntimeMode::GpuStaticPlasticCpuShadowGuarded,
        )?;
        let gpu_available = static_report.backend.fallback_reason.is_none();
        let mut gpu_scores_used = false;
        let mut plasticity_applied = false;
        let mut last_delta = self.telemetry.last_h_shadow_delta;
        let mut post_seal_readback = 0_usize;
        let mut product_claim = format!("{:?}", static_report.product_runtime_claim);

        let proposals = if gpu_available
            && static_report.action_summary.is_some()
            && static_report.cpu_shadow_parity_passed
            && matches!(
                static_report.product_runtime_claim,
                FullGpuRuntimeProductClaim::CpuShadowGuarded
                    | FullGpuRuntimeProductClaim::CpuShadowGuardedStaticPlusLiveHShadow
            ) {
            gpu_scores_used = true;
            let action_summary = static_report.action_summary.clone().ok_or(
                GameAppShellError::VisibleWorldMismatch {
                    message: "graphical GPU scoring reported parity without action summary",
                },
            )?;
            live.current_context_proposals_with_scores(scores_from_action_summary(action_summary)?)?
        } else {
            live.current_context_proposals()?
        };

        if gpu_available && !self.h_shadow_applied_once {
            live.initialize_neural_projection_schema(full_gpu_runtime_live_plasticity_schema()?)?;
        }
        let tick = live.tick_with_proposals_detailed(proposals, !gpu_available);

        if gpu_available
            && gpu_scores_used
            && tick.summary.patch_sealed
            && !self.h_shadow_applied_once
            && post_seal_gpu_plasticity_diagnostic_enabled()
        {
            if let Some(patch) = tick.sealed_patch.as_ref() {
                let plasticity_report =
                    run_full_gpu_runtime_post_seal_plasticity_diagnostic(input)?;
                post_seal_readback = plasticity_report.diagnostic_readback_bytes;
                let batch =
                    post_seal_delta_batch_from_plasticity_report(patch, &plasticity_report)?;
                let receipt = live.apply_post_seal_lifetime_deltas(patch, batch)?;
                plasticity_applied = true;
                self.h_shadow_applied_once = true;
                last_delta = receipt.max_abs_delta;
                product_claim = "CpuShadowGuardedStaticPlusLiveHShadow".to_string();
            }
        }

        let h_shadow_applications =
            self.telemetry.h_shadow_applications + u32::from(plasticity_applied);
        if h_shadow_applications > 0 && gpu_scores_used && static_report.cpu_shadow_parity_passed {
            product_claim = "CpuShadowGuardedStaticPlusLiveHShadow".to_string();
        }

        self.telemetry = GraphicalGpuRuntimeTelemetry {
            requested_mode: self.mode,
            selected_backend: format!("{:?}", static_report.backend.selected),
            fallback_reason: static_report
                .backend
                .fallback_reason
                .map(|reason| format!("{reason:?}")),
            hardware_identifier: static_report.hardware_identifier.clone(),
            product_runtime_claim: product_claim,
            gpu_static_dispatched_ticks: self
                .telemetry
                .gpu_static_dispatched_ticks
                .saturating_add(u32::from(gpu_available)),
            gpu_scores_used_for_proposals: gpu_scores_used,
            cpu_shadow_parity: static_report.cpu_shadow_parity_passed,
            parity_failures: self.telemetry.parity_failures
                + u32::from(!static_report.cpu_shadow_parity_passed),
            sealed_patches: tick.summary.sealed_patch_count,
            h_shadow_applications,
            last_h_shadow_delta: last_delta,
            compact_readback_bytes: static_report.readback.compact_readback_bytes,
            post_seal_readback_bytes: post_seal_readback,
            total_gpu_runtime_ms: static_report.timing.total_gpu_runtime_ms,
            no_active_bulk_readback: static_report.readback.bulk_neural_readback_forbidden,
            full_action_authoritative_claim: false,
        };
        self.telemetry.validate()?;
        Ok(tick.summary)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuLongrunSoakOptions {
    pub ticks: u32,
    pub report_every: u32,
    pub stop_on_first_parity_failure: bool,
    pub stop_on_first_hshadow_rejection: bool,
    pub json_path: Option<PathBuf>,
}

impl Default for GpuLongrunSoakOptions {
    fn default() -> Self {
        Self {
            ticks: GPU_LONGRUN_SOAK_DEFAULT_TICKS,
            report_every: GPU_LONGRUN_SOAK_DEFAULT_REPORT_EVERY,
            stop_on_first_parity_failure: true,
            stop_on_first_hshadow_rejection: true,
            json_path: None,
        }
    }
}

impl GpuLongrunSoakOptions {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.ticks == 0
            || self.ticks > GPU_LONGRUN_SOAK_MAX_TICKS_MANUAL
            || self.report_every == 0
            || self.report_every > GPU_LONGRUN_SOAK_MAX_TICKS_MANUAL
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU long-run soak options must stay within manual bounds",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuLongrunSoakSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_ticks: u32,
    pub ticks_completed: u32,
    pub report_every: u32,
    pub selected_backend: String,
    pub fallback_reason: Option<String>,
    pub hardware_identifier: Option<String>,
    pub sealed_patches: usize,
    pub packed_logs: usize,
    pub gpu_static_dispatched_ticks: u32,
    pub gpu_proposal_ticks: u32,
    pub cpu_shadow_parity_checks: u32,
    pub parity_failures: u32,
    pub first_parity_failure_tick: Option<u32>,
    pub h_shadow_applications: u32,
    pub h_shadow_rejected_applications: u32,
    pub first_h_shadow_rejection_tick: Option<u32>,
    pub total_h_shadow_records_applied: u32,
    pub max_h_shadow_abs_delta: f32,
    pub w_genetic_fixed_unchanged: bool,
    pub lifetime_consolidated_unchanged: bool,
    pub h_operational_unchanged: bool,
    pub compact_active_readback_bytes: usize,
    pub post_seal_readback_bytes: usize,
    pub no_active_bulk_readback: bool,
    pub total_upload_ms: f32,
    pub total_submit_poll_ms: f32,
    pub total_compact_readback_ms: f32,
    pub total_post_seal_readback_ms: f32,
    pub total_cpu_shadow_ms: f32,
    pub total_wall_ms: f32,
    pub average_ms_per_tick: f32,
    pub ticks_per_second: f32,
    pub product_runtime_claim: String,
    pub full_action_authoritative_claim: bool,
    pub stop_on_first_parity_failure: bool,
    pub stop_on_first_hshadow_rejection: bool,
}

impl GpuLongrunSoakSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != GPU_LONGRUN_SOAK_SCHEMA
            || self.schema_version != GPU_LONGRUN_SOAK_SCHEMA_VERSION
            || self.requested_ticks == 0
            || self.requested_ticks > GPU_LONGRUN_SOAK_MAX_TICKS_MANUAL
            || self.ticks_completed == 0
            || self.ticks_completed > self.requested_ticks
            || !self.no_active_bulk_readback
            || self.full_action_authoritative_claim
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU long-run soak summary violated manual evidence bounds",
            });
        }
        for value in [
            self.max_h_shadow_abs_delta,
            self.total_upload_ms,
            self.total_submit_poll_ms,
            self.total_compact_readback_ms,
            self.total_post_seal_readback_ms,
            self.total_cpu_shadow_ms,
            self.total_wall_ms,
            self.average_ms_per_tick,
            self.ticks_per_second,
        ] {
            alife_core::validate_finite(value)?;
        }
        if self.selected_backend == "CpuReference"
            && (self.gpu_static_dispatched_ticks > 0
                || self.gpu_proposal_ticks > 0
                || self.h_shadow_applications > 0
                || self.product_runtime_claim != "None")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CPU fallback soak must not claim GPU work",
            });
        }
        if self.product_runtime_claim == "CpuShadowGuardedStaticPlusLiveHShadow"
            && (self.gpu_proposal_ticks == 0 || self.h_shadow_applications == 0)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "combined soak claim requires GPU proposal ticks and H_shadow receipt",
            });
        }
        if self.parity_failures > 0 && self.first_parity_failure_tick.is_none() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "parity failures must record first failure tick",
            });
        }
        if self.h_shadow_rejected_applications > 0 && self.first_h_shadow_rejection_tick.is_none() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "H_shadow rejection must record first failure tick",
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuSustainedLearningSoakOptions {
    pub ticks: u32,
    pub report_every: u32,
    pub episode_ticks: u32,
    pub stop_on_first_parity_failure: bool,
    pub stop_on_first_hshadow_rejection: bool,
    pub json_path: Option<PathBuf>,
}

impl Default for GpuSustainedLearningSoakOptions {
    fn default() -> Self {
        Self {
            ticks: GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_TICKS,
            report_every: GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_REPORT_EVERY,
            episode_ticks: GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_EPISODE_TICKS,
            stop_on_first_parity_failure: true,
            stop_on_first_hshadow_rejection: true,
            json_path: None,
        }
    }
}

impl GpuSustainedLearningSoakOptions {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.ticks == 0
            || self.ticks > GPU_SUSTAINED_LEARNING_SOAK_MAX_TICKS_MANUAL
            || self.report_every == 0
            || self.report_every > GPU_SUSTAINED_LEARNING_SOAK_MAX_TICKS_MANUAL
            || self.episode_ticks == 0
            || self.episode_ticks > GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_EPISODE_TICKS
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU sustained-learning soak options must stay within manual bounds",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSustainedLearningSoakSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_ticks: u32,
    pub ticks_completed: u32,
    pub report_every: u32,
    pub episode_ticks: u32,
    pub episodes: u32,
    pub selected_backend: String,
    pub fallback_reason: Option<String>,
    pub hardware_identifier: Option<String>,
    pub sealed_patches_total: usize,
    pub packed_logs_total: usize,
    pub gpu_static_dispatched_ticks: u32,
    pub gpu_proposal_ticks: u32,
    pub cpu_shadow_parity_checks: u32,
    pub parity_failures: u32,
    pub first_parity_failure_tick: Option<u32>,
    pub h_shadow_application_attempts: u32,
    pub h_shadow_applications_succeeded: u32,
    pub h_shadow_applications_rejected: u32,
    pub first_h_shadow_rejection_tick: Option<u32>,
    pub total_h_shadow_records_applied: u32,
    pub max_h_shadow_abs_delta: f32,
    pub replay_protection_active: bool,
    pub repeated_learning_uses_episode_rotation: bool,
    pub w_genetic_fixed_unchanged: bool,
    pub lifetime_consolidated_unchanged: bool,
    pub h_operational_unchanged: bool,
    pub compact_active_readback_bytes: usize,
    pub post_seal_readback_bytes: usize,
    pub no_active_bulk_readback: bool,
    pub total_upload_ms: f32,
    pub total_submit_poll_ms: f32,
    pub total_compact_readback_ms: f32,
    pub total_post_seal_readback_ms: f32,
    pub total_cpu_shadow_ms: f32,
    pub total_wall_ms: f32,
    pub average_ms_per_tick: f32,
    pub ticks_per_second: f32,
    pub product_runtime_claim: String,
    pub full_action_authoritative_claim: bool,
    pub stop_on_first_parity_failure: bool,
    pub stop_on_first_hshadow_rejection: bool,
}

impl GpuSustainedLearningSoakSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != GPU_SUSTAINED_LEARNING_SOAK_SCHEMA
            || self.schema_version != GPU_SUSTAINED_LEARNING_SOAK_SCHEMA_VERSION
            || self.requested_ticks == 0
            || self.requested_ticks > GPU_SUSTAINED_LEARNING_SOAK_MAX_TICKS_MANUAL
            || self.ticks_completed == 0
            || self.ticks_completed > self.requested_ticks
            || self.episode_ticks == 0
            || self.episode_ticks > GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_EPISODE_TICKS
            || self.episodes == 0
            || !self.no_active_bulk_readback
            || self.full_action_authoritative_claim
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU sustained-learning soak summary violated manual evidence bounds",
            });
        }
        for value in [
            self.max_h_shadow_abs_delta,
            self.total_upload_ms,
            self.total_submit_poll_ms,
            self.total_compact_readback_ms,
            self.total_post_seal_readback_ms,
            self.total_cpu_shadow_ms,
            self.total_wall_ms,
            self.average_ms_per_tick,
            self.ticks_per_second,
        ] {
            alife_core::validate_finite(value)?;
        }
        if self.selected_backend == "CpuReference"
            && (self.gpu_static_dispatched_ticks > 0
                || self.gpu_proposal_ticks > 0
                || self.h_shadow_applications_succeeded > 0
                || self.product_runtime_claim != "None")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CPU fallback sustained soak must not claim GPU work",
            });
        }
        if self.product_runtime_claim == "CpuShadowGuardedStaticPlusLiveHShadow"
            && (self.gpu_proposal_ticks == 0
                || self.h_shadow_applications_succeeded == 0
                || self.total_h_shadow_records_applied == 0)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "sustained soak claim requires GPU proposals and applied H_shadow records",
            });
        }
        if self.parity_failures > 0 && self.first_parity_failure_tick.is_none() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "parity failures must record first failure tick",
            });
        }
        if self.h_shadow_applications_rejected > 0 && self.first_h_shadow_rejection_tick.is_none() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "H_shadow rejection must record first failure tick",
            });
        }
        if self.h_shadow_applications_succeeded > self.h_shadow_application_attempts {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "H_shadow successes cannot exceed attempts",
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
pub fn run_gpu_longrun_soak(
    launch: &AppShellLaunchConfig,
    options: GpuLongrunSoakOptions,
) -> Result<GpuLongrunSoakSummary, GameAppShellError> {
    options.validate()?;
    let wall_start = std::time::Instant::now();
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut sealed_patches = 0_usize;
    let mut packed_logs = 0_usize;
    for _ in 0..options.ticks {
        let proposals = live.current_context_proposals()?;
        let tick = live.tick_with_proposals_detailed(proposals, true);
        sealed_patches = tick.summary.sealed_patch_count;
        packed_logs = tick.summary.packed_record_count;
    }
    let wall_ms = elapsed_ms_app(wall_start);
    let summary = GpuLongrunSoakSummary {
        schema: GPU_LONGRUN_SOAK_SCHEMA,
        schema_version: GPU_LONGRUN_SOAK_SCHEMA_VERSION,
        requested_ticks: options.ticks,
        ticks_completed: options.ticks,
        report_every: options.report_every,
        selected_backend: "CpuReference".to_string(),
        fallback_reason: Some("FeatureDisabled".to_string()),
        hardware_identifier: None,
        sealed_patches,
        packed_logs,
        gpu_static_dispatched_ticks: 0,
        gpu_proposal_ticks: 0,
        cpu_shadow_parity_checks: 0,
        parity_failures: 0,
        first_parity_failure_tick: None,
        h_shadow_applications: 0,
        h_shadow_rejected_applications: 0,
        first_h_shadow_rejection_tick: None,
        total_h_shadow_records_applied: 0,
        max_h_shadow_abs_delta: 0.0,
        w_genetic_fixed_unchanged: true,
        lifetime_consolidated_unchanged: true,
        h_operational_unchanged: true,
        compact_active_readback_bytes: 0,
        post_seal_readback_bytes: 0,
        no_active_bulk_readback: true,
        total_upload_ms: 0.0,
        total_submit_poll_ms: 0.0,
        total_compact_readback_ms: 0.0,
        total_post_seal_readback_ms: 0.0,
        total_cpu_shadow_ms: 0.0,
        total_wall_ms: wall_ms,
        average_ms_per_tick: wall_ms / options.ticks as f32,
        ticks_per_second: ticks_per_second(options.ticks, wall_ms),
        product_runtime_claim: "None".to_string(),
        full_action_authoritative_claim: false,
        stop_on_first_parity_failure: options.stop_on_first_parity_failure,
        stop_on_first_hshadow_rejection: options.stop_on_first_hshadow_rejection,
    };
    summary.validate()?;
    if let Some(path) = options.json_path {
        summary.write_json(path)?;
    }
    Ok(summary)
}

#[cfg(feature = "gpu-runtime")]
pub fn run_gpu_longrun_soak(
    launch: &AppShellLaunchConfig,
    options: GpuLongrunSoakOptions,
) -> Result<GpuLongrunSoakSummary, GameAppShellError> {
    use alife_gpu_backend::{
        full_gpu_runtime_live_plasticity_schema, post_seal_delta_batch_from_plasticity_report,
        FullGpuRuntimeProductClaim, FullGpuRuntimeSession,
    };

    options.validate()?;
    let wall_start = std::time::Instant::now();
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mode = FullGpuRuntimeSmokeMode::StaticPlasticCpuShadowGuarded;
    let session = FullGpuRuntimeSession::new(backend_mode(mode))?;
    let mut schema_initialized = false;
    let mut plasticity_attempted = false;

    let mut selected_backend = "CpuReference".to_string();
    let mut fallback_reason = None;
    let mut hardware_identifier = None;
    let mut sealed_patches = 0_usize;
    let mut packed_logs = 0_usize;
    let mut gpu_static_dispatched_ticks = 0_u32;
    let mut gpu_proposal_ticks = 0_u32;
    let mut cpu_shadow_parity_checks = 0_u32;
    let mut parity_failures = 0_u32;
    let mut first_parity_failure_tick = None;
    let mut h_shadow_applications = 0_u32;
    let mut h_shadow_rejected_applications = 0_u32;
    let mut first_h_shadow_rejection_tick = None;
    let mut total_h_shadow_records_applied = 0_u32;
    let mut max_h_shadow_abs_delta = 0.0_f32;
    let mut w_genetic_fixed_unchanged = true;
    let mut lifetime_consolidated_unchanged = true;
    let mut h_operational_unchanged = true;
    let mut compact_active_readback_bytes = 0_usize;
    let mut post_seal_readback_bytes = 0_usize;
    let mut total_upload_ms = 0.0_f32;
    let mut total_submit_poll_ms = 0.0_f32;
    let mut total_compact_readback_ms = 0.0_f32;
    let mut total_post_seal_readback_ms = 0.0_f32;
    let mut total_cpu_shadow_ms = 0.0_f32;
    let mut ticks_completed = 0_u32;

    for tick_index in 0..options.ticks {
        let tick_number = tick_index.saturating_add(1);
        let report = live.current_sensory_report()?;
        let input = runtime_input_from_sensory(&report)?;
        let static_report = session.run_static_tick(input)?;
        if tick_index == 0 {
            selected_backend = format!("{:?}", static_report.backend.selected);
            fallback_reason = static_report
                .backend
                .fallback_reason
                .map(|reason| format!("{reason:?}"));
            hardware_identifier = static_report.hardware_identifier.clone();
        }
        total_upload_ms += static_report.timing.upload_ms;
        total_submit_poll_ms += static_report.timing.gpu_submit_poll_ms;
        total_compact_readback_ms += static_report.timing.compact_readback_ms;
        total_cpu_shadow_ms += static_report.timing.cpu_shadow_ms;
        compact_active_readback_bytes = compact_active_readback_bytes
            .saturating_add(static_report.readback.compact_readback_bytes);
        if static_report.action_summary.is_some() {
            gpu_static_dispatched_ticks = gpu_static_dispatched_ticks.saturating_add(1);
        }
        cpu_shadow_parity_checks = cpu_shadow_parity_checks.saturating_add(1);
        let gpu_static_available = static_report.action_summary.is_some()
            && static_report.backend.fallback_reason.is_none();
        if gpu_static_available && !schema_initialized {
            live.initialize_neural_projection_schema(full_gpu_runtime_live_plasticity_schema()?)?;
            schema_initialized = true;
        }
        let use_gpu_scores = static_report.action_summary.is_some()
            && static_report.cpu_shadow_parity_passed
            && matches!(
                static_report.product_runtime_claim,
                FullGpuRuntimeProductClaim::CpuShadowGuarded
                    | FullGpuRuntimeProductClaim::CpuShadowGuardedStaticPlusLiveHShadow
                    | FullGpuRuntimeProductClaim::ActionAuthoritative
            );
        if gpu_static_available && !static_report.cpu_shadow_parity_passed {
            parity_failures = parity_failures.saturating_add(1);
            first_parity_failure_tick.get_or_insert(tick_number);
        }
        let proposals = if use_gpu_scores {
            gpu_proposal_ticks = gpu_proposal_ticks.saturating_add(1);
            let action_summary =
                static_report
                    .action_summary
                    .ok_or(GameAppShellError::VisibleWorldMismatch {
                        message: "GPU static scoring reported parity without an action summary",
                    })?;
            live.current_context_proposals_with_scores(scores_from_action_summary(action_summary)?)?
        } else {
            live.current_context_proposals()?
        };
        let tick = live.tick_with_proposals_detailed(proposals, !gpu_static_available);
        sealed_patches = tick.summary.sealed_patch_count;
        packed_logs = tick.summary.packed_record_count;
        ticks_completed = tick_number;

        if gpu_static_available
            && !static_report.cpu_shadow_parity_passed
            && options.stop_on_first_parity_failure
        {
            break;
        }

        if tick.summary.patch_sealed
            && gpu_static_available
            && !plasticity_attempted
            && post_seal_gpu_plasticity_diagnostic_enabled()
        {
            plasticity_attempted = true;
            match session.run_post_seal_plasticity_diagnostic(input) {
                Ok(plasticity_report) => {
                    total_submit_poll_ms += plasticity_report.submit_poll_ms;
                    total_post_seal_readback_ms += plasticity_report.diagnostic_readback_ms;
                    post_seal_readback_bytes = post_seal_readback_bytes
                        .saturating_add(plasticity_report.diagnostic_readback_bytes);
                    w_genetic_fixed_unchanged &= plasticity_report.genetic_fixed_unchanged;
                    lifetime_consolidated_unchanged &=
                        plasticity_report.lifetime_consolidated_unchanged;
                    h_operational_unchanged &= plasticity_report.h_operational_unchanged;
                    if let Some(patch) = tick.sealed_patch.as_ref() {
                        match post_seal_delta_batch_from_plasticity_report(
                            patch,
                            &plasticity_report,
                        ) {
                            Ok(batch) => match live.apply_post_seal_lifetime_deltas(patch, batch) {
                                Ok(receipt) => {
                                    h_shadow_applications = h_shadow_applications.saturating_add(1);
                                    total_h_shadow_records_applied = total_h_shadow_records_applied
                                        .saturating_add(receipt.applied_records);
                                    max_h_shadow_abs_delta =
                                        max_h_shadow_abs_delta.max(receipt.max_abs_delta);
                                }
                                Err(_) => {
                                    h_shadow_rejected_applications =
                                        h_shadow_rejected_applications.saturating_add(1);
                                    first_h_shadow_rejection_tick.get_or_insert(tick_number);
                                    if options.stop_on_first_hshadow_rejection {
                                        break;
                                    }
                                }
                            },
                            Err(_) => {
                                h_shadow_rejected_applications =
                                    h_shadow_rejected_applications.saturating_add(1);
                                first_h_shadow_rejection_tick.get_or_insert(tick_number);
                                if options.stop_on_first_hshadow_rejection {
                                    break;
                                }
                            }
                        };
                    } else {
                        h_shadow_rejected_applications =
                            h_shadow_rejected_applications.saturating_add(1);
                        first_h_shadow_rejection_tick.get_or_insert(tick_number);
                        if options.stop_on_first_hshadow_rejection {
                            break;
                        }
                    }
                }
                Err(_) => {
                    h_shadow_rejected_applications =
                        h_shadow_rejected_applications.saturating_add(1);
                    first_h_shadow_rejection_tick.get_or_insert(tick_number);
                    if options.stop_on_first_hshadow_rejection {
                        break;
                    }
                }
            }
        }
    }

    let wall_ms = elapsed_ms_app(wall_start);
    let product_runtime_claim = if selected_backend == "CpuReference" {
        "None"
    } else if gpu_proposal_ticks > 0 && h_shadow_applications > 0 && parity_failures == 0 {
        "CpuShadowGuardedStaticPlusLiveHShadow"
    } else if gpu_proposal_ticks > 0 {
        "CpuShadowGuarded"
    } else {
        "None"
    }
    .to_string();
    let summary = GpuLongrunSoakSummary {
        schema: GPU_LONGRUN_SOAK_SCHEMA,
        schema_version: GPU_LONGRUN_SOAK_SCHEMA_VERSION,
        requested_ticks: options.ticks,
        ticks_completed,
        report_every: options.report_every,
        selected_backend,
        fallback_reason,
        hardware_identifier,
        sealed_patches,
        packed_logs,
        gpu_static_dispatched_ticks,
        gpu_proposal_ticks,
        cpu_shadow_parity_checks,
        parity_failures,
        first_parity_failure_tick,
        h_shadow_applications,
        h_shadow_rejected_applications,
        first_h_shadow_rejection_tick,
        total_h_shadow_records_applied,
        max_h_shadow_abs_delta,
        w_genetic_fixed_unchanged,
        lifetime_consolidated_unchanged,
        h_operational_unchanged,
        compact_active_readback_bytes,
        post_seal_readback_bytes,
        no_active_bulk_readback: true,
        total_upload_ms,
        total_submit_poll_ms,
        total_compact_readback_ms,
        total_post_seal_readback_ms,
        total_cpu_shadow_ms,
        total_wall_ms: wall_ms,
        average_ms_per_tick: wall_ms / ticks_completed.max(1) as f32,
        ticks_per_second: ticks_per_second(ticks_completed, wall_ms),
        product_runtime_claim,
        full_action_authoritative_claim: false,
        stop_on_first_parity_failure: options.stop_on_first_parity_failure,
        stop_on_first_hshadow_rejection: options.stop_on_first_hshadow_rejection,
    };
    summary.validate()?;
    if let Some(path) = options.json_path {
        summary.write_json(path)?;
    }
    Ok(summary)
}

#[cfg(not(feature = "gpu-runtime"))]
pub fn run_gpu_sustained_learning_soak(
    launch: &AppShellLaunchConfig,
    options: GpuSustainedLearningSoakOptions,
) -> Result<GpuSustainedLearningSoakSummary, GameAppShellError> {
    options.validate()?;
    let wall_start = std::time::Instant::now();
    let mut ticks_completed = 0_u32;
    let mut episodes = 0_u32;
    let mut sealed_patches_total = 0_usize;
    let mut packed_logs_total = 0_usize;
    while ticks_completed < options.ticks {
        episodes = episodes.saturating_add(1);
        let mut live = LiveBrainLoop::from_p34_launch(launch)?;
        let mut previous_sealed = 0_usize;
        let mut previous_packed = 0_usize;
        let remaining = options.ticks.saturating_sub(ticks_completed);
        let episode_ticks = options.episode_ticks.min(remaining);
        for _ in 0..episode_ticks {
            let proposals = live.current_context_proposals()?;
            let tick = live.tick_with_proposals_detailed(proposals, true);
            sealed_patches_total = sealed_patches_total.saturating_add(
                tick.summary
                    .sealed_patch_count
                    .saturating_sub(previous_sealed),
            );
            packed_logs_total = packed_logs_total.saturating_add(
                tick.summary
                    .packed_record_count
                    .saturating_sub(previous_packed),
            );
            previous_sealed = tick.summary.sealed_patch_count;
            previous_packed = tick.summary.packed_record_count;
            ticks_completed = ticks_completed.saturating_add(1);
        }
    }
    let wall_ms = elapsed_ms_app(wall_start);
    let summary = GpuSustainedLearningSoakSummary {
        schema: GPU_SUSTAINED_LEARNING_SOAK_SCHEMA,
        schema_version: GPU_SUSTAINED_LEARNING_SOAK_SCHEMA_VERSION,
        requested_ticks: options.ticks,
        ticks_completed,
        report_every: options.report_every,
        episode_ticks: options.episode_ticks,
        episodes,
        selected_backend: "CpuReference".to_string(),
        fallback_reason: Some("FeatureDisabled".to_string()),
        hardware_identifier: None,
        sealed_patches_total,
        packed_logs_total,
        gpu_static_dispatched_ticks: 0,
        gpu_proposal_ticks: 0,
        cpu_shadow_parity_checks: 0,
        parity_failures: 0,
        first_parity_failure_tick: None,
        h_shadow_application_attempts: 0,
        h_shadow_applications_succeeded: 0,
        h_shadow_applications_rejected: 0,
        first_h_shadow_rejection_tick: None,
        total_h_shadow_records_applied: 0,
        max_h_shadow_abs_delta: 0.0,
        replay_protection_active: true,
        repeated_learning_uses_episode_rotation: true,
        w_genetic_fixed_unchanged: true,
        lifetime_consolidated_unchanged: true,
        h_operational_unchanged: true,
        compact_active_readback_bytes: 0,
        post_seal_readback_bytes: 0,
        no_active_bulk_readback: true,
        total_upload_ms: 0.0,
        total_submit_poll_ms: 0.0,
        total_compact_readback_ms: 0.0,
        total_post_seal_readback_ms: 0.0,
        total_cpu_shadow_ms: 0.0,
        total_wall_ms: wall_ms,
        average_ms_per_tick: wall_ms / ticks_completed.max(1) as f32,
        ticks_per_second: ticks_per_second(ticks_completed, wall_ms),
        product_runtime_claim: "None".to_string(),
        full_action_authoritative_claim: false,
        stop_on_first_parity_failure: options.stop_on_first_parity_failure,
        stop_on_first_hshadow_rejection: options.stop_on_first_hshadow_rejection,
    };
    summary.validate()?;
    if let Some(path) = options.json_path {
        summary.write_json(path)?;
    }
    Ok(summary)
}

#[cfg(feature = "gpu-runtime")]
pub fn run_gpu_sustained_learning_soak(
    launch: &AppShellLaunchConfig,
    options: GpuSustainedLearningSoakOptions,
) -> Result<GpuSustainedLearningSoakSummary, GameAppShellError> {
    use alife_gpu_backend::{
        full_gpu_runtime_live_plasticity_schema, post_seal_delta_batch_from_plasticity_report,
        FullGpuRuntimeProductClaim, FullGpuRuntimeSession,
    };

    options.validate()?;
    let wall_start = std::time::Instant::now();
    let mode = FullGpuRuntimeSmokeMode::StaticPlasticCpuShadowGuarded;
    let session = FullGpuRuntimeSession::new(backend_mode(mode))?;

    let mut selected_backend = "CpuReference".to_string();
    let mut fallback_reason = None;
    let mut hardware_identifier = None;
    let mut sealed_patches_total = 0_usize;
    let mut packed_logs_total = 0_usize;
    let mut gpu_static_dispatched_ticks = 0_u32;
    let mut gpu_proposal_ticks = 0_u32;
    let mut cpu_shadow_parity_checks = 0_u32;
    let mut parity_failures = 0_u32;
    let mut first_parity_failure_tick = None;
    let mut h_shadow_application_attempts = 0_u32;
    let mut h_shadow_applications_succeeded = 0_u32;
    let mut h_shadow_applications_rejected = 0_u32;
    let mut first_h_shadow_rejection_tick = None;
    let mut total_h_shadow_records_applied = 0_u32;
    let mut max_h_shadow_abs_delta = 0.0_f32;
    let mut w_genetic_fixed_unchanged = true;
    let mut lifetime_consolidated_unchanged = true;
    let mut h_operational_unchanged = true;
    let mut compact_active_readback_bytes = 0_usize;
    let mut post_seal_readback_bytes = 0_usize;
    let mut total_upload_ms = 0.0_f32;
    let mut total_submit_poll_ms = 0.0_f32;
    let mut total_compact_readback_ms = 0.0_f32;
    let mut total_post_seal_readback_ms = 0.0_f32;
    let mut total_cpu_shadow_ms = 0.0_f32;
    let mut ticks_completed = 0_u32;
    let mut episodes = 0_u32;
    let mut stop_soak = false;

    while ticks_completed < options.ticks && !stop_soak {
        episodes = episodes.saturating_add(1);
        let mut live = LiveBrainLoop::from_p34_launch(launch)?;
        let mut schema_initialized = false;
        let mut plasticity_attempted = false;
        let mut previous_sealed = 0_usize;
        let mut previous_packed = 0_usize;
        let remaining = options.ticks.saturating_sub(ticks_completed);
        let episode_ticks = options.episode_ticks.min(remaining);

        for _ in 0..episode_ticks {
            let tick_number = ticks_completed.saturating_add(1);
            let report = live.current_sensory_report()?;
            let input = runtime_input_from_sensory(&report)?;
            let static_report = session.run_static_tick(input)?;
            if tick_number == 1 {
                selected_backend = format!("{:?}", static_report.backend.selected);
                fallback_reason = static_report
                    .backend
                    .fallback_reason
                    .map(|reason| format!("{reason:?}"));
                hardware_identifier = static_report.hardware_identifier.clone();
            }
            total_upload_ms += static_report.timing.upload_ms;
            total_submit_poll_ms += static_report.timing.gpu_submit_poll_ms;
            total_compact_readback_ms += static_report.timing.compact_readback_ms;
            total_cpu_shadow_ms += static_report.timing.cpu_shadow_ms;
            compact_active_readback_bytes = compact_active_readback_bytes
                .saturating_add(static_report.readback.compact_readback_bytes);
            if static_report.action_summary.is_some() {
                gpu_static_dispatched_ticks = gpu_static_dispatched_ticks.saturating_add(1);
            }
            cpu_shadow_parity_checks = cpu_shadow_parity_checks.saturating_add(1);
            let gpu_static_available = static_report.action_summary.is_some()
                && static_report.backend.fallback_reason.is_none();
            if gpu_static_available && !schema_initialized {
                live.initialize_neural_projection_schema(
                    full_gpu_runtime_live_plasticity_schema()?
                )?;
                schema_initialized = true;
            }
            let use_gpu_scores = static_report.action_summary.is_some()
                && static_report.cpu_shadow_parity_passed
                && matches!(
                    static_report.product_runtime_claim,
                    FullGpuRuntimeProductClaim::CpuShadowGuarded
                        | FullGpuRuntimeProductClaim::CpuShadowGuardedStaticPlusLiveHShadow
                        | FullGpuRuntimeProductClaim::ActionAuthoritative
                );
            if gpu_static_available && !static_report.cpu_shadow_parity_passed {
                parity_failures = parity_failures.saturating_add(1);
                first_parity_failure_tick.get_or_insert(tick_number);
            }
            let proposals = if use_gpu_scores {
                gpu_proposal_ticks = gpu_proposal_ticks.saturating_add(1);
                let action_summary = static_report.action_summary.ok_or(
                    GameAppShellError::VisibleWorldMismatch {
                        message: "GPU static scoring reported parity without an action summary",
                    },
                )?;
                live.current_context_proposals_with_scores(scores_from_action_summary(
                    action_summary,
                )?)?
            } else {
                live.current_context_proposals()?
            };
            let tick = live.tick_with_proposals_detailed(proposals, !gpu_static_available);
            sealed_patches_total = sealed_patches_total.saturating_add(
                tick.summary
                    .sealed_patch_count
                    .saturating_sub(previous_sealed),
            );
            packed_logs_total = packed_logs_total.saturating_add(
                tick.summary
                    .packed_record_count
                    .saturating_sub(previous_packed),
            );
            previous_sealed = tick.summary.sealed_patch_count;
            previous_packed = tick.summary.packed_record_count;
            ticks_completed = tick_number;

            if gpu_static_available
                && !static_report.cpu_shadow_parity_passed
                && options.stop_on_first_parity_failure
            {
                stop_soak = true;
                break;
            }

            if tick.summary.patch_sealed
                && gpu_static_available
                && !plasticity_attempted
                && post_seal_gpu_plasticity_diagnostic_enabled()
            {
                plasticity_attempted = true;
                h_shadow_application_attempts = h_shadow_application_attempts.saturating_add(1);
                match session.run_post_seal_plasticity_diagnostic(input) {
                    Ok(plasticity_report) => {
                        total_submit_poll_ms += plasticity_report.submit_poll_ms;
                        total_post_seal_readback_ms += plasticity_report.diagnostic_readback_ms;
                        post_seal_readback_bytes = post_seal_readback_bytes
                            .saturating_add(plasticity_report.diagnostic_readback_bytes);
                        w_genetic_fixed_unchanged &= plasticity_report.genetic_fixed_unchanged;
                        lifetime_consolidated_unchanged &=
                            plasticity_report.lifetime_consolidated_unchanged;
                        h_operational_unchanged &= plasticity_report.h_operational_unchanged;
                        if let Some(patch) = tick.sealed_patch.as_ref() {
                            match post_seal_delta_batch_from_plasticity_report(
                                patch,
                                &plasticity_report,
                            ) {
                                Ok(batch) => {
                                    match live.apply_post_seal_lifetime_deltas(patch, batch) {
                                        Ok(receipt) => {
                                            h_shadow_applications_succeeded =
                                                h_shadow_applications_succeeded.saturating_add(1);
                                            total_h_shadow_records_applied =
                                                total_h_shadow_records_applied
                                                    .saturating_add(receipt.applied_records);
                                            max_h_shadow_abs_delta =
                                                max_h_shadow_abs_delta.max(receipt.max_abs_delta);
                                        }
                                        Err(_) => {
                                            h_shadow_applications_rejected =
                                                h_shadow_applications_rejected.saturating_add(1);
                                            first_h_shadow_rejection_tick
                                                .get_or_insert(tick_number);
                                            if options.stop_on_first_hshadow_rejection {
                                                stop_soak = true;
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    h_shadow_applications_rejected =
                                        h_shadow_applications_rejected.saturating_add(1);
                                    first_h_shadow_rejection_tick.get_or_insert(tick_number);
                                    if options.stop_on_first_hshadow_rejection {
                                        stop_soak = true;
                                    }
                                }
                            };
                        } else {
                            h_shadow_applications_rejected =
                                h_shadow_applications_rejected.saturating_add(1);
                            first_h_shadow_rejection_tick.get_or_insert(tick_number);
                            if options.stop_on_first_hshadow_rejection {
                                stop_soak = true;
                            }
                        }
                    }
                    Err(_) => {
                        h_shadow_applications_rejected =
                            h_shadow_applications_rejected.saturating_add(1);
                        first_h_shadow_rejection_tick.get_or_insert(tick_number);
                        if options.stop_on_first_hshadow_rejection {
                            stop_soak = true;
                        }
                    }
                }
            }
            if stop_soak {
                break;
            }
        }
    }

    let wall_ms = elapsed_ms_app(wall_start);
    let product_runtime_claim = if selected_backend == "CpuReference" {
        "None"
    } else if gpu_proposal_ticks > 0 && h_shadow_applications_succeeded > 0 && parity_failures == 0
    {
        "CpuShadowGuardedStaticPlusLiveHShadow"
    } else if gpu_proposal_ticks > 0 {
        "CpuShadowGuarded"
    } else {
        "None"
    }
    .to_string();
    let summary = GpuSustainedLearningSoakSummary {
        schema: GPU_SUSTAINED_LEARNING_SOAK_SCHEMA,
        schema_version: GPU_SUSTAINED_LEARNING_SOAK_SCHEMA_VERSION,
        requested_ticks: options.ticks,
        ticks_completed,
        report_every: options.report_every,
        episode_ticks: options.episode_ticks,
        episodes,
        selected_backend,
        fallback_reason,
        hardware_identifier,
        sealed_patches_total,
        packed_logs_total,
        gpu_static_dispatched_ticks,
        gpu_proposal_ticks,
        cpu_shadow_parity_checks,
        parity_failures,
        first_parity_failure_tick,
        h_shadow_application_attempts,
        h_shadow_applications_succeeded,
        h_shadow_applications_rejected,
        first_h_shadow_rejection_tick,
        total_h_shadow_records_applied,
        max_h_shadow_abs_delta,
        replay_protection_active: true,
        repeated_learning_uses_episode_rotation: true,
        w_genetic_fixed_unchanged,
        lifetime_consolidated_unchanged,
        h_operational_unchanged,
        compact_active_readback_bytes,
        post_seal_readback_bytes,
        no_active_bulk_readback: true,
        total_upload_ms,
        total_submit_poll_ms,
        total_compact_readback_ms,
        total_post_seal_readback_ms,
        total_cpu_shadow_ms,
        total_wall_ms: wall_ms,
        average_ms_per_tick: wall_ms / ticks_completed.max(1) as f32,
        ticks_per_second: ticks_per_second(ticks_completed, wall_ms),
        product_runtime_claim,
        full_action_authoritative_claim: false,
        stop_on_first_parity_failure: options.stop_on_first_parity_failure,
        stop_on_first_hshadow_rejection: options.stop_on_first_hshadow_rejection,
    };
    summary.validate()?;
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
        full_gpu_runtime_live_plasticity_schema, post_seal_delta_batch_from_plasticity_report,
        run_full_gpu_runtime_post_seal_plasticity_diagnostic, run_full_gpu_runtime_static_tick,
        FullGpuRuntimeProductClaim,
    };

    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let backend_mode = backend_mode(mode);
    let mut tick_summaries = Vec::with_capacity(ticks as usize);
    let mut last_static = None;
    let mut plasticity = None;
    let mut post_seal_receipt = None;
    let mut plasticity_failure_note = None;
    let mut gpu_output_used = false;

    for tick_index in 0..ticks {
        let report = live.current_sensory_report()?;
        let input = runtime_input_from_sensory(&report)?;
        let static_report = run_full_gpu_runtime_static_tick(input, backend_mode)?;
        let gpu_static_available = static_report.backend.fallback_reason.is_none();
        if tick_index == 0 && mode.requests_plasticity() && gpu_static_available {
            live.initialize_neural_projection_schema(full_gpu_runtime_live_plasticity_schema()?)?;
        }
        let proposals = if static_report.action_summary.is_some()
            && static_report.cpu_shadow_parity_passed
            && matches!(
                static_report.product_runtime_claim,
                FullGpuRuntimeProductClaim::CpuShadowGuarded
                    | FullGpuRuntimeProductClaim::CpuShadowGuardedStaticPlusLiveHShadow
                    | FullGpuRuntimeProductClaim::ActionAuthoritative
            ) {
            gpu_output_used = true;
            let action_summary =
                static_report
                    .action_summary
                    .ok_or(GameAppShellError::VisibleWorldMismatch {
                        message: "GPU static scoring reported parity without an action summary",
                    })?;
            live.current_context_proposals_with_scores(scores_from_action_summary(action_summary)?)?
        } else {
            live.current_context_proposals()?
        };
        let tick = live.tick_with_proposals_detailed(
            proposals,
            !(mode.requests_plasticity() && gpu_static_available),
        );
        if tick.summary.patch_sealed
            && tick_index == 0
            && mode.requests_plasticity()
            && gpu_static_available
        {
            if post_seal_gpu_plasticity_diagnostic_enabled() {
                match run_full_gpu_runtime_post_seal_plasticity_diagnostic(input) {
                    Ok(plasticity_report) => {
                        if let Some(patch) = tick.sealed_patch.as_ref() {
                            match post_seal_delta_batch_from_plasticity_report(
                                patch,
                                &plasticity_report,
                            ) {
                                Ok(batch) => {
                                    match live.apply_post_seal_lifetime_deltas(patch, batch) {
                                        Ok(receipt) => {
                                            post_seal_receipt = Some(receipt);
                                        }
                                        Err(error) => {
                                            plasticity_failure_note = Some(format!(
                                                "post-seal H_shadow application rejected: {error}"
                                            ));
                                        }
                                    }
                                }
                                Err(error) => {
                                    plasticity_failure_note = Some(format!(
                                        "post-seal H_shadow delta conversion rejected: {error}"
                                    ));
                                }
                            }
                        } else {
                            plasticity_failure_note = Some(
                                "post-seal GPU plasticity skipped because no sealed patch was available"
                                    .to_string(),
                            );
                        }
                        plasticity = Some(plasticity_report);
                    }
                    Err(error) => {
                        plasticity_failure_note =
                            Some(format!("post-seal GPU plasticity unavailable: {error}"));
                    }
                }
            } else {
                plasticity_failure_note = Some(
                    "post-seal GPU plasticity unavailable: ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE=0"
                        .to_string(),
                );
            }
        }
        last_static = Some(static_report);
        tick_summaries.push(tick.summary);
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
    let receipt = post_seal_receipt.as_ref();
    let product_runtime_claim =
        if receipt.is_some() && gpu_output_used && static_report.cpu_shadow_parity_passed {
            "CpuShadowGuardedStaticPlusLiveHShadow".to_string()
        } else {
            format!("{:?}", static_report.product_runtime_claim)
        };
    let summary = FullGpuRuntimeSmokeSummary {
        schema: FULL_GPU_NEURAL_RUNTIME_SCHEMA,
        schema_version: FULL_GPU_NEURAL_RUNTIME_SCHEMA_VERSION,
        requested_mode: mode.label().to_string(),
        combined_mode: mode == FullGpuRuntimeSmokeMode::StaticPlasticCpuShadowGuarded,
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
        plasticity_live_core_update_applied: receipt.is_some(),
        post_seal_hshadow_applied: receipt.is_some(),
        plasticity_post_seal_only: plasticity_report.is_none_or(|report| report.post_seal_only),
        post_seal_diagnostic_readback_bytes: plasticity_report
            .map_or(0, |report| report.diagnostic_readback_bytes),
        post_seal_diagnostic_readback_ms: plasticity_report
            .map_or(0.0, |report| report.diagnostic_readback_ms),
        post_seal_diagnostic_readback_boundary_scoped: plasticity_report
            .is_none_or(|report| report.post_seal_only),
        h_shadow_changed: plasticity_report.is_some_and(|report| report.h_shadow_changed),
        h_shadow_updated_values: plasticity_report.map_or(0, |report| report.updated_values_count),
        h_shadow_max_delta_q: plasticity_report.map_or(0, |report| report.max_delta_q),
        post_seal_delta_applied_records: receipt.map_or(0, |receipt| receipt.applied_records),
        post_seal_delta_changed_records: receipt.map_or(0, |receipt| receipt.changed_records),
        post_seal_delta_max_abs_delta: receipt.map_or(0.0, |receipt| receipt.max_abs_delta),
        post_seal_delta_sequence_id: receipt.map(|receipt| receipt.sealed_sequence_id.raw()),
        post_seal_replay_protected: receipt.is_none_or(|receipt| receipt.replay_protected),
        w_genetic_fixed_unchanged: plasticity_report
            .is_none_or(|report| report.genetic_fixed_unchanged),
        lifetime_consolidated_unchanged: plasticity_report
            .is_none_or(|report| report.lifetime_consolidated_unchanged),
        h_operational_unchanged: plasticity_report
            .is_none_or(|report| report.h_operational_unchanged),
        experience_patch_sealed_before_plasticity: plasticity_report
            .is_none_or(|_| sealed_patches > 0),
        upload_ms: static_report.timing.upload_ms,
        gpu_submit_poll_ms: static_report.timing.gpu_submit_poll_ms,
        compact_readback_ms: static_report.timing.compact_readback_ms,
        cpu_shadow_ms: static_report.timing.cpu_shadow_ms,
        total_gpu_runtime_ms: static_report.timing.total_gpu_runtime_ms,
        product_runtime_claim,
        unsupported_full_runtime_gap_remaining: true,
        plasticity_live_gap: if receipt.is_some() {
            "post-seal H_shadow delta batch applied through alife_core contract; full action-authoritative static+routing+plastic runtime remains unsupported"
                .to_string()
        } else if let Some(note) = plasticity_failure_note {
            note
        } else {
            static_report.fallback_note.clone().unwrap_or_else(|| {
                "live H_shadow application did not run; CPU fallback or unsupported mode kept plasticity shadow-only"
                    .to_string()
            })
        },
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
        combined_mode: mode == FullGpuRuntimeSmokeMode::StaticPlasticCpuShadowGuarded,
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
        plasticity_live_core_update_applied: false,
        post_seal_hshadow_applied: false,
        plasticity_post_seal_only: true,
        post_seal_diagnostic_readback_bytes: 0,
        post_seal_diagnostic_readback_ms: 0.0,
        post_seal_diagnostic_readback_boundary_scoped: true,
        h_shadow_changed: false,
        h_shadow_updated_values: 0,
        h_shadow_max_delta_q: 0,
        post_seal_delta_applied_records: 0,
        post_seal_delta_changed_records: 0,
        post_seal_delta_max_abs_delta: 0.0,
        post_seal_delta_sequence_id: None,
        post_seal_replay_protected: true,
        w_genetic_fixed_unchanged: true,
        lifetime_consolidated_unchanged: true,
        h_operational_unchanged: true,
        experience_patch_sealed_before_plasticity: false,
        upload_ms: 0.0,
        gpu_submit_poll_ms: 0.0,
        compact_readback_ms: 0.0,
        cpu_shadow_ms: 0.0,
        total_gpu_runtime_ms: 0.0,
        product_runtime_claim: "None".to_string(),
        unsupported_full_runtime_gap_remaining: true,
        plasticity_live_gap:
            "GPU feature unavailable; CPU reference sealed patches remain authoritative".to_string(),
    }
}

#[cfg(feature = "gpu-runtime")]
fn post_seal_gpu_plasticity_diagnostic_enabled() -> bool {
    std::env::var("ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE")
        .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE" | "False"))
        .unwrap_or(true)
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
        FullGpuRuntimeSmokeMode::StaticPlasticCpuShadowGuarded => {
            alife_gpu_backend::FullGpuRuntimeMode::GpuStaticPlasticCpuShadowGuarded
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

fn elapsed_ms_app(start: std::time::Instant) -> f32 {
    start.elapsed().as_secs_f64().mul_add(1000.0, 0.0) as f32
}

fn ticks_per_second(ticks: u32, wall_ms: f32) -> f32 {
    if wall_ms <= 0.0 {
        0.0
    } else {
        ticks as f32 / (wall_ms / 1000.0)
    }
}
