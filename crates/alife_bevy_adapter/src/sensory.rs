//! Explicit Bevy-world sensory conversion into grounded core snapshots.

use alife_core::{
    AffordanceBits, Confidence, ContextStreams, HeardToken, LanguageContextSnapshot,
    NormalizedScalar, OrganismId, ReferenceSensoryAdapter, ReferenceSensoryRequest,
    ScaffoldContractError, SensoryChannels, SensorySnapshot, SignedValence, SocialAgentSnapshot,
    SocialProximityEntry, Tick, Validate, WorldEntityId, MAX_HEARD_TOKENS, MAX_SOCIAL_AGENTS,
    SENSORY_AUDITORY_CHANNEL_COUNT, SENSORY_SMELL_CHANNEL_COUNT, SENSORY_TACTILE_CHANNEL_COUNT,
    SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT,
};
use bevy::prelude::{Entity, Vec3};

use crate::math::bevy_vec3_to_core;

pub const DEFAULT_VISION_RADIUS: f32 = 8.0;
pub const DEFAULT_HEARING_RADIUS: f32 = 6.0;
const CONTACT_RADIUS: f32 = 0.75;
const MAX_VISIBLE_ENTITIES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObserverSensoryProfile {
    pub vision_radius_meters: f32,
    pub hearing_radius_meters: f32,
}

impl Default for ObserverSensoryProfile {
    fn default() -> Self {
        Self {
            vision_radius_meters: DEFAULT_VISION_RADIUS,
            hearing_radius_meters: DEFAULT_HEARING_RADIUS,
        }
    }
}

impl ObserverSensoryProfile {
    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        if !self.vision_radius_meters.is_finite()
            || !self.hearing_radius_meters.is_finite()
            || self.vision_radius_meters < 0.0
            || self.hearing_radius_meters < 0.0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(self)
    }
}

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
    pub visual_salience_scale: f32,
    pub smell_salience_scale: f32,
    pub audible_radius_meters: f32,
    pub forward: Vec3,
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
            visual_salience_scale: 1.0,
            smell_salience_scale: 1.0,
            audible_radius_meters: DEFAULT_HEARING_RADIUS,
            forward: Vec3::NEG_Z,
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

#[derive(Debug, Clone, Copy)]
struct Ranked<T> {
    salience: f32,
    world_id: WorldEntityId,
    value: T,
}

#[derive(Debug, Clone, Copy)]
struct SocialSample {
    proximity: SocialProximityEntry,
    agent: SocialAgentSnapshot,
}

pub fn gather_sensory_from_observed(
    organism_id: OrganismId,
    tick: Tick,
    observer_world_id: WorldEntityId,
    observer_position: Vec3,
    observed: &[ObservedBevyEntity],
) -> Result<SensorySnapshot, ScaffoldContractError> {
    gather_sensory_from_observed_with_profile(
        organism_id,
        tick,
        observer_world_id,
        observer_position,
        ObserverSensoryProfile::default(),
        observed,
    )
}

pub fn gather_sensory_from_observed_with_profile(
    organism_id: OrganismId,
    tick: Tick,
    observer_world_id: WorldEntityId,
    observer_position: Vec3,
    observer_profile: ObserverSensoryProfile,
    observed: &[ObservedBevyEntity],
) -> Result<SensorySnapshot, ScaffoldContractError> {
    organism_id.validate()?;
    observer_world_id.validate()?;
    observer_profile.validate()?;
    let observer_core_position = bevy_vec3_to_core(observer_position)?;

    let mut visual = [0.0_f32; SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT];
    let mut auditory = [0.0_f32; SENSORY_AUDITORY_CHANNEL_COUNT];
    let mut smell = [0.0_f32; SENSORY_SMELL_CHANNEL_COUNT];
    let mut tactile = [0.0_f32; SENSORY_TACTILE_CHANNEL_COUNT];
    let mut affordances = AffordanceBits::NONE;
    let mut pain = 0.0_f32;
    let mut ranked_tokens = [None; MAX_HEARD_TOKENS];
    let mut ranked_social = [None; MAX_SOCIAL_AGENTS];
    let mut visible_count = 0_usize;

    for entity in observed {
        entity.world_id.validate()?;
        bevy_vec3_to_core(entity.position)?;
        bevy_vec3_to_core(entity.forward)?;
        for value in [
            entity.nutrition,
            entity.hazard_pain,
            entity.visual_salience_scale,
            entity.smell_salience_scale,
            entity.audible_radius_meters,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        if entity.world_id == observer_world_id {
            continue;
        }

        let delta = entity.position - observer_position;
        let distance = delta.length();
        let visual_salience = (proximity_salience(distance, observer_profile.vision_radius_meters)
            * entity.visual_salience_scale)
            .clamp(0.0, 1.0);
        let audible_salience = proximity_salience(
            distance,
            observer_profile
                .hearing_radius_meters
                .min(entity.audible_radius_meters),
        );
        let touching = distance <= CONTACT_RADIUS;
        if visual_salience == 0.0 && audible_salience == 0.0 && !touching {
            continue;
        }
        if visual_salience > 0.0 {
            visible_count = visible_count.saturating_add(1);
        }
        if visual_salience > 0.0 || touching {
            affordances |= entity.affordances;
        }

        if entity.affordances.contains(AffordanceBits::FOOD) {
            visual[0] = visual[0].max(visual_salience);
            smell[0] = smell[0].max(
                (visual_salience * entity.smell_salience_scale * entity.nutrition.max(0.1))
                    .clamp(0.0, 1.0),
            );
        }
        if entity.affordances.contains(AffordanceBits::HAZARD) {
            visual[1] = visual[1].max(visual_salience);
            smell[1] =
                smell[1].max((visual_salience * entity.smell_salience_scale).clamp(0.0, 1.0));
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
            if visual_salience > 0.0 {
                if let Some(agent_id) = entity.organism_id {
                    agent_id.validate()?;
                    insert_ranked(
                        &mut ranked_social,
                        Ranked {
                            salience: visual_salience,
                            world_id: entity.world_id,
                            value: SocialSample {
                                proximity: SocialProximityEntry {
                                    agent_id,
                                    proximity: NormalizedScalar::new(visual_salience)?,
                                    confidence: Confidence::new(0.8)?,
                                },
                                agent: SocialAgentSnapshot {
                                    agent_id,
                                    body_entity: Some(entity.world_id),
                                    relative_position: bevy_vec3_to_core(delta)?,
                                    gaze_direction: bevy_vec3_to_core(entity.forward)?,
                                    orientation_forward: bevy_vec3_to_core(entity.forward)?,
                                    affinity: SignedValence::new(0.0)?,
                                    proximity: NormalizedScalar::new(visual_salience)?,
                                },
                            },
                        },
                    );
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
        if touching {
            tactile[1] = 1.0;
        }
        if let Some(token_id) = entity.token_id {
            if audible_salience > 0.0 {
                auditory[0] = auditory[0].max(audible_salience);
                insert_ranked(
                    &mut ranked_tokens,
                    Ranked {
                        salience: audible_salience,
                        world_id: entity.world_id,
                        value: HeardToken {
                            utterance_id: alife_core::UtteranceId::new(entity.world_id.raw())?,
                            sequence_position: 0,
                            source_kind: if entity.organism_id.is_some() {
                                alife_core::UtteranceSourceKind::Creature
                            } else {
                                alife_core::UtteranceSourceKind::Teacher
                            },
                            speaker_id: entity.organism_id,
                            addressee: None,
                            source_entity: Some(entity.world_id),
                            token_id,
                            source_position: bevy_vec3_to_core(entity.position)?,
                            confidence: Confidence::new(audible_salience.max(0.1))?,
                            teacher_channel: None,
                        },
                    },
                );
            }
        }
    }

    let vocal_tokens = ranked_tokens.map(|entry| entry.map(|entry| entry.value));
    let social_proximity = ranked_social.map(|entry| entry.map(|entry| entry.value.proximity));
    let social_agents = ranked_social.map(|entry| entry.map(|entry| entry.value.agent));
    let heard_any = vocal_tokens.iter().any(Option::is_some);

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
        word_confidence: Confidence::new(if heard_any { 0.8 } else { 0.0 })?,
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
        request.organism_id.validate()?;
        request.body_pose.validate()?;
        request.body_velocity.validate()?;
        request.homeostasis.validate_contract()?;
        if self.snapshot.organism_id != request.organism_id {
            return Err(ScaffoldContractError::MismatchedCreatureId);
        }
        if self.snapshot.tick != request.tick
            || self.snapshot.observer_position != request.body_pose.translation
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
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

fn insert_ranked<T: Copy, const N: usize>(slots: &mut [Option<Ranked<T>>; N], value: Ranked<T>) {
    let insert_at = slots.iter().position(|slot| {
        slot.is_none_or(|current| {
            value.salience > current.salience
                || (value.salience == current.salience
                    && value.world_id.raw() < current.world_id.raw())
        })
    });
    let Some(insert_at) = insert_at else {
        return;
    };
    for index in (insert_at + 1..N).rev() {
        slots[index] = slots[index - 1];
    }
    slots[insert_at] = Some(value);
}
