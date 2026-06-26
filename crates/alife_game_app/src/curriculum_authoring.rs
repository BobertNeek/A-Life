//! CA25 curriculum authoring and verifier UI contracts.

use std::{collections::BTreeSet, fs, path::Path};

use crate::prelude::*;
use crate::{
    run_school_mode_smoke, GameAppShellError, CA25_CURRICULUM_AUTHORING_SCHEMA,
    CA25_CURRICULUM_AUTHORING_SCHEMA_VERSION, CA25_MAX_LESSONS, CA25_MAX_VERIFIER_CONDITIONS,
};

pub const CA25_DEFAULT_LESSON_MANIFEST: &str = "../../examples/ca25/lesson_manifest.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonManifest {
    pub schema: String,
    pub schema_version: u16,
    pub curriculum_id: String,
    pub title: String,
    pub lessons: Vec<LessonManifestLesson>,
}

impl LessonManifest {
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, GameAppShellError> {
        let text = fs::read_to_string(path)?;
        Self::from_json_str(&text)
    }

    pub fn from_json_str(text: &str) -> Result<Self, GameAppShellError> {
        let manifest = serde_json::from_str::<Self>(text)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA25_CURRICULUM_AUTHORING_SCHEMA
            || self.schema_version != CA25_CURRICULUM_AUTHORING_SCHEMA_VERSION
            || self.curriculum_id.is_empty()
            || self.curriculum_id.len() > 96
            || self.curriculum_id.contains("Entity(")
            || self.title.is_empty()
            || self.title.len() > 96
            || self.title.contains("Entity(")
            || self.lessons.is_empty()
            || self.lessons.len() > CA25_MAX_LESSONS
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA25 lesson manifest header is invalid",
            });
        }
        let mut lesson_ids = BTreeSet::new();
        for lesson in &self.lessons {
            lesson.validate()?;
            if !lesson_ids.insert(lesson.lesson_id) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "CA25 lesson IDs must be unique",
                });
            }
        }
        Ok(())
    }

    pub fn active_lesson(&self) -> Result<&LessonManifestLesson, GameAppShellError> {
        self.lessons
            .first()
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "CA25 manifest must contain an active lesson",
            })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonManifestLesson {
    pub lesson_id: u64,
    pub title: String,
    pub step_kind: String,
    pub teacher_cues: Vec<String>,
    pub verifier_conditions: Vec<LessonVerifierCondition>,
}

impl LessonManifestLesson {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        LessonId::new(self.lesson_id)?;
        if self.title.is_empty()
            || self.title.len() > 96
            || self.title.contains("Entity(")
            || self.step_kind.is_empty()
            || self.step_kind.len() > 48
            || self.step_kind.contains("Entity(")
            || self.teacher_cues.is_empty()
            || self.teacher_cues.len() > 8
            || self.verifier_conditions.is_empty()
            || self.verifier_conditions.len() > CA25_MAX_VERIFIER_CONDITIONS
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA25 lesson entry is invalid",
            });
        }
        for cue in &self.teacher_cues {
            if cue.is_empty() || cue.len() > 80 || cue.contains("Entity(") {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "CA25 teacher cue text must be player-facing stable text",
                });
            }
        }
        for condition in &self.verifier_conditions {
            condition.to_verifier_check()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LessonVerifierCondition {
    HeardToken { token_id: u32, channel: String },
    RewardAtLeast { threshold: f32 },
    NoHiddenSemanticContext,
    NoDirectTeacherActionSelection,
    SelectedByArbitration,
}

impl LessonVerifierCondition {
    pub fn to_verifier_check(&self) -> Result<VerifierCheck, GameAppShellError> {
        let check = match self {
            Self::HeardToken { token_id, channel } => {
                let channel = parse_teacher_channel(channel)?;
                VerifierCheck::HeardToken {
                    token_id: *token_id,
                    channel,
                }
            }
            Self::RewardAtLeast { threshold } => VerifierCheck::RewardAtLeast(*threshold),
            Self::NoHiddenSemanticContext => VerifierCheck::NoHiddenSemanticContext,
            Self::NoDirectTeacherActionSelection => VerifierCheck::NoDirectTeacherActionSelection,
            Self::SelectedByArbitration => VerifierCheck::SelectedByArbitration,
        };
        validate_manifest_verifier_check(check)?;
        Ok(check)
    }

    pub fn label(&self) -> String {
        match self {
            Self::HeardToken { token_id, channel } => {
                format!("heard_token:{} via {}", token_id, channel)
            }
            Self::RewardAtLeast { threshold } => format!("reward_at_least:{threshold:.2}"),
            Self::NoHiddenSemanticContext => "no_hidden_semantic_context".to_string(),
            Self::NoDirectTeacherActionSelection => {
                "no_direct_teacher_action_selection".to_string()
            }
            Self::SelectedByArbitration => "selected_by_arbitration".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurriculumLessonSaveState {
    pub schema: String,
    pub schema_version: u16,
    pub curriculum_id: String,
    pub active_lesson_id: u64,
    pub completed_lesson_ids: Vec<u64>,
    pub verifier_passed: bool,
    pub editor_dirty: bool,
    pub teacher_private_state_saved: bool,
    pub model_inference_saved: bool,
}

impl CurriculumLessonSaveState {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA25_CURRICULUM_AUTHORING_SCHEMA
            || self.schema_version != CA25_CURRICULUM_AUTHORING_SCHEMA_VERSION
            || self.curriculum_id.is_empty()
            || self.curriculum_id.contains("Entity(")
            || self.active_lesson_id == 0
            || self.completed_lesson_ids.len() > CA25_MAX_LESSONS
            || self.teacher_private_state_saved
            || self.model_inference_saved
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA25 lesson save state must remain portable and teacher-private-free",
            });
        }
        LessonId::new(self.active_lesson_id)?;
        let mut ids = BTreeSet::new();
        for id in &self.completed_lesson_ids {
            LessonId::new(*id)?;
            if !ids.insert(*id) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "CA25 completed lesson IDs must be unique",
                });
            }
        }
        Ok(())
    }

    pub fn to_json_string_pretty(&self) -> Result<String, GameAppShellError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json_str(text: &str) -> Result<Self, GameAppShellError> {
        let state = serde_json::from_str::<Self>(text)?;
        state.validate()?;
        Ok(state)
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.curriculum_id,
            self.active_lesson_id,
            self.completed_lesson_ids
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join("+"),
            self.verifier_passed,
            self.editor_dirty,
            self.teacher_private_state_saved
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurriculumAuthoringSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub manifest_path: String,
    pub curriculum_id: String,
    pub lesson_count: usize,
    pub active_lesson_id: u64,
    pub active_lesson_title: String,
    pub verifier_condition_labels: Vec<String>,
    pub verifier_uses_sealed_patches: bool,
    pub verifier_passed: bool,
    pub completed_lesson_ids: Vec<u64>,
    pub progress_display: String,
    pub editor_panel_text: String,
    pub save_roundtrip_signature: String,
    pub model_inference_required: bool,
    pub fake_model_output_used: bool,
    pub can_issue_actions: bool,
    pub can_rewrite_weights: bool,
}

impl CurriculumAuthoringSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA25_CURRICULUM_AUTHORING_SCHEMA
            || self.schema_version != CA25_CURRICULUM_AUTHORING_SCHEMA_VERSION
            || self.manifest_path.is_empty()
            || self.manifest_path.contains("Entity(")
            || self.curriculum_id.is_empty()
            || self.curriculum_id.contains("Entity(")
            || self.lesson_count == 0
            || self.lesson_count > CA25_MAX_LESSONS
            || self.active_lesson_id == 0
            || self.active_lesson_title.is_empty()
            || self.active_lesson_title.contains("Entity(")
            || self.verifier_condition_labels.is_empty()
            || self.verifier_condition_labels.len() > CA25_MAX_VERIFIER_CONDITIONS
            || !self.verifier_uses_sealed_patches
            || self.progress_display.is_empty()
            || self.progress_display.contains("Entity(")
            || self.editor_panel_text.is_empty()
            || self.editor_panel_text.contains("Entity(")
            || self.save_roundtrip_signature.is_empty()
            || self.model_inference_required
            || self.fake_model_output_used
            || self.can_issue_actions
            || self.can_rewrite_weights
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA25 curriculum authoring summary is invalid",
            });
        }
        LessonId::new(self.active_lesson_id)?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema,
            self.schema_version,
            self.curriculum_id,
            self.lesson_count,
            self.active_lesson_id,
            self.verifier_passed,
            self.completed_lesson_ids.len(),
            self.save_roundtrip_signature
        )
    }
}

pub fn default_ca25_lesson_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CA25_DEFAULT_LESSON_MANIFEST)
}

pub fn run_curriculum_authoring_smoke() -> Result<CurriculumAuthoringSummary, GameAppShellError> {
    run_curriculum_authoring_smoke_with_manifest(default_ca25_lesson_manifest_path())
}

pub fn run_curriculum_authoring_smoke_with_manifest(
    path: impl AsRef<Path>,
) -> Result<CurriculumAuthoringSummary, GameAppShellError> {
    let path = path.as_ref();
    let manifest = LessonManifest::from_json_file(path)?;
    let active = manifest.active_lesson()?;
    let school = run_school_mode_smoke()?;
    let verifier_condition_labels = active
        .verifier_conditions
        .iter()
        .map(LessonVerifierCondition::label)
        .collect::<Vec<_>>();
    let verifier_uses_sealed_patches = school.verifier_panel.sealed_patch_count > 0;
    let verifier_passed = school.verifier_panel.passed
        && active
            .verifier_conditions
            .iter()
            .any(|condition| matches!(condition, LessonVerifierCondition::HeardToken { .. }))
        && active
            .verifier_conditions
            .iter()
            .any(|condition| matches!(condition, LessonVerifierCondition::NoHiddenSemanticContext))
        && active.verifier_conditions.iter().any(|condition| {
            matches!(
                condition,
                LessonVerifierCondition::NoDirectTeacherActionSelection
            )
        });
    let completed_lesson_ids = if verifier_passed {
        vec![active.lesson_id]
    } else {
        Vec::new()
    };
    let save = CurriculumLessonSaveState {
        schema: CA25_CURRICULUM_AUTHORING_SCHEMA.to_string(),
        schema_version: CA25_CURRICULUM_AUTHORING_SCHEMA_VERSION,
        curriculum_id: manifest.curriculum_id.clone(),
        active_lesson_id: active.lesson_id,
        completed_lesson_ids: completed_lesson_ids.clone(),
        verifier_passed,
        editor_dirty: false,
        teacher_private_state_saved: false,
        model_inference_saved: false,
    };
    let json = save.to_json_string_pretty()?;
    let loaded = CurriculumLessonSaveState::from_json_str(&json)?;
    let progress_display = format!(
        "Curriculum: {} | Lesson: {} | Progress: {}/{} | Verifier sealed-patch pass={}",
        manifest.curriculum_id,
        active.title,
        completed_lesson_ids.len(),
        manifest.lessons.len(),
        verifier_passed
    );
    let editor_panel_text = format!(
        concat!(
            "Lesson Editor: validator-only JSON\n",
            "Active stable lesson={} kind={}\n",
            "Teacher cues={} verifier conditions={}\n",
            "Boundary: perception-only; no model inference in CA25"
        ),
        active.lesson_id,
        active.step_kind,
        active.teacher_cues.len(),
        active.verifier_conditions.len()
    );
    let summary = CurriculumAuthoringSummary {
        schema: CA25_CURRICULUM_AUTHORING_SCHEMA,
        schema_version: CA25_CURRICULUM_AUTHORING_SCHEMA_VERSION,
        manifest_path: path.display().to_string(),
        curriculum_id: manifest.curriculum_id.clone(),
        lesson_count: manifest.lessons.len(),
        active_lesson_id: active.lesson_id,
        active_lesson_title: active.title.clone(),
        verifier_condition_labels,
        verifier_uses_sealed_patches,
        verifier_passed,
        completed_lesson_ids,
        progress_display,
        editor_panel_text,
        save_roundtrip_signature: loaded.signature_line(),
        model_inference_required: false,
        fake_model_output_used: false,
        can_issue_actions: false,
        can_rewrite_weights: false,
    };
    summary.validate()?;
    Ok(summary)
}

fn parse_teacher_channel(channel: &str) -> Result<TeacherPerceptionChannel, GameAppShellError> {
    match channel {
        "hearing" => Ok(TeacherPerceptionChannel::Hearing),
        "vision" => Ok(TeacherPerceptionChannel::Vision),
        "writing" => Ok(TeacherPerceptionChannel::Writing),
        "gesture" => Ok(TeacherPerceptionChannel::Gesture),
        _ => Err(GameAppShellError::VisibleWorldMismatch {
            message: "CA25 verifier condition has unknown teacher channel",
        }),
    }
}

fn validate_manifest_verifier_check(check: VerifierCheck) -> Result<(), GameAppShellError> {
    match check {
        VerifierCheck::HeardToken { token_id: 0, .. } => {
            Err(GameAppShellError::Core(ScaffoldContractError::InvalidId))
        }
        VerifierCheck::RewardAtLeast(threshold)
            if !threshold.is_finite() || !(-1.0..=1.0).contains(&threshold) =>
        {
            Err(GameAppShellError::Core(
                ScaffoldContractError::ScalarOutOfRange,
            ))
        }
        _ => Ok(()),
    }
}
