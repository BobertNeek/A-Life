//! CA30 compact neural activity/lobe profiler.
//!
//! The profiler is a read-only presentation surface. It summarizes core lobe
//! layout and bounded runtime/GPU telemetry; it never reads raw neural tensors,
//! emits actions, or mutates cognition.

use crate::prelude::*;
use crate::*;

pub const CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA: &str = "alife.ca30.neural_activity_profiler.v1";
pub const CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA_VERSION: u16 = 1;
pub const CA30_MAX_LOBE_ROWS: usize = 6;
pub const CA30_ACTIVITY_BAR_WIDTH: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct NeuralLobeActivityRow {
    pub lobe: alife_core::LobeKind,
    pub lobe_index: u16,
    pub start: u32,
    pub len: u32,
    pub activity: f32,
    pub bar: String,
    pub source: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NeuralRouteStatusSummary {
    pub requested_mode: String,
    pub selected_backend: String,
    pub fallback_reason: Option<String>,
    pub cpu_shadow_gate: bool,
    pub cpu_shadow_parity: bool,
    pub gpu_scores_used_for_proposals: bool,
    pub compact_readback_bytes: usize,
    pub post_seal_readback_bytes: usize,
    pub no_active_bulk_readback: bool,
    pub product_runtime_claim: String,
    pub full_action_authoritative_claim: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NeuralTileActivitySummary {
    pub active_tiles: u32,
    pub max_active_tiles: u32,
    pub skipped_tiles: u32,
    pub active_synapses: u32,
    pub max_active_synapses: u32,
    pub source: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NeuralActivityProfilerSnapshot {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub brain_class_id: u16,
    pub neuron_count: u32,
    pub lobe_rows: Vec<NeuralLobeActivityRow>,
    pub tile_summary: NeuralTileActivitySummary,
    pub route_status: NeuralRouteStatusSummary,
    pub read_only: bool,
    pub compact_summary_only: bool,
    pub active_bulk_readback_allowed: bool,
    pub offline_export_boundary: bool,
    pub can_emit_actions: bool,
    pub can_mutate_weights: bool,
}

impl NeuralActivityProfilerSnapshot {
    pub fn from_live_loop(
        live: &LiveBrainLoop,
        recent_summaries: &[LiveBrainTickSummary],
        gpu: Option<&GraphicalGpuRuntimeTelemetry>,
    ) -> Result<Self, GameAppShellError> {
        let brain_class = live.mind().brain_class();
        let route_status = route_status_from_gpu(gpu);
        let tile_summary = tile_summary_from(brain_class, recent_summaries, gpu);
        let rows = lobe_rows_from(
            brain_class,
            recent_summaries,
            gpu,
            live.mind().memory_bank().records_chronological().len(),
            live.mind().topological_map().concepts().len(),
        )?;
        let snapshot = Self {
            schema: CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA,
            schema_version: CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA_VERSION,
            organism_id: live.organism_id(),
            tick: live.mind().current_tick(),
            brain_class_id: brain_class.id.raw(),
            neuron_count: brain_class.neuron_count,
            lobe_rows: rows,
            tile_summary,
            route_status,
            read_only: true,
            compact_summary_only: true,
            active_bulk_readback_allowed: false,
            offline_export_boundary: true,
            can_emit_actions: false,
            can_mutate_weights: false,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn pending(organism_id: OrganismId, tick: Tick) -> Self {
        Self {
            schema: CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA,
            schema_version: CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA_VERSION,
            organism_id,
            tick,
            brain_class_id: 0,
            neuron_count: 0,
            lobe_rows: Vec::new(),
            tile_summary: NeuralTileActivitySummary {
                active_tiles: 0,
                max_active_tiles: 0,
                skipped_tiles: 0,
                active_synapses: 0,
                max_active_synapses: 0,
                source: "pending",
            },
            route_status: NeuralRouteStatusSummary {
                requested_mode: "pending".to_string(),
                selected_backend: "PendingFirstTick".to_string(),
                fallback_reason: None,
                cpu_shadow_gate: true,
                cpu_shadow_parity: false,
                gpu_scores_used_for_proposals: false,
                compact_readback_bytes: 0,
                post_seal_readback_bytes: 0,
                no_active_bulk_readback: true,
                product_runtime_claim: "PendingTick".to_string(),
                full_action_authoritative_claim: false,
            },
            read_only: true,
            compact_summary_only: true,
            active_bulk_readback_allowed: false,
            offline_export_boundary: true,
            can_emit_actions: false,
            can_mutate_weights: false,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA
            || self.schema_version != CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.organism_id.validate()?;
        if !self.read_only
            || !self.compact_summary_only
            || self.active_bulk_readback_allowed
            || !self.offline_export_boundary
            || self.can_emit_actions
            || self.can_mutate_weights
            || self.route_status.full_action_authoritative_claim
            || !self.route_status.cpu_shadow_gate
            || !self.route_status.no_active_bulk_readback
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if self.lobe_rows.len() > CA30_MAX_LOBE_ROWS {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for row in &self.lobe_rows {
            validate_unit(row.activity)?;
            validate_ca30_display_line(&row.bar)?;
            if row.lobe_index == 0 || row.len == 0 {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        validate_ca30_display_line(&self.route_status.requested_mode)?;
        validate_ca30_display_line(&self.route_status.selected_backend)?;
        validate_ca30_display_line(&self.route_status.product_runtime_claim)?;
        if let Some(reason) = &self.route_status.fallback_reason {
            validate_ca30_display_line(reason)?;
        }
        Ok(())
    }

    pub fn panel_text(&self) -> String {
        let lobe_lines = if self.lobe_rows.is_empty() {
            "lobes: pending compact summary".to_string()
        } else {
            self.lobe_rows
                .iter()
                .map(|row| {
                    format!(
                        "{} {:<8} {:.2} {}",
                        lobe_short_label(row.lobe),
                        row.bar,
                        row.activity,
                        row.source
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let fallback = self
            .route_status
            .fallback_reason
            .as_deref()
            .unwrap_or("none");
        format!(
            concat!(
                "Neural Profiler (compact)\n",
                "brain={} neurons={} tick={}\n",
                "{}\n",
                "tiles {}/{} skip={} syn {}/{}\n",
                "route {} backend={} fallback={}\n",
                "scores={} parity={} readback={}B post={}B\n",
                "Boundary: compact summary; offline export only"
            ),
            self.brain_class_id,
            self.neuron_count,
            self.tick.raw(),
            lobe_lines,
            self.tile_summary.active_tiles,
            self.tile_summary.max_active_tiles,
            self.tile_summary.skipped_tiles,
            self.tile_summary.active_synapses,
            self.tile_summary.max_active_synapses,
            self.route_status.requested_mode,
            self.route_status.selected_backend,
            fallback,
            self.route_status.gpu_scores_used_for_proposals,
            self.route_status.cpu_shadow_parity,
            self.route_status.compact_readback_bytes,
            self.route_status.post_seal_readback_bytes,
        )
    }

    pub fn compact_line(&self) -> String {
        format!(
            "Neural: lobes={} tiles={}/{} syn={}/{} route={} bulk-readback=false",
            self.lobe_rows.len(),
            self.tile_summary.active_tiles,
            self.tile_summary.max_active_tiles,
            self.tile_summary.active_synapses,
            self.tile_summary.max_active_synapses,
            self.route_status.selected_backend
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:org={}:tick={}:brain={}:neurons={}:lobes={}:tiles={}:syn={}:backend={}:bulk={}:actions={}:weights={}",
            self.schema,
            self.schema_version,
            self.organism_id.raw(),
            self.tick.raw(),
            self.brain_class_id,
            self.neuron_count,
            self.lobe_rows.len(),
            self.tile_summary.active_tiles,
            self.tile_summary.active_synapses,
            self.route_status.selected_backend,
            self.active_bulk_readback_allowed,
            self.can_emit_actions,
            self.can_mutate_weights
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NeuralActivityProfilerSmokeSummary {
    pub snapshot: NeuralActivityProfilerSnapshot,
    pub panel_text: String,
    pub status_text: String,
    pub bulk_readback_blocked: bool,
    pub action_authority_blocked: bool,
    pub weight_mutation_blocked: bool,
}

impl NeuralActivityProfilerSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        self.snapshot.validate()?;
        if self.snapshot.lobe_rows.is_empty()
            || self.snapshot.tile_summary.max_active_tiles == 0
            || self.snapshot.tile_summary.max_active_synapses == 0
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA30 neural profiler must expose lobes, tiles, and synapse bounds",
            });
        }
        if !self.bulk_readback_blocked
            || !self.action_authority_blocked
            || !self.weight_mutation_blocked
            || self.snapshot.active_bulk_readback_allowed
            || self.snapshot.can_emit_actions
            || self.snapshot.can_mutate_weights
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA30 neural profiler must be read-only and no-readback",
            });
        }
        if !self.panel_text.contains("Neural Profiler (compact)")
            || !self
                .panel_text
                .contains("Boundary: compact summary; offline export only")
            || self.panel_text.contains("Entity(")
            || self.panel_text.contains("full action-authoritative")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA30 neural profiler panel must be compact and claim-safe",
            });
        }
        if !self.status_text.contains("Neural:") {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA30 status panel must expose a compact neural profiler line",
            });
        }
        Ok(())
    }
}

pub fn run_neural_activity_profiler_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<NeuralActivityProfilerSmokeSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    panel.apply_command(&mut live, RuntimeControlCommand::RunForTicks(5))?;
    let snapshot = panel.neural_profiler.clone();
    let panel_text = snapshot.panel_text();
    let status_text = panel.structured_status_panel_text_with_backend("GPU: GpuPlastic requested");
    let summary = NeuralActivityProfilerSmokeSummary {
        snapshot,
        panel_text,
        status_text,
        bulk_readback_blocked: true,
        action_authority_blocked: true,
        weight_mutation_blocked: true,
    };
    summary.validate()?;
    Ok(summary)
}

pub(crate) fn route_status_from_gpu(
    gpu: Option<&GraphicalGpuRuntimeTelemetry>,
) -> NeuralRouteStatusSummary {
    match gpu {
        Some(gpu) => NeuralRouteStatusSummary {
            requested_mode: gpu.requested_mode.label().to_string(),
            selected_backend: gpu.selected_backend.clone(),
            fallback_reason: gpu.fallback_reason.clone(),
            cpu_shadow_gate: true,
            cpu_shadow_parity: gpu.cpu_shadow_parity,
            gpu_scores_used_for_proposals: gpu.gpu_scores_used_for_proposals,
            compact_readback_bytes: gpu.compact_readback_bytes,
            post_seal_readback_bytes: gpu.post_seal_readback_bytes,
            no_active_bulk_readback: gpu.no_active_bulk_readback,
            product_runtime_claim: gpu.product_runtime_claim.clone(),
            full_action_authoritative_claim: gpu.full_action_authoritative_claim,
        },
        None => NeuralRouteStatusSummary {
            requested_mode: "cpu-reference-summary".to_string(),
            selected_backend: "CpuReference".to_string(),
            fallback_reason: None,
            cpu_shadow_gate: true,
            cpu_shadow_parity: false,
            gpu_scores_used_for_proposals: false,
            compact_readback_bytes: 0,
            post_seal_readback_bytes: 0,
            no_active_bulk_readback: true,
            product_runtime_claim: "None".to_string(),
            full_action_authoritative_claim: false,
        },
    }
}

fn tile_summary_from(
    brain_class: &alife_core::BrainClassSpec,
    recent_summaries: &[LiveBrainTickSummary],
    gpu: Option<&GraphicalGpuRuntimeTelemetry>,
) -> NeuralTileActivitySummary {
    let max_active_tiles = brain_class
        .max_active_microtiles
        .max(brain_class.compute_budget.max_active_tiles);
    let max_active_synapses = brain_class
        .max_active_synapses
        .max(brain_class.compute_budget.max_active_synapses);
    if let Some(gpu) = gpu {
        let active = if gpu.gpu_static_dispatched_ticks > 0 {
            max_active_tiles.min(gpu.gpu_static_dispatched_ticks.saturating_mul(4).max(1))
        } else {
            0
        };
        let active_synapses = active.saturating_mul(128).min(max_active_synapses);
        return NeuralTileActivitySummary {
            active_tiles: active,
            max_active_tiles,
            skipped_tiles: max_active_tiles.saturating_sub(active),
            active_synapses,
            max_active_synapses,
            source: "gpu-telemetry-compact",
        };
    }
    let sealed = recent_summaries
        .iter()
        .filter(|summary| summary.patch_sealed)
        .count() as u32;
    let active = max_active_tiles.min(sealed.saturating_mul(2).max(u32::from(sealed > 0)));
    NeuralTileActivitySummary {
        active_tiles: active,
        max_active_tiles,
        skipped_tiles: max_active_tiles.saturating_sub(active),
        active_synapses: active.saturating_mul(64).min(max_active_synapses),
        max_active_synapses,
        source: "cpu-summary-estimate",
    }
}

fn lobe_rows_from(
    brain_class: &alife_core::BrainClassSpec,
    recent_summaries: &[LiveBrainTickSummary],
    gpu: Option<&GraphicalGpuRuntimeTelemetry>,
    memory_records: usize,
    concept_count: usize,
) -> Result<Vec<NeuralLobeActivityRow>, ScaffoldContractError> {
    let rows = brain_class
        .lobe_regions()
        .take(CA30_MAX_LOBE_ROWS)
        .map(|region| {
            let activity = activity_for_lobe(
                region.kind,
                recent_summaries,
                gpu,
                memory_records,
                concept_count,
            );
            let bar = activity_bar(activity)?;
            Ok(NeuralLobeActivityRow {
                lobe: region.kind,
                lobe_index: region.kind.stable_id().raw(),
                start: region.start,
                len: region.len,
                activity,
                bar,
                source: activity_source_for_lobe(region.kind),
            })
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    Ok(rows)
}

fn activity_for_lobe(
    lobe: alife_core::LobeKind,
    recent_summaries: &[LiveBrainTickSummary],
    gpu: Option<&GraphicalGpuRuntimeTelemetry>,
    memory_records: usize,
    concept_count: usize,
) -> f32 {
    let latest = recent_summaries.last();
    let sealed = recent_summaries
        .iter()
        .filter(|summary| summary.patch_sealed)
        .count() as f32;
    let contacts = recent_summaries
        .iter()
        .filter(|summary| summary.physical_contact.is_some())
        .count() as f32;
    let gpu_score = gpu.map_or(0.0, |gpu| {
        if gpu.gpu_scores_used_for_proposals && gpu.cpu_shadow_parity {
            1.0
        } else {
            0.0
        }
    });
    let learning = gpu.map_or_else(
        || {
            recent_summaries
                .iter()
                .map(|summary| summary.learning_updates)
                .sum::<u32>() as f32
        },
        |gpu| gpu.h_shadow_applications as f32,
    );
    let value = match lobe {
        alife_core::LobeKind::SensoryGrounding => {
            let target_signal = if latest.and_then(|summary| summary.target_entity).is_some() {
                1.0
            } else {
                0.0
            };
            (contacts + target_signal).min(4.0) / 4.0
        }
        alife_core::LobeKind::MetabolicDrive | alife_core::LobeKind::HomeostaticRegulation => {
            (sealed / 5.0).min(1.0)
        }
        alife_core::LobeKind::MotorArbitration => {
            f32::from(latest.and_then(|s| s.selected_action_kind).is_some()).max(gpu_score)
        }
        alife_core::LobeKind::EpisodicMemory => (memory_records as f32 / 8.0).min(1.0),
        alife_core::LobeKind::CoreAssociation | alife_core::LobeKind::LexiconConcept => {
            (concept_count as f32 / 6.0).min(1.0)
        }
        alife_core::LobeKind::WorkingMemory => (recent_summaries.len() as f32 / 5.0).min(1.0),
        _ => (learning / 4.0).min(1.0),
    };
    value.clamp(0.0, 1.0)
}

fn activity_source_for_lobe(lobe: alife_core::LobeKind) -> &'static str {
    match lobe {
        alife_core::LobeKind::SensoryGrounding => "sensory",
        alife_core::LobeKind::MetabolicDrive | alife_core::LobeKind::HomeostaticRegulation => {
            "drive"
        }
        alife_core::LobeKind::MotorArbitration => "motor",
        alife_core::LobeKind::EpisodicMemory => "memory",
        alife_core::LobeKind::CoreAssociation | alife_core::LobeKind::LexiconConcept => "concept",
        alife_core::LobeKind::WorkingMemory => "recent",
        _ => "learning",
    }
}

fn lobe_short_label(lobe: alife_core::LobeKind) -> &'static str {
    match lobe {
        alife_core::LobeKind::SensoryGrounding => "Sense",
        alife_core::LobeKind::MetabolicDrive => "Drive",
        alife_core::LobeKind::AuditorySpeech => "Audio",
        alife_core::LobeKind::GlyphVision => "Glyph",
        alife_core::LobeKind::LexiconConcept => "Lex",
        alife_core::LobeKind::CoreAssociation => "Assoc",
        alife_core::LobeKind::EpisodicMemory => "Memory",
        alife_core::LobeKind::WorkingMemory => "Work",
        alife_core::LobeKind::MotorArbitration => "Motor",
        alife_core::LobeKind::HomeostaticRegulation => "Homeo",
        _ => "Future",
    }
}

fn activity_bar(activity: f32) -> Result<String, ScaffoldContractError> {
    validate_unit(activity)?;
    let filled = (activity * CA30_ACTIVITY_BAR_WIDTH as f32).round() as usize;
    let filled = filled.min(CA30_ACTIVITY_BAR_WIDTH);
    Ok(format!(
        "{}{}",
        "#".repeat(filled),
        ".".repeat(CA30_ACTIVITY_BAR_WIDTH - filled)
    ))
}

fn validate_unit(value: f32) -> Result<(), ScaffoldContractError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn validate_ca30_display_line(line: &str) -> Result<(), ScaffoldContractError> {
    if line.is_empty()
        || line.len() > 180
        || line.contains("Entity(")
        || line.contains("http://")
        || line.contains("https://")
    {
        Err(ScaffoldContractError::InvalidId)
    } else {
        Ok(())
    }
}
