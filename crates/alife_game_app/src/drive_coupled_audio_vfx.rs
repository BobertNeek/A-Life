//! CA39 display-only drive-coupled audio and VFX cue mapping.
//!
//! This module does not play audio or mutate simulation state. It maps existing
//! sealed feedback events and runtime learning telemetry into player-readable
//! cue descriptors that Bevy can present.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Ca39DriveCueKind {
    HungerSatisfaction,
    HazardPain,
    SleepRest,
    LearningPulse,
}

impl Ca39DriveCueKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::HungerSatisfaction => "hunger-satisfaction",
            Self::HazardPain => "hazard-pain",
            Self::SleepRest => "sleep-rest",
            Self::LearningPulse => "learning-pulse",
        }
    }

    pub const fn player_label(self) -> &'static str {
        match self {
            Self::HungerSatisfaction => "Food chime",
            Self::HazardPain => "Hazard pulse",
            Self::SleepRest => "Rest bloom",
            Self::LearningPulse => "Learning pulse",
        }
    }

    pub const fn drive_channel(self) -> &'static str {
        match self {
            Self::HungerSatisfaction => "hunger",
            Self::HazardPain => "pain/fear",
            Self::SleepRest => "fatigue/sleep",
            Self::LearningPulse => "H_shadow",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca39RuntimeCueEvidence {
    pub selected_backend: String,
    pub unavailable_reason: Option<String>,
    pub authoritative: bool,
    pub sealed_patches: usize,
    pub learning_updates: u32,
    pub no_active_bulk_readback: bool,
}

impl Ca39RuntimeCueEvidence {
    pub fn from_graphical_gpu(gpu: &GraphicalGpuRuntimeTelemetry) -> Self {
        Self {
            selected_backend: gpu.selected_backend.clone(),
            unavailable_reason: gpu.unavailable_reason.clone(),
            authoritative: gpu.authoritative,
            sealed_patches: gpu.sealed_patches,
            learning_updates: gpu.learning_updates,
            no_active_bulk_readback: gpu.no_active_bulk_readback,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca39DriveCue {
    pub schema: &'static str,
    pub schema_version: u16,
    pub kind: Ca39DriveCueKind,
    pub active: bool,
    pub target_stable_id: Option<WorldEntityId>,
    pub source_milestone: String,
    pub drive_channel: &'static str,
    pub audio_asset_id: Option<String>,
    pub vfx_asset_id: Option<String>,
    pub audio_stub_label: &'static str,
    pub vfx_stub_label: &'static str,
    pub intensity: FeedbackIntensity,
    pub display_only: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
}

impl Ca39DriveCue {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA39_DRIVE_AUDIO_VFX_SCHEMA
            || self.schema_version != CA39_DRIVE_AUDIO_VFX_SCHEMA_VERSION
            || self.source_milestone.is_empty()
            || self.drive_channel.is_empty()
            || self.audio_stub_label.is_empty()
            || self.vfx_stub_label.is_empty()
            || !self.display_only
            || !self.no_action_authority
            || !self.no_weight_authority
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if let Some(stable_id) = self.target_stable_id {
            stable_id.validate()?;
        }
        Ok(())
    }

    pub fn compact_line(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.kind.player_label(),
            if self.active { "on" } else { "ready" },
            self.audio_asset_id
                .as_deref()
                .unwrap_or(self.audio_stub_label),
            self.vfx_asset_id.as_deref().unwrap_or(self.vfx_stub_label)
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca39DriveAudioVfxSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub cues: Vec<Ca39DriveCue>,
    pub active_cue_count: usize,
    pub audio_cue_count: usize,
    pub vfx_cue_count: usize,
    pub sealed_feedback_sources: usize,
    pub selected_backend: String,
    pub unavailable_reason: Option<String>,
    pub authoritative: bool,
    pub learning_updates: u32,
    pub no_active_bulk_readback: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
    pub no_cognition_mutation: bool,
    pub no_large_assets_added: bool,
}

impl Ca39DriveAudioVfxSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA39_DRIVE_AUDIO_VFX_SCHEMA
            || self.schema_version != CA39_DRIVE_AUDIO_VFX_SCHEMA_VERSION
            || self.cues.len() != CA39_REQUIRED_DRIVE_CUE_COUNT
            || self.audio_cue_count < CA39_REQUIRED_DRIVE_CUE_COUNT
            || self.vfx_cue_count < CA39_REQUIRED_DRIVE_CUE_COUNT
            || self.sealed_feedback_sources == 0
            || self.selected_backend.is_empty()
            || !self.authoritative
            || !self.no_active_bulk_readback
            || !self.no_action_authority
            || !self.no_weight_authority
            || !self.no_cognition_mutation
            || !self.no_large_assets_added
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        for required in [
            Ca39DriveCueKind::HungerSatisfaction,
            Ca39DriveCueKind::HazardPain,
            Ca39DriveCueKind::SleepRest,
            Ca39DriveCueKind::LearningPulse,
        ] {
            if !self.cues.iter().any(|cue| cue.kind == required) {
                return Err(ScaffoldContractError::MissingPhaseData);
            }
        }
        for cue in &self.cues {
            cue.validate()?;
        }
        Ok(())
    }

    pub fn active_labels(&self) -> Vec<&'static str> {
        self.cues
            .iter()
            .filter(|cue| cue.active)
            .map(|cue| cue.kind.label())
            .collect()
    }

    pub fn compact_overlay_text(&self) -> String {
        format!(
            concat!(
                "Drive Audio/VFX\n",
                "{}\n",
                "Backend: {} unavailable={}\n",
                "Learning updates: {} cue={}\n",
                "Boundary: display-only; GPU authoritative; no actions/weights"
            ),
            self.cues
                .iter()
                .map(Ca39DriveCue::compact_line)
                .collect::<Vec<_>>()
                .join(" | "),
            self.selected_backend,
            self.unavailable_reason.as_deref().unwrap_or("none"),
            self.learning_updates,
            if self.learning_updates > 0 {
                "on"
            } else {
                "ready"
            },
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.cues
                .iter()
                .map(|cue| cue.kind.label())
                .collect::<Vec<_>>()
                .join(">"),
            self.active_labels().join(">"),
            self.selected_backend,
            self.learning_updates,
            self.authoritative
        )
    }
}

pub fn ca39_drive_audio_vfx_summary(
    feedback: &FeedbackPolishSummary,
    evidence: &Ca39RuntimeCueEvidence,
) -> Result<Ca39DriveAudioVfxSummary, ScaffoldContractError> {
    feedback.validate()?;
    let cues = vec![
        cue_from_feedback(feedback, CA39_HUNGER_SATISFACTION_SPEC),
        cue_from_feedback(feedback, CA39_HAZARD_PAIN_SPEC),
        cue_from_feedback(feedback, CA39_SLEEP_REST_SPEC),
        learning_pulse_cue(evidence),
    ];
    let active_cue_count = cues.iter().filter(|cue| cue.active).count();
    let audio_cue_count = cues
        .iter()
        .filter(|cue| cue.audio_asset_id.is_some() || !cue.audio_stub_label.is_empty())
        .count();
    let vfx_cue_count = cues
        .iter()
        .filter(|cue| cue.vfx_asset_id.is_some() || !cue.vfx_stub_label.is_empty())
        .count();
    let summary = Ca39DriveAudioVfxSummary {
        schema: CA39_DRIVE_AUDIO_VFX_SCHEMA,
        schema_version: CA39_DRIVE_AUDIO_VFX_SCHEMA_VERSION,
        cues,
        active_cue_count,
        audio_cue_count,
        vfx_cue_count,
        sealed_feedback_sources: feedback.sealed_outcome_event_count,
        selected_backend: evidence.selected_backend.clone(),
        unavailable_reason: evidence.unavailable_reason.clone(),
        authoritative: evidence.authoritative,
        learning_updates: evidence.learning_updates,
        no_active_bulk_readback: evidence.no_active_bulk_readback,
        no_action_authority: true,
        no_weight_authority: true,
        no_cognition_mutation: true,
        no_large_assets_added: true,
    };
    summary.validate()?;
    Ok(summary)
}

pub fn ca39_drive_audio_vfx_summary_from_graphical(
    feedback: &FeedbackPolishSummary,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> Result<Ca39DriveAudioVfxSummary, ScaffoldContractError> {
    ca39_drive_audio_vfx_summary(feedback, &Ca39RuntimeCueEvidence::from_graphical_gpu(gpu))
}

pub fn ca39_drive_audio_vfx_panel_text(
    feedback: &FeedbackPolishSummary,
    evidence: &Ca39RuntimeCueEvidence,
) -> Result<String, ScaffoldContractError> {
    Ok(ca39_drive_audio_vfx_summary(feedback, evidence)?.compact_overlay_text())
}

pub fn ca39_drive_audio_vfx_panel_text_from_graphical(
    feedback: &FeedbackPolishSummary,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> Result<String, ScaffoldContractError> {
    Ok(ca39_drive_audio_vfx_summary_from_graphical(feedback, gpu)?.compact_overlay_text())
}

pub fn run_drive_coupled_audio_vfx_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<Ca39DriveAudioVfxSummary, GameAppShellError> {
    let feedback = run_feedback_polish_smoke(launch)?;
    let mut telemetry = GraphicalGpuRuntimeTelemetry::pending("N2048");
    telemetry.authoritative = true;
    telemetry.sealed_patches = 3;
    let evidence = Ca39RuntimeCueEvidence::from_graphical_gpu(&telemetry);
    Ok(ca39_drive_audio_vfx_summary(&feedback, &evidence)?)
}

#[derive(Debug, Clone, Copy)]
struct Ca39FeedbackCueSpec {
    event_kind: FeedbackEventKind,
    cue_kind: Ca39DriveCueKind,
    source_milestone: &'static str,
    audio_stub_label: &'static str,
    vfx_stub_label: &'static str,
    default_audio_asset_id: &'static str,
    default_vfx_asset_id: &'static str,
    intensity: FeedbackIntensity,
    required_active: bool,
}

const CA39_HUNGER_SATISFACTION_SPEC: Ca39FeedbackCueSpec = Ca39FeedbackCueSpec {
    event_kind: FeedbackEventKind::FoodReward,
    cue_kind: Ca39DriveCueKind::HungerSatisfaction,
    source_milestone: "food reward: hunger satisfaction after sealed outcome",
    audio_stub_label: "soft food reward chime",
    vfx_stub_label: "green food spark",
    default_audio_asset_id: "g17-audio-food-chime",
    default_vfx_asset_id: "g17-vfx-food-spark",
    intensity: FeedbackIntensity::High,
    required_active: true,
};

const CA39_HAZARD_PAIN_SPEC: Ca39FeedbackCueSpec = Ca39FeedbackCueSpec {
    event_kind: FeedbackEventKind::HazardPain,
    cue_kind: Ca39DriveCueKind::HazardPain,
    source_milestone: "hazard pain: fear and pain made visible",
    audio_stub_label: "sharp hazard warning pulse",
    vfx_stub_label: "red hazard flash",
    default_audio_asset_id: "g17-audio-hazard-pulse",
    default_vfx_asset_id: "g17-vfx-hazard-flash",
    intensity: FeedbackIntensity::High,
    required_active: true,
};

const CA39_SLEEP_REST_SPEC: Ca39FeedbackCueSpec = Ca39FeedbackCueSpec {
    event_kind: FeedbackEventKind::SleepTransition,
    cue_kind: Ca39DriveCueKind::SleepRest,
    source_milestone: "sleep/rest: recovery state entered after sealed outcome",
    audio_stub_label: "soft rest chime",
    vfx_stub_label: "blue sleep bloom",
    default_audio_asset_id: "g17-audio-sleep-soft",
    default_vfx_asset_id: "g17-vfx-sleep-bloom",
    intensity: FeedbackIntensity::Medium,
    required_active: true,
};

fn cue_from_feedback(feedback: &FeedbackPolishSummary, spec: Ca39FeedbackCueSpec) -> Ca39DriveCue {
    let event = feedback
        .events
        .iter()
        .find(|event| event.kind == spec.event_kind);
    Ca39DriveCue {
        schema: CA39_DRIVE_AUDIO_VFX_SCHEMA,
        schema_version: CA39_DRIVE_AUDIO_VFX_SCHEMA_VERSION,
        kind: spec.cue_kind,
        active: event.is_some() || !spec.required_active,
        target_stable_id: event.and_then(|event| event.stable_entity),
        source_milestone: event
            .map(|event| event.notification.clone())
            .unwrap_or_else(|| spec.source_milestone.to_string()),
        drive_channel: spec.cue_kind.drive_channel(),
        audio_asset_id: Some(
            event
                .and_then(|event| event.audio_asset_id.clone())
                .unwrap_or_else(|| spec.default_audio_asset_id.to_string()),
        ),
        vfx_asset_id: Some(
            event
                .and_then(|event| event.vfx_asset_id.clone())
                .unwrap_or_else(|| spec.default_vfx_asset_id.to_string()),
        ),
        audio_stub_label: spec.audio_stub_label,
        vfx_stub_label: spec.vfx_stub_label,
        intensity: spec.intensity,
        display_only: true,
        no_action_authority: true,
        no_weight_authority: true,
    }
}

fn learning_pulse_cue(evidence: &Ca39RuntimeCueEvidence) -> Ca39DriveCue {
    Ca39DriveCue {
        schema: CA39_DRIVE_AUDIO_VFX_SCHEMA,
        schema_version: CA39_DRIVE_AUDIO_VFX_SCHEMA_VERSION,
        kind: Ca39DriveCueKind::LearningPulse,
        active: evidence.learning_updates > 0,
        target_stable_id: Some(WorldEntityId(1)),
        source_milestone: format!(
            "post-seal H_shadow learning applications={}",
            evidence.learning_updates
        ),
        drive_channel: Ca39DriveCueKind::LearningPulse.drive_channel(),
        audio_asset_id: Some("ca39-audio-learning-pulse".to_string()),
        vfx_asset_id: Some("ca39-vfx-learning-pulse".to_string()),
        audio_stub_label: "soft learning ping",
        vfx_stub_label: "teal H_shadow pulse",
        intensity: FeedbackIntensity::Medium,
        display_only: true,
        no_action_authority: true,
        no_weight_authority: true,
    }
}
