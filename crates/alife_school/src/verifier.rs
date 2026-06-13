//! v0 scaffold: lesson verifiers over sealed experience evidence.

use alife_core::{
    validate_finite, ActionDecisionStatus, ExperiencePatch, ScaffoldContractError,
    TeacherPerceptionChannel, Validate,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TopologySummary {
    pub concept_count: usize,
    pub edge_count: usize,
    pub simplex_count: usize,
    pub gap_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct SchoolEvidence<'a> {
    pub patches: &'a [ExperiencePatch],
    pub memory_record_count: usize,
    pub topology_summary: TopologySummary,
}

impl<'a> SchoolEvidence<'a> {
    pub const fn new(patches: &'a [ExperiencePatch]) -> Self {
        Self {
            patches,
            memory_record_count: 0,
            topology_summary: TopologySummary {
                concept_count: 0,
                edge_count: 0,
                simplex_count: 0,
                gap_count: 0,
            },
        }
    }

    pub const fn with_memory_record_count(mut self, memory_record_count: usize) -> Self {
        self.memory_record_count = memory_record_count;
        self
    }

    pub const fn with_topology_summary(mut self, topology_summary: TopologySummary) -> Self {
        self.topology_summary = topology_summary;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerifierCheck {
    HeardToken {
        token_id: u32,
        channel: TeacherPerceptionChannel,
    },
    RewardAtLeast(f32),
    NoHiddenSemanticContext,
    NoDirectTeacherActionSelection,
    SelectedByArbitration,
    MinimumMemoryRecords(usize),
    MinimumTopologyConcepts(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LessonVerification {
    pub passed: bool,
    pub observed_checks: Vec<VerifierCheck>,
    pub failed_checks: Vec<VerifierCheck>,
}

pub trait LessonVerifier {
    fn verify_checks(
        &self,
        checks: &[VerifierCheck],
        evidence: &SchoolEvidence<'_>,
    ) -> Result<LessonVerification, ScaffoldContractError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PatchLogLessonVerifier;

impl PatchLogLessonVerifier {
    pub fn verify_checks(
        &self,
        checks: &[VerifierCheck],
        evidence: &SchoolEvidence<'_>,
    ) -> Result<LessonVerification, ScaffoldContractError> {
        <Self as LessonVerifier>::verify_checks(self, checks, evidence)
    }
}

impl LessonVerifier for PatchLogLessonVerifier {
    fn verify_checks(
        &self,
        checks: &[VerifierCheck],
        evidence: &SchoolEvidence<'_>,
    ) -> Result<LessonVerification, ScaffoldContractError> {
        for patch in evidence.patches {
            patch.validate_contract()?;
        }
        let mut observed_checks = Vec::new();
        let mut failed_checks = Vec::new();
        for check in checks {
            validate_check(*check)?;
            if check_passes(*check, evidence) {
                observed_checks.push(*check);
            } else {
                failed_checks.push(*check);
            }
        }
        Ok(LessonVerification {
            passed: failed_checks.is_empty(),
            observed_checks,
            failed_checks,
        })
    }
}

fn validate_check(check: VerifierCheck) -> Result<(), ScaffoldContractError> {
    match check {
        VerifierCheck::HeardToken { token_id, .. } => {
            if token_id == 0 {
                Err(ScaffoldContractError::InvalidId)
            } else {
                Ok(())
            }
        }
        VerifierCheck::RewardAtLeast(threshold) => {
            validate_finite(threshold)?;
            if (-1.0..=1.0).contains(&threshold) {
                Ok(())
            } else {
                Err(ScaffoldContractError::ScalarOutOfRange)
            }
        }
        VerifierCheck::NoHiddenSemanticContext
        | VerifierCheck::NoDirectTeacherActionSelection
        | VerifierCheck::SelectedByArbitration
        | VerifierCheck::MinimumMemoryRecords(_)
        | VerifierCheck::MinimumTopologyConcepts(_) => Ok(()),
    }
}

fn check_passes(check: VerifierCheck, evidence: &SchoolEvidence<'_>) -> bool {
    match check {
        VerifierCheck::HeardToken { token_id, channel } => evidence
            .patches
            .iter()
            .any(|patch| heard_token_matches(patch, token_id, channel)),
        VerifierCheck::RewardAtLeast(threshold) => evidence
            .patches
            .iter()
            .any(|patch| patch.outcome().reward_valence.raw() >= threshold),
        VerifierCheck::NoHiddenSemanticContext => evidence.patches.iter().all(|patch| {
            patch.pre_action().sensory.semantic_context.is_none()
                && patch.pre_action().sensory.gaussian_context.is_none()
        }),
        VerifierCheck::NoDirectTeacherActionSelection => evidence
            .patches
            .iter()
            .all(selected_action_came_from_arbitration),
        VerifierCheck::SelectedByArbitration => evidence
            .patches
            .iter()
            .all(selected_action_came_from_arbitration),
        VerifierCheck::MinimumMemoryRecords(min) => evidence.memory_record_count >= min,
        VerifierCheck::MinimumTopologyConcepts(min) => {
            evidence.topology_summary.concept_count >= min
        }
    }
}

fn heard_token_matches(
    patch: &ExperiencePatch,
    token_id: u32,
    channel: TeacherPerceptionChannel,
) -> bool {
    patch
        .pre_action()
        .sensory
        .language_context
        .heard_tokens
        .iter()
        .chain(
            patch
                .pre_action()
                .sensory
                .context_streams
                .vocal_tokens
                .iter(),
        )
        .flatten()
        .any(|token| token.token_id == token_id && token.teacher_channel == Some(channel))
}

fn selected_action_came_from_arbitration(patch: &ExperiencePatch) -> bool {
    let decision = patch.decision();
    let selected_id = decision.selected_action.action_id;
    match decision.status {
        ActionDecisionStatus::Selected => {
            decision
                .proposals
                .iter()
                .any(|proposal| proposal.action_id == selected_id)
                && decision.arbitration_trace.wta_result.selected_action_id == Some(selected_id)
        }
        ActionDecisionStatus::FallbackSelected => decision
            .arbitration_trace
            .wta_result
            .selected_action_id
            .is_none(),
    }
}
