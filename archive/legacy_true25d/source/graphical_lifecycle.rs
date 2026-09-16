//! CA20 graphical lifecycle and lineage presentation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Ca20LifecyclePanelRow {
    pub stable_id: Option<WorldEntityId>,
    pub label: String,
    pub status: String,
}

impl Ca20LifecyclePanelRow {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if let Some(stable_id) = self.stable_id {
            stable_id.validate()?;
        }
        if self.label.is_empty()
            || self.status.is_empty()
            || self.label.contains("Entity(")
            || self.status.contains("Entity(")
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}",
            self.stable_id
                .map(|id| id.raw().to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.label,
            self.status
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca20GraphicalLifecycleSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub living_population: usize,
    pub population_cap: usize,
    pub births: usize,
    pub deaths: usize,
    pub reproduction_blocked_count: usize,
    pub lineage_count: usize,
    pub selected_stable_id: Option<WorldEntityId>,
    pub genetic_lifetime_separated: bool,
    pub birth_weight_assets_are_initializers: bool,
    pub save_load_lineages_roundtrip: bool,
    pub event_rows: Vec<Ca20LifecyclePanelRow>,
    pub lineage_rows: Vec<Ca20LifecyclePanelRow>,
    pub signature: String,
}

impl Ca20GraphicalLifecycleSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA20_GRAPHICAL_LIFECYCLE_SCHEMA
            || self.schema_version != CA20_GRAPHICAL_LIFECYCLE_SCHEMA_VERSION
            || self.living_population > self.population_cap
            || self.population_cap > G09_MAX_LIFECYCLE_POPULATION_CAP
            || self.lineage_count == 0
            || self.event_rows.is_empty()
            || self.event_rows.len() > CA20_MAX_LIFECYCLE_EVENT_ROWS
            || self.lineage_rows.is_empty()
            || !self.genetic_lifetime_separated
            || !self.birth_weight_assets_are_initializers
            || !self.save_load_lineages_roundtrip
            || self.signature.contains("Entity(")
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self
            .selected_stable_id
            .is_some_and(|id| id.validate().is_err())
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        for row in &self.event_rows {
            row.validate()?;
        }
        for row in &self.lineage_rows {
            row.validate()?;
        }
        Ok(())
    }

    pub fn compact_overlay_text(&self) -> String {
        format!(
            concat!(
                "Lifecycle\n",
                "Pop: {}/{}  Births:{} Deaths:{} Blocked:{}\n",
                "Selected: {}  Lineages:{}\n",
                "Genetic fixed separate from lifetime: {}\n",
                "Save/load lineages: {}\n",
                "Events: {}\n",
                "Lineage: {}"
            ),
            self.living_population,
            self.population_cap,
            self.births,
            self.deaths,
            self.reproduction_blocked_count,
            self.selected_stable_id
                .map(|id| format!("stable:{}", id.raw()))
                .unwrap_or_else(|| "none".to_string()),
            self.lineage_count,
            self.genetic_lifetime_separated,
            self.save_load_lineages_roundtrip,
            self.event_rows
                .iter()
                .map(|row| row.status.as_str())
                .collect::<Vec<_>>()
                .join(" | "),
            self.lineage_rows
                .iter()
                .map(|row| row.status.as_str())
                .collect::<Vec<_>>()
                .join(" | "),
        )
    }
}

pub fn ca20_graphical_lifecycle_summary() -> Result<Ca20GraphicalLifecycleSummary, GameAppShellError>
{
    let lifecycle = run_lifecycle_lineage_smoke()?;
    ca20_graphical_lifecycle_summary_from_lifecycle(&lifecycle)
}

pub fn run_graphical_lifecycle_smoke() -> Result<Ca20GraphicalLifecycleSummary, GameAppShellError> {
    ca20_graphical_lifecycle_summary()
}

pub fn ca20_graphical_lifecycle_summary_from_lifecycle(
    lifecycle: &LifecycleLineageSummary,
) -> Result<Ca20GraphicalLifecycleSummary, GameAppShellError> {
    lifecycle.validate()?;
    let genetic_lifetime_separated = lifecycle
        .creatures
        .iter()
        .all(|record| !record.lamarckian_enabled && !record.inherited_lifetime_state)
        && lifecycle
            .lineage_records
            .iter()
            .all(|record| !record.lamarckian_enabled && !record.inherited_lifetime_state);
    let birth_weight_assets_are_initializers = lifecycle
        .lineage_records
        .iter()
        .all(|record| record.birth_weight_asset_id.is_some());
    let save_load_lineages_roundtrip = LifecycleSaveState::from_summary(lifecycle)?
        .signature_line()
        == lifecycle.save_roundtrip_signature;
    let event_rows = lifecycle
        .events
        .iter()
        .rev()
        .take(CA20_MAX_LIFECYCLE_EVENT_ROWS)
        .map(|event| Ca20LifecyclePanelRow {
            stable_id: event.stable_id,
            label: event.kind.label().to_string(),
            status: format!(
                "{} stable:{}",
                event.kind.label(),
                event
                    .stable_id
                    .map(|id| id.raw().to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
        })
        .collect::<Vec<_>>();
    let lineage_rows = lifecycle
        .lineage_records
        .iter()
        .map(|record| Ca20LifecyclePanelRow {
            stable_id: None,
            label: format!("lineage-{}", record.lineage_id.raw()),
            status: format!(
                "gen{} child genome:{} parents:{}+{}",
                record.generation,
                record.offspring_genome_id.raw(),
                record.parent_genome_ids[0].raw(),
                record.parent_genome_ids[1].raw()
            ),
        })
        .collect::<Vec<_>>();
    let signature = format!(
        "{}:{}:living={}:cap={}:births={}:deaths={}:lineages={}:roundtrip={}:genetic_lifetime_separate={}",
        CA20_GRAPHICAL_LIFECYCLE_SCHEMA,
        CA20_GRAPHICAL_LIFECYCLE_SCHEMA_VERSION,
        lifecycle.metrics.living_population,
        lifecycle.population_cap,
        lifecycle.metrics.births,
        lifecycle.metrics.deaths,
        lifecycle.lineage_records.len(),
        save_load_lineages_roundtrip,
        genetic_lifetime_separated,
    );
    let summary = Ca20GraphicalLifecycleSummary {
        schema: CA20_GRAPHICAL_LIFECYCLE_SCHEMA,
        schema_version: CA20_GRAPHICAL_LIFECYCLE_SCHEMA_VERSION,
        living_population: lifecycle.metrics.living_population,
        population_cap: lifecycle.population_cap,
        births: lifecycle.metrics.births,
        deaths: lifecycle.metrics.deaths,
        reproduction_blocked_count: lifecycle.metrics.reproduction_blocked_count,
        lineage_count: lifecycle.lineage_records.len(),
        selected_stable_id: lifecycle.selected_stable_id,
        genetic_lifetime_separated,
        birth_weight_assets_are_initializers,
        save_load_lineages_roundtrip,
        event_rows,
        lineage_rows,
        signature,
    };
    summary.validate()?;
    Ok(summary)
}
