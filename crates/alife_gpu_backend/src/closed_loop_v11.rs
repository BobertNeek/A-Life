//! Pass 1 backend join for bounded dendritic and structural computation.
//!
//! The closed-loop GPU path remains authoritative for normal neural execution.
//! This state owns the bounded descriptors and receipts compiled into the
//! production WGSL recurrent dispatch.

use alife_core::cognitive_work::CognitiveWorkCounters;
use alife_core::{
    apply_dendritic_conjunctions, BrainCapacityClass, BrainPhenotype, CanonicalDigestBuilder,
    CoactivationEvidence, CognitiveWorkReceipt, PhenotypeHash,
    DendriticBranch, DendriticBranchSet, DendriticInputRef, DendriticWorkReceipt,
    ScaffoldContractError, StructuralPlasticityConfig, StructuralPlasticityState,
    StructuralWorkReceipt, MAX_ACCEPTED_PER_PHASE, MAX_CANDIDATES_PER_REGION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const GPU_V11_CAUSAL_STATE_SCHEMA_VERSION: u16 = 1;
pub const GPU_LIVE_TOPOLOGY_CHECKPOINT_SCHEMA_VERSION: u16 = 1;

const GPU_LIVE_TOPOLOGY_DIGEST_DOMAIN: &[u8] = b"alife.gpu.live-topology.v1";

/// Backend-owned semantic projection of the exact fixed-slot execution plan.
/// Absolute arena offsets are excluded so the checkpoint remains portable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuLiveTopologyCheckpointV1 {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub neuron_count: u32,
    pub total_synapse_count: u32,
    pub recurrent_synapse_count: u32,
    pub decoder_synapse_count: u32,
    pub target_offsets: Vec<u32>,
    pub source_indices: Vec<u32>,
    pub route_indices: Vec<u32>,
    pub genetic_weight_bits: Vec<u32>,
    pub alpha_bits: Vec<u32>,
    pub synapse_learning_metadata_words: Vec<u32>,
    pub decoder_eligibility_metadata_words: Vec<u32>,
    pub decoder_synapse_starts: Vec<u32>,
    pub decoder_weight_global_synapse_ids: Vec<u32>,
    pub memory_weight_indices: Vec<u32>,
    pub v11_checkpoint: GpuV11Checkpoint,
    pub canonical_digest: [u64; 4],
}

impl GpuLiveTopologyCheckpointV1 {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(GPU_LIVE_TOPOLOGY_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        for word in self.phenotype_hash.0 {
            digest.write_u64(word);
        }
        digest.write_u32(self.neuron_count);
        digest.write_u32(self.total_synapse_count);
        digest.write_u32(self.recurrent_synapse_count);
        digest.write_u32(self.decoder_synapse_count);
        for values in [
            &self.target_offsets,
            &self.source_indices,
            &self.route_indices,
            &self.genetic_weight_bits,
            &self.alpha_bits,
            &self.synapse_learning_metadata_words,
            &self.decoder_eligibility_metadata_words,
            &self.decoder_synapse_starts,
            &self.decoder_weight_global_synapse_ids,
            &self.memory_weight_indices,
        ] {
            digest.write_sequence_len(values.len());
            for value in values {
                digest.write_u32(*value);
            }
        }
        Ok(digest.finish256())
    }

    pub fn validate_for_capacity(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError> {
        let recurrent = usize::try_from(self.recurrent_synapse_count)
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let total = usize::try_from(self.total_synapse_count)
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let decoder = usize::try_from(self.decoder_synapse_count)
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let neuron_count = usize::try_from(self.neuron_count)
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        GpuV11CausalState::restore(self.v11_checkpoint.clone())?;
        if self.schema_version != GPU_LIVE_TOPOLOGY_CHECKPOINT_SCHEMA_VERSION
            || self.phenotype_hash == PhenotypeHash([0; 4])
            || self.neuron_count == 0
            || self.neuron_count > capacity.execution().max_neurons()
            || self.total_synapse_count > capacity.execution().max_total_synapses()
            || self.recurrent_synapse_count > capacity.execution().max_recurrent_synapses()
            || self.recurrent_synapse_count.checked_add(self.decoder_synapse_count)
                != Some(self.total_synapse_count)
            || self.target_offsets.len() != neuron_count.saturating_add(1)
            || self.target_offsets.first().copied() != Some(0)
            || self.target_offsets.last().copied() != Some(self.recurrent_synapse_count)
            || self.target_offsets.windows(2).any(|pair| pair[0] > pair[1])
            || self.source_indices.len() != recurrent
            || self.route_indices.len() != recurrent
            || self.source_indices.iter().any(|source| *source >= self.neuron_count)
            || self.genetic_weight_bits.len() != total
            || self.alpha_bits.len() != total
            || self
                .genetic_weight_bits
                .iter()
                .chain(&self.alpha_bits)
                .any(|bits| !f32::from_bits(*bits).is_finite())
            || self.decoder_eligibility_metadata_words.is_empty() != (decoder == 0)
            || self.v11_checkpoint.neuron_count != self.neuron_count
            || self.v11_checkpoint.pending_lifetime_synapse.is_some()
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpuV11SparseEdge {
    pub source: u32,
    pub target: u32,
    #[serde(default)]
    pub route: u32,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddLifetimeSynapse {
    pub source: u32,
    pub target: u32,
    pub route: u32,
    pub initial_weight: f32,
    pub evidence: CoactivationEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuV11SparseSpan {
    pub target: u32,
    pub edges: Vec<GpuV11SparseEdge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuV11WorkReceipt {
    pub dendritic: DendriticWorkReceipt,
    pub structural: StructuralWorkReceipt,
    pub cognitive: CognitiveWorkReceipt,
}

#[cfg(feature = "gpu-tests")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuV11MutableStateProbe {
    pub lifetime_weight_banks: [u32; 2],
    pub fast_weight_banks: [u32; 2],
    pub decoder_eligibility_banks: [u32; 2],
    pub activation_sides: [u32; 2],
}

impl Default for GpuV11WorkReceipt {
    fn default() -> Self {
        Self {
            dendritic: DendriticWorkReceipt::default(),
            structural: StructuralWorkReceipt {
                candidate_comparisons: 0,
                candidates_kept: 0,
                accepted_edges: 0,
                pruned_edges: 0,
                active_edges: 0,
                deterministic_digest: 0,
            },
            cognitive: CognitiveWorkReceipt::zero(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuV11Checkpoint {
    pub schema_version: u16,
    pub neuron_count: u32,
    pub dendritic_branches: DendriticBranchSet,
    pub structural: StructuralPlasticityState,
    pub sparse_spans: Vec<GpuV11SparseSpan>,
    #[serde(default)]
    pub pending_lifetime_synapse: Option<AddLifetimeSynapse>,
    pub work: GpuV11WorkReceipt,
}

impl GpuV11Checkpoint {
    pub fn canonical_for_phenotype(
        phenotype: &BrainPhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        Ok(GpuV11CausalState::for_phenotype(phenotype)?.checkpoint())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuV11CausalState {
    neuron_count: u32,
    dendritic_branches: DendriticBranchSet,
    structural: StructuralPlasticityState,
    sparse_spans: Vec<GpuV11SparseSpan>,
    #[serde(default)]
    pending_lifetime_synapse: Option<AddLifetimeSynapse>,
    last_work: GpuV11WorkReceipt,
}

impl GpuV11CausalState {
    pub fn for_phenotype(phenotype: &BrainPhenotype) -> Result<Self, ScaffoldContractError> {
        let neuron_count = phenotype.neuron_count();
        let motor_start = phenotype.candidate_decoder().motor_start();
        let motor_width = phenotype.candidate_decoder().motor_width();
        let branch_count = usize::from(
            phenotype
                .cognitive_architecture_plan()
                .dendritic_branch_capacity(),
        )
        .max(1)
        .min(usize::from(motor_width))
        .min(64);
        let mut branches = Vec::with_capacity(branch_count);
        for index in 0..branch_count {
            let target = motor_start + index as u32;
            if target >= neuron_count {
                return Err(ScaffoldContractError::GpuLayoutMismatch);
            }
            let second_source = if target + 1 < neuron_count {
                target + 1
            } else {
                target.saturating_sub(1)
            };
            branches.push(DendriticBranch::new(
                target,
                -1.0,
                6.0,
                vec![
                    DendriticInputRef::new(target, 1.0)?,
                    DendriticInputRef::new(second_source, 1.0)?,
                ],
            )?);
        }
        let architecture = phenotype.cognitive_architecture_plan();
        let structural_edit_budget = u16::from(architecture.structural_edit_budget().max(1));
        let structural_config = StructuralPlasticityConfig {
            max_candidates_per_region: architecture
                .structural_candidate_budget()
                .max(1)
                .min(8),
            max_regions: 1,
            max_accepted_per_phase: structural_edit_budget.min(4),
            max_structural_edges: structural_edit_budget.saturating_mul(4).clamp(1, 64),
            min_candidate_score: 2,
            prune_score_below: 1,
        };
        Self::new(
            neuron_count,
            DendriticBranchSet::new(branches)?,
            structural_config,
        )
    }

    pub fn new(
        neuron_count: u32,
        dendritic_branches: DendriticBranchSet,
        structural_config: StructuralPlasticityConfig,
    ) -> Result<Self, ScaffoldContractError> {
        if neuron_count == 0 {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let structural = StructuralPlasticityState::new(neuron_count, structural_config)
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        Ok(Self {
            neuron_count,
            dendritic_branches,
            structural,
            sparse_spans: Vec::new(),
            pending_lifetime_synapse: None,
            last_work: GpuV11WorkReceipt::default(),
        })
    }

    pub fn new_for_phenotype(
        neuron_count: u32,
        dendritic_branches: DendriticBranchSet,
        phenotype: &BrainPhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        let plan = phenotype.cognitive_architecture();
        if phenotype.neuron_count() != neuron_count
            || dendritic_branches.branches().len()
                > usize::from(plan.dendritic_branch_capacity())
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let structural_config = StructuralPlasticityConfig {
            max_candidates_per_region: plan
                .structural_candidate_budget()
                .min(MAX_CANDIDATES_PER_REGION as u16),
            max_accepted_per_phase: u16::from(plan.structural_edit_budget())
                .min(MAX_ACCEPTED_PER_PHASE as u16),
            ..StructuralPlasticityConfig::default()
        };
        Self::new(neuron_count, dendritic_branches, structural_config)
    }

    pub fn dendritic_branches(&self) -> &DendriticBranchSet {
        &self.dendritic_branches
    }

    pub fn sparse_spans(&self) -> &[GpuV11SparseSpan] {
        &self.sparse_spans
    }

    pub(crate) fn pending_lifetime_synapse(&self) -> Option<AddLifetimeSynapse> {
        self.pending_lifetime_synapse.clone()
    }

    pub(crate) fn clear_pending_lifetime_synapse(
        &mut self,
        expected: &AddLifetimeSynapse,
    ) -> Result<(), ScaffoldContractError> {
        if self.pending_lifetime_synapse.as_ref() != Some(expected) {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        self.pending_lifetime_synapse = None;
        Ok(())
    }

    pub const fn last_work(&self) -> GpuV11WorkReceipt {
        self.last_work
    }

    pub(crate) fn record_gpu_recurrent_work(&mut self, work: GpuV11WorkReceipt) {
        self.last_work = work;
    }

    pub(crate) fn gpu_recurrent_work_receipt(
        &self,
        branches_evaluated: u32,
        inputs_evaluated: u32,
        gated_branches: u32,
        structural_edges_evaluated: u32,
    ) -> Result<GpuV11WorkReceipt, ScaffoldContractError> {
        if gated_branches > branches_evaluated {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let mut work = self.last_work;
        work.dendritic = DendriticWorkReceipt {
            branches_evaluated,
            inputs_evaluated,
            gated_branches,
            work_units: branches_evaluated.saturating_add(inputs_evaluated),
        };
        work.cognitive =
            self.make_cognitive_receipt_for(work.dendritic.work_units, structural_edges_evaluated)?;
        Ok(work)
    }

    pub fn set_dendritic_branches(
        &mut self,
        branches: DendriticBranchSet,
    ) -> Result<(), ScaffoldContractError> {
        let mut accumulators = vec![0.0; self.neuron_count as usize];
        apply_dendritic_conjunctions(
            &vec![0.0; self.neuron_count as usize],
            &mut accumulators,
            &branches,
        )
        .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        self.dendritic_branches = branches;
        Ok(())
    }

    #[cfg(test)]
    pub fn recurrent_step<F>(
        &mut self,
        activations: &[f32],
        base_accumulators: &[f32],
        final_activation: F,
    ) -> Result<Vec<f32>, ScaffoldContractError>
    where
        F: Fn(f32) -> f32,
    {
        if activations.len() != self.neuron_count as usize
            || base_accumulators.len() != self.neuron_count as usize
            || activations
                .iter()
                .chain(base_accumulators.iter())
                .any(|value| !value.is_finite())
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let mut accumulators = base_accumulators.to_vec();
        let dendritic =
            apply_dendritic_conjunctions(activations, &mut accumulators, &self.dendritic_branches)
                .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        for span in &self.sparse_spans {
            let target = span.target as usize;
            for edge in &span.edges {
                accumulators[target] += activations[edge.source as usize] * edge.weight;
            }
        }
        if accumulators.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        let output = accumulators
            .into_iter()
            .map(final_activation)
            .collect::<Vec<_>>();
        if output.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        self.last_work.dendritic = dendritic;
        self.last_work.cognitive = self.make_cognitive_receipt()?;
        Ok(output)
    }

    /// Discovers and accepts bounded core candidates, then atomically replaces
    /// only the sparse target spans touched by the bounded evidence set.
    pub fn apply_structural_phase(
        &mut self,
        evidence: &[CoactivationEvidence],
    ) -> Result<GpuV11WorkReceipt, ScaffoldContractError> {
        if self.pending_lifetime_synapse.is_some()
            || evidence
                .iter()
                .any(|item| item.source >= self.neuron_count || item.target >= self.neuron_count)
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let mut next = self.clone();
        let _discovery = next
            .structural
            .discover_candidates(evidence)
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let structural = next
            .structural
            .apply_structural_phase()
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;

        let mut indexed_pairs = BTreeMap::<u32, BTreeMap<u32, u32>>::new();
        for span in &self.sparse_spans {
            for edge in &span.edges {
                indexed_pairs
                    .entry(edge.source)
                    .or_default()
                    .insert(edge.target, edge.route);
            }
        }
        for item in evidence {
            indexed_pairs
                .entry(item.source)
                .or_default()
                .entry(item.target)
                .or_insert(u32::from(item.region));
        }

        let mut rebuilt_by_target = BTreeMap::<u32, Vec<GpuV11SparseEdge>>::new();
        for (source, targets) in indexed_pairs {
            let mut input = vec![0.0; self.neuron_count as usize];
            input[source as usize] = 1.0;
            let output = next
                .structural
                .compute(&input)
                .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
            for (target, route) in targets {
                let weight = output[target as usize];
                if weight == 0.0 {
                    continue;
                }
                rebuilt_by_target
                    .entry(target)
                    .or_default()
                    .push(GpuV11SparseEdge {
                        source,
                        target,
                        route,
                        weight,
                    });
            }
        }
        let mut rebuilt = rebuilt_by_target
            .into_iter()
            .map(|(target, edges)| GpuV11SparseSpan { target, edges })
            .collect::<Vec<_>>();
        for span in &mut rebuilt {
            span.edges.sort_unstable_by_key(|edge| edge.source);
        }
        if structural.accepted_edges > 0 {
            let accepted = evidence.iter().find_map(|item| {
                let already_present = self.sparse_spans.iter().any(|span| {
                    span.target == item.target
                        && span
                            .edges
                            .iter()
                            .any(|edge| edge.source == item.source && edge.target == item.target)
                });
                if already_present {
                    return None;
                }
                rebuilt
                    .iter()
                    .find(|span| span.target == item.target)
                    .and_then(|span| {
                        span.edges
                            .iter()
                            .find(|edge| edge.source == item.source && edge.target == item.target)
                    })
                    .filter(|edge| edge.weight.is_finite() && edge.weight != 0.0)
                    .map(|edge| AddLifetimeSynapse {
                        source: item.source,
                        target: item.target,
                        route: edge.route,
                        initial_weight: edge.weight,
                        evidence: item.clone(),
                    })
            });
            next.pending_lifetime_synapse = accepted;
        }
        next.sparse_spans = rebuilt;
        next.last_work.structural = structural;
        next.last_work.cognitive = next.make_cognitive_receipt()?;
        *self = next;
        Ok(self.last_work)
    }

    pub fn checkpoint(&self) -> GpuV11Checkpoint {
        GpuV11Checkpoint {
            schema_version: GPU_V11_CAUSAL_STATE_SCHEMA_VERSION,
            neuron_count: self.neuron_count,
            dendritic_branches: self.dendritic_branches.clone(),
            structural: self.structural.clone(),
            sparse_spans: self.sparse_spans.clone(),
            pending_lifetime_synapse: self.pending_lifetime_synapse.clone(),
            work: self.last_work,
        }
    }

    pub fn restore(checkpoint: GpuV11Checkpoint) -> Result<Self, ScaffoldContractError> {
        if checkpoint.schema_version != GPU_V11_CAUSAL_STATE_SCHEMA_VERSION
            || checkpoint.neuron_count == 0
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let state = Self {
            neuron_count: checkpoint.neuron_count,
            dendritic_branches: checkpoint.dendritic_branches,
            structural: checkpoint.structural,
            sparse_spans: checkpoint.sparse_spans,
            pending_lifetime_synapse: checkpoint.pending_lifetime_synapse,
            last_work: checkpoint.work,
        };
        if state.sparse_spans.iter().any(|span| {
            span.target >= state.neuron_count
                || span.edges.iter().any(|edge| {
                    edge.source >= state.neuron_count
                        || edge.target != span.target
                        || edge.route > u16::MAX as u32
                        || !edge.weight.is_finite()
                })
        }) {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        if let Some(pending) = &state.pending_lifetime_synapse {
            let present = state.sparse_spans.iter().any(|span| {
                span.target == pending.target
                    && span.edges.iter().any(|edge| {
                        edge.source == pending.source
                            && edge.target == pending.target
                            && edge.weight == pending.initial_weight
                    })
            });
            if pending.source >= state.neuron_count
                || pending.target >= state.neuron_count
                || !pending.initial_weight.is_finite()
                || pending.evidence.source != pending.source
                || pending.evidence.target != pending.target
                || pending.route != pending.evidence.region as u32
                || !present
            {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
        }
        Ok(state)
    }

    fn make_cognitive_receipt(&self) -> Result<CognitiveWorkReceipt, ScaffoldContractError> {
        let structural = self.last_work.structural;
        let structural_work_units = u64::from(structural.candidate_comparisons)
            .saturating_add(u64::from(structural.candidates_kept))
            .saturating_add(u64::from(structural.accepted_edges))
            .saturating_add(u64::from(structural.pruned_edges))
            .saturating_add(u64::from(structural.active_edges));
        self.make_cognitive_receipt_for(
            self.last_work.dendritic.work_units,
            u32::try_from(structural_work_units)
                .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?,
        )
    }

    fn make_cognitive_receipt_for(
        &self,
        dendritic_work_units: u32,
        structural_work_units: u32,
    ) -> Result<CognitiveWorkReceipt, ScaffoldContractError> {
        CognitiveWorkCounters::new(
            0,
            0,
            u64::from(dendritic_work_units),
            0,
            0,
            0,
            0,
            0,
            0,
            u64::from(structural_work_units),
            0,
            0,
        )
        .and_then(|counters| counters.into_receipt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{DendriticBranch, DendriticInputRef};

    #[test]
    fn backend_path_joins_conjunction_growth_receipts_and_checkpoint() {
        let branches = DendriticBranchSet::new(vec![DendriticBranch::new(
            0,
            1.0,
            1.0,
            vec![
                DendriticInputRef::new(1, 1.0).unwrap(),
                DendriticInputRef::new(2, 1.0).unwrap(),
            ],
        )
        .unwrap()])
        .unwrap();
        let mut state = GpuV11CausalState::new(
            8,
            branches,
            StructuralPlasticityConfig {
                max_candidates_per_region: 8,
                max_regions: 1,
                max_accepted_per_phase: 1,
                max_structural_edges: 1,
                min_candidate_score: 2,
                prune_score_below: 1,
            },
        )
        .unwrap();
        let mut activations = vec![0.0; 8];
        activations[1] = 0.6;
        activations[2] = 0.6;
        let before = state
            .recurrent_step(&activations, &[0.0; 8], |value| value)
            .unwrap();
        assert!(before[0] > 0.0);

        let work = state
            .apply_structural_phase(&[CoactivationEvidence {
                region: 0,
                source: 3,
                target: 4,
                coactivation: 100,
                eligibility: 0,
                concept_gap_support: 0,
            }])
            .unwrap();
        assert_eq!(work.structural.accepted_edges, 1);
        activations[3] = 1.0;
        let after = state
            .recurrent_step(&activations, &[0.0; 8], |value| value)
            .unwrap();
        assert!(after[4] > 0.0);
        assert!(work.cognitive.dendritic_ops > 0);
        assert!(work.cognitive.structural_ops > 0);
        assert_eq!(
            GpuV11CausalState::restore(state.checkpoint()).unwrap(),
            state
        );
    }
}
