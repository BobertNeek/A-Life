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

#[cfg(feature = "receipt-lineage")]
fn exact_json_sha256(value: &impl serde::Serialize) -> Result<[u8; 32], TrainingError> {
    use sha2::{Digest, Sha256};
    struct HashWriter(Sha256);
    impl std::io::Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = HashWriter(Sha256::new());
    serde_json::to_writer(&mut writer, value)
        .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
    Ok(writer.0.finalize().into())
}

/// Compile validated inputs, reusing at most eight exact input/capacity pairs.
/// The receipt codec is used only as an in-process cache key, not as a new
/// durable identity. It retains signed zero in every serialized float lane.
pub fn compile_validated_n2048_cached(
    inputs: &PhenotypeCompilerInputs,
    capacity: &BrainCapacityClass,
) -> Result<BrainPhenotype, TrainingError> {
    inputs.validate_against(capacity)?;
    #[cfg(not(feature = "receipt-lineage"))]
    return Ok(PhenotypeCompiler::compile_validated(inputs, capacity)?);

    #[cfg(feature = "receipt-lineage")]
    {
        use std::{
            collections::VecDeque,
            sync::{Mutex, OnceLock},
        };
        type CacheEntry = ([u8; 32], BrainPhenotype);
        static CACHE: OnceLock<Mutex<VecDeque<CacheEntry>>> = OnceLock::new();
        let key = exact_json_sha256(&(inputs, capacity))?;
        let cache = CACHE.get_or_init(|| Mutex::new(VecDeque::with_capacity(8)));
        if let Ok(mut guard) = cache.lock() {
            if let Some(pos) = guard.iter().position(|(cached_key, _)| *cached_key == key) {
                let entry = guard.remove(pos).expect("checked cache index");
                let phenotype = entry.1.clone();
                guard.push_front(entry);
                return Ok(phenotype);
            }
        }
        let phenotype = PhenotypeCompiler::compile_validated(inputs, capacity)?;
        if let Ok(mut guard) = cache.lock() {
            // Compilation occurs outside the lock. A concurrent successful
            // compile of the same key must not consume another cache slot.
            if !guard.iter().any(|(cached_key, _)| *cached_key == key) {
                if guard.len() >= 8 {
                    guard.pop_back();
                }
                guard.push_front((key, phenotype.clone()));
            }
        }
        Ok(phenotype)
    }
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
                if marker.policy_version == self.policy_version => {}
            _ => return Err(invalid()),
        }
        let capacity = BrainCapacityClass::n2048();
        self.compiler_inputs.validate_against(&capacity)?;
        let foundation = self
            .compiler_inputs
            .n2048_candidate_asset()
            .ok_or_else(invalid)?;
        let phenotype = compile_validated_n2048_cached(&self.compiler_inputs, &capacity)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{
        AlphaStoragePolicy, BrainGenome, DevelopmentState, NormalizedScalar, SensorProfile, Tick,
        TrainingStageManifest,
    };

    fn fixture(
        alpha: f32,
    ) -> (
        BrainGenome,
        DevelopmentState,
        BrainPhenotype,
        FoundationWeightAsset,
    ) {
        let capacity = BrainCapacityClass::n2048();
        let mut genome = BrainGenome::scaffold(42, capacity.id());
        genome.alpha_mask.default_alpha = NormalizedScalar::new(alpha).unwrap();
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let base = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedTerrainVisionV1,
        )
        .unwrap();
        let asset = FoundationWeightAsset::from_trained_weights(
            &base,
            base.synapses().iter().map(|s| s.genetic_weight()).collect(),
            TrainingStageManifest::new(1, 1, 1),
        )
        .unwrap();
        (genome, development, base, asset)
    }

    fn exact(phenotype: &BrainPhenotype) -> Vec<u8> {
        serde_json::to_vec(phenotype).unwrap()
    }

    #[test]
    fn cached_n2048_compilation_matches_fresh_and_distinguishes_weight_mutations() {
        let capacity = BrainCapacityClass::n2048();
        let (genome, development, base, asset_a) = fixture(0.25);
        let (expected_a, inputs_a) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome.clone(),
            development.clone(),
            asset_a,
        )
        .unwrap();
        for _ in 0..2 {
            assert_eq!(
                exact(&compile_validated_n2048_cached(&inputs_a, &capacity).unwrap()),
                exact(&expected_a)
            );
        }
        let mut weights: Vec<_> = base.synapses().iter().map(|s| s.genetic_weight()).collect();
        weights[0] *= 0.95;
        let asset_b = FoundationWeightAsset::from_trained_weights(
            &base,
            weights,
            TrainingStageManifest::new(1, 1, 1),
        )
        .unwrap();
        let (expected_b, inputs_b) =
            PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset_b)
                .unwrap();
        let cached_b = compile_validated_n2048_cached(&inputs_b, &capacity).unwrap();
        assert_eq!(exact(&cached_b), exact(&expected_b));
        assert_ne!(
            cached_b.synapses()[0].genetic_weight().to_bits(),
            expected_a.synapses()[0].genetic_weight().to_bits()
        );
    }

    #[test]
    fn cached_n2048_compilation_preserves_genome_signed_zero_and_failed_admission() {
        let capacity = BrainCapacityClass::n2048();
        let (mut genome, development, _, asset) = fixture(0.0);
        let positive = PhenotypeCompilerInputs::try_new_with_n2048_candidate(
            genome.clone(),
            &capacity,
            development.clone(),
            asset.manifest().sensor_profile(),
            asset.clone(),
        )
        .unwrap();
        let expected = PhenotypeCompiler::compile_validated(&positive, &capacity).unwrap();
        assert_eq!(
            exact(&compile_validated_n2048_cached(&positive, &capacity).unwrap()),
            exact(&expected)
        );
        genome.alpha_mask.default_alpha = NormalizedScalar::new(-0.0).unwrap();
        let negative = PhenotypeCompilerInputs::try_new_with_n2048_candidate(
            genome.clone(),
            &capacity,
            development.clone(),
            asset.manifest().sensor_profile(),
            asset.clone(),
        )
        .unwrap();
        // Ordinary equality and canonical digests normalize signed zero. The
        // cache must nevertheless follow the exact fresh compiler result.
        assert_eq!(positive, negative);
        assert_ne!(
            serde_json::to_vec(&positive).unwrap(),
            serde_json::to_vec(&negative).unwrap()
        );
        match (
            PhenotypeCompiler::compile_validated(&negative, &capacity),
            compile_validated_n2048_cached(&negative, &capacity),
        ) {
            (Ok(expected_negative), Ok(actual)) => {
                assert_ne!(exact(&expected_negative), exact(&expected));
                assert_eq!(exact(&actual), exact(&expected_negative));
            }
            (Err(_), Err(_)) => (),
            _ => panic!("cache changed signed-zero compilation admission"),
        }
        assert!(compile_validated_n2048_cached(&positive, &BrainCapacityClass::n512()).is_err());
        genome.alpha_mask.storage_policy = AlphaStoragePolicy::DenseDebugReference;
        genome.alpha_mask.dense_reference_opt_in = true;
        let invalid = PhenotypeCompilerInputs::try_new_with_n2048_candidate(
            genome,
            &capacity,
            development,
            asset.manifest().sensor_profile(),
            asset,
        )
        .unwrap();
        for _ in 0..2 {
            assert!(compile_validated_n2048_cached(&invalid, &capacity).is_err());
        }
        assert_eq!(
            exact(&compile_validated_n2048_cached(&positive, &capacity).unwrap()),
            exact(&expected)
        );
    }

    #[cfg(feature = "receipt-lineage")]
    #[test]
    fn compilation_cache_key_streams_exact_codec_and_binds_capacity() {
        use sha2::{Digest, Sha256};
        let (genome, development, _, asset) = fixture(0.25);
        let capacity = BrainCapacityClass::n2048();
        let inputs = PhenotypeCompilerInputs::try_new_with_n2048_candidate(
            genome,
            &capacity,
            development,
            asset.manifest().sensor_profile(),
            asset,
        )
        .unwrap();
        let wire = serde_json::to_vec(&(&inputs, &capacity)).unwrap();
        assert_eq!(
            exact_json_sha256(&(&inputs, &capacity)).unwrap(),
            <[u8; 32]>::from(Sha256::digest(wire))
        );
        assert_ne!(
            exact_json_sha256(&(&inputs, &capacity)).unwrap(),
            exact_json_sha256(&(&inputs, &BrainCapacityClass::n512())).unwrap()
        );
        assert_ne!(
            exact_json_sha256(&0.0_f32).unwrap(),
            exact_json_sha256(&-0.0_f32).unwrap()
        );
        struct Fails;
        impl serde::Serialize for Fails {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("injected codec failure"))
            }
        }
        assert!(matches!(
            exact_json_sha256(&Fails),
            Err(TrainingError::Contract(
                ScaffoldContractError::PhenotypeCompile
            ))
        ));
    }
}
