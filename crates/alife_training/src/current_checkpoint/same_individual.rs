//! Exact same-individual continuation. Counts describe retained training, never
//! inferred policy ages or a freshly generated replacement organism.
use super::*;
use crate::FoundationContinuationKindV1;
use alife_core::{BrainCapacityClass, BrainPhenotype, PhenotypeCompiler, PhenotypeCompilerInputs};
use alife_gpu_backend::GpuTrainingSamplingConfig;
mod progress;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SameIndividualProgressV1 {
    pub session_nonce: u64,
    pub organism_id: u64,
    pub session_start_world_tick: u64,
    pub world_tick: u64,
    /// Explicit session-to-date retained PPO rows and completed PPO updates.
    pub trained_rows: u64,
    pub completed_updates: u64,
    pub sampling_temperature: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SameIndividualCurrentV1 {
    pub schema: u16,
    pub kind: String,
    pub counting_scope: String,
    pub progress: SameIndividualProgressV1,
    pub baseline_current_sha256: String,
    pub baseline_policy_version: u64,
    pub baseline_actor_optimizer_step: u32,
    pub baseline_value_optimizer_step: u32,
    pub baseline_trained_rows: u64,
    pub baseline_completed_updates: u64,
    pub total_trained_rows: u64,
    pub total_completed_updates: u64,
    pub compiler_inputs_sha256: String,
    pub actor_sha256: String,
    pub value_sha256: String,
    pub private_life_sha256: String,
    /// For an admitted individual without a composite birth record, this is
    /// the foundation identity in its pre-adoption private compiler inputs.
    /// It is an adoption anchor, not a claim about the organism's birth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adopted_foundation_digest: Option<alife_core::Blake3Digest>,
    pub sampling_state: GpuTrainingSamplingConfig,
}

struct Sha256Writer(Sha256);

impl std::io::Write for Sha256Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn sha(value: &impl Serialize) -> Result<String> {
    let mut writer = Sha256Writer(Sha256::new());
    serde_json::to_writer(&mut writer, value)?;
    Ok(format!("{:x}", writer.0.finalize()))
}

type CachedPrivateBindingKey = ([u8; 32], [u8; 32]);
type CachedPrivateBindingValue = (PhenotypeCompilerInputs, String, alife_core::PhenotypeHash);

fn cached_private_phenotype_binding(
    inputs_bytes: &[u8],
    phenotype_bytes: &[u8],
) -> Result<CachedPrivateBindingValue> {
    use std::collections::VecDeque;
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<VecDeque<(CachedPrivateBindingKey, CachedPrivateBindingValue)>>> =
        OnceLock::new();
    let key: CachedPrivateBindingKey = (
        Sha256::digest(inputs_bytes).into(),
        Sha256::digest(phenotype_bytes).into(),
    );
    let cache = CACHE.get_or_init(|| Mutex::new(VecDeque::with_capacity(8)));
    if let Ok(mut guard) = cache.lock() {
        if let Some(pos) = guard.iter().position(|(k, _)| *k == key) {
            let entry = guard.remove(pos).expect("checked cache index");
            let value = entry.1.clone();
            guard.push_back(entry);
            return Ok(value);
        }
    }
    let inputs: PhenotypeCompilerInputs = serde_json::from_slice(inputs_bytes)?;
    let phenotype: BrainPhenotype = serde_json::from_slice(phenotype_bytes)?;
    let compiled = crate::compile_validated_n2048_cached(&inputs, &BrainCapacityClass::n2048())?;
    let inputs_sha = sha(&inputs)?;
    if sha(&phenotype)? != sha(&compiled)? {
        return Err("same-individual private compiler/phenotype differs from global heads".into());
    }
    let value = (inputs, inputs_sha, compiled.phenotype_hash());
    if let Ok(mut guard) = cache.lock() {
        if !guard.iter().any(|(k, _)| *k == key) {
            if guard.len() >= 8 {
                guard.pop_front();
            }
            guard.push_back((key, value.clone()));
        }
    }
    Ok(value)
}

fn check_foundation_anchor(
    birth: Option<alife_core::Blake3Digest>,
    adopted: Option<alife_core::Blake3Digest>,
    live: alife_core::Blake3Digest,
) -> Result<()> {
    let expected = match (birth, adopted) {
        (Some(value), None) | (None, Some(value)) => value,
        (Some(_), Some(_)) => return Err("unexpected adoption anchor for composite birth".into()),
        (None, None) => return Err("private foundation adoption anchor missing".into()),
    };
    if live != expected {
        return Err("same-individual overlay changed private foundation identity".into());
    }
    Ok(())
}

#[cfg(test)]
mod foundation_anchor_tests {
    use super::*;
    #[test]
    fn composite_and_adopted_foundations_are_bound_without_inventing_birth() {
        let a = alife_core::Blake3Digest::from_bytes([1; 32]);
        let b = alife_core::Blake3Digest::from_bytes([2; 32]);
        assert!(check_foundation_anchor(Some(a), None, a).is_ok());
        assert!(check_foundation_anchor(None, Some(a), a).is_ok());
        assert!(check_foundation_anchor(None, None, a).is_err());
        assert!(check_foundation_anchor(Some(a), Some(a), a).is_err());
        assert!(check_foundation_anchor(None, Some(a), b).is_err());
        assert!(check_foundation_anchor(Some(a), None, b).is_err());
    }
}

fn private_asset(
    life: &CurrentLifeCheckpointV1,
    save: &alife_world::PortableSaveFile,
    asset_id: &str,
) -> Result<Vec<u8>> {
    let mut entries = save
        .assets
        .entries
        .iter()
        .filter(|e| e.asset_id == asset_id);
    let entry = entries
        .next()
        .ok_or("same-individual private asset missing")?;
    if entries.next().is_some() {
        return Err("same-individual private asset identity is ambiguous".into());
    }
    life.asset_bytes(&entry.relative_path)
}

/// Validates actual saved compiler/immutable-head coordinates, without a device
/// or any fresh-cohort compiler. Native admission additionally validates every
/// private asset, journal and mutable-state codec through the ordinary loader.
fn private_binding(
    life: &CurrentLifeCheckpointV1,
    handoff: &FoundationTrainingHandoffV1,
    progress: &SameIndividualProgressV1,
    adopted_foundation_digest: Option<alife_core::Blake3Digest>,
) -> Result<GpuTrainingSamplingConfig> {
    let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
    if save.creatures.len() != 1
        || save.creatures[0].organism_id.raw() != progress.organism_id
        || save.world.tick.raw() != progress.world_tick
    {
        return Err("same-individual organism or world tick differs from private save".into());
    }
    let creature = &save.creatures[0];
    let gpu = creature
        .gpu_brain
        .as_ref()
        .ok_or("same-individual private GPU state missing")?;
    if gpu.checkpoint_tick.raw() != progress.world_tick || gpu.organism_id != creature.organism_id {
        return Err("same-individual private GPU checkpoint boundary differs".into());
    }
    let inputs_bytes = private_asset(life, &save, &gpu.phenotype_compiler_inputs.asset_id)?;
    let phenotype_bytes = private_asset(life, &save, &gpu.immutable_phenotype.asset_id)?;
    let (inputs, inputs_sha, compiled_hash) =
        cached_private_phenotype_binding(&inputs_bytes, &phenotype_bytes)?;
    if inputs_sha != sha(&handoff.compiler_inputs)? || gpu.phenotype_hash != compiled_hash {
        return Err("same-individual private compiler/phenotype differs from global heads".into());
    }
    let birth_foundation = if let Some(birth) = &creature.composite_genetics {
        Some(
            alife_core::FoundationWeightAsset::decode_canonical(&private_asset(
                life,
                &save,
                &birth.foundation_asset_id,
            )?)?
            .digest(),
        )
    } else {
        None
    };
    check_foundation_anchor(
        birth_foundation,
        adopted_foundation_digest,
        inputs
            .live_policy()
            .ok_or("same-individual live policy marker missing")?
            .birth_foundation_digest,
    )?;
    let sampling =
        private_sampling(life, &save)?.ok_or("exact private training sampler missing")?;
    if sampling.temperature.to_bits() != progress.sampling_temperature.to_bits() {
        return Err("same-individual sampling configuration differs from private state".into());
    }
    Ok(sampling)
}

fn private_sampling(
    life: &CurrentLifeCheckpointV1,
    save: &alife_world::PortableSaveFile,
) -> Result<Option<GpuTrainingSamplingConfig>> {
    if save.creatures.len() != 1 {
        return Err("same-individual sampler requires exactly one retained organism".into());
    }
    let Some(exact) = save.creatures[0]
        .gpu_brain
        .as_ref()
        .and_then(|gpu| gpu.exact_cognitive_state.as_ref())
    else {
        return Ok(None);
    };
    let packet: serde_json::Value =
        serde_json::from_slice(&private_asset(life, save, &exact.asset_id)?)?;
    // The runtime owns this optional private byte packet. Reading the explicit
    // field here also allows this admission code to preserve older codecs.
    let Some(value) = packet
        .get("private_training_sampling")
        .filter(|v| !v.is_null())
    else {
        return Ok(None);
    };
    let sampling_bytes: Vec<u8> = serde_json::from_value(value.clone())?;
    if sampling_bytes.len() > 4096 {
        return Err("private training sampler exceeds its bound".into());
    }
    let sampling: GpuTrainingSamplingConfig = serde_json::from_slice(&sampling_bytes)?;
    sampling.validate()?;
    if sampling.demonstrator.is_some() {
        return Err("same-individual saved sampler contains a demonstrator".into());
    }
    Ok(Some(sampling))
}

impl SameIndividualCurrentV1 {
    pub fn validate(&self, current: &CurrentTrainingCheckpointV1) -> Result<()> {
        let p = &self.progress;
        progress::validate_session_counts(
            p.session_start_world_tick,
            p.world_tick,
            p.trained_rows,
            p.completed_updates,
            10_000_000,
        )?;
        progress::validate_actor_updates(
            self.baseline_actor_optimizer_step,
            current.handoff.actor.optimizer_step,
            p.completed_updates,
        )?;
        if self.counting_scope != "retained-same-individual-ppo-since-first-tag"
            || self.schema != 1
            || self.kind != "same-individual-ppo-v1"
            || current.handoff.continuation != FoundationContinuationKindV1::SameIndividual
            || p.session_nonce == 0
            || p.session_nonce == current.seed
            || p.organism_id == 0
            || p.world_tick < p.session_start_world_tick
            || !p.sampling_temperature.is_finite()
            || !(0.0001..=128.).contains(&p.sampling_temperature)
            || (p.completed_updates == 0) != (p.trained_rows == 0)
            || p.completed_updates > p.trained_rows
            || !current.seeds.contains(p.session_nonce)
            || current
                .seeds
                .pending_range
                .is_some_and(|[a, b]| a <= p.session_nonce && p.session_nonce <= b)
            || self.baseline_current_sha256.len() != 64
            || !self
                .baseline_current_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            || self
                .baseline_policy_version
                .checked_add(p.completed_updates)
                != Some(current.handoff.policy_version)
            || self.baseline_trained_rows.checked_add(p.trained_rows)
                != Some(self.total_trained_rows)
            || self
                .baseline_completed_updates
                .checked_add(p.completed_updates)
                != Some(self.total_completed_updates)
            || current.handoff.actor.optimizer_step < self.baseline_actor_optimizer_step
            || current.handoff.value.optimizer_step < self.baseline_value_optimizer_step
            || self.compiler_inputs_sha256 != sha(&current.handoff.compiler_inputs)?
            || self.actor_sha256 != sha(&current.handoff.actor)?
            || self.value_sha256 != sha(&current.handoff.value)?
        {
            return Err(
                "same-individual identity, exact heads, counters or session nonce mismatch".into(),
            );
        }
        let life = current
            .life
            .as_ref()
            .ok_or("same-individual private life missing")?;
        if self.private_life_sha256 != sha(life)?
            || sha(&self.sampling_state)?
                != sha(&private_binding(
                    life,
                    &current.handoff,
                    p,
                    self.adopted_foundation_digest,
                )?)?
        {
            return Err("same-individual private closure or exact sampler mismatch".into());
        }
        Ok(())
    }
}

impl CurrentTrainingCheckpointV1 {
    /// A retained private stream overrides a new request's sampling defaults.
    /// Missing legacy packets may be initialized by the producer, never reset.
    pub fn saved_training_sampling(&self) -> Result<Option<GpuTrainingSamplingConfig>> {
        let life = self.life.as_ref().ok_or("baseline private life missing")?;
        let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
        private_sampling(life, &save)
    }

    /// Admit the decoded full state while retaining the original file identity.
    /// JSON whitespace/order is not silently substituted for source provenance.
    pub fn admitted_source_sha256(&self, bytes: &[u8]) -> Result<String> {
        let encoded = self.encode()?;
        if bytes != encoded && Self::decode(bytes)?.encode()? != encoded {
            return Err("same-individual baseline bytes differ from admitted source state".into());
        }
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    pub fn same_individual_successor(
        &self,
        baseline_bytes: &[u8],
        handoff: FoundationTrainingHandoffV1,
        life: CurrentLifeCheckpointV1,
        progress: SameIndividualProgressV1,
        seeds: CheckpointSeedsV1,
    ) -> Result<Self> {
        self.validate()?;
        self.require_training_session_nonce(progress.session_nonce)?;
        let baseline_current_sha256 = self.admitted_source_sha256(baseline_bytes)?;
        handoff.validate()?;
        let previous_actor = &self.handoff.actor;
        let next_actor = &handoff.actor;
        if previous_actor.stage_mask != next_actor.stage_mask
            || sha(&previous_actor.config)? != sha(&next_actor.config)?
            || previous_actor.weights.len() != next_actor.weights.len()
            || previous_actor.update_ages.len() != next_actor.update_ages.len()
            || previous_actor.update_ages.len() != previous_actor.weights.len()
        {
            return Err(
                "same-individual optimizer configuration, mask or age coordinates changed".into(),
            );
        }
        for i in 0..previous_actor.weights.len() {
            if next_actor.update_ages[i] < previous_actor.update_ages[i]
                || (!previous_actor.stage_mask.is_trainable(i)
                    && (next_actor.weights[i].to_bits() != previous_actor.weights[i].to_bits()
                        || next_actor.first_moment[i].to_bits()
                            != previous_actor.first_moment[i].to_bits()
                        || next_actor.second_moment[i].to_bits()
                            != previous_actor.second_moment[i].to_bits()
                        || next_actor.update_ages[i] != previous_actor.update_ages[i]))
            {
                return Err(
                    "same-individual per-coordinate Adam age rolled back or frozen state changed"
                        .into(),
                );
            }
        }
        let old_life = self
            .life
            .as_ref()
            .ok_or("same-individual baseline private life missing")?;
        let old = alife_world::PortableSaveFile::from_json_str(&old_life.world_json)?;
        let saved = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
        if old.creatures.len() != 1
            || saved.creatures.len() != 1
            || old.creatures[0].organism_id != saved.creatures[0].organism_id
            || old.creatures[0].genome_id != saved.creatures[0].genome_id
            || old.creatures[0].composite_genetics != saved.creatures[0].composite_genetics
            || progress.session_start_world_tick != old.world.tick.raw()
            || saved.deterministic_seed != self.seed
        {
            return Err(
                "same-individual successor replaced baseline organism, birth or world".into(),
            );
        }
        let adopted_foundation_digest = if old.creatures[0].composite_genetics.is_none() {
            let gpu = old.creatures[0]
                .gpu_brain
                .as_ref()
                .ok_or("baseline private GPU state missing")?;
            let old_inputs: PhenotypeCompilerInputs = serde_json::from_slice(&private_asset(
                old_life,
                &old,
                &gpu.phenotype_compiler_inputs.asset_id,
            )?)?;
            if old_inputs.genome() != handoff.compiler_inputs.genome()
                || old_inputs.development() != handoff.compiler_inputs.development()
                || old_inputs.sensor_profile() != handoff.compiler_inputs.sensor_profile()
            {
                return Err("same-individual adoption changed compiler identity".into());
            }
            let old_phenotype =
                PhenotypeCompiler::compile_validated(&old_inputs, &BrainCapacityClass::n2048())?;
            let new_phenotype = PhenotypeCompiler::compile_validated(
                &handoff.compiler_inputs,
                &BrainCapacityClass::n2048(),
            )?;
            alife_core::validate_weight_only_policy_successor(&old_phenotype, &new_phenotype)?;
            let anchor = old_inputs
                .live_policy()
                .map(|p| p.birth_foundation_digest)
                .or_else(|| {
                    old_inputs
                        .n2048_candidate_asset()
                        .map(|asset| asset.digest())
                })
                .ok_or("baseline private foundation identity missing")?;
            if self
                .same_individual
                .as_ref()
                .and_then(|tag| tag.adopted_foundation_digest)
                .is_some_and(|previous| previous != anchor)
            {
                return Err("baseline adoption anchor changed".into());
            }
            Some(anchor)
        } else {
            None
        };
        let (rows, updates) = self.same_individual.as_ref().map_or((0, 0), |tag| {
            (tag.total_trained_rows, tag.total_completed_updates)
        });
        let tag = SameIndividualCurrentV1 {
            schema: 1,
            kind: "same-individual-ppo-v1".into(),
            counting_scope: "retained-same-individual-ppo-since-first-tag".into(),
            baseline_current_sha256,
            baseline_policy_version: self.handoff.policy_version,
            baseline_actor_optimizer_step: self.handoff.actor.optimizer_step,
            baseline_value_optimizer_step: self.handoff.value.optimizer_step,
            baseline_trained_rows: rows,
            baseline_completed_updates: updates,
            total_trained_rows: rows
                .checked_add(progress.trained_rows)
                .ok_or("trained rows overflow")?,
            total_completed_updates: updates
                .checked_add(progress.completed_updates)
                .ok_or("update count overflow")?,
            compiler_inputs_sha256: sha(&handoff.compiler_inputs)?,
            actor_sha256: sha(&handoff.actor)?,
            value_sha256: sha(&handoff.value)?,
            private_life_sha256: sha(&life)?,
            adopted_foundation_digest,
            sampling_state: private_binding(&life, &handoff, &progress, adopted_foundation_digest)?,
            progress,
        };
        let next = Self {
            schema_version: 1,
            seed: self.seed,
            founder_seed_base: self.founder_seed_base,
            seeds,
            handoff,
            life: Some(life),
            same_individual: Some(tag),
            terminal_disposition: None,
            training_attempt_origin: self.training_attempt_origin.clone(),
        };
        next.validate()?;
        Ok(next)
    }
}

#[cfg(test)]
mod cache_codec_tests {
    use super::*;
    use alife_core::{
        BrainGenome, DevelopmentState, FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler,
        SensorProfile, Tick, TrainingStageManifest,
    };

    fn compiler_fixture(weight_scale: f32) -> (PhenotypeCompilerInputs, BrainPhenotype) {
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(0x74_00, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let base = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let mut weights: Vec<_> = base.synapses().iter().map(|s| s.genetic_weight()).collect();
        weights[0] *= weight_scale;
        let asset = FoundationWeightAsset::from_trained_weights(
            &base,
            weights,
            TrainingStageManifest::new(1, 1, 1),
        )
        .unwrap();
        let (phenotype, inputs) =
            PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset)
                .unwrap();
        (inputs, phenotype)
    }

    #[test]
    fn streamed_sha_matches_previous_vec_codec_with_exact_float_lanes_and_errors() {
        #[derive(Serialize)]
        struct Packet {
            floats: Vec<f32>,
            text: String,
            payload: Vec<u8>,
        }
        let packet = Packet {
            floats: vec![
                0.0,
                -0.0,
                f32::from_bits(1),
                f32::MIN_POSITIVE,
                f32::MIN,
                f32::MAX,
            ],
            text: "\"\\\n\t☃".repeat(1024),
            payload: (0..=255).cycle().take(131_073).collect(),
        };
        assert_eq!(
            sha(&packet).unwrap(),
            format!("{:x}", Sha256::digest(serde_json::to_vec(&packet).unwrap()))
        );
        assert_ne!(sha(&0.0_f32).unwrap(), sha(&-0.0_f32).unwrap());
        struct Fails;
        impl Serialize for Fails {
            fn serialize<S: serde::Serializer>(
                &self,
                _: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("injected serialization error"))
            }
        }
        assert!(sha(&Fails).is_err());
    }

    #[test]
    fn private_binding_cache_rechecks_changed_raw_assets_and_never_remembers_failures() {
        let (inputs, phenotype) = compiler_fixture(1.0);
        let inputs_bytes = serde_json::to_vec(&inputs).unwrap();
        let phenotype_bytes = serde_json::to_vec(&phenotype).unwrap();
        for _ in 0..2 {
            let (actual_inputs, actual_sha, actual_hash) =
                cached_private_phenotype_binding(&inputs_bytes, &phenotype_bytes).unwrap();
            assert_eq!(serde_json::to_vec(&actual_inputs).unwrap(), inputs_bytes);
            assert_eq!(actual_sha, sha(&inputs).unwrap());
            assert_eq!(actual_hash, phenotype.phenotype_hash());
        }
        let (other_inputs, other_phenotype) = compiler_fixture(0.95);
        let other_inputs_bytes = serde_json::to_vec(&other_inputs).unwrap();
        let other_phenotype_bytes = serde_json::to_vec(&other_phenotype).unwrap();
        for _ in 0..2 {
            assert!(
                cached_private_phenotype_binding(&inputs_bytes, &other_phenotype_bytes).is_err()
            );
            assert!(
                cached_private_phenotype_binding(&other_inputs_bytes, &phenotype_bytes).is_err()
            );
            assert!(cached_private_phenotype_binding(
                &inputs_bytes[..inputs_bytes.len() - 1],
                &phenotype_bytes
            )
            .is_err());
        }
        let other =
            cached_private_phenotype_binding(&other_inputs_bytes, &other_phenotype_bytes).unwrap();
        assert_eq!(other.1, sha(&other_inputs).unwrap());
        assert_ne!(other.1, sha(&inputs).unwrap());
        assert!(cached_private_phenotype_binding(&inputs_bytes, &phenotype_bytes).is_ok());
    }

    fn current_fixture() -> CurrentTrainingCheckpointV1 {
        let (inputs, phenotype) = compiler_fixture(1.0);
        let count = phenotype.synapses().len();
        let mut actor = crate::FoundationTrainerCheckpoint {
            schema_version: 2,
            phenotype_hash: phenotype.phenotype_hash(),
            source_foundation_digest: inputs.n2048_candidate_asset().unwrap().digest(),
            optimizer_step: 3,
            config: crate::AdamWConfig::default(),
            stage_mask: crate::StageTrainableMask::from_synapse_indices(
                &phenotype,
                &(0..count as u32).collect::<Vec<_>>(),
            )
            .unwrap(),
            weights: phenotype
                .synapses()
                .iter()
                .map(|s| s.genetic_weight())
                .collect(),
            first_moment: vec![0.01; count],
            second_moment: vec![0.02; count],
            update_ages: vec![3; count],
        };
        actor.first_moment[0] = -0.0;
        let value = crate::PpoValueHeadCheckpoint {
            feature_count: phenotype.neuron_count(),
            optimizer_step: 3,
            parameters: vec![0.0; (phenotype.neuron_count() as usize + 1) * 3],
            last_updated_policy_version: Some(7),
        };
        CurrentTrainingCheckpointV1 {
            schema_version: 1,
            seed: 7400,
            founder_seed_base: 7400,
            seeds: CheckpointSeedsV1 {
                historical_freshness: "unknown".into(),
                excluded_ranges: vec![[1, 7400]],
                pending_range: None,
            },
            handoff: FoundationTrainingHandoffV1::new(
                crate::FoundationTrainingBackendV1::NativeCpu,
                7,
                inputs,
                actor,
                value,
            )
            .unwrap(),
            life: None,
            same_individual: None,
            terminal_disposition: None,
            training_attempt_origin: None,
        }
    }

    #[test]
    fn source_identity_fast_path_keeps_canonical_and_original_noncanonical_bytes_exact() {
        let source = current_fixture();
        let canonical = source.encode().unwrap();
        assert_eq!(
            source.admitted_source_sha256(&canonical).unwrap(),
            format!("{:x}", Sha256::digest(&canonical))
        );
        assert_eq!(
            CurrentTrainingCheckpointV1::decode(&canonical)
                .unwrap()
                .encode()
                .unwrap(),
            canonical
        );
        let payload = serde_json::to_vec_pretty(&source).unwrap();
        let mut noncanonical = MAGIC.to_vec();
        noncanonical.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        noncanonical.extend_from_slice(&Sha256::digest(&payload));
        noncanonical.extend_from_slice(&payload);
        assert_ne!(canonical, noncanonical);
        assert_eq!(
            source.admitted_source_sha256(&noncanonical).unwrap(),
            format!("{:x}", Sha256::digest(&noncanonical))
        );
        let mut changed = source.clone();
        changed.handoff.actor.first_moment[0] = 0.0;
        assert!(source
            .admitted_source_sha256(&changed.encode().unwrap())
            .is_err());
        let last = noncanonical.len() - 1;
        noncanonical[last] ^= 1;
        assert!(source.admitted_source_sha256(&noncanonical).is_err());
    }
}
