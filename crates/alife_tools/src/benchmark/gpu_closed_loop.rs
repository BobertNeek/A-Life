//! Tooling-only contracts, canonical validation, and orchestration data for real GPU benchmarks.

use std::{collections::BTreeSet, fs, path::Path};

use alife_core::{
    BrainActivityPolicyV1, BrainCapacityClass, BrainClassId, CanonicalDigestBuilder,
    PhenotypeEvidenceManifest, PhenotypeHash, SchemaKind, SchemaVersions, SensorProfile,
};
pub use alife_core::{
    GpuClosedLoopBenchmarkProtocolV1, GPU_CLOSED_LOOP_BENCHMARK_BASE_SEED,
    GPU_CLOSED_LOOP_BENCHMARK_MEASURED_TICKS, GPU_CLOSED_LOOP_BENCHMARK_PROTOCOL_VERSION,
    GPU_CLOSED_LOOP_BENCHMARK_SCHEMA, GPU_CLOSED_LOOP_BENCHMARK_TIMESTAMP_SCOPE_SPLIT,
    GPU_CLOSED_LOOP_BENCHMARK_WARMUP_TICKS,
};
use alife_game_app::{
    compile_gpu_closed_loop_benchmark_phenotype, run_gpu_closed_loop_benchmark_trial,
    GpuClosedLoopBenchmarkTrialError, GpuClosedLoopBenchmarkTrialOptions,
};
use alife_gpu_backend::GpuAdmissionReceipt;
use alife_world::GpuBackendProvenanceSave;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const GPU_CLOSED_LOOP_BENCHMARK_P95_INDEX: usize = 972;
pub const GPU_BENCHMARK_UNAVAILABLE_NO_ADAPTER: u16 = 1;
pub const GPU_BENCHMARK_UNAVAILABLE_REQUIRED_CAPABILITY: u16 = 2;
pub const GPU_BENCHMARK_UNAVAILABLE_ADMISSION: u16 = 3;

const ENVIRONMENT_DOMAIN: &[u8] = b"alife.gpu.closed-loop-benchmark.environment.v1";
const ADAPTER_IDENTITY_DOMAIN: &[u8] = b"alife.gpu.closed-loop-benchmark.adapter.v1";
const ROW_DOMAIN: &[u8] = b"alife.gpu.closed-loop-benchmark.row.v1";
const MANIFEST_DOMAIN: &[u8] = b"alife.gpu.closed-loop-benchmark.manifest.v1";
const TARGETS_DOMAIN: &[u8] = b"alife.gpu.closed-loop-benchmark.targets.v1";

#[derive(Debug, Error)]
pub enum GpuBenchmarkError {
    #[error("GPU benchmark contract failed: {0}")]
    Contract(&'static str),
    #[error("GPU benchmark contract failed: {0}")]
    ContractDetail(String),
    #[error(transparent)]
    Core(#[from] alife_core::ScaffoldContractError),
    #[error(transparent)]
    Persistence(#[from] alife_world::PersistenceError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuBenchmarkStatus {
    Completed,
    Missed,
    Unavailable { reason_code: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuBenchmarkEnvironmentReceipt {
    pub schema_version: u16,
    pub availability_reason_code: u16,
    pub adapter: Option<GpuBackendProvenanceSave>,
    pub adapter_identity_digest_or_zero: [u64; 4],
    pub environment_digest: [u64; 4],
}

impl GpuBenchmarkEnvironmentReceipt {
    pub fn new(
        availability_reason_code: u16,
        adapter: Option<GpuBackendProvenanceSave>,
    ) -> Result<Self, GpuBenchmarkError> {
        let adapter_identity_digest_or_zero = adapter
            .as_ref()
            .map(adapter_identity_digest)
            .transpose()?
            .unwrap_or([0; 4]);
        let mut receipt = Self {
            schema_version: GPU_CLOSED_LOOP_BENCHMARK_SCHEMA,
            availability_reason_code,
            adapter,
            adapter_identity_digest_or_zero,
            environment_digest: [0; 4],
        };
        receipt.environment_digest = receipt.recompute_digest()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn recompute_digest(&self) -> Result<[u64; 4], GpuBenchmarkError> {
        canonical_struct_digest(ENVIRONMENT_DOMAIN, self, "environment_digest")
    }

    pub fn validate(&self) -> Result<(), GpuBenchmarkError> {
        let expected_identity = self
            .adapter
            .as_ref()
            .map(adapter_identity_digest)
            .transpose()?
            .unwrap_or([0; 4]);
        if let Some(adapter) = &self.adapter {
            adapter.validate()?;
        }
        if self.schema_version != GPU_CLOSED_LOOP_BENCHMARK_SCHEMA
            || self.adapter_identity_digest_or_zero != expected_identity
            || self.environment_digest != self.recompute_digest()?
            || (self.adapter.is_none()
                && !matches!(
                    self.availability_reason_code,
                    GPU_BENCHMARK_UNAVAILABLE_NO_ADAPTER
                        | GPU_BENCHMARK_UNAVAILABLE_REQUIRED_CAPABILITY
                ))
            || (self.adapter.is_some()
                && !matches!(
                    self.availability_reason_code,
                    0 | GPU_BENCHMARK_UNAVAILABLE_REQUIRED_CAPABILITY
                        | GPU_BENCHMARK_UNAVAILABLE_ADMISSION
                ))
        {
            return Err(GpuBenchmarkError::Contract(
                "benchmark environment receipt is inconsistent",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuClosedLoopBenchmarkRow {
    pub schema_version: u16,
    pub class_id_raw: u16,
    pub sensor_profile_id_raw: u16,
    pub sensor_profile_schema: u16,
    pub sensory_abi_raw: u16,
    pub population: u32,
    pub fixture_seed: u64,
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest: PhenotypeEvidenceManifest,
    pub phenotype_manifest_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
    pub runtime_profile_digest: [u64; 4],
    pub activity_policy_digest: [u64; 4],
    pub protocol_digest: [u64; 4],
    pub target_p95_ns: u64,
    pub measured_p95_ns: Option<u64>,
    pub timestamp_period_ns_q24: u64,
    pub raw_inference_timestamp_ticks: Vec<u64>,
    pub raw_plasticity_timestamp_ticks: Vec<u64>,
    pub raw_neural_tick_ns: Vec<u64>,
    pub environment: GpuBenchmarkEnvironmentReceipt,
    pub admission: Option<GpuAdmissionReceipt>,
    pub gpu_selections: u64,
    pub executed_actions: u64,
    pub sealed_patches: u64,
    pub learning_commits: u64,
    pub distinct_selected_families: u16,
    pub active_synapses: u32,
    pub status: GpuBenchmarkStatus,
    pub row_digest: [u64; 4],
}

impl GpuClosedLoopBenchmarkRow {
    pub fn recompute_row_digest(&self) -> Result<[u64; 4], GpuBenchmarkError> {
        canonical_struct_digest(ROW_DOMAIN, self, "row_digest")
    }

    pub fn seal_digest(&mut self) -> Result<(), GpuBenchmarkError> {
        self.row_digest = self.recompute_row_digest()?;
        Ok(())
    }

    pub fn validate(
        &self,
        protocol: &GpuClosedLoopBenchmarkProtocolV1,
        target: &GpuPerformanceTargetRowV1,
    ) -> Result<(), GpuBenchmarkError> {
        let capacity = BrainCapacityClass::production_for_id(BrainClassId(self.class_id_raw))?;
        let profile = SensorProfile::try_from_raw(self.sensor_profile_id_raw)?;
        let expected_samples = usize::try_from(protocol.measured_ticks)
            .map_err(|_| GpuBenchmarkError::Contract("sample count does not fit usize"))?;
        let expected_events = u64::from(protocol.measured_ticks)
            .checked_mul(u64::from(self.population))
            .ok_or(GpuBenchmarkError::Contract(
                "benchmark event count overflow",
            ))?;
        self.phenotype_manifest.validate_for_capacity(&capacity)?;
        self.environment.validate()?;
        if self.schema_version != GPU_CLOSED_LOOP_BENCHMARK_SCHEMA
            || self.population == 0
            || self.sensor_profile_schema
                != SchemaVersions::current_for(SchemaKind::SensorProfile).raw()
            || self.sensory_abi_raw != SchemaVersions::CURRENT.sensory_abi.raw()
            || self.phenotype_manifest.phenotype_sensor_profile_raw != profile.raw()
            || self.phenotype_hash != self.phenotype_manifest.phenotype_hash
            || self.phenotype_manifest_digest != self.phenotype_manifest.manifest_digest
            || self.capacity_digest != capacity.canonical_digest()
            || self.phenotype_manifest.capacity_digest != self.capacity_digest
            || self.runtime_profile_digest == [0; 4]
            || self.activity_policy_digest != BrainActivityPolicyV1::production_v1().policy_digest
            || self.protocol_digest != protocol.protocol_digest
            || self.fixture_seed
                != protocol.row_seed(
                    self.class_id_raw,
                    self.sensor_profile_id_raw,
                    self.population,
                )
            || target.key() != self.key()
            || target.target_p95_ns != self.target_p95_ns
            || self.target_p95_ns == 0
            || self.active_synapses == 0
            || self.row_digest != self.recompute_row_digest()?
        {
            return Err(GpuBenchmarkError::Contract(
                "benchmark row identity is inconsistent",
            ));
        }

        if let Some(admission) = &self.admission {
            admission.validate_contract()?;
            admission.runtime.validate_for(capacity.execution())?;
            let adapter = self
                .environment
                .adapter
                .as_ref()
                .ok_or(GpuBenchmarkError::Contract(
                    "benchmark admission has no observed adapter",
                ))?;
            if admission.runtime.profile_digest != self.runtime_profile_digest
                || admission.runtime.adapter_limits_digest != adapter.adapter_limits_digest
                || admission.runtime.required_features_digest()? != adapter.required_features_digest
                || admission
                    .runtime
                    .required_limits_digest_for(capacity.execution())?
                    != adapter.required_limits_digest
            {
                return Err(GpuBenchmarkError::Contract(
                    "benchmark admission disagrees with adapter or runtime profile",
                ));
            }
        }

        match self.status {
            GpuBenchmarkStatus::Completed | GpuBenchmarkStatus::Missed => {
                let admission = self.admission.as_ref().ok_or(GpuBenchmarkError::Contract(
                    "executed benchmark row has no admission receipt",
                ))?;
                if self.environment.availability_reason_code != 0
                    || self.environment.adapter.is_none()
                    || self.timestamp_period_ns_q24 == 0
                    || self.raw_inference_timestamp_ticks.len() != expected_samples
                    || self.raw_plasticity_timestamp_ticks.len() != expected_samples
                    || self.raw_neural_tick_ns.len() != expected_samples
                    || self.gpu_selections != expected_events
                    || self.executed_actions != expected_events
                    || self.sealed_patches != expected_events
                    || self.learning_commits == 0
                    || self.learning_commits > self.sealed_patches
                    || self.distinct_selected_families < 2
                    || admission.live_brains != self.population
                {
                    return Err(GpuBenchmarkError::Contract(
                        "executed benchmark row did not prove causal work",
                    ));
                }
                let mut converted = Vec::with_capacity(expected_samples);
                for (&inference, &plasticity) in self
                    .raw_inference_timestamp_ticks
                    .iter()
                    .zip(&self.raw_plasticity_timestamp_ticks)
                {
                    converted.push(
                        timestamp_ticks_to_ns(inference, self.timestamp_period_ns_q24)?
                            .checked_add(timestamp_ticks_to_ns(
                                plasticity,
                                self.timestamp_period_ns_q24,
                            )?)
                            .ok_or(GpuBenchmarkError::Contract("timing sum overflow"))?,
                    );
                }
                if converted != self.raw_neural_tick_ns {
                    return Err(GpuBenchmarkError::Contract(
                        "raw timestamp samples do not reproduce neural tick samples",
                    ));
                }
                let measured = nearest_rank_p95(&mut converted)?;
                if self.measured_p95_ns != Some(measured)
                    || (self.status == GpuBenchmarkStatus::Completed)
                        != (measured <= self.target_p95_ns)
                    || (self.status == GpuBenchmarkStatus::Missed)
                        != (measured > self.target_p95_ns)
                {
                    return Err(GpuBenchmarkError::Contract(
                        "benchmark status does not derive from measured p95",
                    ));
                }
            }
            GpuBenchmarkStatus::Unavailable { reason_code } => {
                if !matches!(
                    reason_code,
                    GPU_BENCHMARK_UNAVAILABLE_NO_ADAPTER
                        | GPU_BENCHMARK_UNAVAILABLE_REQUIRED_CAPABILITY
                        | GPU_BENCHMARK_UNAVAILABLE_ADMISSION
                ) || self.environment.availability_reason_code != reason_code
                    || self.measured_p95_ns.is_some()
                    || self.timestamp_period_ns_q24 != 0
                    || !self.raw_inference_timestamp_ticks.is_empty()
                    || !self.raw_plasticity_timestamp_ticks.is_empty()
                    || !self.raw_neural_tick_ns.is_empty()
                    || self.gpu_selections != 0
                    || self.executed_actions != 0
                    || self.sealed_patches != 0
                    || self.learning_commits != 0
                    || self.distinct_selected_families != 0
                    || (reason_code == GPU_BENCHMARK_UNAVAILABLE_NO_ADAPTER
                        && (self.environment.adapter.is_some() || self.admission.is_some()))
                    || (reason_code == GPU_BENCHMARK_UNAVAILABLE_REQUIRED_CAPABILITY
                        && self.admission.is_some())
                    || (reason_code == GPU_BENCHMARK_UNAVAILABLE_ADMISSION
                        && self.environment.adapter.is_none())
                {
                    return Err(GpuBenchmarkError::Contract(
                        "unavailable benchmark row forges executed evidence",
                    ));
                }
                if let Some(admission) = &self.admission {
                    if admission.live_brains >= self.population {
                        return Err(GpuBenchmarkError::Contract(
                            "admission-unavailable row contains a successful admission",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub const fn key(&self) -> (u16, u16, u32) {
        (
            self.class_id_raw,
            self.sensor_profile_id_raw,
            self.population,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuClosedLoopBenchmarkManifest {
    pub schema_version: u16,
    pub git_commit: String,
    pub source_tree_digest: String,
    pub adapter: Option<GpuBackendProvenanceSave>,
    pub adapter_identity_digest_or_zero: [u64; 4],
    pub protocol: GpuClosedLoopBenchmarkProtocolV1,
    pub rows: Vec<GpuClosedLoopBenchmarkRow>,
    pub manifest_digest: [u64; 4],
}

impl GpuClosedLoopBenchmarkManifest {
    pub fn recompute_manifest_digest(&self) -> Result<[u64; 4], GpuBenchmarkError> {
        canonical_struct_digest(MANIFEST_DOMAIN, self, "manifest_digest")
    }

    pub fn seal_digest(&mut self) -> Result<(), GpuBenchmarkError> {
        self.manifest_digest = self.recompute_manifest_digest()?;
        Ok(())
    }

    pub fn validate(&self, targets: &GpuPerformanceTargetsV1) -> Result<(), GpuBenchmarkError> {
        targets.validate()?;
        if !self.protocol.is_canonical() {
            return Err(GpuBenchmarkError::Contract(
                "benchmark protocol is not canonical",
            ));
        }
        if self.schema_version != GPU_CLOSED_LOOP_BENCHMARK_SCHEMA
            || !is_lower_hex_oid(&self.git_commit)
            || !is_lower_hex_oid(&self.source_tree_digest)
            || self.rows.len() != targets.rows.len()
            || self.rows.is_empty()
            || self.manifest_digest != self.recompute_manifest_digest()?
        {
            return Err(GpuBenchmarkError::Contract(
                "benchmark manifest header is inconsistent",
            ));
        }
        let manifest_identity = self
            .adapter
            .as_ref()
            .map(adapter_identity_digest)
            .transpose()?
            .unwrap_or([0; 4]);
        if self.adapter_identity_digest_or_zero != manifest_identity {
            return Err(GpuBenchmarkError::Contract(
                "benchmark manifest adapter digest is inconsistent",
            ));
        }
        if let Some(adapter) = &self.adapter {
            adapter.validate()?;
        }

        let mut keys = BTreeSet::new();
        let mut observed_identity = None;
        for (row, target) in self.rows.iter().zip(&targets.rows) {
            if !keys.insert(row.key()) || row.key() != target.key() {
                return Err(GpuBenchmarkError::Contract(
                    "benchmark rows are missing, duplicated, or unsorted",
                ));
            }
            row.validate(&self.protocol, target)?;
            if let Some(adapter) = &row.environment.adapter {
                let identity = adapter_identity_digest(adapter)?;
                match observed_identity {
                    None => observed_identity = Some(identity),
                    Some(expected) if expected == identity => {}
                    Some(_) => {
                        return Err(GpuBenchmarkError::Contract(
                            "benchmark child rows used different adapters",
                        ));
                    }
                }
            }
        }
        if observed_identity.unwrap_or([0; 4]) != manifest_identity {
            return Err(GpuBenchmarkError::Contract(
                "benchmark manifest adapter disagrees with child rows",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GpuPerformanceTargetRowV1 {
    pub class_id_raw: u16,
    pub sensor_profile_id_raw: u16,
    pub population: u32,
    pub target_p95_ns: u64,
}

impl GpuPerformanceTargetRowV1 {
    pub const fn key(self) -> (u16, u16, u32) {
        (
            self.class_id_raw,
            self.sensor_profile_id_raw,
            self.population,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuPerformanceTargetsV1 {
    pub schema_version: u16,
    pub protocol_digest: [u64; 4],
    pub rows: Vec<GpuPerformanceTargetRowV1>,
    pub targets_digest: [u64; 4],
}

impl GpuPerformanceTargetsV1 {
    pub fn recompute_digest(&self) -> Result<[u64; 4], GpuBenchmarkError> {
        canonical_struct_digest(TARGETS_DOMAIN, self, "targets_digest")
    }

    pub fn seal_digest(&mut self) -> Result<(), GpuBenchmarkError> {
        self.targets_digest = self.recompute_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), GpuBenchmarkError> {
        let canonical = canonical_performance_targets_v1();
        if self.schema_version != GPU_CLOSED_LOOP_BENCHMARK_SCHEMA
            || self.protocol_digest != GpuClosedLoopBenchmarkProtocolV1::canonical().protocol_digest
            || self.rows != canonical.rows
            || self.targets_digest != self.recompute_digest()?
        {
            return Err(GpuBenchmarkError::Contract(
                "performance targets are not the exact v1 matrix",
            ));
        }
        Ok(())
    }
}

pub fn canonical_performance_targets_v1() -> GpuPerformanceTargetsV1 {
    let populations = [1_u32, 10, 50, 100, 250, 500];
    let class_targets_ms = [
        (BrainCapacityClass::N512_ID.raw(), [2_u64, 4, 8, 12, 25, 50]),
        (
            BrainCapacityClass::N1024_ID.raw(),
            [3_u64, 6, 12, 20, 40, 80],
        ),
        (
            BrainCapacityClass::N2048_ID.raw(),
            [4_u64, 8, 20, 35, 70, 140],
        ),
    ];
    let mut rows = Vec::with_capacity(36);
    for (class_id_raw, targets_ms) in class_targets_ms {
        for profile in [
            SensorProfile::PrivilegedAffordanceV1,
            SensorProfile::GroundedObjectSlotsV1,
        ] {
            for (population, target_ms) in populations.into_iter().zip(targets_ms) {
                rows.push(GpuPerformanceTargetRowV1 {
                    class_id_raw,
                    sensor_profile_id_raw: profile.raw(),
                    population,
                    target_p95_ns: target_ms * 1_000_000,
                });
            }
        }
    }
    rows.sort_unstable();
    let mut targets = GpuPerformanceTargetsV1 {
        schema_version: GPU_CLOSED_LOOP_BENCHMARK_SCHEMA,
        protocol_digest: GpuClosedLoopBenchmarkProtocolV1::canonical().protocol_digest,
        rows,
        targets_digest: [0; 4],
    };
    targets.targets_digest = targets
        .recompute_digest()
        .expect("the static benchmark target table is canonical JSON");
    targets
}

pub fn load_performance_targets(
    path: impl AsRef<Path>,
) -> Result<GpuPerformanceTargetsV1, GpuBenchmarkError> {
    let targets: GpuPerformanceTargetsV1 = serde_json::from_slice(&fs::read(path)?)?;
    targets.validate()?;
    Ok(targets)
}

pub fn load_benchmark_manifest(
    path: impl AsRef<Path>,
    targets: &GpuPerformanceTargetsV1,
) -> Result<GpuClosedLoopBenchmarkManifest, GpuBenchmarkError> {
    let manifest: GpuClosedLoopBenchmarkManifest = serde_json::from_slice(&fs::read(path)?)?;
    manifest.validate(targets)?;
    Ok(manifest)
}

pub fn run_single_benchmark_row(
    target: GpuPerformanceTargetRowV1,
) -> Result<GpuClosedLoopBenchmarkRow, GpuBenchmarkError> {
    let protocol = GpuClosedLoopBenchmarkProtocolV1::canonical();
    let capacity = BrainCapacityClass::production_for_id(BrainClassId(target.class_id_raw))?;
    let sensor_profile = SensorProfile::try_from_raw(target.sensor_profile_id_raw)?;
    let options = GpuClosedLoopBenchmarkTrialOptions {
        capacity,
        sensor_profile,
        population: target.population,
        fixture_seed: protocol.row_seed(
            target.class_id_raw,
            target.sensor_profile_id_raw,
            target.population,
        ),
        warmup_ticks: protocol.warmup_ticks,
        measured_ticks: protocol.measured_ticks,
    };
    let phenotype = compile_gpu_closed_loop_benchmark_phenotype(options)
        .map_err(|error| GpuBenchmarkError::ContractDetail(error.to_string()))?;
    let runtime_profile_digest =
        alife_gpu_backend::GpuRuntimeProfile::production_v1().canonical_digest()?;
    let activity_policy_digest = alife_core::BrainActivityPolicyV1::production_v1().policy_digest;
    let base = |environment: GpuBenchmarkEnvironmentReceipt,
                admission: Option<GpuAdmissionReceipt>,
                status: GpuBenchmarkStatus| GpuClosedLoopBenchmarkRow {
        schema_version: GPU_CLOSED_LOOP_BENCHMARK_SCHEMA,
        class_id_raw: target.class_id_raw,
        sensor_profile_id_raw: target.sensor_profile_id_raw,
        sensor_profile_schema: SchemaVersions::current_for(SchemaKind::SensorProfile).raw(),
        sensory_abi_raw: SchemaVersions::CURRENT.sensory_abi.raw(),
        population: target.population,
        fixture_seed: options.fixture_seed,
        phenotype_hash: phenotype.manifest.phenotype_hash,
        phenotype_manifest: phenotype.manifest.clone(),
        phenotype_manifest_digest: phenotype.manifest.manifest_digest,
        capacity_digest: capacity.canonical_digest(),
        runtime_profile_digest,
        activity_policy_digest,
        protocol_digest: protocol.protocol_digest,
        target_p95_ns: target.target_p95_ns,
        measured_p95_ns: None,
        timestamp_period_ns_q24: 0,
        raw_inference_timestamp_ticks: Vec::new(),
        raw_plasticity_timestamp_ticks: Vec::new(),
        raw_neural_tick_ns: Vec::new(),
        environment,
        admission,
        gpu_selections: 0,
        executed_actions: 0,
        sealed_patches: 0,
        learning_commits: 0,
        distinct_selected_families: 0,
        active_synapses: phenotype.active_synapses,
        status,
        row_digest: [0; 4],
    };

    let mut row = match run_gpu_closed_loop_benchmark_trial(options) {
        Ok(trial) => {
            if trial.phenotype_manifest != phenotype.manifest
                || trial.runtime_profile_digest != runtime_profile_digest
                || trial.activity_policy_digest != activity_policy_digest
            {
                return Err(GpuBenchmarkError::Contract(
                    "runtime benchmark trial disagrees with compiled fixture",
                ));
            }
            let environment =
                GpuBenchmarkEnvironmentReceipt::new(0, Some(trial.backend_provenance.clone()))?;
            let mut raw_neural_tick_ns = Vec::with_capacity(protocol.measured_ticks as usize);
            for (&inference, &plasticity) in trial
                .raw_inference_timestamp_ticks
                .iter()
                .zip(&trial.raw_plasticity_timestamp_ticks)
            {
                raw_neural_tick_ns.push(
                    timestamp_ticks_to_ns(inference, trial.timestamp_period_ns_q24)?
                        .checked_add(timestamp_ticks_to_ns(
                            plasticity,
                            trial.timestamp_period_ns_q24,
                        )?)
                        .ok_or(GpuBenchmarkError::Contract("timing sum overflow"))?,
                );
            }
            let mut sorted = raw_neural_tick_ns.clone();
            let measured_p95_ns = nearest_rank_p95(&mut sorted)?;
            let status = if measured_p95_ns <= target.target_p95_ns {
                GpuBenchmarkStatus::Completed
            } else {
                GpuBenchmarkStatus::Missed
            };
            let mut row = base(environment, Some(trial.admission), status);
            row.measured_p95_ns = Some(measured_p95_ns);
            row.timestamp_period_ns_q24 = trial.timestamp_period_ns_q24;
            row.raw_inference_timestamp_ticks = trial.raw_inference_timestamp_ticks;
            row.raw_plasticity_timestamp_ticks = trial.raw_plasticity_timestamp_ticks;
            row.raw_neural_tick_ns = raw_neural_tick_ns;
            row.gpu_selections = trial.gpu_selections;
            row.executed_actions = trial.executed_actions;
            row.sealed_patches = trial.sealed_patches;
            row.learning_commits = trial.learning_commits;
            row.distinct_selected_families = trial.distinct_selected_families;
            row.active_synapses = trial.active_synapses;
            row
        }
        Err(GpuClosedLoopBenchmarkTrialError::Unavailable(unavailable)) => {
            let environment =
                GpuBenchmarkEnvironmentReceipt::new(unavailable.reason_code, unavailable.adapter)?;
            base(
                environment,
                unavailable.admission,
                GpuBenchmarkStatus::Unavailable {
                    reason_code: unavailable.reason_code,
                },
            )
        }
        Err(GpuClosedLoopBenchmarkTrialError::Fatal(error)) => {
            return Err(GpuBenchmarkError::ContractDetail(error.to_string()));
        }
        Err(
            error @ (GpuClosedLoopBenchmarkTrialError::Tick { .. }
            | GpuClosedLoopBenchmarkTrialError::CausalInvariant(_)),
        ) => {
            return Err(GpuBenchmarkError::ContractDetail(error.to_string()));
        }
    };
    row.seal_digest()?;
    row.validate(&protocol, &target)?;
    Ok(row)
}

pub fn nearest_rank_p95(samples: &mut [u64]) -> Result<u64, GpuBenchmarkError> {
    if samples.len() != GPU_CLOSED_LOOP_BENCHMARK_MEASURED_TICKS as usize {
        return Err(GpuBenchmarkError::Contract(
            "p95 requires exactly 1024 measured samples",
        ));
    }
    samples.sort_unstable();
    Ok(samples[GPU_CLOSED_LOOP_BENCHMARK_P95_INDEX])
}

pub fn timestamp_ticks_to_ns(
    delta_ticks: u64,
    timestamp_period_ns_q24: u64,
) -> Result<u64, GpuBenchmarkError> {
    if delta_ticks == 0 || timestamp_period_ns_q24 == 0 {
        return Err(GpuBenchmarkError::Contract("timestamp sample is zero"));
    }
    let rounded = u128::from(delta_ticks)
        .checked_mul(u128::from(timestamp_period_ns_q24))
        .and_then(|value| value.checked_add(1_u128 << 23))
        .ok_or(GpuBenchmarkError::Contract("timestamp conversion overflow"))?
        >> 24;
    u64::try_from(rounded)
        .map_err(|_| GpuBenchmarkError::Contract("timestamp conversion exceeds u64"))
}

pub fn adapter_identity_digest(
    adapter: &GpuBackendProvenanceSave,
) -> Result<[u64; 4], GpuBenchmarkError> {
    adapter.validate()?;
    let mut digest = CanonicalDigestBuilder::new(ADAPTER_IDENTITY_DOMAIN);
    digest.write_u16(adapter.schema_version);
    digest.write_u16(adapter.backend_api_raw);
    digest.write_u32(adapter.vendor_id);
    digest.write_u32(adapter.device_id);
    digest.write_u16(adapter.backend_version_major);
    digest.write_u16(adapter.backend_version_minor);
    digest.write_u16(adapter.backend_version_patch);
    for words in [
        adapter.driver_digest,
        adapter.available_features_digest,
        adapter.adapter_limits_digest,
    ] {
        for word in words {
            digest.write_u64(word);
        }
    }
    Ok(digest.finish256())
}

pub fn is_lower_hex_oid(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_struct_digest<T: Serialize>(
    domain: &[u8],
    value: &T,
    excluded_field: &str,
) -> Result<[u64; 4], GpuBenchmarkError> {
    let mut value = serde_json::to_value(value)?;
    let object = value.as_object_mut().ok_or(GpuBenchmarkError::Contract(
        "canonical digest requires a struct",
    ))?;
    object
        .remove(excluded_field)
        .ok_or(GpuBenchmarkError::Contract(
            "canonical digest exclusion field is missing",
        ))?;
    let mut digest = CanonicalDigestBuilder::new(domain);
    encode_json_value(&mut digest, &value)?;
    Ok(digest.finish256())
}

fn encode_json_value(
    digest: &mut CanonicalDigestBuilder,
    value: &Value,
) -> Result<(), GpuBenchmarkError> {
    match value {
        Value::Null => digest.write_u8(0),
        Value::Bool(value) => {
            digest.write_u8(1);
            digest.write_bool(*value);
        }
        Value::Number(value) => {
            digest.write_u8(2);
            if let Some(value) = value.as_u64() {
                digest.write_u8(0);
                digest.write_u64(value);
            } else if let Some(value) = value.as_i64() {
                digest.write_u8(1);
                digest.write_u64(value as u64);
            } else {
                return Err(GpuBenchmarkError::Contract(
                    "canonical benchmark evidence cannot contain JSON floats",
                ));
            }
        }
        Value::String(value) => {
            digest.write_u8(3);
            digest.write_utf8(value);
        }
        Value::Array(values) => {
            digest.write_u8(4);
            digest.write_sequence_len(values.len());
            for value in values {
                encode_json_value(digest, value)?;
            }
        }
        Value::Object(values) => {
            digest.write_u8(5);
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            digest.write_sequence_len(entries.len());
            for (key, value) in entries {
                digest.write_utf8(key);
                encode_json_value(digest, value)?;
            }
        }
    }
    Ok(())
}
