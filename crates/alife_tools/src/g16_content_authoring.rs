//! G16 content authoring validation for tiny worlds, lessons, presets, and assets.
//!
//! This module is tooling-only. It validates versioned content pack fixtures and
//! P34 references without making authoring tools a runtime dependency.

use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

use alife_core::{
    BrainClassSpec, BrainGenome, BrainScaleTier, GenomeId, OrganismId, Validate, Vec3f,
    WorldEntityId,
};
use alife_world::persistence::{AssetManifest, PersistenceError, PortableSaveFile, RuntimeConfig};
use serde::Deserialize;
use thiserror::Error;

pub const G16_CONTENT_PACK_SCHEMA: &str = "alife.g16.content_pack.v1";
pub const G16_CONTENT_PACK_SCHEMA_VERSION: u16 = 1;
pub const G16_WORLD_PRESET_SCHEMA: &str = "alife.g16.world_preset.v1";
pub const G16_WORLD_PRESET_SCHEMA_VERSION: u16 = 1;
pub const G16_LESSON_PACK_SCHEMA: &str = "alife.g16.lesson_pack.v1";
pub const G16_LESSON_PACK_SCHEMA_VERSION: u16 = 1;
pub const G16_CREATURE_PRESET_SCHEMA: &str = "alife.g16.creature_preset.v1";
pub const G16_CREATURE_PRESET_SCHEMA_VERSION: u16 = 1;
pub const G16_MAX_CONTENT_FILE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Error)]
pub enum ContentAuthoringError {
    #[error("core contract violation: {0}")]
    Core(#[from] alife_core::ScaffoldContractError),
    #[error("persistence contract violation: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("schema mismatch for {field}: expected {expected}, got {actual}")]
    Schema {
        field: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("schema version mismatch for {field}: expected {expected}, got {actual}")]
    SchemaVersion {
        field: &'static str,
        expected: u16,
        actual: u16,
    },
    #[error("invalid content field {field}: {message}")]
    InvalidContent {
        field: &'static str,
        message: &'static str,
    },
    #[error("missing required content {field} at {path:?}")]
    MissingContent { field: &'static str, path: PathBuf },
    #[error("content file is too large: {path:?} has {bytes} bytes")]
    OversizedContent { path: PathBuf, bytes: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ContentPackManifest {
    pub schema: String,
    pub schema_version: u16,
    pub pack_id: String,
    pub display_name: String,
    pub p34_runtime_config: String,
    pub p34_asset_manifest: String,
    pub p34_asset_root: String,
    pub p34_save: String,
    pub worlds: Vec<ContentEntry>,
    pub lessons: Vec<ContentEntry>,
    pub creatures: Vec<ContentEntry>,
    pub generated_weight_refs: Vec<ContentAssetRef>,
    pub semantic_assets: Vec<ContentAssetRef>,
    pub scenario_packs: Vec<ContentEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ContentEntry {
    pub id: String,
    pub relative_path: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ContentAssetRef {
    pub asset_id: String,
    pub relative_path: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentPackValidationReport {
    pub pack_id: String,
    pub checked_files: usize,
    pub world_presets: usize,
    pub lesson_packs: usize,
    pub creature_presets: usize,
    pub generated_weight_refs: usize,
    pub semantic_assets: usize,
    pub largest_file_bytes: u64,
    pub stable_id_worlds: usize,
    pub perception_only_lessons: usize,
    pub valid_creature_presets: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorldPreset {
    pub schema: String,
    pub schema_version: u16,
    pub world_id: String,
    pub stable_seed: u64,
    pub objects: Vec<WorldPresetObject>,
    pub terrain_zones: Vec<TerrainZone>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorldPresetObject {
    pub stable_id: u64,
    pub label: String,
    pub kind: WorldObjectPresetKind,
    pub organism_id: Option<u64>,
    pub position: [f32; 3],
    pub radius: f32,
    pub nutrition: f32,
    pub hazard_pain: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum WorldObjectPresetKind {
    Agent,
    Food,
    Hazard,
    Obstacle,
    Token,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TerrainZone {
    pub zone_id: u64,
    pub label: String,
    pub kind: String,
    pub center: [f32; 3],
    pub radius: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LessonPack {
    pub schema: String,
    pub schema_version: u16,
    pub lesson_pack_id: String,
    pub steps: Vec<LessonStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LessonStep {
    pub lesson_id: u64,
    pub kind: String,
    pub channel: LessonChannel,
    pub token_id: Option<u64>,
    pub target_world_entity_id: Option<u64>,
    pub expected_observation: String,
    pub perception_only: bool,
    pub direct_motor_bypass: bool,
    pub hidden_vector_injection: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum LessonChannel {
    Hearing,
    Vision,
    Touch,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreaturePreset {
    pub schema: String,
    pub schema_version: u16,
    pub preset_id: String,
    pub organism_id: u64,
    pub genome_id: u64,
    pub brain_class: BrainScaleTier,
    pub generated_weight_asset_id: String,
    pub inherited_weight_only: bool,
    pub lifetime_state_included: bool,
    pub role_tags: Vec<String>,
}

pub fn validate_content_pack(
    manifest_path: impl AsRef<Path>,
) -> Result<ContentPackValidationReport, ContentAuthoringError> {
    let manifest_path = absolute_existing_path(manifest_path.as_ref())?;
    let workspace_root = find_workspace_root(&manifest_path)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| find_workspace_root(&cwd))
        })
        .ok_or(ContentAuthoringError::InvalidContent {
            field: "workspace_root",
            message: "could not find Cargo.toml workspace root",
        })?;
    let manifest = read_json::<ContentPackManifest>(&manifest_path)?;
    require_schema(
        "content_pack.schema",
        &manifest.schema,
        G16_CONTENT_PACK_SCHEMA,
        manifest.schema_version,
        G16_CONTENT_PACK_SCHEMA_VERSION,
    )?;
    require_nonempty_id("pack_id", &manifest.pack_id)?;
    require_nonempty_id("display_name", &manifest.display_name)?;

    let mut checked_files = 1;
    let mut largest_file_bytes = file_size(&manifest_path)?;

    let config_path = resolve_workspace_path(
        &workspace_root,
        "p34_runtime_config",
        &manifest.p34_runtime_config,
    )?;
    largest_file_bytes = largest_file_bytes.max(file_size(&config_path)?);
    checked_files += 1;
    let config = RuntimeConfig::from_json_file(&config_path)?;
    config.validate()?;

    let asset_manifest_path = resolve_workspace_path(
        &workspace_root,
        "p34_asset_manifest",
        &manifest.p34_asset_manifest,
    )?;
    largest_file_bytes = largest_file_bytes.max(file_size(&asset_manifest_path)?);
    checked_files += 1;
    let asset_root =
        resolve_workspace_path(&workspace_root, "p34_asset_root", &manifest.p34_asset_root)?;
    let asset_manifest = AssetManifest::from_json_file(&asset_manifest_path)?;
    asset_manifest.validate_with_root(&asset_root)?;

    let save_path = resolve_workspace_path(&workspace_root, "p34_save", &manifest.p34_save)?;
    largest_file_bytes = largest_file_bytes.max(file_size(&save_path)?);
    checked_files += 1;
    let save = PortableSaveFile::from_json_file(&save_path)?;
    save.validate_with_asset_root(&asset_root)?;

    let mut seen_content_ids = BTreeSet::new();
    let mut stable_id_worlds = 0;
    for entry in &manifest.worlds {
        validate_entry(entry, &mut seen_content_ids)?;
        if let Some(path) = required_or_optional_path(
            &workspace_root,
            "world.relative_path",
            entry.required,
            &entry.relative_path,
        )? {
            largest_file_bytes = largest_file_bytes.max(validate_content_file(&path)?);
            checked_files += 1;
            validate_world_preset_file(&path)?;
            stable_id_worlds += 1;
        }
    }

    let mut perception_only_lessons = 0;
    for entry in &manifest.lessons {
        validate_entry(entry, &mut seen_content_ids)?;
        if let Some(path) = required_or_optional_path(
            &workspace_root,
            "lesson.relative_path",
            entry.required,
            &entry.relative_path,
        )? {
            largest_file_bytes = largest_file_bytes.max(validate_content_file(&path)?);
            checked_files += 1;
            let lesson = validate_lesson_pack_file(&path)?;
            perception_only_lessons += lesson.steps.len();
        }
    }

    let mut valid_creature_presets = 0;
    for entry in &manifest.creatures {
        validate_entry(entry, &mut seen_content_ids)?;
        if let Some(path) = required_or_optional_path(
            &workspace_root,
            "creature.relative_path",
            entry.required,
            &entry.relative_path,
        )? {
            largest_file_bytes = largest_file_bytes.max(validate_content_file(&path)?);
            checked_files += 1;
            validate_creature_preset_file(&path, &asset_manifest)?;
            valid_creature_presets += 1;
        }
    }

    for entry in &manifest.generated_weight_refs {
        validate_asset_ref(entry, &asset_manifest)?;
        if let Some(path) = required_or_optional_path(
            &workspace_root,
            "generated_weight.relative_path",
            entry.required,
            &entry.relative_path,
        )? {
            largest_file_bytes = largest_file_bytes.max(validate_content_file(&path)?);
            checked_files += 1;
        }
    }

    for entry in &manifest.semantic_assets {
        validate_asset_ref(entry, &asset_manifest)?;
        if let Some(path) = required_or_optional_path(
            &workspace_root,
            "semantic.relative_path",
            entry.required,
            &entry.relative_path,
        )? {
            largest_file_bytes = largest_file_bytes.max(validate_content_file(&path)?);
            checked_files += 1;
        }
    }

    for entry in &manifest.scenario_packs {
        validate_entry(entry, &mut seen_content_ids)?;
        if let Some(path) = required_or_optional_path(
            &workspace_root,
            "scenario.relative_path",
            entry.required,
            &entry.relative_path,
        )? {
            largest_file_bytes = largest_file_bytes.max(validate_content_file(&path)?);
            checked_files += 1;
        }
    }

    Ok(ContentPackValidationReport {
        pack_id: manifest.pack_id,
        checked_files,
        world_presets: manifest.worlds.len(),
        lesson_packs: manifest.lessons.len(),
        creature_presets: manifest.creatures.len(),
        generated_weight_refs: manifest.generated_weight_refs.len(),
        semantic_assets: manifest.semantic_assets.len(),
        largest_file_bytes,
        stable_id_worlds,
        perception_only_lessons,
        valid_creature_presets,
    })
}

pub fn validate_world_preset_file(
    path: impl AsRef<Path>,
) -> Result<WorldPreset, ContentAuthoringError> {
    let world = read_json::<WorldPreset>(path.as_ref())?;
    validate_world_preset(&world)?;
    Ok(world)
}

pub fn validate_lesson_pack_file(
    path: impl AsRef<Path>,
) -> Result<LessonPack, ContentAuthoringError> {
    let lesson = read_json::<LessonPack>(path.as_ref())?;
    validate_lesson_pack(&lesson)?;
    Ok(lesson)
}

pub fn validate_creature_preset_file(
    path: impl AsRef<Path>,
    asset_manifest: &AssetManifest,
) -> Result<CreaturePreset, ContentAuthoringError> {
    let creature = read_json::<CreaturePreset>(path.as_ref())?;
    validate_creature_preset(&creature, asset_manifest)?;
    Ok(creature)
}

pub fn validate_world_preset(world: &WorldPreset) -> Result<(), ContentAuthoringError> {
    require_schema(
        "world.schema",
        &world.schema,
        G16_WORLD_PRESET_SCHEMA,
        world.schema_version,
        G16_WORLD_PRESET_SCHEMA_VERSION,
    )?;
    require_nonempty_id("world_id", &world.world_id)?;
    if world.stable_seed == 0 {
        return invalid("stable_seed", "stable seed must be nonzero");
    }
    if world.objects.is_empty() {
        return invalid("objects", "world preset must contain at least one object");
    }
    let mut ids = BTreeSet::new();
    let mut labels = BTreeSet::new();
    for object in &world.objects {
        WorldEntityId(object.stable_id).validate()?;
        if !ids.insert(object.stable_id) {
            return invalid("objects.stable_id", "duplicate stable world entity id");
        }
        require_nonempty_id("objects.label", &object.label)?;
        reject_engine_local_text("objects.label", &object.label)?;
        if !labels.insert(object.label.clone()) {
            return invalid("objects.label", "duplicate object label");
        }
        if let Some(organism_id) = object.organism_id {
            OrganismId(organism_id).validate()?;
        }
        validate_vec3("objects.position", object.position)?;
        validate_unitish_nonnegative("objects.radius", object.radius, 0.01, 128.0)?;
        validate_unitish_nonnegative("objects.nutrition", object.nutrition, 0.0, 1.0)?;
        validate_unitish_nonnegative("objects.hazard_pain", object.hazard_pain, 0.0, 1.0)?;
    }
    for zone in &world.terrain_zones {
        if zone.zone_id == 0 {
            return invalid("terrain_zones.zone_id", "terrain zone id must be nonzero");
        }
        require_nonempty_id("terrain_zones.label", &zone.label)?;
        require_nonempty_id("terrain_zones.kind", &zone.kind)?;
        validate_vec3("terrain_zones.center", zone.center)?;
        validate_unitish_nonnegative("terrain_zones.radius", zone.radius, 0.01, 1024.0)?;
    }
    Ok(())
}

pub fn validate_lesson_pack(lesson: &LessonPack) -> Result<(), ContentAuthoringError> {
    require_schema(
        "lesson.schema",
        &lesson.schema,
        G16_LESSON_PACK_SCHEMA,
        lesson.schema_version,
        G16_LESSON_PACK_SCHEMA_VERSION,
    )?;
    require_nonempty_id("lesson_pack_id", &lesson.lesson_pack_id)?;
    if lesson.steps.is_empty() {
        return invalid("steps", "lesson pack must contain at least one step");
    }
    let mut ids = BTreeSet::new();
    for step in &lesson.steps {
        if step.lesson_id == 0 || !ids.insert(step.lesson_id) {
            return invalid("steps.lesson_id", "lesson ids must be nonzero and unique");
        }
        require_nonempty_id("steps.kind", &step.kind)?;
        require_nonempty_id("steps.expected_observation", &step.expected_observation)?;
        if let Some(token_id) = step.token_id {
            if token_id == 0 {
                return invalid("steps.token_id", "token id must be nonzero when present");
            }
        }
        if let Some(target) = step.target_world_entity_id {
            WorldEntityId(target).validate()?;
        }
        if !step.perception_only || step.direct_motor_bypass || step.hidden_vector_injection {
            return invalid(
                "steps.perception_boundary",
                "lessons must be perception-only with no motor bypass or hidden vector injection",
            );
        }
    }
    Ok(())
}

pub fn validate_creature_preset(
    creature: &CreaturePreset,
    asset_manifest: &AssetManifest,
) -> Result<(), ContentAuthoringError> {
    require_schema(
        "creature.schema",
        &creature.schema,
        G16_CREATURE_PRESET_SCHEMA,
        creature.schema_version,
        G16_CREATURE_PRESET_SCHEMA_VERSION,
    )?;
    require_nonempty_id("preset_id", &creature.preset_id)?;
    let organism_id = OrganismId(creature.organism_id).validate()?;
    let genome_id = GenomeId(creature.genome_id).validate()?;
    BrainClassSpec::for_tier(creature.brain_class).validate()?;
    let mut genome =
        BrainGenome::scaffold(creature.genome_id, creature.brain_class.default_class_id());
    genome.id = genome_id;
    genome.validate_contract()?;
    if organism_id.raw() == genome_id.raw() && creature.role_tags.is_empty() {
        return invalid("role_tags", "creature preset role tags are required");
    }
    if !creature.inherited_weight_only || creature.lifetime_state_included {
        return invalid(
            "weight_boundary",
            "creature presets may reference inherited birth weights only, not lifetime state",
        );
    }
    if !asset_manifest.contains_asset(&creature.generated_weight_asset_id) {
        return Err(ContentAuthoringError::Persistence(
            PersistenceError::MissingAssetReference {
                asset_id: creature.generated_weight_asset_id.clone(),
            },
        ));
    }
    for tag in &creature.role_tags {
        require_nonempty_id("role_tags", tag)?;
        reject_engine_local_text("role_tags", tag)?;
    }
    Ok(())
}

fn validate_entry(
    entry: &ContentEntry,
    seen_content_ids: &mut BTreeSet<String>,
) -> Result<(), ContentAuthoringError> {
    require_nonempty_id("content_entry.id", &entry.id)?;
    if !seen_content_ids.insert(entry.id.clone()) {
        return invalid("content_entry.id", "duplicate content entry id");
    }
    validate_relative_path(
        "content_entry.relative_path",
        Path::new(&entry.relative_path),
    )?;
    Ok(())
}

fn validate_asset_ref(
    entry: &ContentAssetRef,
    asset_manifest: &AssetManifest,
) -> Result<(), ContentAuthoringError> {
    require_nonempty_id("asset_ref.asset_id", &entry.asset_id)?;
    validate_relative_path("asset_ref.relative_path", Path::new(&entry.relative_path))?;
    if entry.required && !asset_manifest.contains_asset(&entry.asset_id) {
        return Err(ContentAuthoringError::Persistence(
            PersistenceError::MissingAssetReference {
                asset_id: entry.asset_id.clone(),
            },
        ));
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ContentAuthoringError> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn required_or_optional_path(
    workspace_root: &Path,
    field: &'static str,
    required: bool,
    relative_path: &str,
) -> Result<Option<PathBuf>, ContentAuthoringError> {
    let path = resolve_workspace_path(workspace_root, field, relative_path)?;
    if path.exists() {
        Ok(Some(path))
    } else if required {
        Err(ContentAuthoringError::MissingContent { field, path })
    } else {
        Ok(None)
    }
}

fn resolve_workspace_path(
    workspace_root: &Path,
    field: &'static str,
    relative_path: &str,
) -> Result<PathBuf, ContentAuthoringError> {
    let path = Path::new(relative_path);
    validate_relative_path(field, path)?;
    Ok(workspace_root.join(path))
}

fn absolute_existing_path(path: &Path) -> Result<PathBuf, ContentAuthoringError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if !absolute.exists() {
        return Err(ContentAuthoringError::MissingContent {
            field: "manifest",
            path: absolute,
        });
    }
    Ok(absolute.canonicalize()?)
}

fn find_workspace_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.join("Cargo.toml").exists() && ancestor.join("crates").exists())
        .map(Path::to_path_buf)
}

fn validate_content_file(path: &Path) -> Result<u64, ContentAuthoringError> {
    let bytes = file_size(path)?;
    if bytes > G16_MAX_CONTENT_FILE_BYTES {
        return Err(ContentAuthoringError::OversizedContent {
            path: path.to_path_buf(),
            bytes,
        });
    }
    Ok(bytes)
}

fn file_size(path: &Path) -> Result<u64, ContentAuthoringError> {
    Ok(fs::metadata(path)?.len())
}

fn require_schema(
    field: &'static str,
    actual_schema: &str,
    expected_schema: &'static str,
    actual_version: u16,
    expected_version: u16,
) -> Result<(), ContentAuthoringError> {
    if actual_schema != expected_schema {
        return Err(ContentAuthoringError::Schema {
            field,
            expected: expected_schema,
            actual: actual_schema.to_string(),
        });
    }
    if actual_version != expected_version {
        return Err(ContentAuthoringError::SchemaVersion {
            field,
            expected: expected_version,
            actual: actual_version,
        });
    }
    Ok(())
}

fn require_nonempty_id(field: &'static str, value: &str) -> Result<(), ContentAuthoringError> {
    if value.trim().is_empty() {
        invalid(field, "value must be nonempty")
    } else {
        reject_engine_local_text(field, value)
    }
}

fn reject_engine_local_text(field: &'static str, value: &str) -> Result<(), ContentAuthoringError> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("entity(")
        || lower.contains("bevy")
        || lower.contains("avian")
        || lower.contains("wgpu")
        || lower.contains("windowhandle")
    {
        invalid(
            field,
            "portable content cannot contain engine-local handle text",
        )
    } else {
        Ok(())
    }
}

fn validate_relative_path(field: &'static str, path: &Path) -> Result<(), ContentAuthoringError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return invalid(field, "path must be nonempty and relative");
    }
    for component in path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return invalid(field, "path must not escape the workspace");
        }
    }
    Ok(())
}

fn validate_vec3(field: &'static str, xyz: [f32; 3]) -> Result<(), ContentAuthoringError> {
    Vec3f::new(xyz[0], xyz[1], xyz[2]).validate()?;
    if xyz.iter().any(|value| value.abs() > 1_000_000.0) {
        invalid(
            field,
            "coordinate magnitude is outside the authoring bounds",
        )
    } else {
        Ok(())
    }
}

fn validate_unitish_nonnegative(
    field: &'static str,
    value: f32,
    min: f32,
    max: f32,
) -> Result<(), ContentAuthoringError> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(())
    } else {
        invalid(field, "value must be finite and within the authoring range")
    }
}

fn invalid<T>(field: &'static str, message: &'static str) -> Result<T, ContentAuthoringError> {
    Err(ContentAuthoringError::InvalidContent { field, message })
}
