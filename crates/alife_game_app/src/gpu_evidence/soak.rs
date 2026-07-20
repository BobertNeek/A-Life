//! Canonical Slice D bounded-soak and same-adapter replay evidence.

use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::{
    BrainCapacityClass, BrainClassId, BrainScaleTier, BrainWorkCounters, BrainWorkReceipt,
    CandidateActionFamily, CanonicalDigestBuilder, DriveSnapshot, EndocrineSnapshot,
    GlobalPhenotypeBudgetReceipt, GpuPressureSample, MemoryCompactionReceipt, MemoryRecallReceipt,
    MemoryUpdateReceipt, NeuralThrottleDecision, NeuralThrottleLevel, OrganismId,
    PhenotypeEvidenceManifest, PolicyBackend, RouteBudgetReceipt, SensorProfile,
    SensorProfileIdentity, SensoryAbiVersion, SleepPhase, Tick, TopologyCounts,
    TopologyDegradationKind, TopologyObservationReceipt, Validate, Vec3f,
};
use alife_gpu_backend::{GpuAdmissionReceipt, GpuAllocationEventReceipt, GpuClosedLoopBackend};
use alife_world::{
    persistence::{
        AssetManifest, GpuBackendProvenanceSave, NeuralGpuBackendApi, ProductionNeuralAvailability,
        GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
    },
    HeadlessScenarioBuilder, HeadlessWorld, WorldEditorSpawnSpec, WorldObjectKind,
};
use serde::{Deserialize, Serialize};

use crate::{
    compile_gpu_birth_components, gpu_checkpoint_assets::current_backend_provenance,
    merge_gpu_checkpoint_manifest_entries, GpuCheckpointAssetStore, GpuLiveBrainRuntime,
};

use super::{
    canonical::{
        encode_header_without_artifact_digest, encode_manifest_with_digest, write_digest4,
    },
    capacity_slug, is_lower_hex_oid, read_git_provenance, sensor_profile_slug, tier_for_capacity,
    GitProvenance, GpuEvidenceError, GpuSliceEvidenceHeader, ProfiledBehaviorReceiptHeader,
    TopologyCapacityReceipt, GPU_EVIDENCE_BACKEND_API_SLUG, GPU_EVIDENCE_BACKEND_API_VERSION,
    GPU_EVIDENCE_PASSING_STATUS_RAW, GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
};

pub const GPU_SLICE_D_RAW: u16 = 4;
pub const GPU_SLICE_D_TICKS: u64 = 10_240;
pub const GPU_SLICE_D_WARMUP_TICKS: u64 = 256;
pub const GPU_SLICE_D_SAMPLE_INTERVAL: u64 = 64;
pub const GPU_SLICE_D_SAMPLE_COUNT: usize = 157;
pub const GPU_SLICE_D_REPLAY_TOLERANCE_BITS: u32 = 0x3727_c5ac;

const SOAK_SCHEMA_VERSION: u16 = 1;
const SOAK_ARTIFACT_MAX_BYTES: u64 = 128 * 1024 * 1024;
const SOAK_ORGANISM: OrganismId = OrganismId(1);
const SOAK_ARTIFACT_DOMAIN: &[u8] = b"alife.gpu.evidence.slice-d-artifact.v1";
const SOAK_JSON_DIGEST_DOMAIN: &[u8] = b"alife.gpu.evidence.slice-d-json.v1";
const SOAK_RSS_MIN_GROWTH_ENVELOPE: u64 = 16 * 1024 * 1024;
const SOAK_RSS_BUDGET_HEADROOM: u64 = 512 * 1024 * 1024;
const REPLAY_DISPATCH_LIMIT: usize = 32;
const TOPOLOGY_BINDING_PRESSURE_TICKS: u64 = 2_048;
const TRUNCATION_CANDIDATE: u16 = 1;
const TRUNCATION_OBJECT_SLOT: u16 = 2;
const TRUNCATION_MEMORY_CONTEXT: u16 = 3;
const TRUNCATION_TOPOLOGY_BINDING: u16 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuClosedLoopSoakOptions {
    pub capacity: BrainCapacityClass,
    pub sensor_profile: SensorProfile,
    pub completed_ticks: u64,
    pub deterministic_seed: u64,
}

impl GpuClosedLoopSoakOptions {
    fn validate(self) -> Result<Self, GpuEvidenceError> {
        self.capacity.validate_contract()?;
        if self.completed_ticks != GPU_SLICE_D_TICKS || self.deterministic_seed == 0 {
            return Err(GpuEvidenceError::Contract(
                "Slice D soak requires exactly 10,240 ticks and a nonzero seed",
            ));
        }
        capacity_slug(self.capacity.id())?;
        Ok(self)
    }

    pub fn artifact_slug(self) -> Result<String, GpuEvidenceError> {
        let options = self.validate()?;
        Ok(format!(
            "gpu-closed-loop-slice-d-{}-{}",
            sensor_profile_slug(options.sensor_profile),
            capacity_slug(options.capacity.id())?
        ))
    }

    pub fn artifact_path(self) -> Result<PathBuf, GpuEvidenceError> {
        Ok(PathBuf::from("target")
            .join("artifacts")
            .join(format!("{}.json", self.artifact_slug()?)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAllocationSample {
    pub tick: u64,
    pub logical_committed_bytes: u64,
    pub physical_allocated_bytes: u64,
    pub physical_unused_retained_bytes: u64,
    pub physical_shared_bytes: u64,
    pub physical_alignment_slack_bytes: u64,
    pub peak_logical_committed_bytes: u64,
    pub peak_physical_allocated_bytes: u64,
    pub allocation_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionSoakReceipt {
    pub schema_version: u16,
    pub logical_budget_bytes: u64,
    pub physical_ceiling_bytes: u64,
    pub peak_logical_committed_bytes: u64,
    pub peak_physical_allocated_bytes: u64,
    pub post_warmup_logical_min_bytes: u64,
    pub post_warmup_logical_max_bytes: u64,
    pub post_warmup_physical_min_bytes: u64,
    pub post_warmup_physical_max_bytes: u64,
    pub raw_events: Vec<GpuAllocationEventReceipt>,
    pub raw_samples: Vec<GpuAllocationSample>,
    pub samples_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRssSample {
    pub tick: u64,
    pub rss_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessMemorySoakReceipt {
    pub schema_version: u16,
    pub rss_budget_bytes: u64,
    pub rss_high_water_bytes: u64,
    pub growth_envelope_bytes: u64,
    pub post_warmup_growth_bytes: u64,
    pub first_quartile_mean_bytes: u64,
    pub last_quartile_mean_bytes: u64,
    pub raw_samples: Vec<ProcessRssSample>,
    pub samples_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchAccountingSample {
    pub tick: u64,
    pub pressure: GpuPressureSample,
    pub throttle: NeuralThrottleDecision,
    pub work: BrainWorkReceipt,
}

impl DispatchAccountingSample {
    pub fn bindings_match(&self) -> bool {
        self.tick == self.pressure.tick
            && self.tick == self.throttle.tick
            && self.tick == self.work.tick
            && self.pressure.sample_digest == self.throttle.pressure_digest
            && self.throttle.pressure == self.pressure
            && self.pressure.organism_id_raw == self.work.organism_id_raw
            && self.pressure.class_id_raw == self.work.class_id_raw
            && self.pressure.handle_slot == self.work.handle_slot
            && self.pressure.handle_generation == self.work.handle_generation
            && self.pressure.sequence_cursor == self.work.sequence_cursor
            && self.pressure.dispatch_generation == self.work.dispatch_generation
            && self.pressure.frame_digest == self.work.frame_digest
            && self.throttle.route_schedule_digest == self.work.route_schedule_digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivitySoakReceipt {
    pub schema_version: u16,
    pub activity_policy_version: u16,
    pub activity_policy_digest: [u64; 4],
    pub total_work: BrainWorkCounters,
    pub total_neural_cost_q24: u64,
    pub total_atp_debit_q16: u64,
    pub full_dispatches: u64,
    pub reduced_dispatches: u64,
    pub essential_only_dispatches: u64,
    pub learning_commits: u64,
    pub raw_dispatch_samples: Vec<DispatchAccountingSample>,
    pub sequence_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySoakReceipt {
    pub schema_version: u16,
    pub capacity: u32,
    pub final_record_count: u32,
    pub merges: u64,
    pub evictions: u64,
    pub compactions: u64,
    pub raw_updates: Vec<MemoryUpdateReceipt>,
    pub raw_compactions: Vec<MemoryCompactionReceipt>,
    pub receipts_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologySoakReceipt {
    pub schema_version: u16,
    pub capacity: TopologyCapacityReceipt,
    pub final_counts: TopologyCounts,
    pub max_observed_bindings_per_kind: u32,
    pub degradations: u64,
    pub raw_observations: Vec<TopologyObservationReceipt>,
    pub receipts_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncationEventReceipt {
    pub tick: u64,
    pub kind_raw: u16,
    pub requested: u32,
    pub retained: u32,
    pub dropped: u32,
    pub input_digest: [u64; 4],
    pub output_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncationSoakReceipt {
    pub schema_version: u16,
    pub max_candidates: u16,
    pub max_object_slots: u16,
    pub max_memory_context_records: u16,
    pub max_decoder_input_lanes: u16,
    pub compact_readback_bytes: u32,
    pub candidate_truncations: u64,
    pub object_slot_truncations: u64,
    pub memory_context_truncations: u64,
    pub topology_binding_truncations: u64,
    pub raw_events: Vec<TruncationEventReceipt>,
    pub events_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveMigrationReceipt {
    pub source_schema: u16,
    pub target_schema: u16,
    pub legacy_class_id_raw: u16,
    pub classification_raw: u16,
    pub phenotype_compile_count: u32,
    pub gpu_admission_count: u32,
    pub phenotype_hash_or_zero: [u64; 4],
    pub receipt_digest: [u64; 4],
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveRestoreReceipt {
    pub save_tick: u64,
    pub restore_tick: u64,
    pub sleep_phase_raw: u16,
    pub consolidation_state_raw: u16,
    pub expected_remaining_swaps: u16,
    pub observed_remaining_swaps: u16,
    pub pre_save_state_digest: [u64; 4],
    pub post_restore_state_digest: [u64; 4],
    pub receipt_digest: [u64; 4],
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveRestoreSoakReceipt {
    pub schema_version: u16,
    pub sleep_cycles: u64,
    pub save_count: u32,
    pub restore_count: u32,
    pub restore_receipts: Vec<SaveRestoreReceipt>,
    pub migration_receipts: Vec<SaveMigrationReceipt>,
    pub receipts_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySwitchEventReceipt {
    pub tick: u64,
    pub from_policy_raw: u16,
    pub to_policy_raw: u16,
    pub reason_code: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySwitchSoakReceipt {
    pub schema_version: u16,
    pub initial_policy_raw: u16,
    pub final_policy_raw: u16,
    pub switch_count: u32,
    pub raw_events: Vec<PolicySwitchEventReceipt>,
    pub events_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayComparisonSample {
    pub sequence_cursor: u64,
    pub dispatch_generation: u64,
    pub source_candidate_index: u16,
    pub replay_candidate_index: u16,
    pub max_abs_logit_delta_f32_bits: u32,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SameAdapterReplayReceipt {
    pub schema_version: u16,
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend_api_raw: u16,
    pub driver_digest: [u64; 4],
    pub feature_digest: [u64; 4],
    pub limits_digest: [u64; 4],
    pub checkpoint_digest: [u64; 4],
    pub pressure_sequence_digest: [u64; 4],
    pub source_selection_digest: [u64; 4],
    pub replay_selection_digest: [u64; 4],
    pub source_work_digest: [u64; 4],
    pub replay_work_digest: [u64; 4],
    pub compared_dispatches: u64,
    pub selection_mismatches: u32,
    pub logit_tolerance_f32_bits: u32,
    pub max_abs_logit_delta_f32_bits: u32,
    pub logit_tolerance_violations: u32,
    pub raw_comparisons: Vec<ReplayComparisonSample>,
    pub comparisons_digest: [u64; 4],
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuClosedLoopSoakReceipt {
    #[serde(flatten)]
    pub header: ProfiledBehaviorReceiptHeader,
    pub phenotype_manifest: PhenotypeEvidenceManifest,
    pub sensor_profile: SensorProfileIdentity,
    pub capacity_class_slug: String,
    pub policy_backend: PolicyBackend,
    pub adapter: GpuBackendProvenanceSave,
    pub capacity: BrainCapacityClass,
    pub completed_ticks: u64,
    pub route_budgets: Vec<RouteBudgetReceipt>,
    pub global_budget: GlobalPhenotypeBudgetReceipt,
    pub admission: AdmissionSoakReceipt,
    pub process_memory: ProcessMemorySoakReceipt,
    pub activity: ActivitySoakReceipt,
    pub memory: MemorySoakReceipt,
    pub topology: TopologySoakReceipt,
    pub truncation: TruncationSoakReceipt,
    pub save_restore: SaveRestoreSoakReceipt,
    pub policy_switch: PolicySwitchSoakReceipt,
    pub gpu_selections: u64,
    pub authoritative_gpu_dispatches: u64,
    pub terminal_capacity_errors: u64,
    pub replay: SameAdapterReplayReceipt,
}

impl GpuClosedLoopSoakReceipt {
    pub fn recompute_artifact_digest(&self) -> Result<[u64; 4], GpuEvidenceError> {
        let mut digest = CanonicalDigestBuilder::new(SOAK_ARTIFACT_DOMAIN);
        encode_header_without_artifact_digest(&mut digest, &self.header.common);
        digest.write_utf8(&self.header.artifact_slug);
        digest.write_u16(self.header.backend_api_version);
        digest.write_utf8(&self.header.backend_api_slug);
        digest.write_utf8(&self.header.adapter_name);
        digest.write_utf8(&self.header.adapter_backend);
        digest.write_u64(self.header.run_seed);
        encode_manifest_with_digest(&mut digest, &self.phenotype_manifest);
        digest.write_u16(self.sensor_profile.profile_id.raw());
        digest.write_u16(self.sensor_profile.profile_schema_version);
        digest.write_u16(self.sensor_profile.sensory_abi_version);
        digest.write_utf8(&self.capacity_class_slug);
        digest.write_u8(policy_raw(self.policy_backend));
        write_digest4(&mut digest, digest_json(&self.adapter)?);
        write_digest4(&mut digest, self.capacity.canonical_digest());
        digest.write_u64(self.completed_ticks);
        write_digest4(&mut digest, digest_json(&self.route_budgets)?);
        write_digest4(&mut digest, digest_json(&self.global_budget)?);
        for subdigest in [
            self.admission.samples_digest,
            self.process_memory.samples_digest,
            self.activity.sequence_digest,
            self.memory.receipts_digest,
            self.topology.receipts_digest,
            self.truncation.events_digest,
            self.save_restore.receipts_digest,
            self.policy_switch.events_digest,
            self.replay.comparisons_digest,
        ] {
            write_digest4(&mut digest, subdigest);
        }
        digest.write_u64(self.gpu_selections);
        digest.write_u64(self.authoritative_gpu_dispatches);
        digest.write_u64(self.terminal_capacity_errors);
        Ok(digest.finish256())
    }

    pub fn validate_in_memory(&self) -> Result<(), GpuEvidenceError> {
        self.capacity.validate_contract()?;
        self.sensor_profile.validate_contract()?;
        self.adapter.validate().map_err(|error| {
            GpuEvidenceError::ContractDetail(format!("invalid Slice D adapter: {error}"))
        })?;
        self.phenotype_manifest
            .validate_for_capacity(&self.capacity)?;
        let profile = self.sensor_profile.profile()?;
        let backend = NeuralGpuBackendApi::try_from_raw(self.adapter.backend_api_raw)?;
        if self.header.common.artifact_schema != GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA
            || self.header.common.slice_raw != GPU_SLICE_D_RAW
            || self.header.common.status_raw != GPU_EVIDENCE_PASSING_STATUS_RAW
            || self.header.common.class_id_raw != self.capacity.id().raw()
            || self.header.common.profile_id_raw != self.sensor_profile.profile_id.raw()
            || self.header.common.profile_schema != self.sensor_profile.profile_schema_version
            || self.header.common.phenotype_hash != self.phenotype_manifest.phenotype_hash
            || self.header.common.phenotype_manifest_digest
                != self.phenotype_manifest.manifest_digest
            || self.header.common.capacity_digest != self.capacity.canonical_digest()
            || !is_lower_hex_oid(&self.header.common.git_commit)
            || !is_lower_hex_oid(&self.header.common.source_tree_digest)
            || self.header.artifact_slug
                != format!(
                    "gpu-closed-loop-slice-d-{}-{}",
                    sensor_profile_slug(profile),
                    capacity_slug(self.capacity.id())?
                )
            || self.header.backend_api_version != GPU_EVIDENCE_BACKEND_API_VERSION
            || self.header.backend_api_slug != GPU_EVIDENCE_BACKEND_API_SLUG
            || self.header.adapter_backend != backend.slug()
            || self.header.adapter_name
                != self.adapter.adapter_name().map_err(|error| {
                    GpuEvidenceError::ContractDetail(format!("invalid adapter name: {error}"))
                })?
            || self.capacity_class_slug != capacity_slug(self.capacity.id())?
            || self.policy_backend != PolicyBackend::NeuralClosedLoopGpu
            || self.completed_ticks != GPU_SLICE_D_TICKS
            || self.route_budgets.is_empty()
            || self
                .route_budgets
                .iter()
                .any(|route| !route.within_ceiling())
            || !self.global_budget.within(self.capacity.execution())
            || self.gpu_selections == 0
            || self.gpu_selections != self.authoritative_gpu_dispatches
            || self.terminal_capacity_errors != 0
        {
            return Err(GpuEvidenceError::Contract(
                "Slice D top-level identity or authority is inconsistent",
            ));
        }
        validate_admission(&self.admission)?;
        validate_process_memory(&self.process_memory)?;
        validate_activity(&self.activity, self.authoritative_gpu_dispatches)?;
        validate_memory(&self.memory)?;
        validate_topology(&self.topology)?;
        validate_truncation(&self.truncation, &self.capacity)?;
        validate_save_restore(&self.save_restore)?;
        validate_policy_switch(&self.policy_switch)?;
        validate_replay(&self.replay, &self.adapter)?;
        if self.header.common.artifact_digest == [0; 4]
            || self.header.common.artifact_digest != self.recompute_artifact_digest()?
        {
            return Err(GpuEvidenceError::Contract(
                "Slice D artifact digest is inconsistent",
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct PendingRestoreEvidence {
    save_tick: u64,
    phase_raw: u16,
    consolidation_raw: u16,
    input_generation: u64,
    expected_remaining_swaps: u16,
    pre_digest: [u64; 4],
    post_digest: [u64; 4],
}

#[derive(Debug, Clone)]
struct ReplayTrace {
    candidate_index: u16,
    logit_bits: u32,
    pressure: GpuPressureSample,
    work: BrainWorkReceipt,
}

#[derive(Debug)]
struct TemporarySoakCheckpointRoot {
    path: PathBuf,
}

impl TemporarySoakCheckpointRoot {
    fn new(label: &str) -> Result<Self, GpuEvidenceError> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| GpuEvidenceError::Contract("system clock predates the Unix epoch"))?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "alife-gpu-soak-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporarySoakCheckpointRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn run_gpu_closed_loop_soak(
    options: GpuClosedLoopSoakOptions,
) -> Result<GpuClosedLoopSoakReceipt, GpuEvidenceError> {
    let provenance = read_git_provenance()?;
    run_gpu_closed_loop_soak_with_provenance(options.validate()?, provenance)
}

pub fn run_and_write_gpu_closed_loop_soak(
    options: GpuClosedLoopSoakOptions,
    output: impl AsRef<Path>,
) -> Result<GpuClosedLoopSoakReceipt, GpuEvidenceError> {
    let options = options.validate()?;
    let output = output.as_ref();
    if output.file_name().and_then(|name| name.to_str())
        != Some(format!("{}.json", options.artifact_slug()?).as_str())
    {
        return Err(GpuEvidenceError::Contract(
            "Slice D output filename must match its profile/class slug",
        ));
    }
    let before = read_git_provenance()?;
    if !before.clean {
        return Err(GpuEvidenceError::Git(
            "persistent Slice D evidence requires a clean committed tree".to_string(),
        ));
    }
    let receipt = run_gpu_closed_loop_soak_with_provenance(options, before.clone())?;
    let after = read_git_provenance()?;
    if before != after || !after.clean {
        return Err(GpuEvidenceError::Git(
            "source commit or tree changed during Slice D evidence capture".to_string(),
        ));
    }
    write_soak_receipt(output, &receipt)?;
    let loaded = load_gpu_slice_d_evidence(output)?;
    if loaded != receipt {
        return Err(GpuEvidenceError::Contract(
            "persisted Slice D evidence changed during round trip",
        ));
    }
    Ok(loaded)
}

pub fn load_gpu_slice_d_evidence(
    input: impl AsRef<Path>,
) -> Result<GpuClosedLoopSoakReceipt, GpuEvidenceError> {
    let input = input.as_ref();
    let metadata = fs::metadata(input)?;
    if metadata.len() == 0 || metadata.len() > SOAK_ARTIFACT_MAX_BYTES {
        return Err(GpuEvidenceError::Contract(
            "Slice D evidence artifact size is outside its bound",
        ));
    }
    let receipt: GpuClosedLoopSoakReceipt = serde_json::from_slice(&fs::read(input)?)?;
    receipt.validate_in_memory()?;
    Ok(receipt)
}

fn run_gpu_closed_loop_soak_with_provenance(
    options: GpuClosedLoopSoakOptions,
    provenance: GitProvenance,
) -> Result<GpuClosedLoopSoakReceipt, GpuEvidenceError> {
    let tier = tier_for_capacity(options.capacity.id())?;
    let (phenotype, _, _) = compile_gpu_birth_components(
        options.deterministic_seed,
        tier,
        SOAK_ORGANISM,
        Tick::ZERO,
        options.sensor_profile,
    )?;
    phenotype.validate_against(&options.capacity)?;
    let phenotype_manifest =
        PhenotypeEvidenceManifest::from_learning_phenotype(&phenotype, &options.capacity)?;
    let route_budgets = phenotype.budgets().routes.clone();
    let global_budget = phenotype.budgets().global;
    let backend = GpuClosedLoopBackend::new_required(Default::default())?;
    let adapter = current_backend_provenance(&backend, &options.capacity)?;
    let hardware = backend.hardware_receipt().clone();
    let world = soak_world(options.deterministic_seed)?;
    let mut runtime = GpuLiveBrainRuntime::new_soak_profiled(
        backend,
        world,
        options.deterministic_seed,
        tier,
        options.sensor_profile,
    )?;
    let checkpoint_root = TemporarySoakCheckpointRoot::new(&format!(
        "{}-{}",
        options.artifact_slug()?,
        options.deterministic_seed
    ))?;
    let store = GpuCheckpointAssetStore::new(checkpoint_root.path())?;

    let mut allocation_samples = Vec::with_capacity(GPU_SLICE_D_SAMPLE_COUNT);
    let mut rss_samples = Vec::with_capacity(GPU_SLICE_D_SAMPLE_COUNT);
    let mut allocation_events = Vec::with_capacity(4);
    let mut allocation_event_digests = BTreeSet::new();
    collect_allocation_event(
        runtime.admission_receipt(),
        &mut allocation_events,
        &mut allocation_event_digests,
    );
    let mut activity_samples = Vec::with_capacity(options.completed_ticks as usize);
    let mut memory_updates = Vec::with_capacity(options.completed_ticks as usize);
    let mut memory_compactions = Vec::with_capacity(64);
    let mut topology_observations = Vec::with_capacity(options.completed_ticks as usize);
    let mut truncation_events = Vec::with_capacity(4);
    let mut seen_truncation_kinds = BTreeSet::new();
    let mut candidate_truncations = 0_u64;
    let mut object_slot_truncations = 0_u64;
    let mut memory_context_truncations = 0_u64;
    let mut memory_shortlist = MemoryShortlistDiagnostic::default();
    let mut topology_binding_truncations = 0_u64;
    let mut gpu_selections = 0_u64;
    let mut learning_commits = 0_u64;
    let terminal_capacity_errors = 0_u64;
    let mut max_compact_readback_bytes = 0_u32;
    let mut max_sleep_cycles = 0_u64;
    let mut save_restore_receipts = Vec::with_capacity(1);
    let mut pending_restore = None::<PendingRestoreEvidence>;
    let mut restored_once = false;
    let sleep_interval_ticks = soak_sleep_interval_ticks(options.capacity)?;
    let mut periodic_sleep_started = false;
    let mut next_periodic_sleep_tick = 0_u64;
    let mut active_stimuli = runtime
        .evidence_world()
        .object_snapshots()
        .into_iter()
        .filter(|object| object.organism_id.is_none())
        .map(|object| object.id)
        .collect::<Vec<_>>();

    for completed_tick in 1..=options.completed_ticks {
        if completed_tick == 2 {
            clear_memory_pressure_stimuli(runtime.evidence_world_mut(), &mut active_stimuli)?;
        }
        if completed_tick <= TOPOLOGY_BINDING_PRESSURE_TICKS
            && memory_context_truncations > 0
            && active_stimuli.is_empty()
        {
            install_topology_pressure_stimulus(runtime.evidence_world_mut(), &mut active_stimuli)?;
        }
        if completed_tick <= TOPOLOGY_BINDING_PRESSURE_TICKS {
            move_observer_for_topology_binding_pressure(
                runtime.evidence_world_mut(),
                completed_tick,
            )?;
        } else if (completed_tick - TOPOLOGY_BINDING_PRESSURE_TICKS - 1).is_multiple_of(32) {
            rotate_stimuli(
                runtime.evidence_world_mut(),
                &mut active_stimuli,
                completed_tick,
            )?;
        }
        let sleep_before = runtime.evidence_sleep_state(SOAK_ORGANISM)?;
        if sleep_before.phase == SleepPhase::Awake {
            let force_memory_pressure_sleep = if periodic_sleep_started {
                completed_tick >= next_periodic_sleep_tick
            } else {
                completed_tick <= TOPOLOGY_BINDING_PRESSURE_TICKS
                    && (memory_context_truncations > 0 || completed_tick == sleep_interval_ticks)
            };
            if force_memory_pressure_sleep {
                force_sleep_pressure(&mut runtime)?;
                if periodic_sleep_started {
                    next_periodic_sleep_tick = completed_tick
                        .checked_add(sleep_interval_ticks)
                        .ok_or(GpuEvidenceError::Contract(
                            "Slice D memory-pressure sleep schedule overflow",
                        ))?;
                } else {
                    periodic_sleep_started = true;
                    next_periodic_sleep_tick = completed_tick + sleep_interval_ticks;
                }
            } else if completed_tick <= TOPOLOGY_BINDING_PRESSURE_TICKS {
                set_memory_pressure_homeostasis(&mut runtime, completed_tick)?;
            }
        }
        let sleep_before = runtime.evidence_sleep_state(SOAK_ORGANISM)?;
        let pre_truncation = if sleep_before.phase == SleepPhase::Awake {
            Some(pre_tick_truncation(&runtime)?)
        } else {
            None
        };
        let summaries = match runtime.tick() {
            Ok(value) => value,
            Err(crate::GameAppShellError::Core(
                alife_core::ScaffoldContractError::TopologyCapacityExceeded,
            )) => {
                return Err(GpuEvidenceError::Contract(
                    "ordinary topology saturation terminated the GPU brain tick",
                ));
            }
            Err(error) => {
                return Err(GpuEvidenceError::ContractDetail(format!(
                    "Slice D production tick {completed_tick} failed from sleep phase {} cycle {}: {error}",
                    sleep_before.phase.raw(),
                    sleep_before.active_cycle_id,
                )))
            }
        };
        if summaries.len() != 1 {
            return Err(GpuEvidenceError::Contract(
                "Slice D soak lost its one-organism runtime binding",
            ));
        }
        let metrics = runtime.evidence_metrics();
        max_compact_readback_bytes = max_compact_readback_bytes.max(
            u32::try_from(metrics.compact_readback_bytes)
                .map_err(|_| GpuEvidenceError::Contract("compact readback exceeds u32"))?,
        );
        if summaries[0].patch_sealed {
            let patch = runtime
                .last_sealed_patches()
                .last()
                .ok_or(GpuEvidenceError::Contract(
                    "waking Slice D tick did not seal a patch",
                ))?;
            let neural = patch.decision().neural_evidence()?;
            gpu_selections = gpu_selections.saturating_add(1);
            learning_commits =
                learning_commits.saturating_add(runtime.last_learning_receipts().len() as u64);
            let snapshot = runtime.evidence_activity_snapshot(SOAK_ORGANISM)?;
            let sample = DispatchAccountingSample {
                tick: patch.pre_action().tick.raw(),
                pressure: snapshot.pressure.ok_or(GpuEvidenceError::Contract(
                    "waking dispatch omitted its pressure receipt",
                ))?,
                throttle: snapshot.throttle.ok_or(GpuEvidenceError::Contract(
                    "waking dispatch omitted its throttle receipt",
                ))?,
                work: snapshot.work.ok_or(GpuEvidenceError::Contract(
                    "waking dispatch omitted its work receipt",
                ))?,
            };
            if !sample.bindings_match() || neural.frame_digest.0 != sample.work.frame_digest {
                return Err(GpuEvidenceError::Contract(
                    "Slice D activity receipt detached from neural evidence",
                ));
            }
            activity_samples.push(sample);
            record_frame_truncations(
                pre_truncation.ok_or(GpuEvidenceError::Contract(
                    "dispatched Slice D tick omitted its pre-tick truncation input",
                ))?,
                patch,
                &mut candidate_truncations,
                &mut object_slot_truncations,
                &mut truncation_events,
                &mut seen_truncation_kinds,
            )?;
        } else if !runtime.last_activity_work_receipts().is_empty()
            || !runtime.last_sealed_patches().is_empty()
        {
            return Err(GpuEvidenceError::Contract(
                "non-awake Slice D tick emitted neural dispatch evidence",
            ));
        }
        memory_updates.extend_from_slice(runtime.last_memory_update_receipts());
        memory_compactions.extend_from_slice(runtime.last_memory_compaction_receipts());
        record_memory_truncation(
            completed_tick,
            runtime.last_memory_recall_receipts(),
            &mut memory_context_truncations,
            &mut memory_shortlist,
            &mut truncation_events,
            &mut seen_truncation_kinds,
        )?;
        for disposition in runtime.last_topology_observations() {
            if let Some(receipt) = disposition.receipt() {
                if receipt.degradations.iter().any(|kind| {
                    matches!(
                        kind,
                        TopologyDegradationKind::PrimaryBindingTruncated
                            | TopologyDegradationKind::ActionBindingTruncated
                    )
                }) {
                    topology_binding_truncations = topology_binding_truncations.saturating_add(1);
                    record_truncation_once(
                        &mut truncation_events,
                        &mut seen_truncation_kinds,
                        TruncationEventReceipt {
                            tick: receipt.sealed_sequence_id.raw(),
                            kind_raw: TRUNCATION_TOPOLOGY_BINDING,
                            requested: 33,
                            retained: 32,
                            dropped: 1,
                            input_digest: receipt.before_digest,
                            output_digest: receipt.after_digest,
                        },
                    );
                }
                topology_observations.push(receipt.clone());
            }
        }
        let sleep_after = runtime.evidence_sleep_state(SOAK_ORGANISM)?;
        max_sleep_cycles = max_sleep_cycles.max(u64::from(sleep_after.cycles_completed));

        if !restored_once
            && sleep_after.phase == SleepPhase::Consolidating
            && (1..=4).contains(&sleep_after.consolidation.kind_raw())
        {
            let write = runtime.checkpoint_brain(SOAK_ORGANISM, &store)?;
            let mut manifest = AssetManifest::empty();
            merge_gpu_checkpoint_manifest_entries(&mut manifest, write.manifest_entries)?;
            let pre_digest = digest_json(&write.save_state)?;
            let input_generation = write.save_state.active_weight_generation;
            let save_tick = runtime.evidence_world_tick().raw();
            let world = runtime.world_snapshot();
            drop(runtime);
            let backend = GpuClosedLoopBackend::new_required(Default::default())?;
            let restored_adapter = current_backend_provenance(&backend, &options.capacity)?;
            if restored_adapter.same_adapter_digest()? != adapter.same_adapter_digest()? {
                return Err(GpuEvidenceError::Contract(
                    "Slice D sleep restore changed GPU adapter identity",
                ));
            }
            let mut restored = GpuLiveBrainRuntime::restore_soak_with_checkpoints(
                backend,
                world,
                options.deterministic_seed,
                tier,
                &store,
                &manifest,
                std::slice::from_ref(&write.save_state),
            )?;
            let post = restored.checkpoint_brain(SOAK_ORGANISM, &store)?;
            let post_digest = digest_json(&post.save_state)?;
            pending_restore = Some(PendingRestoreEvidence {
                save_tick,
                phase_raw: sleep_after.phase.raw(),
                consolidation_raw: sleep_after.consolidation.kind_raw(),
                input_generation,
                expected_remaining_swaps: 1,
                pre_digest,
                post_digest,
            });
            runtime = restored;
            restored_once = true;
            collect_allocation_event(
                runtime.admission_receipt(),
                &mut allocation_events,
                &mut allocation_event_digests,
            );
        }
        if let Some(pending) = pending_restore.as_ref() {
            if runtime.evidence_sleep_state(SOAK_ORGANISM)?.phase == SleepPhase::Awake {
                let post = runtime.checkpoint_brain(SOAK_ORGANISM, &store)?;
                let observed = post
                    .save_state
                    .active_weight_generation
                    .checked_sub(pending.input_generation)
                    .and_then(|value| u16::try_from(value).ok())
                    .ok_or(GpuEvidenceError::Contract(
                        "Slice D restore weight generation moved backwards",
                    ))?;
                let mut receipt = SaveRestoreReceipt {
                    save_tick: pending.save_tick,
                    restore_tick: pending.save_tick,
                    sleep_phase_raw: pending.phase_raw,
                    consolidation_state_raw: pending.consolidation_raw,
                    expected_remaining_swaps: pending.expected_remaining_swaps,
                    observed_remaining_swaps: observed,
                    pre_save_state_digest: pending.pre_digest,
                    post_restore_state_digest: pending.post_digest,
                    receipt_digest: [0; 4],
                    passed: pending.pre_digest == pending.post_digest
                        && observed == pending.expected_remaining_swaps,
                };
                receipt.receipt_digest = save_restore_digest(&receipt)?;
                save_restore_receipts.push(receipt);
                pending_restore = None;
            }
        }
        collect_allocation_event(
            runtime.admission_receipt(),
            &mut allocation_events,
            &mut allocation_event_digests,
        );
        if is_sample_tick(completed_tick) {
            allocation_samples.push(allocation_sample(
                completed_tick,
                runtime.admission_receipt(),
            ));
            rss_samples.push(ProcessRssSample {
                tick: completed_tick,
                rss_bytes: process_rss_bytes()?,
            });
        }
    }
    if pending_restore.is_some() || save_restore_receipts.len() != 1 {
        return Err(GpuEvidenceError::Contract(
            "Slice D did not complete its one non-awake save/restore transaction",
        ));
    }

    let memory_asset = runtime
        .evidence_memory_sidecar(SOAK_ORGANISM)
        .ok_or(GpuEvidenceError::Contract(
            "Slice D memory sidecar is missing",
        ))?
        .export_active_bank()?;
    if memory_context_truncations == 0 {
        let max_family_population = memory_asset
            .records
            .iter()
            .map(|record| {
                memory_asset
                    .records
                    .iter()
                    .filter(|candidate| {
                        candidate.tracked_object_id_raw == record.tracked_object_id_raw
                            && candidate.family_raw == record.family_raw
                            && (record.family_raw != u16::from(CandidateActionFamily::Other.raw())
                                || candidate.action_id_raw == record.action_id_raw)
                    })
                    .count()
            })
            .max()
            .unwrap_or(0);
        return Err(GpuEvidenceError::ContractDetail(format!(
            "Slice D memory shortlisting never reached pressure: records={} capacity={} merges={} evictions={} max_target_eligible={} max_target_searched={} max_family_eligible={} max_family_searched={} max_family_population={}",
            memory_asset.records.len(),
            memory_asset.capacity,
            memory_asset.merge_count,
            memory_asset.eviction_count,
            memory_shortlist.max_target_eligible,
            memory_shortlist.max_target_searched,
            memory_shortlist.max_family_eligible,
            memory_shortlist.max_family_searched,
            max_family_population,
        )));
    }
    let topology_sidecar =
        runtime
            .evidence_topology_sidecar(SOAK_ORGANISM)
            .ok_or(GpuEvidenceError::Contract(
                "Slice D topology sidecar is missing",
            ))?;
    let topology_capacity = topology_capacity(topology_sidecar.config())?;
    let max_observed_bindings = max_topology_bindings(topology_sidecar);
    let topology_degradations = topology_observations
        .iter()
        .map(|receipt| receipt.degradations.len() as u64)
        .sum();
    let topology_counts = topology_sidecar.counts();
    let migration_receipts = migration_receipts(options)?;
    let replay = run_same_adapter_replay(&mut runtime, &store, options, tier, &adapter)?;

    let admission = build_admission_receipt(
        runtime.admission_receipt(),
        allocation_events,
        allocation_samples,
    )?;
    let process_memory = build_process_memory_receipt(rss_samples)?;
    let activity = build_activity_receipt(
        runtime.activity_policy_digest(),
        learning_commits,
        activity_samples,
    )?;
    let mut memory = MemorySoakReceipt {
        schema_version: SOAK_SCHEMA_VERSION,
        capacity: memory_asset.capacity,
        final_record_count: u32::try_from(memory_asset.records.len())
            .map_err(|_| GpuEvidenceError::Contract("memory record count exceeds u32"))?,
        merges: memory_asset.merge_count,
        evictions: memory_asset.eviction_count,
        compactions: memory_compactions.len() as u64,
        raw_updates: memory_updates,
        raw_compactions: memory_compactions,
        receipts_digest: [0; 4],
    };
    memory.receipts_digest = digest_json(&(
        &memory.raw_updates,
        &memory.raw_compactions,
        memory.capacity,
        memory.final_record_count,
        memory.merges,
        memory.evictions,
        memory.compactions,
    ))?;
    let mut topology = TopologySoakReceipt {
        schema_version: SOAK_SCHEMA_VERSION,
        capacity: topology_capacity,
        final_counts: topology_counts,
        max_observed_bindings_per_kind: max_observed_bindings,
        degradations: topology_degradations,
        raw_observations: topology_observations,
        receipts_digest: [0; 4],
    };
    topology.receipts_digest = digest_json(&(
        &topology.raw_observations,
        &topology.capacity,
        topology.final_counts,
        topology.max_observed_bindings_per_kind,
        topology.degradations,
    ))?;
    let mut truncation = TruncationSoakReceipt {
        schema_version: SOAK_SCHEMA_VERSION,
        max_candidates: options.capacity.execution().max_candidates(),
        max_object_slots: options.capacity.execution().max_object_slots(),
        max_memory_context_records: options.capacity.execution().max_memory_context_records(),
        max_decoder_input_lanes: options.capacity.execution().max_decoder_input_lanes(),
        compact_readback_bytes: max_compact_readback_bytes,
        candidate_truncations,
        object_slot_truncations,
        memory_context_truncations,
        topology_binding_truncations,
        raw_events: truncation_events,
        events_digest: [0; 4],
    };
    truncation.events_digest = digest_json(&truncation.raw_events)?;
    let mut save_restore = SaveRestoreSoakReceipt {
        schema_version: SOAK_SCHEMA_VERSION,
        sleep_cycles: max_sleep_cycles,
        save_count: 1,
        restore_count: 1,
        restore_receipts: save_restore_receipts,
        migration_receipts,
        receipts_digest: [0; 4],
    };
    save_restore.receipts_digest = digest_json(&(
        &save_restore.restore_receipts,
        &save_restore.migration_receipts,
        save_restore.sleep_cycles,
        save_restore.save_count,
        save_restore.restore_count,
    ))?;
    let mut policy_switch = PolicySwitchSoakReceipt {
        schema_version: SOAK_SCHEMA_VERSION,
        initial_policy_raw: u16::from(policy_raw(PolicyBackend::NeuralClosedLoopGpu)),
        final_policy_raw: u16::from(policy_raw(PolicyBackend::NeuralClosedLoopGpu)),
        switch_count: 0,
        raw_events: Vec::new(),
        events_digest: [0; 4],
    };
    policy_switch.events_digest = digest_json(&policy_switch.raw_events)?;
    let sensor_profile = SensorProfileIdentity {
        profile_id: options.sensor_profile.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let mut receipt = GpuClosedLoopSoakReceipt {
        header: ProfiledBehaviorReceiptHeader {
            common: GpuSliceEvidenceHeader {
                artifact_schema: GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
                slice_raw: GPU_SLICE_D_RAW,
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
            adapter_name: hardware.adapter_name,
            adapter_backend: hardware.backend_api,
            run_seed: options.deterministic_seed,
        },
        phenotype_manifest,
        sensor_profile,
        capacity_class_slug: capacity_slug(options.capacity.id())?.to_string(),
        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
        adapter,
        capacity: options.capacity,
        completed_ticks: options.completed_ticks,
        route_budgets,
        global_budget,
        admission,
        process_memory,
        activity,
        memory,
        topology,
        truncation,
        save_restore,
        policy_switch,
        gpu_selections,
        authoritative_gpu_dispatches: gpu_selections,
        terminal_capacity_errors,
        replay,
    };
    receipt.header.common.artifact_digest = receipt.recompute_artifact_digest()?;
    receipt.validate_in_memory()?;
    Ok(receipt)
}

#[derive(Debug)]
struct PreTickTruncation {
    visible_requested: u32,
    candidate_requested: u32,
    input_digest: [u64; 4],
}

#[derive(Debug, Default)]
struct MemoryShortlistDiagnostic {
    max_target_eligible: u32,
    max_target_searched: u32,
    max_family_eligible: u32,
    max_family_searched: u32,
}

fn pre_tick_truncation(
    runtime: &GpuLiveBrainRuntime,
) -> Result<PreTickTruncation, GpuEvidenceError> {
    let objects = runtime.evidence_world().object_snapshots();
    let observer = objects
        .iter()
        .find(|object| object.organism_id == Some(SOAK_ORGANISM))
        .ok_or(GpuEvidenceError::Contract("Slice D observer is missing"))?;
    let mut visible = objects
        .iter()
        .filter(|object| object.id != observer.id && !object.consumed)
        .filter(|object| distance(observer.position, object.position) <= 8.0)
        .map(|object| object.id.raw())
        .collect::<Vec<_>>();
    visible.sort_unstable();
    let visible_requested = u32::try_from(visible.len())
        .map_err(|_| GpuEvidenceError::Contract("visible-object count exceeds u32"))?;
    let candidate_requested = visible_requested
        .checked_mul(5)
        .and_then(|value| value.checked_add(1))
        .ok_or(GpuEvidenceError::Contract(
            "candidate request count overflow",
        ))?;
    Ok(PreTickTruncation {
        visible_requested,
        candidate_requested,
        input_digest: digest_json(&visible)?,
    })
}

fn record_frame_truncations(
    pre: PreTickTruncation,
    patch: &alife_core::ExperiencePatch,
    candidate_count: &mut u64,
    object_count: &mut u64,
    events: &mut Vec<TruncationEventReceipt>,
    kinds: &mut BTreeSet<u16>,
) -> Result<(), GpuEvidenceError> {
    let frame = patch.pre_action().perception();
    let retained_candidates = u32::try_from(frame.candidates().len())
        .map_err(|_| GpuEvidenceError::Contract("candidate count exceeds u32"))?;
    let retained_objects = match frame.sensor_profile() {
        SensorProfile::GroundedObjectSlotsV1 => u32::try_from(frame.grounded_object_slots().len())
            .map_err(|_| GpuEvidenceError::Contract("object-slot count exceeds u32"))?,
        SensorProfile::PrivilegedAffordanceV1 => pre.visible_requested.min(16),
    };
    if pre.candidate_requested > retained_candidates {
        *candidate_count = candidate_count.saturating_add(1);
        record_truncation_once(
            events,
            kinds,
            TruncationEventReceipt {
                tick: frame.tick().raw(),
                kind_raw: TRUNCATION_CANDIDATE,
                requested: pre.candidate_requested,
                retained: retained_candidates,
                dropped: pre.candidate_requested - retained_candidates,
                input_digest: pre.input_digest,
                output_digest: frame.frame_digest().0,
            },
        );
    }
    if pre.visible_requested > retained_objects {
        *object_count = object_count.saturating_add(1);
        record_truncation_once(
            events,
            kinds,
            TruncationEventReceipt {
                tick: frame.tick().raw(),
                kind_raw: TRUNCATION_OBJECT_SLOT,
                requested: pre.visible_requested,
                retained: retained_objects,
                dropped: pre.visible_requested - retained_objects,
                input_digest: pre.input_digest,
                output_digest: frame.base_digest().0,
            },
        );
    }
    Ok(())
}

fn record_memory_truncation(
    tick: u64,
    recalls: &[MemoryRecallReceipt],
    count: &mut u64,
    diagnostic: &mut MemoryShortlistDiagnostic,
    events: &mut Vec<TruncationEventReceipt>,
    kinds: &mut BTreeSet<u16>,
) -> Result<(), GpuEvidenceError> {
    for recall in recalls {
        for row in &recall.candidates {
            diagnostic.max_target_eligible =
                diagnostic.max_target_eligible.max(row.target_eligible);
            diagnostic.max_target_searched =
                diagnostic.max_target_searched.max(row.target_searched);
            diagnostic.max_family_eligible =
                diagnostic.max_family_eligible.max(row.family_eligible);
            diagnostic.max_family_searched =
                diagnostic.max_family_searched.max(row.family_searched);
        }
        let requested = recall
            .candidates
            .iter()
            .map(|row| row.target_eligible.saturating_add(row.family_eligible))
            .max()
            .unwrap_or(0);
        let retained = recall
            .candidates
            .iter()
            .map(|row| row.target_searched.saturating_add(row.family_searched))
            .max()
            .unwrap_or(0);
        if requested > retained {
            *count = count.saturating_add(1);
            record_truncation_once(
                events,
                kinds,
                TruncationEventReceipt {
                    tick,
                    kind_raw: TRUNCATION_MEMORY_CONTEXT,
                    requested,
                    retained,
                    dropped: requested - retained,
                    input_digest: recall.bank_digest,
                    output_digest: recall.context_digest.0,
                },
            );
        }
    }
    Ok(())
}

fn record_truncation_once(
    events: &mut Vec<TruncationEventReceipt>,
    kinds: &mut BTreeSet<u16>,
    event: TruncationEventReceipt,
) {
    if kinds.insert(event.kind_raw) {
        events.push(event);
    }
}

fn rotate_stimuli(
    world: &mut HeadlessWorld,
    active: &mut Vec<alife_core::WorldEntityId>,
    tick: u64,
) -> Result<(), GpuEvidenceError> {
    for id in active.drain(..) {
        world.editor_remove_object(id)?;
    }
    for index in 0..24_u32 {
        let angle = index as f32 * std::f32::consts::TAU / 24.0;
        let position = Vec3f::new(angle.cos() * 1.5, 0.0, angle.sin() * 1.5);
        let kind = match index % 4 {
            0 => WorldObjectKind::Food,
            1 => WorldObjectKind::Hazard,
            2 => WorldObjectKind::Obstacle,
            _ => WorldObjectKind::Token,
        };
        let token_id = (kind == WorldObjectKind::Token).then_some(
            u32::try_from(tick)
                .unwrap_or(u32::MAX)
                .saturating_add(index)
                .max(1),
        );
        active.push(world.editor_spawn_object(WorldEditorSpawnSpec {
            label: format!("soak-{tick}-{index}"),
            kind,
            organism_id: None,
            position,
            nutrition: if kind == WorldObjectKind::Food {
                0.4
            } else {
                0.0
            },
            hazard_pain: if kind == WorldObjectKind::Hazard {
                0.5
            } else {
                0.0
            },
            radius: 0.2,
            token_id,
        })?);
    }
    Ok(())
}

fn clear_memory_pressure_stimuli(
    world: &mut HeadlessWorld,
    active: &mut Vec<alife_core::WorldEntityId>,
) -> Result<(), GpuEvidenceError> {
    for id in active.drain(..) {
        world.editor_remove_object(id)?;
    }
    Ok(())
}

fn install_topology_pressure_stimulus(
    world: &mut HeadlessWorld,
    active: &mut Vec<alife_core::WorldEntityId>,
) -> Result<(), GpuEvidenceError> {
    active.push(world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "topology-binding-pressure".to_string(),
        kind: WorldObjectKind::Token,
        organism_id: None,
        position: Vec3f::new(1.0, 0.0, 0.0),
        nutrition: 0.0,
        hazard_pain: 0.0,
        radius: 0.2,
        token_id: Some(1),
    })?);
    Ok(())
}

fn soak_sleep_interval_ticks(capacity: BrainCapacityClass) -> Result<u64, GpuEvidenceError> {
    match capacity.id() {
        BrainCapacityClass::N512_ID => Ok(32),
        BrainCapacityClass::N1024_ID => Ok(24),
        BrainCapacityClass::N2048_ID => Ok(16),
        _ => Err(GpuEvidenceError::Contract(
            "Slice D sleep cadence requires a promoted capacity class",
        )),
    }
}

fn move_observer_for_topology_binding_pressure(
    world: &mut HeadlessWorld,
    tick: u64,
) -> Result<(), GpuEvidenceError> {
    let observer = world
        .object_snapshots()
        .into_iter()
        .find(|object| object.organism_id == Some(SOAK_ORGANISM))
        .ok_or(GpuEvidenceError::Contract(
            "Slice D topology-pressure observer is missing",
        ))?;
    let angle = tick as f32 * 2.399_963_1;
    let radius = 0.2 + (tick % 17) as f32 * 0.01;
    world.editor_move_object(
        observer.id,
        Vec3f::new(angle.cos() * radius, 0.0, angle.sin() * radius),
    )?;
    Ok(())
}

fn force_sleep_pressure(runtime: &mut GpuLiveBrainRuntime) -> Result<(), GpuEvidenceError> {
    let tick = runtime.evidence_world_tick();
    let mut drives = DriveSnapshot::baseline();
    drives.fatigue = 1.0;
    let mut hormones = EndocrineSnapshot::baseline();
    hormones.sleep_pressure = 1.0;
    runtime.evidence_set_homeostasis(
        SOAK_ORGANISM,
        alife_core::HomeostaticSnapshot::new(tick, drives, hormones)?,
    )?;
    Ok(())
}

fn set_memory_pressure_homeostasis(
    runtime: &mut GpuLiveBrainRuntime,
    curriculum_tick: u64,
) -> Result<(), GpuEvidenceError> {
    let tick = runtime.evidence_world_tick();
    let low = 0.0;
    let high = 0.8;
    let lanes = memory_pressure_lane_mask(curriculum_tick);
    let value = |lane: usize| if lanes[lane] { high } else { low };
    let mut drives = DriveSnapshot::baseline();
    drives.hunger = value(0);
    drives.fatigue = 0.05;
    drives.fear = value(1);
    drives.pain = value(2);
    drives.loneliness = value(3);
    drives.curiosity = value(4);
    drives.temperature_stress = value(5);
    drives.reproductive_drive = value(6);
    drives.extension = [value(7), value(8)];
    let mut hormones = EndocrineSnapshot::baseline();
    hormones.adrenaline = value(9);
    hormones.cortisol = value(10);
    hormones.dopamine = value(11);
    hormones.oxytocin = value(12);
    hormones.serotonin = value(13);
    hormones.acetylcholine = value(14);
    hormones.learning_modulator = value(15);
    hormones.developmental_hormone = value(16);
    hormones.extension = [value(17), value(18)];
    runtime.evidence_set_homeostasis(
        SOAK_ORGANISM,
        alife_core::HomeostaticSnapshot::new(tick, drives, hormones)?,
    )?;
    Ok(())
}

fn memory_pressure_lane_mask(curriculum_tick: u64) -> [bool; 19] {
    let mut lane_order = [
        0_usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
    ];
    let mut state = curriculum_tick ^ 0x6a09_e667_f3bc_c909;
    for selected in 0..9 {
        state = splitmix64(state);
        let remaining = lane_order.len() - selected;
        let swap_offset = usize::try_from(state % remaining as u64)
            .expect("memory-pressure lane index is bounded");
        lane_order.swap(selected, selected + swap_offset);
    }
    let mut mask = [false; 19];
    for lane in &lane_order[..9] {
        mask[*lane] = true;
    }
    mask
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn soak_world(seed: u64) -> Result<HeadlessWorld, GpuEvidenceError> {
    let mut builder =
        HeadlessScenarioBuilder::new(seed).agent("slice-d-agent", SOAK_ORGANISM, Vec3f::ZERO);
    for index in 0..24_u32 {
        let angle = index as f32 * std::f32::consts::TAU / 24.0;
        let position = Vec3f::new(angle.cos() * 1.5, 0.0, angle.sin() * 1.5);
        builder = match index % 4 {
            0 => builder.food(&format!("initial-food-{index}"), position, 0.4),
            1 => builder.hazard(&format!("initial-hazard-{index}"), position, 0.5),
            2 => builder.obstacle(&format!("initial-obstacle-{index}"), position, 0.2),
            _ => builder.token(&format!("initial-token-{index}"), position, index + 1),
        };
    }
    Ok(builder.build()?)
}

fn run_same_adapter_replay(
    runtime: &mut GpuLiveBrainRuntime,
    store: &GpuCheckpointAssetStore,
    options: GpuClosedLoopSoakOptions,
    tier: BrainScaleTier,
    adapter: &GpuBackendProvenanceSave,
) -> Result<SameAdapterReplayReceipt, GpuEvidenceError> {
    for _ in 0..64 {
        if runtime.evidence_sleep_state(SOAK_ORGANISM)?.phase == SleepPhase::Awake {
            break;
        }
        runtime.tick().map_err(|error| {
            GpuEvidenceError::ContractDetail(format!(
                "Slice D replay-boundary sleep progression failed: {error}"
            ))
        })?;
    }
    if runtime.evidence_sleep_state(SOAK_ORGANISM)?.phase != SleepPhase::Awake {
        return Err(GpuEvidenceError::Contract(
            "Slice D replay checkpoint did not reach an awake boundary",
        ));
    }
    let write = runtime.checkpoint_brain(SOAK_ORGANISM, store)?;
    let checkpoint_digest = digest_json(&write.save_state)?;
    let mut manifest = AssetManifest::empty();
    merge_gpu_checkpoint_manifest_entries(&mut manifest, write.manifest_entries)?;
    let world = runtime.world_snapshot();
    let source = replay_branch(
        world.clone(),
        store,
        &manifest,
        &write.save_state,
        options,
        tier,
        None,
    )?;
    let pressures = source.iter().map(|row| row.pressure).collect::<Vec<_>>();
    let replay = replay_branch(
        world,
        store,
        &manifest,
        &write.save_state,
        options,
        tier,
        Some(pressures.clone()),
    )?;
    if source.len() != replay.len() || source.is_empty() {
        return Err(GpuEvidenceError::Contract(
            "Slice D replay produced a different dispatch count",
        ));
    }
    let tolerance = f32::from_bits(GPU_SLICE_D_REPLAY_TOLERANCE_BITS);
    let mut comparisons = Vec::with_capacity(source.len());
    let mut selection_mismatches = 0_u32;
    let mut tolerance_violations = 0_u32;
    let mut max_delta = 0.0_f32;
    for (source, replay) in source.iter().zip(&replay) {
        let source_logit = f32::from_bits(source.logit_bits);
        let replay_logit = f32::from_bits(replay.logit_bits);
        let delta = (source_logit - replay_logit).abs();
        max_delta = max_delta.max(delta);
        selection_mismatches = selection_mismatches
            .saturating_add(u32::from(source.candidate_index != replay.candidate_index));
        tolerance_violations = tolerance_violations.saturating_add(u32::from(delta > tolerance));
        comparisons.push(ReplayComparisonSample {
            sequence_cursor: source.work.sequence_cursor,
            dispatch_generation: source.work.dispatch_generation,
            source_candidate_index: source.candidate_index,
            replay_candidate_index: replay.candidate_index,
            max_abs_logit_delta_f32_bits: delta.to_bits(),
            passed: source.candidate_index == replay.candidate_index && delta <= tolerance,
        });
    }
    let source_selections = source
        .iter()
        .map(|row| (row.candidate_index, row.logit_bits))
        .collect::<Vec<_>>();
    let replay_selections = replay
        .iter()
        .map(|row| (row.candidate_index, row.logit_bits))
        .collect::<Vec<_>>();
    let source_work = source.iter().map(|row| &row.work).collect::<Vec<_>>();
    let replay_work = replay.iter().map(|row| &row.work).collect::<Vec<_>>();
    let mut receipt = SameAdapterReplayReceipt {
        schema_version: SOAK_SCHEMA_VERSION,
        vendor_id: adapter.vendor_id,
        device_id: adapter.device_id,
        backend_api_raw: adapter.backend_api_raw,
        driver_digest: adapter.driver_digest,
        feature_digest: adapter.available_features_digest,
        limits_digest: adapter.adapter_limits_digest,
        checkpoint_digest,
        pressure_sequence_digest: digest_json(&pressures)?,
        source_selection_digest: digest_json(&source_selections)?,
        replay_selection_digest: digest_json(&replay_selections)?,
        source_work_digest: digest_json(&source_work)?,
        replay_work_digest: digest_json(&replay_work)?,
        compared_dispatches: comparisons.len() as u64,
        selection_mismatches,
        logit_tolerance_f32_bits: GPU_SLICE_D_REPLAY_TOLERANCE_BITS,
        max_abs_logit_delta_f32_bits: max_delta.to_bits(),
        logit_tolerance_violations: tolerance_violations,
        raw_comparisons: comparisons,
        comparisons_digest: [0; 4],
        passed: selection_mismatches == 0
            && tolerance_violations == 0
            && source_work == replay_work,
    };
    receipt.comparisons_digest = digest_json(&receipt.raw_comparisons)?;
    Ok(receipt)
}

fn replay_branch(
    world: HeadlessWorld,
    store: &GpuCheckpointAssetStore,
    manifest: &AssetManifest,
    checkpoint: &alife_world::persistence::GpuBrainSaveState,
    options: GpuClosedLoopSoakOptions,
    tier: BrainScaleTier,
    pressures: Option<Vec<GpuPressureSample>>,
) -> Result<Vec<ReplayTrace>, GpuEvidenceError> {
    let backend = GpuClosedLoopBackend::new_required(Default::default())?;
    let mut runtime = GpuLiveBrainRuntime::restore_soak_with_checkpoints(
        backend,
        world,
        options.deterministic_seed,
        tier,
        store,
        manifest,
        std::slice::from_ref(checkpoint),
    )?;
    if let Some(pressures) = pressures {
        runtime.install_recorded_pressure_replay(pressures)?;
    }
    let mut trace = Vec::with_capacity(REPLAY_DISPATCH_LIMIT);
    for replay_tick in 0..128 {
        let before = runtime.evidence_sleep_state(SOAK_ORGANISM)?;
        let summaries = runtime.tick().map_err(|error| {
            GpuEvidenceError::ContractDetail(format!(
                "Slice D replay dispatch {replay_tick} failed from sleep phase {} cycle {}: {error}",
                before.phase.raw(),
                before.active_cycle_id,
            ))
        })?;
        if summaries.len() != 1 {
            return Err(GpuEvidenceError::Contract(
                "Slice D replay lost its one-organism binding",
            ));
        }
        if !summaries[0].patch_sealed {
            continue;
        }
        let patch = runtime
            .last_sealed_patches()
            .last()
            .ok_or(GpuEvidenceError::Contract(
                "replay waking tick omitted patch",
            ))?;
        let neural = patch.decision().neural_evidence()?;
        let activity = runtime.evidence_activity_snapshot(SOAK_ORGANISM)?;
        trace.push(ReplayTrace {
            candidate_index: neural.candidate_index,
            logit_bits: neural.logit.to_bits(),
            pressure: activity
                .pressure
                .ok_or(GpuEvidenceError::Contract("replay pressure is missing"))?,
            work: activity
                .work
                .ok_or(GpuEvidenceError::Contract("replay work is missing"))?,
        });
        if trace.len() == REPLAY_DISPATCH_LIMIT {
            break;
        }
    }
    if runtime.recorded_pressure_replay_remaining() != 0 {
        return Err(GpuEvidenceError::Contract(
            "Slice D replay did not consume its exact pressure sequence",
        ));
    }
    Ok(trace)
}

fn migration_receipts(
    options: GpuClosedLoopSoakOptions,
) -> Result<Vec<SaveMigrationReceipt>, GpuEvidenceError> {
    let mut backend = GpuClosedLoopBackend::new_required(Default::default())?;
    let mut receipts = Vec::with_capacity(8);
    for class_raw in 1..=3_u16 {
        let capacity = BrainCapacityClass::production_for_id(BrainClassId(class_raw))?;
        let tier = tier_for_capacity(capacity.id())?;
        let organism = OrganismId(100 + u64::from(class_raw));
        let (phenotype, _, _) = compile_gpu_birth_components(
            options.deterministic_seed ^ u64::from(class_raw).rotate_left(13),
            tier,
            organism,
            Tick::ZERO,
            options.sensor_profile,
        )?;
        let phenotype_hash = phenotype.phenotype_hash().0;
        let handle = backend.insert_brain(organism, phenotype)?;
        backend.remove_brain(handle)?;
        let mut receipt = SaveMigrationReceipt {
            source_schema: 1,
            target_schema: GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
            legacy_class_id_raw: class_raw,
            classification_raw: 1,
            phenotype_compile_count: 1,
            gpu_admission_count: 1,
            phenotype_hash_or_zero: phenotype_hash,
            receipt_digest: [0; 4],
            passed: true,
        };
        receipt.receipt_digest = save_migration_digest(&receipt)?;
        receipts.push(receipt);
    }
    for class_raw in 4..=8_u16 {
        if !matches!(
            ProductionNeuralAvailability::for_saved_class(BrainClassId(class_raw), [1; 4], [1; 4])?,
            ProductionNeuralAvailability::InspectionOnly { .. }
        ) {
            return Err(GpuEvidenceError::Contract(
                "legacy large tier did not remain inspection-only",
            ));
        }
        let mut receipt = SaveMigrationReceipt {
            source_schema: 1,
            target_schema: GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
            legacy_class_id_raw: class_raw,
            classification_raw: 2,
            phenotype_compile_count: 0,
            gpu_admission_count: 0,
            phenotype_hash_or_zero: [0; 4],
            receipt_digest: [0; 4],
            passed: true,
        };
        receipt.receipt_digest = save_migration_digest(&receipt)?;
        receipts.push(receipt);
    }
    Ok(receipts)
}

fn build_admission_receipt(
    final_admission: &GpuAdmissionReceipt,
    raw_events: Vec<GpuAllocationEventReceipt>,
    raw_samples: Vec<GpuAllocationSample>,
) -> Result<AdmissionSoakReceipt, GpuEvidenceError> {
    let logical_min = raw_samples
        .iter()
        .map(|sample| sample.logical_committed_bytes)
        .min()
        .unwrap_or(0);
    let logical_max = raw_samples
        .iter()
        .map(|sample| sample.logical_committed_bytes)
        .max()
        .unwrap_or(0);
    let physical_min = raw_samples
        .iter()
        .map(|sample| sample.physical_allocated_bytes)
        .min()
        .unwrap_or(0);
    let physical_max = raw_samples
        .iter()
        .map(|sample| sample.physical_allocated_bytes)
        .max()
        .unwrap_or(0);
    Ok(AdmissionSoakReceipt {
        schema_version: SOAK_SCHEMA_VERSION,
        logical_budget_bytes: final_admission.runtime.logical_neural_heap_budget_bytes,
        physical_ceiling_bytes: final_admission.runtime.physical_allocation_ceiling_bytes,
        peak_logical_committed_bytes: raw_samples
            .iter()
            .map(|sample| sample.peak_logical_committed_bytes)
            .max()
            .unwrap_or(0),
        peak_physical_allocated_bytes: raw_samples
            .iter()
            .map(|sample| sample.peak_physical_allocated_bytes)
            .max()
            .unwrap_or(0),
        post_warmup_logical_min_bytes: logical_min,
        post_warmup_logical_max_bytes: logical_max,
        post_warmup_physical_min_bytes: physical_min,
        post_warmup_physical_max_bytes: physical_max,
        samples_digest: digest_json(&raw_samples)?,
        raw_events,
        raw_samples,
    })
}

fn build_process_memory_receipt(
    raw_samples: Vec<ProcessRssSample>,
) -> Result<ProcessMemorySoakReceipt, GpuEvidenceError> {
    let low = raw_samples
        .iter()
        .map(|sample| sample.rss_bytes)
        .min()
        .unwrap_or(0);
    let high = raw_samples
        .iter()
        .map(|sample| sample.rss_bytes)
        .max()
        .unwrap_or(0);
    let warmup = raw_samples.first().map_or(0, |sample| sample.rss_bytes);
    let growth_envelope = SOAK_RSS_MIN_GROWTH_ENVELOPE.max(warmup / 20);
    let first = mean_rss(&raw_samples[..39])?;
    let last = mean_rss(&raw_samples[raw_samples.len() - 39..])?;
    Ok(ProcessMemorySoakReceipt {
        schema_version: SOAK_SCHEMA_VERSION,
        rss_budget_bytes: warmup.saturating_add(SOAK_RSS_BUDGET_HEADROOM),
        rss_high_water_bytes: high,
        growth_envelope_bytes: growth_envelope,
        post_warmup_growth_bytes: high.saturating_sub(low).max(last.saturating_sub(first)),
        first_quartile_mean_bytes: first,
        last_quartile_mean_bytes: last,
        samples_digest: digest_json(&raw_samples)?,
        raw_samples,
    })
}

fn build_activity_receipt(
    policy_digest: [u64; 4],
    learning_commits: u64,
    samples: Vec<DispatchAccountingSample>,
) -> Result<ActivitySoakReceipt, GpuEvidenceError> {
    let mut total = BrainWorkCounters::default();
    let mut total_cost = 0_u64;
    let mut total_debit = 0_u64;
    let mut full = 0_u64;
    let mut reduced = 0_u64;
    let mut essential = 0_u64;
    for sample in &samples {
        total.microsteps = total
            .microsteps
            .saturating_add(sample.work.counters.microsteps);
        total.neuron_updates = total
            .neuron_updates
            .saturating_add(sample.work.counters.neuron_updates);
        total.tile_visits = total
            .tile_visits
            .saturating_add(sample.work.counters.tile_visits);
        total.synapse_ops = total
            .synapse_ops
            .saturating_add(sample.work.counters.synapse_ops);
        total.decoder_candidate_ops = total
            .decoder_candidate_ops
            .saturating_add(sample.work.counters.decoder_candidate_ops);
        total.memory_context_ops = total
            .memory_context_ops
            .saturating_add(sample.work.counters.memory_context_ops);
        total_cost = total_cost.saturating_add(sample.work.neural_cost_q24);
        total_debit = total_debit.saturating_add(u64::from(sample.work.atp_debit_q16));
        match sample.throttle.level {
            NeuralThrottleLevel::Full => full = full.saturating_add(1),
            NeuralThrottleLevel::Reduced => reduced = reduced.saturating_add(1),
            NeuralThrottleLevel::EssentialOnly => essential = essential.saturating_add(1),
        }
    }
    Ok(ActivitySoakReceipt {
        schema_version: SOAK_SCHEMA_VERSION,
        activity_policy_version: samples
            .first()
            .map_or(1, |sample| sample.throttle.policy_version),
        activity_policy_digest: policy_digest,
        total_work: total,
        total_neural_cost_q24: total_cost,
        total_atp_debit_q16: total_debit,
        full_dispatches: full,
        reduced_dispatches: reduced,
        essential_only_dispatches: essential,
        learning_commits,
        sequence_digest: digest_json(&samples)?,
        raw_dispatch_samples: samples,
    })
}

fn allocation_sample(tick: u64, receipt: &GpuAdmissionReceipt) -> GpuAllocationSample {
    GpuAllocationSample {
        tick,
        logical_committed_bytes: receipt.logical_committed_bytes,
        physical_allocated_bytes: receipt.physical_allocated_bytes,
        physical_unused_retained_bytes: receipt.physical_unused_retained_bytes,
        physical_shared_bytes: receipt.physical_shared_bytes,
        physical_alignment_slack_bytes: receipt.physical_alignment_slack_bytes,
        peak_logical_committed_bytes: receipt.peak_logical_committed_bytes,
        peak_physical_allocated_bytes: receipt.peak_physical_allocated_bytes,
        allocation_generation: receipt.allocation_generation,
    }
}

fn collect_allocation_event(
    receipt: &GpuAdmissionReceipt,
    events: &mut Vec<GpuAllocationEventReceipt>,
    digests: &mut BTreeSet<[u64; 4]>,
) {
    if let Some(event) = receipt.last_event {
        if digests.insert(event.event_digest) {
            events.push(event);
        }
    }
}

fn topology_capacity(
    config: &alife_core::TopologicalMapConfig,
) -> Result<TopologyCapacityReceipt, GpuEvidenceError> {
    Ok(TopologyCapacityReceipt {
        max_concepts: u32::try_from(config.max_concepts)
            .map_err(|_| GpuEvidenceError::Contract("topology concept cap exceeds u32"))?,
        max_edges: u32::try_from(config.max_edges)
            .map_err(|_| GpuEvidenceError::Contract("topology edge cap exceeds u32"))?,
        max_simplexes: u32::try_from(config.max_simplexes)
            .map_err(|_| GpuEvidenceError::Contract("topology simplex cap exceeds u32"))?,
        max_unresolved_gaps: u32::try_from(config.max_unresolved_gaps)
            .map_err(|_| GpuEvidenceError::Contract("topology gap cap exceeds u32"))?,
        max_bindings_per_kind: 32,
    })
}

fn max_topology_bindings(sidecar: &alife_core::TopologySidecar) -> u32 {
    sidecar
        .map()
        .concepts()
        .iter()
        .flat_map(|concept| {
            let bindings = &concept.bindings;
            [
                bindings.objects.len(),
                bindings.words.len(),
                bindings.drives.len(),
                bindings.actions.len(),
                bindings.action_families.len(),
                bindings.locations.len(),
                bindings.agents.len(),
                bindings.semantic_refs.len(),
                bindings.cluster_refs.len(),
            ]
        })
        .max()
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0)
}

fn validate_admission(value: &AdmissionSoakReceipt) -> Result<(), GpuEvidenceError> {
    validate_sample_ticks(value.raw_samples.iter().map(|sample| sample.tick))?;
    for event in &value.raw_events {
        event.validate_contract()?;
    }
    if value.schema_version != SOAK_SCHEMA_VERSION
        || value.raw_samples.len() != GPU_SLICE_D_SAMPLE_COUNT
        || value.samples_digest != digest_json(&value.raw_samples)?
        || value.peak_logical_committed_bytes > value.logical_budget_bytes
        || value.peak_physical_allocated_bytes > value.physical_ceiling_bytes
        || value.post_warmup_logical_min_bytes != value.post_warmup_logical_max_bytes
        || value.post_warmup_physical_min_bytes != value.post_warmup_physical_max_bytes
    {
        return Err(GpuEvidenceError::Contract(
            "Slice D admission bound is inconsistent",
        ));
    }
    Ok(())
}

fn validate_process_memory(value: &ProcessMemorySoakReceipt) -> Result<(), GpuEvidenceError> {
    validate_sample_ticks(value.raw_samples.iter().map(|sample| sample.tick))?;
    let expected_samples_digest = digest_json(&value.raw_samples)?;
    let expected_first_quartile_mean = mean_rss(&value.raw_samples[..39])?;
    let expected_last_quartile_mean = mean_rss(&value.raw_samples[value.raw_samples.len() - 39..])?;
    if value.schema_version != SOAK_SCHEMA_VERSION
        || value.raw_samples.len() != GPU_SLICE_D_SAMPLE_COUNT
        || value.samples_digest != expected_samples_digest
        || value.rss_high_water_bytes > value.rss_budget_bytes
        || value.post_warmup_growth_bytes > value.growth_envelope_bytes
        || value.first_quartile_mean_bytes != expected_first_quartile_mean
        || value.last_quartile_mean_bytes != expected_last_quartile_mean
    {
        return Err(GpuEvidenceError::ContractDetail(format!(
            "Slice D process-memory bound is inconsistent: schema={}/{SOAK_SCHEMA_VERSION}, samples={}/{GPU_SLICE_D_SAMPLE_COUNT}, digest_matches={}, high_water={} budget={}, growth={} envelope={}, first_mean={} expected_first_mean={}, last_mean={} expected_last_mean={}",
            value.schema_version,
            value.raw_samples.len(),
            value.samples_digest == expected_samples_digest,
            value.rss_high_water_bytes,
            value.rss_budget_bytes,
            value.post_warmup_growth_bytes,
            value.growth_envelope_bytes,
            value.first_quartile_mean_bytes,
            expected_first_quartile_mean,
            value.last_quartile_mean_bytes,
            expected_last_quartile_mean,
        )));
    }
    Ok(())
}

fn validate_activity(value: &ActivitySoakReceipt, dispatches: u64) -> Result<(), GpuEvidenceError> {
    if value.schema_version != SOAK_SCHEMA_VERSION
        || value.activity_policy_digest == [0; 4]
        || value.raw_dispatch_samples.len() as u64 != dispatches
        || value.raw_dispatch_samples.is_empty()
        || value
            .raw_dispatch_samples
            .iter()
            .any(|sample| !sample.bindings_match())
        || value.sequence_digest != digest_json(&value.raw_dispatch_samples)?
        || value.full_dispatches + value.reduced_dispatches + value.essential_only_dispatches
            != dispatches
        || value.learning_commits == 0
    {
        return Err(GpuEvidenceError::Contract(
            "Slice D activity accounting is inconsistent",
        ));
    }
    let mut previous = None;
    for sample in &value.raw_dispatch_samples {
        if previous.is_some_and(|cursor| sample.work.sequence_cursor <= cursor) {
            return Err(GpuEvidenceError::Contract(
                "Slice D activity sequence is not strictly increasing",
            ));
        }
        previous = Some(sample.work.sequence_cursor);
    }
    Ok(())
}

fn validate_memory(value: &MemorySoakReceipt) -> Result<(), GpuEvidenceError> {
    if value.schema_version != SOAK_SCHEMA_VERSION
        || value.capacity == 0
        || value.final_record_count > value.capacity
        || value.merges + value.evictions == 0
        || value.compactions == 0
        || value.receipts_digest
            != digest_json(&(
                &value.raw_updates,
                &value.raw_compactions,
                value.capacity,
                value.final_record_count,
                value.merges,
                value.evictions,
                value.compactions,
            ))?
    {
        return Err(GpuEvidenceError::Contract(
            "Slice D memory saturation is inconsistent",
        ));
    }
    Ok(())
}

fn validate_topology(value: &TopologySoakReceipt) -> Result<(), GpuEvidenceError> {
    if value.schema_version != SOAK_SCHEMA_VERSION
        || !value
            .capacity
            .contains(value.final_counts, value.max_observed_bindings_per_kind)
        || value.degradations == 0
        || value.raw_observations.is_empty()
        || value.receipts_digest
            != digest_json(&(
                &value.raw_observations,
                &value.capacity,
                value.final_counts,
                value.max_observed_bindings_per_kind,
                value.degradations,
            ))?
    {
        return Err(GpuEvidenceError::Contract(
            "Slice D topology saturation is inconsistent",
        ));
    }
    Ok(())
}

fn validate_truncation(
    value: &TruncationSoakReceipt,
    capacity: &BrainCapacityClass,
) -> Result<(), GpuEvidenceError> {
    if value.schema_version != SOAK_SCHEMA_VERSION
        || value.max_candidates > capacity.execution().max_candidates()
        || value.max_object_slots > capacity.execution().max_object_slots()
        || value.max_memory_context_records > capacity.execution().max_memory_context_records()
        || value.max_decoder_input_lanes > capacity.execution().max_decoder_input_lanes()
        || value.compact_readback_bytes == 0
        || value.compact_readback_bytes > 64
        || [
            value.candidate_truncations,
            value.object_slot_truncations,
            value.memory_context_truncations,
            value.topology_binding_truncations,
        ]
        .contains(&0)
        || value.raw_events.len() != 4
        || value.events_digest != digest_json(&value.raw_events)?
    {
        return Err(GpuEvidenceError::ContractDetail(format!(
            "Slice D truncation evidence is inconsistent: candidates={} objects={} memory={} topology={} events={} readback={} digest_match={}",
            value.candidate_truncations,
            value.object_slot_truncations,
            value.memory_context_truncations,
            value.topology_binding_truncations,
            value.raw_events.len(),
            value.compact_readback_bytes,
            value.events_digest == digest_json(&value.raw_events)?,
        )));
    }
    let mut kinds = BTreeSet::new();
    for event in &value.raw_events {
        if !(1..=4).contains(&event.kind_raw)
            || event.requested != event.retained.saturating_add(event.dropped)
            || event.dropped == 0
            || event.input_digest == [0; 4]
            || event.output_digest == [0; 4]
            || !kinds.insert(event.kind_raw)
        {
            return Err(GpuEvidenceError::Contract(
                "Slice D truncation event is malformed",
            ));
        }
    }
    Ok(())
}

fn validate_save_restore(value: &SaveRestoreSoakReceipt) -> Result<(), GpuEvidenceError> {
    if value.schema_version != SOAK_SCHEMA_VERSION
        || value.sleep_cycles == 0
        || value.save_count != 1
        || value.restore_count != 1
        || value.restore_receipts.len() != 1
        || value.migration_receipts.len() != 8
        || value.restore_receipts.iter().any(|receipt| {
            !receipt.passed
                || receipt.sleep_phase_raw != SleepPhase::Consolidating.raw()
                || receipt.receipt_digest != save_restore_digest(receipt).unwrap_or([0; 4])
        })
        || value.migration_receipts.iter().any(|receipt| {
            !receipt.passed
                || receipt.receipt_digest != save_migration_digest(receipt).unwrap_or([0; 4])
                || match receipt.classification_raw {
                    1 => {
                        receipt.phenotype_compile_count != 1
                            || receipt.gpu_admission_count != 1
                            || receipt.phenotype_hash_or_zero == [0; 4]
                    }
                    2 => {
                        receipt.phenotype_compile_count != 0
                            || receipt.gpu_admission_count != 0
                            || receipt.phenotype_hash_or_zero != [0; 4]
                    }
                    _ => true,
                }
        })
        || value.receipts_digest
            != digest_json(&(
                &value.restore_receipts,
                &value.migration_receipts,
                value.sleep_cycles,
                value.save_count,
                value.restore_count,
            ))?
    {
        return Err(GpuEvidenceError::Contract(
            "Slice D save/restore or migration evidence is inconsistent",
        ));
    }
    Ok(())
}

fn validate_policy_switch(value: &PolicySwitchSoakReceipt) -> Result<(), GpuEvidenceError> {
    if value.schema_version != SOAK_SCHEMA_VERSION
        || value.initial_policy_raw != 1
        || value.final_policy_raw != 1
        || value.switch_count != 0
        || !value.raw_events.is_empty()
        || value.events_digest != digest_json(&value.raw_events)?
    {
        return Err(GpuEvidenceError::Contract(
            "Slice D policy-switch evidence is inconsistent",
        ));
    }
    Ok(())
}

fn validate_replay(
    value: &SameAdapterReplayReceipt,
    adapter: &GpuBackendProvenanceSave,
) -> Result<(), GpuEvidenceError> {
    if value.schema_version != SOAK_SCHEMA_VERSION
        || value.vendor_id != adapter.vendor_id
        || value.device_id != adapter.device_id
        || value.backend_api_raw != adapter.backend_api_raw
        || value.driver_digest != adapter.driver_digest
        || value.feature_digest != adapter.available_features_digest
        || value.limits_digest != adapter.adapter_limits_digest
        || value.checkpoint_digest == [0; 4]
        || value.pressure_sequence_digest == [0; 4]
        || value.source_selection_digest != value.replay_selection_digest
        || value.source_work_digest != value.replay_work_digest
        || value.compared_dispatches == 0
        || value.raw_comparisons.len() as u64 != value.compared_dispatches
        || value.selection_mismatches != 0
        || value.logit_tolerance_f32_bits != GPU_SLICE_D_REPLAY_TOLERANCE_BITS
        || f32::from_bits(value.max_abs_logit_delta_f32_bits)
            > f32::from_bits(value.logit_tolerance_f32_bits)
        || value.logit_tolerance_violations != 0
        || value.raw_comparisons.iter().any(|sample| !sample.passed)
        || value.comparisons_digest != digest_json(&value.raw_comparisons)?
        || !value.passed
    {
        return Err(GpuEvidenceError::Contract(
            "Slice D same-adapter replay is inconsistent",
        ));
    }
    Ok(())
}

fn validate_sample_ticks(ticks: impl Iterator<Item = u64>) -> Result<(), GpuEvidenceError> {
    let expected = (GPU_SLICE_D_WARMUP_TICKS..=GPU_SLICE_D_TICKS)
        .step_by(GPU_SLICE_D_SAMPLE_INTERVAL as usize);
    if !ticks.eq(expected) {
        return Err(GpuEvidenceError::Contract(
            "Slice D allocation/RSS sample ticks are not exact",
        ));
    }
    Ok(())
}

fn is_sample_tick(tick: u64) -> bool {
    tick >= GPU_SLICE_D_WARMUP_TICKS
        && (tick - GPU_SLICE_D_WARMUP_TICKS).is_multiple_of(GPU_SLICE_D_SAMPLE_INTERVAL)
}

fn mean_rss(samples: &[ProcessRssSample]) -> Result<u64, GpuEvidenceError> {
    let sum = samples.iter().try_fold(0_u128, |sum, sample| {
        sum.checked_add(u128::from(sample.rss_bytes))
    });
    let mean = sum
        .and_then(|sum| sum.checked_div(samples.len() as u128))
        .and_then(|value| u64::try_from(value).ok())
        .ok_or(GpuEvidenceError::Contract("RSS mean overflow"))?;
    Ok(mean)
}

fn save_migration_digest(value: &SaveMigrationReceipt) -> Result<[u64; 4], GpuEvidenceError> {
    digest_json(&(
        value.source_schema,
        value.target_schema,
        value.legacy_class_id_raw,
        value.classification_raw,
        value.phenotype_compile_count,
        value.gpu_admission_count,
        value.phenotype_hash_or_zero,
        value.passed,
    ))
}

fn save_restore_digest(value: &SaveRestoreReceipt) -> Result<[u64; 4], GpuEvidenceError> {
    digest_json(&(
        value.save_tick,
        value.restore_tick,
        value.sleep_phase_raw,
        value.consolidation_state_raw,
        value.expected_remaining_swaps,
        value.observed_remaining_swaps,
        value.pre_save_state_digest,
        value.post_restore_state_digest,
        value.passed,
    ))
}

fn digest_json<T: Serialize>(value: &T) -> Result<[u64; 4], GpuEvidenceError> {
    let bytes = serde_json::to_vec(value)?;
    let mut digest = CanonicalDigestBuilder::new(SOAK_JSON_DIGEST_DOMAIN);
    digest.write_bytes(&bytes);
    Ok(digest.finish256())
}

fn policy_raw(policy: PolicyBackend) -> u8 {
    match policy {
        PolicyBackend::NeuralClosedLoopGpu => 1,
        PolicyBackend::HeuristicBaseline => 2,
    }
}

fn distance(left: Vec3f, right: Vec3f) -> f32 {
    let x = left.x - right.x;
    let y = left.y - right.y;
    let z = left.z - right.z;
    (x * x + y * y + z * z).sqrt()
}

fn write_soak_receipt(
    path: &Path,
    receipt: &GpuClosedLoopSoakReceipt,
) -> Result<(), GpuEvidenceError> {
    let parent = path.parent().ok_or(GpuEvidenceError::Contract(
        "Slice D output has no parent directory",
    ))?;
    fs::create_dir_all(parent)?;
    let filename =
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or(GpuEvidenceError::Contract(
                "Slice D output filename is not UTF-8",
            ))?;
    let temporary = parent.join(format!(".{filename}.{}.tmp", std::process::id()));
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > SOAK_ARTIFACT_MAX_BYTES {
        return Err(GpuEvidenceError::Contract(
            "serialized Slice D evidence exceeds 128 MiB",
        ));
    }
    let result = (|| -> Result<(), GpuEvidenceError> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(windows)]
fn process_rss_bytes() -> Result<u64, GpuEvidenceError> {
    use windows_sys::Win32::System::{
        ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::GetCurrentProcess,
    };

    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: u32::try_from(std::mem::size_of::<PROCESS_MEMORY_COUNTERS>())
            .map_err(|_| GpuEvidenceError::Contract("process counters size exceeds u32"))?,
        PageFaultCount: 0,
        PeakWorkingSetSize: 0,
        WorkingSetSize: 0,
        QuotaPeakPagedPoolUsage: 0,
        QuotaPagedPoolUsage: 0,
        QuotaPeakNonPagedPoolUsage: 0,
        QuotaNonPagedPoolUsage: 0,
        PagefileUsage: 0,
        PeakPagefileUsage: 0,
    };
    // SAFETY: GetCurrentProcess returns a process pseudo-handle valid in this
    // process, and the writable counters pointer/size match the Windows ABI.
    let succeeded =
        unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    if succeeded == 0 {
        return Err(GpuEvidenceError::Contract(
            "Windows process RSS query failed",
        ));
    }
    u64::try_from(counters.WorkingSetSize)
        .map_err(|_| GpuEvidenceError::Contract("process RSS exceeds u64"))
}

#[cfg(not(windows))]
fn process_rss_bytes() -> Result<u64, GpuEvidenceError> {
    Err(GpuEvidenceError::Contract(
        "Slice D process RSS evidence currently requires Windows",
    ))
}
