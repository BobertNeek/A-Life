//! Slice B real-hardware evidence for immediate learning, replay, and sleep restore.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::{
    ActionCandidate, ActionKind, ActionTarget, BodySnapshot, BoundedReplayBatch,
    BrainCapacityClass, BrainClassId, BrainGenome, BrainPhenotype, BrainScaleTier,
    CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef, CanonicalDigestBuilder,
    CompiledSynapseKind, Confidence, ConsolidationIntent, ConsolidationState, DecisionSnapshot,
    DecoderHeadKind, DevelopmentState, DurationTicks, EndocrineDelta, ExperiencePatch,
    ExperiencePatchBuilder, ExperienceSequenceId, HomeostaticDelta, HomeostaticSnapshot,
    MemoryBankConfig, MemorySidecarState, NeuralActionSelection, NormalizedScalar, OrganismId,
    PerceptionFrame, PhenotypeCompilerInputs, PhysicalActionOutcome, PhysicalContactKind,
    PolicyBackend, Pose, PostActionOutcome, PreActionSnapshot, ScaffoldContractError,
    SensorProfile, SensorProfileIdentity, SensorProfileProvenance, SensoryAbiVersion,
    SensoryChannels, SensorySnapshot, SignedValence, SleepPhase, SleepState, SleepTrigger, Tick,
    TopologicalMapConfig, TopologySidecar, Validate, Vec3f, Velocity,
    SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};
use alife_gpu_backend::{
    pack_replay_eligibility_sample, unpack_replay_eligibility_sample, GpuBrainCheckpointParts,
    GpuBrainCheckpointSnapshot, GpuBrainHandle, GpuBrainRestoreRequest, GpuClosedLoopBackend,
    GpuHardwareReceipt,
};
use alife_world::{
    persistence::{
        AssetManifest, CreatureMindSaveSummary, CreatureSaveState, GpuBrainAssetRef,
        GpuBrainSaveState, LearningTraceSaveSummary, PortableAssetDigest, PortableSaveFile,
        RuntimeConfig, WeightLayerSaveSummary,
    },
    CreatureAppearanceGenome, HeadlessScenarioBuilder, HeadlessWorld,
};
use serde::{Deserialize, Serialize};

use crate::{
    compile_gpu_birth_components, merge_gpu_checkpoint_manifest_entries, GameAppShellError,
    GpuBrainSidecarCapture, GpuCheckpointAssetStore, GpuLiveBrainRuntime, LiveBrainTickSummary,
};

use super::{
    adapter_identity_digest, atomic_write_receipt, capacity_slug, encode_hardware,
    encode_header_without_artifact_digest, encode_manifest_with_digest, hardware_digests,
    is_lower_hex_oid, policy_raw, read_git_provenance, synapse_payload_digest, tier_for_capacity,
    write_digest4, GitProvenance, GpuEvidenceError, GpuSliceEvidenceHeader,
    PhenotypeEvidenceManifest, GPU_EVIDENCE_MAX_ARTIFACT_BYTES, GPU_EVIDENCE_PASSING_STATUS_RAW,
    GPU_SLICE_B_RAW, GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
};

pub const GPU_SLICE_B_FIXTURE_SCHEMA: u16 = 1;
pub const GPU_SLICE_B_TOLERANCE: f32 = 1.0e-5;
const GPU_SLICE_B_EXPOSURES: u64 = 16;
const GPU_SLICE_B_POST_WAKE_PROBE_TICKS: u64 = 32;
const GPU_SLICE_B_MAX_WAKE_TICKS: usize = 128;
const SLICE_B_ARTIFACT_DOMAIN: &[u8] = b"alife.gpu.evidence.slice-b-artifact.v1";
const SAVE_ASSET_DOMAIN: &[u8] = b"alife.gpu.evidence.slice-b-save-assets.v1";
const EVIDENCE_SENSOR_PROFILE: SensorProfile = SensorProfile::PrivilegedAffordanceV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuLearningSleepAcceptanceOptions {
    pub capacity: BrainCapacityClass,
    pub deterministic_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpuSleepRestoreEvidence {
    pub checkpoint_phase_raw: u16,
    pub consolidation_state_raw: u16,
    pub cycle_id: u64,
    pub input_generation: u64,
    pub output_generation: u64,
    pub expected_remaining_swaps: u32,
    pub actual_remaining_swaps: u32,
    pub duplicate_swaps: u32,
    pub actions_while_non_awake: u32,
    pub save_asset_digest: [u64; 4],
    pub genetic_digest_before: [u64; 4],
    pub genetic_digest_after: [u64; 4],
    pub retained_target_delta: f32,
    pub tolerance: f32,
    pub reached_awake: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSliceBAcceptanceReceipt {
    #[serde(flatten)]
    pub header: GpuSliceEvidenceHeader,
    pub capacity_class: String,
    pub capacity_class_id: BrainClassId,
    pub capacity: BrainCapacityClass,
    pub phenotype_manifest: PhenotypeEvidenceManifest,
    pub fixture_schema_version: u16,
    pub deterministic_seed: u64,
    pub source_tree_clean: bool,
    pub backend_api: String,
    pub adapter_name: String,
    pub hardware: GpuHardwareReceipt,
    pub policy_backend: PolicyBackend,
    pub tolerance: f32,
    pub reward_logit_before: f32,
    pub reward_target_logit_after: f32,
    pub reward_neutral_logit_after: f32,
    pub reward_target_signed_delta: f32,
    pub reward_target_delta: f32,
    pub pain_logit_before: f32,
    pub pain_target_logit_after: f32,
    pub pain_neutral_logit_after: f32,
    pub pain_target_signed_delta: f32,
    pub pain_avoidance_delta: f32,
    pub unrelated_target_delta: f32,
    pub modulator_ablation_delta: f32,
    pub sleep_transition_sequence: Vec<u16>,
    pub consolidation_dispatches: u32,
    pub genetic_digest_before: [u64; 4],
    pub genetic_digest_after: [u64; 4],
    pub post_wake_retained_delta: f32,
    pub replay_event_count: u32,
    pub replay_sample_count: u32,
    pub replay_induced_fast_l1: f32,
    pub replay_vs_zero_sample_post_wake_delta: f32,
    pub gpu_learning_dispatches: u64,
    pub restore: GpuSleepRestoreEvidence,
}

#[derive(Debug)]
struct ImmediateLearningEvidence {
    hardware: GpuHardwareReceipt,
    reward_logit_before: f32,
    reward_target_logit_after: f32,
    reward_neutral_logit_after: f32,
    reward_target_signed_delta: f32,
    reward_target_delta: f32,
    pain_logit_before: f32,
    pain_target_logit_after: f32,
    pain_neutral_logit_after: f32,
    pain_target_signed_delta: f32,
    pain_avoidance_delta: f32,
    unrelated_target_delta: f32,
    modulator_ablation_delta: f32,
    learning_dispatches: u64,
}

#[derive(Debug)]
struct ReplayRestoreEvidence {
    hardware: GpuHardwareReceipt,
    sleep_transition_sequence: Vec<u16>,
    consolidation_dispatches: u32,
    post_wake_retained_delta: f32,
    replay_event_count: u32,
    replay_sample_count: u32,
    replay_induced_fast_l1: f32,
    replay_vs_zero_sample_post_wake_delta: f32,
    learning_dispatches: u64,
    restore: GpuSleepRestoreEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EvidenceCandidateProfile {
    target_family: CandidateActionFamily,
    unrelated_family: CandidateActionFamily,
    adapter_identity: [u64; 4],
}

impl GpuLearningSleepAcceptanceOptions {
    fn validate(self) -> Result<Self, GpuEvidenceError> {
        self.capacity.validate_contract()?;
        tier_for_capacity(self.capacity.id())?;
        if self.deterministic_seed == 0 {
            return Err(GpuEvidenceError::Contract(
                "Slice B evidence seed must be nonzero",
            ));
        }
        Ok(self)
    }
}

impl GpuSliceBAcceptanceReceipt {
    pub fn recompute_artifact_digest(&self) -> Result<[u64; 4], GpuEvidenceError> {
        let mut digest = CanonicalDigestBuilder::new(SLICE_B_ARTIFACT_DOMAIN);
        encode_header_without_artifact_digest(&mut digest, &self.header);
        encode_manifest_with_digest(&mut digest, &self.phenotype_manifest);
        digest.write_utf8(&self.capacity_class);
        digest.write_u16(self.capacity_class_id.raw());
        write_digest4(&mut digest, self.capacity.canonical_digest());
        digest.write_u16(self.fixture_schema_version);
        digest.write_u64(self.deterministic_seed);
        digest.write_bool(self.source_tree_clean);
        digest.write_utf8(&self.backend_api);
        digest.write_utf8(&self.adapter_name);
        encode_hardware(&mut digest, &self.hardware);
        digest.write_u8(policy_raw(self.policy_backend));
        digest.write_f32(self.tolerance)?;
        for value in [
            self.reward_logit_before,
            self.reward_target_logit_after,
            self.reward_neutral_logit_after,
            self.reward_target_signed_delta,
            self.reward_target_delta,
            self.pain_logit_before,
            self.pain_target_logit_after,
            self.pain_neutral_logit_after,
            self.pain_target_signed_delta,
            self.pain_avoidance_delta,
            self.unrelated_target_delta,
            self.modulator_ablation_delta,
        ] {
            digest.write_f32(value)?;
        }
        digest.write_sequence_len(self.sleep_transition_sequence.len());
        for phase in &self.sleep_transition_sequence {
            digest.write_u16(*phase);
        }
        digest.write_u32(self.consolidation_dispatches);
        write_digest4(&mut digest, self.genetic_digest_before);
        write_digest4(&mut digest, self.genetic_digest_after);
        digest.write_f32(self.post_wake_retained_delta)?;
        digest.write_u32(self.replay_event_count);
        digest.write_u32(self.replay_sample_count);
        digest.write_f32(self.replay_induced_fast_l1)?;
        digest.write_f32(self.replay_vs_zero_sample_post_wake_delta)?;
        digest.write_u64(self.gpu_learning_dispatches);
        encode_restore(&mut digest, &self.restore)?;
        Ok(digest.finish256())
    }

    pub fn validate_in_memory(&self) -> Result<(), GpuEvidenceError> {
        self.validate(false)
    }

    fn validate(&self, require_clean_source: bool) -> Result<(), GpuEvidenceError> {
        self.capacity.validate_contract()?;
        self.phenotype_manifest
            .validate_for_capacity(&self.capacity)?;
        let signed_opposite = self.reward_target_signed_delta * self.pain_target_signed_delta < 0.0;
        let restore = &self.restore;
        if self.header.artifact_schema != GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA
            || self.header.slice_raw != GPU_SLICE_B_RAW
            || self.header.class_id_raw != self.capacity.id().raw()
            || self.header.profile_id_raw != 0
            || self.header.profile_schema != 0
            || self.header.status_raw != GPU_EVIDENCE_PASSING_STATUS_RAW
            || !is_lower_hex_oid(&self.header.git_commit)
            || !is_lower_hex_oid(&self.header.source_tree_digest)
            || self.header.phenotype_hash != self.phenotype_manifest.phenotype_hash
            || self.header.phenotype_manifest_digest != self.phenotype_manifest.manifest_digest
            || self.header.capacity_digest != self.capacity.canonical_digest()
            || self.capacity_class != capacity_slug(self.capacity.id())?
            || self.capacity_class_id != self.capacity.id()
            || self.fixture_schema_version != GPU_SLICE_B_FIXTURE_SCHEMA
            || self.deterministic_seed == 0
            || (require_clean_source && !self.source_tree_clean)
            || self.backend_api != "vulkan"
            || self.adapter_name.trim().is_empty()
            || self.hardware.backend_api != self.backend_api
            || self.hardware.adapter_name != self.adapter_name
            || hardware_digests(&self.hardware).contains(&[0; 4])
            || self.policy_backend != PolicyBackend::NeuralClosedLoopGpu
            || !self.tolerance.is_finite()
            || self.tolerance <= 0.0
            || self.tolerance > 1.0e-3
            || !all_finite(&[
                self.reward_logit_before,
                self.reward_target_logit_after,
                self.reward_neutral_logit_after,
                self.reward_target_signed_delta,
                self.reward_target_delta,
                self.pain_logit_before,
                self.pain_target_logit_after,
                self.pain_neutral_logit_after,
                self.pain_target_signed_delta,
                self.pain_avoidance_delta,
                self.unrelated_target_delta,
                self.modulator_ablation_delta,
                self.post_wake_retained_delta,
                self.replay_induced_fast_l1,
                self.replay_vs_zero_sample_post_wake_delta,
            ])
            || !approximately_equal(
                self.reward_target_delta,
                self.reward_target_signed_delta.abs(),
                self.tolerance,
            )
            || !approximately_equal(
                self.pain_avoidance_delta,
                self.pain_target_signed_delta.abs(),
                self.tolerance,
            )
            || !signed_opposite
            || self.reward_target_delta <= self.tolerance
            || self.pain_avoidance_delta <= self.tolerance
            || self.unrelated_target_delta.abs() >= self.reward_target_delta.abs()
            || self.reward_target_delta <= self.modulator_ablation_delta + self.tolerance
            || self.sleep_transition_sequence.is_empty()
            || !self
                .sleep_transition_sequence
                .contains(&SleepPhase::Consolidating.raw())
            || !self
                .sleep_transition_sequence
                .contains(&SleepPhase::Awake.raw())
            || self.consolidation_dispatches != 1
            || self.genetic_digest_before == [0; 4]
            || self.genetic_digest_before != self.genetic_digest_after
            || self.post_wake_retained_delta <= self.tolerance
            || self.replay_event_count == 0
            || self.replay_sample_count == 0
            || self.replay_induced_fast_l1 <= self.tolerance
            || self.replay_vs_zero_sample_post_wake_delta <= self.tolerance
            || self.gpu_learning_dispatches == 0
            || !restore.passed
            || restore.checkpoint_phase_raw != SleepPhase::Consolidating.raw()
            || restore.consolidation_state_raw != 3
            || restore.cycle_id == 0
            || restore.output_generation != restore.input_generation.saturating_add(1)
            || restore.expected_remaining_swaps != 1
            || restore.actual_remaining_swaps != 1
            || restore.duplicate_swaps != 0
            || restore.actions_while_non_awake != 0
            || restore.save_asset_digest == [0; 4]
            || restore.genetic_digest_before != restore.genetic_digest_after
            || restore.genetic_digest_before != self.genetic_digest_before
            || !restore.retained_target_delta.is_finite()
            || restore.retained_target_delta <= restore.tolerance
            || !restore.tolerance.is_finite()
            || restore.tolerance <= 0.0
            || !restore.reached_awake
            || self.header.artifact_digest != self.recompute_artifact_digest()?
        {
            return Err(GpuEvidenceError::ContractDetail(format!(
                "Slice B learning/sleep evidence is inconsistent: reward_signed={}, reward_abs={}, pain_signed={}, pain_abs={}, unrelated={}, ablation={}, post_wake={}, replay_fast_l1={}, replay_post_delta={}, consolidation={}, restore_passed={}, restore_retained={}",
                self.reward_target_signed_delta,
                self.reward_target_delta,
                self.pain_target_signed_delta,
                self.pain_avoidance_delta,
                self.unrelated_target_delta,
                self.modulator_ablation_delta,
                self.post_wake_retained_delta,
                self.replay_induced_fast_l1,
                self.replay_vs_zero_sample_post_wake_delta,
                self.consolidation_dispatches,
                restore.passed,
                restore.retained_target_delta,
            )));
        }
        Ok(())
    }
}

pub fn run_gpu_learning_sleep_acceptance(
    options: GpuLearningSleepAcceptanceOptions,
) -> Result<GpuSliceBAcceptanceReceipt, GpuEvidenceError> {
    let provenance = read_git_provenance()?;
    run_gpu_learning_sleep_acceptance_with_provenance(options.validate()?, provenance)
}

pub fn run_and_write_gpu_learning_sleep_acceptance(
    options: GpuLearningSleepAcceptanceOptions,
    output: impl AsRef<Path>,
) -> Result<GpuSliceBAcceptanceReceipt, GpuEvidenceError> {
    let options = options.validate()?;
    let output = output.as_ref();
    validate_slice_b_output_filename(output, options.capacity.id())?;
    let before = read_git_provenance()?;
    if !before.clean {
        return Err(GpuEvidenceError::Git(
            "persistent Slice B evidence requires a clean committed worktree".to_string(),
        ));
    }
    let receipt = run_gpu_learning_sleep_acceptance_with_provenance(options, before.clone())?;
    let after = read_git_provenance()?;
    if before != after || !after.clean {
        return Err(GpuEvidenceError::Git(
            "source commit or tree changed during Slice B evidence capture".to_string(),
        ));
    }
    receipt.validate(true)?;
    atomic_write_receipt(output, &receipt)?;
    let loaded = load_gpu_slice_b_evidence(output)?;
    if loaded != receipt {
        return Err(GpuEvidenceError::Contract(
            "persisted Slice B evidence changed during round trip",
        ));
    }
    Ok(loaded)
}

pub fn load_gpu_slice_b_evidence(
    input: impl AsRef<Path>,
) -> Result<GpuSliceBAcceptanceReceipt, GpuEvidenceError> {
    let input = input.as_ref();
    let metadata = fs::metadata(input)?;
    if metadata.len() == 0 || metadata.len() > GPU_EVIDENCE_MAX_ARTIFACT_BYTES {
        return Err(GpuEvidenceError::Contract(
            "Slice B artifact size is outside its bound",
        ));
    }
    let receipt: GpuSliceBAcceptanceReceipt = serde_json::from_slice(&fs::read(input)?)?;
    receipt.validate(true)?;
    Ok(receipt)
}

fn run_gpu_learning_sleep_acceptance_with_provenance(
    options: GpuLearningSleepAcceptanceOptions,
    provenance: GitProvenance,
) -> Result<GpuSliceBAcceptanceReceipt, GpuEvidenceError> {
    let tier = tier_for_capacity(options.capacity.id())?;
    let (phenotype, genome, development) = compile_gpu_birth_components(
        options.deterministic_seed,
        tier,
        OrganismId(1),
        Tick::ZERO,
        EVIDENCE_SENSOR_PROFILE,
    )?;
    let compiler_inputs = PhenotypeCompilerInputs::try_new(
        genome.clone(),
        &options.capacity,
        development.clone(),
        EVIDENCE_SENSOR_PROFILE,
    )?;
    phenotype.validate_against(&options.capacity)?;
    let phenotype_manifest =
        PhenotypeEvidenceManifest::from_learning_phenotype(&phenotype, &options.capacity)?;
    let candidate_profile = evidence_stage(
        "select replay-captured evidence candidate family",
        select_evidence_candidate_profile(&phenotype),
    )?;
    let immediate = evidence_stage(
        "run immediate learning probe",
        run_immediate_learning_probe(&phenotype, &genome, &development, candidate_profile),
    )?;
    let replay = evidence_stage(
        "run replay and submitted-restore probe",
        run_replay_restore_probe(
            &options,
            tier,
            &phenotype,
            &genome,
            &development,
            &compiler_inputs,
            candidate_profile,
        ),
    )?;
    if candidate_profile.adapter_identity != adapter_identity_digest(&immediate.hardware)
        || candidate_profile.adapter_identity != adapter_identity_digest(&replay.hardware)
    {
        return Err(GpuEvidenceError::Contract(
            "Slice B candidate selection and probes did not run on the same adapter",
        ));
    }
    let genetic_digest = synapse_payload_digest(&phenotype)?;
    let mut receipt = GpuSliceBAcceptanceReceipt {
        header: GpuSliceEvidenceHeader {
            artifact_schema: GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
            slice_raw: GPU_SLICE_B_RAW,
            class_id_raw: options.capacity.id().raw(),
            profile_id_raw: 0,
            profile_schema: 0,
            status_raw: GPU_EVIDENCE_PASSING_STATUS_RAW,
            git_commit: provenance.commit,
            source_tree_digest: provenance.tree,
            artifact_digest: [0; 4],
            phenotype_hash: phenotype.phenotype_hash(),
            phenotype_manifest_digest: phenotype_manifest.manifest_digest,
            capacity_digest: options.capacity.canonical_digest(),
        },
        capacity_class: capacity_slug(options.capacity.id())?.to_string(),
        capacity_class_id: options.capacity.id(),
        capacity: options.capacity,
        phenotype_manifest,
        fixture_schema_version: GPU_SLICE_B_FIXTURE_SCHEMA,
        deterministic_seed: options.deterministic_seed,
        source_tree_clean: provenance.clean,
        backend_api: immediate.hardware.backend_api.clone(),
        adapter_name: immediate.hardware.adapter_name.clone(),
        hardware: immediate.hardware,
        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
        tolerance: GPU_SLICE_B_TOLERANCE,
        reward_logit_before: immediate.reward_logit_before,
        reward_target_logit_after: immediate.reward_target_logit_after,
        reward_neutral_logit_after: immediate.reward_neutral_logit_after,
        reward_target_signed_delta: immediate.reward_target_signed_delta,
        reward_target_delta: immediate.reward_target_delta,
        pain_logit_before: immediate.pain_logit_before,
        pain_target_logit_after: immediate.pain_target_logit_after,
        pain_neutral_logit_after: immediate.pain_neutral_logit_after,
        pain_target_signed_delta: immediate.pain_target_signed_delta,
        pain_avoidance_delta: immediate.pain_avoidance_delta,
        unrelated_target_delta: immediate.unrelated_target_delta,
        modulator_ablation_delta: immediate.modulator_ablation_delta,
        sleep_transition_sequence: replay.sleep_transition_sequence,
        consolidation_dispatches: replay.consolidation_dispatches,
        genetic_digest_before: genetic_digest,
        genetic_digest_after: genetic_digest,
        post_wake_retained_delta: replay.post_wake_retained_delta,
        replay_event_count: replay.replay_event_count,
        replay_sample_count: replay.replay_sample_count,
        replay_induced_fast_l1: replay.replay_induced_fast_l1,
        replay_vs_zero_sample_post_wake_delta: replay.replay_vs_zero_sample_post_wake_delta,
        gpu_learning_dispatches: immediate
            .learning_dispatches
            .saturating_add(replay.learning_dispatches),
        restore: replay.restore,
    };
    receipt.header.artifact_digest = receipt.recompute_artifact_digest()?;
    receipt.validate_in_memory()?;
    Ok(receipt)
}

fn run_immediate_learning_probe(
    phenotype: &BrainPhenotype,
    genome: &BrainGenome,
    development: &DevelopmentState,
    candidate_profile: EvidenceCandidateProfile,
) -> Result<ImmediateLearningEvidence, GpuEvidenceError> {
    let ids = [
        OrganismId(11),
        OrganismId(12),
        OrganismId(13),
        OrganismId(14),
    ];
    let mut backend = evidence_stage(
        "create immediate-learning GPU backend",
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1()),
    )?;
    let handles = ids
        .map(|organism| backend.insert_brain(organism, phenotype.clone()))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let hardware = backend.hardware_receipt().clone();

    let reward_series = run_paired_waking_exposures(
        &mut backend,
        [handles[0], handles[1]],
        [ids[0], ids[1]],
        genome,
        development,
        1_000,
        0.8,
        0.0,
        candidate_profile,
    )?;
    let reward_probe_tick = 1_000 + GPU_SLICE_B_EXPOSURES * 2;
    let reward_after_frames = ids[..2]
        .iter()
        .map(|organism| {
            evidence_frame(
                *organism,
                Tick::new(reward_probe_tick),
                EvidenceCandidate::Target,
                candidate_profile,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let reward_after = backend.tick_batch(&[
        (handles[0], reward_after_frames[0].clone()),
        (handles[1], reward_after_frames[1].clone()),
    ])?;
    let reward_target_signed_delta =
        reward_after[0].selection.logit - reward_after[1].selection.logit;
    discard_probe_ticks(&mut backend, &reward_after)?;

    let unrelated_frames = ids[..2]
        .iter()
        .map(|organism| {
            evidence_frame(
                *organism,
                Tick::new(reward_probe_tick + 2),
                EvidenceCandidate::Unrelated,
                candidate_profile,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let unrelated = backend.tick_batch(&[
        (handles[0], unrelated_frames[0].clone()),
        (handles[1], unrelated_frames[1].clone()),
    ])?;
    let unrelated_target_delta = unrelated[0].selection.logit - unrelated[1].selection.logit;
    discard_probe_ticks(&mut backend, &unrelated)?;

    let pain_series = run_paired_waking_exposures(
        &mut backend,
        [handles[2], handles[3]],
        [ids[2], ids[3]],
        genome,
        development,
        2_000,
        0.0,
        0.8,
        candidate_profile,
    )?;
    let pain_probe_tick = 2_000 + GPU_SLICE_B_EXPOSURES * 2;
    let pain_after_frames = ids[2..]
        .iter()
        .map(|organism| {
            evidence_frame(
                *organism,
                Tick::new(pain_probe_tick),
                EvidenceCandidate::Target,
                candidate_profile,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let pain_after = backend.tick_batch(&[
        (handles[2], pain_after_frames[0].clone()),
        (handles[3], pain_after_frames[1].clone()),
    ])?;
    let pain_target_signed_delta = pain_after[0].selection.logit - pain_after[1].selection.logit;
    discard_probe_ticks(&mut backend, &pain_after)?;

    Ok(ImmediateLearningEvidence {
        hardware,
        reward_logit_before: reward_series.initial_logit,
        reward_target_logit_after: reward_after[0].selection.logit,
        reward_neutral_logit_after: reward_after[1].selection.logit,
        reward_target_signed_delta,
        reward_target_delta: reward_target_signed_delta.abs(),
        pain_logit_before: pain_series.initial_logit,
        pain_target_logit_after: pain_after[0].selection.logit,
        pain_neutral_logit_after: pain_after[1].selection.logit,
        pain_target_signed_delta,
        pain_avoidance_delta: pain_target_signed_delta.abs(),
        unrelated_target_delta,
        modulator_ablation_delta: reward_series
            .control_max_abs_delta
            .max(pain_series.control_max_abs_delta),
        learning_dispatches: reward_series
            .learning_dispatches
            .saturating_add(pain_series.learning_dispatches),
    })
}

#[derive(Debug, Clone, Copy)]
struct PairedWakingExposureEvidence {
    initial_logit: f32,
    control_max_abs_delta: f32,
    learning_dispatches: u64,
}

#[allow(clippy::too_many_arguments)]
fn run_paired_waking_exposures(
    backend: &mut GpuClosedLoopBackend,
    handles: [GpuBrainHandle; 2],
    organisms: [OrganismId; 2],
    genome: &BrainGenome,
    development: &DevelopmentState,
    first_tick: u64,
    experimental_reward: f32,
    experimental_pain: f32,
    candidate_profile: EvidenceCandidateProfile,
) -> Result<PairedWakingExposureEvidence, GpuEvidenceError> {
    let mut initial_logit = None;
    let mut control_max_abs_delta = 0.0_f32;
    let mut learning_dispatches = 0_u64;
    for exposure in 0..GPU_SLICE_B_EXPOSURES {
        let tick = Tick::new(first_tick.saturating_add(exposure.saturating_mul(2)));
        let frames = organisms
            .map(|organism| {
                evidence_frame(organism, tick, EvidenceCandidate::Target, candidate_profile)
            })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let ticks = backend.tick_batch(&[
            (handles[0], frames[0].clone()),
            (handles[1], frames[1].clone()),
        ])?;
        if exposure == 0 {
            require_matching_initial_logits(&ticks)?;
            initial_logit = Some(ticks[0].selection.logit);
        }
        let experimental_patch = sealed_credit_patch(
            handles[0],
            genome,
            development,
            &frames[0],
            &ticks[0],
            exposure + 1,
            experimental_reward,
            experimental_pain,
        )?;
        let control_patch = sealed_credit_patch(
            handles[1],
            genome,
            development,
            &frames[1],
            &ticks[1],
            exposure + 1,
            0.0,
            0.0,
        )?;
        let receipts = backend.apply_sealed_outcome_batch(&[
            (handles[0], &experimental_patch),
            (handles[1], &control_patch),
        ])?;
        if receipts.len() != 2 {
            return Err(GpuEvidenceError::Contract(
                "paired waking exposure did not return two learning receipts",
            ));
        }
        control_max_abs_delta = control_max_abs_delta.max(receipts[1].max_abs_delta);
        learning_dispatches = learning_dispatches.saturating_add(receipts.len() as u64);
    }
    Ok(PairedWakingExposureEvidence {
        initial_logit: initial_logit.ok_or(GpuEvidenceError::Contract(
            "paired waking exposure series was empty",
        ))?,
        control_max_abs_delta,
        learning_dispatches,
    })
}

fn run_replay_restore_probe(
    options: &GpuLearningSleepAcceptanceOptions,
    tier: BrainScaleTier,
    phenotype: &BrainPhenotype,
    genome: &BrainGenome,
    development: &DevelopmentState,
    compiler_inputs: &PhenotypeCompilerInputs,
    candidate_profile: EvidenceCandidateProfile,
) -> Result<ReplayRestoreEvidence, GpuEvidenceError> {
    let organism_id = OrganismId(21);
    let mut source_backend = evidence_stage(
        "create replay probe GPU backend",
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1()),
    )?;
    let source_handle = evidence_stage(
        "insert replay source brain",
        source_backend.insert_brain(organism_id, phenotype.clone()),
    )?;
    let source_hardware = source_backend.hardware_receipt().clone();
    let mut learning_dispatches = 0_u64;
    for exposure in 0..GPU_SLICE_B_EXPOSURES {
        let tick = Tick::new(exposure);
        let frame = evidence_frame(
            organism_id,
            tick,
            EvidenceCandidate::Target,
            candidate_profile,
        )?;
        let ticks = evidence_stage(
            "dispatch replay learning exposure",
            source_backend.tick_batch(&[(source_handle, frame.clone())]),
        )?;
        let patch = sealed_credit_patch(
            source_handle,
            genome,
            development,
            &frame,
            &ticks[0],
            exposure + 1,
            0.8,
            0.0,
        )?;
        let receipts = evidence_stage(
            "apply replay learning exposure",
            source_backend.apply_sealed_outcome_batch(&[(source_handle, &patch)]),
        )?;
        learning_dispatches = learning_dispatches.saturating_add(receipts.len() as u64);
    }

    let checkpoint_tick = Tick::new(GPU_SLICE_B_EXPOSURES);
    let source_snapshot = evidence_stage(
        "capture exact pre-sleep source checkpoint",
        source_backend.snapshot_brain(source_handle, checkpoint_tick),
    )?;
    let source_parts = source_snapshot.clone().into_parts();
    let source_replay = evidence_stage(
        "build source replay batch",
        source_backend.build_sleep_replay_batch(source_handle),
    )?;
    if source_replay.events.is_empty()
        || source_replay.eligibility_samples.is_empty()
        || source_replay
            .eligibility_samples
            .iter()
            .all(|sample| sample.eligibility_q15 == 0)
    {
        return Err(GpuEvidenceError::ContractDetail(format!(
            "Slice B replay fixture did not capture nonzero target eligibility: class={}, events={}, spans={}, samples={}, target_capture=[{}], family_activation=[{}]",
            options.capacity.id().raw(),
            source_replay.events.len(),
            source_replay.synapse_spans.len(),
            source_replay.eligibility_samples.len(),
            replay_target_capture_summary(
                phenotype,
                &source_replay,
                &source_parts,
                candidate_profile.target_family,
            ),
            decoder_family_activation_summary(phenotype, &source_parts),
        )));
    }
    drop(source_backend);

    let mut replayed_backend = evidence_stage(
        "create exact-replay restore backend",
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1()),
    )?;
    let replayed_restore = evidence_stage(
        "restore exact pre-sleep checkpoint",
        replayed_backend.restore_brain(
            organism_id,
            phenotype.clone(),
            GpuBrainRestoreRequest::try_new(source_snapshot)?,
        ),
    )?;
    let replayed_handle = replayed_restore.handle;
    let replayed_batch = evidence_stage(
        "build exact restored replay batch",
        replayed_backend.build_sleep_replay_batch(replayed_handle),
    )?;
    if replayed_batch != source_replay {
        return Err(GpuEvidenceError::Contract(
            "exact pre-sleep restore changed the replay batch",
        ));
    }

    let mut zero_parts = source_parts.clone();
    let mut changed_sample = false;
    for packed in &mut zero_parts.replay_samples {
        let (event_index, eligibility_q15) = unpack_replay_eligibility_sample(*packed);
        changed_sample |= eligibility_q15 != 0;
        *packed = pack_replay_eligibility_sample(event_index, 0);
    }
    if !changed_sample {
        return Err(GpuEvidenceError::Contract(
            "zero-sample checkpoint ablation changed no replay eligibility",
        ));
    }
    let mut exactness_check = zero_parts.clone();
    exactness_check.replay_samples = source_parts.replay_samples.clone();
    if exactness_check != source_parts {
        return Err(GpuEvidenceError::Contract(
            "zero-sample checkpoint ablation changed non-replay state",
        ));
    }
    let zero_snapshot = GpuBrainCheckpointSnapshot::try_from_parts(zero_parts)?;
    let mut zero_backend = evidence_stage(
        "create zero-sample restore backend",
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1()),
    )?;
    let zero_restore = evidence_stage(
        "restore zero-sample pre-sleep checkpoint",
        zero_backend.restore_brain(
            organism_id,
            phenotype.clone(),
            GpuBrainRestoreRequest::try_new(zero_snapshot)?,
        ),
    )?;
    let zero_handle = zero_restore.handle;
    let zero_sample_batch = evidence_stage(
        "build zero-sample restored replay batch",
        zero_backend.build_sleep_replay_batch(zero_handle),
    )?;
    if zero_sample_batch.events != replayed_batch.events
        || zero_sample_batch.synapse_spans != replayed_batch.synapse_spans
        || zero_sample_batch.eligibility_samples.len() != replayed_batch.eligibility_samples.len()
        || zero_sample_batch
            .eligibility_samples
            .iter()
            .any(|sample| sample.eligibility_q15 != 0)
    {
        return Err(GpuEvidenceError::Contract(
            "zero-sample restore changed more than replay eligibility values",
        ));
    }

    let hardware = replayed_backend.hardware_receipt().clone();
    let zero_hardware = zero_backend.hardware_receipt().clone();
    if adapter_identity_digest(&source_hardware) != adapter_identity_digest(&hardware)
        || adapter_identity_digest(&hardware) != adapter_identity_digest(&zero_hardware)
    {
        return Err(GpuEvidenceError::Contract(
            "paired replay restores used different adapters",
        ));
    }

    let intent = ConsolidationIntent { cycle_id: 1 };
    let replayed_request = evidence_stage(
        "prepare replayed sleep consolidation",
        replayed_backend.prepare_sleep_consolidation(replayed_handle, intent, &replayed_batch),
    )?;
    let zero_request = evidence_stage(
        "prepare zero-sample sleep consolidation",
        zero_backend.prepare_sleep_consolidation(zero_handle, intent, &zero_sample_batch),
    )?;
    if replayed_request.cycle_id != zero_request.cycle_id
        || replayed_request.phenotype_hash != zero_request.phenotype_hash
        || replayed_request.input_generation != zero_request.input_generation
        || replayed_request.expected_output_generation != zero_request.expected_output_generation
        || replayed_request.input_digest != zero_request.input_digest
        || replayed_request.max_replay_events != zero_request.max_replay_events
        || replayed_request.max_replay_eligibility_samples
            != zero_request.max_replay_eligibility_samples
        || replayed_request.replay_digest == zero_request.replay_digest
    {
        return Err(GpuEvidenceError::Contract(
            "replay ablation requests differ outside their replay digest",
        ));
    }
    let replayed_job = evidence_stage(
        "submit replayed sleep consolidation",
        replayed_backend.submit_sleep_consolidation(
            replayed_handle,
            &replayed_request,
            &replayed_batch,
        ),
    )?;
    let zero_job = evidence_stage(
        "submit zero-sample sleep consolidation",
        zero_backend.submit_sleep_consolidation(zero_handle, &zero_request, &zero_sample_batch),
    )?;

    let submitted_state = submitted_sleep_state(checkpoint_tick, replayed_request, replayed_job)?;
    let temporary = TemporaryEvidenceRoot::new("slice-b")?;
    let store = GpuCheckpointAssetStore::new(temporary.path())?;
    let world = evidence_world(options.deterministic_seed, organism_id, checkpoint_tick)?;
    let sidecar_profile = SensorProfileIdentity {
        profile_id: EVIDENCE_SENSOR_PROFILE.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let memory = MemorySidecarState::new_profiled(
        organism_id,
        sidecar_profile,
        MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0)?)?,
    )?;
    let topology = TopologySidecar::new_profiled(
        organism_id,
        sidecar_profile,
        TopologicalMapConfig::default(),
    )?;
    let submitted_write = evidence_stage(
        "capture submitted sleep checkpoint",
        store.capture_brain(
            &mut replayed_backend,
            replayed_handle,
            phenotype,
            compiler_inputs,
            submitted_state,
            checkpoint_tick,
            None,
            GpuBrainSidecarCapture {
                sensor_profile: sidecar_profile,
                memory: &memory,
                topology: &topology,
                tracked_objects: world.tracked_objects().save_state(organism_id)?,
                retained_learning: None,
            },
        ),
    )?;
    let mut manifest = AssetManifest::empty();
    merge_gpu_checkpoint_manifest_entries(&mut manifest, submitted_write.manifest_entries)?;
    let save = build_checkpoint_save(
        options,
        tier,
        phenotype,
        genome,
        &world,
        manifest.clone(),
        submitted_write.save_state.clone(),
    )?;
    evidence_stage(
        "validate submitted portable save",
        save.validate_with_asset_root(temporary.path()),
    )?;
    evidence_stage(
        "restore submitted payloads in isolation",
        validate_checkpoint_payloads(&store, &save),
    )?;
    let save_asset_digest = evidence_stage(
        "digest submitted portable save assets",
        canonical_gpu_save_asset_digest(&save, temporary.path()),
    )?;
    evidence_stage(
        "reject every missing or corrupted submitted save asset",
        validate_save_asset_corruption_matrix(&save, temporary.path(), save_asset_digest),
    )?;

    let replayed_staged = evidence_stage(
        "poll replayed sleep consolidation",
        replayed_backend.poll_sleep_consolidation(replayed_handle, replayed_job),
    )?
    .ok_or(GpuEvidenceError::Contract(
        "replayed sleep job did not complete",
    ))?;
    let zero_staged = evidence_stage(
        "poll zero-sample sleep consolidation",
        zero_backend.poll_sleep_consolidation(zero_handle, zero_job),
    )?
    .ok_or(GpuEvidenceError::Contract(
        "zero-sample sleep job did not complete",
    ))?;
    let replayed_commit = evidence_stage(
        "commit replayed source sleep job",
        replayed_backend.commit_sleep_consolidation(
            replayed_handle,
            &replayed_request,
            &replayed_staged.staged,
        ),
    )?;
    let zero_commit = evidence_stage(
        "commit zero-sample source sleep job",
        zero_backend.commit_sleep_consolidation(zero_handle, &zero_request, &zero_staged.staged),
    )?;
    let expected_awake = evidence_stage(
        "capture expected post-consolidation checkpoint",
        store.capture_brain(
            &mut replayed_backend,
            replayed_handle,
            phenotype,
            compiler_inputs,
            SleepState::awake_at(checkpoint_tick),
            checkpoint_tick,
            None,
            GpuBrainSidecarCapture {
                sensor_profile: sidecar_profile,
                memory: &memory,
                topology: &topology,
                tracked_objects: world.tracked_objects().save_state(organism_id)?,
                retained_learning: None,
            },
        ),
    )?;

    let mut post_delta = 0.0_f32;
    for offset in 0..GPU_SLICE_B_POST_WAKE_PROBE_TICKS {
        let post_frame = evidence_frame(
            organism_id,
            Tick::new(100 + offset),
            EvidenceCandidate::Target,
            candidate_profile,
        )?;
        let replayed_post = evidence_stage(
            "dispatch replayed post-consolidation comparison",
            replayed_backend.tick_batch(&[(replayed_handle, post_frame.clone())]),
        )?;
        let zero_post = evidence_stage(
            "dispatch zero-sample post-consolidation comparison",
            zero_backend.tick_batch(&[(zero_handle, post_frame)]),
        )?;
        post_delta =
            post_delta.max((replayed_post[0].selection.logit - zero_post[0].selection.logit).abs());
        discard_probe_ticks(&mut replayed_backend, &replayed_post)?;
        discard_probe_ticks(&mut zero_backend, &zero_post)?;
    }
    if replayed_commit.generation_swaps != 1
        || zero_commit.generation_swaps != 1
        || replayed_commit.replay_induced_fast_l1 <= GPU_SLICE_B_TOLERANCE
        || zero_commit.replay_induced_fast_l1 > GPU_SLICE_B_TOLERANCE
        || post_delta <= GPU_SLICE_B_TOLERANCE
    {
        let capture_summary = replay_nonzero_capture_summary(phenotype, &replayed_batch);
        let target_capture_summary = replay_target_capture_summary(
            phenotype,
            &replayed_batch,
            &source_parts,
            candidate_profile.target_family,
        );
        let family_activation_summary = decoder_family_activation_summary(phenotype, &source_parts);
        return Err(GpuEvidenceError::ContractDetail(format!(
            "Slice B replay gate failed: replay_swaps={}, zero_swaps={}, replay_fast_l1={}, zero_fast_l1={}, post_logit_delta={}, tolerance={}, nonzero_capture=[{}], target_capture=[{}], family_activation=[{}]",
            replayed_commit.generation_swaps,
            zero_commit.generation_swaps,
            replayed_commit.replay_induced_fast_l1,
            zero_commit.replay_induced_fast_l1,
            post_delta,
            GPU_SLICE_B_TOLERANCE,
            capture_summary,
            target_capture_summary,
            family_activation_summary,
        )));
    }
    drop(replayed_backend);
    drop(zero_backend);

    let restored_backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())?;
    let mut restored = evidence_stage(
        "construct live runtime from submitted checkpoint",
        GpuLiveBrainRuntime::restore_with_checkpoints(
            restored_backend,
            world,
            options.deterministic_seed,
            tier,
            &store,
            &manifest,
            std::slice::from_ref(&submitted_write.save_state),
        ),
    )?;
    let restored_hardware = restored.hardware_receipt().clone();
    if adapter_identity_digest(&hardware) != adapter_identity_digest(&restored_hardware) {
        return Err(GpuEvidenceError::Contract(
            "Submitted restore used a different adapter",
        ));
    }
    let dispatches_before = restored.evidence_completed_dispatch_count();
    let mut actions_while_non_awake = 0_u32;
    let mut phase_sequence = vec![SleepPhase::Consolidating.raw()];
    let mut reached_awake = false;
    for _ in 0..GPU_SLICE_B_MAX_WAKE_TICKS {
        let before = restored.evidence_sleep_state(organism_id)?;
        if before.phase == SleepPhase::Awake {
            reached_awake = true;
            break;
        }
        let summaries = evidence_stage("advance restored submitted sleep", restored.tick())?;
        actions_while_non_awake =
            actions_while_non_awake.saturating_add(count_non_idle_actions(&summaries));
        let after = restored.evidence_sleep_state(organism_id)?;
        if phase_sequence.last().copied() != Some(after.phase.raw()) {
            phase_sequence.push(after.phase.raw());
        }
    }
    if !reached_awake
        || restored.evidence_completed_dispatch_count() != dispatches_before
        || actions_while_non_awake != 0
    {
        return Err(GpuEvidenceError::Contract(
            "Submitted restore did not wake without neural actions",
        ));
    }
    if phase_sequence.last().copied() != Some(SleepPhase::Awake.raw()) {
        phase_sequence.push(SleepPhase::Awake.raw());
    }
    let restored_awake = evidence_stage(
        "capture restored awake checkpoint",
        restored.checkpoint_brain(organism_id, &store),
    )?;
    let output_generation = restored_awake.save_state.active_weight_generation;
    let actual_remaining_swaps = output_generation
        .checked_sub(submitted_write.save_state.active_weight_generation)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(GpuEvidenceError::Contract(
            "restored sleep generation moved backwards",
        ))?;
    let duplicate_check = evidence_stage(
        "capture duplicate-swap checkpoint",
        restored.checkpoint_brain(organism_id, &store),
    )?;
    let duplicate_swaps = duplicate_check
        .save_state
        .active_weight_generation
        .saturating_sub(output_generation) as u32;
    let exact_output_assets = restored_awake.save_state.lifetime_weights
        == expected_awake.save_state.lifetime_weights
        && restored_awake.save_state.fast_weights == expected_awake.save_state.fast_weights
        && restored_awake.save_state.immutable_phenotype
            == submitted_write.save_state.immutable_phenotype;
    let genetic_digest = synapse_payload_digest(phenotype)?;
    let restore = GpuSleepRestoreEvidence {
        checkpoint_phase_raw: submitted_state.phase.raw(),
        consolidation_state_raw: submitted_state.consolidation.kind_raw(),
        cycle_id: 1,
        input_generation: submitted_write.save_state.active_weight_generation,
        output_generation,
        expected_remaining_swaps: 1,
        actual_remaining_swaps,
        duplicate_swaps,
        actions_while_non_awake,
        save_asset_digest,
        genetic_digest_before: genetic_digest,
        genetic_digest_after: genetic_digest,
        retained_target_delta: if exact_output_assets { post_delta } else { 0.0 },
        tolerance: GPU_SLICE_B_TOLERANCE,
        reached_awake,
        passed: reached_awake
            && exact_output_assets
            && actual_remaining_swaps == 1
            && duplicate_swaps == 0
            && actions_while_non_awake == 0
            && post_delta > GPU_SLICE_B_TOLERANCE,
    };
    Ok(ReplayRestoreEvidence {
        hardware,
        sleep_transition_sequence: phase_sequence,
        consolidation_dispatches: replayed_commit.generation_swaps,
        post_wake_retained_delta: post_delta,
        replay_event_count: u32::try_from(replayed_batch.events.len()).unwrap_or(u32::MAX),
        replay_sample_count: u32::try_from(replayed_batch.eligibility_samples.len())
            .unwrap_or(u32::MAX),
        replay_induced_fast_l1: replayed_commit.replay_induced_fast_l1,
        replay_vs_zero_sample_post_wake_delta: post_delta,
        learning_dispatches,
        restore,
    })
}

fn replay_nonzero_capture_summary(
    phenotype: &BrainPhenotype,
    replay: &BoundedReplayBatch,
) -> String {
    let mut recurrent = phenotype
        .synapses()
        .iter()
        .filter(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
        .collect::<Vec<_>>();
    recurrent.sort_unstable_by_key(|synapse| {
        (synapse.target(), synapse.source(), synapse.route_index())
    });
    let mut decoders = phenotype
        .synapses()
        .iter()
        .filter(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Decoder(_)))
        .collect::<Vec<_>>();
    decoders.sort_unstable_by_key(|synapse| match synapse.kind() {
        CompiledSynapseKind::Decoder(coordinate) => (
            coordinate.head().raw(),
            coordinate.family().raw(),
            coordinate.input_lane(),
            coordinate.motor_index(),
            synapse.source(),
            synapse.target(),
        ),
        CompiledSynapseKind::Recurrent => unreachable!("decoder filter is exact"),
    });
    let executable_order = recurrent.into_iter().chain(decoders).collect::<Vec<_>>();
    replay
        .synapse_spans
        .iter()
        .filter_map(|span| {
            let start = usize::try_from(span.sample_start).ok()?;
            let end = usize::try_from(span.sample_start.checked_add(span.sample_count)?).ok()?;
            let samples = replay.eligibility_samples.get(start..end)?;
            let nonzero = samples
                .iter()
                .filter(|sample| sample.eligibility_q15 != 0)
                .count();
            if nonzero == 0 {
                return None;
            }
            let signed_sum = samples
                .iter()
                .map(|sample| i64::from(sample.eligibility_q15))
                .sum::<i64>();
            let max_abs = samples
                .iter()
                .map(|sample| i32::from(sample.eligibility_q15).unsigned_abs())
                .max()
                .unwrap_or(0);
            let synapse = executable_order.get(span.local_synapse_id as usize)?;
            let receptor = phenotype
                .plasticity_receptors()
                .get(usize::from(synapse.receptor_index()))?;
            let identity = match synapse.kind() {
                CompiledSynapseKind::Recurrent => format!(
                    "recurrent(route={},source={},target={})",
                    synapse.route_index(),
                    synapse.source(),
                    synapse.target()
                ),
                CompiledSynapseKind::Decoder(coordinate) => format!(
                    "decoder(head={},family={},lane={},motor={})",
                    coordinate.head().raw(),
                    coordinate.family().raw(),
                    coordinate.input_lane(),
                    coordinate.motor_index()
                ),
            };
            Some(format!(
                "id={}:{}:nonzero={}:sum={}:max_abs={}:alpha={}:replay_rate={}",
                span.local_synapse_id,
                identity,
                nonzero,
                signed_sum,
                max_abs,
                synapse.alpha(),
                receptor.sleep_replay_rate()
            ))
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn replay_target_capture_summary(
    phenotype: &BrainPhenotype,
    replay: &BoundedReplayBatch,
    checkpoint: &GpuBrainCheckpointParts,
    target_family: CandidateActionFamily,
) -> String {
    let mut recurrent = phenotype
        .synapses()
        .iter()
        .filter(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
        .collect::<Vec<_>>();
    recurrent.sort_unstable_by_key(|synapse| {
        (synapse.target(), synapse.source(), synapse.route_index())
    });
    let mut decoders = phenotype
        .synapses()
        .iter()
        .filter(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Decoder(_)))
        .collect::<Vec<_>>();
    decoders.sort_unstable_by_key(|synapse| match synapse.kind() {
        CompiledSynapseKind::Decoder(coordinate) => (
            coordinate.head().raw(),
            coordinate.family().raw(),
            coordinate.input_lane(),
            coordinate.motor_index(),
            synapse.source(),
            synapse.target(),
        ),
        CompiledSynapseKind::Recurrent => unreachable!("decoder filter is exact"),
    });
    let executable_order = recurrent.into_iter().chain(decoders).collect::<Vec<_>>();
    let activation_bits = if checkpoint.active_activation_side == 0 {
        &checkpoint.activation_a_bits
    } else {
        &checkpoint.activation_b_bits
    };

    replay
        .synapse_spans
        .iter()
        .filter_map(|span| {
            let synapse = executable_order.get(span.local_synapse_id as usize)?;
            let CompiledSynapseKind::Decoder(coordinate) = synapse.kind() else {
                return None;
            };
            if coordinate.head() != DecoderHeadKind::ActionCandidate
                || coordinate.family() != target_family
            {
                return None;
            }
            let start = usize::try_from(span.sample_start).ok()?;
            let end = usize::try_from(span.sample_start.checked_add(span.sample_count)?).ok()?;
            let samples = replay.eligibility_samples.get(start..end)?;
            let nonzero = samples
                .iter()
                .filter(|sample| sample.eligibility_q15 != 0)
                .count();
            let activation = activation_bits
                .get(synapse.source() as usize)
                .copied()
                .map(f32::from_bits)?;
            Some(format!(
                "id={}:lane={}:motor={}:source={}:activation={}:nonzero={}",
                span.local_synapse_id,
                coordinate.input_lane(),
                coordinate.motor_index(),
                synapse.source(),
                activation,
                nonzero
            ))
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn decoder_family_activation_summary(
    phenotype: &BrainPhenotype,
    checkpoint: &GpuBrainCheckpointParts,
) -> String {
    let activation_bits = if checkpoint.active_activation_side == 0 {
        &checkpoint.activation_a_bits
    } else {
        &checkpoint.activation_b_bits
    };
    let mut maxima = [(0.0_f32, 0_u16, 0_u32); 8];
    for synapse in phenotype.synapses() {
        let CompiledSynapseKind::Decoder(coordinate) = synapse.kind() else {
            continue;
        };
        if coordinate.head() != DecoderHeadKind::ActionCandidate {
            continue;
        }
        let Some(activation) = activation_bits
            .get(synapse.source() as usize)
            .copied()
            .map(f32::from_bits)
        else {
            continue;
        };
        let slot = &mut maxima[usize::from(coordinate.family().raw())];
        if activation.abs() > slot.0.abs() {
            *slot = (activation, coordinate.motor_index(), synapse.source());
        }
    }
    maxima
        .iter()
        .enumerate()
        .map(|(family, (activation, motor, source))| {
            format!(
                "family={}:max_activation={}:motor={}:source={}",
                family, activation, motor, source
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn evidence_stage<T, E>(stage: &'static str, result: Result<T, E>) -> Result<T, GpuEvidenceError>
where
    E: std::error::Error + Send + Sync + 'static,
{
    result.map_err(|source| GpuEvidenceError::Stage {
        stage,
        source: Box::new(source),
    })
}

fn select_evidence_candidate_profile(
    phenotype: &BrainPhenotype,
) -> Result<EvidenceCandidateProfile, GpuEvidenceError> {
    const SELECTOR_TICKS: u64 = 8;
    const MIN_CAPTURED_ACTIVATION: f32 = 1.0e-4;

    let organism_id = OrganismId(10);
    let mut backend = evidence_stage(
        "create candidate-selector GPU backend",
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1()),
    )?;
    let adapter_identity = adapter_identity_digest(backend.hardware_receipt());
    let handle = evidence_stage(
        "insert candidate-selector brain",
        backend.insert_brain(organism_id, phenotype.clone()),
    )?;
    for tick in 0..SELECTOR_TICKS {
        let frame = evidence_frame_for_family(
            organism_id,
            Tick::new(tick),
            CandidateActionFamily::Idle,
            0,
        )?;
        let ticks = evidence_stage(
            "dispatch candidate-selector stimulus",
            backend.tick_batch(&[(handle, frame)]),
        )?;
        discard_probe_ticks(&mut backend, &ticks)?;
    }
    let checkpoint = evidence_stage(
        "capture candidate-selector activation checkpoint",
        backend.snapshot_brain(handle, Tick::new(SELECTOR_TICKS)),
    )?
    .into_parts();
    let activation_bits = if checkpoint.active_activation_side == 0 {
        &checkpoint.activation_a_bits
    } else {
        &checkpoint.activation_b_bits
    };
    let mut maxima = [0.0_f32; 8];
    let mut captured = [false; 8];
    for canonical_id in phenotype
        .replay_capture_plan()
        .global_synapse_ids()
        .iter()
        .copied()
    {
        let Some(synapse) = phenotype.synapses().get(canonical_id as usize) else {
            return Err(GpuEvidenceError::Contract(
                "replay capture plan referenced a missing canonical synapse",
            ));
        };
        let CompiledSynapseKind::Decoder(coordinate) = synapse.kind() else {
            continue;
        };
        if coordinate.head() != DecoderHeadKind::ActionCandidate || coordinate.input_lane() != 0 {
            continue;
        }
        let activation = activation_bits
            .get(synapse.source() as usize)
            .copied()
            .map(f32::from_bits)
            .ok_or(GpuEvidenceError::Contract(
                "candidate-selector source neuron was outside the checkpoint",
            ))?;
        if !activation.is_finite() {
            return Err(GpuEvidenceError::Contract(
                "candidate-selector checkpoint contained a non-finite activation",
            ));
        }
        let family_index = usize::from(coordinate.family().raw());
        captured[family_index] = true;
        maxima[family_index] = maxima[family_index].max(activation.abs());
    }
    let mut active_families = (0_u8..8)
        .filter(|raw| {
            let index = usize::from(*raw);
            captured[index] && maxima[index] > MIN_CAPTURED_ACTIVATION
        })
        .map(|raw| {
            CandidateActionFamily::try_from_raw(raw)
                .map(|family| (family, maxima[usize::from(raw)]))
        })
        .collect::<Result<Vec<_>, _>>()?;
    active_families.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.raw().cmp(&right.0.raw()))
    });
    let (target_family, _) = active_families.first().copied().ok_or_else(|| {
        GpuEvidenceError::ContractDetail(format!(
            "no replay-captured action decoder exceeded the activation threshold: maxima={maxima:?}, captured={captured:?}"
        ))
    })?;
    let unrelated_family = active_families
        .iter()
        .map(|(family, _)| *family)
        .find(|family| *family != target_family)
        .or_else(|| {
            (0_u8..8)
                .filter(|raw| captured[usize::from(*raw)])
                .filter_map(|raw| CandidateActionFamily::try_from_raw(raw).ok())
                .find(|family| *family != target_family)
        })
        .or_else(|| {
            (0_u8..8)
                .filter_map(|raw| CandidateActionFamily::try_from_raw(raw).ok())
                .find(|family| *family != target_family)
        })
        .ok_or(GpuEvidenceError::Contract(
            "candidate-selector could not choose a distinct unrelated family",
        ))?;
    Ok(EvidenceCandidateProfile {
        target_family,
        unrelated_family,
        adapter_identity,
    })
}

#[derive(Debug, Clone, Copy)]
enum EvidenceCandidate {
    Target,
    Unrelated,
}

fn evidence_frame(
    organism_id: OrganismId,
    tick: Tick,
    candidate_kind: EvidenceCandidate,
    profile: EvidenceCandidateProfile,
) -> Result<PerceptionFrame, ScaffoldContractError> {
    let (family, feature_lane) = match candidate_kind {
        EvidenceCandidate::Target => (profile.target_family, 0),
        EvidenceCandidate::Unrelated => (profile.unrelated_family, 7),
    };
    evidence_frame_for_family(organism_id, tick, family, feature_lane)
}

fn evidence_frame_for_family(
    organism_id: OrganismId,
    tick: Tick,
    family: CandidateActionFamily,
    feature_lane: usize,
) -> Result<PerceptionFrame, ScaffoldContractError> {
    let mut channels = SensoryChannels::ZERO;
    for (index, value) in channels.visual_affordance.iter_mut().enumerate() {
        *value = 0.2 + (index % 5) as f32 * 0.1;
    }
    for (index, value) in channels.auditory_acoustic.iter_mut().enumerate() {
        *value = 0.25 + (index % 4) as f32 * 0.1;
    }
    for (index, value) in channels.smell_chemistry.iter_mut().enumerate() {
        *value = 0.3 + (index % 3) as f32 * 0.1;
    }
    for (index, value) in channels.tactile_contact.iter_mut().enumerate() {
        *value = 0.35 + (index % 2) as f32 * 0.1;
    }
    channels.pain_signal = NormalizedScalar::new(0.1)?;
    channels.novelty_signal = NormalizedScalar::new(0.6)?;
    let sensory =
        SensorySnapshot::new(organism_id, tick, Vec3f::ZERO, channels, Default::default())?;
    let kind = match family {
        CandidateActionFamily::Idle => ActionKind::Idle,
        CandidateActionFamily::Rest => ActionKind::Rest,
        CandidateActionFamily::Inspect => ActionKind::Inspect,
        CandidateActionFamily::Approach | CandidateActionFamily::Avoid => ActionKind::Move,
        CandidateActionFamily::Contact | CandidateActionFamily::Ingest => ActionKind::Interact,
        CandidateActionFamily::Other => ActionKind::Hold,
    };
    let mut features = CandidateFeatureVector::zero();
    features.0[feature_lane] = 0.75;
    let candidate = ActionCandidate::new(
        0,
        kind.canonical_id(),
        kind,
        family,
        CandidateObservationRef::None,
        ActionTarget::new(None, Some(Vec3f::new(1.0, 0.0, 0.0))),
        features,
        Confidence::new(0.8)?,
        NormalizedScalar::new(0.1)?,
        DurationTicks::new(1),
        DurationTicks::new(1),
    )?;
    PerceptionFrame::new(
        organism_id,
        tick,
        EVIDENCE_SENSOR_PROFILE,
        sensory,
        BodySnapshot {
            pose: Pose {
                translation: Vec3f::new(0.25, -0.5, 0.75),
                ..Pose::IDENTITY
            },
            velocity: Velocity {
                linear: Vec3f::new(0.2, -0.1, 0.3),
                angular: Vec3f::new(-0.15, 0.25, 0.05),
            },
        },
        HomeostaticSnapshot::baseline(tick),
        vec![candidate],
        SensorProfileProvenance::new(EVIDENCE_SENSOR_PROFILE, SensoryAbiVersion::CURRENT, tick)?,
        Vec::new(),
    )
}

#[allow(clippy::too_many_arguments)]
fn sealed_credit_patch(
    handle: GpuBrainHandle,
    genome: &BrainGenome,
    development: &DevelopmentState,
    frame: &PerceptionFrame,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    sequence_raw: u64,
    reward: f32,
    pain: f32,
) -> Result<ExperiencePatch, ScaffoldContractError> {
    let sequence_id = ExperienceSequenceId(sequence_raw);
    let selection = NeuralActionSelection {
        candidate_index: tick.selection.candidate_index,
        logit: tick.selection.logit,
        confidence: tick.selection.confidence,
        active_tiles: tick.selection.active_tiles,
        active_synapses: tick.selection.active_synapses,
    };
    let candidate = frame.candidates()[usize::from(selection.candidate_index)];
    let command = candidate.to_command(handle.organism_id(), selection.confidence)?;
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence_id,
        handle.class_id(),
        handle.phenotype_hash(),
        genome.id,
        genome.schema_version,
        development.clone(),
        frame.clone(),
    )?;
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id,
        handle.phenotype_hash(),
        tick.dispatch_generation,
        tick.active_activation_side,
        frame,
        selection,
        command,
    )?;
    let outcome = PostActionOutcome::new(
        handle.organism_id(),
        sequence_id,
        Tick::new(frame.tick().raw().saturating_add(1)),
        reward > 0.0 && pain == 0.0,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.0)?,
        },
        HomeostaticDelta {
            drives: alife_core::DriveDelta::zero(),
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(reward)?,
        NormalizedScalar::new(0.0)?,
        NormalizedScalar::new(pain)?,
        SignedValence::new(0.0)?,
        NormalizedScalar::new(0.0)?,
    )?;
    ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre_action)?
        .record_decision(decision)?
        .record_outcome(outcome)?
        .seal()
}

fn require_matching_initial_logits(
    ticks: &[alife_gpu_backend::GpuClosedLoopTick],
) -> Result<(), GpuEvidenceError> {
    if ticks.len() != 2
        || (ticks[0].selection.logit - ticks[1].selection.logit).abs() > GPU_SLICE_B_TOLERANCE
    {
        return Err(GpuEvidenceError::Contract(
            "paired Slice B brains did not begin from equal logits",
        ));
    }
    Ok(())
}

fn discard_probe_ticks(
    backend: &mut GpuClosedLoopBackend,
    ticks: &[alife_gpu_backend::GpuClosedLoopTick],
) -> Result<(), GpuEvidenceError> {
    for tick in ticks {
        backend.discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())?;
    }
    Ok(())
}

fn submitted_sleep_state(
    checkpoint_tick: Tick,
    request: alife_core::GpuConsolidationRequest,
    job_id: alife_core::ConsolidationJobId,
) -> Result<SleepState, ScaffoldContractError> {
    let state = SleepState {
        schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
        phase: SleepPhase::Consolidating,
        phase_started_tick: checkpoint_tick,
        entered_sleep_tick: Some(checkpoint_tick),
        cycles_completed: 0,
        last_trigger: Some(SleepTrigger::FatigueThreshold),
        active_cycle_id: request.cycle_id,
        last_consolidated_cycle_id: 0,
        consolidation: ConsolidationState::Submitted { request, job_id },
    };
    state.validate_contract()?;
    Ok(state)
}

fn evidence_world(
    seed: u64,
    organism_id: OrganismId,
    tick: Tick,
) -> Result<HeadlessWorld, ScaffoldContractError> {
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("slice-b-learner", organism_id, Vec3f::ZERO)
        .food("slice-b-food", Vec3f::new(1.0, 0.0, 0.0), 0.9)
        .hazard("slice-b-hazard", Vec3f::new(-2.0, 0.0, 0.0), 0.7)
        .build()?;
    for _ in 0..tick.raw() {
        world.advance_tick();
    }
    Ok(world)
}

fn build_checkpoint_save(
    options: &GpuLearningSleepAcceptanceOptions,
    tier: BrainScaleTier,
    phenotype: &BrainPhenotype,
    genome: &BrainGenome,
    world: &HeadlessWorld,
    assets: AssetManifest,
    checkpoint: GpuBrainSaveState,
) -> Result<PortableSaveFile, GpuEvidenceError> {
    let tick = world.tick();
    let creature = CreatureSaveState {
        organism_id: checkpoint.organism_id,
        genome_id: genome.id,
        brain_class: tier,
        development_tick: tick,
        appearance: CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick,
            homeostasis: HomeostaticSnapshot::baseline(tick),
            memory_record_count: 0,
            memory_source_ids: Vec::new(),
            concept_count: 0,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "Consolidating".to_string(),
            diagnostics: Vec::new(),
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest: PortableAssetDigest::for_bytes(&serde_json::to_vec(phenotype)?).0,
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: 0,
            h_operational_entries: 0,
            h_shadow_entries: 0,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: true,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: None,
        },
        gpu_brain: Some(checkpoint),
    };
    let mut config = RuntimeConfig::deterministic_default(options.deterministic_seed, tier);
    config.features.gpu_backend_enabled = true;
    let save = PortableSaveFile::from_headless_world(
        format!(
            "gpu-learning-sleep-slice-b-{}",
            capacity_slug(options.capacity.id())?
        ),
        world,
        config,
        assets,
        vec![creature],
    )?;
    Ok(save)
}

fn validate_checkpoint_payloads(
    store: &GpuCheckpointAssetStore,
    save: &PortableSaveFile,
) -> Result<(), GpuEvidenceError> {
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())?;
    for state in save
        .creatures
        .iter()
        .filter_map(|creature| creature.gpu_brain.as_ref())
    {
        let restored = store.restore_brain(&mut backend, &save.assets, state)?;
        backend.remove_brain(restored.receipt.handle)?;
    }
    Ok(())
}

pub fn canonical_gpu_save_asset_digest(
    save: &PortableSaveFile,
    asset_root: impl AsRef<Path>,
) -> Result<[u64; 4], GpuEvidenceError> {
    let asset_root = asset_root.as_ref();
    save.validate_with_asset_root(asset_root)?;
    let mut digest = CanonicalDigestBuilder::new(SAVE_ASSET_DOMAIN);
    let save_bytes = serde_json::to_vec(save)?;
    digest.write_bytes(&save_bytes);
    let mut count = 0_usize;
    for (creature_index, creature) in save.creatures.iter().enumerate() {
        let Some(state) = creature.gpu_brain.as_ref() else {
            continue;
        };
        for (field, asset) in checkpoint_asset_refs(state) {
            count = count.saturating_add(1);
            write_save_asset_binding(
                &mut digest,
                save,
                asset_root,
                &format!("creatures[{creature_index}].gpu_brain.{field}"),
                asset,
            )?;
        }
    }
    digest.write_sequence_len(count);
    if count == 0 {
        return Err(GpuEvidenceError::Contract(
            "Slice B save contains no GPU checkpoint assets",
        ));
    }
    Ok(digest.finish256())
}

fn checkpoint_asset_refs(state: &GpuBrainSaveState) -> Vec<(&'static str, &GpuBrainAssetRef)> {
    let mut refs = vec![
        ("immutable_phenotype", &state.immutable_phenotype),
        (
            "phenotype_compiler_inputs",
            &state.phenotype_compiler_inputs,
        ),
        ("activation_state", &state.activation_state),
        ("neuron_homeostasis", &state.neuron_homeostasis),
        ("lifetime_weights", &state.lifetime_weights),
        ("fast_weights", &state.fast_weights),
        ("eligibility", &state.eligibility),
        ("replay_journal", &state.replay_journal),
    ];
    if let Some(asset) = state.pending_experience_transaction.as_ref() {
        refs.push(("pending_experience_transaction", asset));
    }
    for (field, asset) in [
        (
            "sleep_assets.replay_batch",
            state.sleep_assets.replay_batch.as_ref(),
        ),
        (
            "sleep_assets.lifetime_staging",
            state.sleep_assets.lifetime_staging.as_ref(),
        ),
        (
            "sleep_assets.fast_staging",
            state.sleep_assets.fast_staging.as_ref(),
        ),
        (
            "sleep_assets.eligibility_staging",
            state.sleep_assets.eligibility_staging.as_ref(),
        ),
        (
            "sleep_assets.replay_journal_staging",
            state.sleep_assets.replay_journal_staging.as_ref(),
        ),
    ] {
        if let Some(asset) = asset {
            refs.push((field, asset));
        }
    }
    refs
}

fn write_save_asset_binding(
    digest: &mut CanonicalDigestBuilder,
    save: &PortableSaveFile,
    asset_root: &Path,
    field_path: &str,
    asset: &GpuBrainAssetRef,
) -> Result<(), GpuEvidenceError> {
    let entry = save
        .assets
        .entries
        .iter()
        .find(|entry| entry.asset_id == asset.asset_id)
        .ok_or(GpuEvidenceError::Contract(
            "GPU checkpoint ref is absent from its manifest",
        ))?;
    if entry.digest != asset.digest {
        return Err(GpuEvidenceError::Contract(
            "GPU checkpoint ref digest disagrees with its manifest",
        ));
    }
    let path = asset_root.join(&entry.relative_path);
    let bytes = fs::read(&path)?;
    if PortableAssetDigest::for_file(&path)? != entry.digest {
        return Err(GpuEvidenceError::Contract(
            "GPU checkpoint asset bytes disagree with their declared digest",
        ));
    }
    let decoded: serde_json::Value = serde_json::from_slice(&bytes)?;
    let canonical_decoded = serde_json::to_vec(&decoded)?;
    digest.write_utf8(field_path);
    digest.write_utf8(&asset.asset_id);
    digest.write_utf8(&entry.digest.0);
    digest.write_u16(entry.schema_version);
    digest.write_bytes(&canonical_decoded);
    Ok(())
}

fn validate_save_asset_corruption_matrix(
    save: &PortableSaveFile,
    asset_root: &Path,
    expected_digest: [u64; 4],
) -> Result<(), GpuEvidenceError> {
    let bindings = save
        .creatures
        .iter()
        .enumerate()
        .flat_map(|(creature_index, creature)| {
            creature
                .gpu_brain
                .as_ref()
                .into_iter()
                .flat_map(move |state| {
                    checkpoint_asset_refs(state)
                        .into_iter()
                        .map(move |(field, asset)| {
                            (
                                format!("creatures[{creature_index}].gpu_brain.{field}"),
                                asset.clone(),
                            )
                        })
                })
        })
        .collect::<Vec<_>>();
    if bindings.is_empty() {
        return Err(GpuEvidenceError::Contract(
            "save corruption matrix contains no GPU checkpoint refs",
        ));
    }
    for (field_path, asset) in bindings {
        let mut missing = save.clone();
        let manifest_len = missing.assets.entries.len();
        missing
            .assets
            .entries
            .retain(|entry| entry.asset_id != asset.asset_id);
        if missing.assets.entries.len() == manifest_len {
            return Err(GpuEvidenceError::ContractDetail(format!(
                "save corruption matrix could not remove manifest ref for {field_path}"
            )));
        }
        if canonical_gpu_save_asset_digest(&missing, asset_root).is_ok() {
            return Err(GpuEvidenceError::ContractDetail(format!(
                "save corruption matrix accepted a missing ref for {field_path}"
            )));
        }

        let entry = save
            .assets
            .entries
            .iter()
            .find(|entry| entry.asset_id == asset.asset_id)
            .ok_or_else(|| {
                GpuEvidenceError::ContractDetail(format!(
                    "save corruption matrix could not resolve {field_path}"
                ))
            })?;
        let path = asset_root.join(&entry.relative_path);
        let original = fs::read(&path)?;
        if original.is_empty() {
            return Err(GpuEvidenceError::ContractDetail(format!(
                "save corruption matrix found an empty asset for {field_path}"
            )));
        }
        let mut corrupted = original.clone();
        corrupted[0] ^= 0x01;
        fs::write(&path, &corrupted)?;
        let corruption_result = canonical_gpu_save_asset_digest(save, asset_root);
        fs::write(&path, &original)?;
        if corruption_result.is_ok() {
            return Err(GpuEvidenceError::ContractDetail(format!(
                "save corruption matrix accepted changed bytes for {field_path}"
            )));
        }
    }
    let untouched_digest = canonical_gpu_save_asset_digest(save, asset_root)?;
    if untouched_digest != expected_digest {
        return Err(GpuEvidenceError::Contract(
            "save corruption matrix did not restore the exact untouched fixture",
        ));
    }
    Ok(())
}

fn count_non_idle_actions(summaries: &[LiveBrainTickSummary]) -> u32 {
    summaries
        .iter()
        .filter(|summary| summary.selected_action_id.is_some() || summary.patch_sealed)
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

fn encode_restore(
    digest: &mut CanonicalDigestBuilder,
    restore: &GpuSleepRestoreEvidence,
) -> Result<(), ScaffoldContractError> {
    digest.write_u16(restore.checkpoint_phase_raw);
    digest.write_u16(restore.consolidation_state_raw);
    digest.write_u64(restore.cycle_id);
    digest.write_u64(restore.input_generation);
    digest.write_u64(restore.output_generation);
    digest.write_u32(restore.expected_remaining_swaps);
    digest.write_u32(restore.actual_remaining_swaps);
    digest.write_u32(restore.duplicate_swaps);
    digest.write_u32(restore.actions_while_non_awake);
    write_digest4(digest, restore.save_asset_digest);
    write_digest4(digest, restore.genetic_digest_before);
    write_digest4(digest, restore.genetic_digest_after);
    digest.write_f32(restore.retained_target_delta)?;
    digest.write_f32(restore.tolerance)?;
    digest.write_bool(restore.reached_awake);
    digest.write_bool(restore.passed);
    Ok(())
}

fn validate_slice_b_output_filename(
    path: &Path,
    class_id: BrainClassId,
) -> Result<(), GpuEvidenceError> {
    let expected = format!(
        "gpu-learning-sleep-slice-b-{}.json",
        capacity_slug(class_id)?
    );
    if path.file_name().and_then(|name| name.to_str()) != Some(expected.as_str()) {
        return Err(GpuEvidenceError::Contract(
            "Slice B output must use its exact class-qualified artifact filename",
        ));
    }
    Ok(())
}

fn all_finite(values: &[f32]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn approximately_equal(left: f32, right: f32, tolerance: f32) -> bool {
    (left - right).abs() <= tolerance
}

struct TemporaryEvidenceRoot {
    path: PathBuf,
}

impl TemporaryEvidenceRoot {
    fn new(label: &str) -> Result<Self, GpuEvidenceError> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| GpuEvidenceError::Contract("system clock predates the Unix epoch"))?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "alife-gpu-evidence-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryEvidenceRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

impl From<alife_world::persistence::PersistenceError> for GpuEvidenceError {
    fn from(error: alife_world::persistence::PersistenceError) -> Self {
        GpuEvidenceError::App(GameAppShellError::Persistence(error))
    }
}
