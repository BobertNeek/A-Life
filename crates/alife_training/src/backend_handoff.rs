//! Device-free founder/optimizer handoff between explicit offline backends.
use crate::{
    FoundationTrainer, FoundationTrainerCheckpoint, PpoTrainingState, PpoValueHeadCheckpoint,
    TrainingError,
};
use alife_core::{
    BrainCapacityClass, BrainPhenotype, FoundationWeightAsset, PhenotypeCompiler,
    PhenotypeCompilerInputs, ScaffoldContractError,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FoundationTrainingBackendV1 {
    NativeCpu,
    NativeGpu,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FoundationContinuationKindV1 {
    FreshCohort,
}
/// Training-only package. It contains no creature decisions or hardware claims;
/// normal gameplay can consume the canonical asset through its existing codec.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationTrainingHandoffV1 {
    pub schema_version: u16,
    pub origin: FoundationTrainingBackendV1,
    pub continuation: FoundationContinuationKindV1,
    pub policy_version: u64,
    pub biological_objective_version: u16,
    pub critic_objective_version: u16,
    pub objective_state_reset: bool,
    pub compiler_inputs: PhenotypeCompilerInputs,
    pub actor: FoundationTrainerCheckpoint,
    pub value: PpoValueHeadCheckpoint,
}
pub struct AdmittedFoundationTrainingHandoff {
    pub origin: FoundationTrainingBackendV1,
    pub policy_version: u64,
    pub phenotype: BrainPhenotype,
    pub foundation: FoundationWeightAsset,
    pub actor: FoundationTrainerCheckpoint,
    pub value: PpoValueHeadCheckpoint,
}
impl FoundationTrainingHandoffV1 {
    pub fn new(
        origin: FoundationTrainingBackendV1,
        policy_version: u64,
        compiler_inputs: PhenotypeCompilerInputs,
        actor: FoundationTrainerCheckpoint,
        value: PpoValueHeadCheckpoint,
    ) -> Result<Self, TrainingError> {
        let handoff = Self {
            schema_version: 1,
            origin,
            continuation: FoundationContinuationKindV1::FreshCohort,
            policy_version,
            biological_objective_version: 2,
            critic_objective_version: 1,
            objective_state_reset: false,
            compiler_inputs,
            actor,
            value,
        };
        handoff.validate()?;
        Ok(handoff)
    }
    pub fn validate(&self) -> Result<(), TrainingError> {
        let invalid = || TrainingError::Contract(ScaffoldContractError::InvalidDecisionEvidence);
        if self.schema_version != 1
            || self.policy_version == 0
            || self.biological_objective_version != 2
            || self.critic_objective_version != 1
            || self.objective_state_reset
        {
            return Err(invalid());
        }
        let capacity = BrainCapacityClass::n2048();
        self.compiler_inputs.validate_against(&capacity)?;
        let foundation = self
            .compiler_inputs
            .n2048_candidate_asset()
            .ok_or_else(invalid)?;
        let phenotype = PhenotypeCompiler::compile_validated(&self.compiler_inputs, &capacity)?;
        self.actor.validate_for(&phenotype, foundation.digest())?;
        PpoTrainingState::from_checkpoint(self.value.clone())?;
        if self.value.feature_count != phenotype.neuron_count()
            || self.value.last_updated_policy_version != Some(self.policy_version)
            || self
                .actor
                .weights
                .iter()
                .zip(phenotype.synapses())
                .any(|(w, s)| w.to_bits() != s.genetic_weight().to_bits())
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn admit(self) -> Result<AdmittedFoundationTrainingHandoff, TrainingError> {
        self.validate()?;
        let phenotype = PhenotypeCompiler::compile_validated(
            &self.compiler_inputs,
            &BrainCapacityClass::n2048(),
        )?;
        let foundation = self
            .compiler_inputs
            .n2048_candidate_asset()
            .ok_or(ScaffoldContractError::PhenotypeCompile)?
            .clone();
        Ok(AdmittedFoundationTrainingHandoff {
            origin: self.origin,
            policy_version: self.policy_version,
            phenotype,
            foundation,
            actor: self.actor,
            value: self.value,
        })
    }
    /// Import into the real existing GPU Training session. No CPU decisions are
    /// relabeled, and neither optimizer is reinitialized. A real GPU owner must
    /// execute this path; CPU-only callers can validate/admit without a device.
    pub fn into_gpu_training_session(
        self,
        session: alife_runtime::GpuAuthoritativeSession,
    ) -> Result<(FoundationTrainer, PpoTrainingState), TrainingError> {
        let admitted = self.admit()?;
        if session.authority().consumer() != alife_runtime::GpuSessionConsumerKind::Training {
            return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
        }
        let mut trainer = FoundationTrainer::from_session(
            session,
            admitted.phenotype,
            admitted.foundation,
            admitted.actor.stage_mask.clone(),
            admitted.actor.config,
        )?;
        trainer.restore_checkpoint(&admitted.actor)?;
        Ok((trainer, PpoTrainingState::from_checkpoint(admitted.value)?))
    }
}
