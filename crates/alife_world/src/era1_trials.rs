//! Deterministic, unscored world fixtures for the Era 1 Norn-plus battery.

use alife_core::{
    Confidence, LanguageTokenId, OrganismId, ScaffoldContractError, SpeechActKind,
    SpeechMotorPayload, Tick, UtteranceId, Validate, Vec3f,
};
use serde::{Deserialize, Serialize};

use crate::{
    GroundedPhysicalProperties, HeadlessScenarioBuilder, HeadlessWorld, WorldEditorSpawnSpec,
    WorldObjectKind,
};

pub const ERA1_TRIAL_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const ERA1_WORLD_FAMILY_COUNT: usize = 8;
pub const ERA1_ACQUISITION_END_TICK: u64 = 8;
pub const ERA1_PROBE_START_TICK: u64 = 12;
pub const ERA1_TRIAL_END_TICK: u64 = 20;

const SUBJECT_LABEL: &str = "era1-subject";
const FAMILIAR_LABEL: &str = "era1-familiar";
const NOVEL_LABEL: &str = "era1-novel";
const OBJECT_A_LABEL: &str = "era1-object-a";
const OBJECT_B_LABEL: &str = "era1-object-b";
const CUE_LABEL: &str = "era1-cue";
const HAZARD_A_LABEL: &str = "era1-hazard-a";
const HAZARD_B_LABEL: &str = "era1-hazard-b";
const WALL_A_LABEL: &str = "era1-wall-a";
const WALL_B_LABEL: &str = "era1-wall-b";
const WALL_C_LABEL: &str = "era1-wall-c";
const ACQUISITION_HIDDEN_DISTANCE: f32 = 16.0;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era1WorldFamily {
    ForagingHazardMaze = 1,
    DelayedLocation = 2,
    RewardReversal = 3,
    TransformedObjectsLayout = 4,
    TwoStepAccessProblem = 5,
    FamiliarNovelIndividual = 6,
    PeerDemonstration = 7,
    GroundedVocabulary = 8,
}

impl Era1WorldFamily {
    pub const ALL: [Self; ERA1_WORLD_FAMILY_COUNT] = [
        Self::ForagingHazardMaze,
        Self::DelayedLocation,
        Self::RewardReversal,
        Self::TransformedObjectsLayout,
        Self::TwoStepAccessProblem,
        Self::FamiliarNovelIndividual,
        Self::PeerDemonstration,
        Self::GroundedVocabulary,
    ];
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era1TrialPhase {
    Acquisition = 1,
    Delay = 2,
    Probe = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1WorldTransition {
    pub at_tick: Tick,
    pub from: Era1TrialPhase,
    pub to: Era1TrialPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1TrialManifest {
    pub schema_version: u16,
    pub seed: u64,
    pub family: Era1WorldFamily,
    pub subject: OrganismId,
    pub familiar_peer: OrganismId,
    pub novel_peer: OrganismId,
    pub world_variant_id: u64,
    pub held_out_transform: bool,
    pub starter_token_id: u16,
}

impl Era1TrialManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        seed: u64,
        family: Era1WorldFamily,
        subject: OrganismId,
        familiar_peer: OrganismId,
        novel_peer: OrganismId,
        world_variant_id: u64,
        held_out_transform: bool,
        starter_token_id: u16,
    ) -> Result<Self, ScaffoldContractError> {
        let manifest = Self {
            schema_version: ERA1_TRIAL_MANIFEST_SCHEMA_VERSION,
            seed,
            family,
            subject,
            familiar_peer,
            novel_peer,
            world_variant_id,
            held_out_transform,
            starter_token_id,
        };
        manifest.validate_contract()?;
        Ok(manifest)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ScaffoldContractError> {
        self.validate_contract()?;
        serde_json::to_vec(self).map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)
    }

    pub fn phase_at(&self, tick: Tick) -> Result<Era1TrialPhase, ScaffoldContractError> {
        if tick.raw() > ERA1_TRIAL_END_TICK {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        Ok(if tick.raw() < ERA1_ACQUISITION_END_TICK {
            Era1TrialPhase::Acquisition
        } else if tick.raw() < ERA1_PROBE_START_TICK {
            Era1TrialPhase::Delay
        } else {
            Era1TrialPhase::Probe
        })
    }

    pub const fn transitions(&self) -> [Era1WorldTransition; 2] {
        [
            Era1WorldTransition {
                at_tick: Tick::new(ERA1_ACQUISITION_END_TICK),
                from: Era1TrialPhase::Acquisition,
                to: Era1TrialPhase::Delay,
            },
            Era1WorldTransition {
                at_tick: Tick::new(ERA1_PROBE_START_TICK),
                from: Era1TrialPhase::Delay,
                to: Era1TrialPhase::Probe,
            },
        ]
    }
}

impl Validate for Era1TrialManifest {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.subject.validate()?;
        self.familiar_peer.validate()?;
        self.novel_peer.validate()?;
        if self.schema_version != ERA1_TRIAL_MANIFEST_SCHEMA_VERSION
            || self.seed == 0
            || self.world_variant_id == 0
            || self.starter_token_id == 0
            || LanguageTokenId::new(self.starter_token_id).is_err()
            || self.subject == self.familiar_peer
            || self.subject == self.novel_peer
            || self.familiar_peer == self.novel_peer
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

pub fn build_era1_trial_world(
    manifest: &Era1TrialManifest,
) -> Result<HeadlessWorld, ScaffoldContractError> {
    manifest.validate_contract()?;
    let subject_position = position(manifest, 0.0, 0.0);
    let mut builder = HeadlessScenarioBuilder::new(manifest.seed).agent(
        SUBJECT_LABEL,
        manifest.subject,
        subject_position,
    );

    builder = match manifest.family {
        Era1WorldFamily::ForagingHazardMaze => builder
            .food(
                OBJECT_A_LABEL,
                position(manifest, ACQUISITION_HIDDEN_DISTANCE, 1.0),
                0.7,
            )
            .grounded_physical(OBJECT_A_LABEL, physical(1))
            .food(
                OBJECT_B_LABEL,
                position(manifest, -ACQUISITION_HIDDEN_DISTANCE, 2.0),
                0.45,
            )
            .grounded_physical(OBJECT_B_LABEL, physical(2))
            .hazard(HAZARD_A_LABEL, position(manifest, 1.0, 0.0), 0.8)
            .grounded_physical(HAZARD_A_LABEL, physical(3))
            .hazard(HAZARD_B_LABEL, position(manifest, -2.0, -2.0), 0.55)
            .grounded_physical(HAZARD_B_LABEL, physical(4))
            .obstacle(
                WALL_A_LABEL,
                position(manifest, ACQUISITION_HIDDEN_DISTANCE, -2.0),
                0.45,
            )
            .obstacle(
                WALL_B_LABEL,
                position(manifest, -ACQUISITION_HIDDEN_DISTANCE, 1.0),
                0.45,
            )
            .obstacle(
                WALL_C_LABEL,
                position(manifest, -ACQUISITION_HIDDEN_DISTANCE, -1.0),
                0.45,
            ),
        Era1WorldFamily::DelayedLocation => builder
            .food(OBJECT_A_LABEL, position(manifest, 3.0, 0.0), 0.65)
            .grounded_physical(OBJECT_A_LABEL, physical(1))
            .hazard(OBJECT_B_LABEL, position(manifest, -3.0, 0.0), 0.65)
            .grounded_physical(OBJECT_B_LABEL, physical(2))
            .token(
                CUE_LABEL,
                position(manifest, 1.0, 0.0),
                u32::from(manifest.starter_token_id),
            )
            .grounded_physical(CUE_LABEL, physical(5)),
        Era1WorldFamily::RewardReversal => builder
            .food(OBJECT_A_LABEL, position(manifest, 3.0, 1.0), 0.7)
            .grounded_physical(OBJECT_A_LABEL, physical(1))
            .hazard(OBJECT_B_LABEL, position(manifest, -3.0, 1.0), 0.7)
            .grounded_physical(OBJECT_B_LABEL, physical(2)),
        Era1WorldFamily::TransformedObjectsLayout => builder
            .food(OBJECT_A_LABEL, position(manifest, 2.0, 2.0), 0.7)
            .grounded_physical(OBJECT_A_LABEL, physical(1))
            .hazard(OBJECT_B_LABEL, position(manifest, -2.0, 1.0), 0.7)
            .grounded_physical(OBJECT_B_LABEL, physical(2))
            .obstacle("era1-landmark-a", position(manifest, 0.0, 3.0), 0.5)
            .grounded_physical("era1-landmark-a", physical(3)),
        Era1WorldFamily::TwoStepAccessProblem => builder
            .food(OBJECT_A_LABEL, position(manifest, 4.0, 0.0), 0.8)
            .grounded_physical(OBJECT_A_LABEL, physical(1))
            .obstacle(OBJECT_B_LABEL, position(manifest, 2.0, 0.0), 0.75)
            .grounded_physical(OBJECT_B_LABEL, physical(2))
            .token(
                CUE_LABEL,
                position(manifest, 0.75, 1.0),
                u32::from(manifest.starter_token_id),
            )
            .grounded_physical(CUE_LABEL, physical(3)),
        Era1WorldFamily::FamiliarNovelIndividual => builder
            .social_agent(
                FAMILIAR_LABEL,
                manifest.familiar_peer,
                position(manifest, 2.0, 0.0),
                0.6,
            )
            .grounded_physical(FAMILIAR_LABEL, physical(6)),
        Era1WorldFamily::PeerDemonstration => builder
            .social_agent(
                FAMILIAR_LABEL,
                manifest.familiar_peer,
                position(manifest, 1.0, 0.0),
                0.4,
            )
            .grounded_physical(FAMILIAR_LABEL, physical(6))
            .food(OBJECT_A_LABEL, position(manifest, 2.0, 1.0), 0.7)
            .grounded_physical(OBJECT_A_LABEL, physical(1))
            .hazard(OBJECT_B_LABEL, position(manifest, 2.0, -1.0), 0.7)
            .grounded_physical(OBJECT_B_LABEL, physical(2)),
        Era1WorldFamily::GroundedVocabulary => builder
            .social_agent(
                FAMILIAR_LABEL,
                manifest.familiar_peer,
                position(manifest, 1.0, 0.0),
                0.4,
            )
            .grounded_physical(FAMILIAR_LABEL, physical(6))
            .food(OBJECT_A_LABEL, position(manifest, 2.0, 0.0), 0.7)
            .grounded_physical(OBJECT_A_LABEL, physical(1))
            .token(
                CUE_LABEL,
                position(manifest, 1.5, 0.5),
                u32::from(manifest.starter_token_id),
            )
            .grounded_physical(CUE_LABEL, physical(5)),
    };

    let mut world = builder.build()?;
    if manifest.family == Era1WorldFamily::GroundedVocabulary {
        let payload = SpeechMotorPayload::try_new(
            SpeechActKind::Declare,
            vec![LanguageTokenId::new(manifest.starter_token_id)?],
            Confidence::new(1.0)?,
        )?;
        world.emit_creature_utterance(
            UtteranceId::new(1)?,
            manifest.familiar_peer,
            Some(manifest.subject),
            payload,
        )?;
    }
    Ok(world)
}

pub fn apply_era1_world_transition(
    manifest: &Era1TrialManifest,
    transition: Era1WorldTransition,
    world: &mut HeadlessWorld,
) -> Result<(), ScaffoldContractError> {
    manifest.validate_contract()?;
    if world.seed() != manifest.seed
        || world.tick() != transition.at_tick
        || !manifest.transitions().contains(&transition)
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }

    match (manifest.family, transition.to) {
        (Era1WorldFamily::ForagingHazardMaze, Era1TrialPhase::Delay) => {
            move_label_preserving_physical(world, OBJECT_A_LABEL, position(manifest, 2.0, 1.0))?;
            move_label_preserving_physical(world, OBJECT_B_LABEL, position(manifest, -3.0, 2.0))?;
            move_label_preserving_physical(world, WALL_A_LABEL, position(manifest, 1.0, -2.0))?;
            move_label_preserving_physical(world, WALL_B_LABEL, position(manifest, -1.0, 1.0))?;
            move_label_preserving_physical(world, WALL_C_LABEL, position(manifest, -1.0, -1.0))?;
        }
        (Era1WorldFamily::DelayedLocation, Era1TrialPhase::Delay)
        | (Era1WorldFamily::TwoStepAccessProblem, Era1TrialPhase::Delay) => {
            remove_label(world, CUE_LABEL)?;
        }
        (Era1WorldFamily::PeerDemonstration, Era1TrialPhase::Delay)
        | (Era1WorldFamily::GroundedVocabulary, Era1TrialPhase::Delay) => {
            remove_label_if_present(world, FAMILIAR_LABEL)?;
            if manifest.family == Era1WorldFamily::GroundedVocabulary {
                remove_label(world, CUE_LABEL)?;
            }
        }
        (Era1WorldFamily::RewardReversal, Era1TrialPhase::Probe) => {
            swap_positions(world, OBJECT_A_LABEL, OBJECT_B_LABEL)?;
        }
        (Era1WorldFamily::TransformedObjectsLayout, Era1TrialPhase::Probe) => {
            move_label(world, OBJECT_A_LABEL, position(manifest, -3.0, 2.0))?;
            move_label(world, OBJECT_B_LABEL, position(manifest, 2.0, -2.0))?;
        }
        (Era1WorldFamily::FamiliarNovelIndividual, Era1TrialPhase::Probe) => {
            if world.entity_id(FAMILIAR_LABEL).is_some() {
                move_label(world, FAMILIAR_LABEL, position(manifest, -2.0, 0.0))?;
            }
            let novel = world.editor_spawn_object(WorldEditorSpawnSpec {
                label: NOVEL_LABEL.to_string(),
                kind: WorldObjectKind::Agent,
                organism_id: Some(manifest.novel_peer),
                position: position(manifest, 2.0, 0.0),
                nutrition: 0.0,
                hazard_pain: 0.0,
                radius: 0.5,
                token_id: None,
            })?;
            world.set_grounded_physical_properties(novel, physical(7))?;
        }
        _ => {}
    }
    Ok(())
}

fn remove_label(world: &mut HeadlessWorld, label: &str) -> Result<(), ScaffoldContractError> {
    let id = world
        .entity_id(label)
        .ok_or(ScaffoldContractError::InvalidId)?;
    world.editor_remove_object(id)?;
    Ok(())
}

fn remove_label_if_present(
    world: &mut HeadlessWorld,
    label: &str,
) -> Result<(), ScaffoldContractError> {
    let Some(id) = world.entity_id(label) else {
        return Ok(());
    };
    world.editor_remove_object(id)?;
    Ok(())
}

fn move_label(
    world: &mut HeadlessWorld,
    label: &str,
    destination: Vec3f,
) -> Result<(), ScaffoldContractError> {
    let id = world
        .entity_id(label)
        .ok_or(ScaffoldContractError::InvalidId)?;
    world.editor_move_object(id, destination)
}

fn move_label_preserving_physical(
    world: &mut HeadlessWorld,
    label: &str,
    destination: Vec3f,
) -> Result<(), ScaffoldContractError> {
    let id = world
        .entity_id(label)
        .ok_or(ScaffoldContractError::InvalidId)?;
    let physical = world
        .entity(id)
        .ok_or(ScaffoldContractError::InvalidId)?
        .grounded_physical;
    world.editor_move_object(id, destination)?;
    world.set_grounded_physical_properties(id, physical)
}

fn swap_positions(
    world: &mut HeadlessWorld,
    first_label: &str,
    second_label: &str,
) -> Result<(), ScaffoldContractError> {
    let first = world
        .entity_id(first_label)
        .ok_or(ScaffoldContractError::InvalidId)?;
    let second = world
        .entity_id(second_label)
        .ok_or(ScaffoldContractError::InvalidId)?;
    let first_position = world
        .entity(first)
        .ok_or(ScaffoldContractError::InvalidId)?
        .position;
    let second_position = world
        .entity(second)
        .ok_or(ScaffoldContractError::InvalidId)?
        .position;
    world.editor_move_object(first, second_position)?;
    world.editor_move_object(second, first_position)
}

fn position(manifest: &Era1TrialManifest, x: f32, y: f32) -> Vec3f {
    if manifest.held_out_transform {
        let shift = (manifest.world_variant_id % 3) as f32 * 0.25;
        Vec3f::new(-y + shift, x - shift, 0.0)
    } else {
        Vec3f::new(x, y, 0.0)
    }
}

fn physical(index: u8) -> GroundedPhysicalProperties {
    let tint = f32::from(index) / 8.0;
    GroundedPhysicalProperties {
        velocity: Vec3f::ZERO,
        color: [tint, 1.0 - tint, 0.35],
        material: [0.2 + tint * 0.5, 0.6, 0.3],
        shape: [0.25 + tint * 0.5, 0.45, 0.7 - tint * 0.4],
        chemical: [tint * 2.0 - 1.0, 0.15, -0.2],
        surface_temperature: tint * 1.5 - 0.75,
        terrain: [0.5, 0.25 + tint * 0.5],
    }
}
