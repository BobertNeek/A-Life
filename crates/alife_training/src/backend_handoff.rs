//! Device-free founder/optimizer handoff between explicit offline backends.
use crate::{
    FoundationTrainer, FoundationTrainerCheckpoint, PpoTrainingState, PpoValueHeadCheckpoint,
    TrainingError,
};
use alife_core::{
    BrainCapacityClass, BrainPhenotype, FoundationWeightAsset, PhenotypeCompiler,
    PhenotypeCompilerInputs, ScaffoldContractError,
};
/// Resolve existing output parents (including symlinks) without creating them,
/// and reject containment in either direction with every preserved source.
/// Callers must write through the returned path after successful admission.
pub fn resolve_isolated_training_output<'a>(
    output: &std::path::Path,
    preserved_sources: impl IntoIterator<Item = &'a std::path::Path>,
) -> std::io::Result<std::path::PathBuf> {
    use std::path::{Component, PathBuf};
    let mut resolved = PathBuf::new();
    for component in std::path::absolute(output)?.components() {
        match component {
            // A Windows drive/verbatim prefix alone is not a rooted path. Join
            // its RootDir before asking the filesystem to resolve anything.
            Component::Prefix(_) => {
                resolved.push(component.as_os_str());
                continue;
            }
            Component::CurDir => continue,
            Component::ParentDir => {
                resolved.pop();
            }
            _ => resolved.push(component.as_os_str()),
        }
        match std::fs::symlink_metadata(&resolved) {
            Ok(_) => resolved = resolved.canonicalize()?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
    }
    for source in preserved_sources {
        let source = source.canonicalize()?;
        if resolved.starts_with(&source) || source.starts_with(&resolved) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "training output must not overlap any read-only native source directory",
            ));
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod output_resolution_tests {
    use super::resolve_isolated_training_output;
    #[test]
    fn rooted_and_canonical_output_paths_resolve_before_source_containment_checks() {
        let root = std::env::temp_dir().join(format!(
            "alife-output-roots-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let canonical = root.canonicalize().unwrap();
        let source = canonical.join("source");
        std::fs::create_dir(&source).unwrap();
        let expected = canonical.join("new-lane/episode");
        for output in [root.join("new-lane/episode"), expected.clone()] {
            assert_eq!(
                resolve_isolated_training_output(
                    &output,
                    [&source].map(std::path::PathBuf::as_path)
                )
                .unwrap(),
                expected
            );
        }
        for output in [source.clone(), source.join("child"), canonical.clone()] {
            assert!(resolve_isolated_training_output(
                &output,
                [&source].map(std::path::PathBuf::as_path)
            )
            .is_err());
        }
        assert!(!expected.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FoundationTrainingBackendV1 {
    NativeCpu,
    NativeGpu,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FoundationContinuationKindV1 {
    FreshCohort,
    SameIndividual,
}
/// Founder and optimizer continuation package, separate from organism state.
/// Gameplay reads only the foundation asset through its normal codec.
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
    pub fn new_same_individual(
        origin: FoundationTrainingBackendV1,
        policy_version: u64,
        compiler_inputs: PhenotypeCompilerInputs,
        actor: FoundationTrainerCheckpoint,
        value: PpoValueHeadCheckpoint,
    ) -> Result<Self, TrainingError> {
        let handoff = Self {
            schema_version: 1,
            origin,
            continuation: FoundationContinuationKindV1::SameIndividual,
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
        self.validated_phenotype().map(|_| ())
    }

    fn validated_phenotype(&self) -> Result<BrainPhenotype, TrainingError> {
        let invalid = || TrainingError::Contract(ScaffoldContractError::InvalidDecisionEvidence);
        if self.schema_version != 1
            || self.policy_version == 0
            || self.biological_objective_version != 2
            || self.critic_objective_version != 1
            || self.objective_state_reset
        {
            return Err(invalid());
        }
        match (self.continuation, self.compiler_inputs.live_policy()) {
            (FoundationContinuationKindV1::FreshCohort, None) => (),
            (FoundationContinuationKindV1::SameIndividual, Some(marker))
                if marker.policy_version == self.policy_version =>
            {}
            _ => return Err(invalid()),
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
        Ok(phenotype)
    }
    pub fn admit(self) -> Result<AdmittedFoundationTrainingHandoff, TrainingError> {
        let phenotype = self.validated_phenotype()?;
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
    /// Restores founder and optimizer state into a GPU training session.
    /// Device-free validation is available through `admit`.
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

pub const SERIAL_HANDOFF_BINDING_FILE: &str = "native-training-handoff.json";
pub const SERIAL_HANDOFF_FILE: &str = "training-handoff.json";
pub const SERIAL_HANDOFF_SOURCE_FILES: [&str; 4] = [
    "cycle.json",
    "trained.alife-foundation",
    "actor-checkpoint.json",
    "value-checkpoint.json",
];
/// Portable boundary emitted only after native serial continuity admission.
/// Pins the complete native checkpoints, including moments, ages and stage mask.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationSerialHandoffBindingV1 {
    pub schema_version: u16,
    pub native_serial_continuity_validated: bool,
    pub source_sha256: std::collections::BTreeMap<String, String>,
    pub training_handoff_sha256: String,
}
impl FoundationSerialHandoffBindingV1 {
    pub fn validate(
        &self,
        actual_source_sha256: &std::collections::BTreeMap<String, String>,
        actual_handoff_sha256: &str,
    ) -> Result<(), TrainingError> {
        let valid_hash = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if self.schema_version != 1
            || !self.native_serial_continuity_validated
            || self.source_sha256.len() != SERIAL_HANDOFF_SOURCE_FILES.len()
            || SERIAL_HANDOFF_SOURCE_FILES
                .iter()
                .any(|name| !self.source_sha256.contains_key(*name))
            || self.source_sha256.values().any(|v| !valid_hash(v))
            || !valid_hash(&self.training_handoff_sha256)
            || self.source_sha256 != *actual_source_sha256
            || self.training_handoff_sha256 != actual_handoff_sha256
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        Ok(())
    }
}
