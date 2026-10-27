//! An explicit learned-head overlay; biological/genetic compiler inputs remain
//! unchanged. Preparing this value performs no live mutation or GPU work.
use super::{BrainCapacityClass, BrainPhenotype, PhenotypeCompiler, PhenotypeCompilerInputs};
use crate::{Blake3Digest, FoundationWeightAsset, ScaffoldContractError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct N2048LivePolicyProvenanceV1 {
    pub schema: u16,
    pub birth_foundation_digest: Blake3Digest,
    pub policy_version: u64,
}

impl N2048LivePolicyProvenanceV1 {
    pub(super) fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.schema != 1
            || self.policy_version == 0
            || self.birth_foundation_digest.bytes() == &[0; 32]
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

/// Checked compiler/weight identity for an in-place learning boundary. Actual
/// live installation additionally requires a sealed neural authority boundary.
pub struct N2048LivePolicyTransition {
    previous_phenotype: BrainPhenotype,
    next_phenotype: BrainPhenotype,
    next_inputs: PhenotypeCompilerInputs,
    previous_policy: u64,
}

impl N2048LivePolicyTransition {
    pub fn prepare(
        previous_inputs: &PhenotypeCompilerInputs,
        previous_policy: u64,
        next_asset: FoundationWeightAsset,
    ) -> Result<Self, ScaffoldContractError> {
        let capacity = BrainCapacityClass::n2048();
        previous_inputs.validate_against(&capacity)?;
        let previous_asset = previous_inputs
            .n2048_candidate_asset()
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        if previous_inputs
            .live_policy()
            .is_some_and(|p| p.policy_version != previous_policy)
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let policy_version = previous_policy
            .checked_add(1)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let birth_foundation_digest = previous_inputs
            .live_policy()
            .map_or(previous_asset.digest(), |p| p.birth_foundation_digest);
        let next_inputs = PhenotypeCompilerInputs::try_new_with_live_n2048_policy(
            previous_inputs.genome().clone(),
            &capacity,
            previous_inputs.development().clone(),
            previous_inputs.sensor_profile(),
            next_asset,
            N2048LivePolicyProvenanceV1 {
                schema: 1,
                birth_foundation_digest,
                policy_version,
            },
        )?;
        let previous_phenotype = PhenotypeCompiler::compile_validated(previous_inputs, &capacity)?;
        let next_phenotype = PhenotypeCompiler::compile_validated(&next_inputs, &capacity)?;
        validate_weight_only_policy_successor(&previous_phenotype, &next_phenotype)?;
        Ok(Self {
            previous_phenotype,
            next_phenotype,
            next_inputs,
            previous_policy,
        })
    }

    pub const fn previous_phenotype(&self) -> &BrainPhenotype {
        &self.previous_phenotype
    }
    pub const fn next_phenotype(&self) -> &BrainPhenotype {
        &self.next_phenotype
    }
    pub const fn next_inputs(&self) -> &PhenotypeCompilerInputs {
        &self.next_inputs
    }
    pub const fn previous_policy(&self) -> u64 {
        self.previous_policy
    }
    pub const fn next_policy(&self) -> u64 {
        self.previous_policy + 1
    }
}

/// Identical coordinates/dynamics/topology, allowing only finite genetic weight
/// changes and their explicit foundation identity. No private state is reset.
pub fn validate_weight_only_policy_successor(
    a: &BrainPhenotype,
    b: &BrainPhenotype,
) -> Result<(), ScaffoldContractError> {
    if a.brain_class_id() != BrainCapacityClass::N2048_ID
        || a.brain_class_id() != b.brain_class_id()
        || a.schema_version() != b.schema_version()
        || a.neuron_count() != b.neuron_count()
        || a.microstep_count() != b.microstep_count()
        || a.sensor_profile() != b.sensor_profile()
        || a.foundation_abi().foundation_id() != b.foundation_abi().foundation_id()
        || a.foundation_abi().foundation_version() != b.foundation_abi().foundation_version()
        || a.foundation_abi().compatibility_family_id()
            != b.foundation_abi().compatibility_family_id()
        || a.lobe_layout() != b.lobe_layout()
        || a.language_codebook() != b.language_codebook()
        || a.cognitive_architecture() != b.cognitive_architecture()
        || a.projections() != b.projections()
        || a.neuron_dynamics() != b.neuron_dynamics()
        || a.sensor_encoder() != b.sensor_encoder()
        || a.candidate_decoder() != b.candidate_decoder()
        || a.speech_decoder() != b.speech_decoder()
        || a.memory_decoder() != b.memory_decoder()
        || a.cognitive_decoder() != b.cognitive_decoder()
        || a.cognitive_channel_plan() != b.cognitive_channel_plan()
        || a.plasticity_receptors() != b.plasticity_receptors()
        || a.replay_capture_plan() != b.replay_capture_plan()
        || a.sleep_consolidation_plan() != b.sleep_consolidation_plan()
        || a.plasticity_plan_digest() != b.plasticity_plan_digest()
        || a.persistent_address_map() != b.persistent_address_map()
        || a.route_abi_digest() != b.route_abi_digest()
        || a.plasticity_abi_digest() != b.plasticity_abi_digest()
        || a.budgets() != b.budgets()
        || a.synapses().len() != b.synapses().len()
        || a.synapses().iter().zip(b.synapses()).any(|(x, y)| {
            x.source() != y.source()
                || x.target() != y.target()
                || x.alpha().to_bits() != y.alpha().to_bits()
                || x.route_index() != y.route_index()
                || x.receptor_index() != y.receptor_index()
                || x.kind() != y.kind()
                || !x.genetic_weight().is_finite()
                || !y.genetic_weight().is_finite()
        })
    {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(())
}
