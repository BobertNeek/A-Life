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
    require_version, BrainCapacityClass, BrainScaleTier, CreatureGenome, FoundationWeightAsset,
    GenomeId, HomeostaticSnapshot, MemoryId, OrganismId, PackedExperienceFrame, PhenotypeCompiler,
    PhenotypeHash, PolicyBackend, ScaffoldContractError, SchemaKind, SchemaVersions, SensorProfile,
    TeacherPerceptionChannel, Tick, Validate, Vec3f, WorldEntityId,
};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::{
    appearance::CreatureAppearanceGenome,
    ecology::EcologyState,
    grounded_sensing::GroundedPhysicalProperties,
    habitat::{HabitatAuthority, HabitatAuthorityError, HabitatId},
    headless::{HeadlessWorld, HeadlessWorldPersistenceParts, WorldObject, WorldObjectKind},
    legacy_neural_policy_v1::LegacyBackendConfigV1,
    organism::{OrganismRegistryError, WorldOrganismRecord, WorldOrganismRegistry},
    persistent_voxel::{
        migrated_voxel_backend_for_world, PersistentVoxelProfileId, PersistentVoxelWorldSaveState,
    },
    tracked_objects::{
        PhysicalTrackingKey, PhysicalTrackingProvenance,
        PHYSICAL_TRACKING_PROVENANCE_SCHEMA_VERSION,
    },
    AudibleUtterance,
};

mod gpu_brain;
pub use gpu_brain::*;
mod gpu_brain_vnext;
pub use gpu_brain_vnext::*;

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
pub const WORLD_OBJECT_SAVE_SCHEMA_VERSION: u16 = 1;

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
    #[error("habitat authority violation: {0}")]
    Habitat(#[from] HabitatAuthorityError),
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
    CompositeGenome,
    FoundationWeights,
    LifetimeState,
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

    fn entry(&self, asset_id: &str) -> Result<&AssetManifestEntry, PersistenceError> {
        self.entries
            .iter()
            .find(|entry| entry.asset_id == asset_id)
            .ok_or_else(|| PersistenceError::MissingAssetReference {
                asset_id: asset_id.to_string(),
            })
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

pub fn persist_composite_genetic_birth_assets(
    root: impl AsRef<Path>,
    creature_genome: &CreatureGenome,
    foundation: &FoundationWeightAsset,
    phenotype_hash: PhenotypeHash,
) -> Result<(CompositeGeneticSaveRef, Vec<AssetManifestEntry>), PersistenceError> {
    creature_genome.validate_contract()?;
    if phenotype_hash.0 == [0; 4] {
        return Err(PersistenceError::InvalidConfig {
            field: "composite_genetics.phenotype_hash",
            message: "phenotype hash must be nonzero",
        });
    }
    let genome_bytes = serde_json::to_vec_pretty(creature_genome)?;
    let foundation_bytes = foundation.encode_canonical()?;
    let genome_entry = persist_content_addressed_asset(
        root.as_ref(),
        AssetKind::CompositeGenome,
        "composite-genomes",
        "json",
        &genome_bytes,
        "authoritative composite CreatureGenome",
    )?;
    let foundation_entry = persist_content_addressed_asset(
        root.as_ref(),
        AssetKind::FoundationWeights,
        "foundations",
        "alife-foundation",
        &foundation_bytes,
        "required immutable neural foundation",
    )?;
    let reference = CompositeGeneticSaveRef {
        schema_version: 1,
        creature_genome_asset_id: genome_entry.asset_id.clone(),
        foundation_asset_id: foundation_entry.asset_id.clone(),
        phenotype_hash,
    };
    Ok((reference, vec![genome_entry, foundation_entry]))
}

pub fn persist_creature_lifetime_state_asset(
    root: impl AsRef<Path>,
    state: &CreatureLifetimeStateAsset,
) -> Result<(CreatureLifetimeStateSaveRef, AssetManifestEntry), PersistenceError> {
    state.validate()?;
    let bytes = serde_json::to_vec_pretty(state)?;
    let entry = persist_content_addressed_asset(
        root.as_ref(),
        AssetKind::LifetimeState,
        "lifetime-state",
        "json",
        &bytes,
        "non-genetic creature lifetime memory and weights",
    )?;
    Ok((
        CreatureLifetimeStateSaveRef {
            schema_version: 1,
            asset_id: entry.asset_id.clone(),
        },
        entry,
    ))
}

fn persist_content_addressed_asset(
    root: &Path,
    kind: AssetKind,
    directory: &str,
    extension: &str,
    bytes: &[u8],
    provenance: &str,
) -> Result<AssetManifestEntry, PersistenceError> {
    if bytes.is_empty() {
        return Err(PersistenceError::InvalidAssetManifest {
            asset_id: directory.to_string(),
            message: "content-addressed asset cannot be empty",
        });
    }
    let digest = PortableAssetDigest::for_bytes(bytes);
    let digest_suffix =
        digest
            .0
            .strip_prefix("fnv1a64:")
            .ok_or(PersistenceError::InvalidAssetManifest {
                asset_id: directory.to_string(),
                message: "content-addressed asset digest is malformed",
            })?;
    let relative_path = format!("assets/{directory}/{digest_suffix}.{extension}");
    let path = root.join(&relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        if fs::read(&path)? != bytes {
            return Err(PersistenceError::DigestMismatch {
                asset_id: format!("{directory}-{digest_suffix}"),
                expected: digest.0.clone(),
                actual: PortableAssetDigest::for_file(&path)?.0,
            });
        }
    } else {
        fs::write(&path, bytes)?;
    }
    Ok(AssetManifestEntry {
        asset_id: format!("{directory}-{digest_suffix}"),
        kind,
        relative_path,
        digest,
        presence: AssetPresence::Required,
        schema_version: 1,
        size_bytes: Some(bytes.len() as u64),
        provenance: Some(provenance.to_string()),
    })
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
    #[serde(default)]
    pub composite_genetics: Option<CompositeGeneticSaveRef>,
    #[serde(default)]
    pub lifetime_state_asset: Option<CreatureLifetimeStateSaveRef>,
    #[serde(default)]
    pub gpu_brain: Option<GpuBrainSaveState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositeGeneticSaveRef {
    pub schema_version: u16,
    pub creature_genome_asset_id: String,
    pub foundation_asset_id: String,
    pub phenotype_hash: PhenotypeHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureLifetimeStateSaveRef {
    pub schema_version: u16,
    pub asset_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureLifetimeMemoryRecord {
    pub memory_id: MemoryId,
    pub source_organism_id: OrganismId,
    pub value_q16: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureLifetimeWeightValue {
    pub synapse_index: u32,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureLifetimeStateAsset {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub memory_records: Vec<CreatureLifetimeMemoryRecord>,
    pub lifetime_weight_values: Vec<CreatureLifetimeWeightValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedCompositeGeneticBirth {
    pub creature_genome: CreatureGenome,
    pub foundation: FoundationWeightAsset,
    pub phenotype_hash: PhenotypeHash,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorldObjectSaveState {
    pub schema_version: u16,
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
    pub grounded_physical: GroundedPhysicalProperties,
    pub tracking_provenance: PhysicalTrackingProvenance,
    pub tracking_key: PhysicalTrackingKey,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorldSaveState {
    pub seed: u64,
    pub tick: Tick,
    pub next_entity_id: u64,
    pub next_organism_id: u64,
    pub next_spawn_sequence: u64,
    pub next_utterance_id: u64,
    pub objects: Vec<WorldObjectSaveState>,
    pub last_touched_entities: Vec<WorldEntityId>,
    #[serde(default)]
    pub audible_utterances: Vec<AudibleUtterance>,
    #[serde(default)]
    pub last_creature_utterance_ticks: Vec<(OrganismId, Tick)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organism_records: Option<Vec<WorldOrganismRecord>>,
    #[serde(default)]
    pub ecology: EcologyState,
    #[serde(default)]
    pub voxel_backend: Option<PersistentVoxelWorldSaveState>,
    pub habitats: HabitatAuthority,
    #[serde(skip)]
    habitat_authority_was_missing: bool,
}

#[derive(Deserialize)]
struct WorldObjectSaveWire {
    #[serde(default)]
    schema_version: Option<u16>,
    id: WorldEntityId,
    label: String,
    kind: WorldObjectKind,
    organism_id: Option<OrganismId>,
    position: Vec3f,
    radius: f32,
    nutrition: f32,
    hazard_pain: f32,
    token_id: Option<u32>,
    social_affinity: f32,
    teacher_channel: Option<TeacherPerceptionChannel>,
    consumed: bool,
    carried_by: Option<OrganismId>,
    #[serde(default)]
    grounded_physical: Option<GroundedPhysicalProperties>,
    #[serde(default)]
    tracking_provenance: Option<PhysicalTrackingProvenance>,
    #[serde(default)]
    tracking_key: Option<PhysicalTrackingKey>,
}

impl WorldObjectSaveWire {
    fn into_current(
        self,
        world_seed: u64,
        canonical_spawn_sequence: u64,
    ) -> Result<WorldObjectSaveState, &'static str> {
        let (grounded_physical, tracking_provenance, tracking_key) = match (
            self.schema_version,
            self.grounded_physical,
            self.tracking_provenance,
            self.tracking_key,
        ) {
            (
                Some(WORLD_OBJECT_SAVE_SCHEMA_VERSION),
                Some(physical),
                Some(provenance),
                Some(key),
            ) => {
                physical
                    .validate_contract()
                    .map_err(|_| "invalid grounded physical properties")?;
                provenance
                    .validate_contract()
                    .map_err(|_| "invalid physical tracking provenance")?;
                if provenance.world_seed != world_seed || key != provenance.canonical_key() {
                    return Err("physical tracking provenance does not match world or key");
                }
                (physical, provenance, key)
            }
            (None, None, None, None) => {
                let provenance = PhysicalTrackingProvenance {
                    schema_version: PHYSICAL_TRACKING_PROVENANCE_SCHEMA_VERSION,
                    world_seed,
                    zone_id: 0,
                    spawn_sequence: canonical_spawn_sequence,
                    lineage_key: self.organism_id.map_or(0, OrganismId::raw),
                };
                let key = provenance.canonical_key();
                (
                    GroundedPhysicalProperties::deterministic_default(canonical_spawn_sequence),
                    provenance,
                    key,
                )
            }
            (Some(_), _, _, _) => return Err("unsupported world-object save schema"),
            _ => return Err("partial grounded world-object provenance is forbidden"),
        };
        Ok(WorldObjectSaveState {
            schema_version: WORLD_OBJECT_SAVE_SCHEMA_VERSION,
            id: self.id,
            label: self.label,
            kind: self.kind,
            organism_id: self.organism_id,
            position: self.position,
            radius: self.radius,
            nutrition: self.nutrition,
            hazard_pain: self.hazard_pain,
            token_id: self.token_id,
            social_affinity: self.social_affinity,
            teacher_channel: self.teacher_channel,
            consumed: self.consumed,
            carried_by: self.carried_by,
            grounded_physical,
            tracking_provenance,
            tracking_key,
        })
    }
}

impl<'de> Deserialize<'de> for WorldObjectSaveState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = WorldObjectSaveWire::deserialize(deserializer)?;
        let world_seed = wire
            .tracking_provenance
            .map(|provenance| provenance.world_seed)
            .ok_or_else(|| D::Error::custom("legacy world object requires world save context"))?;
        let spawn_sequence = wire
            .tracking_provenance
            .map(|provenance| provenance.spawn_sequence)
            .ok_or_else(|| D::Error::custom("missing physical tracking provenance"))?;
        wire.into_current(world_seed, spawn_sequence)
            .map_err(D::Error::custom)
    }
}

fn deserialize_present_organism_records<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<WorldOrganismRecord>>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<WorldOrganismRecord>::deserialize(deserializer).map(Some)
}

impl<'de> Deserialize<'de> for WorldSaveState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            seed: u64,
            tick: Tick,
            next_entity_id: u64,
            #[serde(default)]
            next_organism_id: Option<u64>,
            #[serde(default)]
            next_spawn_sequence: Option<u64>,
            objects: Vec<WorldObjectSaveWire>,
            last_touched_entities: Vec<WorldEntityId>,
            #[serde(default)]
            audible_utterances: Vec<AudibleUtterance>,
            #[serde(default)]
            next_utterance_id: Option<u64>,
            #[serde(default)]
            last_creature_utterance_ticks: Vec<(OrganismId, Tick)>,
            #[serde(default, deserialize_with = "deserialize_present_organism_records")]
            organism_records: Option<Vec<WorldOrganismRecord>>,
            #[serde(default)]
            ecology: EcologyState,
            #[serde(default)]
            voxel_backend: Option<PersistentVoxelWorldSaveState>,
            #[serde(default)]
            habitats: Option<HabitatAuthority>,
        }

        let mut wire = Wire::deserialize(deserializer)?;
        wire.objects.sort_by(|left, right| {
            left.id
                .raw()
                .cmp(&right.id.raw())
                .then_with(|| left.label.cmp(&right.label))
        });
        if let Some(records) = &mut wire.organism_records {
            records.sort_unstable_by_key(|record| record.organism_id().raw());
        }
        let mut objects = Vec::with_capacity(wire.objects.len());
        for (index, object) in wire.objects.into_iter().enumerate() {
            let spawn_sequence = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| D::Error::custom("world object count exceeds identity space"))?;
            objects.push(
                object
                    .into_current(wire.seed, spawn_sequence)
                    .map_err(D::Error::custom)?,
            );
        }
        let max_spawn_sequence = objects
            .iter()
            .map(|object| object.tracking_provenance.spawn_sequence)
            .max()
            .unwrap_or(0);
        let next_spawn_sequence = match wire.next_spawn_sequence {
            Some(value) => value,
            None => max_spawn_sequence
                .checked_add(1)
                .ok_or_else(|| D::Error::custom("world spawn identity space exhausted"))?,
        };
        let max_utterance_id = wire
            .audible_utterances
            .iter()
            .map(|utterance| utterance.utterance_id.raw())
            .max()
            .unwrap_or(0);
        let next_utterance_id = wire
            .next_utterance_id
            .unwrap_or_else(|| max_utterance_id.saturating_add(1));
        let max_present_organism_id = objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Agent)
            .filter_map(|object| object.organism_id.map(OrganismId::raw))
            .chain(
                wire.organism_records
                    .as_ref()
                    .into_iter()
                    .flat_map(|records| records.iter())
                    .map(|record| record.organism_id().raw()),
            )
            .max()
            .unwrap_or(0);
        let next_organism_id = match wire.next_organism_id {
            Some(value) => value,
            None => max_present_organism_id
                .checked_add(1)
                .ok_or_else(|| D::Error::custom("world organism identity space exhausted"))?,
        };
        let habitat_authority_was_missing = wire.habitats.is_none();
        let state = Self {
            seed: wire.seed,
            tick: wire.tick,
            next_entity_id: wire.next_entity_id,
            next_organism_id,
            next_spawn_sequence,
            next_utterance_id,
            objects,
            last_touched_entities: wire.last_touched_entities,
            audible_utterances: wire.audible_utterances,
            last_creature_utterance_ticks: wire.last_creature_utterance_ticks,
            organism_records: wire.organism_records,
            ecology: wire.ecology,
            voxel_backend: wire.voxel_backend,
            habitats: wire.habitats.unwrap_or_default(),
            habitat_authority_was_missing,
        };
        state
            .validate_organism_records()
            .map_err(|error| D::Error::custom(error.to_string()))?;
        Ok(state)
    }
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
        world_state.populate_default_habitat_memberships_if_unassigned(&creatures)?;
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
        let mut save: Self = serde_json::from_str(text)?;
        save.world.migrate_legacy_habitats(&save.creatures)?;
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
        self.assets.validate_with_root(root.as_ref())?;
        self.world.validate()?;
        self.adapter_remap.validate()?;
        self.school.validate()?;
        require_version(
            SchemaKind::PackedLog,
            PackedExperienceFrame::SCHEMA_VERSION,
            self.packed_log_schema_version,
        )?;
        for creature in &self.creatures {
            creature.validate(&self.assets, self.world.seed)?;
            if creature.composite_genetics.is_some() {
                self.load_composite_genetic_birth(creature.organism_id, root.as_ref())?;
            }
            if creature.lifetime_state_asset.is_some() {
                self.load_creature_lifetime_state(creature.organism_id, root.as_ref())?;
            }
        }
        self.validate_creature_summaries_against_organism_records()?;
        self.world
            .habitats
            .validate_at_tick(&creature_ids(&self.creatures)?, self.world.tick)?;
        for asset_id in self
            .generated_weight_asset_refs
            .iter()
            .chain(self.etf_prototype_asset_refs.iter())
        {
            require_asset_reference(&self.assets, asset_id)?;
        }
        Ok(())
    }

    fn validate_creature_summaries_against_organism_records(
        &self,
    ) -> Result<(), PersistenceError> {
        let Some(records) = &self.world.organism_records else {
            return Ok(());
        };

        for creature in &self.creatures {
            let mut matching = records
                .iter()
                .filter(|record| record.organism_id() == creature.organism_id);
            let Some(record) = matching.next() else {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.organism_id",
                    message: "creature summary must match exactly one world organism record",
                });
            };
            if matching.next().is_some() {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.organism_id",
                    message: "creature summary must match exactly one world organism record",
                });
            }

            let biochemistry = record.biochemistry();
            if creature.genome_id != record.genome().id {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.genome_id",
                    message: "creature summary disagrees with world organism record",
                });
            }
            if creature.brain_class.default_class_id() != record.genome().foundation.brain_class_id
            {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.brain_class",
                    message: "creature summary disagrees with world organism record",
                });
            }
            if creature.mind.tick != biochemistry.tick {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.mind.tick",
                    message: "creature summary disagrees with world organism record",
                });
            }
            if creature.mind.homeostasis != biochemistry.homeostasis {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.mind.homeostasis",
                    message: "creature summary disagrees with world organism record",
                });
            }
            if creature.development_tick != biochemistry.development.last_update_tick {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.development_tick",
                    message: "creature summary disagrees with world organism record",
                });
            }
        }
        Ok(())
    }

    pub fn load_composite_genetic_birth(
        &self,
        organism_id: OrganismId,
        root: impl AsRef<Path>,
    ) -> Result<LoadedCompositeGeneticBirth, PersistenceError> {
        let creature = self.creature(organism_id)?;
        let reference =
            creature
                .composite_genetics
                .as_ref()
                .ok_or(PersistenceError::InvalidConfig {
                    field: "creature.composite_genetics",
                    message: "creature has no composite genetic asset reference",
                })?;
        reference.validate(&self.assets)?;
        let genome_entry = self.assets.entry(&reference.creature_genome_asset_id)?;
        let foundation_entry = self.assets.entry(&reference.foundation_asset_id)?;
        if genome_entry.kind != AssetKind::CompositeGenome
            || foundation_entry.kind != AssetKind::FoundationWeights
        {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.composite_genetics",
                message: "composite genome and foundation asset kinds must match",
            });
        }
        let creature_genome = serde_json::from_slice::<CreatureGenome>(&fs::read(
            root.as_ref().join(&genome_entry.relative_path),
        )?)?;
        creature_genome.validate_contract()?;
        let foundation = FoundationWeightAsset::decode_canonical(&fs::read(
            root.as_ref().join(&foundation_entry.relative_path),
        )?)?;
        let manifest = foundation.manifest();
        if creature_genome.id != creature.genome_id
            || creature_genome.foundation.foundation_id != manifest.foundation_id().raw()
            || u32::from(creature_genome.foundation.version) != manifest.foundation_version().raw()
            || creature_genome.foundation.compatibility_family_id
                != manifest.compatibility_family_id().raw()
        {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.composite_genetics",
                message: "composite genome identity does not match creature or foundation",
            });
        }
        let expressed = creature_genome.express()?;
        let development = expressed.development_state_at(Tick::new(u64::from(
            expressed.development.maturation_duration_ticks,
        )))?;
        let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &expressed.brain_genome,
            &BrainCapacityClass::production_for_id(creature_genome.foundation.brain_class_id)?,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &foundation,
        )?;
        if phenotype.phenotype_hash() != reference.phenotype_hash {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.composite_genetics.phenotype_hash",
                message: "restored composite genome phenotype hash does not match save",
            });
        }
        Ok(LoadedCompositeGeneticBirth {
            creature_genome,
            foundation,
            phenotype_hash: phenotype.phenotype_hash(),
        })
    }

    pub fn load_creature_lifetime_state(
        &self,
        organism_id: OrganismId,
        root: impl AsRef<Path>,
    ) -> Result<CreatureLifetimeStateAsset, PersistenceError> {
        let creature = self.creature(organism_id)?;
        let reference =
            creature
                .lifetime_state_asset
                .as_ref()
                .ok_or(PersistenceError::InvalidConfig {
                    field: "creature.lifetime_state_asset",
                    message: "creature has no lifetime-state asset reference",
                })?;
        reference.validate(&self.assets)?;
        let entry = self.assets.entry(&reference.asset_id)?;
        if entry.kind != AssetKind::LifetimeState {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.lifetime_state_asset",
                message: "lifetime-state reference has the wrong asset kind",
            });
        }
        let state = serde_json::from_slice::<CreatureLifetimeStateAsset>(&fs::read(
            root.as_ref().join(&entry.relative_path),
        )?)?;
        state.validate()?;
        let memory_ids = state
            .memory_records
            .iter()
            .map(|record| record.memory_id)
            .collect::<Vec<_>>();
        if state.organism_id != organism_id
            || usize::try_from(creature.mind.memory_record_count).ok()
                != Some(state.memory_records.len())
            || creature.mind.memory_source_ids != memory_ids
            || usize::try_from(creature.weights.lifetime_consolidated_entries).ok()
                != Some(state.lifetime_weight_values.len())
        {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.lifetime_state_asset",
                message: "lifetime-state payload does not match creature summary",
            });
        }
        Ok(state)
    }

    fn creature(&self, organism_id: OrganismId) -> Result<&CreatureSaveState, PersistenceError> {
        self.creatures
            .iter()
            .find(|creature| creature.organism_id == organism_id)
            .ok_or(PersistenceError::InvalidConfig {
                field: "creature.organism_id",
                message: "save has no creature with the requested organism id",
            })
    }

    pub fn restore_headless_world(&self) -> Result<HeadlessWorld, PersistenceError> {
        self.world
            .habitats
            .validate_at_tick(&creature_ids(&self.creatures)?, self.world.tick)?;
        self.world.restore()
    }

    /// Replaces only the engine-neutral world snapshot while preserving the
    /// save's configuration, assets, creatures, school state, adapter remaps,
    /// and renderer-owned voxel snapshot. GPU checkpoint publication uses this
    /// at an explicit sealed boundary so the world tick and every brain
    /// checkpoint remain one atomic portable-save generation.
    pub fn replace_headless_world_snapshot(
        &mut self,
        world: &HeadlessWorld,
    ) -> Result<(), PersistenceError> {
        if world.seed() != self.deterministic_seed || world.seed() != self.config.deterministic_seed
        {
            return Err(PersistenceError::InvalidConfig {
                field: "deterministic_seed",
                message: "replacement world must preserve the save seed",
            });
        }
        let voxel_backend = self.world.voxel_backend.clone();
        let mut candidate = WorldSaveState::from_parts(world.persistence_parts());
        candidate.voxel_backend = voxel_backend;
        candidate.populate_default_habitat_memberships_if_unassigned(&self.creatures)?;
        candidate.validate()?;
        self.world = candidate;
        if let Some(gpu_runtime) = self.gpu_runtime.as_mut() {
            gpu_runtime.last_safe_checkpoint.world_tick = self.world.tick;
        }
        Ok(())
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
    fn validate(&self, assets: &AssetManifest, world_seed: u64) -> Result<(), PersistenceError> {
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
        if let Some(reference) = &self.composite_genetics {
            reference.validate(assets)?;
        }
        if let Some(reference) = &self.lifetime_state_asset {
            reference.validate(assets)?;
        }
        if self.learning.lamarckian_mode_enabled {
            return Err(PersistenceError::InvalidConfig {
                field: "learning.lamarckian_mode_enabled",
                message: "portable P34 saves keep Lamarckian inheritance default-off",
            });
        }
        if let Some(gpu_brain) = &self.gpu_brain {
            gpu_brain.validate()?;
            gpu_brain.validate_asset_manifest(assets)?;
            let expected_class = match self.brain_class {
                BrainScaleTier::Nano512 => alife_core::BrainCapacityClass::N512_ID,
                BrainScaleTier::Small1024 => alife_core::BrainCapacityClass::N1024_ID,
                BrainScaleTier::Standard2048 => alife_core::BrainCapacityClass::N2048_ID,
                _ => {
                    return Err(PersistenceError::InvalidConfig {
                        field: "creature.gpu_brain.capacity_class_id",
                        message: "GPU checkpoint requires a promoted production brain class",
                    });
                }
            };
            if gpu_brain.organism_id != self.organism_id
                || gpu_brain.capacity_class_id != expected_class
                || gpu_brain.tracked_objects.world_seed != world_seed
            {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.gpu_brain",
                    message: "GPU checkpoint identity must match its creature record",
                });
            }
        }
        Ok(())
    }
}

impl CompositeGeneticSaveRef {
    fn validate(&self, assets: &AssetManifest) -> Result<(), PersistenceError> {
        if self.schema_version != 1
            || self.creature_genome_asset_id.is_empty()
            || self.foundation_asset_id.is_empty()
            || self.creature_genome_asset_id == self.foundation_asset_id
            || self.phenotype_hash.0 == [0; 4]
        {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.composite_genetics",
                message: "composite genetic asset reference is invalid",
            });
        }
        require_asset_reference(assets, &self.creature_genome_asset_id)?;
        require_asset_reference(assets, &self.foundation_asset_id)
    }
}

impl CreatureLifetimeStateSaveRef {
    fn validate(&self, assets: &AssetManifest) -> Result<(), PersistenceError> {
        if self.schema_version != 1 || self.asset_id.is_empty() {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.lifetime_state_asset",
                message: "lifetime-state asset reference is invalid",
            });
        }
        require_asset_reference(assets, &self.asset_id)
    }
}

impl CreatureLifetimeStateAsset {
    fn validate(&self) -> Result<(), PersistenceError> {
        self.organism_id.validate()?;
        if self.schema_version != 1
            || self.memory_records.len() > 65_536
            || self.lifetime_weight_values.len() > 1_048_576
        {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.lifetime_state",
                message: "lifetime-state asset schema or bounds are invalid",
            });
        }
        let mut memory_ids = BTreeSet::new();
        for record in &self.memory_records {
            record.memory_id.validate()?;
            record.source_organism_id.validate()?;
            if record.value_q16 > 65_535 || !memory_ids.insert(record.memory_id.0) {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.lifetime_state.memory_records",
                    message: "lifetime memory identity or value is invalid",
                });
            }
        }
        let mut synapse_indices = BTreeSet::new();
        for weight in &self.lifetime_weight_values {
            if !weight.value.is_finite()
                || weight.value == 0.0
                || !synapse_indices.insert(weight.synapse_index)
            {
                return Err(PersistenceError::InvalidConfig {
                    field: "creature.lifetime_state.lifetime_weight_values",
                    message: "lifetime weight identity or value is invalid",
                });
            }
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

fn creature_ids(creatures: &[CreatureSaveState]) -> Result<Vec<OrganismId>, PersistenceError> {
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(creatures.len());
    for creature in creatures {
        if !creature.organism_id.is_valid() {
            return Err(PersistenceError::Habitat(
                HabitatAuthorityError::UnknownCreature(creature.organism_id),
            ));
        }
        if !seen.insert(creature.organism_id.raw()) {
            return Err(PersistenceError::InvalidConfig {
                field: "creatures.organism_id",
                message: "creature organism ids must be unique",
            });
        }
        ids.push(creature.organism_id);
    }
    Ok(ids)
}

fn map_organism_registry_error(error: OrganismRegistryError) -> PersistenceError {
    match error {
        OrganismRegistryError::InvalidRecord(error) => PersistenceError::Contract(error),
        _ => PersistenceError::Contract(ScaffoldContractError::InvalidId),
    }
}

impl WorldSaveState {
    fn populate_default_habitat_memberships_if_unassigned(
        &mut self,
        creatures: &[CreatureSaveState],
    ) -> Result<(), PersistenceError> {
        let ids = creature_ids(creatures)?;
        if self.habitats.is_unassigned_default() {
            for organism_id in &ids {
                self.habitats.register_creature(
                    *organism_id,
                    HabitatId::DEFAULT_WILD,
                    Tick::ZERO,
                )?;
            }
        }
        self.habitats.validate_at_tick(&ids, self.tick)?;
        Ok(())
    }

    fn migrate_legacy_habitats(
        &mut self,
        creatures: &[CreatureSaveState],
    ) -> Result<(), PersistenceError> {
        if self.habitat_authority_was_missing {
            self.habitats = HabitatAuthority::default();
            self.populate_default_habitat_memberships_if_unassigned(creatures)?;
            self.habitat_authority_was_missing = false;
        }
        Ok(())
    }

    fn from_parts(parts: HeadlessWorldPersistenceParts) -> Self {
        let mut organism_records = parts.organism_records;
        if let Some(records) = &mut organism_records {
            records.sort_unstable_by_key(|record| record.organism_id().raw());
        }
        Self {
            seed: parts.seed,
            tick: parts.tick,
            next_entity_id: parts.next_entity_id,
            next_organism_id: parts.next_organism_id,
            next_spawn_sequence: parts.next_spawn_sequence,
            next_utterance_id: parts.next_utterance_id,
            objects: parts
                .objects
                .into_iter()
                .map(WorldObjectSaveState::from)
                .collect(),
            last_touched_entities: parts.last_touched_entities,
            audible_utterances: parts.audible_utterances,
            last_creature_utterance_ticks: parts.last_creature_utterance_ticks,
            organism_records,
            ecology: parts.ecology,
            voxel_backend: None,
            habitats: parts.habitats,
            habitat_authority_was_missing: false,
        }
    }

    fn validate_organism_records(&self) -> Result<(), PersistenceError> {
        let max_present_organism_id = self
            .objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Agent)
            .filter_map(|object| object.organism_id.map(OrganismId::raw))
            .chain(
                self.organism_records
                    .as_ref()
                    .into_iter()
                    .flat_map(|records| records.iter())
                    .map(|record| record.organism_id().raw()),
            )
            .max()
            .unwrap_or(0);
        if self.next_organism_id == 0
            || max_present_organism_id == u64::MAX
            || self.next_organism_id <= max_present_organism_id
        {
            return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
        }
        let Some(records) = &self.organism_records else {
            return Ok(());
        };
        let registry = WorldOrganismRegistry::from_exact_records(records.clone())
            .map_err(map_organism_registry_error)?;
        let mut objects_by_entity = BTreeMap::new();
        let mut agent_bindings = BTreeMap::new();
        for object in &self.objects {
            if objects_by_entity.insert(object.id.raw(), object).is_some() {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
            if object.kind != WorldObjectKind::Agent {
                continue;
            }
            let organism_id = object
                .organism_id
                .ok_or(PersistenceError::Contract(ScaffoldContractError::InvalidId))?;
            organism_id.validate()?;
            if agent_bindings
                .insert(organism_id.raw(), object.id)
                .is_some()
            {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
        }
        let registered_ids = registry
            .iter()
            .map(|record| record.organism_id().raw())
            .collect::<BTreeSet<_>>();
        let agent_ids = agent_bindings.keys().copied().collect::<BTreeSet<_>>();
        if registered_ids != agent_ids {
            return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
        }
        for record in registry.iter() {
            let Some(object) = objects_by_entity.get(&record.world_entity_id().raw()) else {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            };
            if object.kind != WorldObjectKind::Agent
                || object.organism_id != Some(record.organism_id())
            {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
        }
        Ok(())
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
        let mut spawn_sequences = BTreeSet::new();
        let mut max_spawn_sequence = 0_u64;
        for object in &self.objects {
            object.validate()?;
            if !ids.insert(object.id.raw()) || !labels.insert(object.label.clone()) {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
            max_id = max_id.max(object.id.raw());
            if object.tracking_provenance.world_seed != self.seed
                || !spawn_sequences.insert(object.tracking_provenance.spawn_sequence)
            {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
            max_spawn_sequence = max_spawn_sequence.max(object.tracking_provenance.spawn_sequence);
        }
        if self.next_entity_id <= max_id || (self.objects.is_empty() && self.next_entity_id == 0) {
            return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
        }
        if self.next_spawn_sequence == 0 || self.next_spawn_sequence <= max_spawn_sequence {
            return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
        }
        let max_utterance_id = self
            .audible_utterances
            .iter()
            .map(|utterance| utterance.utterance_id.raw())
            .max()
            .unwrap_or(0);
        if self.next_utterance_id == 0 || self.next_utterance_id <= max_utterance_id {
            return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
        }
        let mut cooldown_organisms = BTreeSet::new();
        for (organism, tick) in &self.last_creature_utterance_ticks {
            organism.validate()?;
            if tick.raw() > self.tick.raw() || !cooldown_organisms.insert(organism.raw()) {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
        }
        for touched in &self.last_touched_entities {
            touched.validate()?;
            if !ids.contains(&touched.raw()) {
                return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
            }
        }
        for utterance in &self.audible_utterances {
            utterance.validate_contract()?;
            if utterance.emitted_tick.raw() > self.tick.raw()
                || utterance.expires_after_tick.raw() < self.tick.raw()
            {
                return Err(PersistenceError::Contract(
                    ScaffoldContractError::InvalidPerceptionFrame,
                ));
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
        self.validate_organism_records()?;
        let habitat_creatures = self
            .habitats
            .memberships()
            .iter()
            .map(|membership| membership.organism_id)
            .collect::<Vec<_>>();
        self.habitats
            .validate_at_tick(&habitat_creatures, self.tick)?;
        Ok(())
    }

    fn restore(&self) -> Result<HeadlessWorld, PersistenceError> {
        self.validate()?;
        let parts = HeadlessWorldPersistenceParts {
            seed: self.seed,
            tick: self.tick,
            next_entity_id: self.next_entity_id,
            next_organism_id: self.next_organism_id,
            next_spawn_sequence: self.next_spawn_sequence,
            next_utterance_id: self.next_utterance_id,
            objects: self
                .objects
                .iter()
                .cloned()
                .map(WorldObject::from)
                .collect(),
            last_touched_entities: self.last_touched_entities.clone(),
            ecology: self.ecology.clone(),
            audible_utterances: self.audible_utterances.clone(),
            last_creature_utterance_ticks: self.last_creature_utterance_ticks.clone(),
            habitats: self.habitats.clone(),
            organism_records: self.organism_records.clone(),
        };
        Ok(HeadlessWorld::from_persistence_parts(parts)?)
    }
}

impl WorldObjectSaveState {
    fn validate(&self) -> Result<(), PersistenceError> {
        if self.schema_version != WORLD_OBJECT_SAVE_SCHEMA_VERSION {
            return Err(PersistenceError::SchemaVersion {
                schema: "alife.world_object.v1",
                expected: WORLD_OBJECT_SAVE_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
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
        self.grounded_physical.validate_contract()?;
        self.tracking_provenance.validate_contract()?;
        if self.tracking_key != self.tracking_provenance.canonical_key() {
            return Err(PersistenceError::Contract(ScaffoldContractError::InvalidId));
        }
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
            schema_version: WORLD_OBJECT_SAVE_SCHEMA_VERSION,
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
            grounded_physical: value.grounded_physical,
            tracking_provenance: value.tracking_provenance,
            tracking_key: value.tracking_key,
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
            grounded_physical: value.grounded_physical,
            tracking_provenance: value.tracking_provenance,
            tracking_key: value.tracking_key,
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
