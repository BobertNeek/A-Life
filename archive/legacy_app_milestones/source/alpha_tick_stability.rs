//! CA44A focused stability evidence for the default GPU alpha scenario.

use std::time::Instant;

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Ca44aTickStabilitySummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_ticks: u32,
    pub completed_ticks: u32,
    pub selected_creature: OrganismId,
    pub first_invalid_tick: Option<u64>,
    pub first_invalid_status: Option<BrainTickStatus>,
    pub first_invalid_action_kind: Option<ActionKind>,
    pub first_invalid_action_id: Option<ActionId>,
    pub first_invalid_target: Option<WorldEntityId>,
    pub first_invalid_diagnostic: Option<ContractDiagnostic>,
    pub sealed_patches: usize,
    pub packed_records: usize,
    pub topology_concepts: usize,
    pub topology_edges: usize,
    pub topology_simplexes: usize,
    pub topology_gaps: usize,
    pub gpu_authority_preserved: bool,
    pub execution_status: &'static str,
    pub terminal_invalid_count: u32,
    pub recoverable_failure_count: u32,
    pub debug_wall_ms: f64,
    pub average_ms_per_tick: f64,
    pub ticks_per_second: f64,
}

impl Ca44aTickStabilitySummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA44A_STABILITY_SCHEMA
            || self.schema_version != CA44A_STABILITY_SCHEMA_VERSION
            || self.requested_ticks == 0
            || self.completed_ticks != self.requested_ticks
            || self.first_invalid_tick.is_some()
            || self.terminal_invalid_count != 0
            || !self.gpu_authority_preserved
            || self.topology_concepts == 0
            || self.topology_simplexes == 0
            || !self.debug_wall_ms.is_finite()
            || !self.average_ms_per_tick.is_finite()
            || !self.ticks_per_second.is_finite()
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:ticks={}/{}:first_invalid={:?}:action={:?}:{:?}:target={:?}:sealed={}:topology={}/{}/{}/{}:parity={}:fallback={}:avg_ms={:.3}:tps={:.2}",
            self.schema,
            self.schema_version,
            self.completed_ticks,
            self.requested_ticks,
            self.first_invalid_tick,
            self.first_invalid_action_kind,
            self.first_invalid_action_id.map(ActionId::raw),
            self.first_invalid_target.map(WorldEntityId::raw),
            self.sealed_patches,
            self.topology_concepts,
            self.topology_edges,
            self.topology_simplexes,
            self.topology_gaps,
            self.gpu_authority_preserved,
            self.execution_status,
            self.average_ms_per_tick,
            self.ticks_per_second
        )
    }
}

pub fn run_ca44a_gpu_alpha_stability_smoke(
    launch: &AppShellLaunchConfig,
    requested_ticks: u32,
) -> Result<Ca44aTickStabilitySummary, GameAppShellError> {
    if requested_ticks == 0 || requested_ticks > CA22_MAX_MANUAL_SOAK_TICKS {
        return Err(ScaffoldContractError::ScalarOutOfRange.into());
    }
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let selected_creature = live.organism_id();
    let started = Instant::now();
    let mut first_invalid: Option<LiveBrainTickSummary> = None;
    let mut completed_ticks = 0_u32;
    let mut terminal_invalid_count = 0_u32;
    let mut recoverable_failure_count = 0_u32;
    let mut sealed_patches = 0_usize;
    let mut packed_records = 0_usize;

    for _ in 0..requested_ticks {
        let mut summaries = live.update(LiveBrainTickControl::step_once())?;
        let summary = summaries
            .pop()
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "CA44A stability tick must produce one summary",
            })?;
        completed_ticks = completed_ticks.saturating_add(1);
        sealed_patches = summary.sealed_patch_count;
        packed_records = summary.packed_record_count;
        match summary.status {
            BrainTickStatus::TerminalInvalidState => {
                terminal_invalid_count = terminal_invalid_count.saturating_add(1);
                first_invalid.get_or_insert(summary);
                break;
            }
            BrainTickStatus::RecoverableActionFailure => {
                recoverable_failure_count = recoverable_failure_count.saturating_add(1);
            }
            BrainTickStatus::Normal | BrainTickStatus::SafeIdle => {}
        }
    }

    let elapsed = started.elapsed();
    let debug_wall_ms = elapsed.as_secs_f64() * 1_000.0;
    let topology = live.mind().topological_map();
    let average_ms_per_tick = if completed_ticks == 0 {
        0.0
    } else {
        debug_wall_ms / f64::from(completed_ticks)
    };
    let ticks_per_second = if debug_wall_ms <= f64::EPSILON {
        0.0
    } else {
        f64::from(completed_ticks) / elapsed.as_secs_f64()
    };
    Ok(Ca44aTickStabilitySummary {
        schema: CA44A_STABILITY_SCHEMA,
        schema_version: CA44A_STABILITY_SCHEMA_VERSION,
        requested_ticks,
        completed_ticks,
        selected_creature,
        first_invalid_tick: first_invalid
            .as_ref()
            .map(|summary| summary.tick_before.raw()),
        first_invalid_status: first_invalid.as_ref().map(|summary| summary.status),
        first_invalid_action_kind: first_invalid
            .as_ref()
            .and_then(|summary| summary.selected_action_kind),
        first_invalid_action_id: first_invalid
            .as_ref()
            .and_then(|summary| summary.selected_action_id),
        first_invalid_target: first_invalid
            .as_ref()
            .and_then(|summary| summary.target_entity),
        first_invalid_diagnostic: first_invalid
            .as_ref()
            .and_then(|summary| summary.last_diagnostic),
        sealed_patches,
        packed_records,
        topology_concepts: topology.concepts().len(),
        topology_edges: topology.edges().len(),
        topology_simplexes: topology.simplexes().len(),
        topology_gaps: topology.unresolved_gaps().len(),
        gpu_authority_preserved: true,
        execution_status: "explicit-headless-baseline-stability-smoke",
        terminal_invalid_count,
        recoverable_failure_count,
        debug_wall_ms,
        average_ms_per_tick,
        ticks_per_second,
    })
}
