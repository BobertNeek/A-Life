//! CA29 read-only creature memory/history journal.
//!
//! The journal mirrors sealed patch history and bounded memory expectancy bias
//! summaries into player-facing text. It never replays actions and never
//! mutates cognition.

use crate::prelude::*;
use crate::*;

pub const CA29_MEMORY_HISTORY_JOURNAL_SCHEMA: &str = "alife.ca29.memory_history_journal.v1";
pub const CA29_MEMORY_HISTORY_JOURNAL_SCHEMA_VERSION: u16 = 1;
pub const CA29_MAX_PATCH_ROWS: usize = 5;
pub const CA29_MAX_MEMORY_ROWS: usize = 5;
pub const CA29_MAX_EXPECTANCY_ROWS: usize = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryJournalPatchRow {
    pub tick: u64,
    pub sequence_id: Option<u64>,
    pub action_kind: Option<ActionKind>,
    pub target_entity: Option<WorldEntityId>,
    pub success: Option<bool>,
    pub memory_updates: u32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryJournalRecordRow {
    pub memory_id: u64,
    pub source_tick: u64,
    pub sequence_id: u64,
    pub expected_valence: f32,
    pub affordance_bias: f32,
    pub danger_bias: f32,
    pub safety_bias: f32,
    pub novelty_bias: f32,
    pub curiosity_bias: f32,
    pub observed_action_kind: Option<ActionKind>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryExpectancyBiasRow {
    pub source_memory_id: u64,
    pub confidence: f32,
    pub expected_valence: f32,
    pub affordance_bias: f32,
    pub danger_bias: f32,
    pub curiosity_bias: f32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureMemoryHistoryJournalSnapshot {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub memory_record_count: usize,
    pub recent_patches: Vec<MemoryJournalPatchRow>,
    pub recent_memories: Vec<MemoryJournalRecordRow>,
    pub expectancy_rows: Vec<MemoryExpectancyBiasRow>,
    pub save_load_visible: bool,
    pub read_only: bool,
    pub expectancy_bias_only: bool,
    pub can_replay_actions: bool,
    pub can_emit_actions: bool,
    pub direct_cognition_mutation_allowed: bool,
}

impl CreatureMemoryHistoryJournalSnapshot {
    pub fn from_live_loop(
        live: &LiveBrainLoop,
        recent_summaries: &[LiveBrainTickSummary],
    ) -> Result<Self, GameAppShellError> {
        let memory_records = live.mind().memory_bank().records_chronological();
        let recent_memories = memory_records
            .iter()
            .rev()
            .take(CA29_MAX_MEMORY_ROWS)
            .map(memory_record_row)
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let recent_patches = recent_summaries
            .iter()
            .rev()
            .filter(|summary| summary.patch_sealed)
            .take(CA29_MAX_PATCH_ROWS)
            .map(patch_row)
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let expectancy_rows = build_expectancy_rows(live, &memory_records)?;

        let snapshot = Self {
            schema: CA29_MEMORY_HISTORY_JOURNAL_SCHEMA,
            schema_version: CA29_MEMORY_HISTORY_JOURNAL_SCHEMA_VERSION,
            organism_id: live.organism_id(),
            tick: live.mind().current_tick(),
            memory_record_count: memory_records.len(),
            recent_patches,
            recent_memories,
            expectancy_rows,
            save_load_visible: true,
            read_only: true,
            expectancy_bias_only: true,
            can_replay_actions: false,
            can_emit_actions: false,
            direct_cognition_mutation_allowed: false,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn pending(organism_id: OrganismId, tick: Tick) -> Self {
        Self {
            schema: CA29_MEMORY_HISTORY_JOURNAL_SCHEMA,
            schema_version: CA29_MEMORY_HISTORY_JOURNAL_SCHEMA_VERSION,
            organism_id,
            tick,
            memory_record_count: 0,
            recent_patches: Vec::new(),
            recent_memories: Vec::new(),
            expectancy_rows: Vec::new(),
            save_load_visible: true,
            read_only: true,
            expectancy_bias_only: true,
            can_replay_actions: false,
            can_emit_actions: false,
            direct_cognition_mutation_allowed: false,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA29_MEMORY_HISTORY_JOURNAL_SCHEMA
            || self.schema_version != CA29_MEMORY_HISTORY_JOURNAL_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.organism_id.validate()?;
        if !self.save_load_visible
            || !self.read_only
            || !self.expectancy_bias_only
            || self.can_replay_actions
            || self.can_emit_actions
            || self.direct_cognition_mutation_allowed
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if self.recent_patches.len() > CA29_MAX_PATCH_ROWS
            || self.recent_memories.len() > CA29_MAX_MEMORY_ROWS
            || self.expectancy_rows.len() > CA29_MAX_EXPECTANCY_ROWS
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for row in &self.recent_patches {
            if let Some(target) = row.target_entity {
                target.validate()?;
            }
            validate_ca29_display_line(&row.label)?;
        }
        for row in &self.recent_memories {
            validate_unit_signed(row.expected_valence)?;
            validate_unit(row.affordance_bias)?;
            validate_unit(row.danger_bias)?;
            validate_unit(row.safety_bias)?;
            validate_unit(row.novelty_bias)?;
            validate_unit(row.curiosity_bias)?;
            validate_ca29_display_line(&row.label)?;
        }
        for row in &self.expectancy_rows {
            validate_unit(row.confidence)?;
            validate_unit_signed(row.expected_valence)?;
            validate_unit(row.affordance_bias)?;
            validate_unit(row.danger_bias)?;
            validate_unit(row.curiosity_bias)?;
            validate_ca29_display_line(&row.label)?;
        }
        Ok(())
    }

    pub fn panel_text(&self) -> String {
        let patch = self.recent_patches.first().map_or_else(
            || "patch: waiting for sealed experience".to_string(),
            |patch| patch.label.clone(),
        );
        let memory = self.recent_memories.first().map_or_else(
            || "memory: waiting for stored expectancy".to_string(),
            |memory| memory.label.clone(),
        );
        let bias = self.expectancy_rows.first().map_or_else(
            || "bias: neutral expectancy".to_string(),
            |bias| bias.label.clone(),
        );
        format!(
            concat!(
                "Memory Journal (read-only)\n",
                "memories={} tick={} patches={} bias_rows={}\n",
                "{}\n",
                "{}\n",
                "{}\n",
                "Save/load: stable memory IDs visible\n",
                "Boundary: expectancy bias only; no action replay"
            ),
            self.memory_record_count,
            self.tick.raw(),
            self.recent_patches.len(),
            self.expectancy_rows.len(),
            patch,
            memory,
            bias
        )
    }

    pub fn compact_line(&self) -> String {
        format!(
            "Memory: records={} patches={} bias-only no-replay",
            self.memory_record_count,
            self.recent_patches.len()
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:org={}:tick={}:records={}:patches={}:bias_rows={}:save_visible={}:readonly={}:replay={}:actions={}",
            self.schema,
            self.schema_version,
            self.organism_id.raw(),
            self.tick.raw(),
            self.memory_record_count,
            self.recent_patches.len(),
            self.expectancy_rows.len(),
            self.save_load_visible,
            self.read_only,
            self.can_replay_actions,
            self.can_emit_actions
        )
    }

    pub fn preserve_previous_patches_if_empty(
        &mut self,
        previous: &[MemoryJournalPatchRow],
    ) -> Result<(), ScaffoldContractError> {
        if self.recent_patches.is_empty() && !previous.is_empty() {
            self.recent_patches = previous.iter().take(CA29_MAX_PATCH_ROWS).cloned().collect();
        }
        self.validate()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureMemoryHistoryJournalSmokeSummary {
    pub snapshot: CreatureMemoryHistoryJournalSnapshot,
    pub panel_text: String,
    pub status_text: String,
    pub action_replay_blocked: bool,
    pub direct_cognition_mutation_allowed: bool,
}

impl CreatureMemoryHistoryJournalSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        self.snapshot.validate()?;
        if self.snapshot.memory_record_count == 0
            || self.snapshot.recent_patches.is_empty()
            || self.snapshot.expectancy_rows.is_empty()
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA29 memory journal must show records, sealed patches, and expectancy",
            });
        }
        if !self.action_replay_blocked
            || self.direct_cognition_mutation_allowed
            || self.snapshot.can_replay_actions
            || self.snapshot.can_emit_actions
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA29 memory journal must block action replay and authority",
            });
        }
        if !self.panel_text.contains("Memory Journal (read-only)")
            || !self
                .panel_text
                .contains("Boundary: expectancy bias only; no action replay")
            || !self
                .panel_text
                .contains("Save/load: stable memory IDs visible")
            || self.panel_text.contains("Entity(")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA29 memory journal text must be readable and stable-ID safe",
            });
        }
        if !self.status_text.contains("Memory:") {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA29 status panel must expose compact memory journal line",
            });
        }
        Ok(())
    }
}

pub fn run_memory_history_journal_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<CreatureMemoryHistoryJournalSmokeSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    panel.apply_command(&mut live, RuntimeControlCommand::RunForTicks(5))?;
    let snapshot = panel.memory_journal.clone();
    let panel_text = snapshot.panel_text();
    let status_text = panel.structured_status_panel_text_with_backend("GPU: GpuPlastic requested");
    let summary = CreatureMemoryHistoryJournalSmokeSummary {
        snapshot,
        panel_text,
        status_text,
        action_replay_blocked: true,
        direct_cognition_mutation_allowed: panel.direct_cognition_mutation_allowed,
    };
    summary.validate()?;
    Ok(summary)
}

fn patch_row(
    summary: &LiveBrainTickSummary,
) -> Result<MemoryJournalPatchRow, ScaffoldContractError> {
    if let Some(target) = summary.target_entity {
        target.validate()?;
    }
    let action = summary
        .selected_action_kind
        .map_or("None".to_string(), |action| format!("{action:?}"));
    let target = summary.target_entity.map_or_else(
        || "none".to_string(),
        |target| format!("stable:{}", target.raw()),
    );
    let sequence = summary
        .patch_sequence_id
        .map_or_else(|| "pending".to_string(), |sequence| sequence.to_string());
    let success = summary
        .patch_success
        .map_or_else(|| "pending".to_string(), |success| success.to_string());
    let label = format!(
        "patch tick={} seq={} action={} target={} success={} mem+{}",
        summary.tick_after.raw(),
        sequence,
        action,
        target,
        success,
        summary.memory_updates
    );
    validate_ca29_display_line(&label)?;
    Ok(MemoryJournalPatchRow {
        tick: summary.tick_after.raw(),
        sequence_id: summary.patch_sequence_id,
        action_kind: summary.selected_action_kind,
        target_entity: summary.target_entity,
        success: summary.patch_success,
        memory_updates: summary.memory_updates,
        label,
    })
}

fn memory_record_row(
    record: &&alife_core::MemoryRecord,
) -> Result<MemoryJournalRecordRow, ScaffoldContractError> {
    record.validate_contract()?;
    let action = record
        .selected_action_kind
        .map_or("none".to_string(), |kind| format!("{kind:?}"));
    let label = format!(
        "memory m{} seq={} tick={} val={:.2} aff={:.2} danger={:.2} observed={} no-replay",
        record.memory_id.raw(),
        record.source_sequence_id.raw(),
        record.source_tick.raw(),
        record.expected_valence.raw(),
        record.affordance_bias.raw(),
        record.danger_bias.raw(),
        action
    );
    validate_ca29_display_line(&label)?;
    Ok(MemoryJournalRecordRow {
        memory_id: record.memory_id.raw(),
        source_tick: record.source_tick.raw(),
        sequence_id: record.source_sequence_id.raw(),
        expected_valence: record.expected_valence.raw(),
        affordance_bias: record.affordance_bias.raw(),
        danger_bias: record.danger_bias.raw(),
        safety_bias: record.safety_bias.raw(),
        novelty_bias: record.novelty_bias.raw(),
        curiosity_bias: record.curiosity_bias.raw(),
        observed_action_kind: record.selected_action_kind,
        label,
    })
}

fn build_expectancy_rows(
    live: &LiveBrainLoop,
    records: &[&alife_core::MemoryRecord],
) -> Result<Vec<MemoryExpectancyBiasRow>, ScaffoldContractError> {
    let Some(latest) = records
        .iter()
        .rev()
        .find(|record| record.organism_id == live.organism_id())
    else {
        return Ok(Vec::new());
    };
    let query = alife_core::MemoryQuery::new(
        live.organism_id(),
        live.mind().current_tick(),
        latest.features.clone(),
    )?;
    let expectancy = live.mind().memory_bank().recall(&query)?;
    expectancy.validate_contract()?;
    expectancy
        .source_memory_ids
        .iter()
        .take(CA29_MAX_EXPECTANCY_ROWS)
        .map(|memory_id| {
            let label = format!(
                "bias from m{} conf={:.2} val={:.2} aff={:.2} danger={:.2} curiosity={:.2}",
                memory_id.raw(),
                expectancy.confidence.raw(),
                expectancy.expected_valence.raw(),
                expectancy.affordance_bias.raw(),
                expectancy.danger_bias.raw(),
                expectancy.curiosity_bias.raw()
            );
            validate_ca29_display_line(&label)?;
            Ok(MemoryExpectancyBiasRow {
                source_memory_id: memory_id.raw(),
                confidence: expectancy.confidence.raw(),
                expected_valence: expectancy.expected_valence.raw(),
                affordance_bias: expectancy.affordance_bias.raw(),
                danger_bias: expectancy.danger_bias.raw(),
                curiosity_bias: expectancy.curiosity_bias.raw(),
                label,
            })
        })
        .collect()
}

fn validate_unit(value: f32) -> Result<(), ScaffoldContractError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn validate_unit_signed(value: f32) -> Result<(), ScaffoldContractError> {
    if value.is_finite() && (-1.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn validate_ca29_display_line(line: &str) -> Result<(), ScaffoldContractError> {
    if line.is_empty() || line.len() > 180 || line.contains("Entity(") {
        Err(ScaffoldContractError::InvalidId)
    } else {
        Ok(())
    }
}
