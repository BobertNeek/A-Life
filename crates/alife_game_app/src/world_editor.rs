//! Split from the original playable-sim app shell during R13 remediation.

use std::path::Path;

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldEditorMode {
    Simulation,
    EditingPaused,
}

impl WorldEditorMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Simulation => "simulation",
            Self::EditingPaused => "editing-paused",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldEditorConfig {
    pub max_objects: usize,
    pub world_bound: f32,
}

impl Default for WorldEditorConfig {
    fn default() -> Self {
        Self {
            max_objects: G13_EDITOR_MAX_OBJECTS,
            world_bound: 12.0,
        }
    }
}

impl WorldEditorConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.max_objects == 0
            || self.max_objects > G13_EDITOR_MAX_OBJECTS
            || !self.world_bound.is_finite()
            || !(1.0..=512.0).contains(&self.world_bound)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    fn validate_position(&self, position: Vec3f) -> Result<(), ScaffoldContractError> {
        position.validate()?;
        if position.x.abs() > self.world_bound
            || position.y.abs() > self.world_bound
            || position.z.abs() > self.world_bound
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldEditCommand {
    Place {
        label: String,
        kind: WorldObjectKind,
        organism_id: Option<OrganismId>,
        position: Vec3f,
        nutrition: f32,
        hazard_pain: f32,
        radius: f32,
        token_id: Option<u32>,
    },
    Remove {
        stable_id: WorldEntityId,
    },
    Move {
        stable_id: WorldEntityId,
        position: Vec3f,
    },
    SetFoodResourceRate {
        food_id: WorldEntityId,
        home_zone: EcologyZoneId,
        regrow_after_ticks: u32,
        decay_after_ticks: u32,
    },
}

impl WorldEditCommand {
    pub fn place_food(label: impl Into<String>, position: Vec3f, nutrition: f32) -> Self {
        Self::Place {
            label: label.into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position,
            nutrition,
            hazard_pain: 0.0,
            radius: 0.5,
            token_id: None,
        }
    }

    pub fn place_hazard(label: impl Into<String>, position: Vec3f, pain: f32) -> Self {
        Self::Place {
            label: label.into(),
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: pain,
            radius: 0.75,
            token_id: None,
        }
    }

    pub fn place_obstacle(label: impl Into<String>, position: Vec3f, radius: f32) -> Self {
        Self::Place {
            label: label.into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius,
            token_id: None,
        }
    }

    pub fn place_creature(
        label: impl Into<String>,
        organism_id: OrganismId,
        position: Vec3f,
    ) -> Self {
        Self::Place {
            label: label.into(),
            kind: WorldObjectKind::Agent,
            organism_id: Some(organism_id),
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.75,
            token_id: None,
        }
    }

    pub fn validate(&self, config: WorldEditorConfig) -> Result<(), ScaffoldContractError> {
        config.validate()?;
        match self {
            Self::Place {
                label,
                kind,
                organism_id,
                position,
                nutrition,
                hazard_pain,
                radius,
                token_id,
            } => {
                if label.is_empty() || label.len() > 64 {
                    return Err(ScaffoldContractError::InvalidId);
                }
                config.validate_position(*position)?;
                if !radius.is_finite() || !(0.1..=4.0).contains(radius) {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
                for value in [*nutrition, *hazard_pain] {
                    NormalizedScalar::new(value)?;
                }
                if *kind == WorldObjectKind::Agent {
                    let organism_id = organism_id.ok_or(ScaffoldContractError::InvalidId)?;
                    organism_id.validate()?;
                } else if organism_id.is_some() {
                    return Err(ScaffoldContractError::InvalidId);
                }
                if *kind == WorldObjectKind::Token {
                    if token_id.is_none() {
                        return Err(ScaffoldContractError::InvalidId);
                    }
                } else if token_id.is_some() {
                    return Err(ScaffoldContractError::InvalidId);
                }
            }
            Self::Remove { stable_id } => {
                stable_id.validate()?;
            }
            Self::Move {
                stable_id,
                position,
            } => {
                stable_id.validate()?;
                config.validate_position(*position)?;
            }
            Self::SetFoodResourceRate {
                food_id,
                home_zone,
                regrow_after_ticks,
                decay_after_ticks,
            } => {
                food_id.validate()?;
                if home_zone.raw() == 0 || *regrow_after_ticks == 0 || *decay_after_ticks == 0 {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
            }
        }
        Ok(())
    }

    pub const fn kind_label(&self) -> &'static str {
        match self {
            Self::Place { .. } => "place",
            Self::Remove { .. } => "remove",
            Self::Move { .. } => "move",
            Self::SetFoodResourceRate { .. } => "set-resource-rate",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldEditorSession {
    world: HeadlessWorld,
    source_save: Option<PortableSaveFile>,
    source_asset_root: Option<PathBuf>,
    mode: WorldEditorMode,
    config: WorldEditorConfig,
    undo_stack: Vec<HeadlessWorld>,
    edits_applied: Vec<String>,
    rejected_edits: u32,
}

impl WorldEditorSession {
    pub fn new(world: HeadlessWorld, config: WorldEditorConfig) -> Result<Self, GameAppShellError> {
        config.validate()?;
        if world.object_count() > config.max_objects {
            return Err(ScaffoldContractError::ScalarOutOfRange.into());
        }
        Ok(Self {
            world,
            source_save: None,
            source_asset_root: None,
            mode: WorldEditorMode::Simulation,
            config,
            undo_stack: Vec::new(),
            edits_applied: Vec::new(),
            rejected_edits: 0,
        })
    }

    /// Keep exact acquired state and dependency references attached to the
    /// loaded organisms while the editor changes their surrounding world.
    pub fn from_portable_save(
        save: &PortableSaveFile,
        asset_root: impl AsRef<Path>,
        config: WorldEditorConfig,
    ) -> Result<Self, GameAppShellError> {
        save.validate_with_asset_root(asset_root.as_ref())?;
        let mut session = Self::new(save.restore_headless_world()?, config)?;
        session.source_save = Some(save.clone());
        session.source_asset_root = Some(asset_root.as_ref().to_path_buf());
        Ok(session)
    }

    pub const fn mode(&self) -> WorldEditorMode {
        self.mode
    }

    pub const fn world(&self) -> &HeadlessWorld {
        &self.world
    }

    pub fn enter_editor(&mut self) {
        self.mode = WorldEditorMode::EditingPaused;
    }

    pub fn resume_simulation(&mut self) {
        self.mode = WorldEditorMode::Simulation;
    }

    pub fn apply_edit(
        &mut self,
        command: WorldEditCommand,
    ) -> Result<Option<WorldEntityId>, GameAppShellError> {
        if self.mode != WorldEditorMode::EditingPaused {
            self.rejected_edits = self.rejected_edits.saturating_add(1);
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        if let Err(error) = command.validate(self.config) {
            self.rejected_edits = self.rejected_edits.saturating_add(1);
            return Err(error.into());
        }
        if matches!(command, WorldEditCommand::Place { .. })
            && self.world.object_count() >= self.config.max_objects
        {
            self.rejected_edits = self.rejected_edits.saturating_add(1);
            return Err(ScaffoldContractError::ScalarOutOfRange.into());
        }
        let world_before = self.world.clone();
        let label = command.kind_label().to_string();
        let result = match command {
            WorldEditCommand::Place {
                label,
                kind,
                organism_id,
                position,
                nutrition,
                hazard_pain,
                radius,
                token_id,
            } => self
                .world
                .editor_spawn_object(WorldEditorSpawnSpec {
                    label,
                    kind,
                    organism_id,
                    position,
                    nutrition,
                    hazard_pain,
                    radius,
                    token_id,
                })
                .map(Some),
            WorldEditCommand::Remove { stable_id } => {
                self.world.editor_remove_object(stable_id).map(|_| None)
            }
            WorldEditCommand::Move {
                stable_id,
                position,
            } => self
                .world
                .editor_move_object(stable_id, position)
                .map(|_| Some(stable_id)),
            WorldEditCommand::SetFoodResourceRate {
                food_id,
                home_zone,
                regrow_after_ticks,
                decay_after_ticks,
            } => self
                .world
                .track_resource_lifecycle(food_id, home_zone, regrow_after_ticks, decay_after_ticks)
                .map(|_| Some(food_id)),
        };
        match result {
            Ok(result) => {
                self.undo_stack.push(world_before);
                self.edits_applied.push(label);
                Ok(result)
            }
            Err(error) => {
                self.world = world_before;
                self.rejected_edits = self.rejected_edits.saturating_add(1);
                Err(error.into())
            }
        }
    }

    pub fn undo_last(&mut self) -> Result<(), GameAppShellError> {
        if self.mode != WorldEditorMode::EditingPaused {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        self.world = self
            .undo_stack
            .pop()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        self.edits_applied.push("undo".to_string());
        Ok(())
    }

    pub fn save_portable(&self, save_id: &str) -> Result<PortableSaveFile, GameAppShellError> {
        if let Some(source) = &self.source_save {
            let mut save = source.clone();
            for record in self.world.organism_registry().iter() {
                let creature = save
                    .creatures
                    .iter_mut()
                    .find(|creature| creature.organism_id == record.organism_id())
                    .ok_or(PersistenceError::InvalidConfig {
                        field: "creature.organism_id",
                        message: "editor save requires the existing organism's cognitive summary",
                    })?;
                if creature.genome_id != record.genome().id
                    || creature.brain_class.default_class_id()
                        != record.genome().foundation.brain_class_id
                {
                    return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
                }
                // These are read-only projections of organism authority. The
                // saved memory, weights, GPU checkpoint, and dependencies stay
                // attached to the original individual (AOA-PERSIST-001/002).
                creature.development_tick = record.biochemistry().development.last_update_tick;
                creature.mind.tick = record.biochemistry().tick;
                creature.mind.homeostasis = record.biochemistry().homeostasis;
            }
            save.replace_headless_world_snapshot(&self.world)?;
            save.save_id = save_id.to_string();
            if let Some(runtime) = &mut save.gpu_runtime {
                runtime.last_safe_checkpoint.save_id = save.save_id.clone();
            }
            save.validate_with_asset_root(
                self.source_asset_root
                    .as_deref()
                    .ok_or(ScaffoldContractError::MissingPhaseData)?,
            )?;
            return Ok(save);
        }
        if self.world.organism_registry().iter().next().is_some() {
            return Err(PersistenceError::InvalidConfig {
                field: "creature.mind",
                message: "populated editor saves require the source cognitive summaries and assets",
            }
            .into());
        }
        let config =
            RuntimeConfig::deterministic_default(self.world.seed(), BrainScaleTier::Nano512);
        let save = PortableSaveFile::from_headless_world(
            save_id,
            &self.world,
            config,
            AssetManifest::empty(),
            Vec::new(),
        )?;
        Ok(save)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldEditorSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub mode_after_edits: WorldEditorMode,
    pub placed_count: usize,
    pub removed_count: usize,
    pub moved_count: usize,
    pub resource_rate_changes: usize,
    pub invalid_edit_rejected: bool,
    pub undo_available: bool,
    pub stable_ids: Vec<WorldEntityId>,
    pub saved_roundtrip_signature: Vec<String>,
    pub simulation_resumed: bool,
    pub resumed_patch_sealed: bool,
    pub cognition_direct_mutation_count: u32,
    pub edit_log: Vec<String>,
}

impl WorldEditorSmokeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G13_WORLD_EDITOR_SCHEMA
            || self.schema_version != G13_WORLD_EDITOR_SCHEMA_VERSION
            || self.seed == 0
            || self.mode_after_edits != WorldEditorMode::EditingPaused
            || self.placed_count < 4
            || self.removed_count == 0
            || self.moved_count == 0
            || self.resource_rate_changes == 0
            || !self.invalid_edit_rejected
            || !self.undo_available
            || !self.simulation_resumed
            || !self.resumed_patch_sealed
            || self.cognition_direct_mutation_count != 0
            || self.saved_roundtrip_signature.is_empty()
            || self.edit_log.is_empty()
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        for id in &self.stable_ids {
            id.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.mode_after_edits.label(),
            self.placed_count,
            self.removed_count,
            self.moved_count,
            self.resource_rate_changes,
            self.invalid_edit_rejected,
            self.simulation_resumed,
            self.resumed_patch_sealed,
            self.saved_roundtrip_signature.join("|")
        )
    }
}

pub fn run_world_editor_smoke() -> Result<WorldEditorSmokeSummary, GameAppShellError> {
    let seed = 13_013;
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("editor-agent", OrganismId(13_001), Vec3f::ZERO)
        .build()?;
    world.add_terrain_zone(TerrainZone::new(
        EcologyZoneId(13),
        "editor-meadow",
        TerrainZoneKind::Meadow,
        Vec3f::ZERO,
        8.0,
        0.8,
        0.1,
    )?)?;

    let mut session = WorldEditorSession::new(world, WorldEditorConfig::default())?;
    session.enter_editor();

    let food = session
        .apply_edit(WorldEditCommand::place_food(
            "editor-food",
            Vec3f::new(0.9, 0.0, 0.0),
            0.75,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let hazard = session
        .apply_edit(WorldEditCommand::place_hazard(
            "editor-hazard",
            Vec3f::new(3.0, 0.0, 0.0),
            0.35,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let obstacle = session
        .apply_edit(WorldEditCommand::place_obstacle(
            "editor-wall",
            Vec3f::new(2.0, 0.0, 0.0),
            0.8,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let creature = session
        .apply_edit(WorldEditCommand::place_creature(
            "editor-creature",
            OrganismId(13_002),
            Vec3f::new(-1.25, 0.0, 0.0),
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;

    session.apply_edit(WorldEditCommand::Move {
        stable_id: food,
        position: Vec3f::new(1.0, 0.0, 0.0),
    })?;
    session.apply_edit(WorldEditCommand::SetFoodResourceRate {
        food_id: food,
        home_zone: EcologyZoneId(13),
        regrow_after_ticks: 2,
        decay_after_ticks: 4,
    })?;
    session.apply_edit(WorldEditCommand::Remove {
        stable_id: obstacle,
    })?;

    let invalid_edit_rejected = session
        .apply_edit(WorldEditCommand::place_food(
            "out-of-bounds-food",
            Vec3f::new(99.0, 0.0, 0.0),
            0.5,
        ))
        .is_err();
    let mode_after_edits = session.mode();
    let undo_available = !session.undo_stack.is_empty();

    let save = session.save_portable("g13-edited-world")?;
    save.validate_with_asset_root(std::env::temp_dir())?;
    let json = save.to_json_string_pretty()?;
    let loaded = PortableSaveFile::from_json_str(&json)?;
    loaded.validate_with_asset_root(std::env::temp_dir())?;
    let restored = loaded.restore_headless_world()?;
    let saved_roundtrip_signature = restored.stable_signature();

    session.resume_simulation();
    let mut mind = CreatureMind::scaffold(
        OrganismId(13_001),
        BrainScaleTier::Nano512,
        seed,
        Tick::ZERO,
    )?;
    let mut harness = HeadlessBrainHarness::new(session.world().clone());
    let tick = harness.tick_mind(
        &mut mind,
        BrainTickInput::new(
            Tick::ZERO,
            vec![proposal(
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                Some(food),
                None,
                0.9,
                0.95,
                1.0,
            )?],
        )
        .with_pack_experience(true)
        .with_action_duration(DurationTicks::new(1)),
    );

    let summary = WorldEditorSmokeSummary {
        schema: G13_WORLD_EDITOR_SCHEMA,
        schema_version: G13_WORLD_EDITOR_SCHEMA_VERSION,
        seed,
        mode_after_edits,
        placed_count: 4,
        removed_count: 1,
        moved_count: 1,
        resource_rate_changes: 1,
        invalid_edit_rejected,
        undo_available,
        stable_ids: vec![food, hazard, creature],
        saved_roundtrip_signature,
        simulation_resumed: session.mode() == WorldEditorMode::Simulation,
        resumed_patch_sealed: tick.brain.experience_patch.is_some(),
        cognition_direct_mutation_count: 0,
        edit_log: session.edits_applied.clone(),
    };
    summary.validate()?;
    Ok(summary)
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSandboxEditorSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub scenario_id: String,
    pub initial_object_count: usize,
    pub final_object_count: usize,
    pub placed_food: bool,
    pub removed_food: bool,
    pub placed_hazard: bool,
    pub removed_hazard: bool,
    pub placed_obstacle: bool,
    pub removed_obstacle: bool,
    pub edit_mode_required: bool,
    pub saved_roundtrip_signature: Vec<String>,
    pub saved_json_bytes: usize,
    pub output_written: bool,
    pub player_status_lines: Vec<String>,
    pub stable_ids: Vec<WorldEntityId>,
}

impl PlayerSandboxEditorSmokeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA11_PLAYER_SANDBOX_EDITOR_SCHEMA
            || self.schema_version != CA11_PLAYER_SANDBOX_EDITOR_SCHEMA_VERSION
            || self.scenario_id.is_empty()
            || self.final_object_count <= self.initial_object_count
            || !self.placed_food
            || !self.removed_food
            || !self.placed_hazard
            || !self.removed_hazard
            || !self.placed_obstacle
            || !self.removed_obstacle
            || !self.edit_mode_required
            || self.saved_roundtrip_signature.is_empty()
            || self.saved_json_bytes == 0
            || self.player_status_lines.is_empty()
            || self.stable_ids.len() < 3
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        for id in &self.stable_ids {
            id.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:initial={}:final={}:food={}/{}:hazard={}/{}:obstacle={}/{}:pause={}:bytes={}:saved={}",
            self.schema,
            self.schema_version,
            self.scenario_id,
            self.initial_object_count,
            self.final_object_count,
            self.placed_food,
            self.removed_food,
            self.placed_hazard,
            self.removed_hazard,
            self.placed_obstacle,
            self.removed_obstacle,
            self.edit_mode_required,
            self.saved_json_bytes,
            self.saved_roundtrip_signature.join("|")
        )
    }
}

pub fn run_player_sandbox_editor_smoke(
    manifest_path: impl AsRef<Path>,
    scenario_id: Option<&str>,
    output_path: Option<&Path>,
) -> Result<PlayerSandboxEditorSmokeSummary, GameAppShellError> {
    let selection = select_environment_scenario(manifest_path, scenario_id)?;
    let source_save = PortableSaveFile::from_json_file(&selection.launch.save_path)?;
    let mut session = WorldEditorSession::from_portable_save(
        &source_save,
        &selection.launch.asset_root,
        WorldEditorConfig::default(),
    )?;
    let initial_object_count = session.world().object_count();

    let edit_mode_required = session
        .apply_edit(WorldEditCommand::place_food(
            "live-edit-rejected",
            Vec3f::new(0.25, 0.0, 0.0),
            0.25,
        ))
        .is_err();

    session.enter_editor();

    let temp_food = session
        .apply_edit(WorldEditCommand::place_food(
            "sandbox-temp-food",
            Vec3f::new(1.6, 0.0, 0.0),
            0.55,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    session.apply_edit(WorldEditCommand::Remove {
        stable_id: temp_food,
    })?;

    let temp_hazard = session
        .apply_edit(WorldEditCommand::place_hazard(
            "sandbox-temp-hazard",
            Vec3f::new(2.4, 0.0, 0.0),
            0.45,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    session.apply_edit(WorldEditCommand::Remove {
        stable_id: temp_hazard,
    })?;

    let temp_obstacle = session
        .apply_edit(WorldEditCommand::place_obstacle(
            "sandbox-temp-obstacle",
            Vec3f::new(-2.4, 0.0, 0.0),
            0.7,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    session.apply_edit(WorldEditCommand::Remove {
        stable_id: temp_obstacle,
    })?;

    let final_food = session
        .apply_edit(WorldEditCommand::place_food(
            "sandbox-food",
            Vec3f::new(1.2, 0.0, 0.0),
            0.7,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let final_hazard = session
        .apply_edit(WorldEditCommand::place_hazard(
            "sandbox-hazard",
            Vec3f::new(3.2, 0.0, 0.0),
            0.55,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let final_obstacle = session
        .apply_edit(WorldEditCommand::place_obstacle(
            "sandbox-obstacle",
            Vec3f::new(-1.9, 0.0, 0.0),
            0.9,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;

    let edited_save = session.save_portable("ca11-player-sandbox-edited")?;
    let (loaded, saved_json_bytes) = if let Some(path) = output_path {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(PersistenceError::Io)?;
        }
        edited_save.to_json_file(path)?;
        (
            PortableSaveFile::from_json_file(path)?,
            std::fs::read(path)?.len(),
        )
    } else {
        // This serialization verifies an internal roundtrip; it is not an
        // inline export payload. File exports use the file-save API above.
        let json = serde_json::to_string_pretty(&edited_save)?;
        (PortableSaveFile::from_json_str(&json)?, json.len())
    };
    loaded.validate_with_asset_root(&selection.launch.asset_root)?;
    let restored = loaded.restore_headless_world()?;
    let saved_roundtrip_signature = restored.stable_signature();

    let summary = PlayerSandboxEditorSmokeSummary {
        schema: CA11_PLAYER_SANDBOX_EDITOR_SCHEMA,
        schema_version: CA11_PLAYER_SANDBOX_EDITOR_SCHEMA_VERSION,
        scenario_id: selection.entry.id,
        initial_object_count,
        final_object_count: restored.object_count(),
        placed_food: true,
        removed_food: true,
        placed_hazard: true,
        removed_hazard: true,
        placed_obstacle: true,
        removed_obstacle: true,
        edit_mode_required,
        saved_roundtrip_signature,
        saved_json_bytes,
        output_written: output_path.is_some(),
        player_status_lines: vec![
            "Sandbox editor paused simulation before applying edits.".to_string(),
            "Placed and removed food, hazard, and obstacle markers.".to_string(),
            "Edited scenario saved through portable stable-ID save data.".to_string(),
        ],
        stable_ids: vec![final_food, final_hazard, final_obstacle],
    };
    summary.validate()?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn populated_save() -> (PortableSaveFile, PathBuf) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../alife_world/tests/fixtures/production_voxel");
        (crate::tests::fixtures::current_scene_save(&root, 3), root)
    }

    fn checkpoint_metadata(save: &PortableSaveFile) -> alife_world::persistence::GpuBrainSaveState {
        use alife_core::{
            BrainActivityPolicyV1, BrainCapacityClass, MemoryBankConfig, MemorySidecarState,
            PhenotypeHash, SensorProfile, SensorProfileIdentity, SensoryAbiVersion, SleepState,
            TopologicalMapConfig, TopologySidecar,
        };
        use alife_world::persistence::{
            GpuBrainAssetRef, GpuBrainSaveState, GpuSleepAssetState, MemorySidecarSaveState,
            ThrottleReplaySaveState, TopologySidecarSaveSummary,
            GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
        };

        let organism_id = save.creatures[0].organism_id;
        let profile = SensorProfileIdentity {
            profile_id: SensorProfile::GroundedObjectSlotsV1.into(),
            profile_schema_version: 1,
            sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
        };
        // This CPU test exercises preservation of metadata and references; it
        // does not interpret the referenced payload as live neural tensors.
        let entry = &save.assets.entries[0];
        let reference = GpuBrainAssetRef {
            asset_id: entry.asset_id.clone(),
            digest: entry.digest.clone(),
        };
        let memory = MemorySidecarState::new_profiled(
            organism_id,
            profile,
            MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
        )
        .unwrap();
        let topology =
            TopologySidecar::new_profiled(organism_id, profile, TopologicalMapConfig::default())
                .unwrap();
        let historical: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../alife_world/tests/fixtures/p34/tiny_save.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let policy = BrainActivityPolicyV1::production_v1();
        GpuBrainSaveState {
            schema_version: GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
            organism_id,
            phenotype_hash: PhenotypeHash([1, 2, 3, 4]),
            capacity_class_id: BrainCapacityClass::N512_ID,
            sensor_profile: profile,
            immutable_phenotype: reference.clone(),
            phenotype_compiler_inputs: reference.clone(),
            live_structural_topology: Some(reference.clone()),
            legacy_nano512_compatibility_receipt: None,
            active_weight_generation: 7,
            active_weight_bank: 1,
            active_eligibility_bank: 1,
            learning_transaction_generation: 3,
            lifetime_weights: reference.clone(),
            fast_weights: reference.clone(),
            eligibility: reference.clone(),
            replay_journal: reference.clone(),
            replay_journal_generation: 2,
            replay_journal_cursor: 0,
            replay_journal_event_count: 0,
            activation_state: reference.clone(),
            neuron_homeostasis: reference.clone(),
            checkpoint_tick: save.world.tick,
            exact_cognitive_state: Some(reference.clone()),
            last_learning_replay_key: None,
            pending_eligibility: None,
            pending_experience_transaction: None,
            memory: MemorySidecarSaveState::from_sidecar(&memory, reference.clone(), None, None)
                .unwrap(),
            topology: TopologySidecarSaveSummary::from_sidecar(&topology, reference.clone())
                .unwrap(),
            tracked_objects: alife_world::TrackedObjectRegistry::new(save.world.seed, 1_024)
                .unwrap()
                .save_state(organism_id)
                .unwrap(),
            language_grounding: Default::default(),
            life_statistics: None,
            sleep: SleepState::awake_at(save.world.tick),
            sleep_assets: GpuSleepAssetState::default(),
            backend_provenance: serde_json::from_value(
                historical["creatures"][0]["gpu_brain"]["backend_provenance"].clone(),
            )
            .unwrap(),
            runtime_profile_id: 1,
            runtime_profile_digest: [31, 32, 33, 34],
            activity_policy_version: policy.policy_version,
            activity_policy_digest: policy.policy_digest,
            throttle_replay: ThrottleReplaySaveState::bootstrap(reference).unwrap(),
        }
    }

    #[test]
    fn populated_editor_save_preserves_organisms_acquired_state_and_asset_references() {
        let (mut source, asset_root) = populated_save();
        source.creatures[0].weights.lifetime_consolidated_entries = 3;
        source.creatures[0].weights.h_operational_entries = 2;
        source.creatures[0].weights.h_shadow_entries = 1;
        source.creatures[0].learning.last_consolidated_tick = Some(source.world.tick);
        source.creatures[0].gpu_brain = Some(checkpoint_metadata(&source));
        source.validate_with_asset_root(&asset_root).unwrap();
        let mut session = WorldEditorSession::from_portable_save(
            &source,
            &asset_root,
            WorldEditorConfig::default(),
        )
        .unwrap();
        session.enter_editor();
        let added_food = session
            .apply_edit(WorldEditCommand::place_food(
                "preservation-test-food",
                Vec3f::new(1.2, 0.0, 0.0),
                0.7,
            ))
            .unwrap()
            .unwrap();
        let saved = session
            .save_portable("editor-preserved-individuals")
            .unwrap();
        assert_eq!(saved.creatures, source.creatures);
        assert_eq!(saved.world.organism_records, source.world.organism_records);
        assert_eq!(saved.world.habitats, source.world.habitats);
        assert_eq!(saved.world.voxel_backend, source.world.voxel_backend);
        assert_eq!(saved.config, source.config);
        assert_eq!(saved.assets, source.assets);
        assert_eq!(saved.school, source.school);
        assert_eq!(saved.adapter_remap, source.adapter_remap);
        assert_eq!(
            saved.generated_weight_asset_refs,
            source.generated_weight_asset_refs
        );
        assert_eq!(
            saved.etf_prototype_asset_refs,
            source.etf_prototype_asset_refs
        );
        assert_eq!(source.world.objects.len() + 1, saved.world.objects.len());
        assert!(saved
            .world
            .objects
            .iter()
            .any(|object| object.id == added_food));
        assert!(matches!(
            saved.to_json_string_pretty(),
            Err(PersistenceError::HugeInlinePayload { .. })
        ));

        let output = std::env::temp_dir().join(format!(
            "alife-editor-preserved-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        saved.to_json_file(&output).unwrap();
        let loaded = PortableSaveFile::from_json_file(&output).unwrap();
        loaded.validate_with_asset_root(&asset_root).unwrap();
        assert_eq!(loaded, saved);
        let restored = loaded.restore_headless_world().unwrap();
        assert_eq!(restored.object_count(), saved.world.objects.len());
        for record in source.world.organism_records.as_ref().unwrap() {
            assert_eq!(
                restored.organism_registry().get(record.organism_id()),
                Some(record)
            );
        }
        std::fs::remove_file(output).unwrap();
    }

    #[test]
    fn populated_editor_without_source_cognition_rejects_export() {
        let (source, _) = populated_save();
        let session = WorldEditorSession::new(
            source.restore_headless_world().unwrap(),
            WorldEditorConfig::default(),
        )
        .unwrap();
        assert!(matches!(
            session.save_portable("missing-source-cognition"),
            Err(GameAppShellError::Persistence(
                PersistenceError::InvalidConfig {
                    field: "creature.mind",
                    ..
                }
            ))
        ));
    }

    #[test]
    fn synthetic_editor_without_authoritative_organisms_keeps_its_existing_smoke_path() {
        let summary = run_world_editor_smoke().unwrap();
        assert!(summary.simulation_resumed);
        assert!(summary.resumed_patch_sealed);
        assert!(!summary.saved_roundtrip_signature.is_empty());
    }

    #[test]
    fn failed_world_edit_preserves_world_and_undo_history() {
        let world = HeadlessScenarioBuilder::new(73)
            .food("existing-food", Vec3f::ZERO, 0.5)
            .build()
            .unwrap();
        let signature_before = world.stable_signature();
        let mut session = WorldEditorSession::new(world, WorldEditorConfig::default()).unwrap();
        session.enter_editor();

        let result = session.apply_edit(WorldEditCommand::Move {
            stable_id: WorldEntityId(99_999),
            position: Vec3f::new(1.0, 0.0, 0.0),
        });

        assert!(result.is_err());
        assert_eq!(session.world().stable_signature(), signature_before);
        assert!(session.undo_stack.is_empty());
        assert_eq!(session.rejected_edits, 1);
    }

    #[test]
    fn place_rejects_fields_owned_by_another_object_kind() {
        let command = WorldEditCommand::Place {
            label: "malformed-food".to_string(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: Vec3f::ZERO,
            nutrition: 0.5,
            hazard_pain: 0.0,
            radius: 0.5,
            token_id: Some(7),
        };

        assert!(command.validate(WorldEditorConfig::default()).is_err());
    }
}
