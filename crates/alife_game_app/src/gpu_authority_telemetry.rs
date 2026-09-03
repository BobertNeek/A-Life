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
        let selected = match (self.selected_candidate, self.selected_logit) {
            (Some(candidate), Some(logit)) if logit.is_finite() => {
                format!("candidate {candidate}  logit {logit:+.3}")
            }
            (Some(candidate), _) => format!("candidate {candidate}  logit unavailable"),
            (None, _) => "pending".to_string(),
        };
        let authority_status = if self.authoritative {
            "authoritative"
        } else if self.unavailable_reason.is_some() {
            "unavailable"
        } else {
            "initializing"
        };
        let unavailable = self
            .unavailable_reason
            .as_deref()
            .map_or_else(String::new, |reason| format!("Unavailable: {reason}\n"));
        let checkpoint_tick = self
            .checkpoint_tick
            .map_or_else(|| "pending".to_string(), |tick| tick.to_string());
        format!(
            concat!(
                "GPU neural: {}\n",
                "{}",
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
            authority_status,
            unavailable,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_reports_a_terminal_backend_failure_as_unavailable() {
        let mut telemetry = GpuBrainAuthorityTelemetry::pending("N512");
        telemetry.unavailable_reason = Some("map callback failed".to_string());

        let overlay = telemetry.overlay_text();

        assert!(overlay.contains("GPU neural: unavailable"));
        assert!(overlay.contains("map callback failed"));
        assert!(!overlay.contains("GPU neural: initializing"));
    }

    #[test]
    fn overlay_does_not_invent_a_zero_logit_for_a_selected_candidate() {
        let mut telemetry = GpuBrainAuthorityTelemetry::pending("N512");
        telemetry.selected_candidate = Some(7);

        let overlay = telemetry.overlay_text();

        assert!(overlay.contains("candidate 7  logit unavailable"));
        assert!(!overlay.contains("candidate 7  logit +0.000"));
    }
}
