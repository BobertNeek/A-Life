//! CA28 read-only topological concept overlay for graphical cognition inspection.
//!
//! This module mirrors the engine-independent `alife_core` topology ledger into
//! bounded player/dev UI rows. It never emits actions and never mutates
//! cognition.

use crate::prelude::*;
use crate::*;

pub const CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA: &str =
    "alife.ca28.topological_concept_overlay.v1";
pub const CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA_VERSION: u16 = 1;
pub const CA28_MAX_CONCEPT_NODES: usize = 4;
pub const CA28_MAX_EDGE_ROWS: usize = 3;
pub const CA28_MAX_GAP_ROWS: usize = 3;
pub const CA28_MAX_EVENT_LINKS: usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct TopologicalConceptNodeRow {
    pub concept_id: ConceptCellId,
    pub label: String,
    pub observation_count: u32,
    pub salience: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopologicalConceptEdgeRow {
    pub edge_id: CognitiveEdgeId,
    pub from: ConceptCellId,
    pub to: ConceptCellId,
    pub relation: EdgeRelationKind,
    pub strength: f32,
    pub evidence_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopologicalConceptGapRow {
    pub gap_id: UnresolvedGapId,
    pub source_concepts: Vec<ConceptCellId>,
    pub status: GapResolutionStatus,
    pub salience: f32,
    pub curiosity_voltage: f32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopologicalBehaviorEventLink {
    pub tick: u64,
    pub sequence_id: Option<u64>,
    pub target_entity: Option<WorldEntityId>,
    pub action_kind: Option<ActionKind>,
    pub topology_updates: u32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopologicalConceptOverlaySnapshot {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub concept_count: usize,
    pub edge_count: usize,
    pub gap_count: usize,
    pub nodes: Vec<TopologicalConceptNodeRow>,
    pub edges: Vec<TopologicalConceptEdgeRow>,
    pub gaps: Vec<TopologicalConceptGapRow>,
    pub event_links: Vec<TopologicalBehaviorEventLink>,
    pub read_only: bool,
    pub bias_only: bool,
    pub can_emit_actions: bool,
    pub direct_cognition_mutation_allowed: bool,
}

impl TopologicalConceptOverlaySnapshot {
    pub fn from_live_loop(
        live: &LiveBrainLoop,
        recent_summaries: &[LiveBrainTickSummary],
    ) -> Result<Self, GameAppShellError> {
        let map = live.mind().topological_map();
        let nodes = map
            .concepts()
            .iter()
            .rev()
            .take(CA28_MAX_CONCEPT_NODES)
            .map(concept_node_row)
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let edges = map
            .edges()
            .iter()
            .rev()
            .take(CA28_MAX_EDGE_ROWS)
            .map(edge_row)
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let gaps = map
            .unresolved_gaps()
            .iter()
            .rev()
            .take(CA28_MAX_GAP_ROWS)
            .map(gap_row)
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let event_links = recent_summaries
            .iter()
            .rev()
            .filter(|summary| summary.topology_updates > 0 || summary.patch_sealed)
            .take(CA28_MAX_EVENT_LINKS)
            .map(event_link_row)
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;

        let snapshot = Self {
            schema: CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA,
            schema_version: CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA_VERSION,
            organism_id: live.organism_id(),
            tick: live.mind().current_tick(),
            concept_count: map.concepts().len(),
            edge_count: map.edges().len(),
            gap_count: map.unresolved_gaps().len(),
            nodes,
            edges,
            gaps,
            event_links,
            read_only: true,
            bias_only: true,
            can_emit_actions: false,
            direct_cognition_mutation_allowed: false,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn pending(organism_id: OrganismId, tick: Tick) -> Self {
        Self {
            schema: CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA,
            schema_version: CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA_VERSION,
            organism_id,
            tick,
            concept_count: 0,
            edge_count: 0,
            gap_count: 0,
            nodes: Vec::new(),
            edges: Vec::new(),
            gaps: Vec::new(),
            event_links: Vec::new(),
            read_only: true,
            bias_only: true,
            can_emit_actions: false,
            direct_cognition_mutation_allowed: false,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA
            || self.schema_version != CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.organism_id.validate()?;
        if !self.read_only
            || !self.bias_only
            || self.can_emit_actions
            || self.direct_cognition_mutation_allowed
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if self.nodes.len() > CA28_MAX_CONCEPT_NODES
            || self.edges.len() > CA28_MAX_EDGE_ROWS
            || self.gaps.len() > CA28_MAX_GAP_ROWS
            || self.event_links.len() > CA28_MAX_EVENT_LINKS
        {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }
        for node in &self.nodes {
            node.concept_id.validate()?;
            validate_unit(node.salience)?;
            validate_unit(node.confidence)?;
            validate_display_line(&node.label)?;
        }
        for edge in &self.edges {
            edge.edge_id.validate()?;
            edge.from.validate()?;
            edge.to.validate()?;
            validate_unit(edge.strength)?;
        }
        for gap in &self.gaps {
            gap.gap_id.validate()?;
            if gap.source_concepts.is_empty() {
                return Err(ScaffoldContractError::MissingPhaseData);
            }
            for concept in &gap.source_concepts {
                concept.validate()?;
            }
            validate_unit(gap.salience)?;
            validate_unit(gap.curiosity_voltage)?;
            validate_display_line(&gap.label)?;
        }
        for event in &self.event_links {
            if let Some(target) = event.target_entity {
                target.validate()?;
            }
            validate_display_line(&event.label)?;
        }
        Ok(())
    }

    pub fn panel_text(&self) -> String {
        let node_line = self.nodes.first().map_or_else(
            || "node: waiting for sealed topology update".to_string(),
            |node| {
                format!(
                    "node c{} {} sal={:.2} obs={}",
                    node.concept_id.raw(),
                    node.label,
                    node.salience,
                    node.observation_count
                )
            },
        );
        let edge_line = self.edges.first().map_or_else(
            || "edge: pending concept relation".to_string(),
            |edge| {
                format!(
                    "edge e{} c{}->c{} {:?} s={:.2}",
                    edge.edge_id.raw(),
                    edge.from.raw(),
                    edge.to.raw(),
                    edge.relation,
                    edge.strength
                )
            },
        );
        let gap_line = self.gaps.first().map_or_else(
            || "gap: none open".to_string(),
            |gap| {
                format!(
                    "gap g{} {} {:?} cv={:.2}",
                    gap.gap_id.raw(),
                    gap.label,
                    gap.status,
                    gap.curiosity_voltage
                )
            },
        );
        let event_line = self.event_links.first().map_or_else(
            || "event link: pending sealed behavior".to_string(),
            |event| event.label.clone(),
        );
        format!(
            concat!(
                "Concept Map (read-only)\n",
                "nodes={} edges={} gaps={} tick={}\n",
                "{}\n",
                "{}\n",
                "{}\n",
                "{}\n",
                "Boundary: bias/context only; no actions"
            ),
            self.concept_count,
            self.edge_count,
            self.gap_count,
            self.tick.raw(),
            node_line,
            edge_line,
            gap_line,
            event_line
        )
    }

    pub fn compact_line(&self) -> String {
        format!(
            "Concepts: nodes={} edges={} gaps={} read-only bias-only",
            self.concept_count, self.edge_count, self.gap_count
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:org={}:tick={}:nodes={}:edges={}:gaps={}:events={}:readonly={}:biasonly={}:actions={}",
            self.schema,
            self.schema_version,
            self.organism_id.raw(),
            self.tick.raw(),
            self.concept_count,
            self.edge_count,
            self.gap_count,
            self.event_links.len(),
            self.read_only,
            self.bias_only,
            self.can_emit_actions
        )
    }

    pub fn preserve_previous_event_links_if_empty(
        &mut self,
        previous: &[TopologicalBehaviorEventLink],
    ) -> Result<(), ScaffoldContractError> {
        if self.event_links.is_empty() && !previous.is_empty() {
            self.event_links = previous
                .iter()
                .take(CA28_MAX_EVENT_LINKS)
                .cloned()
                .collect();
        }
        self.validate()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopologicalConceptOverlaySmokeSummary {
    pub snapshot: TopologicalConceptOverlaySnapshot,
    pub panel_text: String,
    pub status_text: String,
    pub topology_action_bypass_blocked: bool,
    pub direct_cognition_mutation_allowed: bool,
}

impl TopologicalConceptOverlaySmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        self.snapshot.validate()?;
        if self.snapshot.concept_count == 0
            || self.snapshot.edge_count == 0
            || self.snapshot.event_links.is_empty()
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA28 topology overlay must show concepts, edges, and behavior links",
            });
        }
        if !self.topology_action_bypass_blocked
            || self.direct_cognition_mutation_allowed
            || self.snapshot.can_emit_actions
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA28 topology overlay must remain read-only and action-bypass blocked",
            });
        }
        if !self.panel_text.contains("Concept Map (read-only)")
            || !self
                .panel_text
                .contains("Boundary: bias/context only; no actions")
            || self.panel_text.contains("Entity(")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA28 topology overlay text must be player-facing and stable-ID safe",
            });
        }
        if !self.status_text.contains("Concepts:") {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA28 status panel must expose a compact topology line",
            });
        }
        Ok(())
    }
}

pub fn run_topological_concept_overlay_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<TopologicalConceptOverlaySmokeSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    panel.apply_command(&mut live, RuntimeControlCommand::RunForTicks(4))?;
    let snapshot = panel.topology_overlay.clone();
    let panel_text = snapshot.panel_text();
    let status_text = panel.structured_status_panel_text_with_backend("GPU: GpuPlastic requested");
    let summary = TopologicalConceptOverlaySmokeSummary {
        snapshot,
        panel_text,
        status_text,
        topology_action_bypass_blocked: true,
        direct_cognition_mutation_allowed: panel.direct_cognition_mutation_allowed,
    };
    summary.validate()?;
    Ok(summary)
}

fn concept_node_row(
    concept: &alife_core::ConceptCell,
) -> Result<TopologicalConceptNodeRow, ScaffoldContractError> {
    concept.validate_contract()?;
    Ok(TopologicalConceptNodeRow {
        concept_id: concept.id,
        label: concept_label(concept)?,
        observation_count: concept.observation_count,
        salience: concept.salience.raw(),
        confidence: concept.confidence.raw(),
    })
}

fn edge_row(
    edge: &alife_core::CognitiveEdge,
) -> Result<TopologicalConceptEdgeRow, ScaffoldContractError> {
    edge.validate_contract()?;
    Ok(TopologicalConceptEdgeRow {
        edge_id: edge.id,
        from: edge.from,
        to: edge.to,
        relation: edge.relation,
        strength: edge.strength.raw(),
        evidence_count: edge.evidence_count,
    })
}

fn gap_row(
    gap: &alife_core::UnresolvedGap,
) -> Result<TopologicalConceptGapRow, ScaffoldContractError> {
    gap.validate_contract()?;
    Ok(TopologicalConceptGapRow {
        gap_id: gap.id,
        source_concepts: gap.source_concepts.clone(),
        status: gap.status,
        salience: gap.salience.raw(),
        curiosity_voltage: gap.curiosity_voltage.raw(),
        label: format!("{:?}", gap.contradiction_type),
    })
}

fn event_link_row(
    summary: &LiveBrainTickSummary,
) -> Result<TopologicalBehaviorEventLink, ScaffoldContractError> {
    if let Some(target) = summary.target_entity {
        target.validate()?;
    }
    let action = summary
        .selected_action_kind
        .map_or("None".to_string(), |action| format!("{action:?}"));
    let target = summary.target_entity.map_or_else(
        || "none".to_string(),
        |target| format!("stable:{}", target.raw()),
    );
    let sequence = summary
        .patch_sequence_id
        .map_or_else(|| "pending".to_string(), |sequence| sequence.to_string());
    let label = format!(
        "event tick={} seq={} action={} target={} topo+{}",
        summary.tick_after.raw(),
        sequence,
        action,
        target,
        summary.topology_updates
    );
    validate_display_line(&label)?;
    Ok(TopologicalBehaviorEventLink {
        tick: summary.tick_after.raw(),
        sequence_id: summary.patch_sequence_id,
        target_entity: summary.target_entity,
        action_kind: summary.selected_action_kind,
        topology_updates: summary.topology_updates,
        label,
    })
}

fn concept_label(concept: &alife_core::ConceptCell) -> Result<String, ScaffoldContractError> {
    let label = if let Some(object) = concept.bindings.objects.first() {
        format!("obj stable:{}", object.raw())
    } else if let Some(action) = concept.bindings.actions.first() {
        format!("action:{:?}", action.kind)
    } else if let Some(agent) = concept.bindings.agents.first() {
        format!("org:{}", agent.raw())
    } else if let Some(word) = concept.bindings.words.first() {
        format!("word:{word}")
    } else {
        "concept".to_string()
    };
    validate_display_line(&label)?;
    Ok(label)
}

fn validate_unit(value: f32) -> Result<(), ScaffoldContractError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn validate_display_line(line: &str) -> Result<(), ScaffoldContractError> {
    if line.is_empty() || line.len() > 160 || line.contains("Entity(") {
        Err(ScaffoldContractError::InvalidId)
    } else {
        Ok(())
    }
}
