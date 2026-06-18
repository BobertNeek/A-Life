//! Headless-safe feedback event and placeholder polish mapping for G17.

use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackChannel {
    Audio,
    Vfx,
    Animation,
    Notification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FeedbackEventKind {
    FoodReward,
    MissingAffordance,
    HazardPain,
    SleepTransition,
    TeacherCue,
    SaveCompleted,
    LoadCompleted,
    SelectionChanged,
}

impl FeedbackEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FoodReward => "food-reward",
            Self::MissingAffordance => "missing-affordance",
            Self::HazardPain => "hazard-pain",
            Self::SleepTransition => "sleep-transition",
            Self::TeacherCue => "teacher-cue",
            Self::SaveCompleted => "save-completed",
            Self::LoadCompleted => "load-completed",
            Self::SelectionChanged => "selection-changed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FeedbackAssetKind {
    AudioCue,
    VfxCue,
    AnimationCurve,
    NotificationStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackIntensity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackAssetEntry {
    pub asset_id: String,
    pub kind: FeedbackAssetKind,
    pub relative_path: Option<String>,
    pub optional: bool,
    pub procedural_fallback: bool,
    pub max_size_bytes: u64,
}

impl FeedbackAssetEntry {
    pub fn validate_with_root(&self, root: &Path) -> Result<bool, GameAppShellError> {
        if self.asset_id.is_empty()
            || self.max_size_bytes > G17_MAX_POLISH_ASSET_BYTES
            || (!self.optional && self.relative_path.is_none())
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        let Some(relative_path) = &self.relative_path else {
            return Ok(self.optional && self.procedural_fallback);
        };
        let relative = Path::new(relative_path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "feedback asset paths must be relative and portable",
            });
        }
        let full_path = root.join(relative);
        if !full_path.exists() {
            if self.optional && self.procedural_fallback {
                return Ok(true);
            }
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "required feedback asset is missing",
            });
        }
        let len = std::fs::metadata(full_path)?.len();
        if len > self.max_size_bytes {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "feedback asset exceeds declared size cap",
            });
        }
        Ok(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackAssetManifest {
    pub schema: String,
    pub schema_version: u16,
    pub entries: Vec<FeedbackAssetEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedbackAssetValidation {
    pub entry_count: usize,
    pub optional_fallback_count: usize,
    pub required_assets_available: bool,
}

impl FeedbackAssetManifest {
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, GameAppShellError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_json_str(&text)
    }

    pub fn from_json_str(text: &str) -> Result<Self, GameAppShellError> {
        Ok(serde_json::from_str(text)?)
    }

    pub fn validate_with_root(
        &self,
        root: impl AsRef<Path>,
    ) -> Result<FeedbackAssetValidation, GameAppShellError> {
        if self.schema != G17_FEEDBACK_ASSET_MANIFEST_SCHEMA
            || self.schema_version != G17_FEEDBACK_ASSET_MANIFEST_SCHEMA_VERSION
            || self.entries.is_empty()
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        let mut ids = BTreeSet::new();
        let mut kinds = BTreeSet::new();
        let mut optional_fallback_count = 0usize;
        for entry in &self.entries {
            if !ids.insert(entry.asset_id.as_str()) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "feedback asset IDs must be unique",
                });
            }
            kinds.insert(entry.kind);
            if entry.validate_with_root(root.as_ref())? {
                optional_fallback_count += 1;
            }
        }
        if !kinds.contains(&FeedbackAssetKind::AudioCue)
            || !kinds.contains(&FeedbackAssetKind::VfxCue)
            || !kinds.contains(&FeedbackAssetKind::AnimationCurve)
            || !kinds.contains(&FeedbackAssetKind::NotificationStyle)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message:
                    "feedback manifest must cover audio, VFX, animation, and notification cues",
            });
        }
        Ok(FeedbackAssetValidation {
            entry_count: self.entries.len(),
            optional_fallback_count,
            required_assets_available: true,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedbackEvent {
    pub schema: &'static str,
    pub schema_version: u16,
    pub kind: FeedbackEventKind,
    pub source_system: &'static str,
    pub source_tick: Option<Tick>,
    pub stable_entity: Option<WorldEntityId>,
    pub channels: Vec<FeedbackChannel>,
    pub audio_asset_id: Option<String>,
    pub vfx_asset_id: Option<String>,
    pub animation: CreatureAnimationState,
    pub intensity: FeedbackIntensity,
    pub notification: String,
    pub non_authoritative: bool,
}

impl FeedbackEvent {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G17_FEEDBACK_POLISH_SCHEMA
            || self.schema_version != G17_FEEDBACK_POLISH_SCHEMA_VERSION
            || self.source_system.is_empty()
            || self.channels.is_empty()
            || self.notification.is_empty()
            || !self.non_authoritative
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if let Some(entity) = self.stable_entity {
            entity.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:?}:{:?}:{}",
            self.schema_version,
            self.kind.label(),
            self.source_system,
            self.stable_entity.map(|id| id.raw()),
            self.source_tick.map(|tick| tick.raw()),
            self.notification
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedbackPolishSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub events: Vec<FeedbackEvent>,
    pub asset_manifest_entries: usize,
    pub optional_asset_fallbacks: usize,
    pub required_assets_available: bool,
    pub sealed_outcome_event_count: usize,
    pub non_authoritative: bool,
}

impl FeedbackPolishSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G17_FEEDBACK_POLISH_SCHEMA
            || self.schema_version != G17_FEEDBACK_POLISH_SCHEMA_VERSION
            || self.events.is_empty()
            || self.events.len() > G17_MAX_FEEDBACK_EVENTS
            || self.asset_manifest_entries == 0
            || !self.required_assets_available
            || self.sealed_outcome_event_count == 0
            || !self.non_authoritative
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        for event in &self.events {
            event.validate()?;
        }
        Ok(())
    }

    pub fn event_labels(&self) -> Vec<&'static str> {
        self.events.iter().map(|event| event.kind.label()).collect()
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.schema_version,
            self.events.len(),
            self.asset_manifest_entries,
            self.optional_asset_fallbacks,
            self.events
                .iter()
                .map(FeedbackEvent::signature_line)
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

pub fn g17_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn g17_feedback_manifest_path() -> PathBuf {
    g17_workspace_root().join("content/fixtures/g17/feedback_polish_manifest.json")
}

pub fn run_feedback_polish_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<FeedbackPolishSummary, GameAppShellError> {
    let manifest = FeedbackAssetManifest::from_json_file(g17_feedback_manifest_path())?;
    let asset_validation = manifest.validate_with_root(g17_workspace_root())?;
    let survival = run_playable_survival_loop_smoke()?;
    let school = run_school_mode_smoke()?;
    let save_load = run_save_load_ux_smoke(launch)?;
    let inspector = run_creature_inspector_smoke(launch)?;

    let mut events = survival_feedback_events(&survival);
    let sealed_outcome_event_count = events.len();
    events.push(school_feedback_event(&school)?);
    events.extend(save_load_feedback_events(&save_load));
    events.push(selection_feedback_event(&inspector));

    let summary = FeedbackPolishSummary {
        schema: G17_FEEDBACK_POLISH_SCHEMA,
        schema_version: G17_FEEDBACK_POLISH_SCHEMA_VERSION,
        events,
        asset_manifest_entries: asset_validation.entry_count,
        optional_asset_fallbacks: asset_validation.optional_fallback_count,
        required_assets_available: asset_validation.required_assets_available,
        sealed_outcome_event_count,
        non_authoritative: true,
    };
    summary.validate()?;
    Ok(summary)
}

fn survival_feedback_events(summary: &PlayableSurvivalLoopSummary) -> Vec<FeedbackEvent> {
    summary
        .events
        .iter()
        .map(|event| match event.kind {
            PlayableSurvivalEventKind::FoodConsumed => FeedbackEvent {
                schema: G17_FEEDBACK_POLISH_SCHEMA,
                schema_version: G17_FEEDBACK_POLISH_SCHEMA_VERSION,
                kind: FeedbackEventKind::FoodReward,
                source_system: "G06-survival-sealed-outcome",
                source_tick: Some(event.tick),
                stable_entity: event.target_entity,
                channels: vec![
                    FeedbackChannel::Audio,
                    FeedbackChannel::Vfx,
                    FeedbackChannel::Animation,
                    FeedbackChannel::Notification,
                ],
                audio_asset_id: Some("g17-audio-food-chime".to_string()),
                vfx_asset_id: Some("g17-vfx-food-spark".to_string()),
                animation: CreatureAnimationState::Interacting,
                intensity: FeedbackIntensity::High,
                notification: "food reward: hunger eased after sealed outcome".to_string(),
                non_authoritative: true,
            },
            PlayableSurvivalEventKind::MissingAffordance => FeedbackEvent {
                schema: G17_FEEDBACK_POLISH_SCHEMA,
                schema_version: G17_FEEDBACK_POLISH_SCHEMA_VERSION,
                kind: FeedbackEventKind::MissingAffordance,
                source_system: "G06-survival-sealed-outcome",
                source_tick: Some(event.tick),
                stable_entity: event.target_entity,
                channels: vec![FeedbackChannel::Audio, FeedbackChannel::Notification],
                audio_asset_id: Some("g17-audio-deny-tick".to_string()),
                vfx_asset_id: None,
                animation: CreatureAnimationState::Inspecting,
                intensity: FeedbackIntensity::Low,
                notification: "missing affordance: action failed without retry loop".to_string(),
                non_authoritative: true,
            },
            PlayableSurvivalEventKind::HazardPain => FeedbackEvent {
                schema: G17_FEEDBACK_POLISH_SCHEMA,
                schema_version: G17_FEEDBACK_POLISH_SCHEMA_VERSION,
                kind: FeedbackEventKind::HazardPain,
                source_system: "G06-survival-sealed-outcome",
                source_tick: Some(event.tick),
                stable_entity: event.target_entity,
                channels: vec![
                    FeedbackChannel::Audio,
                    FeedbackChannel::Vfx,
                    FeedbackChannel::Animation,
                    FeedbackChannel::Notification,
                ],
                audio_asset_id: Some("g17-audio-hazard-pulse".to_string()),
                vfx_asset_id: Some("g17-vfx-hazard-flash".to_string()),
                animation: CreatureAnimationState::Hurt,
                intensity: FeedbackIntensity::High,
                notification: "hazard pain: fear/pain feedback made visible".to_string(),
                non_authoritative: true,
            },
            PlayableSurvivalEventKind::RestSleep => FeedbackEvent {
                schema: G17_FEEDBACK_POLISH_SCHEMA,
                schema_version: G17_FEEDBACK_POLISH_SCHEMA_VERSION,
                kind: FeedbackEventKind::SleepTransition,
                source_system: "G06-survival-sealed-outcome",
                source_tick: Some(event.tick),
                stable_entity: None,
                channels: vec![
                    FeedbackChannel::Audio,
                    FeedbackChannel::Vfx,
                    FeedbackChannel::Animation,
                    FeedbackChannel::Notification,
                ],
                audio_asset_id: Some("g17-audio-sleep-soft".to_string()),
                vfx_asset_id: Some("g17-vfx-sleep-bloom".to_string()),
                animation: CreatureAnimationState::Sleeping,
                intensity: FeedbackIntensity::Medium,
                notification: "sleep transition: rest action entered visible recovery state"
                    .to_string(),
                non_authoritative: true,
            },
        })
        .collect()
}

fn school_feedback_event(summary: &SchoolModeSummary) -> Result<FeedbackEvent, GameAppShellError> {
    let cue = summary
        .cues
        .first()
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    Ok(FeedbackEvent {
        schema: G17_FEEDBACK_POLISH_SCHEMA,
        schema_version: G17_FEEDBACK_POLISH_SCHEMA_VERSION,
        kind: FeedbackEventKind::TeacherCue,
        source_system: "G10-school-perception-only-cue",
        source_tick: None,
        stable_entity: cue.cue_entity.or(cue.object_entity),
        channels: vec![
            FeedbackChannel::Audio,
            FeedbackChannel::Vfx,
            FeedbackChannel::Notification,
        ],
        audio_asset_id: Some("g17-audio-teacher-cue".to_string()),
        vfx_asset_id: Some("g17-vfx-selection-ring".to_string()),
        animation: CreatureAnimationState::Signaling,
        intensity: FeedbackIntensity::Medium,
        notification: format!("teacher cue: {} remained perception-only", cue.label),
        non_authoritative: true,
    })
}

fn save_load_feedback_events(summary: &SaveLoadUxSmokeSummary) -> Vec<FeedbackEvent> {
    vec![
        FeedbackEvent {
            schema: G17_FEEDBACK_POLISH_SCHEMA,
            schema_version: G17_FEEDBACK_POLISH_SCHEMA_VERSION,
            kind: FeedbackEventKind::SaveCompleted,
            source_system: "G15-save-load-ux",
            source_tick: None,
            stable_entity: summary.stable_world_ids.first().copied(),
            channels: vec![FeedbackChannel::Audio, FeedbackChannel::Notification],
            audio_asset_id: Some("g17-audio-save-chime".to_string()),
            vfx_asset_id: None,
            animation: CreatureAnimationState::Idle,
            intensity: FeedbackIntensity::Low,
            notification: format!("manual save available in slot {}", summary.manual_save_slot),
            non_authoritative: true,
        },
        FeedbackEvent {
            schema: G17_FEEDBACK_POLISH_SCHEMA,
            schema_version: G17_FEEDBACK_POLISH_SCHEMA_VERSION,
            kind: FeedbackEventKind::LoadCompleted,
            source_system: "G15-save-load-ux",
            source_tick: None,
            stable_entity: summary.stable_world_ids.first().copied(),
            channels: vec![FeedbackChannel::Vfx, FeedbackChannel::Notification],
            audio_asset_id: None,
            vfx_asset_id: Some("g17-vfx-selection-ring".to_string()),
            animation: CreatureAnimationState::Idle,
            intensity: FeedbackIntensity::Low,
            notification: format!("loaded save {} with stable IDs", summary.loaded_save_id),
            non_authoritative: true,
        },
    ]
}

fn selection_feedback_event(inspector: &CreatureInspectorSnapshot) -> FeedbackEvent {
    FeedbackEvent {
        schema: G17_FEEDBACK_POLISH_SCHEMA,
        schema_version: G17_FEEDBACK_POLISH_SCHEMA_VERSION,
        kind: FeedbackEventKind::SelectionChanged,
        source_system: "G05-read-only-inspector-selection",
        source_tick: None,
        stable_entity: Some(inspector.selection.stable_id),
        channels: vec![
            FeedbackChannel::Vfx,
            FeedbackChannel::Animation,
            FeedbackChannel::Notification,
        ],
        audio_asset_id: None,
        vfx_asset_id: Some("g17-vfx-selection-ring".to_string()),
        animation: inspector.visual.animation,
        intensity: FeedbackIntensity::Low,
        notification: format!(
            "selected stable entity {} without mutating cognition",
            inspector.selection.stable_id.raw()
        ),
        non_authoritative: true,
    }
}
