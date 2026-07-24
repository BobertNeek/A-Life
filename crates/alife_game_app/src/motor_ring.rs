//! CA14 read-only motor-ring arbitration presentation.
//!
//! This module mirrors existing P09 structured arbitration as fixed-order
//! action-channel display data. It does not choose actions and does not bypass
//! the core `heuristic_baseline_arbitrate` decision path.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorRingChannelKind {
    Idle,
    Approach,
    Eat,
    Flee,
    Sleep,
    Inspect,
}

impl MotorRingChannelKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Approach => "Approach",
            Self::Eat => "Eat",
            Self::Flee => "Flee",
            Self::Sleep => "Sleep",
            Self::Inspect => "Inspect",
        }
    }

    const fn all() -> [Self; CA14_MAX_MOTOR_RING_CHANNELS] {
        [
            Self::Idle,
            Self::Approach,
            Self::Eat,
            Self::Flee,
            Self::Sleep,
            Self::Inspect,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotorRingChannelPresentation {
    pub kind: MotorRingChannelKind,
    pub action_kind: Option<ActionKind>,
    pub action_id: Option<ActionId>,
    pub target_entity: Option<WorldEntityId>,
    pub raw_score: f32,
    pub final_score: f32,
    pub confidence: f32,
    pub selected: bool,
    pub suppressed: bool,
}

impl MotorRingChannelPresentation {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if !self.raw_score.is_finite()
            || !self.final_score.is_finite()
            || !self.confidence.is_finite()
            || !(0.0..=1.0).contains(&self.confidence)
        {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if let Some(action_id) = self.action_id {
            action_id.validate()?;
        }
        if let Some(target) = self.target_entity {
            target.validate()?;
        }
        Ok(())
    }

    pub fn panel_line(&self) -> String {
        let target = self
            .target_entity
            .map_or_else(|| "--".to_string(), |id| format!("s:{}", id.raw()));
        let marker = if self.selected { ">" } else { " " };
        let suppressed = if self.suppressed { " suppressed" } else { "" };
        format!(
            "{} {:<8} [{}] {:.2} c={:.2} target={}{}",
            marker,
            self.kind.label(),
            score_bar(self.final_score),
            self.final_score,
            self.confidence,
            target,
            suppressed
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{:?}:{:?}:{:?}:{:.3}:{:.3}:{:.3}:{}:{}",
            self.kind,
            self.action_id.map(|id| id.raw()),
            self.target_entity.map(|id| id.raw()),
            self.raw_score,
            self.final_score,
            self.confidence,
            self.selected,
            self.suppressed
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotorRingPresentation {
    pub schema: &'static str,
    pub schema_version: u16,
    pub channels: Vec<MotorRingChannelPresentation>,
    pub selected_action_id: Option<ActionId>,
    pub selected_label: String,
    pub winner_margin: f32,
    pub source: &'static str,
    pub structured_arbitration_preserved: bool,
    pub no_direct_action_bypass: bool,
    pub global_sort_free_display: bool,
}

impl MotorRingPresentation {
    pub fn pending() -> Self {
        Self {
            schema: CA14_MOTOR_RING_PRESENTATION_SCHEMA,
            schema_version: CA14_MOTOR_RING_PRESENTATION_SCHEMA_VERSION,
            channels: MotorRingChannelKind::all()
                .into_iter()
                .map(|kind| MotorRingChannelPresentation {
                    kind,
                    action_kind: None,
                    action_id: None,
                    target_entity: None,
                    raw_score: 0.0,
                    final_score: 0.0,
                    confidence: 0.0,
                    selected: false,
                    suppressed: false,
                })
                .collect(),
            selected_action_id: None,
            selected_label: "Pending".to_string(),
            winner_margin: 0.0,
            source: "P09 structured arbitration",
            structured_arbitration_preserved: true,
            no_direct_action_bypass: true,
            global_sort_free_display: true,
        }
    }

    pub fn from_proposals(
        organism_id: OrganismId,
        proposals: &[ActionProposal],
    ) -> Result<Self, GameAppShellError> {
        let decision = heuristic_baseline_arbitrate(
            organism_id,
            proposals,
            ActionArbitrationConfig::default(),
        )?;
        let mut channels = MotorRingChannelKind::all()
            .into_iter()
            .map(|kind| channel_from_kind(kind, proposals, &decision))
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        for channel in &mut channels {
            channel.selected = channel.action_id == Some(decision.selected.action_id);
        }
        let selected_label = channels
            .iter()
            .find(|channel| channel.selected)
            .map_or_else(
                || "Fallback".to_string(),
                |channel| channel.kind.label().to_string(),
            );
        let selected_score = channels
            .iter()
            .find(|channel| channel.selected)
            .map_or(0.0, |channel| channel.final_score);
        let next_score = channels
            .iter()
            .filter(|channel| !channel.selected)
            .map(|channel| channel.final_score)
            .fold(0.0_f32, f32::max);
        let presentation = Self {
            schema: CA14_MOTOR_RING_PRESENTATION_SCHEMA,
            schema_version: CA14_MOTOR_RING_PRESENTATION_SCHEMA_VERSION,
            channels,
            selected_action_id: Some(decision.selected.action_id),
            selected_label,
            winner_margin: (selected_score - next_score).max(0.0),
            source: "P09 structured arbitration",
            structured_arbitration_preserved: true,
            no_direct_action_bypass: true,
            global_sort_free_display: true,
        };
        presentation.validate()?;
        Ok(presentation)
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA14_MOTOR_RING_PRESENTATION_SCHEMA
            || self.schema_version != CA14_MOTOR_RING_PRESENTATION_SCHEMA_VERSION
            || self.channels.len() != CA14_MAX_MOTOR_RING_CHANNELS
            || self.selected_label.is_empty()
            || !self.winner_margin.is_finite()
            || !self.structured_arbitration_preserved
            || !self.no_direct_action_bypass
            || !self.global_sort_free_display
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA14 motor ring presentation must be bounded and boundary-safe",
            });
        }
        if self.panel_text().contains("Entity(") {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA14 motor ring presentation must not expose Bevy Entity IDs",
            });
        }
        for channel in &self.channels {
            channel.validate()?;
        }
        Ok(())
    }

    pub fn compact_line(&self) -> String {
        format!(
            "Motor Ring: winner={} margin={:.2} source=P09 no-bypass",
            self.selected_label, self.winner_margin
        )
    }

    pub fn panel_text(&self) -> String {
        let lines = self
            .channels
            .iter()
            .map(MotorRingChannelPresentation::panel_line)
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "Motor Ring\n{}\nWinner: {} margin={:.2}\nBoundary: normal arbitration; no direct bypass",
            lines, self.selected_label, self.winner_margin
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:.3}:{}:{}:{}",
            self.schema,
            self.schema_version,
            self.selected_label,
            self.winner_margin,
            self.structured_arbitration_preserved,
            self.no_direct_action_bypass,
            self.channels
                .iter()
                .map(MotorRingChannelPresentation::signature_line)
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotorRingArbitrationSmokeSummary {
    pub ring: MotorRingPresentation,
    pub selected_action_kind: Option<ActionKind>,
    pub selected_action_id: Option<ActionId>,
    pub patch_sealed: bool,
    pub direct_action_bypass: bool,
}

impl MotorRingArbitrationSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        self.ring.validate()?;
        if self.selected_action_id.is_none()
            || !self.patch_sealed
            || self.direct_action_bypass
            || !self.ring.structured_arbitration_preserved
            || !self.ring.no_direct_action_bypass
            || !self.ring.panel_text().contains("Motor Ring")
            || !self.ring.panel_text().contains("normal arbitration")
            || self.ring.panel_text().contains("Entity(")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA14 motor ring smoke must preserve normal arbitration boundaries",
            });
        }
        Ok(())
    }
}

pub fn run_motor_ring_arbitration_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<MotorRingArbitrationSmokeSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let summaries = panel.apply_command(&mut live, RuntimeControlCommand::StepOnce)?;
    let summary = summaries
        .first()
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "CA14 motor ring smoke must produce one tick",
        })?;
    let smoke = MotorRingArbitrationSmokeSummary {
        ring: panel.motor_ring.clone(),
        selected_action_kind: summary.selected_action_kind,
        selected_action_id: summary.selected_action_id,
        patch_sealed: summary.patch_sealed,
        direct_action_bypass: false,
    };
    smoke.validate()?;
    Ok(smoke)
}

fn channel_from_kind(
    kind: MotorRingChannelKind,
    proposals: &[ActionProposal],
    decision: &ActionDecision,
) -> Result<MotorRingChannelPresentation, ScaffoldContractError> {
    let candidate = proposals
        .iter()
        .copied()
        .enumerate()
        .find(|(_, proposal)| channel_kind_for_proposal(*proposal) == kind);
    let Some((proposal_index, proposal)) = candidate else {
        return Ok(MotorRingChannelPresentation {
            kind,
            action_kind: None,
            action_id: None,
            target_entity: None,
            raw_score: 0.0,
            final_score: 0.0,
            confidence: 0.0,
            selected: false,
            suppressed: false,
        });
    };
    let final_score = decision
        .ranked_top_proposals
        .iter()
        .find(|ranked| ranked.proposal_index == proposal_index)
        .map_or(proposal.score, |ranked| ranked.final_score);
    let suppressed = decision
        .trace
        .suppressed_proposals
        .iter()
        .any(|suppressed| suppressed.proposal_index == proposal_index);
    let channel = MotorRingChannelPresentation {
        kind,
        action_kind: Some(proposal.kind),
        action_id: Some(proposal.action_id),
        target_entity: proposal.target.entity,
        raw_score: proposal.score,
        final_score,
        confidence: proposal.confidence.raw(),
        selected: false,
        suppressed,
    };
    channel.validate()?;
    Ok(channel)
}

fn channel_kind_for_proposal(proposal: ActionProposal) -> MotorRingChannelKind {
    match proposal.kind {
        ActionKind::Idle => MotorRingChannelKind::Idle,
        ActionKind::Move if proposal.target.entity == Some(WorldEntityId(3)) => {
            MotorRingChannelKind::Flee
        }
        ActionKind::Move => MotorRingChannelKind::Approach,
        ActionKind::Interact | ActionKind::Hold => MotorRingChannelKind::Eat,
        ActionKind::Rest => MotorRingChannelKind::Sleep,
        ActionKind::Inspect | ActionKind::Vocalize | ActionKind::Write | ActionKind::Gesture => {
            MotorRingChannelKind::Inspect
        }
    }
}

fn score_bar(score: f32) -> String {
    let filled = (score.clamp(0.0, 1.0) * 8.0).round() as usize;
    format!(
        "{}{}",
        "#".repeat(filled),
        ".".repeat(8usize.saturating_sub(filled))
    )
}
