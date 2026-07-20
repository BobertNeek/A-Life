//! GPU-authority status contracts shared by headless diagnostics and the GPU runtime.

use crate::GraphicalBrainPolicyMode;

#[derive(Debug, Clone, PartialEq)]
pub struct GpuBrainAuthorityTelemetry {
    pub authoritative: bool,
    pub adapter: String,
    pub phenotype_hash_prefix: String,
    pub capacity_class: String,
    pub selected_candidate: Option<u16>,
    pub selected_logit: Option<f32>,
    pub compact_readback_bytes: usize,
    pub finite_rejections: u32,
    pub requested_mode: GraphicalBrainPolicyMode,
    pub selected_backend: String,
    pub unavailable_reason: Option<String>,
    pub sealed_patches: usize,
    pub learning_updates: u32,
    pub last_learning_delta: f32,
    pub active_ticks: u32,
    pub no_active_bulk_readback: bool,
    pub checkpoint_tick: Option<u64>,
    pub checkpoint_sleep_phase: String,
    pub checkpoint_consolidation_state: String,
    pub recovery_status: String,
    pub wgsl: GpuBrainTimingTelemetry,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GpuBrainTimingTelemetry {
    pub timing_available: bool,
    pub upload_ms: f32,
    pub compute_submit_poll_ms: f32,
    pub compact_readback_ms: f32,
    pub routing_active_tiles: u32,
    pub routing_skipped_tiles: u32,
    pub routing_active_synapses: u32,
}

impl GpuBrainAuthorityTelemetry {
    pub fn pending(capacity_class: impl Into<String>) -> Self {
        Self {
            authoritative: false,
            adapter: "initializing".to_string(),
            phenotype_hash_prefix: "pending".to_string(),
            capacity_class: capacity_class.into(),
            selected_candidate: None,
            selected_logit: None,
            compact_readback_bytes: 0,
            finite_rejections: 0,
            requested_mode: GraphicalBrainPolicyMode::GpuRequired,
            selected_backend: "GpuAuthoritative".to_string(),
            unavailable_reason: None,
            sealed_patches: 0,
            learning_updates: 0,
            last_learning_delta: 0.0,
            active_ticks: 0,
            no_active_bulk_readback: true,
            checkpoint_tick: None,
            checkpoint_sleep_phase: "Pending".to_string(),
            checkpoint_consolidation_state: "Pending".to_string(),
            recovery_status: "GPU required".to_string(),
            wgsl: GpuBrainTimingTelemetry::default(),
        }
    }

    pub fn overlay_text(&self) -> String {
        let selected = self.selected_candidate.map_or_else(
            || "pending".to_string(),
            |candidate| {
                format!(
                    "candidate {candidate}  logit {:+.3}",
                    self.selected_logit.unwrap_or_default()
                )
            },
        );
        let checkpoint_tick = self
            .checkpoint_tick
            .map_or_else(|| "pending".to_string(), |tick| tick.to_string());
        format!(
            concat!(
                "GPU neural: {}\n",
                "Adapter: {}\n",
                "Class: {}\n",
                "Selected: {}\n\n",
                "GPU BRAIN CHECKPOINT\n",
                "Phenotype: {}\n",
                "Checkpoint tick: {}\n",
                "Sleep phase: {}\n",
                "Consolidation: {}\n",
                "Recovery: {}\n",
                "Failure policy: stop learned actions"
            ),
            if self.authoritative {
                "authoritative"
            } else {
                "initializing"
            },
            self.adapter,
            self.capacity_class,
            selected,
            self.phenotype_hash_prefix,
            checkpoint_tick,
            self.checkpoint_sleep_phase,
            self.checkpoint_consolidation_state,
            self.recovery_status,
        )
    }
}

pub type GraphicalGpuRuntimeTelemetry = GpuBrainAuthorityTelemetry;
