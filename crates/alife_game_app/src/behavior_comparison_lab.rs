//! CA31 player lab tools for behavior comparison.
//!
//! The lab runner compares isolated scenario copies and exports compact
//! reports. It does not mutate the active graphical runtime or train live
//! cognition.

use std::fs;

use crate::prelude::*;
use crate::*;

pub const CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA: &str = "alife.ca31.behavior_comparison_lab.v1";
pub const CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA_VERSION: u16 = 1;
pub const CA31_DEFAULT_COMPARISON_TICKS: u32 = 8;
pub const CA31_MAX_COMPARISON_TICKS: u32 = 16;
pub const CA31_MAX_REPORT_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorComparisonRun {
    pub scenario_id: String,
    pub scenario_title: String,
    pub fixture_root: PathBuf,
    pub ticks_requested: u32,
    pub ticks_completed: u32,
    pub object_count: usize,
    pub creature_count: usize,
    pub food_count: usize,
    pub hazard_count: usize,
    pub obstacle_count: usize,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub final_action: String,
    pub final_target: String,
    pub topology_signature: String,
    pub memory_signature: String,
    pub neural_signature: String,
    pub stable_world_signature: Vec<String>,
    pub behavior_signature: String,
    pub isolated_run: bool,
    pub report_only: bool,
    pub no_hidden_training_mutation: bool,
}

impl BehaviorComparisonRun {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.scenario_id.is_empty()
            || self.scenario_title.is_empty()
            || self.ticks_requested == 0
            || self.ticks_requested > CA31_MAX_COMPARISON_TICKS
            || self.ticks_completed > self.ticks_requested
            || self.object_count == 0
            || self.creature_count == 0
            || self.final_action.is_empty()
            || self.final_target.is_empty()
            || self.topology_signature.is_empty()
            || self.memory_signature.is_empty()
            || self.neural_signature.is_empty()
            || self.stable_world_signature.is_empty()
            || self.behavior_signature.is_empty()
            || !self.isolated_run
            || !self.report_only
            || !self.no_hidden_training_mutation
            || self.behavior_signature.contains("Entity(")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message:
                    "CA31 behavior comparison run must be bounded, stable-ID based, and report-only",
            });
        }
        Ok(())
    }

    pub fn compact_row(&self) -> String {
        format!(
            "{} ticks={}/{} creatures={} food={} hazards={} sealed={} action={} target={} sig={}",
            self.scenario_id,
            self.ticks_completed,
            self.ticks_requested,
            self.creature_count,
            self.food_count,
            self.hazard_count,
            self.sealed_patch_count,
            self.final_action,
            self.final_target,
            self.behavior_signature
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorComparisonPanel {
    pub headline: String,
    pub scenario_a_label: String,
    pub scenario_b_label: String,
    pub creature_delta: isize,
    pub food_delta: isize,
    pub hazard_delta: isize,
    pub sealed_patch_delta: isize,
    pub signatures_differ: bool,
    pub panel_text: String,
    pub stable_ids_only: bool,
    pub read_only: bool,
}

impl BehaviorComparisonPanel {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.headline != "Behavior Comparison Lab"
            || self.scenario_a_label.is_empty()
            || self.scenario_b_label.is_empty()
            || self.panel_text.is_empty()
            || !self.panel_text.contains("A/B Scenario Runner")
            || !self.panel_text.contains("Read-only report")
            || !self.panel_text.contains("No hidden training mutation")
            || !self.stable_ids_only
            || !self.read_only
            || self.panel_text.contains("Entity(")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA31 comparison panel must be readable, read-only, and stable-ID safe",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorComparisonLabSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub manifest_path: PathBuf,
    pub ticks: u32,
    pub scenario_a: BehaviorComparisonRun,
    pub scenario_b: BehaviorComparisonRun,
    pub panel: BehaviorComparisonPanel,
    pub report_markdown: String,
    pub report_bytes: usize,
    pub export_small_report_supported: bool,
    pub direct_cognition_mutation_allowed: bool,
    pub semantic_action_authority: bool,
    pub gpu_action_authority_claim: bool,
}

impl BehaviorComparisonLabSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA
            || self.schema_version != CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA_VERSION
            || self.ticks == 0
            || self.ticks > CA31_MAX_COMPARISON_TICKS
            || self.report_markdown.is_empty()
            || self.report_bytes == 0
            || self.report_bytes > CA31_MAX_REPORT_BYTES
            || !self.export_small_report_supported
            || self.direct_cognition_mutation_allowed
            || self.semantic_action_authority
            || self.gpu_action_authority_claim
            || self.report_markdown.contains("Entity(")
            || !self.report_markdown.contains("No hidden training mutation")
            || !self
                .report_markdown
                .contains("CPU shadow parity remains the gate")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA31 lab summary must stay bounded, honest, and report-only",
            });
        }
        self.scenario_a.validate()?;
        self.scenario_b.validate()?;
        self.panel.validate()?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema,
            self.schema_version,
            self.ticks,
            self.scenario_a.scenario_id,
            self.scenario_a.behavior_signature,
            self.scenario_b.scenario_id,
            self.scenario_b.behavior_signature
        )
    }
}

pub fn run_behavior_comparison_lab_smoke(
    manifest_path: impl AsRef<Path>,
    scenario_a: Option<&str>,
    scenario_b: Option<&str>,
    ticks: u32,
) -> Result<BehaviorComparisonLabSummary, GameAppShellError> {
    let manifest_path = manifest_path.as_ref();
    let bounded_ticks = ticks.clamp(1, CA31_MAX_COMPARISON_TICKS);
    let manifest = EnvironmentManifest::from_json_file(manifest_path)?;
    manifest.validate(manifest_path)?;
    let default_a = scenario_a.unwrap_or(&manifest.default_scenario_id);
    let default_b = scenario_b.unwrap_or_else(|| {
        manifest
            .scenarios
            .iter()
            .find(|scenario| scenario.id != default_a)
            .map_or(default_a, |scenario| scenario.id.as_str())
    });
    let selection_a = manifest.select(manifest_path, Some(default_a))?;
    let selection_b = manifest.select(manifest_path, Some(default_b))?;
    let run_a = run_behavior_comparison_scenario(selection_a, bounded_ticks)?;
    let run_b = run_behavior_comparison_scenario(selection_b, bounded_ticks)?;
    let panel = behavior_comparison_panel(&run_a, &run_b);
    let report_markdown = behavior_comparison_report_markdown(&run_a, &run_b, &panel);
    let report_bytes = report_markdown.len();
    let summary = BehaviorComparisonLabSummary {
        schema: CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA,
        schema_version: CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA_VERSION,
        manifest_path: manifest_path.to_path_buf(),
        ticks: bounded_ticks,
        scenario_a: run_a,
        scenario_b: run_b,
        panel,
        report_markdown,
        report_bytes,
        export_small_report_supported: true,
        direct_cognition_mutation_allowed: false,
        semantic_action_authority: false,
        gpu_action_authority_claim: false,
    };
    summary.validate()?;
    Ok(summary)
}

pub fn write_behavior_comparison_lab_report(
    summary: &BehaviorComparisonLabSummary,
    output_path: impl AsRef<Path>,
) -> Result<(), GameAppShellError> {
    summary.validate()?;
    let output_path = output_path.as_ref();
    if summary.report_bytes > CA31_MAX_REPORT_BYTES {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "CA31 comparison report exceeds small-report export cap",
        });
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, &summary.report_markdown)?;
    Ok(())
}

fn run_behavior_comparison_scenario(
    selection: EnvironmentScenarioSelection,
    ticks: u32,
) -> Result<BehaviorComparisonRun, GameAppShellError> {
    let visible = load_visible_world_from_p34_save(&selection.launch)?;
    compare_visible_world_to_headless(&visible)?;
    let mut live = LiveBrainLoop::from_p34_launch(&selection.launch)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let summaries = panel.apply_command(&mut live, RuntimeControlCommand::RunForTicks(ticks))?;
    let final_summary = summaries.last();
    let final_action = final_summary
        .and_then(|summary| summary.selected_action_kind)
        .map_or("None".to_string(), |kind| {
            action_badge_label_for_target(
                kind,
                final_summary
                    .and_then(|s| s.target_entity)
                    .map(|id| id.raw()),
            )
            .to_string()
        });
    let final_target = final_summary
        .and_then(|summary| summary.target_entity)
        .map_or_else(|| "none".to_string(), |id| format!("stable:{}", id.raw()));
    let behavior_signature = format!(
        "scenario={}:ticks={}:objects={}:creatures={}:food={}:hazards={}:sealed={}:packed={}:action={}:target={}:topology={}:memory={}:neural={}",
        selection.entry.id,
        summaries.len(),
        visible.object_count,
        visible.kind_count(WorldObjectKind::Agent),
        visible.kind_count(WorldObjectKind::Food),
        visible.kind_count(WorldObjectKind::Hazard),
        panel.sealed_patch_count,
        panel.packed_record_count,
        final_action,
        final_target,
        panel.topology_overlay.compact_line(),
        panel.memory_journal.compact_line(),
        panel.neural_profiler.compact_line()
    );
    let run = BehaviorComparisonRun {
        scenario_id: selection.entry.id,
        scenario_title: selection.entry.title,
        fixture_root: selection.launch.fixture_root,
        ticks_requested: ticks,
        ticks_completed: summaries.len() as u32,
        object_count: visible.object_count,
        creature_count: visible.kind_count(WorldObjectKind::Agent),
        food_count: visible.kind_count(WorldObjectKind::Food),
        hazard_count: visible.kind_count(WorldObjectKind::Hazard),
        obstacle_count: visible.kind_count(WorldObjectKind::Obstacle),
        sealed_patch_count: panel.sealed_patch_count,
        packed_record_count: panel.packed_record_count,
        final_action,
        final_target,
        topology_signature: panel.topology_overlay.signature_line(),
        memory_signature: panel.memory_journal.signature_line(),
        neural_signature: panel.neural_profiler.signature_line(),
        stable_world_signature: visible.visible_signature,
        behavior_signature,
        isolated_run: true,
        report_only: true,
        no_hidden_training_mutation: true,
    };
    run.validate()?;
    Ok(run)
}

fn behavior_comparison_panel(
    scenario_a: &BehaviorComparisonRun,
    scenario_b: &BehaviorComparisonRun,
) -> BehaviorComparisonPanel {
    let creature_delta = scenario_b.creature_count as isize - scenario_a.creature_count as isize;
    let food_delta = scenario_b.food_count as isize - scenario_a.food_count as isize;
    let hazard_delta = scenario_b.hazard_count as isize - scenario_a.hazard_count as isize;
    let sealed_patch_delta =
        scenario_b.sealed_patch_count as isize - scenario_a.sealed_patch_count as isize;
    let signatures_differ = scenario_a.behavior_signature != scenario_b.behavior_signature;
    let panel_text = format!(
        "Behavior Comparison Lab\nA/B Scenario Runner\nA: {}\nB: {}\nDeltas: creatures={:+} food={:+} hazards={:+} sealed={:+}\nRead-only report; stable IDs only.\nNo hidden training mutation; isolated copies only.\nCPU shadow parity remains the gate; no full action-authoritative GPU claim.",
        scenario_a.compact_row(),
        scenario_b.compact_row(),
        creature_delta,
        food_delta,
        hazard_delta,
        sealed_patch_delta
    );
    BehaviorComparisonPanel {
        headline: "Behavior Comparison Lab".to_string(),
        scenario_a_label: scenario_a.scenario_id.clone(),
        scenario_b_label: scenario_b.scenario_id.clone(),
        creature_delta,
        food_delta,
        hazard_delta,
        sealed_patch_delta,
        signatures_differ,
        panel_text,
        stable_ids_only: true,
        read_only: true,
    }
}

fn behavior_comparison_report_markdown(
    scenario_a: &BehaviorComparisonRun,
    scenario_b: &BehaviorComparisonRun,
    panel: &BehaviorComparisonPanel,
) -> String {
    format!(
        "# CA31 Behavior Comparison Lab Report\n\n\
         This report compares isolated scenario copies. It does not mutate the active graphical runtime, emit actions, train live cognition, or rewrite weights.\n\n\
         ## Panel\n\n\
         ```text\n{}\n```\n\n\
         ## Scenario Runs\n\n\
         | Side | Scenario | Ticks | Creatures | Food | Hazards | Obstacles | Sealed patches | Final action | Final target |\n\
         |---|---|---:|---:|---:|---:|---:|---:|---|---|\n\
         | A | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n\
         | B | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n\n\
         ## Behavior Signatures\n\n\
         - A: `{}`\n\
         - B: `{}`\n\n\
         ## Boundaries\n\n\
         - No hidden training mutation: true\n\
         - Stable IDs only: true\n\
         - Read-only comparison panel: true\n\
         - CPU shadow parity remains the gate.\n\
         - Product GPU claim is unchanged; no full action-authoritative GPU runtime is claimed.\n",
        panel.panel_text,
        scenario_a.scenario_id,
        scenario_a.ticks_completed,
        scenario_a.creature_count,
        scenario_a.food_count,
        scenario_a.hazard_count,
        scenario_a.obstacle_count,
        scenario_a.sealed_patch_count,
        scenario_a.final_action,
        scenario_a.final_target,
        scenario_b.scenario_id,
        scenario_b.ticks_completed,
        scenario_b.creature_count,
        scenario_b.food_count,
        scenario_b.hazard_count,
        scenario_b.obstacle_count,
        scenario_b.sealed_patch_count,
        scenario_b.final_action,
        scenario_b.final_target,
        scenario_a.behavior_signature,
        scenario_b.behavior_signature
    )
}
