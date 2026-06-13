//! v0 scaffold: explicit Bevy-world sensory conversion into core snapshots.

use alife_core::{
    AffordanceBits, Confidence, ContextStreams, HeardToken, LanguageContextSnapshot,
    NormalizedScalar, OrganismId, ReferenceSensoryAdapter, ReferenceSensoryRequest,
    ScaffoldContractError, SensoryChannels, SensorySnapshot, SignedValence, SocialAgentSnapshot,
    SocialProximityEntry, Tick, Validate, Vec3f, WorldEntityId, MAX_HEARD_TOKENS,
    MAX_SOCIAL_AGENTS, SENSORY_AUDITORY_CHANNEL_COUNT, SENSORY_SMELL_CHANNEL_COUNT,
    SENSORY_TACTILE_CHANNEL_COUNT, SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT,
};
use bevy::prelude::{Entity, Vec3};

use crate::math::bevy_vec3_to_core;

pub const DEFAULT_VISION_RADIUS: f32 = 8.0;
pub const DEFAULT_HEARING_RADIUS: f32 = 6.0;
const CONTACT_RADIUS: f32 = 0.75;
const MAX_VISIBLE_ENTITIES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObservedBevyEntity {
    pub entity: Entity,
    pub world_id: WorldEntityId,
    pub organism_id: Option<OrganismId>,
    pub position: Vec3,
    pub affordances: AffordanceBits,
    pub nutrition: f32,
    pub hazard_pain: f32,
    pub token_id: Option<u32>,
    pub vision_radius_meters: f32,
    pub hearing_radius_meters: f32,
}

impl ObservedBevyEntity {
    pub const fn new(
        entity: Entity,
        world_id: WorldEntityId,
        position: Vec3,
        affordances: AffordanceBits,
    ) -> Self {
        Self {
            entity,
            world_id,
            organism_id: None,
            position,
            affordances,
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: None,
            vision_radius_meters: DEFAULT_VISION_RADIUS,
            hearing_radius_meters: DEFAULT_HEARING_RADIUS,
        }
    }

    pub const fn with_organism(mut self, organism_id: OrganismId) -> Self {
        self.organism_id = Some(organism_id);
        self
    }

    pub const fn with_nutrition(mut self, nutrition: f32) -> Self {
        self.nutrition = nutrition;
        self
    }

    pub const fn with_hazard_pain(mut self, hazard_pain: f32) -> Self {
        self.hazard_pain = hazard_pain;
        self
    }

    pub const fn with_token(mut self, token_id: u32) -> Self {
        self.token_id = Some(token_id);
        self
    }
}

pub fn gather_sensory_from_observed(
    organism_id: OrganismId,
    tick: Tick,
    observer_world_id: WorldEntityId,
    observer_position: Vec3,
    observed: &[ObservedBevyEntity],
) -> Result<SensorySnapshot, ScaffoldContractError> {
    organism_id.validate()?;
    observer_world_id.validate()?;
    let observer_core_position = bevy_vec3_to_core(observer_position)?;

    let mut visual = [0.0_f32; SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT];
    let mut auditory = [0.0_f32; SENSORY_AUDITORY_CHANNEL_COUNT];
    let mut smell = [0.0_f32; SENSORY_SMELL_CHANNEL_COUNT];
    let mut tactile = [0.0_f32; SENSORY_TACTILE_CHANNEL_COUNT];
    let mut affordances = AffordanceBits::NONE;
    let mut pain = 0.0_f32;
    let mut vocal_tokens = [None; MAX_HEARD_TOKENS];
    let mut social_proximity = [None; MAX_SOCIAL_AGENTS];
    let mut social_agents = [None; MAX_SOCIAL_AGENTS];
    let mut heard_index = 0;
    let mut social_index = 0;
    let mut visible_count = 0_usize;

    for entity in observed {
        entity.world_id.validate()?;
        bevy_vec3_to_core(entity.position)?;
        if entity.world_id == observer_world_id {
            continue;
        }

        let delta = entity.position - observer_position;
        let distance = delta.length();
        let visual_salience = proximity_salience(distance, entity.vision_radius_meters);
        let audible_salience = proximity_salience(distance, entity.hearing_radius_meters);
        if visual_salience == 0.0 && audible_salience == 0.0 {
            continue;
        }
        visible_count = visible_count.saturating_add(1);
        affordances |= entity.affordances;

        if entity.affordances.contains(AffordanceBits::FOOD) {
            visual[0] = visual[0].max(visual_salience);
            smell[0] = smell[0].max((visual_salience * entity.nutrition.max(0.1)).clamp(0.0, 1.0));
        }
        if entity.affordances.contains(AffordanceBits::HAZARD) {
            visual[1] = visual[1].max(visual_salience);
            smell[1] = smell[1].max(visual_salience);
            pain = pain.max(
                entity.hazard_pain.clamp(0.0, 1.0)
                    * proximity_salience(distance, CONTACT_RADIUS * 2.0),
            );
        }
        if entity.affordances.contains(AffordanceBits::RESOURCE) {
            visual[2] = visual[2].max(visual_salience);
        }
        if entity.affordances.contains(AffordanceBits::SOCIAL_AGENT) {
            visual[3] = visual[3].max(visual_salience);
            if let Some(agent_id) = entity.organism_id {
                agent_id.validate()?;
                if social_index < MAX_SOCIAL_AGENTS {
                    social_proximity[social_index] = Some(SocialProximityEntry {
                        agent_id,
                        proximity: NormalizedScalar::new(visual_salience)?,
                        confidence: Confidence::new(0.8)?,
                    });
                    social_agents[social_index] = Some(SocialAgentSnapshot {
                        agent_id,
                        body_entity: Some(entity.world_id),
                        relative_position: bevy_vec3_to_core(delta)?,
                        gaze_direction: Vec3f::new(0.0, 1.0, 0.0),
                        orientation_forward: Vec3f::new(0.0, 1.0, 0.0),
                        affinity: SignedValence::new(0.0)?,
                        proximity: NormalizedScalar::new(visual_salience)?,
                    });
                    social_index += 1;
                }
            }
        }
        if entity.affordances.contains(AffordanceBits::TOOL) {
            visual[6] = visual[6].max(visual_salience);
        }
        if entity
            .affordances
            .contains(AffordanceBits::GLYPH_OR_WRITING)
        {
            visual[7] = visual[7].max(visual_salience);
        }
        if entity.affordances.contains(AffordanceBits::TEACHER_OBJECT) {
            visual[8] = visual[8].max(visual_salience);
        }
        if distance <= CONTACT_RADIUS {
            tactile[1] = 1.0;
        }
        if let Some(token_id) = entity.token_id {
            if heard_index < MAX_HEARD_TOKENS && audible_salience > 0.0 {
                auditory[0] = auditory[0].max(audible_salience);
                vocal_tokens[heard_index] = Some(HeardToken {
                    speaker_id: entity.organism_id,
                    source_entity: Some(entity.world_id),
                    token_id,
                    source_position: bevy_vec3_to_core(entity.position)?,
                    confidence: Confidence::new(audible_salience.max(0.1))?,
                    teacher_channel: None,
                });
                heard_index += 1;
            }
        }
    }

    let channels = SensoryChannels::try_from_groups(
        visual,
        auditory,
        smell,
        tactile,
        NormalizedScalar::new(pain.clamp(0.0, 1.0))?,
        NormalizedScalar::new(
            (visible_count as f32 / MAX_VISIBLE_ENTITIES as f32).clamp(0.0, 1.0),
        )?,
        affordances,
    )?;
    let context_streams = ContextStreams {
        vocal_tokens,
        social_proximity,
        ambient_light: NormalizedScalar::new(0.8)?,
        ..ContextStreams::default()
    };
    context_streams.validate_contract()?;

    let mut snapshot = SensorySnapshot::new(
        organism_id,
        tick,
        observer_core_position,
        channels,
        context_streams,
    )?;
    snapshot.language_context = LanguageContextSnapshot {
        heard_tokens: vocal_tokens,
        word_confidence: Confidence::new(if heard_index > 0 { 0.8 } else { 0.0 })?,
        ..LanguageContextSnapshot::default()
    };
    snapshot.social_context.nearest_agents = social_agents;
    snapshot.validate_contract()?;
    Ok(snapshot)
}

#[derive(Debug, Clone, PartialEq)]
pub struct CachedSensoryAdapter {
    snapshot: SensorySnapshot,
}

impl CachedSensoryAdapter {
    pub fn new(snapshot: SensorySnapshot) -> Result<Self, ScaffoldContractError> {
        snapshot.validate_contract()?;
        Ok(Self { snapshot })
    }
}

impl ReferenceSensoryAdapter for CachedSensoryAdapter {
    fn gather_sensory(
        &mut self,
        request: ReferenceSensoryRequest,
    ) -> Result<SensorySnapshot, ScaffoldContractError> {
        if self.snapshot.organism_id != request.organism_id || self.snapshot.tick != request.tick {
            return Err(ScaffoldContractError::MismatchedCreatureId);
        }
        Ok(self.snapshot.clone())
    }
}

fn proximity_salience(distance: f32, radius: f32) -> f32 {
    if !distance.is_finite() || radius <= 0.0 {
        return 0.0;
    }
    (1.0 - distance / radius).clamp(0.0, 1.0)
}
