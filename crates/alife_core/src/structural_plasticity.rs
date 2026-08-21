//! Bounded core-side structural plasticity.
//!
//! This module owns a small sparse adjacency overlay until the runtime and GPU
//! layers join it. Growth is driven by bounded local evidence, not by a scan
//! over absent neuron pairs. Accepted edges are consumed by `compute`, so the
//! slice exercises real later computation rather than a descriptor-only hook.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const MAX_CANDIDATES_PER_REGION: usize = 8;
pub const MAX_REGIONS_PER_STATE: usize = 16;
pub const MAX_EVIDENCE_PER_PHASE: usize = 256;
pub const MAX_ACCEPTED_PER_PHASE: usize = 4;
pub const MAX_STRUCTURAL_EDGES: usize = 64;
pub const MAX_ROUTE_INDEX: u16 = MAX_REGIONS_PER_STATE as u16;

const RECEIPT_FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
const RECEIPT_FNV_PRIME: u64 = 1_099_511_628_211;
const MAX_WEIGHT_SCORE: u32 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralPlasticityConfig {
    pub max_candidates_per_region: u16,
    pub max_regions: u16,
    pub max_accepted_per_phase: u16,
    pub max_structural_edges: u16,
    pub min_candidate_score: u32,
    pub prune_score_below: u32,
}

impl Default for StructuralPlasticityConfig {
    fn default() -> Self {
        Self {
            max_candidates_per_region: MAX_CANDIDATES_PER_REGION as u16,
            max_regions: MAX_REGIONS_PER_STATE as u16,
            max_accepted_per_phase: MAX_ACCEPTED_PER_PHASE as u16,
            max_structural_edges: MAX_STRUCTURAL_EDGES as u16,
            min_candidate_score: 2,
            prune_score_below: 1,
        }
    }
}

impl StructuralPlasticityConfig {
    fn validate(self) -> Result<(), StructuralPlasticityError> {
        if self.max_candidates_per_region == 0
            || usize::from(self.max_candidates_per_region) > MAX_CANDIDATES_PER_REGION
            || self.max_regions == 0
            || usize::from(self.max_regions) > MAX_REGIONS_PER_STATE
            || self.max_accepted_per_phase == 0
            || usize::from(self.max_accepted_per_phase) > MAX_ACCEPTED_PER_PHASE
            || self.max_structural_edges == 0
            || usize::from(self.max_structural_edges) > MAX_STRUCTURAL_EDGES
        {
            return Err(StructuralPlasticityError::InvalidConfig);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoactivationEvidence {
    pub region: u16,
    pub source: u32,
    pub target: u32,
    pub coactivation: u32,
    pub eligibility: u32,
    /// Bounded concept/gap relevance may score a local candidate. It never
    /// creates a connection without coactivation or eligibility evidence.
    pub concept_gap_support: u32,
}

/// The source of a bounded event.  Source identity is deliberately separate
/// from the candidate identity: a concept, replay event, and neural pair may
/// all support the same candidate without becoming that synapse's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StructuralSource {
    NeuralActivity,
    Eligibility,
    PredictionResidual,
    WorkingMemory,
    ConceptGap,
    RouteLocality,
    ReplayExploration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralEvidenceEvent {
    pub event_id: u64,
    pub tick: u64,
    pub region: u16,
    pub source: u32,
    pub target: u32,
    pub source_identity: u64,
    #[serde(default)]
    pub concept_hint_id: Option<u64>,
    pub neural_coactivity: u32,
    pub eligibility: u32,
    pub prediction_residual: u32,
    pub working_memory: u32,
    pub concept_gap_support: u32,
    pub route_locality: u32,
    pub exploration: u32,
    pub novelty: u32,
    pub outcome_utility: u32,
    pub redundancy: u32,
    pub maintenance_cost: u32,
    pub source_kind: StructuralSource,
}

impl StructuralEvidenceEvent {
    pub fn from_legacy(item: CoactivationEvidence, event_id: u64) -> Self {
        Self {
            event_id,
            tick: 0,
            region: item.region,
            source: item.source,
            target: item.target,
            source_identity: event_id,
            concept_hint_id: None,
            neural_coactivity: item.coactivation,
            eligibility: item.eligibility,
            prediction_residual: 0,
            working_memory: 0,
            concept_gap_support: item.concept_gap_support,
            route_locality: 0,
            exploration: 0,
            novelty: 0,
            outcome_utility: 0,
            redundancy: 0,
            maintenance_cost: 1,
            source_kind: if item.eligibility > 0 {
                StructuralSource::Eligibility
            } else {
                StructuralSource::NeuralActivity
            },
        }
    }

    fn validate(&self, neuron_count: u32, max_regions: u16) -> Result<(), StructuralPlasticityError> {
        if self.source >= neuron_count
            || self.target >= neuron_count
            || self.region >= max_regions
            || self.source == self.target
            || self.event_id == 0
            || self.maintenance_cost == 0
        {
            return Err(StructuralPlasticityError::NodeOutOfRange);
        }
        Ok(())
    }

    fn candidate_score(&self) -> u32 {
        self.neural_coactivity
            .saturating_add(self.eligibility)
            .saturating_add(self.prediction_residual)
            .saturating_add(self.working_memory)
            .saturating_add(self.concept_gap_support)
            .saturating_add(self.route_locality)
            .saturating_add(self.exploration)
            .saturating_add(self.novelty)
            .saturating_add(self.outcome_utility)
            .saturating_sub(self.redundancy)
            .saturating_sub(self.maintenance_cost)
    }

    fn has_neural_anchor(&self) -> bool {
        self.neural_coactivity > 0
            || self.eligibility > 0
            || self.prediction_residual > 0
            || self.working_memory > 0
            || self.route_locality > 0
            || self.exploration > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralDiscoveryReceipt {
    pub evidence_items: u16,
    pub candidate_comparisons: u32,
    pub candidates_kept: u16,
    #[serde(default)]
    pub events_retained: u16,
    #[serde(default)]
    pub maintenance_ops: u32,
    pub deterministic_digest: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralWorkReceipt {
    pub candidate_comparisons: u32,
    pub candidates_kept: u16,
    pub accepted_edges: u16,
    pub pruned_edges: u16,
    pub active_edges: u16,
    #[serde(default)]
    pub maintenance_ops: u32,
    #[serde(default)]
    pub ranking_ops: u32,
    #[serde(default)]
    pub recompaction_ops: u32,
    #[serde(default)]
    pub growth_ops: u32,
    #[serde(default)]
    pub pruning_ops: u32,
    pub deterministic_digest: u64,
}

impl StructuralWorkReceipt {
    pub const fn work_units(self) -> u64 {
        (self.candidate_comparisons as u64)
            .saturating_add(self.candidates_kept as u64)
            .saturating_add(self.maintenance_ops as u64)
            .saturating_add(self.ranking_ops as u64)
            .saturating_add(self.recompaction_ops as u64)
            .saturating_add(self.growth_ops as u64)
            .saturating_add(self.pruning_ops as u64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralPlasticityError {
    InvalidConfig,
    EvidenceBudgetExceeded,
    RegionBudgetExceeded,
    NodeOutOfRange,
    InvalidInputLength,
    NonFiniteValue,
    ConnectionNotFound,
    DuplicateConnection,
    StructuralBudgetExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct StructuralCandidate {
    candidate_id: u64,
    region: u16,
    source: u32,
    target: u32,
    source_identity: u64,
    concept_hint_id: Option<u64>,
    coactivation: u32,
    eligibility: u32,
    prediction_residual: u32,
    working_memory: u32,
    concept_gap_support: u32,
    route_locality: u32,
    exploration: u32,
    novelty: u32,
    outcome_utility: u32,
    redundancy: u32,
    maintenance_cost: u32,
    source_kind: StructuralSource,
    observations: u16,
    last_tick: u64,
    age: u32,
    score: u32,
}

impl StructuralCandidate {
    fn from_event(candidate_id: u64, event: StructuralEvidenceEvent) -> Self {
        Self {
            candidate_id,
            region: event.region,
            source: event.source,
            target: event.target,
            source_identity: event.source_identity,
            concept_hint_id: event.concept_hint_id,
            coactivation: event.neural_coactivity,
            eligibility: event.eligibility,
            prediction_residual: event.prediction_residual,
            working_memory: event.working_memory,
            concept_gap_support: event.concept_gap_support,
            route_locality: event.route_locality,
            exploration: event.exploration,
            novelty: event.novelty,
            outcome_utility: event.outcome_utility,
            redundancy: event.redundancy,
            maintenance_cost: event.maintenance_cost,
            source_kind: event.source_kind,
            observations: 1,
            last_tick: event.tick,
            age: 0,
            score: event.candidate_score(),
        }
    }

    fn merge_event(&mut self, event: StructuralEvidenceEvent) {
        self.source_identity = event.source_identity;
        self.concept_hint_id = event.concept_hint_id.or(self.concept_hint_id);
        self.coactivation = self.coactivation.saturating_add(event.neural_coactivity);
        self.eligibility = self.eligibility.saturating_add(event.eligibility);
        self.prediction_residual = self
            .prediction_residual
            .saturating_add(event.prediction_residual);
        self.working_memory = self.working_memory.saturating_add(event.working_memory);
        self.concept_gap_support = self
            .concept_gap_support
            .saturating_add(event.concept_gap_support);
        self.route_locality = self.route_locality.saturating_add(event.route_locality);
        self.exploration = self.exploration.saturating_add(event.exploration);
        self.novelty = self.novelty.saturating_add(event.novelty);
        self.outcome_utility = self.outcome_utility.saturating_add(event.outcome_utility);
        self.redundancy = self.redundancy.saturating_add(event.redundancy);
        self.maintenance_cost = self
            .maintenance_cost
            .saturating_add(event.maintenance_cost);
        self.source_kind = event.source_kind;
        self.observations = self.observations.saturating_add(1);
        self.last_tick = self.last_tick.max(event.tick);
        self.age = self.age.saturating_add(1);
        self.score = event.candidate_score_from(self);
    }

    fn utility(self) -> u32 {
        self.outcome_utility
            .saturating_add(self.prediction_residual)
            .saturating_add(self.working_memory)
            .saturating_add(self.novelty)
            .saturating_sub(self.redundancy)
    }
}

impl StructuralEvidenceEvent {
    fn candidate_score_from(self, candidate: &StructuralCandidate) -> u32 {
        candidate
            .coactivation
            .saturating_add(candidate.eligibility)
            .saturating_add(candidate.prediction_residual)
            .saturating_add(candidate.working_memory)
            .saturating_add(candidate.concept_gap_support)
            .saturating_add(candidate.route_locality)
            .saturating_add(candidate.exploration)
            .saturating_add(candidate.novelty)
            .saturating_add(candidate.outcome_utility)
            .saturating_sub(candidate.redundancy)
            .saturating_sub(candidate.maintenance_cost)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct SparseConnection {
    candidate_id: u64,
    route: u16,
    source: u32,
    target: u32,
    weight: f32,
    score: u32,
    eligibility: u32,
    structural_utility: u32,
    maintenance_cost: u32,
    observations: u16,
    last_tick: u64,
    age: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralPlasticityState {
    neuron_count: u32,
    config: StructuralPlasticityConfig,
    candidates: BTreeMap<u16, Vec<StructuralCandidate>>,
    connections: BTreeMap<u32, Vec<SparseConnection>>,
    last_candidate_comparisons: u32,
}

impl StructuralPlasticityState {
    pub fn new(
        neuron_count: u32,
        config: StructuralPlasticityConfig,
    ) -> Result<Self, StructuralPlasticityError> {
        if neuron_count == 0 {
            return Err(StructuralPlasticityError::NodeOutOfRange);
        }
        config.validate()?;
        Ok(Self {
            neuron_count,
            config,
            candidates: BTreeMap::new(),
            connections: BTreeMap::new(),
            last_candidate_comparisons: 0,
        })
    }

    pub fn discover_candidates(
        &mut self,
        evidence: &[CoactivationEvidence],
    ) -> Result<StructuralDiscoveryReceipt, StructuralPlasticityError> {
        let events = evidence
            .iter()
            .copied()
            .enumerate()
            .map(|(index, item)| {
                StructuralEvidenceEvent::from_legacy(item, event_identity(item, index as u64))
            })
            .collect::<Vec<_>>();
        self.discover_events(&events)
    }

    /// Retains only bounded event-nominated partners.  This is the structural
    /// discovery boundary: no absent source/target pair is ever generated here.
    pub fn discover_events(
        &mut self,
        evidence: &[StructuralEvidenceEvent],
    ) -> Result<StructuralDiscoveryReceipt, StructuralPlasticityError> {
        if evidence.len() > MAX_EVIDENCE_PER_PHASE {
            return Err(StructuralPlasticityError::EvidenceBudgetExceeded);
        }

        let incoming_regions: BTreeSet<u16> = evidence.iter().map(|item| item.region).collect();
        let new_regions = incoming_regions
            .iter()
            .filter(|region| !self.candidates.contains_key(region))
            .count();
        if self.candidates.len() + new_regions > usize::from(self.config.max_regions) {
            return Err(StructuralPlasticityError::RegionBudgetExceeded);
        }
        for item in evidence {
            item.validate(self.neuron_count, self.config.max_regions)?;
        }

        let mut ordered = evidence.to_vec();
        ordered.sort_unstable_by_key(|item| {
            (
                item.tick,
                item.event_id,
                item.region,
                item.source,
                item.target,
                item.source_kind,
            )
        });

        let mut comparisons = 0_u32;
        let mut maintenance_ops = 0_u32;
        let mut events_retained = 0_u16;
        for item in ordered {
            if !item.has_neural_anchor() || item.candidate_score() < self.config.min_candidate_score {
                continue;
            }
            let candidates = self.candidates.entry(item.region).or_default();
            comparisons = comparisons.saturating_add(candidates.len() as u32);
            let candidate_id = candidate_identity(item.region, item.source, item.target);
            if let Some(candidate) = candidates
                .iter_mut()
                .find(|candidate| candidate.candidate_id == candidate_id)
            {
                candidate.merge_event(item);
                maintenance_ops = maintenance_ops.saturating_add(1);
            } else {
                candidates.push(StructuralCandidate::from_event(candidate_id, item));
                maintenance_ops = maintenance_ops.saturating_add(1);
            }
            candidates.sort_unstable_by(candidate_order);
            candidates.truncate(usize::from(self.config.max_candidates_per_region));
            events_retained = events_retained.saturating_add(1);
        }

        self.last_candidate_comparisons = comparisons;
        Ok(StructuralDiscoveryReceipt {
            evidence_items: evidence.len() as u16,
            candidate_comparisons: comparisons,
            candidates_kept: self.candidate_count() as u16,
            events_retained,
            maintenance_ops,
            deterministic_digest: self.deterministic_digest(),
        })
    }

    pub fn apply_structural_phase(
        &mut self,
    ) -> Result<StructuralWorkReceipt, StructuralPlasticityError> {
        let mut candidates: Vec<StructuralCandidate> = self
            .candidates
            .values()
            .flat_map(|items| items.iter().copied())
            .collect();
        candidates.sort_unstable_by(candidate_order);

        let candidate_comparisons = self.last_candidate_comparisons;
        let mut ranking_ops = 0_u32;
        let mut maintenance_ops = 0_u32;
        let mut accepted = 0_usize;
        let mut replaced = 0_usize;
        for candidate in candidates {
            if accepted >= usize::from(self.config.max_accepted_per_phase) {
                break;
            }
            ranking_ops = ranking_ops.saturating_add(1);
            if self.has_connection(candidate.source, candidate.target) {
                self.remove_candidate(candidate);
                maintenance_ops = maintenance_ops.saturating_add(1);
                continue;
            }
            let weight = score_to_weight(candidate.score);
            if weight <= 0.0 {
                continue;
            }
            let replacement = if self.connection_count()
                >= usize::from(self.config.max_structural_edges)
            {
                let Some(weakest) = self.weakest_connection() else {
                    break;
                };
                if candidate.score <= weakest.score.saturating_add(1) {
                    continue;
                }
                Some((weakest.target, weakest.source))
            } else {
                None
            };
            if let Some((target, source)) = replacement {
                self.remove_connection(source, target);
                replaced = replaced.saturating_add(1);
            }
            self.insert_connection(SparseConnection {
                candidate_id: candidate.candidate_id,
                route: candidate.region,
                source: candidate.source,
                target: candidate.target,
                weight,
                score: candidate.score,
                eligibility: candidate.eligibility,
                structural_utility: candidate.utility(),
                maintenance_cost: candidate.maintenance_cost,
                observations: candidate.observations,
                last_tick: candidate.last_tick,
                age: candidate.age.saturating_add(1),
            })?;
            self.remove_candidate(candidate);
            accepted += 1;
        }

        let pruned = self.prune_weak_connections_internal().saturating_add(replaced);
        let recompaction_ops = self.connection_count() as u32;
        self.last_candidate_comparisons = 0;
        Ok(StructuralWorkReceipt {
            candidate_comparisons,
            candidates_kept: self.candidate_count() as u16,
            accepted_edges: accepted as u16,
            pruned_edges: pruned as u16,
            active_edges: self.connection_count() as u16,
            maintenance_ops,
            ranking_ops,
            recompaction_ops,
            growth_ops: accepted as u32,
            pruning_ops: pruned as u32,
            deterministic_digest: self.deterministic_digest(),
        })
    }

    pub fn record_edge_support(
        &mut self,
        source: u32,
        target: u32,
        support: u32,
    ) -> Result<(), StructuralPlasticityError> {
        if source >= self.neuron_count || target >= self.neuron_count {
            return Err(StructuralPlasticityError::NodeOutOfRange);
        }
        let edge = self
            .connections
            .get_mut(&target)
            .and_then(|edges| edges.iter_mut().find(|edge| edge.source == source))
            .ok_or(StructuralPlasticityError::ConnectionNotFound)?;
        edge.score = support;
        edge.structural_utility = support;
        edge.eligibility = support;
        edge.weight = score_to_weight(support);
        Ok(())
    }

    pub fn compute(&self, input: &[f32]) -> Result<Vec<f32>, StructuralPlasticityError> {
        if input.len() != self.neuron_count as usize {
            return Err(StructuralPlasticityError::InvalidInputLength);
        }
        if input.iter().any(|value| !value.is_finite()) {
            return Err(StructuralPlasticityError::NonFiniteValue);
        }

        let mut output = vec![0.0; input.len()];
        for (target, edges) in &self.connections {
            let mut accumulator = 0.0_f32;
            for edge in edges {
                accumulator += input[edge.source as usize] * edge.weight;
            }
            if !accumulator.is_finite() {
                return Err(StructuralPlasticityError::NonFiniteValue);
            }
            output[*target as usize] = accumulator;
        }
        Ok(output)
    }

    pub fn candidate_count(&self) -> usize {
        self.candidates.values().map(Vec::len).sum()
    }

    pub fn connection_count(&self) -> usize {
        self.connections.values().map(Vec::len).sum()
    }

    fn has_connection(&self, source: u32, target: u32) -> bool {
        self.connections
            .get(&target)
            .is_some_and(|edges| edges.iter().any(|edge| edge.source == source))
    }

    fn weakest_connection(&self) -> Option<SparseConnection> {
        self.connections
            .values()
            .flat_map(|edges| edges.iter().copied())
            .min_by_key(|edge| {
                (
                    edge.score,
                    edge.structural_utility,
                    edge.eligibility,
                    edge.age,
                    edge.target,
                    edge.source,
                )
            })
    }

    fn remove_connection(&mut self, source: u32, target: u32) {
        let mut empty = false;
        if let Some(edges) = self.connections.get_mut(&target) {
            edges.retain(|edge| edge.source != source);
            empty = edges.is_empty();
        }
        if empty {
            self.connections.remove(&target);
        }
    }

    fn insert_connection(
        &mut self,
        connection: SparseConnection,
    ) -> Result<(), StructuralPlasticityError> {
        let edges = self.connections.entry(connection.target).or_default();
        if edges.iter().any(|edge| edge.source == connection.source) {
            return Err(StructuralPlasticityError::DuplicateConnection);
        }
        edges.push(connection);
        edges.sort_unstable_by_key(|edge| edge.source);
        Ok(())
    }

    fn remove_candidate(&mut self, removed: StructuralCandidate) {
        let mut empty = false;
        if let Some(candidates) = self.candidates.get_mut(&removed.region) {
            candidates.retain(|candidate| {
                candidate.source != removed.source || candidate.target != removed.target
            });
            empty = candidates.is_empty();
        }
        if empty {
            self.candidates.remove(&removed.region);
        }
    }

    fn prune_weak_connections_internal(&mut self) -> usize {
        let threshold = self.config.prune_score_below;
        let mut pruned = 0_usize;
        let mut empty_targets = Vec::new();
        for (target, edges) in &mut self.connections {
            let before = edges.len();
            edges.retain(|edge| edge.score >= threshold);
            pruned += before - edges.len();
            if edges.is_empty() {
                empty_targets.push(*target);
            }
        }
        for target in empty_targets {
            self.connections.remove(&target);
        }
        pruned
    }

    fn deterministic_digest(&self) -> u64 {
        let mut digest = RECEIPT_FNV_OFFSET;
        digest = mix_digest(digest, self.neuron_count as u64);
        for (region, candidates) in &self.candidates {
            digest = mix_digest(digest, u64::from(*region));
            for candidate in candidates {
                digest = mix_digest(digest, candidate.candidate_id);
                digest = mix_digest(digest, candidate.source as u64);
                digest = mix_digest(digest, candidate.target as u64);
                digest = mix_digest(digest, candidate.source_identity);
                digest = mix_digest(digest, candidate.concept_hint_id.unwrap_or(0));
                digest = mix_digest(digest, u64::from(candidate.score));
                digest = mix_digest(digest, u64::from(candidate.observations));
                digest = mix_digest(digest, candidate.last_tick);
                digest = mix_digest(digest, u64::from(candidate.age));
            }
        }
        for (target, edges) in &self.connections {
            digest = mix_digest(digest, *target as u64);
            for edge in edges {
                digest = mix_digest(digest, edge.candidate_id);
                digest = mix_digest(digest, u64::from(edge.route));
                digest = mix_digest(digest, edge.source as u64);
                digest = mix_digest(digest, u64::from(edge.score));
                digest = mix_digest(digest, u64::from(edge.eligibility));
                digest = mix_digest(digest, u64::from(edge.structural_utility));
                digest = mix_digest(digest, u64::from(edge.observations));
                digest = mix_digest(digest, edge.last_tick);
                digest = mix_digest(digest, u64::from(edge.age));
            }
        }
        digest
    }
}

fn candidate_order(left: &StructuralCandidate, right: &StructuralCandidate) -> Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| left.region.cmp(&right.region))
        .then_with(|| left.source.cmp(&right.source))
        .then_with(|| left.target.cmp(&right.target))
        .then_with(|| left.candidate_id.cmp(&right.candidate_id))
}

fn score_to_weight(score: u32) -> f32 {
    score.min(MAX_WEIGHT_SCORE) as f32 / MAX_WEIGHT_SCORE as f32
}

fn mix_digest(mut digest: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(RECEIPT_FNV_PRIME);
    }
    digest
}

fn candidate_identity(region: u16, source: u32, target: u32) -> u64 {
    let mut digest = RECEIPT_FNV_OFFSET;
    digest = mix_digest(digest, u64::from(region));
    digest = mix_digest(digest, u64::from(source));
    mix_digest(digest, u64::from(target))
}

fn event_identity(item: CoactivationEvidence, index: u64) -> u64 {
    let mut digest = candidate_identity(item.region, item.source, item.target);
    digest = mix_digest(digest, u64::from(item.coactivation));
    digest = mix_digest(digest, u64::from(item.eligibility));
    let digest = mix_digest(digest, u64::from(item.concept_gap_support));
    let digest = mix_digest(digest, index);
    if digest == 0 { 1 } else { digest }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_growth_changes_later_compute_and_pruning_removes_weak_edge() {
        let config = StructuralPlasticityConfig {
            max_candidates_per_region: 8,
            max_regions: 1,
            max_accepted_per_phase: 1,
            max_structural_edges: 1,
            min_candidate_score: 2,
            prune_score_below: 10,
        };
        let mut plasticity = StructuralPlasticityState::new(2048, config).unwrap();
        let mut evidence = vec![CoactivationEvidence {
            region: 0,
            source: 1,
            target: 2,
            coactivation: 100,
            eligibility: 20,
            concept_gap_support: 0,
        }];
        evidence.extend((0..63).map(|index| CoactivationEvidence {
            region: 0,
            source: 16 + index,
            target: 512 + index,
            coactivation: 1,
            eligibility: 1,
            concept_gap_support: 0,
        }));

        let discovery = plasticity.discover_candidates(&evidence).unwrap();
        assert_eq!(discovery.candidates_kept, 8);
        assert!(discovery.candidate_comparisons <= 64 * 8);
        assert!(discovery.candidate_comparisons < 2048 * 2048);

        let mut input = vec![0.0; 2048];
        input[1] = 1.0;
        let before_growth = plasticity.compute(&input).unwrap();

        let growth = plasticity.apply_structural_phase().unwrap();
        assert_eq!(growth.accepted_edges, 1);
        assert_eq!(plasticity.connection_count(), 1);
        let after_growth = plasticity.compute(&input).unwrap();
        assert_ne!(before_growth[2], after_growth[2]);

        plasticity.record_edge_support(1, 2, 1).unwrap();
        let pruning = plasticity.apply_structural_phase().unwrap();
        assert_eq!(pruning.pruned_edges, 1);
        assert_eq!(plasticity.connection_count(), 0);
        assert_eq!(plasticity.compute(&input).unwrap()[2], 0.0);
    }
}
