//! Admission-bound neural expression; biochemical activations remain tick-fresh.
use alife_core::{
    BrainPhenotype, NeuralReceptorEffects, NeuralReceptorFrame, NeuralReceptorPhenotype,
    PhenotypeHash, ScaffoldContractError,
};

/// The neural phenotype is an immutable compiler record. Learning, structural
/// edits and development update separate live state, not this expression source.
/// Replacements must compile a new value before publishing the resident.
#[derive(Debug, Clone, Copy)]
pub(super) struct ResidentNeuralReceptors {
    source: PhenotypeHash,
    expression: NeuralReceptorPhenotype,
}

impl ResidentNeuralReceptors {
    pub(super) fn compile(phenotype: &BrainPhenotype) -> Result<Self, ScaffoldContractError> {
        // Keep the full canonical phenotype-hash check at every admission and
        // restore. Neither the expression nor its source binding is persisted.
        Ok(Self {
            source: phenotype.phenotype_hash(),
            expression: NeuralReceptorPhenotype::compile(phenotype)?,
        })
    }

    pub(super) fn effects_for(
        &self,
        phenotype: &BrainPhenotype,
        frame: &NeuralReceptorFrame,
    ) -> Result<NeuralReceptorEffects, ScaffoldContractError> {
        // A future resident phenotype replacement must not silently reuse the
        // old expression. Public BrainPhenotype APIs cannot mutate its source.
        if self.source != phenotype.phenotype_hash() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        NeuralReceptorEffects::from_frame(frame, &self.expression)
    }
}

#[cfg(test)]
mod tests;
