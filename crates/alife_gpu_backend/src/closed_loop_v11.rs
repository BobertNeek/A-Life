//! Pass 1 backend join for bounded dendritic and structural computation.
//!
//! The closed-loop GPU path remains authoritative for normal neural execution.
//! This small backend-owned adapter gives the new core mechanisms a real
//! production backend seam while the larger shader ABI migration is deferred.

use alife_core::cognitive_work::CognitiveWorkCounters;
use alife_core::{
    apply_dendritic_conjunctions, CoactivationEvidence, CognitiveWorkReceipt, DendriticBranchSet,
    DendriticWorkReceipt, ScaffoldContractError, StructuralPlasticityConfig,
    StructuralPlasticityState, StructuralWorkReceipt,
};
use serde::{Deserialize, Serialize};

pub const GPU_V11_CAUSAL_STATE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpuV11SparseEdge {
    pub source: u32,
    pub target: u32,
    pub weight: f32,
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
    pub work: GpuV11WorkReceipt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuV11CausalState {
    neuron_count: u32,
    dendritic_branches: DendriticBranchSet,
    structural: StructuralPlasticityState,
    sparse_spans: Vec<GpuV11SparseSpan>,
    last_work: GpuV11WorkReceipt,
}

impl GpuV11CausalState {
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
            last_work: GpuV11WorkReceipt::default(),
        })
    }

    pub fn dendritic_branches(&self) -> &DendriticBranchSet {
        &self.dendritic_branches
    }

    pub fn sparse_spans(&self) -> &[GpuV11SparseSpan] {
        &self.sparse_spans
    }

    pub const fn last_work(&self) -> GpuV11WorkReceipt {
        self.last_work
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

    /// Applies sparse recurrent input, then dendritic conjunctions and accepted
    /// structural spans, before the caller's final activation function.
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
        let _discovery = self
            .structural
            .discover_candidates(evidence)
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let structural = self
            .structural
            .apply_structural_phase()
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;

        let mut sources = evidence.iter().map(|item| item.source).collect::<Vec<_>>();
        let mut targets = evidence.iter().map(|item| item.target).collect::<Vec<_>>();
        for span in &self.sparse_spans {
            targets.push(span.target);
            sources.extend(span.edges.iter().map(|edge| edge.source));
        }
        sources.sort_unstable();
        sources.dedup();
        targets.sort_unstable();
        targets.dedup();

        let mut rebuilt: Vec<GpuV11SparseSpan> = Vec::new();
        for source in sources {
            let mut input = vec![0.0; self.neuron_count as usize];
            input[source as usize] = 1.0;
            let output = self
                .structural
                .compute(&input)
                .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
            for target in &targets {
                let weight = output[*target as usize];
                if weight == 0.0 {
                    continue;
                }
                let span = rebuilt.iter_mut().find(|span| span.target == *target);
                if let Some(span) = span {
                    span.edges.push(GpuV11SparseEdge {
                        source,
                        target: *target,
                        weight,
                    });
                } else {
                    rebuilt.push(GpuV11SparseSpan {
                        target: *target,
                        edges: vec![GpuV11SparseEdge {
                            source,
                            target: *target,
                            weight,
                        }],
                    });
                }
            }
        }
        rebuilt.sort_unstable_by_key(|span| span.target);
        for span in &mut rebuilt {
            span.edges.sort_unstable_by_key(|edge| edge.source);
        }
        self.sparse_spans = rebuilt;
        self.last_work.structural = structural;
        self.last_work.cognitive = self.make_cognitive_receipt()?;
        Ok(self.last_work)
    }

    pub fn checkpoint(&self) -> GpuV11Checkpoint {
        GpuV11Checkpoint {
            schema_version: GPU_V11_CAUSAL_STATE_SCHEMA_VERSION,
            neuron_count: self.neuron_count,
            dendritic_branches: self.dendritic_branches.clone(),
            structural: self.structural.clone(),
            sparse_spans: self.sparse_spans.clone(),
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
            last_work: checkpoint.work,
        };
        if state.sparse_spans.iter().any(|span| {
            span.target >= state.neuron_count
                || span.edges.iter().any(|edge| {
                    edge.source >= state.neuron_count
                        || edge.target != span.target
                        || !edge.weight.is_finite()
                })
        }) {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(state)
    }

    fn make_cognitive_receipt(&self) -> Result<CognitiveWorkReceipt, ScaffoldContractError> {
        CognitiveWorkCounters::new(
            0,
            0,
            u64::from(self.last_work.dendritic.work_units),
            0,
            0,
            0,
            0,
            0,
            0,
            u64::from(self.last_work.structural.candidate_comparisons)
                .saturating_add(u64::from(self.last_work.structural.accepted_edges))
                .saturating_add(u64::from(self.last_work.structural.pruned_edges)),
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
