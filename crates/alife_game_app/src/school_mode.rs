//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolModeConfig {
    pub seed: u64,
    pub curriculum_id: String,
    pub learner_id: OrganismId,
    pub teacher_id: OrganismId,
    pub lesson_limit: usize,
}

impl SchoolModeConfig {
    pub fn grounded_smoke() -> Self {
        Self {
            seed: 10_010,
            curriculum_id: "g10-grounded-object-food".to_string(),
            learner_id: OrganismId(10_001),
            teacher_id: OrganismId(10_900),
            lesson_limit: 1,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.learner_id.validate()?;
        self.teacher_id.validate()?;
        if self.seed == 0
            || self.curriculum_id.is_empty()
            || self.curriculum_id.len() > 96
            || self.lesson_limit == 0
            || self.lesson_limit > 6
            || self.learner_id == self.teacher_id
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolCuePresentation {
    pub lesson_id: LessonId,
    pub input_kind: TeacherInputKind,
    pub channel: TeacherPerceptionChannel,
    pub token_id: Option<u32>,
    pub gesture_id: Option<u32>,
    pub object_entity: Option<WorldEntityId>,
    pub cue_entity: Option<WorldEntityId>,
    pub salience: f32,
    pub perception_only: bool,
    pub direct_motor_bypass: bool,
    pub label: String,
}

impl SchoolCuePresentation {
    pub fn from_event(
        event: TeacherPerceptualEvent,
        cue_entity: Option<WorldEntityId>,
        label: impl Into<String>,
    ) -> Result<Self, ScaffoldContractError> {
        if let Some(entity) = cue_entity {
            entity.validate()?;
        }
        if let Some(object) = event.object_entity {
            object.validate()?;
        }
        let cue = Self {
            lesson_id: event.lesson_id,
            input_kind: event.input_kind,
            channel: event.channel,
            token_id: event.token_id,
            gesture_id: event.gesture_id,
            object_entity: event.object_entity,
            cue_entity,
            salience: event.salience.raw(),
            perception_only: event.input_kind.is_perceptual()
                && !event.hidden_vector_injection_allowed(),
            direct_motor_bypass: event.direct_motor_bypass(),
            label: label.into(),
        };
        cue.validate()?;
        Ok(cue)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.lesson_id.raw() == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        if let Some(token_id) = self.token_id {
            if token_id == 0 {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        if let Some(gesture_id) = self.gesture_id {
            if gesture_id == 0 {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        if let Some(entity) = self.object_entity {
            entity.validate()?;
        }
        if let Some(entity) = self.cue_entity {
            entity.validate()?;
        }
        NormalizedScalar::new(self.salience)?;
        if self.label.is_empty() || !self.perception_only || self.direct_motor_bypass {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:.2}:{}:{}",
            self.lesson_id.raw(),
            self.input_kind,
            self.channel,
            self.token_id,
            self.gesture_id,
            self.object_entity.map(|id| id.raw()),
            self.cue_entity.map(|id| id.raw()),
            self.salience,
            self.perception_only,
            self.label
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolLessonPanel {
    pub curriculum_id: String,
    pub active_lesson_id: LessonId,
    pub step_kind: CurriculumStepKind,
    pub total_steps: usize,
    pub completed_steps: usize,
    pub cue_count: usize,
    pub response_channels: Vec<TeacherLessonResponseChannel>,
    pub status_line: String,
}

impl SchoolLessonPanel {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.active_lesson_id.raw() == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        if self.curriculum_id.is_empty()
            || self.total_steps == 0
            || self.completed_steps > self.total_steps
            || self.cue_count == 0
            || self.response_channels.is_empty()
            || self.status_line.is_empty()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{:?}:{}:{}:{}:{}",
            self.curriculum_id,
            self.active_lesson_id.raw(),
            self.step_kind,
            self.completed_steps,
            self.total_steps,
            self.cue_count,
            self.status_line
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolVerifierPanel {
    pub passed: bool,
    pub observed_checks: Vec<String>,
    pub failed_checks: Vec<String>,
    pub sealed_patch_count: usize,
    pub verifier_message: String,
}

impl SchoolVerifierPanel {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.sealed_patch_count == 0
            || self.observed_checks.is_empty()
            || self.verifier_message.is_empty()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.passed,
            self.sealed_patch_count,
            self.observed_checks.join("+"),
            self.failed_checks.join("+")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchoolModeSaveState {
    pub schema: String,
    pub schema_version: u16,
    pub seed: u64,
    pub curriculum_id: String,
    pub active_lesson_id: u64,
    pub completed_steps: usize,
    pub teacher_avatar_stable_id: WorldEntityId,
    pub cue_entity_ids: Vec<WorldEntityId>,
    pub verifier_passed: bool,
    pub p34_school: SchoolSaveState,
}

impl SchoolModeSaveState {
    pub fn from_summary(summary: &SchoolModeSummary) -> Result<Self, ScaffoldContractError> {
        summary.validate()?;
        Ok(Self {
            schema: G10_SCHOOL_MODE_SCHEMA.to_string(),
            schema_version: G10_SCHOOL_MODE_SCHEMA_VERSION,
            seed: summary.seed,
            curriculum_id: summary.lesson_panel.curriculum_id.clone(),
            active_lesson_id: summary.lesson_panel.active_lesson_id.raw(),
            completed_steps: summary.lesson_panel.completed_steps,
            teacher_avatar_stable_id: summary.teacher_avatar_stable_id,
            cue_entity_ids: summary
                .cues
                .iter()
                .filter_map(|cue| cue.cue_entity)
                .collect(),
            verifier_passed: summary.verifier_panel.passed,
            p34_school: summary.p34_school.clone(),
        })
    }

    pub fn to_json_string_pretty(&self) -> Result<String, GameAppShellError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json_str(json: &str) -> Result<Self, GameAppShellError> {
        let state = serde_json::from_str::<Self>(json)?;
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G10_SCHOOL_MODE_SCHEMA
            || self.schema_version != G10_SCHOOL_MODE_SCHEMA_VERSION
            || self.seed == 0
            || self.curriculum_id.is_empty()
            || self.active_lesson_id == 0
            || self.completed_steps > 6
            || self.cue_entity_ids.is_empty()
            || !self.p34_school.enabled
            || self.p34_school.active_curriculum_id.as_ref() != Some(&self.curriculum_id)
            || self.p34_school.teacher_private_state_saved
            || self.p34_school.schema_version != TEACHER_SCHOOL_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.teacher_avatar_stable_id.validate()?;
        for id in &self.cue_entity_ids {
            id.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.curriculum_id,
            self.active_lesson_id,
            self.completed_steps,
            self.teacher_avatar_stable_id.raw(),
            self.cue_entity_ids
                .iter()
                .map(|id| id.raw().to_string())
                .collect::<Vec<_>>()
                .join("+")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolModeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub teacher_avatar_stable_id: WorldEntityId,
    pub learner_stable_id: WorldEntityId,
    pub lesson_panel: SchoolLessonPanel,
    pub cues: Vec<SchoolCuePresentation>,
    pub verifier_panel: SchoolVerifierPanel,
    pub sensory_heard_tokens: Vec<u32>,
    pub sensory_teacher_channels: Vec<TeacherPerceptionChannel>,
    pub teacher_metadata_bypass_blocked: bool,
    pub teacher_selected_action_id: Option<ActionId>,
    pub world_signature: Vec<String>,
    pub p34_school: SchoolSaveState,
    pub save_roundtrip_signature: String,
}

impl SchoolModeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G10_SCHOOL_MODE_SCHEMA
            || self.schema_version != G10_SCHOOL_MODE_SCHEMA_VERSION
            || self.seed == 0
            || self.cues.is_empty()
            || self.world_signature.is_empty()
            || self.sensory_heard_tokens.is_empty()
            || self.sensory_teacher_channels.is_empty()
            || !self.teacher_metadata_bypass_blocked
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.teacher_avatar_stable_id.validate()?;
        self.learner_stable_id.validate()?;
        self.lesson_panel.validate()?;
        self.verifier_panel.validate()?;
        for cue in &self.cues {
            cue.validate()?;
        }
        let contract = TeacherChannelContract::grounded_default();
        if self.cues.iter().any(|cue| {
            !contract.channels.contains(&cue.channel)
                || !contract.input_kinds.contains(&cue.input_kind)
                || !cue.perception_only
                || cue.direct_motor_bypass
        }) {
            return Err(ScaffoldContractError::InvalidId);
        }
        let save = SchoolModeSaveState::from_summary_without_validate(self)?;
        save.validate()?;
        if save.signature_line() != self.save_roundtrip_signature {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.teacher_avatar_stable_id.raw(),
            self.learner_stable_id.raw(),
            self.lesson_panel.signature_line(),
            self.cues
                .iter()
                .map(SchoolCuePresentation::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.verifier_panel.signature_line(),
            self.save_roundtrip_signature
        )
    }
}

impl SchoolModeSaveState {
    fn from_summary_without_validate(
        summary: &SchoolModeSummary,
    ) -> Result<Self, ScaffoldContractError> {
        Ok(Self {
            schema: G10_SCHOOL_MODE_SCHEMA.to_string(),
            schema_version: G10_SCHOOL_MODE_SCHEMA_VERSION,
            seed: summary.seed,
            curriculum_id: summary.lesson_panel.curriculum_id.clone(),
            active_lesson_id: summary.lesson_panel.active_lesson_id.raw(),
            completed_steps: summary.lesson_panel.completed_steps,
            teacher_avatar_stable_id: summary.teacher_avatar_stable_id,
            cue_entity_ids: summary
                .cues
                .iter()
                .filter_map(|cue| cue.cue_entity)
                .collect(),
            verifier_passed: summary.verifier_panel.passed,
            p34_school: summary.p34_school.clone(),
        })
    }
}

pub fn run_school_mode_smoke() -> Result<SchoolModeSummary, GameAppShellError> {
    let config = SchoolModeConfig::grounded_smoke();
    run_school_mode_smoke_with_config(config)
}

pub fn run_school_mode_smoke_with_config(
    config: SchoolModeConfig,
) -> Result<SchoolModeSummary, GameAppShellError> {
    config.validate()?;
    let world = HeadlessScenarioBuilder::new(config.seed)
        .agent("school-learner", config.learner_id, Vec3f::ZERO)
        .social_agent(
            "teacher-avatar",
            config.teacher_id,
            Vec3f::new(-1.25, 0.0, 0.0),
            0.75,
        )
        .food("teaching-berry", Vec3f::new(1.0, 0.0, 0.0), 0.75)
        .teacher_token(
            "teacher-word-food",
            Vec3f::new(0.45, 0.0, 0.0),
            77,
            TeacherPerceptionChannel::Hearing,
        )
        .build()?;
    let learner_stable_id =
        world
            .entity_id("school-learner")
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "G10 learner stable ID must exist",
            })?;
    let teacher_avatar_stable_id =
        world
            .entity_id("teacher-avatar")
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "G10 teacher avatar stable ID must exist",
            })?;
    let object_id =
        world
            .entity_id("teaching-berry")
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "G10 highlighted object stable ID must exist",
            })?;
    let cue_token_id =
        world
            .entity_id("teacher-word-food")
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "G10 teacher token stable ID must exist",
            })?;

    let lesson_id = LessonId::new(10_100)?;
    let curriculum = Curriculum {
        schema_version: TEACHER_SCHOOL_SCHEMA_VERSION,
        steps: vec![CurriculumStep {
            lesson_id,
            role: TeacherRole::Tutor,
            kind: CurriculumStepKind::NameObject,
            prompt_cues: vec![
                TeacherPerceptualEvent::spoken_token(lesson_id, 77),
                TeacherPerceptualEvent::object_highlight(
                    lesson_id,
                    object_id,
                    NormalizedScalar::new(0.85)?,
                ),
                TeacherPerceptualEvent::social_feedback(
                    lesson_id,
                    FeedbackPolarity::Praise,
                    Confidence::new(0.8)?,
                ),
            ],
            expected_observations: vec![
                ExpectedObservation::HeardToken(77),
                ExpectedObservation::ObjectHighlighted(object_id),
            ],
            verifier_checks: vec![
                VerifierCheck::HeardToken {
                    token_id: 77,
                    channel: TeacherPerceptionChannel::Hearing,
                },
                VerifierCheck::NoHiddenSemanticContext,
                VerifierCheck::NoDirectTeacherActionSelection,
                VerifierCheck::SelectedByArbitration,
            ],
            feedback_events: vec![TeacherPerceptualEvent::visible_reward(
                lesson_id,
                NormalizedScalar::new(0.65)?,
            )],
            response_channels: vec![TeacherLessonResponseChannel::Speech],
        }],
    };
    if curriculum.schema_version != TEACHER_SCHOOL_SCHEMA_VERSION
        || !curriculum.lesson_ids_are_unique()
    {
        return Err(GameAppShellError::Core(ScaffoldContractError::InvalidId));
    }
    let mut runner = HeadlessCurriculumRunner::new(curriculum.clone());
    let dispatch = runner.dispatch_current()?;
    let current_step = runner
        .current_step()
        .cloned()
        .ok_or(GameAppShellError::Core(ScaffoldContractError::InvalidId))?;
    let contract = TeacherChannelContract::grounded_default();
    if !dispatch
        .perception_events
        .iter()
        .all(|event| contract.accepts_event(event))
    {
        return Err(GameAppShellError::Core(ScaffoldContractError::InvalidId));
    }

    let sensory = world.sensory_report(config.learner_id, Tick::ZERO)?;
    let sensory_heard_tokens = sensory
        .core_snapshot
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .map(|token| token.token_id)
        .collect::<Vec<_>>();
    let sensory_teacher_channels = sensory
        .core_snapshot
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .filter_map(|token| token.teacher_channel)
        .collect::<Vec<_>>();
    if !sensory_heard_tokens.contains(&77)
        || !sensory_teacher_channels.contains(&TeacherPerceptionChannel::Hearing)
    {
        return Err(GameAppShellError::Core(ScaffoldContractError::InvalidId));
    }

    let mut harness = HeadlessBrainHarness::new(world);
    let mut mind = CreatureMind::scaffold(
        config.learner_id,
        BrainScaleTier::Nano512,
        config.seed,
        Tick::ZERO,
    )?;
    let tick = harness.tick_mind(
        &mut mind,
        BrainTickInput::new(
            Tick::ZERO,
            vec![proposal(
                ActionKind::Inspect.canonical_id(),
                ActionKind::Inspect,
                Some(cue_token_id),
                None,
                0.82,
                0.90,
                0.0,
            )?],
        )
        .with_pack_experience(true)
        .with_action_duration(DurationTicks::new(1)),
    );
    if tick.brain.experience_patch.is_none() {
        return Err(GameAppShellError::Core(
            ScaffoldContractError::MissingPhaseData,
        ));
    }

    let topology_summary = tick
        .brain
        .topology_update
        .as_ref()
        .map(|update| TopologySummary {
            concept_count: 1,
            edge_count: update.edge_ids.len(),
            simplex_count: 1,
            gap_count: update.gap_ids.len(),
        })
        .unwrap_or_default();
    let evidence = SchoolEvidence::new(&harness.telemetry().sealed_patches)
        .with_memory_record_count(usize::from(tick.brain.memory_update.is_some()))
        .with_topology_summary(topology_summary);
    let verification =
        PatchLogLessonVerifier.verify_checks(&current_step.verifier_checks, &evidence)?;
    let advanced = runner.observe_verification(&verification)?;
    if !advanced {
        return Err(GameAppShellError::Core(ScaffoldContractError::InvalidId));
    }

    let response = LessonResponse::new(
        lesson_id,
        LessonResponseKind::CreatureVocalized,
        TeacherLessonResponseChannel::Speech,
    )
    .with_teacher_entity(teacher_avatar_stable_id);
    let metadata = response.to_action_metadata()?;
    let teacher_tagged_low = proposal(
        ActionId(10_701),
        ActionKind::Vocalize,
        Some(teacher_avatar_stable_id),
        None,
        0.30,
        0.90,
        0.0,
    )?
    .with_teacher_lesson(Some(metadata));
    let ordinary_high = proposal(
        ActionId(10_702),
        ActionKind::Inspect,
        Some(object_id),
        None,
        0.90,
        0.90,
        0.0,
    )?;
    let arbitration = cpu_reference_arbitrate(
        config.learner_id,
        &[teacher_tagged_low, ordinary_high],
        ActionArbitrationConfig::default(),
    )?;
    let teacher_metadata_bypass_blocked = arbitration.selected.action_id == ActionId(10_702)
        && arbitration.selected.teacher_lesson.is_none();

    let cue_lookup = [
        (
            TeacherInputKind::SpokenToken,
            Some(cue_token_id),
            "heard teacher word",
        ),
        (
            TeacherInputKind::ObjectHighlight,
            Some(object_id),
            "highlighted teaching object",
        ),
        (
            TeacherInputKind::SocialFeedback,
            Some(teacher_avatar_stable_id),
            "visible teacher praise",
        ),
    ];
    let cues = dispatch
        .perception_events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let cue_entity = cue_lookup
                .iter()
                .find(|(kind, _, _)| *kind == event.input_kind)
                .and_then(|(_, entity, _)| *entity);
            let label = cue_lookup
                .iter()
                .find(|(kind, _, _)| *kind == event.input_kind)
                .map(|(_, _, label)| *label)
                .unwrap_or("teacher perception cue");
            SchoolCuePresentation::from_event(
                *event,
                cue_entity,
                format!("{} #{}", label, index + 1),
            )
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    let p34_school = SchoolSaveState {
        schema_version: TEACHER_SCHOOL_SCHEMA_VERSION,
        enabled: true,
        active_curriculum_id: Some(config.curriculum_id.clone()),
        teacher_private_state_saved: false,
    };
    let lesson_panel = SchoolLessonPanel {
        curriculum_id: config.curriculum_id.clone(),
        active_lesson_id: current_step.lesson_id,
        step_kind: current_step.kind,
        total_steps: curriculum.steps.len().min(config.lesson_limit),
        completed_steps: runner.completed_step_count(),
        cue_count: cues.len(),
        response_channels: current_step.response_channels.clone(),
        status_line: format!(
            "lesson {:?} passed={} completed={}",
            current_step.kind,
            verification.passed,
            runner.completed_step_count()
        ),
    };
    let verifier_panel = SchoolVerifierPanel {
        passed: verification.passed,
        observed_checks: verification
            .observed_checks
            .iter()
            .map(|check| format!("{check:?}"))
            .collect(),
        failed_checks: verification
            .failed_checks
            .iter()
            .map(|check| format!("{check:?}"))
            .collect(),
        sealed_patch_count: harness.telemetry().sealed_patches.len(),
        verifier_message: if verification.passed {
            "sealed patch verifier passed; teacher remained perception-only".to_string()
        } else {
            "sealed patch verifier failed".to_string()
        },
    };
    let save_preview = SchoolModeSaveState {
        schema: G10_SCHOOL_MODE_SCHEMA.to_string(),
        schema_version: G10_SCHOOL_MODE_SCHEMA_VERSION,
        seed: config.seed,
        curriculum_id: config.curriculum_id.clone(),
        active_lesson_id: lesson_id.raw(),
        completed_steps: lesson_panel.completed_steps,
        teacher_avatar_stable_id,
        cue_entity_ids: cues.iter().filter_map(|cue| cue.cue_entity).collect(),
        verifier_passed: verifier_panel.passed,
        p34_school: p34_school.clone(),
    };
    let json = save_preview.to_json_string_pretty()?;
    let save_roundtrip = SchoolModeSaveState::from_json_str(&json)?;
    let summary = SchoolModeSummary {
        schema: G10_SCHOOL_MODE_SCHEMA,
        schema_version: G10_SCHOOL_MODE_SCHEMA_VERSION,
        seed: config.seed,
        teacher_avatar_stable_id,
        learner_stable_id,
        lesson_panel,
        cues,
        verifier_panel,
        sensory_heard_tokens,
        sensory_teacher_channels,
        teacher_metadata_bypass_blocked,
        teacher_selected_action_id: arbitration
            .selected
            .teacher_lesson
            .map(|_| arbitration.selected.action_id),
        world_signature: harness.world().stable_signature(),
        p34_school,
        save_roundtrip_signature: save_roundtrip.signature_line(),
    };
    summary.validate()?;
    Ok(summary)
}
