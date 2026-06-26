//! CA23 graphical school-mode presentation.

use crate::prelude::*;
use crate::{
    run_school_mode_smoke, GameAppShellError, SchoolCuePresentation, SchoolModeSummary,
    CA23_GRAPHICAL_SCHOOL_SCHEMA, CA23_GRAPHICAL_SCHOOL_SCHEMA_VERSION,
    CA24_TEACHER_WORLD_CUES_SCHEMA, CA24_TEACHER_WORLD_CUES_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ca23TeacherCueMarker {
    pub stable_id: WorldEntityId,
    pub label: String,
    pub channel: TeacherPerceptionChannel,
    pub perception_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ca24TeacherWorldCueKind {
    SpeechToken,
    GestureMarker,
    ObjectHighlight,
    FeedbackCue,
}

impl Ca24TeacherWorldCueKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SpeechToken => "speech token",
            Self::GestureMarker => "gesture marker",
            Self::ObjectHighlight => "object highlight",
            Self::FeedbackCue => "feedback cue",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca24TeacherWorldCue {
    pub stable_id: WorldEntityId,
    pub kind: Ca24TeacherWorldCueKind,
    pub label: String,
    pub channel: TeacherPerceptionChannel,
    pub token_id: Option<u32>,
    pub gesture_id: Option<u32>,
    pub object_entity: Option<WorldEntityId>,
    pub salience: f32,
    pub visible: bool,
    pub audible: bool,
    pub perception_only: bool,
    pub direct_motor_bypass: bool,
    pub hidden_vector_injection: bool,
    pub lesson_condition_source: &'static str,
}

impl Ca24TeacherWorldCue {
    fn from_school_cue(cue: &SchoolCuePresentation) -> Result<Self, GameAppShellError> {
        let stable_id = cue
            .cue_entity
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "CA24 teacher world cues must have stable world IDs",
            })?;
        let kind = match cue.input_kind {
            TeacherInputKind::SpokenToken => Ca24TeacherWorldCueKind::SpeechToken,
            TeacherInputKind::Gesture => Ca24TeacherWorldCueKind::GestureMarker,
            TeacherInputKind::ObjectHighlight => Ca24TeacherWorldCueKind::ObjectHighlight,
            TeacherInputKind::SocialFeedback
            | TeacherInputKind::VisibleReward
            | TeacherInputKind::VisiblePunishment => Ca24TeacherWorldCueKind::FeedbackCue,
        };
        let event = Self {
            stable_id,
            kind,
            label: format!("{} {}", kind.label(), cue.label),
            channel: cue.channel,
            token_id: cue.token_id,
            gesture_id: cue.gesture_id,
            object_entity: cue.object_entity,
            salience: cue.salience,
            visible: !matches!(kind, Ca24TeacherWorldCueKind::SpeechToken)
                || cue.cue_entity.is_some(),
            audible: cue.channel == TeacherPerceptionChannel::Hearing,
            perception_only: cue.perception_only,
            direct_motor_bypass: cue.direct_motor_bypass,
            hidden_vector_injection: false,
            lesson_condition_source: "sensory/environmental",
        };
        event.validate()?;
        Ok(event)
    }

    fn validate(&self) -> Result<(), GameAppShellError> {
        self.stable_id.validate()?;
        if let Some(object) = self.object_entity {
            object.validate()?;
        }
        if self.label.is_empty()
            || self.label.len() > 96
            || self.label.contains("Entity(")
            || !self.perception_only
            || self.direct_motor_bypass
            || self.hidden_vector_injection
            || (!self.visible && !self.audible)
            || self.lesson_condition_source != "sensory/environmental"
            || !self.salience.is_finite()
            || !(0.0..=1.0).contains(&self.salience)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA24 teacher world cue must remain visible/audible and perception-only",
            });
        }
        if matches!(self.kind, Ca24TeacherWorldCueKind::SpeechToken) && self.token_id.is_none() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA24 speech token cue must carry a token ID",
            });
        }
        if matches!(self.kind, Ca24TeacherWorldCueKind::GestureMarker) && self.gesture_id.is_none()
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA24 gesture cue must carry a gesture ID",
            });
        }
        Ok(())
    }

    pub fn compact_line(&self) -> String {
        format!(
            "[{}] stable:{} {:?} token={:?} gesture={:?} source={}",
            self.kind.label(),
            self.stable_id.raw(),
            self.channel,
            self.token_id,
            self.gesture_id,
            self.lesson_condition_source
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca24TeacherWorldCueSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub teacher_avatar_stable_id: WorldEntityId,
    pub learner_stable_id: WorldEntityId,
    pub active_lesson_id: u64,
    pub cue_objects: Vec<Ca24TeacherWorldCue>,
    pub visible_world_events: bool,
    pub audible_token_events: bool,
    pub gesture_events: bool,
    pub lesson_conditions_are_sensory_environmental: bool,
    pub verifier_uses_sealed_patches: bool,
    pub direct_motor_bypass_rejected: bool,
    pub hidden_vector_injection_blocked: bool,
    pub no_action_authority: bool,
}

impl Ca24TeacherWorldCueSummary {
    pub fn from_school_summary(summary: &SchoolModeSummary) -> Result<Self, GameAppShellError> {
        summary.validate()?;
        let cue_objects = summary
            .cues
            .iter()
            .map(Ca24TeacherWorldCue::from_school_cue)
            .collect::<Result<Vec<_>, _>>()?;
        let panel = Self {
            schema: CA24_TEACHER_WORLD_CUES_SCHEMA,
            schema_version: CA24_TEACHER_WORLD_CUES_SCHEMA_VERSION,
            teacher_avatar_stable_id: summary.teacher_avatar_stable_id,
            learner_stable_id: summary.learner_stable_id,
            active_lesson_id: summary.lesson_panel.active_lesson_id.raw(),
            visible_world_events: cue_objects.iter().any(|cue| cue.visible),
            audible_token_events: cue_objects
                .iter()
                .any(|cue| matches!(cue.kind, Ca24TeacherWorldCueKind::SpeechToken) && cue.audible),
            gesture_events: cue_objects
                .iter()
                .any(|cue| matches!(cue.kind, Ca24TeacherWorldCueKind::GestureMarker)),
            lesson_conditions_are_sensory_environmental: cue_objects
                .iter()
                .all(|cue| cue.lesson_condition_source == "sensory/environmental"),
            verifier_uses_sealed_patches: summary.verifier_panel.sealed_patch_count > 0,
            direct_motor_bypass_rejected: summary.teacher_metadata_bypass_blocked
                && summary.teacher_selected_action_id.is_none(),
            hidden_vector_injection_blocked: summary
                .cues
                .iter()
                .all(|cue| cue.perception_only && !cue.direct_motor_bypass),
            no_action_authority: summary.teacher_selected_action_id.is_none(),
            cue_objects,
        };
        panel.validate()?;
        Ok(panel)
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA24_TEACHER_WORLD_CUES_SCHEMA
            || self.schema_version != CA24_TEACHER_WORLD_CUES_SCHEMA_VERSION
            || self.active_lesson_id == 0
            || self.cue_objects.len() < 4
            || self.cue_objects.len() > 8
            || !self.visible_world_events
            || !self.audible_token_events
            || !self.gesture_events
            || !self.lesson_conditions_are_sensory_environmental
            || !self.verifier_uses_sealed_patches
            || !self.direct_motor_bypass_rejected
            || !self.hidden_vector_injection_blocked
            || !self.no_action_authority
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA24 teacher world cues must stay perception-only and verifier-gated",
            });
        }
        self.teacher_avatar_stable_id.validate()?;
        self.learner_stable_id.validate()?;
        for cue in &self.cue_objects {
            cue.validate()?;
        }
        if !self
            .cue_objects
            .iter()
            .any(|cue| matches!(cue.kind, Ca24TeacherWorldCueKind::ObjectHighlight))
            || !self
                .cue_objects
                .iter()
                .any(|cue| matches!(cue.kind, Ca24TeacherWorldCueKind::FeedbackCue))
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA24 teacher cues must include object highlight and feedback events",
            });
        }
        Ok(())
    }

    pub fn compact_overlay_text(&self) -> String {
        format!(
            concat!(
                "Teacher Cues: visible world events\n",
                "Speech token: {}  Gesture: {}  Object/feedback: yes\n",
                "Lesson source: sensory/environmental only\n",
                "Boundary: no direct motor bypass; no hidden vectors"
            ),
            self.audible_token_events, self.gesture_events
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:teacher={}:learner={}:lesson={}:cues={}:speech={}:gesture={}:boundary={}",
            self.schema,
            self.schema_version,
            self.teacher_avatar_stable_id.raw(),
            self.learner_stable_id.raw(),
            self.active_lesson_id,
            self.cue_objects.len(),
            self.audible_token_events,
            self.gesture_events,
            self.no_action_authority
        )
    }
}

impl Ca23TeacherCueMarker {
    fn validate(&self) -> Result<(), GameAppShellError> {
        self.stable_id.validate()?;
        if self.label.is_empty()
            || self.label.len() > 80
            || self.label.contains("Entity(")
            || !self.perception_only
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA23 teacher cue marker must be perception-only player text",
            });
        }
        Ok(())
    }

    pub fn compact_line(&self) -> String {
        format!(
            "[T] stable:{} {} via {:?}",
            self.stable_id.raw(),
            self.label,
            self.channel
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ca23SchoolPanelMode {
    Expanded,
    Collapsed,
}

impl Ca23SchoolPanelMode {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Expanded => Self::Collapsed,
            Self::Collapsed => Self::Expanded,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Expanded => "expanded",
            Self::Collapsed => "collapsed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ca23GraphicalSchoolSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub school_enabled: bool,
    pub toggle_key: &'static str,
    pub panel_mode: Ca23SchoolPanelMode,
    pub teacher_avatar_stable_id: WorldEntityId,
    pub learner_stable_id: WorldEntityId,
    pub active_curriculum_id: String,
    pub active_lesson_id: u64,
    pub completed_steps: usize,
    pub total_steps: usize,
    pub cue_markers: Vec<Ca23TeacherCueMarker>,
    pub verifier_uses_sealed_patches: bool,
    pub verifier_passed: bool,
    pub sealed_patch_count: usize,
    pub perception_only_boundary_visible: bool,
    pub direct_motor_bypass_blocked: bool,
    pub hidden_vector_injection_blocked: bool,
    pub display_only: bool,
}

impl Ca23GraphicalSchoolSummary {
    pub fn toggle_school_enabled(&mut self) {
        self.school_enabled = !self.school_enabled;
    }

    pub fn from_school_summary(summary: &SchoolModeSummary) -> Result<Self, GameAppShellError> {
        summary.validate()?;
        let cue_markers = summary
            .cues
            .iter()
            .filter_map(|cue| {
                cue.cue_entity.map(|stable_id| Ca23TeacherCueMarker {
                    stable_id,
                    label: cue.label.clone(),
                    channel: cue.channel,
                    perception_only: cue.perception_only && !cue.direct_motor_bypass,
                })
            })
            .collect::<Vec<_>>();
        let panel = Self {
            schema: CA23_GRAPHICAL_SCHOOL_SCHEMA,
            schema_version: CA23_GRAPHICAL_SCHOOL_SCHEMA_VERSION,
            school_enabled: summary.p34_school.enabled,
            toggle_key: "T",
            panel_mode: Ca23SchoolPanelMode::Expanded,
            teacher_avatar_stable_id: summary.teacher_avatar_stable_id,
            learner_stable_id: summary.learner_stable_id,
            active_curriculum_id: summary.lesson_panel.curriculum_id.clone(),
            active_lesson_id: summary.lesson_panel.active_lesson_id.raw(),
            completed_steps: summary.lesson_panel.completed_steps,
            total_steps: summary.lesson_panel.total_steps,
            cue_markers,
            verifier_uses_sealed_patches: summary.verifier_panel.sealed_patch_count > 0,
            verifier_passed: summary.verifier_panel.passed,
            sealed_patch_count: summary.verifier_panel.sealed_patch_count,
            perception_only_boundary_visible: summary
                .cues
                .iter()
                .all(|cue| cue.perception_only && !cue.direct_motor_bypass),
            direct_motor_bypass_blocked: summary.teacher_metadata_bypass_blocked
                && summary.teacher_selected_action_id.is_none(),
            hidden_vector_injection_blocked: summary
                .cues
                .iter()
                .all(|cue| cue.perception_only && !cue.direct_motor_bypass),
            display_only: true,
        };
        panel.validate()?;
        Ok(panel)
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA23_GRAPHICAL_SCHOOL_SCHEMA
            || self.schema_version != CA23_GRAPHICAL_SCHOOL_SCHEMA_VERSION
            || !self.school_enabled
            || self.toggle_key != "T"
            || self.active_curriculum_id.is_empty()
            || self.active_curriculum_id.contains("Entity(")
            || self.active_lesson_id == 0
            || self.total_steps == 0
            || self.completed_steps > self.total_steps
            || self.cue_markers.is_empty()
            || !self.verifier_uses_sealed_patches
            || self.sealed_patch_count == 0
            || !self.perception_only_boundary_visible
            || !self.direct_motor_bypass_blocked
            || !self.hidden_vector_injection_blocked
            || !self.display_only
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA23 graphical school panel invariants must remain true",
            });
        }
        self.teacher_avatar_stable_id.validate()?;
        self.learner_stable_id.validate()?;
        for marker in &self.cue_markers {
            marker.validate()?;
        }
        Ok(())
    }

    pub fn compact_overlay_text(&self) -> String {
        if !self.school_enabled {
            return format!(
                concat!(
                    "School Mode: off  [{} toggle]\n",
                    "Teacher cues hidden; verifier evidence remains sealed-patch based.\n",
                    "Boundary: perception-only; no motor bypass; no hidden vectors"
                ),
                self.toggle_key
            );
        }
        if self.panel_mode == Ca23SchoolPanelMode::Collapsed {
            return format!(
                "School: on | lesson {} | verifier sealed={} | press {}",
                self.active_lesson_id, self.sealed_patch_count, self.toggle_key
            );
        }
        let cue_text = self
            .cue_markers
            .iter()
            .take(3)
            .map(Ca23TeacherCueMarker::compact_line)
            .collect::<Vec<_>>()
            .join(" | ");
        format!(
            concat!(
                "School Mode: on  [{} toggle]\n",
                "Teacher: stable:{}  learner: stable:{}\n",
                "Lesson: {} id={} {}/{}\n",
                "Cues: {}\n",
                "Verifier: sealed patches={} pass={}\n",
                "Boundary: perception-only; no motor bypass; no hidden vectors"
            ),
            self.toggle_key,
            self.teacher_avatar_stable_id.raw(),
            self.learner_stable_id.raw(),
            self.active_curriculum_id,
            self.active_lesson_id,
            self.completed_steps,
            self.total_steps,
            cue_text,
            self.sealed_patch_count,
            self.verifier_passed
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:school={}:mode={}:teacher={}:learner={}:lesson={}:sealed={}:cues={}:boundary={}",
            self.schema,
            self.schema_version,
            self.school_enabled,
            self.panel_mode.label(),
            self.teacher_avatar_stable_id.raw(),
            self.learner_stable_id.raw(),
            self.active_lesson_id,
            self.sealed_patch_count,
            self.cue_markers.len(),
            self.perception_only_boundary_visible
        )
    }
}

pub fn run_graphical_school_mode_smoke() -> Result<Ca23GraphicalSchoolSummary, GameAppShellError> {
    let school = run_school_mode_smoke()?;
    Ca23GraphicalSchoolSummary::from_school_summary(&school)
}

pub fn run_teacher_world_cues_smoke() -> Result<Ca24TeacherWorldCueSummary, GameAppShellError> {
    let school = run_school_mode_smoke()?;
    Ca24TeacherWorldCueSummary::from_school_summary(&school)
}
