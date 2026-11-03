//! Founder inference without an offline actor checkpoint, critic or optimizer.
//! The canonical host still owns every recurrent, somatic and biological state.
use super::{invalid, replay, CpuFoundationTrainer};
use crate::{TrainingError, TrainingReplayEvaluation, TrainingSequence};
use alife_core::{BrainPhenotype, FoundationWeightAsset};

mod sealed {
    pub trait Sealed {}
}

pub struct CpuFounderInferenceStep {
    pub evaluation: TrainingReplayEvaluation,
    pub previous_activations: Option<Vec<f32>>,
}

/// Sealed read-only policy interface shared by the existing full-state trainer
/// and a founder-only evaluator. It cannot update or export an offline optimizer.
pub trait CpuFounderPolicy: sealed::Sealed {
    fn phenotype(&self) -> &BrainPhenotype;
    fn weights(&self) -> &[f32];
    fn foundation_asset(&self) -> &FoundationWeightAsset;
    fn infer_organism_step(
        &self,
        sequence: &TrainingSequence,
    ) -> Result<CpuFounderInferenceStep, TrainingError>;
}

pub struct CpuInferenceFounder {
    phenotype: BrainPhenotype,
    asset: FoundationWeightAsset,
    graph: replay::SparseGraph,
}

fn step(
    tape: replay::Tape,
    sequence: &TrainingSequence,
) -> Result<CpuFounderInferenceStep, TrainingError> {
    if sequence.ticks.len() != 1 || sequence.burn_in_ticks != 0 {
        return Err(invalid());
    }
    let count = sequence.ticks[0].microstep_count as usize;
    Ok(CpuFounderInferenceStep {
        previous_activations: count.checked_sub(1).map(|i| tape.a[i].clone()),
        evaluation: tape.evaluation,
    })
}

impl CpuInferenceFounder {
    pub fn new(
        phenotype: BrainPhenotype,
        asset: FoundationWeightAsset,
    ) -> Result<Self, TrainingError> {
        asset.validate_against(&phenotype)?;
        if phenotype
            .projections()
            .iter()
            .any(|p| p.delay_microsteps() != 0)
            || asset.weights().len() != phenotype.synapses().len()
            || asset
                .weights()
                .iter()
                .zip(phenotype.synapses())
                .any(|(w, s)| w.to_bits() != s.genetic_weight().to_bits())
        {
            return Err(invalid());
        }
        let graph = replay::SparseGraph::new(&phenotype);
        Ok(Self {
            phenotype,
            asset,
            graph,
        })
    }
}

impl sealed::Sealed for CpuInferenceFounder {}
impl CpuFounderPolicy for CpuInferenceFounder {
    fn phenotype(&self) -> &BrainPhenotype {
        &self.phenotype
    }
    fn weights(&self) -> &[f32] {
        self.asset.weights()
    }
    fn foundation_asset(&self) -> &FoundationWeightAsset {
        &self.asset
    }
    fn infer_organism_step(
        &self,
        sequence: &TrainingSequence,
    ) -> Result<CpuFounderInferenceStep, TrainingError> {
        sequence.validate_for(&self.phenotype)?;
        step(
            self.graph
                .forward(&self.phenotype, self.weights(), sequence)?,
            sequence,
        )
    }
}

impl sealed::Sealed for CpuFoundationTrainer {}
impl CpuFounderPolicy for CpuFoundationTrainer {
    fn phenotype(&self) -> &BrainPhenotype {
        self.phenotype()
    }
    fn weights(&self) -> &[f32] {
        self.weights()
    }
    fn foundation_asset(&self) -> &FoundationWeightAsset {
        &self.source_foundation
    }
    fn infer_organism_step(
        &self,
        sequence: &TrainingSequence,
    ) -> Result<CpuFounderInferenceStep, TrainingError> {
        step(self.forward(sequence)?, sequence)
    }
}
