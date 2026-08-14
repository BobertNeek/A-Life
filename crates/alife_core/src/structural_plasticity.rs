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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralDiscoveryReceipt {
    pub evidence_items: u16,
    pub candidate_comparisons: u32,
    pub candidates_kept: u16,
    pub deterministic_digest: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralWorkReceipt {
    pub candidate_comparisons: u32,
    pub candidates_kept: u16,
    pub accepted_edges: u16,
    pub pruned_edges: u16,
    pub active_edges: u16,
    pub deterministic_digest: u64,
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
    region: u16,
    source: u32,
    target: u32,
    coactivation: u32,
    eligibility: u32,
    concept_gap_support: u32,
    age: u32,
    score: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct SparseConnection {
    source: u32,
    target: u32,
    weight: f32,
    score: u32,
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
            if item.source >= self.neuron_count || item.target >= self.neuron_count {
                return Err(StructuralPlasticityError::NodeOutOfRange);
            }
        }

        let mut ordered = evidence.to_vec();
        ordered.sort_unstable_by_key(|item| {
            (
                item.region,
                item.source,
                item.target,
                item.coactivation,
                item.eligibility,
                item.concept_gap_support,
            )
        });

        let mut comparisons = 0_u32;
        for item in ordered {
            if item.source == item.target
                || (item.coactivation == 0 && item.eligibility == 0)
            {
                continue;
            }
            let score = item
                .coactivation
                .saturating_add(item.eligibility)
                .saturating_add(item.concept_gap_support);
            if score < self.config.min_candidate_score {
                continue;
            }

            let candidates = self.candidates.entry(item.region).or_default();
            comparisons = comparisons.saturating_add(candidates.len() as u32);
            if let Some(candidate) = candidates
                .iter_mut()
                .find(|candidate| candidate.source == item.source && candidate.target == item.target)
            {
                candidate.coactivation = candidate.coactivation.saturating_add(item.coactivation);
                candidate.eligibility = candidate.eligibility.saturating_add(item.eligibility);
                candidate.concept_gap_support = candidate
                    .concept_gap_support
                    .saturating_add(item.concept_gap_support);
                candidate.age = candidate.age.saturating_add(1);
                candidate.score = candidate_score(*candidate);
            } else {
                candidates.push(StructuralCandidate {
                    region: item.region,
                    source: item.source,
                    target: item.target,
                    coactivation: item.coactivation,
                    eligibility: item.eligibility,
                    concept_gap_support: item.concept_gap_support,
                    age: 0,
                    score,
                });
            }
            candidates.sort_unstable_by(candidate_order);
            candidates.truncate(usize::from(self.config.max_candidates_per_region));
        }

        self.last_candidate_comparisons = comparisons;
        Ok(StructuralDiscoveryReceipt {
            evidence_items: evidence.len() as u16,
            candidate_comparisons: comparisons,
            candidates_kept: self.candidate_count() as u16,
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
        let mut accepted = 0_usize;
        for candidate in candidates {
            if accepted >= usize::from(self.config.max_accepted_per_phase)
                || self.connection_count() >= usize::from(self.config.max_structural_edges)
            {
                break;
            }
            if self.has_connection(candidate.source, candidate.target) {
                self.remove_candidate(candidate);
                continue;
            }
            let weight = score_to_weight(candidate.score);
            if weight <= 0.0 {
                continue;
            }
            self.insert_connection(SparseConnection {
                source: candidate.source,
                target: candidate.target,
                weight,
                score: candidate.score,
                age: candidate.age,
            })?;
            self.remove_candidate(candidate);
            accepted += 1;
        }

        let pruned = self.prune_weak_connections_internal();
        self.last_candidate_comparisons = 0;
        Ok(StructuralWorkReceipt {
            candidate_comparisons,
            candidates_kept: self.candidate_count() as u16,
            accepted_edges: accepted as u16,
            pruned_edges: pruned as u16,
            active_edges: self.connection_count() as u16,
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
                digest = mix_digest(digest, candidate.source as u64);
                digest = mix_digest(digest, candidate.target as u64);
                digest = mix_digest(digest, u64::from(candidate.score));
                digest = mix_digest(digest, u64::from(candidate.age));
            }
        }
        for (target, edges) in &self.connections {
            digest = mix_digest(digest, *target as u64);
            for edge in edges {
                digest = mix_digest(digest, edge.source as u64);
                digest = mix_digest(digest, u64::from(edge.score));
                digest = mix_digest(digest, u64::from(edge.age));
            }
        }
        digest
    }
}

fn candidate_score(candidate: StructuralCandidate) -> u32 {
    candidate
        .coactivation
        .saturating_add(candidate.eligibility)
        .saturating_add(candidate.concept_gap_support)
}

fn candidate_order(left: &StructuralCandidate, right: &StructuralCandidate) -> Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| left.region.cmp(&right.region))
        .then_with(|| left.source.cmp(&right.source))
        .then_with(|| left.target.cmp(&right.target))
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
