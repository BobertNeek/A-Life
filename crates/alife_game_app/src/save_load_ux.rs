//! G15 save/load UX state for P34 portable saves.
//!
//! This module is a product-facing session layer around P34 persistence. It
//! does not define a new save format, and it keeps all save/load data stable-ID
//! based so future Bevy or renderer adapters can remap engine-local handles at
//! the boundary.

use std::collections::BTreeMap;

use alife_core::{ConsolidationState, SleepPhase};
use alife_world::persistence::{PortableAssetDigest, P34_SAVE_FILE_SCHEMA_VERSION};

use crate::prelude::*;
use crate::*;

const ENGINE_LOCAL_TOKENS: [&str; 8] = [
    "bevy::",
    "Entity(",
    "avian::",
    "wgpu::",
    "RendererHandle",
    "WindowHandle",
    "OSWindow",
    "EcsEntity",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveSlotKind {
    Manual,
    Autosave,
}

impl SaveSlotKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Autosave => "autosave",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSlotDescriptor {
    pub slot_id: String,
    pub display_name: String,
    pub kind: SaveSlotKind,
}

impl SaveSlotDescriptor {
    pub fn new(
        slot_id: impl Into<String>,
        display_name: impl Into<String>,
        kind: SaveSlotKind,
    ) -> Result<Self, ScaffoldContractError> {
        let descriptor = Self {
            slot_id: slot_id.into(),
            display_name: display_name.into(),
            kind,
        };
        descriptor.validate()?;
        Ok(descriptor)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.slot_id.is_empty()
            || self.display_name.is_empty()
            || self.slot_id.len() > 48
            || self.display_name.len() > 64
            || self.slot_id.contains("..")
            || self.slot_id.contains('\\')
            || self.slot_id.contains('/')
            || contains_engine_local_token(&self.slot_id)
            || contains_engine_local_token(&self.display_name)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSlotMetadata {
    pub slot_id: String,
    pub display_name: String,
    pub kind: SaveSlotKind,
    pub occupied: bool,
    pub save_id: Option<String>,
    pub deterministic_seed: Option<u64>,
    pub world_tick: Option<Tick>,
    pub object_count: usize,
    pub stable_world_ids: Vec<WorldEntityId>,
    pub schema: Option<String>,
    pub schema_version: Option<u16>,
    pub json_bytes: usize,
    #[serde(default)]
    pub gpu_checkpoint_count: usize,
    #[serde(default)]
    pub gpu_checkpoint_tick: Option<Tick>,
    #[serde(default)]
    pub gpu_sleep_phase: Option<String>,
    #[serde(default)]
    pub gpu_consolidation_state: Option<String>,
}

impl SaveSlotMetadata {
    fn empty(descriptor: &SaveSlotDescriptor) -> Self {
        Self {
            slot_id: descriptor.slot_id.clone(),
            display_name: descriptor.display_name.clone(),
            kind: descriptor.kind,
            occupied: false,
            save_id: None,
            deterministic_seed: None,
            world_tick: None,
            object_count: 0,
            stable_world_ids: Vec::new(),
            schema: None,
            schema_version: None,
            json_bytes: 0,
            gpu_checkpoint_count: 0,
            gpu_checkpoint_tick: None,
            gpu_sleep_phase: None,
            gpu_consolidation_state: None,
        }
    }

    fn from_save(
        descriptor: &SaveSlotDescriptor,
        save: &PortableSaveFile,
        json_bytes: usize,
    ) -> Self {
        let checkpoints = save
            .creatures
            .iter()
            .filter_map(|creature| creature.gpu_brain.as_ref())
            .collect::<Vec<_>>();
        let gpu_checkpoint_tick =
            uniform_checkpoint_value(checkpoints.iter().map(|state| state.checkpoint_tick));
        let gpu_sleep_phase = uniform_checkpoint_value(
            checkpoints
                .iter()
                .map(|state| save_sleep_phase_label(state.sleep.phase).to_string()),
        );
        let gpu_consolidation_state = uniform_checkpoint_value(
            checkpoints
                .iter()
                .map(|state| save_consolidation_label(&state.sleep.consolidation).to_string()),
        );
        Self {
            slot_id: descriptor.slot_id.clone(),
            display_name: descriptor.display_name.clone(),
            kind: descriptor.kind,
            occupied: true,
            save_id: Some(save.save_id.clone()),
            deterministic_seed: Some(save.deterministic_seed),
            world_tick: Some(save.world.tick),
            object_count: save.world.objects.len(),
            stable_world_ids: sorted_save_world_ids(save),
            schema: Some(save.schema.clone()),
            schema_version: Some(save.schema_version),
            json_bytes,
            gpu_checkpoint_count: checkpoints.len(),
            gpu_checkpoint_tick,
            gpu_sleep_phase,
            gpu_consolidation_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveLoadErrorDisplay {
    pub code: String,
    pub message: String,
    pub partial_load_applied: bool,
}

impl SaveLoadErrorDisplay {
    fn overwrite_required() -> Self {
        Self {
            code: "overwrite-confirmation-required".to_string(),
            message: "Slot already contains a save; confirm overwrite before replacing it."
                .to_string(),
            partial_load_applied: false,
        }
    }

    fn from_persistence(error: PersistenceError) -> Self {
        let code = match &error {
            PersistenceError::Schema { .. } => "schema-mismatch",
            PersistenceError::SchemaVersion { .. } => "schema-version",
            PersistenceError::InvalidConfig { .. } => "invalid-config",
            PersistenceError::InvalidAssetManifest { .. } => "invalid-asset-manifest",
            PersistenceError::MissingRequiredAsset { .. } => "missing-required-asset",
            PersistenceError::DigestMismatch { .. } => "digest-mismatch",
            PersistenceError::EngineLocalIdLeak { .. } => "engine-local-id-leak",
            PersistenceError::MissingAssetReference { .. } => "missing-asset-reference",
            PersistenceError::GeneticLayerMutable => "genetic-layer-mutable",
            PersistenceError::MigrationUnsupported { .. } => "migration-unsupported",
            PersistenceError::HugeInlinePayload { .. } => "huge-inline-payload",
            PersistenceError::Contract(_) => "core-contract",
            PersistenceError::Json(_) => "json",
            PersistenceError::Io(_) => "io",
        };
        Self {
            code: code.to_string(),
            message: error.to_string(),
            partial_load_applied: false,
        }
    }

    fn from_shell(message: impl Into<String>) -> Self {
        Self {
            code: "save-load-ux".to_string(),
            message: message.into(),
            partial_load_applied: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveLoadActionResult {
    pub action_label: String,
    pub slot_id: String,
    pub success: bool,
    pub overwrite_confirmation_required: bool,
    pub loaded_save_id: Option<String>,
    pub restored_object_count: usize,
    pub partial_load_applied: bool,
    pub error: Option<SaveLoadErrorDisplay>,
}

impl SaveLoadActionResult {
    fn success(
        action_label: impl Into<String>,
        slot_id: impl Into<String>,
        loaded_save_id: Option<String>,
        restored_object_count: usize,
    ) -> Self {
        Self {
            action_label: action_label.into(),
            slot_id: slot_id.into(),
            success: true,
            overwrite_confirmation_required: false,
            loaded_save_id,
            restored_object_count,
            partial_load_applied: false,
            error: None,
        }
    }

    fn failed(
        action_label: impl Into<String>,
        slot_id: impl Into<String>,
        error: SaveLoadErrorDisplay,
        overwrite_confirmation_required: bool,
    ) -> Self {
        Self {
            action_label: action_label.into(),
            slot_id: slot_id.into(),
            success: false,
            overwrite_confirmation_required,
            loaded_save_id: None,
            restored_object_count: 0,
            partial_load_applied: false,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SaveSlotRecord {
    descriptor: SaveSlotDescriptor,
    metadata: SaveSlotMetadata,
    json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSlotManager {
    capacity: usize,
    slots: BTreeMap<String, SaveSlotRecord>,
    last_loaded_save_id: Option<String>,
    last_error: Option<SaveLoadErrorDisplay>,
}

impl SaveSlotManager {
    pub fn new(capacity: usize) -> Result<Self, ScaffoldContractError> {
        if capacity == 0 || capacity > G15_MAX_SAVE_SLOTS {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self {
            capacity,
            slots: BTreeMap::new(),
            last_loaded_save_id: None,
            last_error: None,
        })
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn last_loaded_save_id(&self) -> Option<&str> {
        self.last_loaded_save_id.as_deref()
    }

    pub fn last_error(&self) -> Option<&SaveLoadErrorDisplay> {
        self.last_error.as_ref()
    }

    pub fn metadata(&self) -> Vec<SaveSlotMetadata> {
        self.slots
            .values()
            .map(|record| record.metadata.clone())
            .collect()
    }

    pub fn save_slot(
        &mut self,
        descriptor: SaveSlotDescriptor,
        save: &PortableSaveFile,
        asset_root: &Path,
        confirm_overwrite: bool,
    ) -> SaveLoadActionResult {
        if let Err(error) = descriptor.validate() {
            let display = SaveLoadErrorDisplay::from_shell(error.to_string());
            self.last_error = Some(display.clone());
            return SaveLoadActionResult::failed("save", descriptor.slot_id, display, false);
        }
        if self.slots.contains_key(&descriptor.slot_id) && !confirm_overwrite {
            let display = SaveLoadErrorDisplay::overwrite_required();
            self.last_error = Some(display.clone());
            return SaveLoadActionResult::failed("save", descriptor.slot_id, display, true);
        }
        if !self.slots.contains_key(&descriptor.slot_id) && self.slots.len() >= self.capacity {
            let display = SaveLoadErrorDisplay::from_shell("Save slot capacity reached.");
            self.last_error = Some(display.clone());
            return SaveLoadActionResult::failed("save", descriptor.slot_id, display, false);
        }
        if let Err(error) = save.validate_with_asset_root(asset_root) {
            let display = SaveLoadErrorDisplay::from_persistence(error);
            self.last_error = Some(display.clone());
            return SaveLoadActionResult::failed("save", descriptor.slot_id, display, false);
        }
        let json = match save.to_json_string_pretty() {
            Ok(json) => json,
            Err(error) => {
                let display = SaveLoadErrorDisplay::from_persistence(error);
                self.last_error = Some(display.clone());
                return SaveLoadActionResult::failed("save", descriptor.slot_id, display, false);
            }
        };
        if contains_engine_local_token(&json) {
            let display =
                SaveLoadErrorDisplay::from_persistence(PersistenceError::EngineLocalIdLeak {
                    field: "save_slot.json",
                    value: "engine-local token found in portable save".to_string(),
                });
            self.last_error = Some(display.clone());
            return SaveLoadActionResult::failed("save", descriptor.slot_id, display, false);
        }
        let metadata = SaveSlotMetadata::from_save(&descriptor, save, json.len());
        let slot_id = descriptor.slot_id.clone();
        self.slots.insert(
            slot_id.clone(),
            SaveSlotRecord {
                descriptor,
                metadata,
                json,
            },
        );
        self.last_error = None;
        SaveLoadActionResult::success("save", slot_id, None, save.world.objects.len())
    }

    pub fn import_raw_slot(
        &mut self,
        descriptor: SaveSlotDescriptor,
        raw_json: impl Into<String>,
        confirm_overwrite: bool,
    ) -> SaveLoadActionResult {
        if let Err(error) = descriptor.validate() {
            let display = SaveLoadErrorDisplay::from_shell(error.to_string());
            self.last_error = Some(display.clone());
            return SaveLoadActionResult::failed("import", descriptor.slot_id, display, false);
        }
        if self.slots.contains_key(&descriptor.slot_id) && !confirm_overwrite {
            let display = SaveLoadErrorDisplay::overwrite_required();
            self.last_error = Some(display.clone());
            return SaveLoadActionResult::failed("import", descriptor.slot_id, display, true);
        }
        if !self.slots.contains_key(&descriptor.slot_id) && self.slots.len() >= self.capacity {
            let display = SaveLoadErrorDisplay::from_shell("Save slot capacity reached.");
            self.last_error = Some(display.clone());
            return SaveLoadActionResult::failed("import", descriptor.slot_id, display, false);
        }
        let json = raw_json.into();
        let metadata = match PortableSaveFile::from_json_str(&json) {
            Ok(save) => SaveSlotMetadata::from_save(&descriptor, &save, json.len()),
            Err(_) => {
                let mut metadata = SaveSlotMetadata::empty(&descriptor);
                metadata.occupied = true;
                metadata.json_bytes = json.len();
                metadata
            }
        };
        let slot_id = descriptor.slot_id.clone();
        self.slots.insert(
            slot_id.clone(),
            SaveSlotRecord {
                descriptor,
                metadata,
                json,
            },
        );
        SaveLoadActionResult::success("import", slot_id, None, 0)
    }

    pub fn load_slot(&mut self, slot_id: &str, asset_root: &Path) -> SaveLoadActionResult {
        let Some(record) = self.slots.get(slot_id) else {
            let display = SaveLoadErrorDisplay::from_shell("Save slot is empty or missing.");
            self.last_error = Some(display.clone());
            return SaveLoadActionResult::failed("load", slot_id, display, false);
        };
        let loaded = PortableSaveFile::from_json_str(&record.json)
            .and_then(|save| {
                save.validate_with_asset_root(asset_root)?;
                let world = save.restore_headless_world()?;
                Ok((save, world.object_count()))
            })
            .map_err(SaveLoadErrorDisplay::from_persistence);
        match loaded {
            Ok((save, object_count)) => {
                self.last_loaded_save_id = Some(save.save_id.clone());
                self.last_error = None;
                SaveLoadActionResult::success("load", slot_id, Some(save.save_id), object_count)
            }
            Err(display) => {
                self.last_error = Some(display.clone());
                SaveLoadActionResult::failed("load", slot_id, display, false)
            }
        }
    }

    pub fn raw_slot_json(&self, slot_id: &str) -> Option<&str> {
        self.slots.get(slot_id).map(|record| record.json.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutosavePolicy {
    pub enabled: bool,
    pub slot_id: &'static str,
    pub every_ticks: DurationTicks,
}

impl AutosavePolicy {
    pub const fn deterministic_default() -> Self {
        Self {
            enabled: true,
            slot_id: "autosave-0",
            every_ticks: DurationTicks(5),
        }
    }

    pub fn should_autosave(self, last_autosave_tick: Option<Tick>, current_tick: Tick) -> bool {
        if !self.enabled {
            return false;
        }
        match last_autosave_tick {
            Some(last) => {
                current_tick.raw().saturating_sub(last.raw()) >= u64::from(self.every_ticks.raw())
            }
            None => true,
        }
    }

    pub fn descriptor(self) -> Result<SaveSlotDescriptor, ScaffoldContractError> {
        SaveSlotDescriptor::new(self.slot_id, "Autosave", SaveSlotKind::Autosave)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigMenuState {
    pub schema_version: u16,
    pub requested_backend: PolicyBackend,
    pub deterministic_seed: u64,
    pub brain_class: BrainScaleTier,
    pub benchmark_population_tier: u16,
    pub school_enabled: bool,
    pub semantic_enabled: bool,
    pub gpu_enabled: bool,
    pub failure_stops_learned_actions: bool,
    pub no_active_readback: bool,
    pub scenario_id: String,
    pub validation_messages: Vec<String>,
}

impl ConfigMenuState {
    pub fn from_config(config: &RuntimeConfig, scenario_id: impl Into<String>) -> Self {
        Self {
            schema_version: config.schema_version,
            requested_backend: config.brain_policy.policy,
            deterministic_seed: config.deterministic_seed,
            brain_class: config.brain_class,
            benchmark_population_tier: config.benchmark_population_tier,
            school_enabled: config.features.school_enabled,
            semantic_enabled: config.features.semantic_adapter_enabled,
            gpu_enabled: config.features.gpu_backend_enabled,
            failure_stops_learned_actions: true,
            no_active_readback: config.gpu_limits.no_active_gameplay_readback,
            scenario_id: scenario_id.into(),
            validation_messages: Vec::new(),
        }
    }

    pub fn validate_config(config: &RuntimeConfig) -> Result<Self, SaveLoadErrorDisplay> {
        config
            .validate()
            .map(|_| Self::from_config(config, "p34-fixture"))
            .map_err(SaveLoadErrorDisplay::from_persistence)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SaveLoadMenuState {
    pub schema: &'static str,
    pub schema_version: u16,
    pub slots: Vec<SaveSlotMetadata>,
    pub selected_slot_id: Option<String>,
    pub manual_save_enabled: bool,
    pub autosave_enabled: bool,
    pub overwrite_confirmation_visible: bool,
    pub stable_id_remap_summary: String,
    pub last_error: Option<SaveLoadErrorDisplay>,
}

impl SaveLoadMenuState {
    fn from_manager(
        manager: &SaveSlotManager,
        selected_slot_id: Option<String>,
        autosave_enabled: bool,
        overwrite_confirmation_visible: bool,
        save: &PortableSaveFile,
    ) -> Self {
        Self {
            schema: G15_SAVE_LOAD_UX_SCHEMA,
            schema_version: G15_SAVE_LOAD_UX_SCHEMA_VERSION,
            slots: manager.metadata(),
            selected_slot_id,
            manual_save_enabled: true,
            autosave_enabled,
            overwrite_confirmation_visible,
            stable_id_remap_summary: stable_id_remap_summary(save),
            last_error: manager.last_error().cloned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SaveLoadUxSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub manual_save_slot: String,
    pub autosave_slot: String,
    pub loaded_save_id: String,
    pub restored_object_count: usize,
    pub stable_world_ids: Vec<WorldEntityId>,
    pub stable_id_remap_preserved: bool,
    pub overwrite_confirmation_visible: bool,
    pub invalid_schema_error: SaveLoadErrorDisplay,
    pub missing_asset_error: SaveLoadErrorDisplay,
    pub digest_error: SaveLoadErrorDisplay,
    pub invalid_config_error: SaveLoadErrorDisplay,
    pub no_partial_load_after_error: bool,
    pub engine_local_token_absent: bool,
    pub deterministic_autosave_due: bool,
    pub config_menu: ConfigMenuState,
    pub menu: SaveLoadMenuState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphicalSaveLoadMenuCommand {
    ToggleMenu,
    ManualSave,
    LoadManualSlot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicalSaveLoadMenuSession {
    pub schema: &'static str,
    pub schema_version: u16,
    menu_open: bool,
    manual_descriptor: SaveSlotDescriptor,
    asset_root: PathBuf,
    source_save: PortableSaveFile,
    manager: SaveSlotManager,
    last_action: Option<SaveLoadActionResult>,
    load_applied_count: usize,
    invalid_error_count: usize,
}

impl GraphicalSaveLoadMenuSession {
    pub fn from_launch(launch: &AppShellLaunchConfig) -> Result<Self, GameAppShellError> {
        let source_save = PortableSaveFile::from_json_file(&launch.save_path)?;
        source_save.validate_with_asset_root(&launch.asset_root)?;
        let mut manager = SaveSlotManager::new(G15_MAX_SAVE_SLOTS)?;
        let manual_descriptor =
            SaveSlotDescriptor::new("slot-0", "Manual Save 1", SaveSlotKind::Manual)?;
        let initial_save = manager.save_slot(
            manual_descriptor.clone(),
            &source_save,
            &launch.asset_root,
            false,
        );
        if !initial_save.success {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA09 initial manual save slot must validate",
            });
        }
        Ok(Self {
            schema: G15_SAVE_LOAD_UX_SCHEMA,
            schema_version: G15_SAVE_LOAD_UX_SCHEMA_VERSION,
            menu_open: false,
            manual_descriptor,
            asset_root: launch.asset_root.clone(),
            source_save,
            manager,
            last_action: Some(initial_save),
            load_applied_count: 0,
            invalid_error_count: 0,
        })
    }

    pub fn apply_command(&mut self, command: GraphicalSaveLoadMenuCommand) -> SaveLoadActionResult {
        match command {
            GraphicalSaveLoadMenuCommand::ToggleMenu => {
                self.menu_open = !self.menu_open;
                SaveLoadActionResult::success(
                    if self.menu_open {
                        "open-menu"
                    } else {
                        "close-menu"
                    },
                    self.manual_descriptor.slot_id.clone(),
                    None,
                    self.source_save.world.objects.len(),
                )
            }
            GraphicalSaveLoadMenuCommand::ManualSave => self.manual_save(),
            GraphicalSaveLoadMenuCommand::LoadManualSlot => self.load_manual_slot(),
        }
    }

    pub fn manual_save(&mut self) -> SaveLoadActionResult {
        let result = self.manager.save_slot(
            self.manual_descriptor.clone(),
            &self.source_save,
            &self.asset_root,
            true,
        );
        self.last_action = Some(result.clone());
        result
    }

    pub fn open(&mut self) {
        self.menu_open = true;
    }

    pub const fn is_open(&self) -> bool {
        self.menu_open
    }

    pub fn load_manual_slot(&mut self) -> SaveLoadActionResult {
        let result = self
            .manager
            .load_slot(&self.manual_descriptor.slot_id, &self.asset_root);
        if result.success {
            self.load_applied_count = self.load_applied_count.saturating_add(1);
        } else {
            self.invalid_error_count = self.invalid_error_count.saturating_add(1);
        }
        self.last_action = Some(result.clone());
        result
    }

    pub fn inject_invalid_schema_and_load(&mut self) -> SaveLoadActionResult {
        let raw = self
            .manager
            .raw_slot_json(&self.manual_descriptor.slot_id)
            .unwrap_or_default()
            .replace(
                &format!("\"schema_version\": {}", P34_SAVE_FILE_SCHEMA_VERSION),
                "\"schema_version\": 999",
            );
        let descriptor =
            match SaveSlotDescriptor::new("slot-invalid", "Invalid Save", SaveSlotKind::Manual) {
                Ok(descriptor) => descriptor,
                Err(error) => {
                    let display = SaveLoadErrorDisplay::from_shell(error.to_string());
                    let result =
                        SaveLoadActionResult::failed("load", "slot-invalid", display, false);
                    self.last_action = Some(result.clone());
                    return result;
                }
            };
        self.manager.import_raw_slot(descriptor.clone(), raw, true);
        let result = self
            .manager
            .load_slot(&descriptor.slot_id, &self.asset_root);
        if !result.success {
            self.invalid_error_count = self.invalid_error_count.saturating_add(1);
        }
        self.last_action = Some(result.clone());
        result
    }

    pub fn menu_state(&self) -> SaveLoadMenuState {
        SaveLoadMenuState::from_manager(
            &self.manager,
            Some(self.manual_descriptor.slot_id.clone()),
            AutosavePolicy::deterministic_default().enabled,
            true,
            &self.source_save,
        )
    }

    pub fn stable_world_ids(&self) -> Vec<WorldEntityId> {
        sorted_save_world_ids(&self.source_save)
    }

    pub fn last_error(&self) -> Option<&SaveLoadErrorDisplay> {
        self.last_action
            .as_ref()
            .and_then(|action| action.error.as_ref())
            .or_else(|| self.manager.last_error())
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != G15_SAVE_LOAD_UX_SCHEMA
            || self.schema_version != G15_SAVE_LOAD_UX_SCHEMA_VERSION
            || self.manual_descriptor.slot_id != "slot-0"
            || self.stable_world_ids().is_empty()
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message:
                    "CA09 graphical save/load session must expose a valid manual stable-ID slot",
            });
        }
        let text = graphical_save_load_menu_text(self);
        if contains_engine_local_token(&text) {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA09 graphical save/load text must not expose engine-local tokens",
            });
        }
        Ok(())
    }
}

fn sorted_save_world_ids(save: &PortableSaveFile) -> Vec<WorldEntityId> {
    let mut ids = save
        .world
        .objects
        .iter()
        .map(|object| object.id)
        .collect::<Vec<_>>();
    ids.sort_by_key(|id| id.raw());
    ids
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GraphicalSaveLoadMenuSmokeSummary {
    pub menu_opened: bool,
    pub manual_save: SaveLoadActionResult,
    pub manual_load: SaveLoadActionResult,
    pub invalid_load: SaveLoadActionResult,
    pub load_applied_count: usize,
    pub invalid_error_count: usize,
    pub stable_world_ids: Vec<WorldEntityId>,
    pub overlay_text: String,
    pub no_partial_load_after_error: bool,
    pub engine_local_token_absent: bool,
}

impl GraphicalSaveLoadMenuSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if !self.menu_opened
            || !self.manual_save.success
            || !self.manual_load.success
            || self.invalid_load.success
            || self.load_applied_count == 0
            || self.invalid_error_count == 0
            || self.stable_world_ids.is_empty()
            || !self.no_partial_load_after_error
            || !self.engine_local_token_absent
            || !self.overlay_text.contains("Save / Load")
            || !self.overlay_text.contains("F5 save")
            || !self.overlay_text.contains("F9 load")
            || !self.overlay_text.contains("Stable IDs")
            || !self.overlay_text.contains("partial_load=false")
            || contains_engine_local_token(&self.overlay_text)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA09 graphical save/load smoke must prove menu, save, load, stable IDs, and no partial load after error",
            });
        }
        Ok(())
    }
}

impl SaveLoadUxSmokeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G15_SAVE_LOAD_UX_SCHEMA
            || self.schema_version != G15_SAVE_LOAD_UX_SCHEMA_VERSION
            || self.manual_save_slot.is_empty()
            || self.autosave_slot.is_empty()
            || self.loaded_save_id.is_empty()
            || self.restored_object_count == 0
            || self.stable_world_ids.is_empty()
            || !self.stable_id_remap_preserved
            || !self.overwrite_confirmation_visible
            || self.invalid_schema_error.code != "schema-version"
            || self.missing_asset_error.code != "missing-required-asset"
            || self.digest_error.code != "digest-mismatch"
            || self.invalid_config_error.code != "invalid-config"
            || !self.no_partial_load_after_error
            || !self.engine_local_token_absent
            || !self.deterministic_autosave_due
            || self.config_menu.deterministic_seed == 0
            || !self.config_menu.failure_stops_learned_actions
            || !self.config_menu.no_active_readback
            || self.menu.schema != G15_SAVE_LOAD_UX_SCHEMA
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        for id in &self.stable_world_ids {
            id.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.manual_save_slot,
            self.autosave_slot,
            self.loaded_save_id,
            self.restored_object_count,
            self.stable_world_ids
                .iter()
                .map(|id| id.raw().to_string())
                .collect::<Vec<_>>()
                .join("|"),
            self.config_menu.requested_backend as u8,
            self.menu.slots.len()
        )
    }
}

pub fn graphical_save_load_menu_text(session: &GraphicalSaveLoadMenuSession) -> String {
    let state = session.menu_state();
    let stable_ids = session
        .stable_world_ids()
        .iter()
        .map(|id| id.raw().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let slot = state
        .slots
        .iter()
        .find(|slot| slot.slot_id == session.manual_descriptor.slot_id)
        .or_else(|| state.slots.first());
    let slot_line = slot.map_or_else(
        || "Slot 1: empty".to_string(),
        |slot| {
            format!(
                "Slot 1: {} occupied={} save={} objects={} tick={} GPU checkpoints={} checkpoint_tick={} recovery={}",
                slot.display_name,
                slot.occupied,
                slot.save_id.as_deref().unwrap_or("empty"),
                slot.object_count,
                slot.world_tick
                    .map(|tick| tick.raw().to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
                slot.gpu_checkpoint_count,
                slot.gpu_checkpoint_tick
                    .map(|tick| tick.raw().to_string())
                    .unwrap_or_else(|| "none".to_string()),
                if slot.gpu_checkpoint_count > 0 {
                    "GPU required"
                } else {
                    "not checkpointed"
                }
            )
        },
    );
    let last_action = session.last_action.as_ref().map_or_else(
        || "Last: ready".to_string(),
        |action| {
            if action.success {
                format!(
                    "Last: {} {} ok objects={} partial_load=false",
                    action.action_label, action.slot_id, action.restored_object_count
                )
            } else {
                let error = action
                    .error
                    .as_ref()
                    .map(|error| format!("{} {}", error.code, error.message))
                    .unwrap_or_else(|| "unknown error".to_string());
                format!(
                    "Last: {} {} failed: {} partial_load={}",
                    action.action_label, action.slot_id, error, action.partial_load_applied
                )
            }
        },
    );
    let error_line = session.last_error().map_or_else(
        || "Errors: none".to_string(),
        |error| {
            format!(
                "Errors: {} partial_load={}",
                error.code, error.partial_load_applied
            )
        },
    );
    if session.menu_open {
        format!(
            concat!(
                "Save / Load\n",
                "{}\n",
                "F5 save manual slot | F9 load manual slot | M close\n",
                "{}\n",
                "Stable IDs: [{}]\n",
                "{}\n",
                "{}\n",
                "Boundary: stable IDs only; no Bevy Entity IDs; no partial load after errors."
            ),
            state.schema, slot_line, stable_ids, last_action, error_line
        )
    } else {
        format!(
            "Save/Load: M menu | F5 save | F9 load | slot={} | Stable IDs [{}] | {} | {}",
            session.manual_descriptor.slot_id, stable_ids, last_action, error_line
        )
    }
}

pub fn player_save_load_menu_text(summary: &SaveLoadUxSmokeSummary) -> String {
    let slot_lines = summary
        .menu
        .slots
        .iter()
        .map(save_slot_menu_line)
        .collect::<Vec<_>>()
        .join("\n");
    let stable_ids = summary
        .stable_world_ids
        .iter()
        .map(|id| id.raw().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        concat!(
            "Save / Load Menu\n",
            "Tabs: New | Save | Load | Settings\n",
            "New: P34 tiny world seed={} scenario={}\n",
            "Stable IDs: [{}]\n",
            "Save: manual slot={} | autosave slot={} due={}\n",
            "Load: {} -> save={} objects={} stable_id_remap={}\n",
            "Overwrite: confirm required={}\n",
            "Cancel: keeps current slot\n",
            "Slots:\n{}\n",
            "Errors: schema={} missing_asset={}\n",
            "        digest={} config={} partial_load_after_error={}\n",
            "Settings: backend={:?} brain={:?}\n",
            "          school={} semantic={} gpu={}\n",
            "          stop_learned_actions={} no_active_readback={}\n",
            "Boundary: stable IDs only; engine-local tokens={}"
        ),
        summary.config_menu.deterministic_seed,
        summary.config_menu.scenario_id,
        stable_ids,
        summary.manual_save_slot,
        summary.autosave_slot,
        summary.deterministic_autosave_due,
        summary.manual_save_slot,
        summary.loaded_save_id,
        summary.restored_object_count,
        summary.stable_id_remap_preserved,
        summary.overwrite_confirmation_visible,
        slot_lines,
        summary.invalid_schema_error.code,
        summary.missing_asset_error.code,
        summary.digest_error.code,
        summary.invalid_config_error.code,
        !summary.no_partial_load_after_error,
        summary.config_menu.requested_backend,
        summary.config_menu.brain_class,
        summary.config_menu.school_enabled,
        summary.config_menu.semantic_enabled,
        summary.config_menu.gpu_enabled,
        summary.config_menu.failure_stops_learned_actions,
        summary.config_menu.no_active_readback,
        !summary.engine_local_token_absent
    )
}

fn save_slot_menu_line(slot: &SaveSlotMetadata) -> String {
    let stable_ids = slot
        .stable_world_ids
        .iter()
        .map(|id| id.raw().to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "- {} ({}) occupied={} save={}\n  tick={} objects={} stable_ids=[{}] schema={} v{} bytes={}\n  GPU checkpoints={} checkpoint_tick={} sleep={} consolidation={} recovery={}",
        slot.display_name,
        slot.kind.label(),
        slot.occupied,
        slot.save_id.as_deref().unwrap_or("empty"),
        slot.world_tick
            .map(|tick| tick.raw().to_string())
            .unwrap_or_else(|| "n/a".to_string()),
        slot.object_count,
        stable_ids,
        slot.schema.as_deref().unwrap_or("none"),
        slot.schema_version
            .map(|version| version.to_string())
            .unwrap_or_else(|| "n/a".to_string()),
        slot.json_bytes,
        slot.gpu_checkpoint_count,
        slot.gpu_checkpoint_tick
            .map(|tick| tick.raw().to_string())
            .unwrap_or_else(|| "none".to_string()),
        slot.gpu_sleep_phase.as_deref().unwrap_or("none"),
        slot.gpu_consolidation_state.as_deref().unwrap_or("none"),
        if slot.gpu_checkpoint_count > 0 {
            "GPU required"
        } else {
            "not checkpointed"
        }
    )
}

fn uniform_checkpoint_value<T: Clone + PartialEq>(
    mut values: impl Iterator<Item = T>,
) -> Option<T> {
    let first = values.next()?;
    values.all(|value| value == first).then_some(first)
}

const fn save_sleep_phase_label(phase: SleepPhase) -> &'static str {
    match phase {
        SleepPhase::Awake => "Awake",
        SleepPhase::EnteringSleep => "Entering sleep",
        SleepPhase::Consolidating => "Consolidating",
        SleepPhase::Waking => "Waking",
        SleepPhase::ForcedRecoverySleep => "Forced recovery sleep",
    }
}

const fn save_consolidation_label(state: &ConsolidationState) -> &'static str {
    match state {
        ConsolidationState::None => "None",
        ConsolidationState::Pending { .. } => "Pending",
        ConsolidationState::Prepared { .. } => "Prepared",
        ConsolidationState::Submitted { .. } => "Submitted",
        ConsolidationState::Completed { .. } => "Completed",
        ConsolidationState::Committed { .. } => "Committed",
    }
}

pub fn run_graphical_save_load_menu_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<GraphicalSaveLoadMenuSmokeSummary, GameAppShellError> {
    let mut session = GraphicalSaveLoadMenuSession::from_launch(launch)?;
    let open = session.apply_command(GraphicalSaveLoadMenuCommand::ToggleMenu);
    let manual_save = session.apply_command(GraphicalSaveLoadMenuCommand::ManualSave);
    let manual_load = session.apply_command(GraphicalSaveLoadMenuCommand::LoadManualSlot);
    let invalid_load = session.inject_invalid_schema_and_load();
    let overlay_text = graphical_save_load_menu_text(&session);
    let summary = GraphicalSaveLoadMenuSmokeSummary {
        menu_opened: open.success && session.is_open(),
        manual_save,
        manual_load,
        invalid_load: invalid_load.clone(),
        load_applied_count: session.load_applied_count,
        invalid_error_count: session.invalid_error_count,
        stable_world_ids: session.stable_world_ids(),
        overlay_text,
        no_partial_load_after_error: !invalid_load.partial_load_applied,
        engine_local_token_absent: !contains_engine_local_token(
            session
                .manager
                .raw_slot_json(&session.manual_descriptor.slot_id)
                .unwrap_or_default(),
        ),
    };
    summary.validate()?;
    Ok(summary)
}

pub fn run_save_load_ux_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<SaveLoadUxSmokeSummary, GameAppShellError> {
    let config = RuntimeConfig::from_json_file(&launch.config_path)?;
    config.validate()?;
    let manifest = AssetManifest::from_json_file(&launch.asset_manifest_path)?;
    manifest.validate_with_root(&launch.asset_root)?;
    let source_save = PortableSaveFile::from_json_file(&launch.save_path)?;
    source_save.validate_with_asset_root(&launch.asset_root)?;
    let source_world = source_save.restore_headless_world()?;

    let manual_save = PortableSaveFile::from_headless_world(
        "g15-manual-slot",
        &source_world,
        config.clone(),
        manifest.clone(),
        source_save.creatures.clone(),
    )?;
    manual_save.validate_with_asset_root(&launch.asset_root)?;

    let mut manager = SaveSlotManager::new(G15_MAX_SAVE_SLOTS)?;
    let manual_descriptor =
        SaveSlotDescriptor::new("slot-0", "Manual Save 1", SaveSlotKind::Manual)?;
    let manual_save_result = manager.save_slot(
        manual_descriptor.clone(),
        &manual_save,
        &launch.asset_root,
        false,
    );
    if !manual_save_result.success {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let overwrite_without_confirmation = manager.save_slot(
        manual_descriptor.clone(),
        &manual_save,
        &launch.asset_root,
        false,
    );
    let overwrite_confirmation_visible =
        overwrite_without_confirmation.overwrite_confirmation_required;
    let confirmed_overwrite = manager.save_slot(
        manual_descriptor.clone(),
        &manual_save,
        &launch.asset_root,
        true,
    );
    if !confirmed_overwrite.success {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }

    let autosave_policy = AutosavePolicy::deterministic_default();
    let deterministic_autosave_due = autosave_policy.should_autosave(None, source_save.world.tick);
    let autosave_descriptor = autosave_policy.descriptor()?;
    let autosave_result = manager.save_slot(
        autosave_descriptor.clone(),
        &manual_save,
        &launch.asset_root,
        false,
    );
    if !autosave_result.success {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }

    let load_result = manager.load_slot(&manual_descriptor.slot_id, &launch.asset_root);
    let loaded_save_id = load_result
        .loaded_save_id
        .clone()
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let stable_world_ids = manual_save
        .world
        .objects
        .iter()
        .map(|object| object.id)
        .collect::<Vec<_>>();
    let manual_json = manager
        .raw_slot_json(&manual_descriptor.slot_id)
        .ok_or(ScaffoldContractError::MissingPhaseData)?
        .to_string();
    let engine_local_token_absent = !contains_engine_local_token(&manual_json);

    let invalid_schema_json = manual_json.replace(
        &format!("\"schema_version\": {}", P34_SAVE_FILE_SCHEMA_VERSION),
        "\"schema_version\": 999",
    );
    let invalid_descriptor =
        SaveSlotDescriptor::new("slot-invalid", "Invalid Save", SaveSlotKind::Manual)?;
    manager.import_raw_slot(invalid_descriptor.clone(), invalid_schema_json, false);
    let invalid_result = manager.load_slot(&invalid_descriptor.slot_id, &launch.asset_root);
    let invalid_schema_error = invalid_result
        .error
        .clone()
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let no_partial_load_after_error = manager.last_loaded_save_id()
        == Some(loaded_save_id.as_str())
        && !invalid_schema_error.partial_load_applied;

    let missing_asset_error = make_missing_asset_error(&manual_save, &launch.asset_root)?;
    let digest_error = make_digest_error(&manual_save, &launch.asset_root)?;
    let invalid_config_error = make_invalid_config_error(&config)?;
    let config_menu = ConfigMenuState::validate_config(&config).map_err(|error| {
        GameAppShellError::VisibleWorldMismatch {
            message: match error.code.as_str() {
                "invalid-config" => "unexpected invalid fixture config",
                _ => "unexpected config menu error",
            },
        }
    })?;
    let menu = SaveLoadMenuState::from_manager(
        &manager,
        Some(manual_descriptor.slot_id.clone()),
        autosave_policy.enabled,
        overwrite_confirmation_visible,
        &manual_save,
    );

    let summary = SaveLoadUxSmokeSummary {
        schema: G15_SAVE_LOAD_UX_SCHEMA,
        schema_version: G15_SAVE_LOAD_UX_SCHEMA_VERSION,
        manual_save_slot: manual_descriptor.slot_id,
        autosave_slot: autosave_descriptor.slot_id,
        loaded_save_id,
        restored_object_count: load_result.restored_object_count,
        stable_world_ids,
        stable_id_remap_preserved: source_save.adapter_remap == manual_save.adapter_remap
            && manual_save.adapter_remap.validate().is_ok(),
        overwrite_confirmation_visible,
        invalid_schema_error,
        missing_asset_error,
        digest_error,
        invalid_config_error,
        no_partial_load_after_error,
        engine_local_token_absent,
        deterministic_autosave_due,
        config_menu,
        menu,
    };
    summary.validate()?;
    Ok(summary)
}

pub fn contains_engine_local_token(text: &str) -> bool {
    ENGINE_LOCAL_TOKENS.iter().any(|token| text.contains(token))
}

fn stable_id_remap_summary(save: &PortableSaveFile) -> String {
    format!(
        "stable_world_ids={} adapter_remap_entries={} engine_local_ids_saved=false",
        save.world.objects.len(),
        save.adapter_remap.entries.len()
    )
}

fn make_missing_asset_error(
    save: &PortableSaveFile,
    asset_root: &Path,
) -> Result<SaveLoadErrorDisplay, GameAppShellError> {
    let mut broken = save.clone();
    let entry = broken.assets.entries.first_mut().ok_or({
        GameAppShellError::VisibleWorldMismatch {
            message: "fixture manifest must contain at least one asset",
        }
    })?;
    entry.relative_path = "missing/g15_required_asset.json".to_string();
    let error = broken
        .validate_with_asset_root(asset_root)
        .expect_err("missing required asset must fail validation");
    Ok(SaveLoadErrorDisplay::from_persistence(error))
}

fn make_digest_error(
    save: &PortableSaveFile,
    asset_root: &Path,
) -> Result<SaveLoadErrorDisplay, GameAppShellError> {
    let mut broken = save.clone();
    let entry = broken.assets.entries.first_mut().ok_or({
        GameAppShellError::VisibleWorldMismatch {
            message: "fixture manifest must contain at least one asset",
        }
    })?;
    entry.digest = PortableAssetDigest("fnv1a64:0000000000000000".to_string());
    let error = broken
        .validate_with_asset_root(asset_root)
        .expect_err("digest mismatch must fail validation");
    Ok(SaveLoadErrorDisplay::from_persistence(error))
}

fn make_invalid_config_error(
    config: &RuntimeConfig,
) -> Result<SaveLoadErrorDisplay, GameAppShellError> {
    let mut invalid = config.clone();
    invalid.deterministic_seed = 0;
    let error = invalid
        .validate()
        .expect_err("zero deterministic seed must fail validation");
    Ok(SaveLoadErrorDisplay::from_persistence(error))
}
