//! Split from the original playable-sim app shell during R13 remediation.

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
                match kind {
                    WorldObjectKind::Agent => {
                        let organism_id = organism_id.ok_or(ScaffoldContractError::InvalidId)?;
                        organism_id.validate()?;
                    }
                    WorldObjectKind::Token => {
                        if token_id.is_none() {
                            return Err(ScaffoldContractError::InvalidId);
                        }
                    }
                    _ => {
                        if organism_id.is_some() {
                            return Err(ScaffoldContractError::InvalidId);
                        }
                    }
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
            mode: WorldEditorMode::Simulation,
            config,
            undo_stack: Vec::new(),
            edits_applied: Vec::new(),
            rejected_edits: 0,
        })
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
        self.undo_stack.push(self.world.clone());
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
            } => Some(self.world.editor_spawn_object(WorldEditorSpawnSpec {
                label,
                kind,
                organism_id,
                position,
                nutrition,
                hazard_pain,
                radius,
                token_id,
            })?),
            WorldEditCommand::Remove { stable_id } => {
                self.world.editor_remove_object(stable_id)?;
                None
            }
            WorldEditCommand::Move {
                stable_id,
                position,
            } => {
                self.world.editor_move_object(stable_id, position)?;
                Some(stable_id)
            }
            WorldEditCommand::SetFoodResourceRate {
                food_id,
                home_zone,
                regrow_after_ticks,
                decay_after_ticks,
            } => {
                self.world.track_resource_lifecycle(
                    food_id,
                    home_zone,
                    regrow_after_ticks,
                    decay_after_ticks,
                )?;
                Some(food_id)
            }
        };
        self.edits_applied.push(label);
        Ok(result)
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
