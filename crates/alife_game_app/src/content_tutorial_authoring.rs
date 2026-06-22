//! S09 content/tutorial/world-authoring product smoke.
//!
//! This module validates the tiny committed S09 content pack from the game-app
//! side without making offline authoring tools a runtime dependency.

use std::collections::BTreeSet;

use crate::prelude::*;
use crate::*;

const G16_CONTENT_PACK_SCHEMA: &str = "alife.g16.content_pack.v1";
const G16_CONTENT_PACK_SCHEMA_VERSION: u16 = 1;
const G16_WORLD_PRESET_SCHEMA: &str = "alife.g16.world_preset.v1";
const G16_WORLD_PRESET_SCHEMA_VERSION: u16 = 1;
const G16_LESSON_PACK_SCHEMA: &str = "alife.g16.lesson_pack.v1";
const G16_LESSON_PACK_SCHEMA_VERSION: u16 = 1;
const G16_CREATURE_PRESET_SCHEMA: &str = "alife.g16.creature_preset.v1";
const G16_CREATURE_PRESET_SCHEMA_VERSION: u16 = 1;
const S09_SCENARIO_PACK_SCHEMA: &str = "alife.s09.tutorial_scenario_pack.v1";
const S09_SCENARIO_PACK_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ContentPackManifest {
    schema: String,
    schema_version: u16,
    pack_id: String,
    display_name: String,
    p34_runtime_config: String,
    p34_asset_manifest: String,
    p34_asset_root: String,
    p34_save: String,
    worlds: Vec<ContentEntry>,
    lessons: Vec<ContentEntry>,
    creatures: Vec<ContentEntry>,
    generated_weight_refs: Vec<ContentAssetRef>,
    semantic_assets: Vec<ContentAssetRef>,
    scenario_packs: Vec<ContentEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ContentEntry {
    id: String,
    relative_path: String,
    required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ContentAssetRef {
    asset_id: String,
    relative_path: String,
    required: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct WorldPreset {
    schema: String,
    schema_version: u16,
    world_id: String,
    stable_seed: u64,
    objects: Vec<WorldPresetObject>,
    terrain_zones: Vec<ContentTerrainZone>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct WorldPresetObject {
    stable_id: u64,
    label: String,
    kind: WorldObjectPresetKind,
    organism_id: Option<u64>,
    position: [f32; 3],
    radius: f32,
    nutrition: f32,
    hazard_pain: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum WorldObjectPresetKind {
    Agent,
    Food,
    Hazard,
    Obstacle,
    Token,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct ContentTerrainZone {
    zone_id: u64,
    label: String,
    kind: String,
    center: [f32; 3],
    radius: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct LessonPack {
    schema: String,
    schema_version: u16,
    lesson_pack_id: String,
    steps: Vec<LessonStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct LessonStep {
    lesson_id: u64,
    kind: String,
    channel: String,
    token_id: Option<u64>,
    target_world_entity_id: Option<u64>,
    expected_observation: String,
    perception_only: bool,
    direct_motor_bypass: bool,
    hidden_vector_injection: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct CreaturePreset {
    schema: String,
    schema_version: u16,
    preset_id: String,
    organism_id: u64,
    genome_id: u64,
    brain_class: BrainScaleTier,
    generated_weight_asset_id: String,
    inherited_weight_only: bool,
    lifetime_state_included: bool,
    role_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct TutorialScenarioPack {
    schema: String,
    schema_version: u16,
    scenario_id: String,
    title: String,
    content_pack: String,
    world_preset: String,
    lesson_pack: String,
    creature_preset: String,
    tutorial_script: String,
    recommended_commands: Vec<TutorialScenarioCommand>,
    manual_graphics_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct TutorialScenarioCommand {
    label: String,
    command: String,
    manual: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentPackValidationSummary {
    pub manifest_path: PathBuf,
    pub pack_id: String,
    pub display_name: String,
    pub checked_files: usize,
    pub largest_file_bytes: u64,
    pub world_presets: usize,
    pub lesson_packs: usize,
    pub creature_presets: usize,
    pub generated_weight_refs: usize,
    pub semantic_asset_refs: usize,
    pub scenario_packs: usize,
    pub stable_id_worlds: usize,
    pub perception_only_lesson_steps: usize,
    pub valid_creature_presets: usize,
    pub required_assets_validated: usize,
    pub missing_required_rejected: bool,
    pub tiny_files_under_limit: bool,
    pub has_food: bool,
    pub has_hazard: bool,
    pub has_social_peer: bool,
    pub has_school_token: bool,
    pub has_resource_zone: bool,
}

impl ContentPackValidationSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.pack_id.trim().is_empty()
            || self.checked_files == 0
            || self.largest_file_bytes > S09_MAX_CONTENT_FILE_BYTES
            || self.world_presets == 0
            || self.lesson_packs == 0
            || self.creature_presets == 0
            || self.scenario_packs == 0
            || self.stable_id_worlds != self.world_presets
            || self.perception_only_lesson_steps == 0
            || self.valid_creature_presets != self.creature_presets
            || self.required_assets_validated == 0
            || !self.missing_required_rejected
            || !self.tiny_files_under_limit
            || !self.has_food
            || !self.has_hazard
            || !self.has_social_peer
            || !self.has_school_token
            || !self.has_resource_zone
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.world_presets,
            self.lesson_packs,
            self.creature_presets,
            self.scenario_packs,
            self.perception_only_lesson_steps,
            self.has_hazard,
            self.has_school_token
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TutorialScenarioSummary {
    pub scenario_id: String,
    pub title: String,
    pub scenario_path: PathBuf,
    pub tutorial_script_path: PathBuf,
    pub recommended_commands: Vec<String>,
    pub manual_command_count: usize,
    pub automated_command_count: usize,
    pub graphical_manual_status: &'static str,
}

impl TutorialScenarioSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.scenario_id.trim().is_empty()
            || self.title.trim().is_empty()
            || !self.scenario_path.is_file()
            || !self.tutorial_script_path.is_file()
            || self.automated_command_count == 0
            || self.manual_command_count == 0
            || self
                .recommended_commands
                .iter()
                .any(|command| contains_stale_command(command))
            || self.graphical_manual_status != "manual-optional"
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}",
            self.recommended_commands.len(),
            self.automated_command_count,
            self.manual_command_count
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentTutorialAuthoringSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub content: ContentPackValidationSummary,
    pub tutorial: TutorialScenarioSummary,
    pub onboarding_tutorial_steps: usize,
    pub content_authoring_docs_path: PathBuf,
    pub new_tester_headless_ready: bool,
    pub school_cues_perception_only: bool,
    pub hidden_provider_required: bool,
    pub huge_assets_committed: bool,
}

impl ContentTutorialAuthoringSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != S09_CONTENT_TUTORIAL_SCHEMA
            || self.schema_version != S09_CONTENT_TUTORIAL_SCHEMA_VERSION
            || self.onboarding_tutorial_steps == 0
            || !self.content_authoring_docs_path.is_file()
            || !self.new_tester_headless_ready
            || !self.school_cues_perception_only
            || self.hidden_provider_required
            || self.huge_assets_committed
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.content.validate()?;
        self.tutorial.validate()?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.schema_version,
            self.content.signature_line(),
            self.tutorial.signature_line(),
            self.onboarding_tutorial_steps,
            self.new_tester_headless_ready
        )
    }
}

pub fn s09_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_game_app should live under crates/")
        .to_path_buf()
}

pub fn s09_content_pack_manifest_path() -> PathBuf {
    s09_workspace_root().join("content/fixtures/s09/content_pack_manifest.json")
}

pub fn run_content_authoring_smoke() -> Result<ContentTutorialAuthoringSummary, GameAppShellError> {
    let root = s09_workspace_root();
    let content = validate_s09_content_pack(&root, &s09_content_pack_manifest_path())?;
    let tutorial = load_s09_tutorial_scenario(&root)?;
    let onboarding = load_g20_tutorial_script()?;
    let docs_path = root.join("docs/playable_sim_spec/content_authoring.md");
    let docs = std::fs::read_to_string(&docs_path)?;

    let summary = ContentTutorialAuthoringSummary {
        schema: S09_CONTENT_TUTORIAL_SCHEMA,
        schema_version: S09_CONTENT_TUTORIAL_SCHEMA_VERSION,
        content,
        tutorial,
        onboarding_tutorial_steps: onboarding.steps.len(),
        content_authoring_docs_path: docs_path,
        new_tester_headless_ready: docs.contains("validate-pack")
            && docs.contains("p35_playground")
            && docs.contains("PowerShell"),
        school_cues_perception_only: true,
        hidden_provider_required: false,
        huge_assets_committed: false,
    };
    summary.validate()?;
    Ok(summary)
}

pub fn validate_s09_content_pack(
    root: &Path,
    manifest_path: &Path,
) -> Result<ContentPackValidationSummary, GameAppShellError> {
    let manifest: ContentPackManifest = read_json(manifest_path)?;
    let summary = validate_content_pack_manifest(root, manifest_path, &manifest)?;

    let mut broken = manifest;
    if let Some(entry) = broken.worlds.first_mut() {
        entry.relative_path = "content/fixtures/s09/worlds/missing_world.json".to_string();
    }
    let missing_required_rejected =
        validate_content_pack_manifest(root, manifest_path, &broken).is_err();

    let summary = ContentPackValidationSummary {
        missing_required_rejected,
        ..summary
    };
    summary.validate()?;
    Ok(summary)
}

fn validate_content_pack_manifest(
    root: &Path,
    manifest_path: &Path,
    manifest: &ContentPackManifest,
) -> Result<ContentPackValidationSummary, GameAppShellError> {
    require_schema(
        &manifest.schema,
        manifest.schema_version,
        G16_CONTENT_PACK_SCHEMA,
        G16_CONTENT_PACK_SCHEMA_VERSION,
    )?;
    require_nonempty(&manifest.pack_id)?;
    require_nonempty(&manifest.display_name)?;

    let mut checked_files = 1;
    let mut largest_file_bytes = require_tiny_file(manifest_path)?;

    let config_path = resolve_workspace_path(root, &manifest.p34_runtime_config)?;
    let config = RuntimeConfig::from_json_file(&config_path)?;
    config.validate()?;
    checked_files += 1;
    largest_file_bytes = largest_file_bytes.max(require_tiny_file(&config_path)?);

    let asset_manifest_path = resolve_workspace_path(root, &manifest.p34_asset_manifest)?;
    let asset_manifest = AssetManifest::from_json_file(&asset_manifest_path)?;
    let asset_root = resolve_workspace_path(root, &manifest.p34_asset_root)?;
    asset_manifest.validate_with_root(&asset_root)?;
    checked_files += 1;
    largest_file_bytes = largest_file_bytes.max(require_tiny_file(&asset_manifest_path)?);

    let save_path = resolve_workspace_path(root, &manifest.p34_save)?;
    let save = PortableSaveFile::from_json_file(&save_path)?;
    save.validate_with_asset_root(&asset_root)?;
    checked_files += 1;
    largest_file_bytes = largest_file_bytes.max(require_tiny_file(&save_path)?);

    let mut ids = BTreeSet::new();
    let mut stable_id_worlds = 0;
    let mut has_food = false;
    let mut has_hazard = false;
    let mut has_social_peer = false;
    let mut has_school_token = false;
    let mut has_resource_zone = false;
    for entry in &manifest.worlds {
        validate_entry(entry, &mut ids)?;
        let path = required_content_path(root, entry)?;
        checked_files += 1;
        largest_file_bytes = largest_file_bytes.max(require_tiny_file(&path)?);
        let world = validate_world_preset_file(&path)?;
        stable_id_worlds += 1;
        has_food |= world.has_food;
        has_hazard |= world.has_hazard;
        has_social_peer |= world.agent_count >= 2;
        has_school_token |= world.has_token;
        has_resource_zone |= world.has_resource_zone;
    }

    let mut perception_only_lesson_steps = 0;
    for entry in &manifest.lessons {
        validate_entry(entry, &mut ids)?;
        let path = required_content_path(root, entry)?;
        checked_files += 1;
        largest_file_bytes = largest_file_bytes.max(require_tiny_file(&path)?);
        perception_only_lesson_steps += validate_lesson_pack_file(&path)?;
    }

    let mut valid_creature_presets = 0;
    for entry in &manifest.creatures {
        validate_entry(entry, &mut ids)?;
        let path = required_content_path(root, entry)?;
        checked_files += 1;
        largest_file_bytes = largest_file_bytes.max(require_tiny_file(&path)?);
        validate_creature_preset_file(&path, &asset_manifest)?;
        valid_creature_presets += 1;
    }

    let mut required_assets_validated = 0;
    for asset in &manifest.generated_weight_refs {
        validate_asset_ref(asset, &asset_manifest)?;
        if asset.required {
            required_assets_validated += 1;
        }
        let path = required_asset_path(root, asset)?;
        checked_files += 1;
        largest_file_bytes = largest_file_bytes.max(require_tiny_file(&path)?);
    }
    for asset in &manifest.semantic_assets {
        validate_asset_ref(asset, &asset_manifest)?;
        if let Some(path) = optional_asset_path(root, asset)? {
            checked_files += 1;
            largest_file_bytes = largest_file_bytes.max(require_tiny_file(&path)?);
        }
    }

    for entry in &manifest.scenario_packs {
        validate_entry(entry, &mut ids)?;
        let path = required_content_path(root, entry)?;
        checked_files += 1;
        largest_file_bytes = largest_file_bytes.max(require_tiny_file(&path)?);
        validate_tutorial_scenario_pack(root, &path)?;
    }

    Ok(ContentPackValidationSummary {
        manifest_path: manifest_path.to_path_buf(),
        pack_id: manifest.pack_id.clone(),
        display_name: manifest.display_name.clone(),
        checked_files,
        largest_file_bytes,
        world_presets: manifest.worlds.len(),
        lesson_packs: manifest.lessons.len(),
        creature_presets: manifest.creatures.len(),
        generated_weight_refs: manifest.generated_weight_refs.len(),
        semantic_asset_refs: manifest.semantic_assets.len(),
        scenario_packs: manifest.scenario_packs.len(),
        stable_id_worlds,
        perception_only_lesson_steps,
        valid_creature_presets,
        required_assets_validated,
        missing_required_rejected: false,
        tiny_files_under_limit: largest_file_bytes <= S09_MAX_CONTENT_FILE_BYTES,
        has_food,
        has_hazard,
        has_social_peer,
        has_school_token,
        has_resource_zone,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WorldFeatureSummary {
    agent_count: usize,
    has_food: bool,
    has_hazard: bool,
    has_token: bool,
    has_resource_zone: bool,
}

fn validate_world_preset_file(path: &Path) -> Result<WorldFeatureSummary, GameAppShellError> {
    let world: WorldPreset = read_json(path)?;
    require_schema(
        &world.schema,
        world.schema_version,
        G16_WORLD_PRESET_SCHEMA,
        G16_WORLD_PRESET_SCHEMA_VERSION,
    )?;
    require_nonempty(&world.world_id)?;
    if world.stable_seed == 0 || world.objects.is_empty() {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let mut ids = BTreeSet::new();
    let mut agent_count = 0;
    let mut has_food = false;
    let mut has_hazard = false;
    let mut has_token = false;
    for object in &world.objects {
        WorldEntityId(object.stable_id).validate()?;
        if !ids.insert(object.stable_id) {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        require_nonempty(&object.label)?;
        reject_engine_local_text(&object.label)?;
        if let Some(organism_id) = object.organism_id {
            OrganismId(organism_id).validate()?;
        }
        Vec3f::new(object.position[0], object.position[1], object.position[2]).validate()?;
        validate_range(object.radius, 0.01, 128.0)?;
        validate_range(object.nutrition, 0.0, 1.0)?;
        validate_range(object.hazard_pain, 0.0, 1.0)?;
        match object.kind {
            WorldObjectPresetKind::Agent => agent_count += 1,
            WorldObjectPresetKind::Food => has_food = true,
            WorldObjectPresetKind::Hazard => has_hazard = true,
            WorldObjectPresetKind::Obstacle => {}
            WorldObjectPresetKind::Token => has_token = true,
        }
    }
    let mut has_resource_zone = false;
    for zone in &world.terrain_zones {
        if zone.zone_id == 0 {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        require_nonempty(&zone.label)?;
        require_nonempty(&zone.kind)?;
        reject_engine_local_text(&zone.label)?;
        Vec3f::new(zone.center[0], zone.center[1], zone.center[2]).validate()?;
        validate_range(zone.radius, 0.01, 1024.0)?;
        has_resource_zone |= zone.kind.contains("resource");
    }
    Ok(WorldFeatureSummary {
        agent_count,
        has_food,
        has_hazard,
        has_token,
        has_resource_zone,
    })
}

fn validate_lesson_pack_file(path: &Path) -> Result<usize, GameAppShellError> {
    let lesson: LessonPack = read_json(path)?;
    require_schema(
        &lesson.schema,
        lesson.schema_version,
        G16_LESSON_PACK_SCHEMA,
        G16_LESSON_PACK_SCHEMA_VERSION,
    )?;
    require_nonempty(&lesson.lesson_pack_id)?;
    if lesson.steps.is_empty() {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let mut ids = BTreeSet::new();
    for step in &lesson.steps {
        if step.lesson_id == 0 || !ids.insert(step.lesson_id) {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        require_nonempty(&step.kind)?;
        require_nonempty(&step.channel)?;
        require_nonempty(&step.expected_observation)?;
        if let Some(token_id) = step.token_id {
            if token_id == 0 {
                return Err(ScaffoldContractError::MissingPhaseData.into());
            }
        }
        if let Some(target) = step.target_world_entity_id {
            WorldEntityId(target).validate()?;
        }
        if !step.perception_only || step.direct_motor_bypass || step.hidden_vector_injection {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
    }
    Ok(lesson.steps.len())
}

fn validate_creature_preset_file(
    path: &Path,
    asset_manifest: &AssetManifest,
) -> Result<(), GameAppShellError> {
    let creature: CreaturePreset = read_json(path)?;
    require_schema(
        &creature.schema,
        creature.schema_version,
        G16_CREATURE_PRESET_SCHEMA,
        G16_CREATURE_PRESET_SCHEMA_VERSION,
    )?;
    require_nonempty(&creature.preset_id)?;
    OrganismId(creature.organism_id).validate()?;
    GenomeId(creature.genome_id).validate()?;
    alife_core::BrainClassSpec::for_tier(creature.brain_class).validate()?;
    if !creature.inherited_weight_only
        || creature.lifetime_state_included
        || creature.role_tags.is_empty()
        || !asset_manifest.contains_asset(&creature.generated_weight_asset_id)
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    for tag in &creature.role_tags {
        require_nonempty(tag)?;
        reject_engine_local_text(tag)?;
    }
    Ok(())
}

fn load_s09_tutorial_scenario(root: &Path) -> Result<TutorialScenarioSummary, GameAppShellError> {
    let scenario_path =
        root.join("content/fixtures/s09/scenarios/first_run_tutorial_scenario.json");
    validate_tutorial_scenario_pack(root, &scenario_path)
}

fn validate_tutorial_scenario_pack(
    root: &Path,
    scenario_path: &Path,
) -> Result<TutorialScenarioSummary, GameAppShellError> {
    let scenario: TutorialScenarioPack = read_json(scenario_path)?;
    require_schema(
        &scenario.schema,
        scenario.schema_version,
        S09_SCENARIO_PACK_SCHEMA,
        S09_SCENARIO_PACK_SCHEMA_VERSION,
    )?;
    require_nonempty(&scenario.scenario_id)?;
    require_nonempty(&scenario.title)?;
    let referenced_paths = [
        &scenario.content_pack,
        &scenario.world_preset,
        &scenario.lesson_pack,
        &scenario.creature_preset,
        &scenario.tutorial_script,
    ];
    for relative in referenced_paths {
        if !resolve_workspace_path(root, relative)?.is_file() {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
    }
    let mut automated = 0;
    let mut manual = 0;
    let mut commands = Vec::new();
    for command in &scenario.recommended_commands {
        require_nonempty(&command.label)?;
        require_nonempty(&command.command)?;
        if contains_stale_command(&command.command) {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        if command.manual {
            manual += 1;
        } else {
            automated += 1;
        }
        commands.push(command.command.clone());
    }
    if scenario.manual_graphics_note.trim().is_empty() || automated == 0 || manual == 0 {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let tutorial_script_path = resolve_workspace_path(root, &scenario.tutorial_script)?;
    let summary = TutorialScenarioSummary {
        scenario_id: scenario.scenario_id,
        title: scenario.title,
        scenario_path: scenario_path.to_path_buf(),
        tutorial_script_path,
        recommended_commands: commands,
        manual_command_count: manual,
        automated_command_count: automated,
        graphical_manual_status: "manual-optional",
    };
    summary.validate()?;
    Ok(summary)
}

fn validate_entry(
    entry: &ContentEntry,
    seen_ids: &mut BTreeSet<String>,
) -> Result<(), GameAppShellError> {
    require_nonempty(&entry.id)?;
    validate_relative_path(&entry.relative_path)?;
    if !seen_ids.insert(entry.id.clone()) {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn validate_asset_ref(
    asset: &ContentAssetRef,
    asset_manifest: &AssetManifest,
) -> Result<(), GameAppShellError> {
    require_nonempty(&asset.asset_id)?;
    validate_relative_path(&asset.relative_path)?;
    if asset.required && !asset_manifest.contains_asset(&asset.asset_id) {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn required_content_path(root: &Path, entry: &ContentEntry) -> Result<PathBuf, GameAppShellError> {
    let path = resolve_workspace_path(root, &entry.relative_path)?;
    if path.exists() {
        Ok(path)
    } else if entry.required {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, path.display().to_string()).into())
    } else {
        Err(ScaffoldContractError::MissingPhaseData.into())
    }
}

fn required_asset_path(root: &Path, asset: &ContentAssetRef) -> Result<PathBuf, GameAppShellError> {
    let path = resolve_workspace_path(root, &asset.relative_path)?;
    if path.exists() && asset.required {
        Ok(path)
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, path.display().to_string()).into())
    }
}

fn optional_asset_path(
    root: &Path,
    asset: &ContentAssetRef,
) -> Result<Option<PathBuf>, GameAppShellError> {
    let path = resolve_workspace_path(root, &asset.relative_path)?;
    if path.exists() {
        Ok(Some(path))
    } else if asset.required {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, path.display().to_string()).into())
    } else {
        Ok(None)
    }
}

fn resolve_workspace_path(root: &Path, relative: &str) -> Result<PathBuf, GameAppShellError> {
    validate_relative_path(relative)?;
    Ok(root.join(relative))
}

fn validate_relative_path(relative: &str) -> Result<(), GameAppShellError> {
    if relative.trim().is_empty()
        || relative.contains("..")
        || relative.contains(':')
        || relative.starts_with('/')
        || relative.starts_with('\\')
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn require_schema(
    actual_schema: &str,
    actual_version: u16,
    expected_schema: &str,
    expected_version: u16,
) -> Result<(), GameAppShellError> {
    if actual_schema != expected_schema || actual_version != expected_version {
        Err(ScaffoldContractError::MissingPhaseData.into())
    } else {
        Ok(())
    }
}

fn require_nonempty(value: &str) -> Result<(), GameAppShellError> {
    if value.trim().is_empty() || contains_stale_command(value) {
        Err(ScaffoldContractError::MissingPhaseData.into())
    } else {
        reject_engine_local_text(value)
    }
}

fn reject_engine_local_text(value: &str) -> Result<(), GameAppShellError> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("entity(")
        || lower.contains("bevy")
        || lower.contains("avian")
        || lower.contains("wgpu")
        || lower.contains("windowhandle")
    {
        Err(ScaffoldContractError::MissingPhaseData.into())
    } else {
        Ok(())
    }
}

fn contains_stale_command(value: &str) -> bool {
    value.contains("bash scripts/check.sh")
        || value.contains("gpu-report")
        || value.contains("ALIFE_GPU_BACKEND")
}

fn validate_range(value: f32, min: f32, max: f32) -> Result<(), GameAppShellError> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::MissingPhaseData.into())
    }
}

fn require_tiny_file(path: &Path) -> Result<u64, GameAppShellError> {
    let bytes = std::fs::metadata(path)?.len();
    if bytes > S09_MAX_CONTENT_FILE_BYTES {
        Err(ScaffoldContractError::MissingPhaseData.into())
    } else {
        Ok(bytes)
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, GameAppShellError> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}
