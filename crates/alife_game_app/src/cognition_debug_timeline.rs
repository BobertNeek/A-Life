//! G14 read-only cognition visualization and debug timeline summaries.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct CognitionTimelineEntry {
    pub tick: Tick,
    pub sequence_id: u64,
    pub selected_action_kind: Option<ActionKind>,
    pub selected_action_id: Option<ActionId>,
    pub target_entity: Option<WorldEntityId>,
    pub success: bool,
    pub contact: Option<PhysicalContactKind>,
    pub status: BrainTickStatus,
    pub sealed_patch_only: bool,
    pub packed_log_available: bool,
    pub summary_line: String,
}

impl CognitionTimelineEntry {
    pub fn from_tick(summary: &LiveBrainTickSummary) -> Result<Self, ScaffoldContractError> {
        if !summary.patch_sealed {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if let Some(target) = summary.target_entity {
            target.validate()?;
        }
        let sequence_id = summary
            .patch_sequence_id
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let entry = Self {
            tick: summary.tick_after,
            sequence_id,
            selected_action_kind: summary.selected_action_kind,
            selected_action_id: summary.selected_action_id,
            target_entity: summary.target_entity,
            success: summary.patch_success.unwrap_or(false),
            contact: summary.physical_contact,
            status: summary.status,
            sealed_patch_only: true,
            packed_log_available: summary.packed_record_count > 0,
            summary_line: format!(
                "tick={} seq={} sealed_patch=true action={:?}:{:?} target={:?} success={} status={:?}",
                summary.tick_after.raw(),
                sequence_id,
                summary.selected_action_kind,
                summary.selected_action_id.map(|id| id.raw()),
                summary.target_entity.map(|id| id.raw()),
                summary.patch_success.unwrap_or(false),
                summary.status
            ),
        };
        entry.validate()?;
        Ok(entry)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.sequence_id == 0 || !self.sealed_patch_only || self.summary_line.is_empty() {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if let Some(action_id) = self.selected_action_id {
            action_id.validate()?;
        }
        if let Some(target) = self.target_entity {
            target.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{:?}:{:?}:{:?}:{}:{}:{:?}",
            self.tick.raw(),
            self.sequence_id,
            self.selected_action_kind,
            self.selected_action_id.map(|id| id.raw()),
            self.target_entity.map(|id| id.raw()),
            self.success,
            self.packed_log_available,
            self.status
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionProposalDebugLine {
    pub proposal_index: usize,
    pub action_kind: ActionKind,
    pub action_id: ActionId,
    pub target_entity: Option<WorldEntityId>,
    pub score: f32,
    pub confidence: f32,
    pub salience: f32,
    pub selected_by_arbitration: bool,
    pub bias_only_sources: Vec<&'static str>,
}

impl ActionProposalDebugLine {
    pub fn from_proposal(
        proposal_index: usize,
        proposal: ActionProposal,
        selected_action_id: Option<ActionId>,
    ) -> Result<Self, ScaffoldContractError> {
        let line = Self {
            proposal_index,
            action_kind: proposal.kind,
            action_id: proposal.action_id,
            target_entity: proposal.target.entity,
            score: proposal.score,
            confidence: proposal.confidence.raw(),
            salience: proposal.salience.raw(),
            selected_by_arbitration: selected_action_id == Some(proposal.action_id),
            bias_only_sources: vec![
                "action_arbitration",
                "memory_expectancy_bias_only",
                "topology_curiosity_bias_only",
                "endocrine_bias",
            ],
        };
        line.validate()?;
        Ok(line)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.action_id.validate()?;
        if let Some(target) = self.target_entity {
            target.validate()?;
        }
        if !self.score.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        Confidence::new(self.confidence)?;
        NormalizedScalar::new(self.salience)?;
        if self.bias_only_sources.is_empty() {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{:?}:{}:{:?}:{:.3}:{:.3}:{:.3}:{}",
            self.proposal_index,
            self.action_kind,
            self.action_id.raw(),
            self.target_entity.map(|id| id.raw()),
            self.score,
            self.confidence,
            self.salience,
            self.selected_by_arbitration
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CognitionBiasSummary {
    pub memory_expectancy_line: String,
    pub topology_gap_line: String,
    pub memory_record_count: usize,
    pub topology_concept_count: usize,
    pub unresolved_gap_count: usize,
    pub action_replay_blocked: bool,
    pub topology_action_bypass_blocked: bool,
}

impl CognitionBiasSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.memory_expectancy_line.is_empty()
            || self.topology_gap_line.is_empty()
            || !self.action_replay_blocked
            || !self.topology_action_bypass_blocked
            || self.memory_expectancy_line.contains("ActionCommand")
            || self.topology_gap_line.contains("ActionCommand")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.memory_record_count,
            self.topology_concept_count,
            self.unresolved_gap_count,
            self.action_replay_blocked,
            self.topology_action_bypass_blocked
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SleepConsolidationDebugSummary {
    pub latest_sleep_phase: SleepPhase,
    pub rest_event_seen: bool,
    pub consolidation_visible: bool,
    pub structural_edits_active_tick_applied: bool,
    pub summary_line: String,
}

impl SleepConsolidationDebugSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if !self.rest_event_seen
            || !self.consolidation_visible
            || self.structural_edits_active_tick_applied
            || self.summary_line.is_empty()
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{:?}:{}:{}:{}",
            self.latest_sleep_phase,
            self.rest_event_seen,
            self.consolidation_visible,
            self.structural_edits_active_tick_applied
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackedLogExportDebugBridge {
    pub packed_record_count: usize,
    pub export_command: String,
    pub offline_only: bool,
    pub mutates_runtime_state: bool,
}

impl PackedLogExportDebugBridge {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.packed_record_count == 0
            || self.export_command.is_empty()
            || !self.export_command.contains("p30_offline")
            || !self.offline_only
            || self.mutates_runtime_state
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}",
            self.packed_record_count, self.offline_only, self.mutates_runtime_state
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CognitionDebugTimelinePanel {
    pub schema: &'static str,
    pub schema_version: u16,
    pub read_only: bool,
    pub organism_id: OrganismId,
    pub timeline_entries: Vec<CognitionTimelineEntry>,
    pub proposal_lines: Vec<ActionProposalDebugLine>,
    pub drive_lines: Vec<String>,
    pub hormone_lines: Vec<String>,
    pub bias_summary: CognitionBiasSummary,
    pub sleep_summary: SleepConsolidationDebugSummary,
    pub gpu_summary: GpuProductTelemetryOverlay,
    pub packed_log_export: PackedLogExportDebugBridge,
    pub no_active_neural_readback: bool,
    pub mutation_controls_enabled: bool,
    pub panel_notes: Vec<String>,
}

impl CognitionDebugTimelinePanel {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G14_COGNITION_DEBUG_SCHEMA
            || self.schema_version != G14_COGNITION_DEBUG_SCHEMA_VERSION
            || !self.read_only
            || self.timeline_entries.is_empty()
            || self.timeline_entries.len() > G14_MAX_TIMELINE_ENTRIES
            || self.proposal_lines.is_empty()
            || self.proposal_lines.len() > G14_MAX_PROPOSAL_LINES
            || self.drive_lines.is_empty()
            || self.hormone_lines.is_empty()
            || !self.no_active_neural_readback
            || self.mutation_controls_enabled
            || self.panel_notes.is_empty()
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.organism_id.validate()?;
        for entry in &self.timeline_entries {
            entry.validate()?;
        }
        for line in &self.proposal_lines {
            line.validate()?;
        }
        self.bias_summary.validate()?;
        self.sleep_summary.validate()?;
        self.gpu_summary.validate()?;
        self.packed_log_export.validate()?;
        if self
            .panel_notes
            .iter()
            .any(|line| line.contains("mutate") || line.contains("readback=active"))
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.organism_id.raw(),
            self.timeline_entries
                .iter()
                .map(CognitionTimelineEntry::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.proposal_lines
                .iter()
                .map(ActionProposalDebugLine::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.bias_summary.signature_line(),
            self.sleep_summary.signature_line(),
            self.gpu_summary.signature_line(),
            self.packed_log_export.signature_line()
        )
    }
}

pub fn cognition_debug_timeline_panel_from_summaries(
    survival: &PlayableSurvivalLoopSummary,
    gpu: &GpuProductHardeningSummary,
) -> Result<CognitionDebugTimelinePanel, GameAppShellError> {
    survival.validate()?;
    gpu.validate()?;
    let timeline_entries = survival
        .tick_summaries
        .iter()
        .map(CognitionTimelineEntry::from_tick)
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    let proposals = cognition_debug_fixture_proposals()?;
    let decision = heuristic_baseline_arbitrate(
        survival.organism_id,
        &proposals,
        ActionArbitrationConfig::default(),
    )?;
    let selected_action_id = Some(decision.selected.action_id);
    let proposal_lines = proposals
        .into_iter()
        .enumerate()
        .map(|(index, proposal)| {
            ActionProposalDebugLine::from_proposal(index, proposal, selected_action_id)
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    let last_event = survival
        .events
        .last()
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let bias_summary = CognitionBiasSummary {
        memory_expectancy_line: format!(
            "memory_expectancy=bias_only records={} no_action_replay=true",
            survival.memory_record_count
        ),
        topology_gap_line: format!(
            "topology_gap=bias_only concepts={} unresolved_gaps={} cannot_emit_action=true",
            survival.topology_concept_count, survival.unresolved_gap_count
        ),
        memory_record_count: survival.memory_record_count,
        topology_concept_count: survival.topology_concept_count,
        unresolved_gap_count: survival.unresolved_gap_count,
        action_replay_blocked: true,
        topology_action_bypass_blocked: true,
    };
    let sleep_summary = SleepConsolidationDebugSummary {
        latest_sleep_phase: last_event.sleep_phase_after,
        rest_event_seen: last_event.kind == PlayableSurvivalEventKind::RestSleep,
        consolidation_visible: matches!(
            last_event.sleep_phase_after,
            SleepPhase::EnteringSleep | SleepPhase::Consolidating | SleepPhase::ForcedRecoverySleep
        ),
        structural_edits_active_tick_applied: false,
        summary_line: format!(
            "sleep_phase={:?} structural_edit_active_tick_applied=false",
            last_event.sleep_phase_after
        ),
    };
    let panel = CognitionDebugTimelinePanel {
        schema: G14_COGNITION_DEBUG_SCHEMA,
        schema_version: G14_COGNITION_DEBUG_SCHEMA_VERSION,
        read_only: true,
        organism_id: survival.organism_id,
        timeline_entries,
        proposal_lines,
        drive_lines: survival
            .events
            .iter()
            .map(|event| {
                format!(
                    "{} hunger={:.2}->{:.2} fatigue={:.2} fear={:.2} pain={:.2} energy={:.2}",
                    event.kind.label(),
                    event.hunger_before,
                    event.hunger_after,
                    event.fatigue_after,
                    event.fear_after,
                    event.pain_after,
                    event.brain_atp_after
                )
            })
            .collect(),
        hormone_lines: vec![format!(
            "sleep_pressure visible through final_sleep_phase={:?}",
            last_event.sleep_phase_after
        )],
        bias_summary,
        sleep_summary,
        gpu_summary: gpu.telemetry_overlay.clone(),
        packed_log_export: PackedLogExportDebugBridge {
            packed_record_count: survival.packed_record_count,
            export_command: "cargo run -p alife_tools --bin p30_offline -- summary --record target/artifacts/g14_packed_records.json --markdown target/artifacts/g14_cognition_summary.md".to_string(),
            offline_only: true,
            mutates_runtime_state: false,
        },
        no_active_neural_readback: gpu.telemetry_overlay.no_active_gameplay_readback,
        mutation_controls_enabled: false,
        panel_notes: vec![
            "timeline is derived from sealed ExperiencePatch summaries only".to_string(),
            "memory and topology lines are bias metadata, not action sources".to_string(),
            "GPU diagnostics are boundary-scoped and may report typed unavailability".to_string(),
        ],
    };
    panel.validate()?;
    Ok(panel)
}

pub fn run_cognition_debug_timeline_smoke() -> Result<CognitionDebugTimelinePanel, GameAppShellError>
{
    let survival = run_playable_survival_loop_smoke()?;
    let gpu = run_gpu_product_hardening_smoke()?;
    cognition_debug_timeline_panel_from_summaries(&survival, &gpu)
}

fn cognition_debug_fixture_proposals() -> Result<Vec<ActionProposal>, ScaffoldContractError> {
    Ok(vec![
        proposal(
            HeadlessActionIds::EAT,
            ActionKind::Interact,
            Some(WorldEntityId(2)),
            None,
            0.96,
            0.97,
            1.0,
        )?,
        proposal(
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            Some(WorldEntityId(3)),
            None,
            0.44,
            0.70,
            2.0,
        )?,
        proposal(
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            None,
            None,
            0.28,
            0.55,
            0.0,
        )?,
    ])
}
