//! v0 scaffold: grounded school curriculum definitions.

use alife_core::{
    Confidence, NormalizedScalar, TeacherLessonResponseChannel, TeacherPerceptionChannel,
    WorldEntityId,
};

use crate::{
    FeedbackPolarity, LessonId, TeacherPerceptualEvent, TeacherRole, VerifierCheck,
    TEACHER_SCHOOL_SCHEMA_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurriculumStepKind {
    NameObject,
    OfferFood,
    DiscouragePoison,
    RequestApproach,
    RequestGrab,
    RequestVocalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedObservation {
    HeardToken(u32),
    ObjectHighlighted(WorldEntityId),
    PositiveReward,
    NegativeFeedback,
    ApproachRequested,
    GrabRequested,
    VocalizationRequested,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurriculumStep {
    pub lesson_id: LessonId,
    pub role: TeacherRole,
    pub kind: CurriculumStepKind,
    pub prompt_cues: Vec<TeacherPerceptualEvent>,
    pub expected_observations: Vec<ExpectedObservation>,
    pub verifier_checks: Vec<VerifierCheck>,
    pub feedback_events: Vec<TeacherPerceptualEvent>,
    pub response_channels: Vec<TeacherLessonResponseChannel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Curriculum {
    pub schema_version: u16,
    pub steps: Vec<CurriculumStep>,
}

impl Curriculum {
    pub fn grounded_object_food_poison() -> Self {
        let ids = [
            LessonId::new(2301).expect("nonzero lesson id"),
            LessonId::new(2302).expect("nonzero lesson id"),
            LessonId::new(2303).expect("nonzero lesson id"),
            LessonId::new(2304).expect("nonzero lesson id"),
            LessonId::new(2305).expect("nonzero lesson id"),
            LessonId::new(2306).expect("nonzero lesson id"),
        ];
        let object = WorldEntityId(7);
        let food = WorldEntityId(8);
        let poison = WorldEntityId(9);
        Self {
            schema_version: TEACHER_SCHOOL_SCHEMA_VERSION,
            steps: vec![
                step(
                    ids[0],
                    CurriculumStepKind::NameObject,
                    vec![
                        TeacherPerceptualEvent::spoken_token(ids[0], 77),
                        TeacherPerceptualEvent::object_highlight(
                            ids[0],
                            object,
                            NormalizedScalar(0.85),
                        ),
                    ],
                    vec![
                        ExpectedObservation::HeardToken(77),
                        ExpectedObservation::ObjectHighlighted(object),
                    ],
                    vec![
                        VerifierCheck::HeardToken {
                            token_id: 77,
                            channel: TeacherPerceptionChannel::Hearing,
                        },
                        VerifierCheck::NoHiddenSemanticContext,
                    ],
                    vec![TeacherPerceptualEvent::social_feedback(
                        ids[0],
                        FeedbackPolarity::Praise,
                        Confidence(0.8),
                    )],
                    vec![TeacherLessonResponseChannel::Speech],
                ),
                step(
                    ids[1],
                    CurriculumStepKind::OfferFood,
                    vec![
                        TeacherPerceptualEvent::spoken_token(ids[1], 41),
                        TeacherPerceptualEvent::object_highlight(
                            ids[1],
                            food,
                            NormalizedScalar(0.9),
                        ),
                    ],
                    vec![ExpectedObservation::PositiveReward],
                    vec![VerifierCheck::RewardAtLeast(0.1)],
                    vec![TeacherPerceptualEvent::visible_reward(
                        ids[1],
                        NormalizedScalar(0.75),
                    )],
                    vec![TeacherLessonResponseChannel::Demonstration],
                ),
                step(
                    ids[2],
                    CurriculumStepKind::DiscouragePoison,
                    vec![
                        TeacherPerceptualEvent::spoken_token(ids[2], 66),
                        TeacherPerceptualEvent::object_highlight(
                            ids[2],
                            poison,
                            NormalizedScalar(0.9),
                        ),
                    ],
                    vec![ExpectedObservation::NegativeFeedback],
                    vec![VerifierCheck::NoDirectTeacherActionSelection],
                    vec![TeacherPerceptualEvent::visible_punishment(
                        ids[2],
                        NormalizedScalar(0.7),
                    )],
                    vec![TeacherLessonResponseChannel::Feedback],
                ),
                step(
                    ids[3],
                    CurriculumStepKind::RequestApproach,
                    vec![
                        TeacherPerceptualEvent::spoken_token(ids[3], 101),
                        TeacherPerceptualEvent::gesture(ids[3], 1),
                    ],
                    vec![ExpectedObservation::ApproachRequested],
                    vec![VerifierCheck::SelectedByArbitration],
                    vec![TeacherPerceptualEvent::social_feedback(
                        ids[3],
                        FeedbackPolarity::Praise,
                        Confidence(0.7),
                    )],
                    vec![TeacherLessonResponseChannel::Gesture],
                ),
                step(
                    ids[4],
                    CurriculumStepKind::RequestGrab,
                    vec![
                        TeacherPerceptualEvent::spoken_token(ids[4], 211),
                        TeacherPerceptualEvent::gesture(ids[4], 2),
                    ],
                    vec![ExpectedObservation::GrabRequested],
                    vec![VerifierCheck::SelectedByArbitration],
                    vec![TeacherPerceptualEvent::social_feedback(
                        ids[4],
                        FeedbackPolarity::Praise,
                        Confidence(0.7),
                    )],
                    vec![TeacherLessonResponseChannel::Demonstration],
                ),
                step(
                    ids[5],
                    CurriculumStepKind::RequestVocalize,
                    vec![
                        TeacherPerceptualEvent::spoken_token(ids[5], 400),
                        TeacherPerceptualEvent::gesture(ids[5], 3),
                    ],
                    vec![ExpectedObservation::VocalizationRequested],
                    vec![VerifierCheck::SelectedByArbitration],
                    vec![TeacherPerceptualEvent::social_feedback(
                        ids[5],
                        FeedbackPolarity::Praise,
                        Confidence(0.7),
                    )],
                    vec![TeacherLessonResponseChannel::Speech],
                ),
            ],
        }
    }

    pub fn lesson_ids_are_unique(&self) -> bool {
        let mut ids = self
            .steps
            .iter()
            .map(|step| step.lesson_id.raw())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        ids.len() == self.steps.len()
    }
}

fn step(
    lesson_id: LessonId,
    kind: CurriculumStepKind,
    prompt_cues: Vec<TeacherPerceptualEvent>,
    expected_observations: Vec<ExpectedObservation>,
    verifier_checks: Vec<VerifierCheck>,
    feedback_events: Vec<TeacherPerceptualEvent>,
    response_channels: Vec<TeacherLessonResponseChannel>,
) -> CurriculumStep {
    CurriculumStep {
        lesson_id,
        role: TeacherRole::Tutor,
        kind,
        prompt_cues,
        expected_observations,
        verifier_checks,
        feedback_events,
        response_channels,
    }
}
