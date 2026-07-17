//! P34 persistence, runtime config, and asset manifest contracts.
//!
//! These portable records intentionally store stable IDs, summaries, and asset
//! references. Engine-local handles and bulk tensors stay outside save files.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use alife_core::{
    require_version, BrainScaleTier, GenomeId, HomeostaticSnapshot, OrganismId,
    PackedExperienceFrame, PolicyBackend, ScaffoldContractError, SchemaKind, SchemaVersions,
    TeacherPerceptionChannel, Tick, Validate, Vec3f, WorldEntityId,
};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::{
    appearance::CreatureAppearanceGenome,
    ecology::EcologyState,
    headless::{HeadlessWorld, HeadlessWorldPersistenceParts, WorldObject, WorldObjectKind},
    legacy_neural_policy_v1::LegacyBackendConfigV1,
    persistent_voxel::{
        migrated_voxel_backend_for_world, PersistentVoxelProfileId, PersistentVoxelWorldSaveState,
    },
};

pub const P34_SAVE_FILE_SCHEMA: &str = "alife.p34.save_file.v1";
pub const P34_SAVE_FILE_SCHEMA_VERSION: u16 = SchemaVersions::CURRENT.save.0;
pub const P34_RUNTIME_CONFIG_SCHEMA: &str = "alife.p34.runtime_config.v1";
pub const P34_RUNTIME_CONFIG_SCHEMA_VERSION: u16 = 1;
pub const P34_ASSET_MANIFEST_SCHEMA: &str = "alife.p34.asset_manifest.v1";
pub const P34_ASSET_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const P34_MIGRATION_HOOK_SCHEMA_VERSION: u16 = 1;
pub const BRAIN_POLICY_CONFIG_SCHEMA_VERSION: u16 = 1;
pub const P34_MAX_INLINE_SAVE_BYTES: u64 = 64 * 1024;
pub const FVR06_GPU_RUNTIME_STATE_SCHEMA: &str = "alife.fvr06.gpu_runtime_state.v1";
pub const FVR06_GPU_RUNTIME_STATE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("schema mismatch: expected '{expected}', got '{actual}'")]
    Schema {
        expected: &'static str,
        actual: String,
    },
    #[error("schema version mismatch for {schema}: expected {expected}, got {actual}")]
    SchemaVersion {
        schema: &'static str,
        expected: u16,
        actual: u16,
    },
    #[error("core contract violation: {0}")]
    Contract(#[from] ScaffoldContractError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid config field {field}: {message}")]
    InvalidConfig {
        field: &'static str,
        message: &'static str,
    },
    #[error("invalid asset manifest entry {asset_id}: {message}")]
    InvalidAssetManifest {
        asset_id: String,
        message: &'static str,
    },
    #[error("missing required asset {asset_id} at {path:?}")]
    MissingRequiredAsset { asset_id: String, path: PathBuf },
    #[error("digest mismatch for {asset_id}: expected {expected}, got {actual}")]
    DigestMismatch {
        asset_id: String,
        expected: String,
        actual: String,
    },
    #[error("engine-local id leaked through {field}: {value}")]
    EngineLocalIdLeak { field: &'static str, value: String },
    #[error("asset reference {asset_id} is not present in the manifest")]
    MissingAssetReference { asset_id: String },
    #[error("genetic fixed layer cannot be mutable in default portable saves")]
    GeneticLayerMutable,
    #[error("migration from {from_schema_version} to {to_schema_version} is not implemented")]
    MigrationUnsupported {
        from_schema_version: u16,
        to_schema_version: u16,
    },
    #[error("inline save payload is too large: {bytes} bytes")]
    HugeInlinePayload { bytes: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortableAssetDigest(pub String);

impl PortableAssetDigest {
    pub fn for_bytes(bytes: &[u8]) -> Self {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        Self(format!("fnv1a64:{hash:016x}"))
    }

    pub fn for_file(path: &Path) -> Result<Self, PersistenceError> {
        let bytes = fs::read(path)?;
        if is_portable_text_asset(path) {
            Ok(Self::for_bytes(&canonicalize_text_line_endings(&bytes)))
        } else {
            Ok(Self::for_bytes(&bytes))
        }
    }

    pub fn validate_format(&self) -> Result<(), PersistenceError> {
        let Some(hex) = self.0.strip_prefix("fnv1a64:") else {
            return Err(PersistenceError::InvalidAssetManifest {
                asset_id: "<digest>".to_string(),
                message: "digest must use fnv1a64:<16-hex> format",
            });
        };
        if hex.len() == 16 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            Ok(())
        } else {
            Err(PersistenceError::InvalidAssetManifest {
                asset_id: "<digest>".to_string(),
                message: "digest must use fnv1a64:<16-hex> format",
            })
        }
    }
}

fn is_portable_text_asset(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("json" | "toml" | "ron" | "txt" | "md")
    )
}

fn canonicalize_text_line_endings(bytes: &[u8]) -> Vec<u8> {
    let mut canonical = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            canonical.push(b'\n');
            index += 2;
        } else {
            canonical.push(bytes[index]);
            index += 1;
        }
    }
    canonical
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    GeneratedWeights,
    EtfPrototypes,
    ScenarioConfig,
    ExampleWorld,
    SemanticGaussian,
    PackedLog,
    BenchmarkReport,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetPresence {
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetManifestEntry {
    pub asset_id: String,
    pub kind: AssetKind,
    pub relative_path: String,
    pub digest: PortableAssetDigest,
    pub presence: AssetPresence,
    pub schema_version: u16,
    pub size_bytes: Option<u64>,
    pub provenance: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetManifest {
    pub schema: String,
    pub schema_version: u16,
    pub entries: Vec<AssetManifestEntry>,
}

impl AssetManifest {
    pub fn empty() -> Self {
        Self {
            schema: P34_ASSET_MANIFEST_SCHEMA.to_string(),
            schema_version: P34_ASSET_MANIFEST_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }

    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let text = fs::read_to_string(path)?;
        peek_schema(
            &text,
            P34_ASSET_MANIFEST_SCHEMA,
            P34_ASSET_MANIFEST_SCHEMA_VERSION,
        )?;
        let manifest: Self = serde_json::from_str(&text)?;
        Ok(manifest)
    }

    pub fn validate_with_root(&self, root: impl AsRef<Path>) -> Result<(), PersistenceError> {
        require_named_schema(
            &self.schema,
            P34_ASSET_MANIFEST_SCHEMA,
            self.schema_version,
            P34_ASSET_MANIFEST_SCHEMA_VERSION,
        )?;
        let mut ids = BTreeSet::new();
        for entry in &self.entries {
            entry.validate(root.as_ref())?;
            if !ids.insert(entry.asset_id.clone()) {
                return Err(PersistenceError::InvalidAssetManifest {
                    asset_id: entry.asset_id.clone(),
                    message: "duplicate asset id",
                });
            }
        }
        Ok(())
    }

    pub fn contains_asset(&self, asset_id: &str) -> bool {
        self.entries.iter().any(|entry| entry.asset_id == asset_id)
    }
}

impl AssetManifestEntry {
    fn validate(&self, root: &Path) -> Result<(), PersistenceError> {
        if self.asset_id.is_empty() || self.schema_version == 0 {
            return Err(PersistenceError::InvalidAssetManifest {
                asset_id: self.asset_id.clone(),
                message: "asset id and schema version are required",
            });
        }
        self.digest.validate_format()?;
        let relative = Path::new(&self.relative_path);
        validate_relative_path(&self.asset_id, relative)?;
        let path = root.join(relative);
        if !path.exists() {
            return match self.presence {
                AssetPresence::Required => Err(PersistenceError::MissingRequiredAsset {
                    asset_id: self.asset_id.clone(),
                    path,
                }),
                AssetPresence::Optional => Ok(()),
            };
        }
        if let Some(expected_size) = self.size_bytes {
            let actual_size = fs::metadata(&path)?.len();
            if actual_size != expected_size {
                return Err(PersistenceError::InvalidAssetManifest {
                    asset_id: self.asset_id.clone(),
                    message: "asset size metadata does not match file",
                });
            }
        }
        let actual = PortableAssetDigest::for_file(&path)?;
        if actual != self.digest {
            return Err(PersistenceError::DigestMismatch {
                asset_id: self.asset_id.clone(),
                expected: self.digest.0.clone(),
                actual: actual.0,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainPolicyConfig {
    pub schema_version: u16,
    pub policy: PolicyBackend,
}

impl BrainPolicyConfig {
    fn validate(self) -> Result<(), PersistenceError> {
        if self.schema_version != BRAIN_POLICY_CONFIG_SCHEMA_VERSION {
            return Err(PersistenceError::SchemaVersion {
                schema: "alife.brain_policy_config.v1",
                expected: BRAIN_POLICY_CONFIG_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureFlagConfig {
    pub school_enabled: bool,
    pub semantic_adapter_enabled: bool,
    pub gpu_backend_enabled: bool,
    pub offline_tools_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchoolConfig {
    pub teacher_enabled: bool,
    pub curriculum_id: Option<String>,
    pub save_teacher_private_state: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticAdapterConfig {
    pub provider: Option<String>,
    pub required: bool,
    pub fake_provider_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuLimitsConfig {
    pub max_storage_buffers: u32,
    pub neural_budget_ms: f32,
    pub no_active_gameplay_readback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeAdapterIdentity {
    pub adapter_name: Option<String>,
    pub backend_api: Option<String>,
    pub adapter_type: Option<String>,
    pub driver: Option<String>,
    pub driver_info: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeResidencySlots {
    pub hot_slots: u16,
    pub warm_slots: u16,
    pub cold_slots: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeClassBucketAllocation {
    pub brain_class: BrainScaleTier,
    pub hot_slots: u16,
    pub warm_slots: u16,
    pub cold_slots: u16,
    pub max_creatures: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuRuntimeActiveProfileCaps {
    pub target_fps: u16,
    pub target_frame_ms: f32,
    pub renderer_reserve_ms: f32,
    pub gpu_neural_budget_ms: f32,
    pub neural_heap_mb: u32,
    pub staging_readback_budget_kib: u32,
    pub chunk_activation_radius: u16,
    pub active_chunk_cap: u16,
    pub vfx_budget: String,
    pub adaptive_throttling_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeShaderAbiVersions {
    pub shader_manifest: Vec<String>,
    pub abi_manifest: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeAuthorityState {
    pub authoritative: bool,
    pub failure_stops_learned_actions: bool,
    pub finite_rejections: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeSafeCheckpoint {
    pub save_id: String,
    pub world_tick: Tick,
    pub sealed_patch_boundary: bool,
    pub checkpoint_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuRuntimeSaveState {
    pub schema: String,
    pub schema_version: u16,
    pub requested_backend_mode: String,
    pub selected_backend_mode: String,
    pub adapter_identity: GpuRuntimeAdapterIdentity,
    pub validation_profile: String,
    pub brain_residency_slots: GpuRuntimeResidencySlots,
    pub class_bucket_allocations: Vec<GpuRuntimeClassBucketAllocation>,
    pub active_profile_caps: GpuRuntimeActiveProfileCaps,
    pub shader_abi_versions: GpuRuntimeShaderAbiVersions,
    pub authority: GpuRuntimeAuthorityState,
    pub last_safe_checkpoint: GpuRuntimeSafeCheckpoint,
    pub unavailable_reason: Option<String>,
    pub selected_scale_profile: String,
    pub compact_action_readback_bytes_per_creature: u32,
    pub no_active_bulk_readback: bool,
}

impl GpuRuntimeSaveState {
    pub fn validate(&self) -> Result<(), PersistenceError> {
        require_named_schema(
            &self.schema,
            FVR06_GPU_RUNTIME_STATE_SCHEMA,
            self.schema_version,
            FVR06_GPU_RUNTIME_STATE_SCHEMA_VERSION,
        )?;
        let backend_state_valid = self.requested_backend_mode == "GpuAuthoritative"
            && if self.authority.authoritative {
                self.selected_backend_mode == "GpuAuthoritative"
                    && self.unavailable_reason.is_none()
            } else {
                self.selected_backend_mode == "Unavailable"
                    && self
                        .unavailable_reason
                        .as_deref()
                        .is_some_and(|reason| !reason.trim().is_empty())
            };
        if self.requested_backend_mode.trim().is_empty()
            || self.selected_backend_mode.trim().is_empty()
            || self.validation_profile.trim().is_empty()
            || self.selected_scale_profile.trim().is_empty()
            || self.class_bucket_allocations.is_empty()
            || self.shader_abi_versions.shader_manifest.is_empty()
            || self.shader_abi_versions.abi_manifest.is_empty()
            || self.last_safe_checkpoint.save_id.trim().is_empty()
            || self.last_safe_checkpoint.checkpoint_label.trim().is_empty()
            || !self.last_safe_checkpoint.sealed_patch_boundary
            || !backend_state_valid
            || !self.authority.failure_stops_learned_actions
            || !self.no_active_bulk_readback
        {
            return Err(PersistenceError::InvalidConfig {
                field: "gpu_runtime",
                message: "FVR06 GPU runtime descriptor is incomplete",
            });
        }
        if self.authority.authoritative
            && (self
                .adapter_identity
                .adapter_name
                .as_deref()
                .is_none_or(str::is_empty)
                || self
                    .adapter_identity
                    .backend_api
                    .as_deref()
                    .is_none_or(str::is_empty))
        {
            return Err(PersistenceError::InvalidConfig {
                field: "gpu_runtime.adapter_identity",
                message: "selected GPU backend requires adapter name and API",
            });
        }
        self.brain_residency_slots.validate()?;
        self.active_profile_caps.validate()?;
        for allocation in &self.class_bucket_allocations {
            allocation.validate()?;
        }
        if self.compact_action_readback_bytes_per_creature == 0
            || self.compact_action_readback_bytes_per_creature
                > self
                    .active_profile_caps
                    .staging_readback_budget_kib
                    .saturating_mul(1024)
        {
            return Err(PersistenceError::InvalidConfig {
                field: "gpu_runtime.compact_action_readback_bytes_per_creature",
                message: "compact action readback budget must be bounded and nonzero",
            });
        }
        let json = serde_json::to_string(self)?;
        if contains_engine_local_runtime_token(&json) {
            return Err(PersistenceError::EngineLocalIdLeak {
                field: "gpu_runtime",
                value: "engine-local token".to_string(),
            });
        }
        Ok(())
    }
}

impl GpuRuntimeResidencySlots {
    fn validate(&self) -> Result<(), PersistenceError> {
        if self.hot_slots == 0
            || self.warm_slots == 0
            || self
                .hot_slots
                .saturating_add(self.warm_slots)
                .saturating_add(self.cold_slots)
                == 0
        {
            return Err(PersistenceError::InvalidConfig {
                field: "gpu_runtime.brain_residency_slots",
                message: "runtime residency slots must include hot and warm brains",
            });
        }
        Ok(())
    }
}

impl GpuRuntimeClassBucketAllocation {
    fn validate(&self) -> Result<(), PersistenceError> {
        if self.brain_class.neuron_count().is_none()
            || self.max_creatures == 0
            || self.hot_slots.saturating_add(self.warm_slots) == 0
        {
            return Err(PersistenceError::InvalidConfig {
                field: "gpu_runtime.class_bucket_allocations",
                message: "class buckets require a canonical brain class and active slots",
            });
        }
        Ok(())
    }
}

impl GpuRuntimeActiveProfileCaps {
    fn validate(&self) -> Result<(), PersistenceError> {
        for value in [
            self.target_frame_ms,
            self.renderer_reserve_ms,
            self.gpu_neural_budget_ms,
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(PersistenceError::InvalidConfig {
                    field: "gpu_runtime.active_profile_caps",
                    message: "profile timing caps must be finite and positive",
                });
            }
        }
        if self.target_fps == 0
            || self.neural_heap_mb == 0
            || self.staging_readback_budget_kib == 0
            || self.chunk_activation_radius == 0
            || self.active_chunk_cap == 0
            || self.vfx_budget.trim().is_empty()
            || self.adaptive_throttling_order.is_empty()
        {
            return Err(PersistenceError::InvalidConfig {
                field: "gpu_runtime.active_profile_caps",
                message:
                    "profile caps must record frame, heap, chunk, staging, and throttle budgets",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub enabled: bool,
    pub packed_log_schema_version: u16,
    pub max_side_buffer_bytes: u64,
    pub relative_log_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuntimeConfig {
    pub schema: String,
    pub schema_version: u16,
    pub deterministic_seed: u64,
    pub brain_class: BrainScaleTier,
    pub benchmark_population_tier: u16,
    pub brain_policy: BrainPolicyConfig,
    pub features: FeatureFlagConfig,
    pub school: SchoolConfig,
    pub semantic: SemanticAdapterConfig,
    pub gpu_limits: GpuLimitsConfig,
    pub logging: LoggingConfig,
    pub asset_root: String,
    pub save_root: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeConfigWire {
    schema: String,
    schema_version: u16,
    deterministic_seed: u64,
    brain_class: BrainScaleTier,
    benchmark_population_tier: u16,
    #[serde(default)]
    brain_policy: Option<BrainPolicyConfig>,
    #[serde(default)]
    backend: Option<LegacyBackendConfigV1>,
    features: FeatureFlagConfig,
    school: SchoolConfig,
    semantic: SemanticAdapterConfig,
    gpu_limits: GpuLimitsConfig,
    logging: LoggingConfig,
    asset_root: String,
    save_root: String,
}

impl<'de> Deserialize<'de> for RuntimeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RuntimeConfigWire::deserialize(deserializer)?;
        let brain_policy = match (wire.brain_policy, wire.backend) {
            (Some(policy), None) => policy,
            (None, Some(legacy)) => BrainPolicyConfig {
                schema_version: BRAIN_POLICY_CONFIG_SCHEMA_VERSION,
                policy: legacy.migrate_policy(),
            },
            (Some(_), Some(_)) => {
                return Err(D::Error::custom(
                    "runtime config cannot contain both brain_policy and legacy backend",
                ));
            }
            (None, None) => {
                return Err(D::Error::custom(
                    "runtime config requires brain_policy or legacy backend",
                ));
            }
        };
        Ok(Self {
            schema: wire.schema,
            schema_version: wire.schema_version,
            deterministic_seed: wire.deterministic_seed,
            brain_class: wire.brain_class,
            benchmark_population_tier: wire.benchmark_population_tier,
            brain_policy,
            features: wire.features,
            school: wire.school,
            semantic: wire.semantic,
            gpu_limits: wire.gpu_limits,
            logging: wire.logging,
            asset_root: wire.asset_root,
            save_root: wire.save_root,
        })
    }
}

impl RuntimeConfig {
    pub fn deterministic_default(deterministic_seed: u64, brain_class: BrainScaleTier) -> Self {
        Self {
            schema: P34_RUNTIME_CONFIG_SCHEMA.to_string(),
            schema_version: P34_RUNTIME_CONFIG_SCHEMA_VERSION,
            deterministic_seed,
            brain_class,
            benchmark_population_tier: 1,
            brain_policy: BrainPolicyConfig {
                schema_version: BRAIN_POLICY_CONFIG_SCHEMA_VERSION,
                policy: PolicyBackend::NeuralClosedLoopGpu,
            },
            features: FeatureFlagConfig {
                school_enabled: false,
                semantic_adapter_enabled: false,
                gpu_backend_enabled: false,
                offline_tools_required: false,
            },
            school: SchoolConfig {
                teacher_enabled: false,
                curriculum_id: None,
                save_teacher_private_state: false,
            },
            semantic: SemanticAdapterConfig {
                provider: None,
                required: false,
                fake_provider_allowed: true,
            },
            gpu_limits: GpuLimitsConfig {
                max_storage_buffers: 16,
                neural_budget_ms: 4.0,
                no_active_gameplay_readback: true,
            },
            logging: LoggingConfig {
                enabled: true,
                packed_log_schema_version: PackedExperienceFrame::SCHEMA_VERSION,
                max_side_buffer_bytes: 4 * 1024 * 1024,
                relative_log_path: None,
            },
            asset_root: "assets".to_string(),
            save_root: "saves".to_string(),
        }
    }

    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let text = fs::read_to_string(path)?;
        peek_schema(
            &text,
            P34_RUNTIME_CONFIG_SCHEMA,
            P34_RUNTIME_CONFIG_SCHEMA_VERSION,
        )?;
        let config: Self = serde_json::from_str(&text)?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), PersistenceError> {
        require_named_schema(
            &self.schema,
            P34_RUNTIME_CONFIG_SCHEMA,
            self.schema_version,
            P34_RUNTIME_CONFIG_SCHEMA_VERSION,
        )?;
        if self.deterministic_seed == 0 {
            return Err(PersistenceError::InvalidConfig {
                field: "deterministic_seed",
                message: "seed must be nonzero for reproducible saves",
            });
        }
        if self.brain_class.neuron_count().is_none() {
            return Err(PersistenceError::InvalidConfig {
                field: "brain_class",
                message: "portable default config requires a canonical brain class",
            });
        }
        if self.benchmark_population_tier == 0 {
            return Err(PersistenceError::InvalidConfig {
                field: "benchmark_population_tier",
                message: "benchmark tier population must be nonzero",
            });
        }
        self.brain_policy.validate()?;
        if self.features.offline_tools_required {
            return Err(PersistenceError::InvalidConfig {
                field: "features.offline_tools_required",
                message: "offline tools cannot be runtime prerequisites",
            });
        }
        if self.school.teacher_enabled && !self.features.school_enabled {
            return Err(PersistenceError::InvalidConfig {
                field: "school.teacher_enabled",
                message: "teacher requires school feature flag",
            });
        }
        if self.school.save_teacher_private_state {
            return Err(PersistenceError::InvalidConfig {
                field: "school.save_teacher_private_state",
                message: "teacher-private state is not part of portable P34 saves",
            });
        }
        if self.semantic.required && self.semantic.provider.is_none() {
            return Err(PersistenceError::InvalidConfig {
                field: "semantic.provider",
                message: "required semantic provider must be named",
            });
        }
        if self.gpu_limits.max_storage_buffers == 0
            || !self.gpu_limits.neural_budget_ms.is_finite()
            || self.gpu_limits.neural_budget_ms <= 0.0
        {
            return Err(PersistenceError::InvalidConfig {
                field: "gpu_limits",
                message: "GPU limits must be finite and positive",
            });
        }
        if !self.gpu_limits.no_active_gameplay_readback {
            return Err(PersistenceError::InvalidConfig {
                field: "gpu_limits.no_active_gameplay_readback",
                message: "portable configs must preserve no-readback policy",
            });
        }
        require_version(
            SchemaKind::PackedLog,
            PackedExperienceFrame::SCHEMA_VERSION,
            self.logging.packed_log_schema_version,
        )?;
        if self.logging.max_side_buffer_bytes == 0 {
            return Err(PersistenceError::InvalidConfig {
                field: "logging.max_side_buffer_bytes",
                message: "side buffer cap must be nonzero",
            });
        }
        if let Some(path) = &self.logging.relative_log_path {
            validate_relative_path("logging.relative_log_path", Path::new(path))?;
        }
        validate_relative_path("asset_root", Path::new(&self.asset_root))?;
        validate_relative_path("save_root", Path::new(&self.save_root))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureMindSaveSummary {
    pub tick: Tick,
    pub homeostasis: HomeostaticSnapshot,
    pub memory_record_count: u32,
    pub memory_source_ids: Vec<alife_core::MemoryId>,
    pub concept_count: u32,
    pub edge_count: u32,
    pub simplex_count: u32,
    pub unresolved_gap_count: u32,
    pub sleep_state_label: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightLayerSaveSummary {
    pub generated_weight_asset_id: Option<String>,
    pub genetic_fixed_digest: String,
    pub genetic_layer_mutable: bool,
    pub lifetime_consolidated_entries: u32,
    pub h_operational_entries: u32,
    pub h_shadow_entries: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningTraceSaveSummary {
    pub lifetime_learning_enabled: bool,
    pub lamarckian_mode_enabled: bool,
    pub last_consolidated_tick: Option<Tick>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureSaveState {
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub brain_class: BrainScaleTier,
    pub development_tick: Tick,
    #[serde(default)]
    pub appearance: CreatureAppearanceGenome,
    pub mind: CreatureMindSaveSummary,
    pub weights: WeightLayerSaveSummary,
    pub learning: LearningTraceSaveSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchoolSaveState {
    pub schema_version: u16,
    pub enabled: bool,
    pub active_curriculum_id: Option<String>,
    pub teacher_private_state_saved: bool,
}

impl Default for SchoolSaveState {
    fn default() -> Self {
        Self {
            schema_version: SchemaVersions::CURRENT.teacher_school.raw(),
            enabled: false,
            active_curriculum_id: None,
            teacher_private_state_saved: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterRemapEntry {
    pub stable_world_entity_id: WorldEntityId,
    pub adapter_namespace: String,
    pub adapter_slot: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterRemapTable {
    pub entries: Vec<AdapterRemapEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldObjectSaveState {
    pub id: WorldEntityId,
    pub label: String,
    pub kind: WorldObjectKind,
    pub organism_id: Option<OrganismId>,
    pub position: Vec3f,
    pub radius: f32,
    pub nutrition: f32,
    pub hazard_pain: f32,
    pub token_id: Option<u32>,
    pub social_affinity: f32,
    pub teacher_channel: Option<TeacherPerceptionChannel>,
    pub consumed: bool,
    pub carried_by: Option<OrganismId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSaveState {
    pub seed: u64,
    pub tick: Tick,
    pub next_entity_id: u64,
    pub objects: Vec<WorldObjectSaveState>,
    pub last_touched_entities: Vec<WorldEntityId>,
    #[serde(default)]
    pub ecology: EcologyState,
    #[serde(default)]
    pub voxel_backend: Option<PersistentVoxelWorldSaveState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableSaveFile {
    pub schema: String,
    pub schema_version: u16,
    pub save_id: String,
    pub deterministic_seed: u64,
    pub config: RuntimeConfig,
    #[serde(default)]
    pub gpu_runtime: Option<GpuRuntimeSaveState>,
    pub assets: AssetManifest,
    pub world: WorldSaveState,
    pub creatures: Vec<CreatureSaveState>,
    pub school: SchoolSaveState,
    pub adapter_remap: AdapterRemapTable,
    pub generated_weight_asset_refs: Vec<String>,
    pub etf_prototype_asset_refs: Vec<String>,
    pub packed_log_schema_version: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationHook {
    pub schema_version: u16,
    pub from_schema_version: u16,
    pub to_schema_version: u16,
}

impl PortableSaveFile {
    pub fn from_headless_world(
        save_id: impl Into<String>,
        world: &HeadlessWorld,
        config: RuntimeConfig,
        assets: AssetManifest,
        creatures: Vec<CreatureSaveState>,
    ) -> Result<Self, PersistenceError> {
        let parts = world.persistence_parts();
        let generated_weight_asset_refs = creatures
            .iter()
            .filter_map(|creature| creature.weights.generated_weight_asset_id.clone())
            .collect();
        let mut world_state = WorldSaveState::from_parts(parts);
        world_state.voxel_backend = Some(
            migrated_voxel_backend_for_world(
                &world_state,
                PersistentVoxelProfileId::MinimumSettings30x30,
            )
            .map_err(PersistenceError::Contract)?,
        );
        let save = Self {
            schema: P34_SAVE_FILE_SCHEMA.to_string(),
            schema_version: P34_SAVE_FILE_SCHEMA_VERSION,
            save_id: save_id.into(),
            deterministic_seed: world.seed(),
            config,
            gpu_runtime: None,
            assets,
            world: world_state,
            creatures,
            school: SchoolSaveState::default(),
            adapter_remap: AdapterRemapTable::default(),
            generated_weight_asset_refs,
            etf_prototype_asset_refs: Vec::new(),
            packed_log_schema_version: PackedExperienceFrame::SCHEMA_VERSION,
        };
        Ok(save)
    }

    pub fn from_json_str(text: &str) -> Result<Self, PersistenceError> {
        peek_schema(text, P34_SAVE_FILE_SCHEMA, P34_SAVE_FILE_SCHEMA_VERSION)?;
        let save: Self = serde_json::from_str(text)?;
        Ok(save)
    }

    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        Self::from_json_str(&fs::read_to_string(path)?)
    }

    pub fn to_json_string_pretty(&self) -> Result<String, PersistenceError> {
        let json = serde_json::to_string_pretty(self)?;
        if json.len() as u64 > P34_MAX_INLINE_SAVE_BYTES {
            return Err(PersistenceError::HugeInlinePayload {
                bytes: json.len() as u64,
            });
        }
        Ok(json)
    }

    pub fn to_json_file(&self, path: impl AsRef<Path>) -> Result<(), PersistenceError> {
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn validate_with_asset_root(&self, root: impl AsRef<Path>) -> Result<(), PersistenceError> {
        require_named_schema(
            &self.schema,
            P34_SAVE_FILE_SCHEMA,
            self.schema_version,
            P34_SAVE_FILE_SCHEMA_VERSION,
        )?;
        if self.save_id.is_empty() || self.deterministic_seed == 0 {
            return Err(PersistenceError::InvalidConfig {
                field: "save_id/deterministic_seed",
                message: "save id and deterministic seed are required",
            });
        }
        if self.deterministic_seed != self.world.seed
            || self.deterministic_seed != self.config.deterministic_seed
        {
            return Err(PersistenceError::InvalidConfig {
                field: "deterministic_seed",
                message: "save, world, and config seeds must agree",
            });
        }
        self.config.validate()?;
        if let Some(gpu_runtime) = &self.gpu_runtime {
            gpu_runtime.validate()?;
            if gpu_runtime.last_safe_checkpoint.save_id != self.save_id
                || gpu_runtime.last_safe_checkpoint.world_tick != self.world.tick
            {
                return Err(PersistenceError::InvalidConfig {
                    field: "gpu_runtime.last_safe_checkpoint",
                    message: "GPU runtime checkpoint must match save id and world tick",
                });
            }
        }
        self.assets.validate_with_root(root)?;
        self.world.validate()?;
        self.adapter_remap.validate()?;
        self.school.validate()?;
        require_version(
            SchemaKind::PackedLog,
            PackedExperienceFrame::SCHEMA_VERSION,
            self.packed_log_schema_version,
        )?;
        for creature in &self.creatures {
            creature.validate(&self.assets)?;
        }
        for asset_id in self
            .generated_weight_asset_refs
            .iter()
            .chain(self.etf_prototype_asset_refs.iter())
        {
            require_asset_reference(&self.assets, asset_id)?;
        }
        Ok(())
    }

    pub fn restore_headless_world(&self) -> Result<HeadlessWorld, PersistenceError> {
        self.world.restore()
    }

    pub fn require_voxel_backend(
        &self,
    ) -> Result<&PersistentVoxelWorldSaveState, PersistenceError> {
        match &self.world.voxel_backend {
            Some(voxel_backend) => {
                voxel_backend
                    .validate()
                    .map_err(PersistenceError::Contract)?;
                Ok(voxel_backend)
            }
            None => Err(PersistenceError::MigrationUnsupported {
                from_schema_version: self.schema_version,
                to_schema_version: self.schema_version,
            }),
        }
    }

    pub fn with_migrated_voxel_backend(
        &self,
        profile_id: PersistentVoxelProfileId,
    ) -> Result<Self, PersistenceError> {
        let mut migrated = self.clone();
        let regenerate = migrated
            .world
            .voxel_backend
            .as_ref()
            .is_none_or(|backend| backend.profile_id != profile_id);
        if regenerate {
            migrated.world.voxel_backend = Some(
                migrated_voxel_backend_for_world(&migrated.world, profile_id)
                    .map_err(PersistenceError::Contract)?,
            );
        }
        Ok(migrated)
    }

    pub fn with_gpu_runtime_state(
        &self,
        gpu_runtime: GpuRuntimeSaveState,
    ) -> Result<Self, PersistenceError> {
        gpu_runtime.validate()?;
        if gpu_runtime.last_safe_checkpoint.save_id != self.save_id
            || gpu_runtime.last_safe_checkpoint.world_tick != self.world.tick
        {
            return Err(PersistenceError::InvalidConfig {
                field: "gpu_runtime.last_safe_checkpoint",
                message: "GPU runtime checkpoint must match save id and world tick",
            });
        }
        let mut save = self.clone();
        save.gpu_runtime = Some(gpu_runtime);
        Ok(save)
    }
}

impl MigrationHook {
    pub fn reject_premature_migration(&self) -> Result<(), PersistenceError> {
        if self.schema_version != P34_MIGRATION_HOOK_SCHEMA_VERSION {
            return Err(PersistenceError::SchemaVersion {
                schema: "alife.p34.migration_hook.v1",
                expected: P34_MIGRATION_HOOK_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        Err(PersistenceError::MigrationUnsupported {
            from_schema_version: self.from_schema_version,
            to_schema_version: self.to_schema_version,
        })
    }
}

impl CreatureSaveState {
    fn validate(&self, assets: &AssetManifest) -> Result<(), PersistenceError> {
        self.organism_id.validate()?;
        self.genome_id.validate()?;
        if self.brain_class.neuron_count().is_none() {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.brain_class",
                message: "creature save requires canonical brain class",
            });
        }
        self.appearance.validate()?;
        self.mind.validate()?;
        self.weights.validate(assets)?;
        if self.learning.lamarckian_mode_enabled {
            return Err(PersistenceError::InvalidConfig {
                field: "learning.lamarckian_mode_enabled",
                message: "portable P34 saves keep Lamarckian inheritance default-off",
            });
        }
        Ok(())
    }
}

impl CreatureMindSaveSummary {
    fn validate(&self) -> Result<(), PersistenceError> {
        self.homeostasis.validate_contract()?;
        if self.homeostasis.tick != self.tick {
            return Err(PersistenceError::InvalidConfig {
                field: "mind.homeostasis.tick",
                message: "homeostasis tick must match mind tick",
            });
        }
        if self.sleep_state_label.is_empty() {
            return Err(PersistenceError::InvalidConfig {
                field: "mind.sleep_state_label",
                message: "sleep state label is required",
            });
        }
        for id in &self.memory_source_ids {
            id.validate()?;
        }
        Ok(())
    }
}

impl WeightLayerSaveSummary {
    fn validate(&self, assets: &AssetManifest) -> Result<(), PersistenceError> {
        if self.genetic_layer_mutable {
            return Err(PersistenceError::GeneticLayerMutable);
        }
        PortableAssetDigest(self.genetic_fixed_digest.clone()).validate_format()?;
        if let Some(asset_id) = &self.generated_weight_asset_id {
            require_asset_reference(assets, asset_id)?;
        }
        Ok(())
    }
}

impl SchoolSaveState {
    fn validate(&self) -> Result<(), PersistenceError> {
        require_version(
            SchemaKind::TeacherSchool,
            SchemaVersions::CURRENT.teacher_school.raw(),
            self.schema_version,
        )?;
        if self.teacher_private_state_saved {
            return Err(PersistenceError::InvalidConfig {
                field: "school.teacher_private_state_saved",
                message: "teacher-private state must not be in portable saves",
            });
        }
        Ok(())
    }
}

impl AdapterRemapTable {
    pub fn validate(&self) -> Result<(), PersistenceError> {
        let mut stable_ids = BTreeSet::new();
        for entry in &self.entries {
            entry.stable_world_entity_id.validate()?;
            if entry.adapter_namespace.is_empty() || entry.adapter_slot.is_empty() {
                return Err(PersistenceError::InvalidConfig {
                    field: "adapter_remap",
                    message: "namespace and slot are required",
                });
            }
            reject_engine_local_token("adapter_namespace", &entry.adapter_namespace)?;
            reject_engine_local_token("adapter_slot", &entry.adapter_slot)?;
            if !stable_ids.insert(entry.stable_world_entity_id.raw()) {
                return Err(PersistenceError::InvalidConfig {
                    field: "adapter_remap",
                    message: "duplicate stable entity remap",
                });
            }
        }
        Ok(())
    }
}

impl WorldSaveState {
    fn from_parts(parts: HeadlessWorldPersistenceParts) -> Self {
        Self {
            seed: parts.seed,
            tick: parts.tick,
            next_entity_id: parts.next_entity_id,
            objects: parts
                .objects
                .into_iter()
                .map(WorldObjectSaveState::from)
                .collect(),
            last_touched_entities: parts.last_touched_entities,
            ecology: parts.ecology,
            voxel_backend: None,
        }
    }

    fn validate(&self) -> Result<(), PersistenceError> {
        if self.seed == 0 {
            return Err(PersistenceError::InvalidConfig {
                field: "world.seed",
                message: "world seed must be nonzero",
            });
        }
        let mut ids = BTreeSet::new();
        let mut labels = BTreeSet::new();
        let mut max_id = 0_u64;
        for object in &self.objects {
            object.validate()?;
            if !ids.insert(object.id.raw()) || !labels.insert(object.label.clone()) {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
            max_id = max_id.max(object.id.raw());
        }
        if self.next_entity_id <= max_id || (self.objects.is_empty() && self.next_entity_id == 0) {
            return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
        }
        for touched in &self.last_touched_entities {
            touched.validate()?;
            if !ids.contains(&touched.raw()) {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
        }
        self.ecology.validate()?;
        if let Some(voxel_backend) = &self.voxel_backend {
            if voxel_backend.world_seed != self.seed {
                return Err(PersistenceError::InvalidConfig {
                    field: "world.voxel_backend.world_seed",
                    message: "voxel backend seed must match world seed",
                });
            }
            voxel_backend
                .validate()
                .map_err(PersistenceError::Contract)?;
        }
        for resource in &self.ecology.resources {
            if !ids.contains(&resource.object_id.raw()) {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
        }
        Ok(())
    }

    fn restore(&self) -> Result<HeadlessWorld, PersistenceError> {
        self.validate()?;
        let parts = HeadlessWorldPersistenceParts {
            seed: self.seed,
            tick: self.tick,
            next_entity_id: self.next_entity_id,
            objects: self
                .objects
                .iter()
                .cloned()
                .map(WorldObject::from)
                .collect(),
            last_touched_entities: self.last_touched_entities.clone(),
            ecology: self.ecology.clone(),
        };
        Ok(HeadlessWorld::from_persistence_parts(parts)?)
    }
}

impl WorldObjectSaveState {
    fn validate(&self) -> Result<(), PersistenceError> {
        self.id.validate()?;
        if self.label.is_empty() {
            return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
        }
        if let Some(id) = self.organism_id {
            id.validate()?;
        }
        if let Some(id) = self.carried_by {
            id.validate()?;
        }
        self.position.validate()?;
        for value in [
            self.radius,
            self.nutrition,
            self.hazard_pain,
            self.social_affinity,
        ] {
            if !value.is_finite() {
                return Err(PersistenceError::Contract(
                    ScaffoldContractError::NonFiniteFloat,
                ));
            }
        }
        if self.radius <= 0.0
            || !(0.0..=1.0).contains(&self.nutrition)
            || !(0.0..=1.0).contains(&self.hazard_pain)
            || !(-1.0..=1.0).contains(&self.social_affinity)
        {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::ScalarOutOfRange,
            ));
        }
        Ok(())
    }
}

impl From<WorldObject> for WorldObjectSaveState {
    fn from(value: WorldObject) -> Self {
        Self {
            id: value.id,
            label: value.label,
            kind: value.kind,
            organism_id: value.organism_id,
            position: value.position,
            radius: value.radius,
            nutrition: value.nutrition,
            hazard_pain: value.hazard_pain,
            token_id: value.token_id,
            social_affinity: value.social_affinity,
            teacher_channel: value.teacher_channel,
            consumed: value.consumed,
            carried_by: value.carried_by,
        }
    }
}

impl From<WorldObjectSaveState> for WorldObject {
    fn from(value: WorldObjectSaveState) -> Self {
        Self {
            id: value.id,
            label: value.label,
            kind: value.kind,
            organism_id: value.organism_id,
            position: value.position,
            radius: value.radius,
            nutrition: value.nutrition,
            hazard_pain: value.hazard_pain,
            token_id: value.token_id,
            social_affinity: value.social_affinity,
            teacher_channel: value.teacher_channel,
            consumed: value.consumed,
            carried_by: value.carried_by,
        }
    }
}

fn require_asset_reference(assets: &AssetManifest, asset_id: &str) -> Result<(), PersistenceError> {
    if assets.contains_asset(asset_id) {
        Ok(())
    } else {
        Err(PersistenceError::MissingAssetReference {
            asset_id: asset_id.to_string(),
        })
    }
}

fn require_named_schema(
    actual_schema: &str,
    expected_schema: &'static str,
    actual_version: u16,
    expected_version: u16,
) -> Result<(), PersistenceError> {
    if actual_schema != expected_schema {
        return Err(PersistenceError::Schema {
            expected: expected_schema,
            actual: actual_schema.to_string(),
        });
    }
    if actual_version != expected_version {
        return Err(PersistenceError::SchemaVersion {
            schema: expected_schema,
            expected: expected_version,
            actual: actual_version,
        });
    }
    Ok(())
}

fn peek_schema(
    text: &str,
    expected_schema: &'static str,
    expected_version: u16,
) -> Result<(), PersistenceError> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let actual_schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let actual_version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .unwrap_or_default();
    require_named_schema(
        actual_schema,
        expected_schema,
        actual_version,
        expected_version,
    )
}

fn validate_relative_path(asset_id: &str, path: &Path) -> Result<(), PersistenceError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(PersistenceError::InvalidAssetManifest {
            asset_id: asset_id.to_string(),
            message: "path must be non-empty and relative",
        });
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(PersistenceError::InvalidAssetManifest {
                    asset_id: asset_id.to_string(),
                    message: "path may not contain parent/current/root prefixes",
                });
            }
        }
    }
    Ok(())
}

fn reject_engine_local_token(field: &'static str, value: &str) -> Result<(), PersistenceError> {
    let lower = value.to_ascii_lowercase();
    let leaks = [
        "entity(",
        "bevy::",
        "avian::",
        "wgpu::",
        "windowhandle",
        "rendererhandle",
        "oswindow",
    ];
    if leaks.iter().any(|needle| lower.contains(needle)) {
        Err(PersistenceError::EngineLocalIdLeak {
            field,
            value: value.to_string(),
        })
    } else {
        Ok(())
    }
}

fn contains_engine_local_runtime_token(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "entity(",
        "bevy::",
        "avian::",
        "wgpu::",
        "windowhandle",
        "rendererhandle",
        "oswindow",
        "handle<",
        "mesh3d",
        "standardmaterial",
        "egui",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[allow(dead_code)]
fn _asset_index(manifest: &AssetManifest) -> BTreeMap<&str, &AssetManifestEntry> {
    manifest
        .entries
        .iter()
        .map(|entry| (entry.asset_id.as_str(), entry))
        .collect()
}
