//! Real-Vulkan Slice C acceptance runner over the production GPU runtime.

use std::{collections::BTreeMap, path::PathBuf};

use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, BrainScaleTier, CandidateActionFamily,
    CanonicalDigestBuilder, DecisionSnapshot, DevelopmentState, ExperiencePatch,
    ExperiencePatchBuilder, ExperienceSequenceId, FinalizedMemoryRecall, HomeostaticSnapshot,
    MemoryBankConfig, MemorySidecarState, NeuralActionSelection, OrganismId, PerceptionFrame,
    PerceptionFrameDraft, PolicyBackend, PostActionOutcome, PreActionSnapshot, SensorProfile,
    SensorProfileIdentity, SensoryAbiVersion, Tick, TopologyCounts, Validate, Vec3f, WorldEntityId,
};
use alife_gpu_backend::{
    GpuBrainCheckpointParts, GpuBrainCheckpointSnapshot, GpuBrainRestoreRequest,
    GpuCandidateLogitEvidenceSnapshot, GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput,
    GpuClosedLoopMemoryTickInput, GpuHardwareReceipt,
};
use alife_world::{
    initial_tracked_object_id, GroundedPhysicalProperties, HeadlessScenarioBuilder, HeadlessWorld,
    WorldEditorSpawnSpec, WorldObjectKind,
};

use crate::{compile_gpu_birth_components, GpuLiveBrainRuntime};

use super::super::{
    atomic_write_receipt, capacity_slug, load_gpu_slice_c_evidence, read_git_provenance,
    tier_for_capacity, GitProvenance, GpuEvidenceError, GpuSliceEvidenceHeader,
    PhenotypeEvidenceManifest, GPU_EVIDENCE_PASSING_STATUS_RAW, GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
};
use super::{
    slice_c_artifact_slug, CapacitySaturationEvidence, GpuMemoryGroundingEvidenceReceipt,
    MemoryContextProbeEvidence, ProfiledBehaviorReceiptHeader, TopologyCapacityReceipt,
    GPU_EVIDENCE_BACKEND_API_SLUG, GPU_EVIDENCE_BACKEND_API_VERSION, GPU_SLICE_C_RAW,
};

const EVIDENCE_ORGANISM: OrganismId = OrganismId(1);
const GROUNDED_ACCEPTANCE_TICKS: u64 = 10_240;
const PRIVILEGED_ACCEPTANCE_TICKS: u64 = 64;
const MEMORY_CAPACITY: usize = 64;
const MEMORY_MAX_FEATURE_LEN: usize = 64;
const MEMORY_MAX_MATCH_COUNT: usize = 4;
const MEMORY_MIN_MATCH_SCORE: f32 = 0.72;
const ACCEPTANCE_TOLERANCE: f32 = 1.0e-5;
const CYAN_POSITION: Vec3f = Vec3f {
    x: 0.0,
    y: -0.35,
    z: 0.0,
};
const AMBER_POSITION: Vec3f = Vec3f {
    x: -0.35,
    y: 0.0,
    z: 0.0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuMemoryGroundingAcceptanceOptions {
    pub capacity: BrainCapacityClass,
    pub requested_ticks: u64,
    pub deterministic_seed: u64,
    pub sensor_profile: SensorProfile,
}

impl GpuMemoryGroundingAcceptanceOptions {
    fn validate(self) -> Result<Self, GpuEvidenceError> {
        self.capacity.validate_contract()?;
        if self.deterministic_seed == 0 {
            return Err(GpuEvidenceError::Contract(
                "Slice C deterministic seed must be nonzero",
            ));
        }
        let expected_ticks = match self.sensor_profile {
            SensorProfile::GroundedObjectSlotsV1 => GROUNDED_ACCEPTANCE_TICKS,
            SensorProfile::PrivilegedAffordanceV1 => PRIVILEGED_ACCEPTANCE_TICKS,
        };
        if self.requested_ticks != expected_ticks {
            return Err(GpuEvidenceError::Contract(
                "Slice C tick count does not match the selected sensor profile",
            ));
        }
        capacity_slug(self.capacity.id())?;
        Ok(self)
    }

    pub fn artifact_slug(self) -> Result<String, GpuEvidenceError> {
        let options = self.validate()?;
        slice_c_artifact_slug(options.sensor_profile, options.capacity.id())
    }

    pub fn artifact_path(self) -> Result<PathBuf, GpuEvidenceError> {
        Ok(PathBuf::from("target")
            .join("artifacts")
            .join(format!("{}.json", self.artifact_slug()?)))
    }

    pub fn aggregate_key(self) -> Result<String, GpuEvidenceError> {
        Ok(format!(
            "{}:seed:{}",
            self.artifact_slug()?,
            self.deterministic_seed
        ))
    }
}

#[derive(Debug)]
struct BehaviorEvidence {
    hardware: GpuHardwareReceipt,
    poisoned_ingest_candidate: u16,
    post_learning_selection: u16,
    poisoned_ingest_logit_before: f32,
    poisoned_ingest_logit_after: f32,
    poisoned_avoid_logit_before: f32,
    poisoned_avoid_logit_after: f32,
    poisoned_ingest_delta: f32,
    safe_ingest_delta: f32,
    cyan_ingest_target_latent: [f32; 8],
    cyan_avoid_target_latent: [f32; 8],
    cyan_ingest_family_value: [f32; 4],
    cyan_avoid_family_value: [f32; 4],
    amber_target_latent: [f32; 8],
    memory_enabled: MemoryContextProbeEvidence,
    memory_ablated: MemoryContextProbeEvidence,
    max_compact_readback_bytes: u32,
}

#[derive(Debug)]
struct ProbeBranchEvidence {
    logits: GpuCandidateLogitEvidenceSnapshot,
    selected_candidate: u16,
    recurrent_activation_digest: [u64; 4],
    compact_readback_bytes: u32,
}

#[derive(Debug)]
struct SaturationRunEvidence {
    completed_waking_ticks: u64,
    gpu_selection_count: u64,
    compact_readback_bytes: u32,
    saturation: Option<CapacitySaturationEvidence>,
}

pub fn run_gpu_memory_grounding_acceptance(
    options: GpuMemoryGroundingAcceptanceOptions,
) -> Result<GpuMemoryGroundingEvidenceReceipt, GpuEvidenceError> {
    let provenance = read_git_provenance()?;
    run_gpu_memory_grounding_acceptance_with_provenance(options.validate()?, provenance)
}

pub fn run_and_write_gpu_memory_grounding_acceptance(
    options: GpuMemoryGroundingAcceptanceOptions,
) -> Result<GpuMemoryGroundingEvidenceReceipt, GpuEvidenceError> {
    let options = options.validate()?;
    let output = options.artifact_path()?;
    let before = read_git_provenance()?;
    if !before.clean {
        return Err(GpuEvidenceError::Git(
            "persistent Slice C evidence requires a clean committed worktree".to_string(),
        ));
    }
    let receipt = run_gpu_memory_grounding_acceptance_with_provenance(options, before.clone())?;
    let after = read_git_provenance()?;
    if before != after || !after.clean {
        return Err(GpuEvidenceError::Git(
            "source commit or tree changed during Slice C evidence capture".to_string(),
        ));
    }
    receipt.validate_in_memory()?;
    atomic_write_receipt(&output, &receipt)?;
    let loaded = load_gpu_slice_c_evidence(&output)?;
    if loaded != receipt {
        return Err(GpuEvidenceError::Contract(
            "persisted Slice C evidence changed during round trip",
        ));
    }
    Ok(loaded)
}

fn run_gpu_memory_grounding_acceptance_with_provenance(
    options: GpuMemoryGroundingAcceptanceOptions,
    provenance: GitProvenance,
) -> Result<GpuMemoryGroundingEvidenceReceipt, GpuEvidenceError> {
    let tier = tier_for_capacity(options.capacity.id())?;
    let (phenotype, genome, development) = compile_gpu_birth_components(
        options.deterministic_seed,
        tier,
        EVIDENCE_ORGANISM,
        Tick::ZERO,
        options.sensor_profile,
    )?;
    phenotype.validate_against(&options.capacity)?;
    let phenotype_manifest =
        PhenotypeEvidenceManifest::from_learning_phenotype(&phenotype, &options.capacity)?;

    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())?;
    let behavior = run_behavior_probe(
        &mut backend,
        options,
        &phenotype,
        &genome,
        &development,
        &phenotype_manifest,
    )?;
    validate_behavior_probe(&behavior, options.sensor_profile)?;
    let saturation = run_saturation_probe(backend, options, tier, &behavior.hardware)?;
    validate_saturation_run(
        &saturation,
        options.sensor_profile,
        &options.capacity,
        options.requested_ticks,
    )?;
    let compact_readback_bytes = behavior
        .max_compact_readback_bytes
        .max(saturation.compact_readback_bytes);
    let sensor_profile = profile_identity(options.sensor_profile);
    let mut receipt = GpuMemoryGroundingEvidenceReceipt {
        header: ProfiledBehaviorReceiptHeader {
            common: GpuSliceEvidenceHeader {
                artifact_schema: GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
                slice_raw: GPU_SLICE_C_RAW,
                class_id_raw: options.capacity.id().raw(),
                profile_id_raw: sensor_profile.profile_id.raw(),
                profile_schema: sensor_profile.profile_schema_version,
                status_raw: GPU_EVIDENCE_PASSING_STATUS_RAW,
                git_commit: provenance.commit,
                source_tree_digest: provenance.tree,
                artifact_digest: [0; 4],
                phenotype_hash: phenotype.phenotype_hash(),
                phenotype_manifest_digest: phenotype_manifest.manifest_digest,
                capacity_digest: options.capacity.canonical_digest(),
            },
            artifact_slug: options.artifact_slug()?,
            backend_api_version: GPU_EVIDENCE_BACKEND_API_VERSION,
            backend_api_slug: GPU_EVIDENCE_BACKEND_API_SLUG.to_string(),
            adapter_name: behavior.hardware.adapter_name.clone(),
            adapter_backend: behavior.hardware.backend_api.clone(),
            run_seed: options.deterministic_seed,
        },
        phenotype_manifest,
        sensor_profile,
        capacity_class_slug: capacity_slug(options.capacity.id())?.to_string(),
        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
        hardware: behavior.hardware,
        completed_ticks: options.requested_ticks,
        completed_waking_ticks: saturation.completed_waking_ticks,
        gpu_selection_count: saturation.gpu_selection_count,
        poisoned_ingest_candidate: behavior.poisoned_ingest_candidate,
        post_learning_selection: behavior.post_learning_selection,
        poisoned_ingest_logit_before: behavior.poisoned_ingest_logit_before,
        poisoned_ingest_logit_after: behavior.poisoned_ingest_logit_after,
        poisoned_avoid_logit_before: behavior.poisoned_avoid_logit_before,
        poisoned_avoid_logit_after: behavior.poisoned_avoid_logit_after,
        poisoned_ingest_delta: behavior.poisoned_ingest_delta,
        safe_ingest_delta: behavior.safe_ingest_delta,
        cyan_ingest_target_latent: behavior.cyan_ingest_target_latent,
        cyan_avoid_target_latent: behavior.cyan_avoid_target_latent,
        cyan_ingest_family_value: behavior.cyan_ingest_family_value,
        cyan_avoid_family_value: behavior.cyan_avoid_family_value,
        amber_target_latent: behavior.amber_target_latent,
        memory_enabled: behavior.memory_enabled,
        memory_ablated: behavior.memory_ablated,
        capacity_saturation: saturation.saturation,
        compact_readback_bytes,
        tolerance: ACCEPTANCE_TOLERANCE,
    };
    receipt.header.common.artifact_digest = receipt.recompute_artifact_digest()?;
    receipt.validate_in_memory()?;
    Ok(receipt)
}

fn validate_behavior_probe(
    behavior: &BehaviorEvidence,
    profile: SensorProfile,
) -> Result<(), GpuEvidenceError> {
    if behavior.memory_enabled.phenotype_hash != behavior.memory_ablated.phenotype_hash
        || behavior.memory_enabled.phenotype_manifest_digest
            != behavior.memory_ablated.phenotype_manifest_digest
        || behavior.memory_enabled.activation_digest != behavior.memory_ablated.activation_digest
        || behavior.memory_enabled.lifetime_weight_digest
            != behavior.memory_ablated.lifetime_weight_digest
        || behavior.memory_enabled.fast_weight_digest != behavior.memory_ablated.fast_weight_digest
        || behavior.memory_enabled.eligibility_digest != behavior.memory_ablated.eligibility_digest
    {
        return Err(GpuEvidenceError::Contract(
            "Slice C paired probes did not restore identical checkpoint state",
        ));
    }
    if behavior.memory_enabled.recurrent_activation_digest
        != behavior.memory_ablated.recurrent_activation_digest
    {
        return Err(GpuEvidenceError::Contract(
            "Slice C candidate memory leaked into recurrent activation state",
        ));
    }
    if behavior.memory_ablated.selected_candidate != behavior.poisoned_ingest_candidate {
        return Err(GpuEvidenceError::Contract(
            "Slice C ablated checkpoint did not select the poisoned ingest baseline",
        ));
    }
    if behavior.memory_enabled.selected_candidate != behavior.post_learning_selection
        || behavior.post_learning_selection == behavior.poisoned_ingest_candidate
    {
        return Err(GpuEvidenceError::Contract(
            "Slice C enabled memory did not change selection away from poisoned ingest",
        ));
    }
    if behavior.poisoned_ingest_delta >= -ACCEPTANCE_TOLERANCE
        || behavior.safe_ingest_delta.abs() >= behavior.poisoned_ingest_delta.abs()
    {
        return Err(GpuEvidenceError::Contract(
            "Slice C target-specific memory did not selectively suppress poisoned ingest",
        ));
    }
    let avoid_delta = behavior.poisoned_avoid_logit_after - behavior.poisoned_avoid_logit_before;
    match profile {
        SensorProfile::GroundedObjectSlotsV1 if avoid_delta <= ACCEPTANCE_TOLERANCE => {
            return Err(GpuEvidenceError::Contract(
                "grounded Slice C target memory did not increase poisoned-target avoidance",
            ));
        }
        SensorProfile::PrivilegedAffordanceV1 if avoid_delta.abs() > ACCEPTANCE_TOLERANCE => {
            return Err(GpuEvidenceError::Contract(
                "privileged Slice C family memory claimed target-specific avoidance",
            ));
        }
        _ => {}
    }
    Ok(())
}

fn validate_saturation_run(
    run: &SaturationRunEvidence,
    profile: SensorProfile,
    capacity: &BrainCapacityClass,
    requested_ticks: u64,
) -> Result<(), GpuEvidenceError> {
    if run.completed_waking_ticks != requested_ticks {
        return Err(GpuEvidenceError::Contract(
            "Slice C run did not complete the exact requested waking ticks",
        ));
    }
    if run.gpu_selection_count != run.completed_waking_ticks {
        return Err(GpuEvidenceError::Contract(
            "Slice C run did not produce one GPU selection per waking tick",
        ));
    }
    if run.compact_readback_bytes > capacity.execution().max_compact_readback_bytes() {
        return Err(GpuEvidenceError::Contract(
            "Slice C run exceeded the compact GPU readback ceiling",
        ));
    }

    match profile {
        SensorProfile::PrivilegedAffordanceV1 => {
            if run.saturation.is_some() {
                return Err(GpuEvidenceError::Contract(
                    "privileged Slice C run made a grounded saturation claim",
                ));
            }
        }
        SensorProfile::GroundedObjectSlotsV1 => {
            let evidence = run.saturation.as_ref().ok_or(GpuEvidenceError::Contract(
                "grounded Slice C run omitted saturation evidence",
            ))?;
            if evidence.grounded_semantic_label_channels_nonzero != 0 {
                return Err(GpuEvidenceError::Contract(
                    "grounded Slice C run exposed privileged semantic labels",
                ));
            }
            if evidence.memory_capacity == 0 || evidence.memory_records > evidence.memory_capacity {
                return Err(GpuEvidenceError::Contract(
                    "grounded Slice C memory exceeded or omitted its bounded capacity",
                ));
            }
            if evidence
                .memory_merges
                .saturating_add(evidence.memory_evictions)
                == 0
            {
                return Err(GpuEvidenceError::Contract(
                    "grounded Slice C memory did not degrade under capacity pressure",
                ));
            }
            if evidence.tracked_object_capacity == 0
                || evidence.tracked_object_records > evidence.tracked_object_capacity
            {
                return Err(GpuEvidenceError::Contract(
                    "grounded Slice C tracker exceeded or omitted its bounded capacity",
                ));
            }
            if evidence.tracked_object_evictions == 0 {
                return Err(GpuEvidenceError::Contract(
                    "grounded Slice C tracker did not evict under capacity pressure",
                ));
            }
            if evidence.tracked_object_id_reuse_count != 0 {
                return Err(GpuEvidenceError::Contract(
                    "grounded Slice C tracker reused a persistent tracked-object ID",
                ));
            }
            if !evidence.topology_capacity.contains(
                evidence.topology_counts,
                evidence.max_observed_bindings_per_kind,
            ) {
                return Err(GpuEvidenceError::Contract(
                    "grounded Slice C topology exceeded or omitted its bounded capacity",
                ));
            }
            if evidence.topology_degradations == 0 {
                return Err(GpuEvidenceError::Contract(
                    "grounded Slice C topology did not degrade under capacity pressure",
                ));
            }
            if evidence.terminal_capacity_errors != 0 {
                return Err(GpuEvidenceError::Contract(
                    "grounded Slice C run reported a terminal capacity error",
                ));
            }
        }
    }
    Ok(())
}

fn run_behavior_probe(
    backend: &mut GpuClosedLoopBackend,
    options: GpuMemoryGroundingAcceptanceOptions,
    phenotype: &BrainPhenotype,
    genome: &BrainGenome,
    development: &DevelopmentState,
    manifest: &PhenotypeEvidenceManifest,
) -> Result<BehaviorEvidence, GpuEvidenceError> {
    let hardware = backend.hardware_receipt().clone();
    let (mut world, cyan, amber) = behavior_world(options.deterministic_seed)?;
    let empty_memory = new_memory_sidecar(options.sensor_profile)?;
    let training_draft = filtered_behavior_draft(
        &mut world,
        Tick::ZERO,
        options.sensor_profile,
        cyan,
        amber,
        BehaviorDraftKind::Training,
    )?;
    let (training_frame, training_recall) = empty_memory
        .recall_frame(&training_draft)?
        .finalize(training_draft)?;
    let mut probe_world = world.clone();
    probe_world.advance_tick();
    probe_world.advance_tick();

    let handle = backend.insert_brain(EVIDENCE_ORGANISM, phenotype.clone())?;
    let training_upload =
        backend.prepare_memory_context_upload(handle, &training_frame, &training_recall)?;
    let training_input =
        GpuClosedLoopMemoryTickInput::try_new(handle, &training_frame, &training_upload)?;
    let training_batch = GpuClosedLoopMemoryBatchInput::try_new(vec![training_input])?;
    let training_tick = backend.tick_memory_batch(&training_batch)?.remove(0);
    require_hardware_generation(training_tick.hardware_receipt_generation, &hardware)?;
    if training_tick.selection.candidate_index != 0 {
        return Err(GpuEvidenceError::Contract(
            "single-candidate poison exposure selected an invalid index",
        ));
    }
    let training_patch = seal_selected_world_outcome(
        &mut world,
        handle,
        genome,
        development,
        &training_frame,
        &training_recall,
        &training_tick,
        ExperienceSequenceId(1),
    )?;
    if training_patch.outcome().pain_delta.raw() <= 0.0
        || training_patch.outcome().physical.contact != alife_core::PhysicalContactKind::Consumed
    {
        return Err(GpuEvidenceError::Contract(
            "controlled cyan ingest did not produce a measured painful consumption",
        ));
    }
    let learning = backend.apply_sealed_outcome(handle, &training_patch)?;
    require_hardware_generation(learning.hardware_receipt_generation, &hardware)?;
    if learning.fast_weights_changed == 0 {
        return Err(GpuEvidenceError::Contract(
            "painful cyan ingest changed no immediately active fast weights",
        ));
    }
    let mut learned_memory = empty_memory;
    learned_memory.observe_sealed_patch(&training_patch)?;
    let source_snapshot = backend.snapshot_brain(handle, Tick::new(1))?;
    let source_parts = source_snapshot.clone().into_parts();
    backend.remove_brain(handle)?;

    let probe_draft = filtered_behavior_draft(
        &mut probe_world,
        Tick::new(2),
        options.sensor_profile,
        cyan,
        amber,
        BehaviorDraftKind::Probe,
    )?;
    let ablated_memory = new_memory_sidecar(options.sensor_profile)?;
    let (ablated_frame, ablated_recall) = ablated_memory
        .recall_frame(&probe_draft)?
        .finalize(probe_draft.clone())?;
    let (enabled_frame, enabled_recall) = learned_memory
        .recall_frame(&probe_draft)?
        .finalize(probe_draft)?;
    if ablated_frame.base_digest() != enabled_frame.base_digest() {
        return Err(GpuEvidenceError::Contract(
            "Slice C paired probes do not share one immutable base frame",
        ));
    }

    let ablated = run_probe_branch(
        backend,
        phenotype,
        &source_snapshot,
        &ablated_frame,
        &ablated_recall,
        &hardware,
    )?;
    let enabled = run_probe_branch(
        backend,
        phenotype,
        &source_snapshot,
        &enabled_frame,
        &enabled_recall,
        &hardware,
    )?;
    if ablated.logits.logits.len() != 3 || enabled.logits.logits.len() != 3 {
        return Err(GpuEvidenceError::Contract(
            "Slice C paired probe did not expose exactly three candidates",
        ));
    }

    let activation_digest = activation_digest(&source_parts);
    let lifetime_weight_digest = lifetime_weight_digest(&source_parts);
    let fast_weight_digest = fast_weight_digest(&source_parts);
    let eligibility_digest = eligibility_digest(&source_parts);
    let poisoned_ingest_delta = enabled.logits.logits[0] - ablated.logits.logits[0];
    let safe_ingest_delta = enabled.logits.logits[2] - ablated.logits.logits[2];
    let contexts = &enabled_recall.context().candidates;
    if contexts.len() != 3 {
        return Err(GpuEvidenceError::Contract(
            "Slice C finalized recall did not preserve candidate order",
        ));
    }
    eprintln!(
        "Slice C paired probe {:?}: ablated={:?} selected={}, enabled={:?} selected={}",
        options.sensor_profile,
        ablated.logits.logits,
        ablated.selected_candidate,
        enabled.logits.logits,
        enabled.selected_candidate,
    );
    let memory_enabled = MemoryContextProbeEvidence {
        phenotype_hash: phenotype.phenotype_hash(),
        phenotype_manifest_digest: manifest.manifest_digest,
        activation_digest,
        recurrent_activation_digest: enabled.recurrent_activation_digest,
        lifetime_weight_digest,
        fast_weight_digest,
        eligibility_digest,
        poisoned_ingest_delta,
        safe_ingest_delta,
        selected_candidate: enabled.selected_candidate,
    };
    let memory_ablated = MemoryContextProbeEvidence {
        phenotype_hash: phenotype.phenotype_hash(),
        phenotype_manifest_digest: manifest.manifest_digest,
        activation_digest,
        recurrent_activation_digest: ablated.recurrent_activation_digest,
        lifetime_weight_digest,
        fast_weight_digest,
        eligibility_digest,
        poisoned_ingest_delta: 0.0,
        safe_ingest_delta: 0.0,
        selected_candidate: ablated.selected_candidate,
    };
    Ok(BehaviorEvidence {
        hardware,
        poisoned_ingest_candidate: 0,
        post_learning_selection: enabled.selected_candidate,
        poisoned_ingest_logit_before: ablated.logits.logits[0],
        poisoned_ingest_logit_after: enabled.logits.logits[0],
        poisoned_avoid_logit_before: ablated.logits.logits[1],
        poisoned_avoid_logit_after: enabled.logits.logits[1],
        poisoned_ingest_delta,
        safe_ingest_delta,
        cyan_ingest_target_latent: contexts[0].target_latent,
        cyan_avoid_target_latent: contexts[1].target_latent,
        cyan_ingest_family_value: contexts[0].family_value,
        cyan_avoid_family_value: contexts[1].family_value,
        amber_target_latent: contexts[2].target_latent,
        memory_enabled,
        memory_ablated,
        max_compact_readback_bytes: ablated
            .compact_readback_bytes
            .max(enabled.compact_readback_bytes)
            .max(u32::try_from(training_tick.compact_readback_bytes).unwrap_or(u32::MAX)),
    })
}

fn run_probe_branch(
    backend: &mut GpuClosedLoopBackend,
    phenotype: &BrainPhenotype,
    source_snapshot: &GpuBrainCheckpointSnapshot,
    frame: &PerceptionFrame,
    recall: &FinalizedMemoryRecall,
    hardware: &GpuHardwareReceipt,
) -> Result<ProbeBranchEvidence, GpuEvidenceError> {
    let restored = backend.restore_brain(
        EVIDENCE_ORGANISM,
        phenotype.clone(),
        GpuBrainRestoreRequest::try_new(source_snapshot.clone())?,
    )?;
    let handle = restored.handle;
    let upload = backend.prepare_memory_context_upload(handle, frame, recall)?;
    let input = GpuClosedLoopMemoryTickInput::try_new(handle, frame, &upload)?;
    let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![input])?;
    let tick = backend.tick_memory_batch(&batch)?.remove(0);
    require_hardware_generation(tick.hardware_receipt_generation, hardware)?;
    let logits = backend.candidate_logits_for_evidence(
        handle,
        frame,
        tick.pending_eligibility.identity(),
    )?;
    let checkpoint = backend.snapshot_brain(handle, frame.tick())?.into_parts();
    let recurrent_activation_digest = recurrent_activation_digest(&checkpoint);
    backend.discard_pending_eligibility(handle, tick.pending_eligibility.identity())?;
    backend.remove_brain(handle)?;
    Ok(ProbeBranchEvidence {
        logits,
        selected_candidate: tick.selection.candidate_index,
        recurrent_activation_digest,
        compact_readback_bytes: u32::try_from(tick.compact_readback_bytes)
            .map_err(|_| GpuEvidenceError::Contract("GPU readback size does not fit u32"))?,
    })
}

fn run_saturation_probe(
    backend: GpuClosedLoopBackend,
    options: GpuMemoryGroundingAcceptanceOptions,
    tier: BrainScaleTier,
    expected_hardware: &GpuHardwareReceipt,
) -> Result<SaturationRunEvidence, GpuEvidenceError> {
    let world = HeadlessScenarioBuilder::new(options.deterministic_seed)
        .agent("slice-c-agent", EVIDENCE_ORGANISM, Vec3f::ZERO)
        .build()?;
    let mut runtime = GpuLiveBrainRuntime::new_causal_acceptance_profiled(
        backend,
        world,
        options.deterministic_seed,
        tier,
        options.sensor_profile,
    )?;
    if runtime.hardware_receipt() != expected_hardware {
        return Err(GpuEvidenceError::Contract(
            "Slice C runtime changed GPU hardware identity",
        ));
    }
    let agent_entity = runtime
        .evidence_world()
        .organism_entity_ids()
        .into_iter()
        .find_map(|(organism, entity)| (organism == EVIDENCE_ORGANISM).then_some(entity))
        .ok_or(GpuEvidenceError::Contract(
            "Slice C saturation world is missing its organism",
        ))?;
    let dispatch_before = runtime.evidence_completed_dispatch_count();
    let mut previous_object = None;
    let mut compact_readback_bytes = 0_u32;
    let mut tracked_identity_by_id = BTreeMap::new();
    let mut tracked_object_id_reuse_count = 0_u64;
    let mut grounded_semantic_label_channels_nonzero = 0_u32;

    for iteration in 0..options.requested_ticks {
        if let Some(object) = previous_object.take() {
            runtime.evidence_world_mut().editor_remove_object(object)?;
        }
        runtime
            .evidence_world_mut()
            .editor_move_object(agent_entity, Vec3f::ZERO)?;
        let tick = runtime.evidence_world().tick();
        runtime.evidence_set_homeostasis(EVIDENCE_ORGANISM, HomeostaticSnapshot::baseline(tick))?;
        let object = runtime
            .evidence_world_mut()
            .editor_spawn_object(rotating_object_spec(iteration))?;
        runtime
            .evidence_world_mut()
            .set_grounded_physical_properties(object, rotating_properties(iteration))?;
        previous_object = Some(object);

        let patch_count_before = runtime.sealed_patches().len();
        let summaries = runtime.tick()?;
        if summaries.len() != 1
            || !summaries[0].patch_sealed
            || runtime.sealed_patches().len() != patch_count_before + 1
        {
            return Err(GpuEvidenceError::Contract(
                "Slice C saturation tick did not seal exactly one neural patch",
            ));
        }
        let patch = runtime
            .sealed_patches()
            .last()
            .ok_or(GpuEvidenceError::Contract(
                "Slice C sealed patch is missing",
            ))?;
        patch.decision().neural_evidence()?;
        compact_readback_bytes = compact_readback_bytes.max(
            u32::try_from(runtime.evidence_metrics().compact_readback_bytes)
                .map_err(|_| GpuEvidenceError::Contract("GPU readback size does not fit u32"))?,
        );
        if options.sensor_profile == SensorProfile::GroundedObjectSlotsV1 {
            grounded_semantic_label_channels_nonzero = grounded_semantic_label_channels_nonzero
                .saturating_add(
                    patch
                        .pre_action()
                        .sensory()
                        .channels
                        .visual_affordance
                        .iter()
                        .filter(|value| value.to_bits() != 0.0_f32.to_bits())
                        .count() as u32,
                );
        }
        let tracked = runtime
            .evidence_world()
            .tracked_objects()
            .save_state(EVIDENCE_ORGANISM)?;
        for record in tracked.records {
            if tracked_identity_by_id
                .insert(record.tracked_object_id.raw(), record.tracking_key)
                .is_some_and(|previous| previous != record.tracking_key)
            {
                tracked_object_id_reuse_count = tracked_object_id_reuse_count.saturating_add(1);
            }
        }
        if (iteration + 1).is_multiple_of(1_024) {
            eprintln!(
                "Slice C {:?} {} progress: {}/{} ticks",
                options.sensor_profile,
                capacity_slug(options.capacity.id())?,
                iteration + 1,
                options.requested_ticks,
            );
        }
    }

    let gpu_selection_count = runtime
        .evidence_completed_dispatch_count()
        .checked_sub(dispatch_before)
        .ok_or(GpuEvidenceError::Contract(
            "Slice C GPU dispatch counter moved backwards",
        ))?;
    let saturation = if options.sensor_profile == SensorProfile::GroundedObjectSlotsV1 {
        Some(collect_saturation_evidence(
            &runtime,
            grounded_semantic_label_channels_nonzero,
            tracked_object_id_reuse_count,
        )?)
    } else {
        None
    };
    Ok(SaturationRunEvidence {
        completed_waking_ticks: options.requested_ticks,
        gpu_selection_count,
        compact_readback_bytes,
        saturation,
    })
}

fn collect_saturation_evidence(
    runtime: &GpuLiveBrainRuntime,
    grounded_semantic_label_channels_nonzero: u32,
    tracked_object_id_reuse_count: u64,
) -> Result<CapacitySaturationEvidence, GpuEvidenceError> {
    let memory = runtime
        .evidence_memory_sidecar(EVIDENCE_ORGANISM)
        .ok_or(GpuEvidenceError::Contract(
            "Slice C runtime is missing its memory sidecar",
        ))?
        .export_active_bank()?;
    let tracked = runtime
        .evidence_world()
        .tracked_objects()
        .save_state(EVIDENCE_ORGANISM)?;
    let topology = runtime
        .evidence_topology_sidecar(EVIDENCE_ORGANISM)
        .ok_or(GpuEvidenceError::Contract(
            "Slice C runtime is missing its topology sidecar",
        ))?
        .export_portable()?;
    let inserted = tracked
        .next_id
        .checked_sub(initial_tracked_object_id(
            tracked.world_seed,
            EVIDENCE_ORGANISM.raw(),
        ))
        .ok_or(GpuEvidenceError::Contract(
            "tracked-object allocator moved backwards",
        ))?;
    let tracked_object_evictions = inserted.saturating_sub(tracked.records.len() as u64);
    let max_observed_bindings_per_kind = topology
        .concepts
        .iter()
        .map(|concept| max_binding_len(&concept.bindings))
        .max()
        .unwrap_or(0);
    Ok(CapacitySaturationEvidence {
        grounded_semantic_label_channels_nonzero,
        memory_records: u32::try_from(memory.records.len()).unwrap_or(u32::MAX),
        memory_capacity: memory.capacity,
        memory_merges: memory.merge_count,
        memory_evictions: memory.eviction_count,
        tracked_object_records: u32::try_from(tracked.records.len()).unwrap_or(u32::MAX),
        tracked_object_capacity: tracked.capacity,
        tracked_object_evictions,
        tracked_object_id_reuse_count,
        topology_counts: TopologyCounts {
            concepts: u32::try_from(topology.concepts.len()).unwrap_or(u32::MAX),
            edges: u32::try_from(topology.edges.len()).unwrap_or(u32::MAX),
            simplexes: u32::try_from(topology.simplexes.len()).unwrap_or(u32::MAX),
            unresolved_gaps: u32::try_from(topology.gaps.len()).unwrap_or(u32::MAX),
        },
        topology_capacity: TopologyCapacityReceipt {
            max_concepts: topology.max_concepts,
            max_edges: topology.max_edges,
            max_simplexes: topology.max_simplexes,
            max_unresolved_gaps: topology.max_unresolved_gaps,
            max_bindings_per_kind: topology.max_bindings_per_kind,
        },
        max_observed_bindings_per_kind,
        topology_degradations: topology.degradation_count,
        terminal_capacity_errors: 0,
    })
}

fn behavior_world(
    seed: u64,
) -> Result<(HeadlessWorld, WorldEntityId, WorldEntityId), GpuEvidenceError> {
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("slice-c-learner", EVIDENCE_ORGANISM, Vec3f::ZERO)
        .build()?;
    let cyan = world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "cyan-bitter-food".to_string(),
        kind: WorldObjectKind::Food,
        organism_id: None,
        position: CYAN_POSITION,
        nutrition: 0.6,
        hazard_pain: 0.9,
        radius: 0.5,
        token_id: None,
    })?;
    let amber = world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "amber-sweet-food".to_string(),
        kind: WorldObjectKind::Food,
        organism_id: None,
        position: AMBER_POSITION,
        nutrition: 0.8,
        hazard_pain: 0.0,
        radius: 0.5,
        token_id: None,
    })?;
    world.set_grounded_physical_properties(
        cyan,
        GroundedPhysicalProperties {
            velocity: Vec3f::new(0.0, 4.0, 0.0),
            color: [0.0, 0.9, 0.9],
            material: [0.2, 0.8, 0.6],
            shape: [0.8, 0.2, 0.2],
            chemical: [-0.9, 0.2, 0.1],
            surface_temperature: -1.0,
            terrain: [0.2, 0.8],
        },
    )?;
    world.set_grounded_physical_properties(
        amber,
        GroundedPhysicalProperties {
            velocity: Vec3f::ZERO,
            color: [0.95, 0.55, 0.05],
            material: [0.8, 0.3, 0.1],
            shape: [0.2, 0.8, 0.2],
            chemical: [0.9, -0.2, 0.1],
            surface_temperature: -0.1,
            terrain: [0.8, 0.2],
        },
    )?;
    Ok((world, cyan, amber))
}

#[derive(Debug, Clone, Copy)]
enum BehaviorDraftKind {
    Training,
    Probe,
}

fn filtered_behavior_draft(
    world: &mut HeadlessWorld,
    tick: Tick,
    profile: SensorProfile,
    cyan: WorldEntityId,
    amber: WorldEntityId,
    kind: BehaviorDraftKind,
) -> Result<PerceptionFrameDraft, GpuEvidenceError> {
    let mut homeostasis = HomeostaticSnapshot::baseline(tick);
    homeostasis.drives.hunger = 0.95;
    homeostasis.drives.fear = 0.65;
    homeostasis.drives.curiosity = 0.80;
    let source = world.perception_frame_draft(EVIDENCE_ORGANISM, tick, profile, homeostasis)?;
    let requested = match kind {
        BehaviorDraftKind::Training => [
            (cyan, CandidateActionFamily::Ingest, true),
            (cyan, CandidateActionFamily::Avoid, false),
            (amber, CandidateActionFamily::Ingest, false),
        ],
        BehaviorDraftKind::Probe => [
            (cyan, CandidateActionFamily::Ingest, true),
            (cyan, CandidateActionFamily::Avoid, true),
            (amber, CandidateActionFamily::Ingest, true),
        ],
    };
    let mut candidates = Vec::new();
    for (entity, family, include) in requested {
        if !include {
            continue;
        }
        let mut candidate = source
            .candidates()
            .iter()
            .find(|candidate| candidate.target.entity == Some(entity) && candidate.family == family)
            .copied()
            .ok_or(GpuEvidenceError::Contract(
                "Slice C world did not enumerate a required unscored candidate",
            ))?;
        candidate.candidate_index = u16::try_from(candidates.len())
            .map_err(|_| GpuEvidenceError::Contract("candidate index does not fit u16"))?;
        candidate.validate_contract()?;
        candidates.push(candidate);
    }
    PerceptionFrameDraft::new(
        source.organism_id(),
        source.tick(),
        source.sensor_profile(),
        source.sensory().clone(),
        source.body(),
        *source.homeostasis(),
        candidates,
        source.profile_provenance(),
        source.grounded_object_slots().to_vec(),
    )
    .map_err(GpuEvidenceError::from)
}

#[allow(clippy::too_many_arguments)]
fn seal_selected_world_outcome(
    world: &mut HeadlessWorld,
    handle: alife_gpu_backend::GpuBrainHandle,
    genome: &BrainGenome,
    development: &DevelopmentState,
    frame: &PerceptionFrame,
    recall: &FinalizedMemoryRecall,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    sequence_id: ExperienceSequenceId,
) -> Result<ExperiencePatch, GpuEvidenceError> {
    let candidate = frame.candidates()[usize::from(tick.selection.candidate_index)];
    let command = candidate.to_command(frame.organism_id(), tick.selection.confidence)?;
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
        NeuralActionSelection {
            candidate_index: tick.selection.candidate_index,
            logit: tick.selection.logit,
            confidence: tick.selection.confidence,
            active_tiles: tick.selection.active_tiles,
            active_synapses: tick.selection.active_synapses,
        },
        command,
    )?
    .with_finalized_memory_recall(frame, recall, tick.selection.candidate_index)?;
    let action_result = world.apply_command(&decision.selected_action)?;
    let mut outcome = PostActionOutcome::new(
        frame.organism_id(),
        sequence_id,
        Tick::new(frame.tick().raw().saturating_add(1)),
        action_result.observation.success && action_result.execution.succeeded,
        action_result.execution.physical,
        action_result.observation.homeostatic_delta,
        action_result.observation.reward_valence,
        action_result.observation.frustration_delta,
        action_result.observation.pain_delta,
        action_result.observation.energy_delta,
        action_result.observation.prediction_error,
    )?;
    outcome.contradiction_observed =
        action_result.observation.contradiction_observed || !action_result.execution.succeeded;
    outcome.validate_contract()?;
    Ok(ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre_action)?
        .record_decision(decision)?
        .record_outcome(outcome)?
        .seal()?)
}

fn new_memory_sidecar(profile: SensorProfile) -> Result<MemorySidecarState, GpuEvidenceError> {
    Ok(MemorySidecarState::new_profiled(
        EVIDENCE_ORGANISM,
        profile_identity(profile),
        MemoryBankConfig::new(
            MEMORY_CAPACITY,
            MEMORY_MAX_FEATURE_LEN,
            MEMORY_MAX_MATCH_COUNT,
            MEMORY_MIN_MATCH_SCORE,
            alife_core::Confidence::new(0.0)?,
        )?,
    )?)
}

fn profile_identity(profile: SensorProfile) -> SensorProfileIdentity {
    SensorProfileIdentity {
        profile_id: profile.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    }
}

fn rotating_object_spec(iteration: u64) -> WorldEditorSpawnSpec {
    let angle = (iteration % 64) as f32 * std::f32::consts::TAU / 64.0;
    WorldEditorSpawnSpec {
        label: format!("slice-c-rotating-{iteration}"),
        kind: WorldObjectKind::Food,
        organism_id: None,
        position: Vec3f::new(angle.cos() * 1.5, angle.sin() * 1.5, 0.0),
        nutrition: ((iteration % 17) as f32 / 16.0).clamp(0.05, 1.0),
        hazard_pain: ((iteration % 13) as f32 / 12.0).clamp(0.0, 1.0),
        radius: 0.35,
        token_id: None,
    }
}

fn rotating_properties(iteration: u64) -> GroundedPhysicalProperties {
    let unit = |shift: u32| {
        (((iteration.rotate_left(shift) ^ (iteration >> shift)) & 0xff) as f32) / 255.0
    };
    let signed = |shift: u32| unit(shift) * 2.0 - 1.0;
    GroundedPhysicalProperties {
        velocity: Vec3f::ZERO,
        color: [unit(1), unit(7), unit(13)],
        material: [unit(19), unit(23), unit(29)],
        shape: [unit(3), unit(11), unit(17)],
        chemical: [signed(5), signed(9), signed(21)],
        surface_temperature: signed(27),
        terrain: [unit(15), unit(25)],
    }
}

fn max_binding_len(bindings: &alife_core::PortableTopologyBindingSetV1) -> u32 {
    [
        bindings.tracked_object_ids_raw.len(),
        bindings.word_ids_raw.len(),
        bindings.drives.len(),
        bindings.actions.len(),
        bindings.action_families_raw.len(),
        bindings.location_bits.len(),
        bindings.agent_ids_raw.len(),
        bindings.semantic_concept_ids_raw.len(),
        bindings.cluster_ids_raw.len(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
    .try_into()
    .unwrap_or(u32::MAX)
}

fn require_hardware_generation(
    observed: u64,
    hardware: &GpuHardwareReceipt,
) -> Result<(), GpuEvidenceError> {
    if observed != hardware.generation || !hardware.backend_api.eq_ignore_ascii_case("vulkan") {
        return Err(GpuEvidenceError::Contract(
            "Slice C GPU tick does not bind the exact Vulkan hardware receipt generation",
        ));
    }
    Ok(())
}

fn activation_digest(parts: &GpuBrainCheckpointParts) -> [u64; 4] {
    digest_bit_slices(
        b"alife.gpu.evidence.slice-c.activation.v1",
        parts.active_activation_side,
        parts.logical_dispatch_generation,
        [&parts.activation_a_bits, &parts.activation_b_bits],
    )
}

fn recurrent_activation_digest(parts: &GpuBrainCheckpointParts) -> [u64; 4] {
    let active = if parts.active_activation_side == 0 {
        &parts.activation_a_bits
    } else {
        &parts.activation_b_bits
    };
    digest_bit_slices(
        b"alife.gpu.evidence.slice-c.recurrent-activation.v1",
        parts.active_activation_side,
        0,
        [active, &[]],
    )
}

fn lifetime_weight_digest(parts: &GpuBrainCheckpointParts) -> [u64; 4] {
    let active = if parts.active_weight_bank == 0 {
        &parts.lifetime_bank_0_bits
    } else {
        &parts.lifetime_bank_1_bits
    };
    digest_bit_slices(
        b"alife.gpu.evidence.slice-c.lifetime-weight.v1",
        parts.active_weight_bank,
        parts.active_weight_generation,
        [active, &[]],
    )
}

fn fast_weight_digest(parts: &GpuBrainCheckpointParts) -> [u64; 4] {
    let active = if parts.active_weight_bank == 0 {
        &parts.fast_bank_0_bits
    } else {
        &parts.fast_bank_1_bits
    };
    digest_bit_slices(
        b"alife.gpu.evidence.slice-c.fast-weight.v1",
        parts.active_weight_bank,
        parts.active_weight_generation,
        [active, &[]],
    )
}

fn eligibility_digest(parts: &GpuBrainCheckpointParts) -> [u64; 4] {
    let (recurrent, decoder) = if parts.active_eligibility_bank == 0 {
        (
            &parts.recurrent_eligibility_bank_0_bits,
            &parts.decoder_eligibility_bank_0_bits,
        )
    } else {
        (
            &parts.recurrent_eligibility_bank_1_bits,
            &parts.decoder_eligibility_bank_1_bits,
        )
    };
    digest_bit_slices(
        b"alife.gpu.evidence.slice-c.eligibility.v1",
        parts.active_eligibility_bank,
        parts.active_eligibility_generation,
        [recurrent, decoder],
    )
}

fn digest_bit_slices<const N: usize>(
    domain: &'static [u8],
    active_bank: u8,
    generation: u64,
    slices: [&[u32]; N],
) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(domain);
    digest.write_u8(active_bank);
    digest.write_u64(generation);
    digest.write_sequence_len(N);
    for slice in slices {
        digest.write_sequence_len(slice.len());
        for value in slice {
            digest.write_u32(*value);
        }
    }
    digest.finish256()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_grounded_saturation_run() -> SaturationRunEvidence {
        SaturationRunEvidence {
            completed_waking_ticks: GROUNDED_ACCEPTANCE_TICKS,
            gpu_selection_count: GROUNDED_ACCEPTANCE_TICKS,
            compact_readback_bytes: 64,
            saturation: Some(CapacitySaturationEvidence {
                grounded_semantic_label_channels_nonzero: 0,
                memory_records: 64,
                memory_capacity: 64,
                memory_merges: 1,
                memory_evictions: 0,
                tracked_object_records: 16,
                tracked_object_capacity: 16,
                tracked_object_evictions: 1,
                tracked_object_id_reuse_count: 0,
                topology_counts: TopologyCounts {
                    concepts: 8,
                    edges: 8,
                    simplexes: 8,
                    unresolved_gaps: 8,
                },
                topology_capacity: TopologyCapacityReceipt {
                    max_concepts: 8,
                    max_edges: 8,
                    max_simplexes: 8,
                    max_unresolved_gaps: 8,
                    max_bindings_per_kind: 4,
                },
                max_observed_bindings_per_kind: 4,
                topology_degradations: 1,
                terminal_capacity_errors: 0,
            }),
        }
    }

    #[test]
    fn saturation_preflight_rejects_missing_grounded_degradation() {
        let mut run = valid_grounded_saturation_run();
        run.saturation.as_mut().unwrap().topology_degradations = 0;
        assert!(matches!(
            validate_saturation_run(
                &run,
                SensorProfile::GroundedObjectSlotsV1,
                &BrainCapacityClass::n512(),
                GROUNDED_ACCEPTANCE_TICKS,
            ),
            Err(GpuEvidenceError::Contract(
                "grounded Slice C topology did not degrade under capacity pressure"
            ))
        ));
    }

    #[test]
    fn saturation_preflight_rejects_profile_evidence_mismatch() {
        let mut run = valid_grounded_saturation_run();
        run.completed_waking_ticks = PRIVILEGED_ACCEPTANCE_TICKS;
        run.gpu_selection_count = PRIVILEGED_ACCEPTANCE_TICKS;
        assert!(matches!(
            validate_saturation_run(
                &run,
                SensorProfile::PrivilegedAffordanceV1,
                &BrainCapacityClass::n512(),
                PRIVILEGED_ACCEPTANCE_TICKS,
            ),
            Err(GpuEvidenceError::Contract(
                "privileged Slice C run made a grounded saturation claim"
            ))
        ));
    }
}
