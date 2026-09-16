//! S07 product-facing aggregation for advanced optional gameplay systems.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedGameplaySocialPanel {
    pub creature_count: usize,
    pub population_cap: usize,
    pub social_context_samples: usize,
    pub vocal_tokens_heard: usize,
    pub collision_feedback_count: usize,
    pub schedule_order: Vec<OrganismId>,
    pub perception_only: bool,
    pub direct_action_bypass_count: usize,
    pub display_lines: Vec<String>,
}

impl AdvancedGameplaySocialPanel {
    fn from_summary(summary: &PopulationSocialLoopSummary) -> Result<Self, ScaffoldContractError> {
        summary.validate()?;
        let direct_action_bypass_count = summary
            .tick_records
            .iter()
            .map(|record| record.social_direct_action_count)
            .sum();
        let display_lines = vec![
            bounded_line(format!(
                "creatures={}/{} schedule={}",
                summary.creature_count,
                summary.population_cap,
                summary
                    .schedule_order
                    .iter()
                    .map(|id| id.raw().to_string())
                    .collect::<Vec<_>>()
                    .join(">")
            ))?,
            bounded_line(format!(
                "social_samples={} vocal_tokens={} collisions={}",
                summary.metrics.social_context_samples,
                summary.metrics.vocal_tokens_heard,
                summary.metrics.collision_feedback_count
            ))?,
            bounded_line("boundary=perception/modulatory only; no direct action path")?,
        ];
        let panel = Self {
            creature_count: summary.creature_count,
            population_cap: summary.population_cap,
            social_context_samples: summary.metrics.social_context_samples,
            vocal_tokens_heard: summary.metrics.vocal_tokens_heard,
            collision_feedback_count: summary.metrics.collision_feedback_count,
            schedule_order: summary.schedule_order.clone(),
            perception_only: true,
            direct_action_bypass_count,
            display_lines,
        };
        panel.validate()?;
        Ok(panel)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.creature_count < 2
            || self.creature_count > self.population_cap
            || self.population_cap > G08_MAX_POPULATION_CAP
            || self.schedule_order.len() != self.creature_count
            || !self.perception_only
            || self.direct_action_bypass_count != 0
            || self.display_lines.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for id in &self.schedule_order {
            id.validate()?;
        }
        validate_display_lines(&self.display_lines)
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.creature_count,
            self.population_cap,
            self.social_context_samples,
            self.vocal_tokens_heard,
            self.direct_action_bypass_count
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedGameplayLifecyclePanel {
    pub living_population: usize,
    pub births: usize,
    pub deaths: usize,
    pub reproduction_blocked_count: usize,
    pub selected_stable_id: Option<WorldEntityId>,
    pub lineage_count: usize,
    pub genetic_lifetime_separated: bool,
    pub birth_weight_assets_are_initializers: bool,
    pub display_lines: Vec<String>,
}

impl AdvancedGameplayLifecyclePanel {
    fn from_summary(summary: &LifecycleLineageSummary) -> Result<Self, ScaffoldContractError> {
        summary.validate()?;
        let genetic_lifetime_separated = summary
            .creatures
            .iter()
            .all(|record| !record.lamarckian_enabled && !record.inherited_lifetime_state)
            && summary
                .lineage_records
                .iter()
                .all(|record| !record.lamarckian_enabled && !record.inherited_lifetime_state);
        let birth_weight_assets_are_initializers = summary
            .lineage_records
            .iter()
            .all(|record| record.birth_weight_asset_id.is_some());
        let display_lines = vec![
            bounded_line(format!(
                "living={} births={} deaths={} blocked={}",
                summary.metrics.living_population,
                summary.metrics.births,
                summary.metrics.deaths,
                summary.metrics.reproduction_blocked_count
            ))?,
            bounded_line(format!(
                "selected={} lineages={} save_roundtrip={}",
                summary
                    .selected_stable_id
                    .map(|id| id.raw().to_string())
                    .unwrap_or_else(|| "none".to_string()),
                summary.lineage_records.len(),
                !summary.save_roundtrip_signature.is_empty()
            ))?,
            bounded_line("boundary=birth assets initialize only; lifetime state not inherited")?,
        ];
        let panel = Self {
            living_population: summary.metrics.living_population,
            births: summary.metrics.births,
            deaths: summary.metrics.deaths,
            reproduction_blocked_count: summary.metrics.reproduction_blocked_count,
            selected_stable_id: summary.selected_stable_id,
            lineage_count: summary.lineage_records.len(),
            genetic_lifetime_separated,
            birth_weight_assets_are_initializers,
            display_lines,
        };
        panel.validate()?;
        Ok(panel)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.living_population == 0
            || self.lineage_count == 0
            || !self.genetic_lifetime_separated
            || !self.birth_weight_assets_are_initializers
            || self.display_lines.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if let Some(stable_id) = self.selected_stable_id {
            stable_id.validate()?;
        }
        validate_display_lines(&self.display_lines)
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.living_population,
            self.births,
            self.deaths,
            self.lineage_count,
            self.genetic_lifetime_separated
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedGameplaySchoolPanel {
    pub curriculum_id: String,
    pub active_lesson_id: LessonId,
    pub cue_count: usize,
    pub verifier_passed: bool,
    pub sealed_patch_count: usize,
    pub teacher_avatar_stable_id: WorldEntityId,
    pub learner_stable_id: WorldEntityId,
    pub perception_only: bool,
    pub direct_motor_bypass_blocked: bool,
    pub display_lines: Vec<String>,
}

impl AdvancedGameplaySchoolPanel {
    fn from_summary(summary: &SchoolModeSummary) -> Result<Self, ScaffoldContractError> {
        summary.validate()?;
        let perception_only = summary.cues.iter().all(|cue| cue.perception_only);
        let direct_motor_bypass_blocked =
            summary.teacher_metadata_bypass_blocked && summary.teacher_selected_action_id.is_none();
        let display_lines = vec![
            bounded_line(format!(
                "curriculum={} lesson={} cues={}",
                summary.lesson_panel.curriculum_id,
                summary.lesson_panel.active_lesson_id.raw(),
                summary.cues.len()
            ))?,
            bounded_line(format!(
                "verifier={} sealed_patches={} channels={}",
                summary.verifier_panel.passed,
                summary.verifier_panel.sealed_patch_count,
                summary.sensory_teacher_channels.len()
            ))?,
            bounded_line("boundary=teacher cues enter perception; verifier reads sealed patches")?,
        ];
        let panel = Self {
            curriculum_id: summary.lesson_panel.curriculum_id.clone(),
            active_lesson_id: summary.lesson_panel.active_lesson_id,
            cue_count: summary.cues.len(),
            verifier_passed: summary.verifier_panel.passed,
            sealed_patch_count: summary.verifier_panel.sealed_patch_count,
            teacher_avatar_stable_id: summary.teacher_avatar_stable_id,
            learner_stable_id: summary.learner_stable_id,
            perception_only,
            direct_motor_bypass_blocked,
            display_lines,
        };
        panel.validate()?;
        Ok(panel)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.teacher_avatar_stable_id.validate()?;
        self.learner_stable_id.validate()?;
        if self.curriculum_id.is_empty()
            || self.active_lesson_id.raw() == 0
            || self.cue_count == 0
            || self.sealed_patch_count == 0
            || !self.perception_only
            || !self.direct_motor_bypass_blocked
            || self.display_lines.is_empty()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        validate_display_lines(&self.display_lines)
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.curriculum_id,
            self.active_lesson_id.raw(),
            self.cue_count,
            self.verifier_passed,
            self.direct_motor_bypass_blocked
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedGameplaySemanticPanel {
    pub disabled_provider_nonfatal: bool,
    pub fake_provider_available: bool,
    pub context_visible: bool,
    pub display_line_count: usize,
    pub semantic_action_bypass_blocked: bool,
    pub weight_rewrite_blocked: bool,
    pub display_lines: Vec<String>,
}

impl AdvancedGameplaySemanticPanel {
    fn from_summary(summary: &SemanticProviderSmokeSummary) -> Result<Self, ScaffoldContractError> {
        summary.validate()?;
        let display_lines = vec![
            bounded_line(format!(
                "disabled_nonfatal={} fake_provider={} context_visible={}",
                summary.provider_absence_nonfatal,
                summary.fake_panel.manifest.available,
                summary.fake_panel.context_visible
            ))?,
            bounded_line(format!(
                "semantic_lines={} codes={} clusters={}",
                summary.fake_panel.display_lines.len(),
                summary.fake_panel.semantic_code_count,
                summary.fake_panel.gaussian_cluster_count
            ))?,
            bounded_line("boundary=optional context only; cannot act or rewrite weights")?,
        ];
        let panel = Self {
            disabled_provider_nonfatal: summary.provider_absence_nonfatal,
            fake_provider_available: summary.fake_panel.manifest.available,
            context_visible: summary.fake_panel.context_visible,
            display_line_count: summary.fake_panel.display_lines.len(),
            semantic_action_bypass_blocked: summary.semantic_action_bypass_blocked,
            weight_rewrite_blocked: summary.weight_rewrite_blocked,
            display_lines,
        };
        panel.validate()?;
        Ok(panel)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if !self.disabled_provider_nonfatal
            || !self.fake_provider_available
            || !self.context_visible
            || self.display_line_count == 0
            || !self.semantic_action_bypass_blocked
            || !self.weight_rewrite_blocked
            || self.display_lines.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        validate_display_lines(&self.display_lines)
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.disabled_provider_nonfatal,
            self.fake_provider_available,
            self.display_line_count,
            self.semantic_action_bypass_blocked,
            self.weight_rewrite_blocked
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedGameplayUxSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub social: AdvancedGameplaySocialPanel,
    pub lifecycle: AdvancedGameplayLifecyclePanel,
    pub school: AdvancedGameplaySchoolPanel,
    pub semantic: AdvancedGameplaySemanticPanel,
    pub display_only: bool,
    pub optional_modes: bool,
    pub no_action_or_weight_bypass: bool,
    pub manual_screenshot_status: String,
    pub report_lines: Vec<String>,
}

impl AdvancedGameplayUxSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != S07_ADVANCED_GAMEPLAY_UX_SCHEMA
            || self.schema_version != S07_ADVANCED_GAMEPLAY_UX_SCHEMA_VERSION
            || !self.display_only
            || !self.optional_modes
            || !self.no_action_or_weight_bypass
            || self.manual_screenshot_status.is_empty()
            || self.report_lines.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.social.validate()?;
        self.lifecycle.validate()?;
        self.school.validate()?;
        self.semantic.validate()?;
        validate_display_lines(&self.report_lines)
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.social.signature_line(),
            self.lifecycle.signature_line(),
            self.school.signature_line(),
            self.semantic.signature_line(),
            self.no_action_or_weight_bypass
        )
    }
}

pub fn run_advanced_gameplay_ux_smoke() -> Result<AdvancedGameplayUxSummary, GameAppShellError> {
    let population = run_population_social_loop_smoke()?;
    let lifecycle = run_lifecycle_lineage_smoke()?;
    let school = run_school_mode_smoke()?;
    let semantic = run_semantic_provider_smoke()?;

    let summary = AdvancedGameplayUxSummary {
        schema: S07_ADVANCED_GAMEPLAY_UX_SCHEMA,
        schema_version: S07_ADVANCED_GAMEPLAY_UX_SCHEMA_VERSION,
        social: AdvancedGameplaySocialPanel::from_summary(&population)?,
        lifecycle: AdvancedGameplayLifecyclePanel::from_summary(&lifecycle)?,
        school: AdvancedGameplaySchoolPanel::from_summary(&school)?,
        semantic: AdvancedGameplaySemanticPanel::from_summary(&semantic)?,
        display_only: true,
        optional_modes: true,
        no_action_or_weight_bypass: true,
        manual_screenshot_status: "manual-gui-evidence-required-for-product-claim".to_string(),
        report_lines: vec![
            bounded_line("S07 aggregates G08-G11 into one player-facing advanced systems panel")?,
            bounded_line("Social, school, and semantic signals remain perception/context only")?,
            bounded_line("Lifecycle display preserves genetic/lifetime separation")?,
        ],
    };
    summary.validate()?;
    Ok(summary)
}

pub fn advanced_gameplay_overlay_text(summary: &AdvancedGameplayUxSummary) -> String {
    format!(
        concat!(
            "Advanced Systems (S07): ",
            "Social:{}/{} tok{} | ",
            "Lifecycle:{} b{} d{} | ",
            "School:cues{} sealed{} | ",
            "Semantic:optional cannot act or rewrite weights | ",
            "display_only={} optional={} no_action_or_weight_bypass={}"
        ),
        summary.social.creature_count,
        summary.social.population_cap,
        summary.social.vocal_tokens_heard,
        summary.lifecycle.living_population,
        summary.lifecycle.births,
        summary.lifecycle.deaths,
        summary.school.cue_count,
        summary.school.sealed_patch_count,
        summary.display_only,
        summary.optional_modes,
        summary.no_action_or_weight_bypass
    )
}

fn bounded_line(line: impl Into<String>) -> Result<String, ScaffoldContractError> {
    let line = line.into();
    if line.is_empty() || line.len() > 240 || line.contains("Entity(") {
        return Err(ScaffoldContractError::InvalidId);
    }
    Ok(line)
}

fn validate_display_lines(lines: &[String]) -> Result<(), ScaffoldContractError> {
    if lines.is_empty() || lines.len() > 8 {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    for line in lines {
        if line.is_empty() || line.len() > 240 || line.contains("Entity(") {
            return Err(ScaffoldContractError::InvalidId);
        }
    }
    Ok(())
}
