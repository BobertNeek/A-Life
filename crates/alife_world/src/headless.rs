//! v0 runtime scaffold: deterministic Bevy-independent headless world harness.
//!
//! This module owns simple world truth for P17 behavior tests. It implements
//! core sensory/action adapter traits without importing renderer, GPU, or ECS
//! concepts.

use std::{
    cell::{Ref, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use alife_core::{
    ActionCommand, ActionId, ActionKind, AffordanceBits, BiochemistryState, BodyEventDelta,
    BodySnapshot, BrainTickInput, BrainTickOutput, CanonicalDigestBuilder, ChannelCommand,
    Confidence, ContextStreams, DriveDelta, EndocrineDelta, ExperiencePatch, HeardToken,
    HomeostaticDelta, HomeostaticSnapshot, Intensity, JointPhysicalOutcome,
    LanguageContextSnapshot, LanguageTokenId, MeasuredChannelObservation, MotorChannel,
    MotorCommandBundle, NeuralEmissionFrame, NormalizedScalar, OrganismId, PassiveBodyUpkeepPolicy,
    PerceptionContextBlock, PerceptionFrame, PerceptionFrameDraft, PhysicalActionOutcome,
    PhysicalContactKind, PlayerUtterance, Pose, Quatf, ReferenceActionExecution,
    ReferenceActionExecutor, ReferenceActionFailure, ReferenceOutcomeObservation,
    ReferenceOutcomeObserver, ReferenceOutcomeRequest, ReferenceSensoryAdapter,
    ReferenceSensoryRequest, ScaffoldContractError, SensorProfile, SensorProfileProvenance,
    SensoryAbiVersion, SensoryChannels, SensorySnapshot, SignedValence, SleepConsolidationReport,
    SleepTransition, SocialAgentSnapshot, SocialProximityEntry, SpeechActKind, SpeechMotorPayload,
    TeacherPerceptionChannel, Tick, UtteranceId, UtteranceSourceKind, Validate, Vec3f, Velocity,
    WorldEntityId, MAX_HEARD_TOKENS, MAX_SOCIAL_AGENTS, SENSORY_AUDITORY_CHANNEL_COUNT,
    SENSORY_SMELL_CHANNEL_COUNT, SENSORY_TACTILE_CHANNEL_COUNT,
    SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT,
};

use crate::candidate_enumerator::{
    CandidateEnumerator, GroundedCandidateEnumerator, HeadlessCandidateEnumerator,
    HEADLESS_VISION_RADIUS,
};
use crate::ecology::{
    deterministic_zone_position, EcologyConfig, EcologyMetrics, EcologySensorySummary,
    EcologyState, EcologyStepReport, EcologyZoneId, ResourceLifecycle, ResourceSpawnPolicy,
    TerrainZone, TerrainZoneKind,
};
use crate::habitat::{HabitatAuthority, HabitatAuthorityError};
use crate::organism::{OrganismRegistryError, WorldOrganismRecord, WorldOrganismRegistry};
use crate::presentation::{
    HabitatCreaturePresentation, HabitatPresentationProjection, PairwiseRelationshipProjection,
    PresentationEvidence,
};
use crate::{
    AudibleUtterance, GroundedPhysicalProperties, GroundedSensorExtractor,
    PhysicalObservationSnapshot, PhysicalObservedObject, PhysicalTrackingKey,
    PhysicalTrackingProvenance, SpatialSpeechBus, TrackedObjectRegistry,
    DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM, PHYSICAL_TRACKING_PROVENANCE_SCHEMA_VERSION,
};

const DEFAULT_ENTITY_ID_START: u64 = 1;
const DEFAULT_ORGANISM_ID_START: u64 = 1;
const DEFAULT_HEARING_RADIUS: f32 = 6.0;
pub(crate) const HEADLESS_CONTACT_RADIUS: f32 = 0.75;
const EAT_RADIUS: f32 = 1.25;
const MOVE_STEP: f32 = 1.0;
const MAX_VISIBLE_ENTITIES: usize = 16;
const VOCAL_TOKEN_ID_BASE: u32 = 400_000;
const SPONTANEOUS_SPEECH_COOLDOWN_TICKS: u64 = 32;
const PROMPTED_SPEECH_COOLDOWN_TICKS: u64 = 8;
const HEADLESS_WORLD_SIGNATURE_DOMAIN: &[u8] = b"alife.headless-world.signature.v4";
/// Current schema required by every fresh headless-world signature receipt.
pub const HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION: u16 = 4;

fn map_organism_registry_error(error: OrganismRegistryError) -> ScaffoldContractError {
    match error {
        OrganismRegistryError::InvalidRecord(error) => error,
        _ => ScaffoldContractError::InvalidId,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HeadlessWorldSignatureDigest {
    pub schema_version: u16,
    pub words: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadlessActionIds;

impl HeadlessActionIds {
    pub const APPROACH: ActionId = ActionId(101);
    pub const FLEE: ActionId = ActionId(102);
    pub const EAT: ActionId = ActionId(210);
    pub const GRAB: ActionId = ActionId(211);
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum WorldObjectKind {
    Agent,
    Food,
    Hazard,
    Obstacle,
    Token,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldObject {
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

impl WorldObject {
    pub const fn is_consumed(&self) -> bool {
        self.consumed
    }

    fn affordances(&self) -> AffordanceBits {
        match self.kind {
            WorldObjectKind::Agent => AffordanceBits::SOCIAL_AGENT,
            WorldObjectKind::Food => AffordanceBits::FOOD,
            WorldObjectKind::Hazard => AffordanceBits::HAZARD,
            WorldObjectKind::Obstacle => AffordanceBits::RESOURCE,
            WorldObjectKind::Token => {
                let mut affordances = AffordanceBits::GLYPH_OR_WRITING;
                if self.teacher_channel.is_some() {
                    affordances |= AffordanceBits::TEACHER_OBJECT;
                }
                affordances
            }
        }
    }

    fn blocks_position(&self, position: Vec3f) -> bool {
        self.kind == WorldObjectKind::Obstacle
            && !self.consumed
            && distance(self.position, position) <= self.radius.max(HEADLESS_CONTACT_RADIUS)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibleWorldEntity {
    pub id: WorldEntityId,
    pub kind: WorldObjectKind,
    pub relative_position: Vec3f,
    pub distance: f32,
    pub affordances: AffordanceBits,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessSensoryReport {
    pub core_snapshot: SensorySnapshot,
    pub visible_entities: Vec<VisibleWorldEntity>,
    pub contact_entities: Vec<WorldEntityId>,
    pub touched_entities: Vec<WorldEntityId>,
    pub ecology: EcologySensorySummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessActionResult {
    pub command: ActionCommand,
    pub execution: ReferenceActionExecution,
    /// Learning observation only. Registered biology is advanced from
    /// `body_event` through the world organism registry.
    pub observation: ReferenceOutcomeObservation,
    pub body_event: BodyEventDelta,
    pub touched_entities: Vec<WorldEntityId>,
    pub emitted_utterance: Option<AudibleUtterance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessActionBiologyReceipt {
    pub organism_id: OrganismId,
    pub world_entity_id: WorldEntityId,
    pub outcome_tick: Tick,
    pub action_result: HeadlessActionResult,
    pub biology_before: BiochemistryState,
    pub biology_after: BiochemistryState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessMotorChannelReceipt {
    pub command: ChannelCommand,
    pub observation: MeasuredChannelObservation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessMotorTransactionReceipt {
    pub bundle: MotorCommandBundle,
    pub outcome_tick: Tick,
    pub succeeded: bool,
    pub joint: JointPhysicalOutcome,
    pub channel_receipts: Vec<HeadlessMotorChannelReceipt>,
    pub body_event: BodyEventDelta,
    pub biology_before: BiochemistryState,
    pub biology_after: BiochemistryState,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HeadlessMotorTransactionError {
    Contract(ScaffoldContractError),
    UnsupportedChannel(MotorChannel),
}

impl From<ScaffoldContractError> for HeadlessMotorTransactionError {
    fn from(error: ScaffoldContractError) -> Self {
        Self::Contract(error)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeadlessTelemetry {
    pub sealed_patches: Vec<ExperiencePatch>,
    pub packed_records: Vec<alife_core::PackedExperienceRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessBrainTick {
    pub brain: BrainTickOutput,
    pub action_result: Option<HeadlessActionResult>,
    pub sleep_transition: Option<SleepTransition>,
    pub sleep_report: Option<SleepConsolidationReport>,
}

#[derive(Debug, Clone, Copy)]
struct SpawnSpec<'a> {
    label: &'a str,
    kind: WorldObjectKind,
    organism_id: Option<OrganismId>,
    position: Vec3f,
    nutrition: f32,
    hazard_pain: f32,
    token_id: Option<u32>,
    social_affinity: f32,
    teacher_channel: Option<TeacherPerceptionChannel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldEditorSpawnSpec {
    pub label: String,
    pub kind: WorldObjectKind,
    pub organism_id: Option<OrganismId>,
    pub position: Vec3f,
    pub nutrition: f32,
    pub hazard_pain: f32,
    pub radius: f32,
    pub token_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct HeadlessWorld {
    seed: u64,
    tick: Tick,
    next_entity_id: u64,
    next_organism_id: u64,
    next_spawn_sequence: u64,
    next_utterance_id: u64,
    objects: BTreeMap<u64, WorldObject>,
    labels: BTreeMap<String, WorldEntityId>,
    last_touched_entities: Vec<WorldEntityId>,
    last_action_result: Option<HeadlessActionResult>,
    ecology: EcologyState,
    speech: SpatialSpeechBus,
    last_creature_utterance_ticks: BTreeMap<u64, Tick>,
    tracked_objects: TrackedObjectRegistry,
    habitats: HabitatAuthority,
    organism_registry: WorldOrganismRegistry,
    #[cfg(test)]
    injected_post_action_failure: bool,
    #[cfg(test)]
    injected_tick_late_failure_after_first_organism: bool,
}

/// Opaque, immutable lookup built once for one same-snapshot perception batch.
/// It contains world IDs only; semantic observations are still assembled by
/// the canonical world sensing paths.
#[derive(Debug, Clone)]
pub struct HeadlessPerceptionBatchIndex {
    world_seed: u64,
    world_tick: Tick,
    object_count: usize,
    cells: BTreeMap<(i32, i32, i32), Vec<u64>>,
    organism_entities: BTreeMap<u64, u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct HeadlessWorldPersistenceParts {
    pub seed: u64,
    pub tick: Tick,
    pub next_entity_id: u64,
    pub next_organism_id: u64,
    pub next_spawn_sequence: u64,
    pub next_utterance_id: u64,
    pub objects: Vec<WorldObject>,
    pub last_touched_entities: Vec<WorldEntityId>,
    pub ecology: EcologyState,
    pub audible_utterances: Vec<AudibleUtterance>,
    pub last_creature_utterance_ticks: Vec<(OrganismId, Tick)>,
    pub habitats: HabitatAuthority,
    pub organism_records: Option<Vec<WorldOrganismRecord>>,
}

const HEADLESS_MATING_RADIUS: f32 = 1.0;

impl HeadlessWorld {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            tick: Tick::ZERO,
            next_entity_id: DEFAULT_ENTITY_ID_START,
            next_organism_id: DEFAULT_ORGANISM_ID_START,
            next_spawn_sequence: 1,
            next_utterance_id: 1,
            objects: BTreeMap::new(),
            labels: BTreeMap::new(),
            last_touched_entities: Vec::new(),
            last_action_result: None,
            ecology: EcologyState::default(),
            speech: SpatialSpeechBus::default(),
            last_creature_utterance_ticks: BTreeMap::new(),
            tracked_objects: TrackedObjectRegistry::new(
                seed,
                DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM,
            )
            .expect("the canonical tracked-object capacity is valid"),
            habitats: HabitatAuthority::default(),
            organism_registry: WorldOrganismRegistry::default(),
            #[cfg(test)]
            injected_post_action_failure: false,
            #[cfg(test)]
            injected_tick_late_failure_after_first_organism: false,
        }
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub const fn tick(&self) -> Tick {
        self.tick
    }

    pub fn habitat_authority(&self) -> &HabitatAuthority {
        &self.habitats
    }

    pub fn replace_habitat_authority(
        &mut self,
        authority: HabitatAuthority,
    ) -> Result<(), HabitatAuthorityError> {
        let known_creatures = authority
            .memberships()
            .iter()
            .map(|membership| membership.organism_id)
            .collect::<Vec<_>>();
        authority.validate(&known_creatures)?;
        self.habitats = authority;
        Ok(())
    }

    pub fn habitat_presentation_projection(
        &self,
    ) -> Result<HabitatPresentationProjection, ScaffoldContractError> {
        let stable_entities = self
            .organism_entity_ids()
            .into_iter()
            .map(|(organism_id, entity_id)| (organism_id.raw(), entity_id))
            .collect::<BTreeMap<_, _>>();
        let mut creatures = Vec::with_capacity(self.habitats.memberships().len());

        for membership in self.habitats.memberships() {
            let habitat_mode = self
                .habitats
                .habitat(membership.habitat_id)
                .map(|habitat| habitat.mode)
                .ok_or(ScaffoldContractError::InvalidId)?;
            // Audible speech proves emission, not learned grounding. Keep this
            // unknown until world state owns an utterance-level grounding
            // receipt that can support the token sequence and evidence tick.
            let latest_grounded_utterance = PresentationEvidence::Unknown;

            let mut relationships = Vec::new();
            if stable_entities.contains_key(&membership.organism_id.raw()) {
                let report = self.sensory_report(membership.organism_id, self.tick)?;
                for agent in report.core_snapshot.social_context.nearest_agents {
                    let Some(agent) = agent else {
                        continue;
                    };
                    relationships.push(PairwiseRelationshipProjection {
                        source_organism_id: membership.organism_id,
                        target_organism_id: agent.agent_id,
                        target_stable_world_entity_id: agent.body_entity,
                        affinity: PresentationEvidence::Observed {
                            value: agent.affinity,
                            tick: self.tick,
                        },
                        trust: PresentationEvidence::Unknown,
                        fear: PresentationEvidence::Unknown,
                    });
                }
                relationships.sort_by_key(|edge| edge.target_organism_id.raw());
            }

            creatures.push(HabitatCreaturePresentation {
                organism_id: membership.organism_id,
                stable_world_entity_id: stable_entities.get(&membership.organism_id.raw()).copied(),
                habitat_id: membership.habitat_id,
                habitat_mode,
                latest_grounded_utterance,
                relationships,
            });
        }

        Ok(HabitatPresentationProjection {
            tick: self.tick,
            creatures,
        })
    }

    pub fn try_advance_tick(&mut self) -> Result<Tick, ScaffoldContractError> {
        let mut candidate = self.clone();
        let next_tick = Tick::new(self.tick.raw().saturating_add(1));
        candidate.validate_organism_bindings()?;
        let mut organism_ids = candidate
            .organism_registry
            .iter()
            .map(|record| record.organism_id())
            .collect::<Vec<_>>();
        organism_ids.sort_unstable_by_key(|organism_id| organism_id.raw());
        let mut alive_organism_ids = candidate
            .organism_registry
            .iter()
            .filter(|record| record.lifecycle().is_alive())
            .map(|record| record.organism_id())
            .collect::<Vec<_>>();
        alive_organism_ids.sort_unstable_by_key(|organism_id| organism_id.raw());

        let mut eligible_pairs = Vec::new();
        let mut mating_organism_ids = BTreeSet::new();
        for (maternal_index, maternal_id) in alive_organism_ids.iter().enumerate() {
            let maternal = candidate
                .organism_registry
                .get(*maternal_id)
                .ok_or(ScaffoldContractError::InvalidId)?;
            let maternal_object = candidate
                .objects
                .get(&maternal.world_entity_id().raw())
                .ok_or(ScaffoldContractError::InvalidId)?;
            for paternal_id in alive_organism_ids.iter().skip(maternal_index + 1) {
                let paternal = candidate
                    .organism_registry
                    .get(*paternal_id)
                    .ok_or(ScaffoldContractError::InvalidId)?;
                let paternal_object = candidate
                    .objects
                    .get(&paternal.world_entity_id().raw())
                    .ok_or(ScaffoldContractError::InvalidId)?;
                if distance(maternal_object.position, paternal_object.position)
                    > HEADLESS_MATING_RADIUS
                {
                    continue;
                }
                let maternal_expressed_brain_class = maternal.genome().expressed_brain_class()?;
                let paternal_expressed_brain_class = paternal.genome().expressed_brain_class()?;
                if maternal.genome().id == paternal.genome().id
                    || maternal.genome().foundation.compatibility_family_id
                        != paternal.genome().foundation.compatibility_family_id
                    || maternal.genome().foundation.brain_class_id
                        != paternal.genome().foundation.brain_class_id
                    || maternal_expressed_brain_class != paternal_expressed_brain_class
                    || maternal_expressed_brain_class != maternal.genome().foundation.brain_class_id
                    || paternal_expressed_brain_class != paternal.genome().foundation.brain_class_id
                {
                    continue;
                }
                mating_organism_ids.insert(maternal_id.raw());
                mating_organism_ids.insert(paternal_id.raw());
                eligible_pairs.push((
                    *maternal_id,
                    *paternal_id,
                    maternal_object.position,
                    paternal_object.position,
                ));
            }
        }

        #[cfg(test)]
        let mut advanced_organism = false;
        for organism_id in organism_ids {
            let biology_tick = candidate
                .organism_registry
                .get(organism_id)
                .ok_or(ScaffoldContractError::InvalidId)?
                .biochemistry()
                .tick;
            match biology_tick {
                tick if tick == self.tick => {
                    let body_event = if mating_organism_ids.contains(&organism_id.raw()) {
                        BodyEventDelta {
                            mating_opportunity: 1.0,
                            ..BodyEventDelta::zero()
                        }
                    } else {
                        BodyEventDelta::zero()
                    };
                    candidate
                        .organism_registry
                        .advance_biology(organism_id, next_tick, body_event)
                        .map_err(map_organism_registry_error)?;
                    #[cfg(test)]
                    {
                        advanced_organism = true;
                    }
                }
                tick if tick == next_tick => {}
                _ => return Err(ScaffoldContractError::NonMonotonicTick),
            }

            let terminal = {
                let record = candidate
                    .organism_registry
                    .get(organism_id)
                    .ok_or(ScaffoldContractError::InvalidId)?;
                let age_ticks = record.age_at(next_tick)?.raw();
                PassiveBodyUpkeepPolicy::is_terminal(
                    &record.biochemistry().body,
                    age_ticks,
                    record.phenotype(),
                )
            };
            if terminal {
                candidate
                    .organism_registry
                    .mark_dead(organism_id, next_tick)
                    .map_err(map_organism_registry_error)?;
            }

            #[cfg(test)]
            if advanced_organism && candidate.injected_tick_late_failure_after_first_organism {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        }

        let living_capacity = candidate
            .ecology
            .config
            .max_world_objects
            .saturating_sub(candidate.ecology.config.max_resource_records);
        let alive_count = candidate
            .organism_registry
            .iter()
            .filter(|record| record.lifecycle().is_alive())
            .count();

        let mut conception_pair = None;
        if alive_count < living_capacity {
            for pair in eligible_pairs {
                let maternal = candidate
                    .organism_registry
                    .get(pair.0)
                    .ok_or(ScaffoldContractError::InvalidId)?;
                let paternal = candidate
                    .organism_registry
                    .get(pair.1)
                    .ok_or(ScaffoldContractError::InvalidId)?;
                let maternal_age = maternal.age_at(next_tick)?;
                let paternal_age = paternal.age_at(next_tick)?;
                if maternal.lifecycle().is_alive()
                    && paternal.lifecycle().is_alive()
                    && maternal.biochemistry().reproduction.ready
                    && paternal.biochemistry().reproduction.ready
                    && maternal.biochemistry().reproduction.last_update_tick == maternal_age
                    && paternal.biochemistry().reproduction.last_update_tick == paternal_age
                {
                    conception_pair = Some(pair);
                    break;
                }
            }
        }

        if let Some((maternal_id, paternal_id, maternal_position, paternal_position)) =
            conception_pair
        {
            let mut conception_seed = candidate.seed
                ^ next_tick.raw().rotate_left(17)
                ^ maternal_id.raw().rotate_left(31)
                ^ paternal_id.raw().rotate_right(11)
                ^ 0xC0A1_CE71_4A2D_0001;
            conception_seed =
                (conception_seed ^ (conception_seed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            conception_seed =
                (conception_seed ^ (conception_seed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            conception_seed ^= conception_seed >> 31;
            if conception_seed == 0 {
                conception_seed = 1;
            }

            let maternal_genome = candidate
                .organism_registry
                .get(maternal_id)
                .ok_or(ScaffoldContractError::InvalidId)?
                .genome();
            let paternal_genome = candidate
                .organism_registry
                .get(paternal_id)
                .ok_or(ScaffoldContractError::InvalidId)?
                .genome();
            let child_genome = alife_core::CreatureGenome::reproduce(
                maternal_genome,
                paternal_genome,
                conception_seed,
            )?;
            let child_phenotype = child_genome.express()?;
            let child_id = OrganismId(candidate.next_organism_id);
            child_id.validate()?;
            candidate.next_organism_id = child_id
                .raw()
                .checked_add(1)
                .ok_or(ScaffoldContractError::InvalidId)?;
            let midpoint = Vec3f::new(
                (maternal_position.x + paternal_position.x) * 0.5,
                (maternal_position.y + paternal_position.y) * 0.5,
                (maternal_position.z + paternal_position.z) * 0.5,
            );
            midpoint.validate()?;
            let child_label = format!("organism-{}", child_id.raw());
            let child_entity_id =
                candidate.spawn_social_agent(&child_label, child_id, midpoint, 0.0)?;
            let child_record = WorldOrganismRecord::newborn(
                child_id,
                child_entity_id,
                child_genome,
                child_phenotype,
                next_tick,
            )
            .map_err(map_organism_registry_error)?;
            candidate.register_organism_record(child_record)?;
            candidate.validate_complete_organism_bindings()?;
        }

        candidate.tick = next_tick;
        candidate.speech.retire_expired(candidate.tick);
        let _ = candidate.advance_ecology_at_current_tick();
        *self = candidate;
        Ok(next_tick)
    }

    pub fn advance_tick(&mut self) -> Tick {
        self.try_advance_tick()
            .expect("authoritative world tick scheduling failed")
    }

    #[cfg(test)]
    fn inject_tick_late_failure_after_first_organism_for_test(&mut self) {
        self.injected_tick_late_failure_after_first_organism = true;
    }

    pub fn emit_player_utterance(
        &mut self,
        utterance: PlayerUtterance,
    ) -> Result<(), ScaffoldContractError> {
        self.next_utterance_id = self
            .next_utterance_id
            .max(utterance.utterance_id.raw().saturating_add(1));
        self.speech
            .emit(AudibleUtterance::from_player(utterance, self.tick)?)
    }

    pub fn emit_player_tokens(
        &mut self,
        addressee: Option<OrganismId>,
        source_position: Vec3f,
        tokens: Vec<alife_core::LanguageTokenId>,
    ) -> Result<AudibleUtterance, ScaffoldContractError> {
        let utterance_id = UtteranceId::new(self.next_utterance_id)?;
        self.next_utterance_id = self
            .next_utterance_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let audible = AudibleUtterance::from_player(
            PlayerUtterance::try_new(utterance_id, addressee, source_position, tokens)?,
            self.tick,
        )?;
        self.speech.emit(audible.clone())?;
        Ok(audible)
    }

    pub fn audible_utterances(&self) -> Vec<AudibleUtterance> {
        self.speech.snapshot()
    }

    pub fn emit_creature_utterance(
        &mut self,
        utterance_id: UtteranceId,
        speaker_id: OrganismId,
        addressee: Option<OrganismId>,
        payload: SpeechMotorPayload,
    ) -> Result<AudibleUtterance, ScaffoldContractError> {
        self.next_utterance_id = self
            .next_utterance_id
            .max(utterance_id.raw().saturating_add(1));
        let position = self.agent_for(speaker_id)?.position;
        let utterance = AudibleUtterance::from_creature(
            utterance_id,
            speaker_id,
            addressee,
            position,
            payload,
            self.tick,
        )?;
        self.speech.emit(utterance.clone())?;
        Ok(utterance)
    }

    pub fn emit_teacher_tokens(
        &mut self,
        addressee: Option<OrganismId>,
        source_position: Vec3f,
        tokens: Vec<alife_core::LanguageTokenId>,
        teacher_channel: TeacherPerceptionChannel,
    ) -> Result<AudibleUtterance, ScaffoldContractError> {
        let utterance_id = UtteranceId::new(self.next_utterance_id)?;
        self.next_utterance_id = self
            .next_utterance_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let utterance = AudibleUtterance::from_teacher(
            utterance_id,
            addressee,
            source_position,
            tokens,
            teacher_channel,
            self.tick,
        )?;
        self.speech.emit(utterance.clone())?;
        Ok(utterance)
    }

    pub fn advance_ecology(&mut self) -> EcologyStepReport {
        self.advance_ecology_at_current_tick()
    }

    pub fn ecology(&self) -> &EcologyState {
        &self.ecology
    }

    pub fn ecology_metrics(&self) -> EcologyMetrics {
        self.ecology.metrics()
    }

    pub fn configure_ecology(
        &mut self,
        ecology: EcologyState,
    ) -> Result<(), ScaffoldContractError> {
        ecology.validate()?;
        for resource in &ecology.resources {
            let Some(object) = self.objects.get(&resource.object_id.raw()) else {
                return Err(ScaffoldContractError::InvalidId);
            };
            if object.kind != WorldObjectKind::Food {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        self.ecology = ecology;
        self.rebuild_ecology_metrics();
        Ok(())
    }

    pub fn add_terrain_zone(&mut self, zone: TerrainZone) -> Result<(), ScaffoldContractError> {
        self.ecology.add_zone(zone)?;
        self.rebuild_ecology_metrics();
        Ok(())
    }

    pub fn track_resource_lifecycle(
        &mut self,
        object_id: WorldEntityId,
        home_zone: EcologyZoneId,
        regrow_after_ticks: u32,
        decay_after_ticks: u32,
    ) -> Result<(), ScaffoldContractError> {
        let Some(object) = self.objects.get(&object_id.raw()) else {
            return Err(ScaffoldContractError::InvalidId);
        };
        if object.kind != WorldObjectKind::Food || self.ecology.zone(home_zone).is_none() {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.ecology.add_resource(ResourceLifecycle {
            object_id,
            home_zone,
            base_nutrition: object.nutrition,
            regrow_after_ticks,
            decay_after_ticks,
            consumed_at_tick: object.consumed.then_some(self.tick),
            last_regrown_tick: None,
            low_salience_marker: object.consumed,
        })?;
        self.rebuild_ecology_metrics();
        Ok(())
    }

    pub fn add_resource_spawn_policy(
        &mut self,
        policy: ResourceSpawnPolicy,
    ) -> Result<(), ScaffoldContractError> {
        self.ecology.add_spawn_policy(policy)?;
        Ok(())
    }

    pub fn entity_id(&self, label: &str) -> Option<WorldEntityId> {
        self.labels.get(label).copied()
    }

    pub fn entity(&self, id: WorldEntityId) -> Option<&WorldObject> {
        self.objects.get(&id.raw())
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Canonical identity of the complete caller-visible world state before an
    /// action. It binds ticks as well as object, ecology, habitat, speech, and
    /// per-organism tracked-object state, so a same-seed later world is not the
    /// same causal input.
    pub fn canonical_signature_digest(
        &self,
    ) -> Result<HeadlessWorldSignatureDigest, ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(HEADLESS_WORLD_SIGNATURE_DOMAIN);
        digest.write_u16(HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION);
        digest.write_u64(self.seed);
        digest.write_u64(self.tick.raw());
        digest.write_u64(self.next_entity_id);
        digest.write_u64(self.next_organism_id);
        digest.write_u64(self.next_spawn_sequence);
        digest.write_u64(self.next_utterance_id);

        digest.write_sequence_len(self.objects.len());
        for object in self.objects.values() {
            write_world_object_signature(&mut digest, object)?;
        }
        self.organism_registry
            .write_canonical_signature(&mut digest)?;
        digest.write_bytes(
            &serde_json::to_vec(&self.ecology).map_err(|_| ScaffoldContractError::InvalidId)?,
        );
        digest.write_bytes(
            &serde_json::to_vec(&self.habitats).map_err(|_| ScaffoldContractError::InvalidId)?,
        );
        digest.write_bytes(
            &serde_json::to_vec(&self.speech.snapshot())
                .map_err(|_| ScaffoldContractError::InvalidId)?,
        );

        digest.write_sequence_len(self.last_touched_entities.len());
        for entity_id in &self.last_touched_entities {
            digest.write_u64(entity_id.raw());
        }
        digest.write_sequence_len(self.last_creature_utterance_ticks.len());
        for (organism_id, tick) in &self.last_creature_utterance_ticks {
            digest.write_u64(*organism_id);
            digest.write_u64(tick.raw());
        }

        let organisms = self.organism_entity_ids();
        digest.write_sequence_len(organisms.len());
        for (organism_id, _) in organisms {
            let tracked = self.tracked_objects.save_state(organism_id)?;
            digest.write_bytes(
                &serde_json::to_vec(&tracked).map_err(|_| ScaffoldContractError::InvalidId)?,
            );
        }
        Ok(HeadlessWorldSignatureDigest {
            schema_version: HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION,
            words: digest.finish256(),
        })
    }

    pub fn stable_signature(&self) -> Vec<String> {
        self.objects
            .values()
            .map(|object| {
                format!(
                    "{}:{:?}:{}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{:?}:{:.3}:{:?}:{}:{:?}:{}",
                    object.id.raw(),
                    object.kind,
                    object.label,
                    object.position.x,
                    object.position.y,
                    object.position.z,
                    object.nutrition,
                    object.hazard_pain,
                    object.token_id,
                    object.social_affinity,
                    object.teacher_channel,
                    object.consumed,
                    object.carried_by,
                    grounded_identity_signature(object),
                )
            })
            .collect()
    }

    pub fn object_snapshots(&self) -> Vec<WorldObject> {
        self.objects.values().cloned().collect()
    }

    pub fn build_perception_batch_index(
        &self,
    ) -> Result<HeadlessPerceptionBatchIndex, ScaffoldContractError> {
        let mut cells = BTreeMap::<(i32, i32, i32), Vec<u64>>::new();
        let mut organism_entities = BTreeMap::new();
        for object in self.objects.values() {
            object.position.validate()?;
            cells
                .entry(perception_cell(object.position))
                .or_default()
                .push(object.id.raw());
            if let Some(organism_id) = object.organism_id {
                if organism_entities
                    .insert(organism_id.raw(), object.id.raw())
                    .is_some()
                {
                    return Err(ScaffoldContractError::InvalidId);
                }
            }
        }
        Ok(HeadlessPerceptionBatchIndex {
            world_seed: self.seed,
            world_tick: self.tick,
            object_count: self.objects.len(),
            cells,
            organism_entities,
        })
    }

    pub fn tracked_objects(&self) -> &TrackedObjectRegistry {
        &self.tracked_objects
    }

    pub fn restore_tracked_object_states(
        &mut self,
        states: impl IntoIterator<Item = crate::TrackedObjectRegistrySaveState>,
    ) -> Result<(), ScaffoldContractError> {
        let restored = TrackedObjectRegistry::from_save_states(
            self.seed,
            self.tracked_objects.per_organism_capacity(),
            states,
        )?;
        self.tracked_objects = restored;
        Ok(())
    }

    pub fn set_grounded_physical_properties(
        &mut self,
        id: WorldEntityId,
        properties: GroundedPhysicalProperties,
    ) -> Result<(), ScaffoldContractError> {
        id.validate()?;
        properties.validate_contract()?;
        let object = self
            .objects
            .get_mut(&id.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        object.grounded_physical = properties;
        Ok(())
    }

    pub fn organism_entity_ids(&self) -> Vec<(OrganismId, WorldEntityId)> {
        self.objects
            .values()
            .filter_map(|object| object.organism_id.map(|organism| (organism, object.id)))
            .collect()
    }

    pub fn organism_registry(&self) -> &WorldOrganismRegistry {
        &self.organism_registry
    }

    pub fn register_organism_record(
        &mut self,
        record: WorldOrganismRecord,
    ) -> Result<(), ScaffoldContractError> {
        record.validate_contract()?;
        let registered_next_organism_id = record
            .organism_id()
            .raw()
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        self.organism_registry
            .validate_contract()
            .map_err(map_organism_registry_error)?;
        if self.organism_registry.get(record.organism_id()).is_some()
            || self
                .organism_registry
                .get_by_world_entity_id(record.world_entity_id())
                .is_some()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        let object = self
            .objects
            .get(&record.world_entity_id().raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        if object.id != record.world_entity_id()
            || object.kind != WorldObjectKind::Agent
            || object.organism_id != Some(record.organism_id())
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.organism_registry
            .insert(record)
            .map_err(map_organism_registry_error)?;
        self.next_organism_id = self.next_organism_id.max(registered_next_organism_id);
        Ok(())
    }

    pub fn link_life_manifest(
        &mut self,
        organism_id: OrganismId,
        digest: alife_core::Blake3Digest,
    ) -> Result<(), ScaffoldContractError> {
        let mut candidate = self.clone();
        candidate.validate_organism_bindings()?;
        candidate
            .organism_registry
            .link_life_manifest(organism_id, digest)
            .map_err(map_organism_registry_error)?;
        candidate.validate_organism_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub fn link_birth_manifest(
        &mut self,
        organism_id: OrganismId,
        digest: alife_core::Blake3Digest,
    ) -> Result<(), ScaffoldContractError> {
        let mut candidate = self.clone();
        candidate.validate_organism_bindings()?;
        candidate
            .organism_registry
            .link_birth_manifest(organism_id, digest)
            .map_err(map_organism_registry_error)?;
        candidate.validate_organism_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub fn retire_dead_organism(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<(WorldOrganismRecord, WorldObject), ScaffoldContractError> {
        let mut candidate = self.clone();
        candidate.validate_organism_bindings()?;
        let record = candidate
            .organism_registry
            .get(organism_id)
            .cloned()
            .ok_or(ScaffoldContractError::InvalidId)?;
        let object = candidate
            .objects
            .get(&record.world_entity_id().raw())
            .cloned()
            .ok_or(ScaffoldContractError::InvalidId)?;
        if object.id != record.world_entity_id()
            || object.kind != WorldObjectKind::Agent
            || object.organism_id != Some(organism_id)
            || candidate.labels.get(&object.label) != Some(&object.id)
        {
            return Err(ScaffoldContractError::InvalidId);
        }

        let final_record = candidate
            .organism_registry
            .remove_dead(organism_id)
            .map_err(map_organism_registry_error)?;
        let final_object = candidate
            .objects
            .remove(&object.id.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        candidate.labels.remove(&final_object.label);
        candidate
            .last_touched_entities
            .retain(|touched| *touched != final_object.id);
        if candidate.last_action_result.as_ref().is_some_and(|result| {
            result.command.organism_id == organism_id
                || result.touched_entities.contains(&final_object.id)
        }) {
            candidate.last_action_result = None;
        }
        candidate
            .last_creature_utterance_ticks
            .remove(&organism_id.raw());
        candidate.validate_organism_bindings()?;
        *self = candidate;
        Ok((final_record, final_object))
    }

    pub fn replace_organism_registry_exact<I>(
        &mut self,
        records: I,
    ) -> Result<(), ScaffoldContractError>
    where
        I: IntoIterator<Item = WorldOrganismRecord>,
    {
        let registry = WorldOrganismRegistry::from_exact_records(records)
            .map_err(map_organism_registry_error)?;
        let replacement_next_organism_id = registry
            .iter()
            .map(|record| record.organism_id().raw())
            .max()
            .map(|organism_id| {
                organism_id
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::InvalidId)
            })
            .transpose()?;
        let mut replacement = self.clone();
        replacement.organism_registry = registry;
        if let Some(next_organism_id) = replacement_next_organism_id {
            replacement.next_organism_id = replacement.next_organism_id.max(next_organism_id);
        }
        replacement.validate_complete_organism_bindings()?;
        *self = replacement;
        Ok(())
    }

    fn validate_complete_organism_bindings(&self) -> Result<(), ScaffoldContractError> {
        self.validate_organism_bindings()?;

        let mut agent_cohort = BTreeMap::new();
        for object in self.objects.values() {
            if object.kind != WorldObjectKind::Agent {
                continue;
            }
            let organism_id = object.organism_id.ok_or(ScaffoldContractError::InvalidId)?;
            organism_id.validate()?;
            if agent_cohort.insert(organism_id.raw(), object.id).is_some() {
                return Err(ScaffoldContractError::InvalidId);
            }
        }

        let registered_ids = self
            .organism_registry
            .iter()
            .map(|record| record.organism_id().raw())
            .collect::<BTreeSet<_>>();
        let cohort_ids = agent_cohort.keys().copied().collect::<BTreeSet<_>>();
        if registered_ids != cohort_ids {
            return Err(ScaffoldContractError::InvalidId);
        }

        for (organism_id, world_entity_id) in agent_cohort {
            let record = self
                .organism_registry
                .get(OrganismId(organism_id))
                .ok_or(ScaffoldContractError::InvalidId)?;
            if record.world_entity_id() != world_entity_id {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }

    /// Validates registered records only. Legacy unregistered Agent fixtures
    /// remain valid until Task 3.2 seeds the production registry.
    pub fn validate_organism_bindings(&self) -> Result<(), ScaffoldContractError> {
        self.organism_registry
            .validate_contract()
            .map_err(map_organism_registry_error)?;
        for record in self.organism_registry.iter() {
            let object = self
                .objects
                .get(&record.world_entity_id().raw())
                .ok_or(ScaffoldContractError::InvalidId)?;
            if object.id != record.world_entity_id()
                || object.kind != WorldObjectKind::Agent
                || object.organism_id != Some(record.organism_id())
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }

    pub fn remove_organism(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<WorldObject, ScaffoldContractError> {
        organism_id.validate()?;
        let entity_id = self
            .organism_entity_ids()
            .into_iter()
            .find_map(|(candidate, entity_id)| (candidate == organism_id).then_some(entity_id))
            .ok_or(ScaffoldContractError::InvalidId)?;
        self.remove_agent_entity(entity_id)
    }

    pub fn spawn_social_agent(
        &mut self,
        label: &str,
        organism_id: OrganismId,
        position: Vec3f,
        affinity: f32,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        organism_id.validate()?;
        self.insert_object(SpawnSpec {
            label,
            kind: WorldObjectKind::Agent,
            organism_id: Some(organism_id),
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: None,
            social_affinity: affinity.clamp(-1.0, 1.0),
            teacher_channel: None,
        })
    }

    pub fn remove_agent_entity(
        &mut self,
        id: WorldEntityId,
    ) -> Result<WorldObject, ScaffoldContractError> {
        id.validate()?;
        let object = self
            .objects
            .get(&id.raw())
            .cloned()
            .ok_or(ScaffoldContractError::InvalidId)?;
        if object.kind != WorldObjectKind::Agent {
            return Err(ScaffoldContractError::InvalidId);
        }
        if self.organism_registry.get_by_world_entity_id(id).is_some() {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.objects.remove(&id.raw());
        self.labels.remove(&object.label);
        self.last_touched_entities.retain(|touched| *touched != id);
        if self
            .last_action_result
            .as_ref()
            .is_some_and(|result| result.touched_entities.contains(&id))
        {
            self.last_action_result = None;
        }
        Ok(object)
    }

    pub fn editor_spawn_object(
        &mut self,
        spec: WorldEditorSpawnSpec,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        if matches!(spec.kind, WorldObjectKind::Agent) && spec.organism_id.is_none() {
            return Err(ScaffoldContractError::InvalidId);
        }
        if !matches!(spec.kind, WorldObjectKind::Agent) && spec.organism_id.is_some() {
            return Err(ScaffoldContractError::InvalidId);
        }
        if matches!(spec.kind, WorldObjectKind::Token) && spec.token_id.is_none() {
            return Err(ScaffoldContractError::InvalidId);
        }
        if !spec.radius.is_finite() || spec.radius <= 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let id = self.insert_object(SpawnSpec {
            label: &spec.label,
            kind: spec.kind,
            organism_id: spec.organism_id,
            position: spec.position,
            nutrition: spec.nutrition,
            hazard_pain: spec.hazard_pain,
            token_id: spec.token_id,
            social_affinity: 0.0,
            teacher_channel: None,
        })?;
        if let Some(object) = self.objects.get_mut(&id.raw()) {
            object.radius = spec.radius.max(0.1);
        }
        self.rebuild_ecology_metrics();
        Ok(id)
    }

    pub fn editor_remove_object(
        &mut self,
        id: WorldEntityId,
    ) -> Result<WorldObject, ScaffoldContractError> {
        id.validate()?;
        if self
            .ecology
            .resources
            .iter()
            .any(|resource| resource.object_id == id)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        if self.objects.get(&id.raw()).is_some_and(|object| {
            object.kind == WorldObjectKind::Agent
                && self.organism_registry.get_by_world_entity_id(id).is_some()
        }) {
            return Err(ScaffoldContractError::InvalidId);
        }
        let object = self
            .objects
            .remove(&id.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        self.labels.remove(&object.label);
        self.last_touched_entities.retain(|touched| *touched != id);
        if self
            .last_action_result
            .as_ref()
            .is_some_and(|result| result.touched_entities.contains(&id))
        {
            self.last_action_result = None;
        }
        self.rebuild_ecology_metrics();
        Ok(object)
    }

    pub fn editor_move_object(
        &mut self,
        id: WorldEntityId,
        position: Vec3f,
    ) -> Result<(), ScaffoldContractError> {
        id.validate()?;
        position.validate()?;
        let object = self
            .objects
            .get_mut(&id.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        object.grounded_physical.velocity = subtract(position, object.position);
        object.position = position;
        self.rebuild_ecology_metrics();
        Ok(())
    }

    pub(crate) fn persistence_parts(&self) -> HeadlessWorldPersistenceParts {
        let mut organism_records: Vec<_> = self.organism_registry.iter().cloned().collect();
        organism_records.sort_unstable_by_key(|record| record.organism_id().raw());
        let has_agent_objects = self
            .objects
            .values()
            .any(|object| object.kind == WorldObjectKind::Agent);
        let organism_records = if organism_records.is_empty() && has_agent_objects {
            None
        } else {
            Some(organism_records)
        };
        let max_present_organism_id = self
            .objects
            .values()
            .filter_map(|object| object.organism_id.map(OrganismId::raw))
            .chain(
                self.organism_registry
                    .iter()
                    .map(|record| record.organism_id().raw()),
            )
            .max()
            .unwrap_or(0);
        let next_organism_id = max_present_organism_id
            .checked_add(1)
            .map_or(self.next_organism_id, |derived| {
                self.next_organism_id.max(derived)
            });
        HeadlessWorldPersistenceParts {
            seed: self.seed,
            tick: self.tick,
            next_entity_id: self.next_entity_id,
            next_organism_id,
            next_spawn_sequence: self.next_spawn_sequence,
            next_utterance_id: self.next_utterance_id,
            objects: self.objects.values().cloned().collect(),
            last_touched_entities: self.last_touched_entities.clone(),
            ecology: self.ecology.clone(),
            audible_utterances: self.speech.snapshot(),
            last_creature_utterance_ticks: self
                .last_creature_utterance_ticks
                .iter()
                .map(|(organism, tick)| (OrganismId(*organism), *tick))
                .collect(),
            habitats: self.habitats.clone(),
            organism_records,
        }
    }

    pub(crate) fn from_persistence_parts(
        parts: HeadlessWorldPersistenceParts,
    ) -> Result<Self, ScaffoldContractError> {
        let max_present_organism_id = parts
            .objects
            .iter()
            .filter_map(|object| object.organism_id.map(OrganismId::raw))
            .chain(
                parts
                    .organism_records
                    .as_ref()
                    .into_iter()
                    .flat_map(|records| records.iter())
                    .map(|record| record.organism_id().raw()),
            )
            .max()
            .unwrap_or(0);
        if parts.next_organism_id == 0
            || max_present_organism_id == u64::MAX
            || parts.next_organism_id <= max_present_organism_id
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        let organism_records = parts.organism_records;
        let has_authoritative_organism_records = organism_records.is_some();
        let organism_registry = match organism_records {
            Some(records) => WorldOrganismRegistry::from_exact_records(records)
                .map_err(map_organism_registry_error)?,
            None => WorldOrganismRegistry::default(),
        };
        let mut objects = BTreeMap::new();
        let mut labels = BTreeMap::new();
        let mut tracking_keys = std::collections::BTreeSet::new();
        let mut spawn_sequences = std::collections::BTreeSet::new();
        let mut max_id = 0_u64;
        for object in parts.objects {
            validate_persisted_object(&object)?;
            let raw_id = object.id.raw();
            if object.tracking_provenance.world_seed != parts.seed
                || objects.contains_key(&raw_id)
                || labels.contains_key(&object.label)
                || !tracking_keys.insert(object.tracking_key)
                || !spawn_sequences.insert(object.tracking_provenance.spawn_sequence)
            {
                return Err(ScaffoldContractError::InvalidId);
            }
            max_id = max_id.max(raw_id);
            labels.insert(object.label.clone(), object.id);
            objects.insert(raw_id, object);
        }
        if parts.next_entity_id <= max_id
            || (objects.is_empty() && parts.next_entity_id < DEFAULT_ENTITY_ID_START)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        let max_spawn_sequence = objects
            .values()
            .map(|object| object.tracking_provenance.spawn_sequence)
            .max()
            .unwrap_or(0);
        if parts.next_spawn_sequence <= max_spawn_sequence || parts.next_spawn_sequence == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        let max_utterance_id = parts
            .audible_utterances
            .iter()
            .map(|utterance| utterance.utterance_id.raw())
            .max()
            .unwrap_or(0);
        if parts.next_utterance_id == 0 || parts.next_utterance_id <= max_utterance_id {
            return Err(ScaffoldContractError::InvalidId);
        }
        let mut cooldown_organisms = std::collections::BTreeSet::new();
        for (organism, tick) in &parts.last_creature_utterance_ticks {
            organism.validate()?;
            if tick.raw() > parts.tick.raw() || !cooldown_organisms.insert(organism.raw()) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        parts.ecology.validate()?;
        for resource in &parts.ecology.resources {
            let Some(object) = objects.get(&resource.object_id.raw()) else {
                return Err(ScaffoldContractError::InvalidId);
            };
            if object.kind != WorldObjectKind::Food {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        for touched in &parts.last_touched_entities {
            touched.validate()?;
            if !objects.contains_key(&touched.raw()) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        let world = Self {
            seed: parts.seed,
            tick: parts.tick,
            next_entity_id: parts.next_entity_id,
            next_organism_id: parts.next_organism_id,
            next_spawn_sequence: parts.next_spawn_sequence,
            next_utterance_id: parts.next_utterance_id,
            objects,
            labels,
            last_touched_entities: parts.last_touched_entities,
            last_action_result: None,
            ecology: parts.ecology,
            speech: SpatialSpeechBus::restore(parts.audible_utterances, parts.tick)?,
            last_creature_utterance_ticks: parts
                .last_creature_utterance_ticks
                .into_iter()
                .map(|(organism, tick)| (organism.raw(), tick))
                .collect(),
            tracked_objects: TrackedObjectRegistry::new(
                parts.seed,
                DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM,
            )?,
            habitats: parts.habitats,
            organism_registry,
            #[cfg(test)]
            injected_post_action_failure: false,
            #[cfg(test)]
            injected_tick_late_failure_after_first_organism: false,
        };
        if has_authoritative_organism_records {
            world.validate_complete_organism_bindings()?;
        }
        Ok(world)
    }

    pub fn perception_frame_draft(
        &mut self,
        organism_id: OrganismId,
        tick: Tick,
        profile: SensorProfile,
        homeostasis: HomeostaticSnapshot,
    ) -> Result<PerceptionFrameDraft, ScaffoldContractError> {
        self.perception_frame_draft_with_index(organism_id, tick, profile, homeostasis, None)
    }

    pub fn perception_frame_draft_indexed(
        &mut self,
        organism_id: OrganismId,
        tick: Tick,
        profile: SensorProfile,
        homeostasis: HomeostaticSnapshot,
        index: &HeadlessPerceptionBatchIndex,
    ) -> Result<PerceptionFrameDraft, ScaffoldContractError> {
        self.perception_frame_draft_with_index(organism_id, tick, profile, homeostasis, Some(index))
    }

    fn perception_frame_draft_with_index(
        &mut self,
        organism_id: OrganismId,
        tick: Tick,
        profile: SensorProfile,
        homeostasis: HomeostaticSnapshot,
        index: Option<&HeadlessPerceptionBatchIndex>,
    ) -> Result<PerceptionFrameDraft, ScaffoldContractError> {
        homeostasis
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
        if homeostasis.tick != tick {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let provenance = SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick)?;
        match profile {
            SensorProfile::PrivilegedAffordanceV1 => {
                let report = match index {
                    Some(index) => self.sensory_report_indexed(organism_id, tick, index)?,
                    None => self.sensory_report(organism_id, tick)?,
                };
                let candidates =
                    HeadlessCandidateEnumerator.enumerate_candidates(&report, profile)?;
                let body = BodySnapshot {
                    pose: Pose {
                        translation: report.core_snapshot.observer_position,
                        rotation: Quatf::IDENTITY,
                    },
                    velocity: Velocity::ZERO,
                };
                PerceptionFrameDraft::new(
                    organism_id,
                    tick,
                    profile,
                    report.core_snapshot,
                    body,
                    homeostasis,
                    candidates,
                    provenance,
                    Vec::new(),
                )
            }
            SensorProfile::GroundedObjectSlotsV1 => {
                let snapshot = match index {
                    Some(index) => {
                        self.physical_observation_snapshot_indexed(organism_id, tick, index)?
                    }
                    None => self.physical_observation_snapshot(organism_id, tick)?,
                };
                let grounded =
                    GroundedSensorExtractor::extract(&snapshot, &mut self.tracked_objects)?;
                let candidates = GroundedCandidateEnumerator.enumerate_candidates(&grounded)?;
                let (mut sensory, body, slots, _transports) = grounded.into_parts();
                let heard = self
                    .speech
                    .heard_tokens(organism_id, body.pose.translation, tick)?;
                let mut language_context = LanguageContextSnapshot::default();
                for (index, token) in heard.into_iter().take(MAX_HEARD_TOKENS).enumerate() {
                    sensory.channels.auditory_acoustic[0] =
                        sensory.channels.auditory_acoustic[0].max(token.confidence.raw());
                    language_context.teacher_channel_marker = language_context
                        .teacher_channel_marker
                        .or(token.teacher_channel);
                    language_context.heard_tokens[index] = Some(token);
                }
                if language_context.heard_tokens.iter().any(Option::is_some) {
                    language_context.word_confidence = Confidence::new(0.8)?;
                }
                sensory.language_context = language_context;
                sensory.validate_contract()?;
                PerceptionFrameDraft::new(
                    organism_id,
                    tick,
                    profile,
                    sensory,
                    body,
                    homeostasis,
                    candidates,
                    provenance,
                    slots,
                )
            }
        }
    }

    pub fn physical_observation_snapshot(
        &self,
        organism_id: OrganismId,
        tick: Tick,
    ) -> Result<PhysicalObservationSnapshot, ScaffoldContractError> {
        organism_id.validate()?;
        let observer = self.agent_for(organism_id)?;
        self.physical_observation_snapshot_from_objects(
            organism_id,
            tick,
            observer,
            self.objects.values(),
        )
    }

    pub fn physical_observation_snapshot_indexed(
        &self,
        organism_id: OrganismId,
        tick: Tick,
        index: &HeadlessPerceptionBatchIndex,
    ) -> Result<PhysicalObservationSnapshot, ScaffoldContractError> {
        organism_id.validate()?;
        let observer = self.indexed_agent_for(organism_id, index)?;
        self.physical_observation_snapshot_from_objects(
            organism_id,
            tick,
            observer,
            self.indexed_nearby_objects(observer, index)?.into_iter(),
        )
    }

    fn physical_observation_snapshot_from_objects<'a>(
        &self,
        organism_id: OrganismId,
        tick: Tick,
        observer: &WorldObject,
        objects: impl Iterator<Item = &'a WorldObject>,
    ) -> Result<PhysicalObservationSnapshot, ScaffoldContractError> {
        let observer_pose = Pose {
            translation: observer.position,
            rotation: Quatf::IDENTITY,
        };
        let observer_velocity = Velocity {
            linear: observer.grounded_physical.velocity,
            angular: Vec3f::ZERO,
        };
        let mut visible = objects
            .filter(|object| object.id != observer.id && !object.consumed)
            .filter_map(|object| {
                let measured_distance = distance(observer.position, object.position);
                (measured_distance <= HEADLESS_VISION_RADIUS).then(|| {
                    Ok((
                        measured_distance,
                        PhysicalObservedObject {
                            transport_entity: object.id,
                            tracking_provenance: object.tracking_provenance,
                            tracking_key: object.tracking_key,
                            position: object.position,
                            properties: object.grounded_physical,
                            contact: measured_distance <= HEADLESS_CONTACT_RADIUS,
                            confidence: Confidence::new(
                                proximity_salience(measured_distance, HEADLESS_VISION_RADIUS)
                                    .max(0.1),
                            )?,
                        },
                    ))
                })
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        visible.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.tracking_key.cmp(&right.1.tracking_key))
        });
        visible.truncate(MAX_VISIBLE_ENTITIES);
        let snapshot = PhysicalObservationSnapshot {
            observer: organism_id,
            tick,
            observer_pose,
            observer_velocity,
            visible: visible.into_iter().map(|(_, object)| object).collect(),
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }

    pub fn perception_frame(
        &mut self,
        organism_id: OrganismId,
        tick: Tick,
        profile: SensorProfile,
        homeostasis: HomeostaticSnapshot,
    ) -> Result<PerceptionFrame, ScaffoldContractError> {
        self.perception_frame_draft(organism_id, tick, profile, homeostasis)?
            .finalize(PerceptionContextBlock::empty())
    }

    pub fn sensory_report(
        &self,
        organism_id: OrganismId,
        tick: Tick,
    ) -> Result<HeadlessSensoryReport, ScaffoldContractError> {
        organism_id.validate()?;
        let agent = self.agent_for(organism_id)?;
        let visible_entities = self.visible_entities_from(agent);
        self.sensory_report_from_visible(organism_id, tick, agent, visible_entities)
    }

    pub fn sensory_report_indexed(
        &self,
        organism_id: OrganismId,
        tick: Tick,
        index: &HeadlessPerceptionBatchIndex,
    ) -> Result<HeadlessSensoryReport, ScaffoldContractError> {
        organism_id.validate()?;
        let agent = self.indexed_agent_for(organism_id, index)?;
        let visible_entities = self.indexed_visible_entities_from(agent, index)?;
        self.sensory_report_from_visible(organism_id, tick, agent, visible_entities)
    }

    fn sensory_report_from_visible(
        &self,
        organism_id: OrganismId,
        tick: Tick,
        agent: &WorldObject,
        visible_entities: Vec<VisibleWorldEntity>,
    ) -> Result<HeadlessSensoryReport, ScaffoldContractError> {
        let contact_entities = visible_entities
            .iter()
            .filter(|visible| visible.distance <= HEADLESS_CONTACT_RADIUS)
            .map(|visible| visible.id)
            .collect::<Vec<_>>();

        let mut visual = [0.0_f32; SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT];
        let mut auditory = [0.0_f32; SENSORY_AUDITORY_CHANNEL_COUNT];
        let mut smell = [0.0_f32; SENSORY_SMELL_CHANNEL_COUNT];
        let mut tactile = [0.0_f32; SENSORY_TACTILE_CHANNEL_COUNT];
        let mut affordances = AffordanceBits::NONE;
        let mut pain = 0.0_f32;
        let mut vocal_tokens = [None; MAX_HEARD_TOKENS];
        let mut social_proximity = [None; MAX_SOCIAL_AGENTS];
        let mut heard_index = 0;
        let mut social_index = 0;
        let mut teacher_channel_marker = None;

        for visible in &visible_entities {
            affordances |= visible.affordances;
            let salience = proximity_salience(visible.distance, HEADLESS_VISION_RADIUS);
            match visible.kind {
                WorldObjectKind::Food => {
                    visual[0] = visual[0].max(salience);
                    smell[0] = smell[0].max(salience);
                }
                WorldObjectKind::Hazard => {
                    visual[1] = visual[1].max(salience);
                    smell[1] = smell[1].max(salience);
                    pain = pain.max(proximity_salience(
                        visible.distance,
                        HEADLESS_CONTACT_RADIUS * 2.0,
                    ));
                }
                WorldObjectKind::Obstacle => {
                    visual[2] = visual[2].max(salience);
                    tactile[0] = tactile[0].max(if visible.distance <= HEADLESS_CONTACT_RADIUS {
                        1.0
                    } else {
                        0.0
                    });
                }
                WorldObjectKind::Agent => {
                    visual[3] = visual[3].max(salience);
                    if social_index < MAX_SOCIAL_AGENTS {
                        let object = self
                            .objects
                            .get(&visible.id.raw())
                            .expect("visible id exists");
                        if let Some(agent_id) = object.organism_id {
                            social_proximity[social_index] = Some(SocialProximityEntry {
                                agent_id,
                                proximity: NormalizedScalar::new(salience)?,
                                confidence: Confidence::new(0.8)?,
                            });
                            social_index += 1;
                        }
                    }
                }
                WorldObjectKind::Token => {
                    visual[7] = visual[7].max(salience);
                    auditory[0] = auditory[0].max(salience);
                    if heard_index < MAX_HEARD_TOKENS && visible.distance <= DEFAULT_HEARING_RADIUS
                    {
                        let object = self
                            .objects
                            .get(&visible.id.raw())
                            .expect("visible id exists");
                        if let Some(token_id) = object.token_id {
                            vocal_tokens[heard_index] = Some(HeardToken {
                                utterance_id: UtteranceId::new(visible.id.raw())?,
                                sequence_position: 0,
                                source_kind: UtteranceSourceKind::Teacher,
                                speaker_id: None,
                                addressee: None,
                                source_entity: Some(visible.id),
                                token_id,
                                source_position: object.position,
                                confidence: Confidence::new(salience.max(0.1))?,
                                teacher_channel: object.teacher_channel,
                            });
                            teacher_channel_marker =
                                teacher_channel_marker.or(object.teacher_channel);
                            heard_index += 1;
                        }
                    }
                }
            }
        }

        for heard in self
            .speech
            .heard_tokens(organism_id, agent.position, tick)?
        {
            if heard_index >= MAX_HEARD_TOKENS {
                break;
            }
            auditory[0] = auditory[0].max(heard.confidence.raw());
            teacher_channel_marker = teacher_channel_marker.or(heard.teacher_channel);
            vocal_tokens[heard_index] = Some(heard);
            heard_index += 1;
        }

        if !contact_entities.is_empty() {
            tactile[1] = 1.0;
        }

        let channels = SensoryChannels::try_from_groups(
            visual,
            auditory,
            smell,
            tactile,
            NormalizedScalar::new(pain.clamp(0.0, 1.0))?,
            NormalizedScalar::new(
                (visible_entities.len() as f32 / MAX_VISIBLE_ENTITIES as f32).clamp(0.0, 1.0),
            )?,
            affordances,
        )?;
        let context_streams = ContextStreams {
            vocal_tokens,
            social_proximity,
            ambient_light: NormalizedScalar::new(self.ambient_light_for_tick())?,
            ..ContextStreams::default()
        };
        context_streams.validate_contract()?;

        let mut core_snapshot =
            SensorySnapshot::new(organism_id, tick, agent.position, channels, context_streams)?;
        core_snapshot.language_context = LanguageContextSnapshot {
            heard_tokens: vocal_tokens,
            word_confidence: Confidence::new(if heard_index > 0 { 0.8 } else { 0.0 })?,
            teacher_channel_marker,
            ..LanguageContextSnapshot::default()
        };
        for (index, entry) in social_proximity.iter().flatten().enumerate() {
            let object = visible_entities
                .iter()
                .filter_map(|visible| self.objects.get(&visible.id.raw()))
                .find(|object| object.organism_id == Some(entry.agent_id))
                .expect("social proximity object exists");
            core_snapshot.social_context.nearest_agents[index] = Some(SocialAgentSnapshot {
                agent_id: entry.agent_id,
                body_entity: Some(object.id),
                relative_position: subtract(object.position, agent.position),
                gaze_direction: Vec3f::new(0.0, 1.0, 0.0),
                orientation_forward: Vec3f::new(0.0, 1.0, 0.0),
                affinity: SignedValence::new(object.social_affinity)?,
                proximity: entry.proximity,
            });
        }
        core_snapshot.validate_contract()?;

        let ecology = self.ecology.sensory_summary(
            agent.position,
            tick,
            self.active_food_count(),
            self.active_hazard_count(),
        );
        ecology.validate()?;
        Ok(HeadlessSensoryReport {
            core_snapshot,
            visible_entities,
            contact_entities,
            touched_entities: self.last_touched_entities.clone(),
            ecology,
        })
    }

    pub fn apply_command(
        &mut self,
        command: &ActionCommand,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        command.validate_contract()?;
        let result = self.execute_command(command)?;
        self.last_touched_entities = result.touched_entities.clone();
        self.last_action_result = Some(result.clone());
        Ok(result)
    }

    /// Executes a bounded factorized command bundle as one world transaction.
    ///
    /// The compatibility adapter may execute several legacy physical actions
    /// in deterministic channel order, but authoritative biology advances only
    /// once and the returned outcome remains joint.
    pub fn apply_registered_motor_bundle(
        &mut self,
        bundle: &MotorCommandBundle,
        world_entity_id: WorldEntityId,
    ) -> Result<HeadlessMotorTransactionReceipt, HeadlessMotorTransactionError> {
        let before = self.clone();
        let result = self.apply_registered_motor_bundle_inner(bundle, world_entity_id, None);
        if let Err(error) = result {
            *self = before;
            return Err(error);
        }
        result
    }

    pub fn apply_registered_motor_bundle_with_neural_emission(
        &mut self,
        bundle: &MotorCommandBundle,
        world_entity_id: WorldEntityId,
        neural: &NeuralEmissionFrame,
    ) -> Result<HeadlessMotorTransactionReceipt, HeadlessMotorTransactionError> {
        let before = self.clone();
        let result =
            self.apply_registered_motor_bundle_inner(bundle, world_entity_id, Some(neural));
        if let Err(error) = result {
            *self = before;
            return Err(error);
        }
        result
    }

    fn apply_registered_motor_bundle_inner(
        &mut self,
        bundle: &MotorCommandBundle,
        world_entity_id: WorldEntityId,
        neural: Option<&NeuralEmissionFrame>,
    ) -> Result<HeadlessMotorTransactionReceipt, HeadlessMotorTransactionError> {
        let outcome_tick = self.validate_registered_motor_bundle(bundle, world_entity_id)?;
        let biology_before = *self
            .organism_registry
            .get(bundle.organism_id)
            .ok_or(ScaffoldContractError::InvalidId)?
            .biochemistry();

        let mut channels = bundle.channels.iter().collect::<Vec<_>>();
        channels.sort_by_key(|command| motor_channel_order(command.channel));
        let mut executed = Vec::with_capacity(channels.len());
        for channel in channels {
            let command = legacy_action_for_motor_channel(bundle.organism_id, channel)?;
            let result = if channel.channel == MotorChannel::Vocal {
                match decode_vocal_channel_payload(channel)? {
                    Some((payload, prompted)) => {
                        self.apply_neural_command(&command, Some(payload), prompted)?
                    }
                    None => self.execute_command(&command)?,
                }
            } else {
                self.execute_command(&command)?
            };
            executed.push((Some(channel.clone()), result));
        }

        let body_event = executed
            .iter()
            .fold(BodyEventDelta::zero(), |total, (_, result)| {
                combine_body_event(total, result.body_event)
            });
        body_event.validate_contract()?;
        if let Some(neural) = neural {
            self.organism_registry
                .advance_biology_with_neural_emission(
                    bundle.organism_id,
                    outcome_tick,
                    body_event,
                    neural,
                )
                .map_err(map_organism_registry_error)?;
        } else {
            self.organism_registry
                .advance_biology(bundle.organism_id, outcome_tick, body_event)
                .map_err(map_organism_registry_error)?;
        }
        self.validate_organism_bindings()?;
        let biology_after = *self
            .organism_registry
            .get(bundle.organism_id)
            .ok_or(ScaffoldContractError::InvalidId)?
            .biochemistry();

        let mut touched = BTreeSet::new();
        let mut channel_observations = Vec::new();
        let mut channel_receipts = Vec::new();
        for (channel, result) in &executed {
            for entity in &result.touched_entities {
                touched.insert(entity.raw());
            }
            let Some(channel) = channel else {
                continue;
            };
            if let Some(observation) = measured_motor_channel_observation(channel, result)? {
                channel_observations.push(observation);
                channel_receipts.push(HeadlessMotorChannelReceipt {
                    command: channel.clone(),
                    observation,
                });
            }
        }
        self.last_touched_entities = touched.into_iter().map(WorldEntityId).collect();
        self.last_action_result = executed.last().map(|(_, result)| result.clone());

        let joint = JointPhysicalOutcome::new(
            aggregate_motor_physical_outcome(&executed)?,
            channel_observations,
        )?;
        let succeeded = executed
            .iter()
            .all(|(_, result)| result.execution.succeeded);

        #[cfg(test)]
        if self.injected_post_action_failure {
            return Err(HeadlessMotorTransactionError::Contract(
                ScaffoldContractError::InvalidDecisionEvidence,
            ));
        }

        Ok(HeadlessMotorTransactionReceipt {
            bundle: bundle.clone(),
            outcome_tick,
            succeeded,
            joint,
            channel_receipts,
            body_event,
            biology_before,
            biology_after,
        })
    }

    fn validate_registered_motor_bundle(
        &self,
        bundle: &MotorCommandBundle,
        world_entity_id: WorldEntityId,
    ) -> Result<Tick, HeadlessMotorTransactionError> {
        bundle.validate_contract()?;
        world_entity_id.validate()?;
        self.validate_organism_bindings()?;
        if bundle.tick != self.tick {
            return Err(HeadlessMotorTransactionError::Contract(
                ScaffoldContractError::NonMonotonicTick,
            ));
        }
        let outcome_tick = self
            .tick
            .raw()
            .checked_add(1)
            .map(Tick::new)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let record = self
            .organism_registry
            .get(bundle.organism_id)
            .ok_or(ScaffoldContractError::InvalidId)?;
        if !record.lifecycle().is_alive()
            || record.world_entity_id() != world_entity_id
            || record.biochemistry().tick != self.tick
        {
            return Err(HeadlessMotorTransactionError::Contract(
                if record.biochemistry().tick != self.tick {
                    ScaffoldContractError::NonMonotonicTick
                } else {
                    ScaffoldContractError::InvalidId
                },
            ));
        }
        let object = self
            .objects
            .get(&world_entity_id.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        if object.kind != WorldObjectKind::Agent
            || object.organism_id != Some(bundle.organism_id)
            || self
                .organism_registry
                .get_by_world_entity_id(world_entity_id)
                .is_none_or(|bound| bound.organism_id() != bundle.organism_id)
        {
            return Err(HeadlessMotorTransactionError::Contract(
                ScaffoldContractError::InvalidId,
            ));
        }
        Ok(outcome_tick)
    }

    pub fn apply_registered_command(
        &mut self,
        command: &ActionCommand,
        world_entity_id: WorldEntityId,
        outcome_tick: Tick,
    ) -> Result<HeadlessActionBiologyReceipt, ScaffoldContractError> {
        self.apply_registered_transaction(
            command,
            world_entity_id,
            outcome_tick,
            RegisteredCommandMode::Legacy,
        )
    }

    pub fn apply_registered_neural_command(
        &mut self,
        command: &ActionCommand,
        world_entity_id: WorldEntityId,
        outcome_tick: Tick,
        speech_payload: Option<SpeechMotorPayload>,
        prompted: bool,
    ) -> Result<HeadlessActionBiologyReceipt, ScaffoldContractError> {
        self.apply_registered_transaction(
            command,
            world_entity_id,
            outcome_tick,
            RegisteredCommandMode::Neural {
                speech_payload,
                prompted,
            },
        )
    }

    fn apply_registered_transaction(
        &mut self,
        command: &ActionCommand,
        world_entity_id: WorldEntityId,
        outcome_tick: Tick,
        mode: RegisteredCommandMode,
    ) -> Result<HeadlessActionBiologyReceipt, ScaffoldContractError> {
        let before = self.clone();
        let result =
            self.apply_registered_transaction_inner(command, world_entity_id, outcome_tick, mode);
        if let Err(error) = result {
            *self = before;
            return Err(error);
        }
        result
    }

    fn apply_registered_transaction_inner(
        &mut self,
        command: &ActionCommand,
        world_entity_id: WorldEntityId,
        outcome_tick: Tick,
        mode: RegisteredCommandMode,
    ) -> Result<HeadlessActionBiologyReceipt, ScaffoldContractError> {
        let biology_before =
            self.validate_registered_action(command, world_entity_id, outcome_tick)?;
        let action_result = match mode {
            RegisteredCommandMode::Legacy => self.apply_command(command)?,
            RegisteredCommandMode::Neural {
                speech_payload,
                prompted,
            } => self.apply_neural_command(command, speech_payload, prompted)?,
        };
        self.organism_registry
            .advance_biology(command.organism_id, outcome_tick, action_result.body_event)
            .map_err(map_organism_registry_error)?;
        self.validate_organism_bindings()?;
        let biology_after = *self
            .organism_registry
            .get(command.organism_id)
            .ok_or(ScaffoldContractError::InvalidId)?
            .biochemistry();
        if biology_after.tick != outcome_tick || action_result.command != *command {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        #[cfg(test)]
        if self.injected_post_action_failure {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(HeadlessActionBiologyReceipt {
            organism_id: command.organism_id,
            world_entity_id,
            outcome_tick,
            action_result,
            biology_before,
            biology_after,
        })
    }

    fn validate_registered_action(
        &self,
        command: &ActionCommand,
        world_entity_id: WorldEntityId,
        outcome_tick: Tick,
    ) -> Result<BiochemistryState, ScaffoldContractError> {
        command.validate_contract()?;
        world_entity_id.validate()?;
        self.validate_organism_bindings()?;
        let expected_tick = self
            .tick
            .raw()
            .checked_add(1)
            .map(Tick::new)
            .ok_or(ScaffoldContractError::InvalidId)?;
        if outcome_tick != expected_tick {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        let record = self
            .organism_registry
            .get(command.organism_id)
            .ok_or(ScaffoldContractError::InvalidId)?;
        if !record.lifecycle().is_alive()
            || record.world_entity_id() != world_entity_id
            || record.biochemistry().tick != self.tick
        {
            return Err(if record.biochemistry().tick != self.tick {
                ScaffoldContractError::NonMonotonicTick
            } else {
                ScaffoldContractError::InvalidId
            });
        }
        let object = self
            .objects
            .get(&world_entity_id.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        if object.kind != WorldObjectKind::Agent
            || object.organism_id != Some(command.organism_id)
            || self
                .organism_registry
                .get_by_world_entity_id(world_entity_id)
                .is_none_or(|bound| bound.organism_id() != command.organism_id)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(*record.biochemistry())
    }

    #[cfg(test)]
    fn inject_post_action_failure(&mut self) {
        self.injected_post_action_failure = true;
    }

    /// Executes a neural action while accepting speech content only from the
    /// GPU-authored payload receipt. A missing payload cannot fall through to
    /// the legacy deterministic token emitter.
    pub fn apply_neural_command(
        &mut self,
        command: &ActionCommand,
        speech_payload: Option<SpeechMotorPayload>,
        prompted: bool,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        command.validate_contract()?;
        if command.kind != ActionKind::Vocalize {
            if speech_payload.is_some() {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
            return self.apply_command(command);
        }
        let Some(payload) = speech_payload else {
            let result = self.finish_action(
                *command,
                false,
                Some(ReferenceActionFailure::ActionRejected),
                physical(PhysicalContactKind::None, None, Vec3f::ZERO, 0.0)?,
                OutcomeProfile::missing_affordance(),
                Vec::new(),
            )?;
            self.last_touched_entities.clear();
            self.last_action_result = Some(result.clone());
            return Ok(result);
        };
        let cooldown = if prompted {
            PROMPTED_SPEECH_COOLDOWN_TICKS
        } else {
            SPONTANEOUS_SPEECH_COOLDOWN_TICKS
        };
        if self
            .last_creature_utterance_ticks
            .get(&command.organism_id.raw())
            .is_some_and(|last| self.tick.raw().saturating_sub(last.raw()) < cooldown)
        {
            let result = self.finish_action(
                *command,
                false,
                Some(ReferenceActionFailure::ActionRejected),
                physical(PhysicalContactKind::None, None, Vec3f::ZERO, 0.0)?,
                OutcomeProfile::missing_affordance(),
                Vec::new(),
            )?;
            self.last_touched_entities.clear();
            self.last_action_result = Some(result.clone());
            return Ok(result);
        }
        let utterance_id = UtteranceId::new(self.next_utterance_id)?;
        self.next_utterance_id = self
            .next_utterance_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let utterance =
            self.emit_creature_utterance(utterance_id, command.organism_id, None, payload)?;
        self.last_creature_utterance_ticks
            .insert(command.organism_id.raw(), self.tick);
        let mut result = self.finish_action(
            *command,
            true,
            None,
            physical(PhysicalContactKind::None, None, Vec3f::ZERO, 0.02)?,
            OutcomeProfile::vocalize(),
            Vec::new(),
        )?;
        result.emitted_utterance = Some(utterance);
        self.last_touched_entities.clear();
        self.last_action_result = Some(result.clone());
        Ok(result)
    }

    fn insert_object(
        &mut self,
        spec: SpawnSpec<'_>,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        spec.position.validate()?;
        if spec.label.is_empty() || self.labels.contains_key(spec.label) {
            return Err(ScaffoldContractError::InvalidId);
        }
        if matches!(spec.kind, WorldObjectKind::Agent)
            && spec.organism_id.is_some_and(|organism_id| {
                self.objects
                    .values()
                    .any(|object| object.organism_id == Some(organism_id))
            })
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        let id = WorldEntityId(self.next_entity_id);
        self.next_entity_id = self
            .next_entity_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let spawn_sequence = self.next_spawn_sequence;
        self.next_spawn_sequence = self
            .next_spawn_sequence
            .checked_add(1)
            .ok_or(ScaffoldContractError::TrackedObjectIdentityExhausted)?;
        let tracking_provenance = PhysicalTrackingProvenance {
            schema_version: PHYSICAL_TRACKING_PROVENANCE_SCHEMA_VERSION,
            world_seed: self.seed,
            zone_id: self
                .ecology
                .zone_at(spec.position)
                .map_or(0, |zone| zone.id.raw()),
            spawn_sequence,
            lineage_key: spec.organism_id.map_or(0, OrganismId::raw),
        };
        tracking_provenance.validate_contract()?;
        let tracking_key = tracking_provenance.canonical_key();
        let object = WorldObject {
            id,
            label: spec.label.to_string(),
            kind: spec.kind,
            organism_id: spec.organism_id,
            position: spec.position,
            radius: HEADLESS_CONTACT_RADIUS,
            nutrition: spec.nutrition.clamp(0.0, 1.0),
            hazard_pain: spec.hazard_pain.clamp(0.0, 1.0),
            token_id: spec.token_id,
            social_affinity: spec.social_affinity.clamp(-1.0, 1.0),
            teacher_channel: spec.teacher_channel,
            consumed: false,
            carried_by: None,
            grounded_physical: GroundedPhysicalProperties::deterministic_default(spawn_sequence),
            tracking_provenance,
            tracking_key,
        };
        self.objects.insert(id.raw(), object);
        self.labels.insert(spec.label.to_string(), id);
        Ok(id)
    }

    fn execute_command(
        &mut self,
        command: &ActionCommand,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        let agent_id = self.agent_entity_id(command.organism_id)?;
        let action = classify_action(command);
        match action {
            HeadlessAction::Idle => self.finish_action(
                *command,
                true,
                None,
                physical(PhysicalContactKind::None, None, Vec3f::ZERO, 0.0)?,
                OutcomeProfile::idle(),
                Vec::new(),
            ),
            HeadlessAction::Rest => self.finish_action(
                *command,
                true,
                None,
                physical(PhysicalContactKind::None, None, Vec3f::ZERO, 0.0)?,
                OutcomeProfile::rest(),
                Vec::new(),
            ),
            HeadlessAction::Inspect => {
                let target = match self.require_target(command) {
                    Ok(target) => target,
                    Err(_) => return self.invalid_target(*command, command.target_entity),
                };
                let profile = self
                    .objects
                    .get(&target.raw())
                    .filter(|object| object.kind == WorldObjectKind::Hazard)
                    .map_or_else(OutcomeProfile::inspect, |object| {
                        OutcomeProfile::hazard(object.hazard_pain)
                    });
                self.finish_action(
                    *command,
                    true,
                    None,
                    physical(PhysicalContactKind::Touch, Some(target), Vec3f::ZERO, 0.02)?,
                    profile,
                    vec![target],
                )
            }
            HeadlessAction::Eat => self.execute_eat(*command),
            HeadlessAction::Move => self.execute_move(*command, agent_id, MoveIntent::Absolute),
            HeadlessAction::Approach => self.execute_move(*command, agent_id, MoveIntent::Approach),
            HeadlessAction::Flee => self.execute_move(*command, agent_id, MoveIntent::Flee),
            HeadlessAction::Grab => {
                let target = match self.require_target(command) {
                    Ok(target) => target,
                    Err(_) => return self.invalid_target(*command, command.target_entity),
                };
                if let Some(object) = self.objects.get_mut(&target.raw()) {
                    object.carried_by = Some(command.organism_id);
                }
                self.finish_action(
                    *command,
                    true,
                    None,
                    physical(PhysicalContactKind::Touch, Some(target), Vec3f::ZERO, 0.06)?,
                    OutcomeProfile::grab(),
                    vec![target],
                )
            }
            HeadlessAction::Vocalize => {
                let token = self.emit_vocalization_token(command.organism_id)?;
                self.finish_action(
                    *command,
                    true,
                    None,
                    physical(PhysicalContactKind::None, Some(token), Vec3f::ZERO, 0.02)?,
                    OutcomeProfile::vocalize(),
                    vec![token],
                )
            }
        }
    }

    fn emit_vocalization_token(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        let position = self.agent_for(organism_id)?.position;
        let label = format!("voice-token-{}", organism_id.raw());
        let token_id = VOCAL_TOKEN_ID_BASE.saturating_add((organism_id.raw() % 10_000) as u32);
        if let Some(existing) = self.labels.get(&label).copied() {
            if let Some(object) = self.objects.get_mut(&existing.raw()) {
                object.position = position;
                object.token_id = Some(token_id);
                object.consumed = false;
            }
            return Ok(existing);
        }
        self.insert_object(SpawnSpec {
            label: &label,
            kind: WorldObjectKind::Token,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: Some(token_id),
            social_affinity: 0.0,
            teacher_channel: None,
        })
    }

    fn execute_eat(
        &mut self,
        command: ActionCommand,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        let target = match self.require_target(&command) {
            Ok(target) => target,
            Err(_) => return self.invalid_target(command, command.target_entity),
        };
        let Some(target_position) = self
            .objects
            .get(&target.raw())
            .map(|object| object.position)
        else {
            return self.invalid_target(command, Some(target));
        };
        let agent = self.agent_for(command.organism_id)?;
        if distance(agent.position, target_position) > EAT_RADIUS {
            return self.finish_action(
                command,
                false,
                Some(ReferenceActionFailure::MissingAffordance),
                physical(
                    PhysicalContactKind::Blocked,
                    Some(target),
                    Vec3f::ZERO,
                    0.04,
                )?,
                OutcomeProfile::missing_affordance(),
                vec![target],
            );
        }
        let Some(object) = self.objects.get_mut(&target.raw()) else {
            return self.invalid_target(command, Some(target));
        };
        if object.kind != WorldObjectKind::Food || object.consumed {
            return self.finish_action(
                command,
                false,
                Some(ReferenceActionFailure::MissingAffordance),
                physical(
                    PhysicalContactKind::Blocked,
                    Some(target),
                    Vec3f::ZERO,
                    0.04,
                )?,
                OutcomeProfile::missing_affordance(),
                vec![target],
            );
        }
        let nutrition = object.nutrition;
        let pain = object.hazard_pain;
        object.consumed = true;
        self.ecology.record_consumed(target, self.tick);
        self.rebuild_ecology_metrics();
        self.finish_action(
            command,
            true,
            None,
            physical(
                PhysicalContactKind::Consumed,
                Some(target),
                Vec3f::ZERO,
                0.03,
            )?,
            OutcomeProfile::food(nutrition, pain),
            vec![target],
        )
    }

    fn execute_move(
        &mut self,
        command: ActionCommand,
        agent_id: WorldEntityId,
        intent: MoveIntent,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        let start = self
            .objects
            .get(&agent_id.raw())
            .expect("agent exists")
            .position;
        let destination = match intent {
            MoveIntent::Absolute => command.target_position.or_else(|| {
                command
                    .target_entity
                    .and_then(|id| self.objects.get(&id.raw()).map(|object| object.position))
            }),
            MoveIntent::Approach => command
                .target_entity
                .and_then(|id| self.objects.get(&id.raw()).map(|object| object.position))
                .map(|target| step_toward(start, target, MOVE_STEP)),
            MoveIntent::Flee => command
                .target_entity
                .and_then(|id| self.objects.get(&id.raw()).map(|object| object.position))
                .map(|target| step_away(start, target, MOVE_STEP)),
        };
        let Some(destination) = destination else {
            return self.invalid_target(command, command.target_entity);
        };
        destination.validate()?;
        if let Some(blocker) = self.blocking_object_at(destination) {
            return self.finish_action(
                command,
                false,
                Some(ReferenceActionFailure::Blocked),
                physical(
                    PhysicalContactKind::Blocked,
                    Some(blocker),
                    Vec3f::ZERO,
                    0.08,
                )?,
                OutcomeProfile::blocked(),
                vec![blocker],
            );
        }

        let touched = self
            .objects
            .iter()
            .filter(|(id, object)| {
                **id != agent_id.raw()
                    && !object.consumed
                    && distance(object.position, destination) <= HEADLESS_CONTACT_RADIUS
            })
            .map(|(id, _)| WorldEntityId(*id))
            .collect::<Vec<_>>();
        let hazard = touched.iter().find_map(|id| {
            self.objects
                .get(&id.raw())
                .filter(|object| object.kind == WorldObjectKind::Hazard)
                .map(|object| (*id, object.hazard_pain))
        });
        let displacement = subtract(destination, start);
        if let Some(agent) = self.objects.get_mut(&agent_id.raw()) {
            agent.position = destination;
            agent.grounded_physical.velocity = displacement;
        }
        let zone_hazard = self
            .ecology
            .zone_at(destination)
            .map_or(0.0, |zone| zone.hazard_pressure);
        let agent_contact = touched.iter().find_map(|id| {
            self.objects
                .get(&id.raw())
                .filter(|object| object.kind == WorldObjectKind::Agent)
                .map(|object| (*id, object.social_affinity))
        });
        let (mut profile, contact, target) = if let Some((hazard_id, pain)) = hazard {
            (
                OutcomeProfile::hazard(pain),
                PhysicalContactKind::Collision,
                Some(hazard_id),
            )
        } else if zone_hazard > 0.0 {
            (
                OutcomeProfile::hazard(zone_hazard),
                PhysicalContactKind::Moved,
                command.target_entity,
            )
        } else if matches!(intent, MoveIntent::Absolute) {
            if let Some((agent_id, affinity)) = agent_contact {
                (
                    OutcomeProfile::social_contact(affinity),
                    PhysicalContactKind::Collision,
                    Some(agent_id),
                )
            } else {
                (
                    OutcomeProfile::movement(),
                    PhysicalContactKind::Moved,
                    command.target_entity,
                )
            }
        } else if let Some((_agent_id, affinity)) = agent_contact {
            (
                OutcomeProfile::movement().with_social_contact(affinity),
                PhysicalContactKind::Moved,
                command.target_entity,
            )
        } else {
            (
                OutcomeProfile::movement(),
                PhysicalContactKind::Moved,
                command.target_entity,
            )
        };
        if (hazard.is_some() || zone_hazard > 0.0) && agent_contact.is_some() {
            if let Some((_, affinity)) = agent_contact {
                profile = profile.with_social_contact(affinity);
            }
        }
        self.finish_action(
            command,
            true,
            None,
            physical(contact, target, displacement, 0.08)?,
            profile,
            touched,
        )
    }

    fn finish_action(
        &self,
        command: ActionCommand,
        succeeded: bool,
        failure: Option<ReferenceActionFailure>,
        physical: PhysicalActionOutcome,
        profile: OutcomeProfile,
        touched_entities: Vec<WorldEntityId>,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        let execution = if succeeded {
            ReferenceActionExecution::succeeded(physical)?
        } else {
            ReferenceActionExecution::failed(
                failure.unwrap_or(ReferenceActionFailure::ActionRejected),
                physical,
            )?
        };
        let mut observation = ReferenceOutcomeObservation::new(
            succeeded,
            profile.homeostatic_delta,
            SignedValence::ZERO,
            NormalizedScalar::new(profile.frustration)?,
            NormalizedScalar::new(profile.pain)?,
            SignedValence::new(profile.energy)?,
            NormalizedScalar::new(profile.prediction_error)?,
        )?;
        observation.contradiction_observed = profile.contradiction || !succeeded;
        Ok(HeadlessActionResult {
            command,
            execution,
            observation,
            body_event: profile.body_event,
            touched_entities,
            emitted_utterance: None,
        })
    }

    fn require_target(
        &self,
        command: &ActionCommand,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        let Some(target) = command.target_entity else {
            return Err(ScaffoldContractError::InvalidId);
        };
        if self.objects.contains_key(&target.raw()) {
            Ok(target)
        } else {
            Err(ScaffoldContractError::InvalidId)
        }
    }

    fn invalid_target(
        &self,
        command: ActionCommand,
        target: Option<WorldEntityId>,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        self.finish_action(
            command,
            false,
            Some(ReferenceActionFailure::ActionRejected),
            physical(PhysicalContactKind::Blocked, target, Vec3f::ZERO, 0.03)?,
            OutcomeProfile::invalid_target(),
            target.into_iter().collect(),
        )
    }

    fn agent_entity_id(
        &self,
        organism_id: OrganismId,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        self.objects
            .iter()
            .find_map(|(id, object)| {
                (object.organism_id == Some(organism_id)).then_some(WorldEntityId(*id))
            })
            .ok_or(ScaffoldContractError::InvalidId)
    }

    fn agent_for(&self, organism_id: OrganismId) -> Result<&WorldObject, ScaffoldContractError> {
        let id = self.agent_entity_id(organism_id)?;
        self.objects
            .get(&id.raw())
            .ok_or(ScaffoldContractError::InvalidId)
    }

    fn indexed_agent_for(
        &self,
        organism_id: OrganismId,
        index: &HeadlessPerceptionBatchIndex,
    ) -> Result<&WorldObject, ScaffoldContractError> {
        self.validate_perception_batch_index(index)?;
        let entity = index
            .organism_entities
            .get(&organism_id.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        self.objects
            .get(entity)
            .ok_or(ScaffoldContractError::InvalidId)
    }

    fn validate_perception_batch_index(
        &self,
        index: &HeadlessPerceptionBatchIndex,
    ) -> Result<(), ScaffoldContractError> {
        if index.world_seed != self.seed
            || index.world_tick != self.tick
            || index.object_count != self.objects.len()
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Ok(())
    }

    fn indexed_nearby_objects<'a>(
        &'a self,
        observer: &WorldObject,
        index: &HeadlessPerceptionBatchIndex,
    ) -> Result<Vec<&'a WorldObject>, ScaffoldContractError> {
        self.validate_perception_batch_index(index)?;
        let (cell_x, cell_y, cell_z) = perception_cell(observer.position);
        let mut nearby = Vec::new();
        for x in cell_x.saturating_sub(1)..=cell_x.saturating_add(1) {
            for y in cell_y.saturating_sub(1)..=cell_y.saturating_add(1) {
                for z in cell_z.saturating_sub(1)..=cell_z.saturating_add(1) {
                    if let Some(ids) = index.cells.get(&(x, y, z)) {
                        for id in ids {
                            nearby.push(
                                self.objects
                                    .get(id)
                                    .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?,
                            );
                        }
                    }
                }
            }
        }
        Ok(nearby)
    }

    fn visible_entities_from(&self, observer: &WorldObject) -> Vec<VisibleWorldEntity> {
        Self::visible_entities_from_objects(observer, self.objects.values())
    }

    fn indexed_visible_entities_from(
        &self,
        observer: &WorldObject,
        index: &HeadlessPerceptionBatchIndex,
    ) -> Result<Vec<VisibleWorldEntity>, ScaffoldContractError> {
        Ok(Self::visible_entities_from_objects(
            observer,
            self.indexed_nearby_objects(observer, index)?.into_iter(),
        ))
    }

    fn visible_entities_from_objects<'a>(
        observer: &WorldObject,
        objects: impl Iterator<Item = &'a WorldObject>,
    ) -> Vec<VisibleWorldEntity> {
        let mut visible = objects
            .filter(|object| object.id != observer.id && !object.consumed)
            .filter_map(|object| {
                let distance = distance(observer.position, object.position);
                (distance <= HEADLESS_VISION_RADIUS).then_some(VisibleWorldEntity {
                    id: object.id,
                    kind: object.kind,
                    relative_position: subtract(object.position, observer.position),
                    distance,
                    affordances: object.affordances(),
                })
            })
            .collect::<Vec<_>>();
        visible.sort_by(|a, b| {
            a.distance
                .total_cmp(&b.distance)
                .then_with(|| a.id.raw().cmp(&b.id.raw()))
        });
        visible.truncate(MAX_VISIBLE_ENTITIES);
        visible
    }

    fn blocking_object_at(&self, position: Vec3f) -> Option<WorldEntityId> {
        self.objects.iter().find_map(|(id, object)| {
            object
                .blocks_position(position)
                .then_some(WorldEntityId(*id))
        })
    }

    fn advance_ecology_at_current_tick(&mut self) -> EcologyStepReport {
        let mut report = EcologyStepReport {
            tick: self.tick,
            ..EcologyStepReport::default()
        };
        if self.ecology.zones.is_empty() && self.ecology.resources.is_empty() {
            self.rebuild_ecology_metrics();
            report.metrics = self.ecology.metrics();
            return report;
        }

        let resource_ids = self
            .ecology
            .resources
            .iter()
            .map(|resource| resource.object_id)
            .collect::<Vec<_>>();
        for object_id in resource_ids {
            let Some(object) = self.objects.get_mut(&object_id.raw()) else {
                report.cap_rejections = report.cap_rejections.saturating_add(1);
                continue;
            };
            let Some(resource) = self.ecology.resource_by_object_mut(object_id) else {
                continue;
            };
            if object.consumed {
                if let Some(consumed_at) = resource.consumed_at_tick {
                    let elapsed = self.tick.raw().saturating_sub(consumed_at.raw());
                    if elapsed >= resource.regrow_after_ticks as u64 {
                        object.consumed = false;
                        object.nutrition = resource.base_nutrition.clamp(0.0, 1.0);
                        resource.last_regrown_tick = Some(self.tick);
                        resource.consumed_at_tick = None;
                        resource.low_salience_marker = false;
                        report.regrown_entities.push(object_id);
                        self.ecology.metrics.resources_regrown =
                            self.ecology.metrics.resources_regrown.saturating_add(1);
                    }
                }
            } else if let Some(regrown_at) = resource.last_regrown_tick {
                let elapsed = self.tick.raw().saturating_sub(regrown_at.raw());
                if elapsed >= resource.decay_after_ticks as u64 {
                    resource.low_salience_marker = true;
                    report.cleanup_marked_entities.push(object_id);
                    self.ecology.metrics.cleanup_marked =
                        self.ecology.metrics.cleanup_marked.saturating_add(1);
                }
            }
        }

        self.spawn_ecology_resources(&mut report);
        self.rebuild_ecology_metrics();
        report.metrics = self.ecology.metrics();
        report
    }

    fn spawn_ecology_resources(&mut self, report: &mut EcologyStepReport) {
        let mut spawned_this_tick = 0_usize;
        let policies = self.ecology.spawn_policies.clone();
        for (policy_index, mut policy) in policies.into_iter().enumerate() {
            if spawned_this_tick >= self.ecology.config.max_spawn_per_tick {
                break;
            }
            if self.tick.raw() < policy.next_spawn_tick.raw() {
                continue;
            }
            let active_for_prefix = self
                .objects
                .values()
                .filter(|object| {
                    object.kind == WorldObjectKind::Food
                        && !object.consumed
                        && object.label.starts_with(&policy.label_prefix)
                })
                .count();
            if active_for_prefix >= policy.max_active
                || self.objects.len() >= self.ecology.config.max_world_objects
                || self.ecology.resources.len() >= self.ecology.config.max_resource_records
            {
                report.cap_rejections = report.cap_rejections.saturating_add(1);
                self.ecology.metrics.cap_rejections =
                    self.ecology.metrics.cap_rejections.saturating_add(1);
                continue;
            }
            let Some(zone) = self.ecology.zone(policy.zone_id).cloned() else {
                report.cap_rejections = report.cap_rejections.saturating_add(1);
                continue;
            };
            let label = format!("{}-{}", policy.label_prefix, policy.spawned_count);
            let position = deterministic_zone_position(&zone, policy.spawned_count);
            let Ok(id) = self.insert_object(SpawnSpec {
                label: &label,
                kind: WorldObjectKind::Food,
                organism_id: None,
                position,
                nutrition: policy.nutrition,
                hazard_pain: 0.0,
                token_id: None,
                social_affinity: 0.0,
                teacher_channel: None,
            }) else {
                report.cap_rejections = report.cap_rejections.saturating_add(1);
                continue;
            };
            let lifecycle = ResourceLifecycle {
                object_id: id,
                home_zone: policy.zone_id,
                base_nutrition: policy.nutrition.clamp(0.0, 1.0),
                regrow_after_ticks: policy.interval_ticks.max(1),
                decay_after_ticks: policy.interval_ticks.saturating_mul(3).max(1),
                consumed_at_tick: None,
                last_regrown_tick: Some(self.tick),
                low_salience_marker: false,
            };
            if self.ecology.add_resource(lifecycle).is_err() {
                if let Some(removed) = self.objects.remove(&id.raw()) {
                    self.labels.remove(&removed.label);
                }
                report.cap_rejections = report.cap_rejections.saturating_add(1);
                continue;
            }
            policy.spawned_count = policy.spawned_count.saturating_add(1);
            policy.next_spawn_tick =
                Tick::new(self.tick.raw().saturating_add(policy.interval_ticks as u64));
            if let Some(slot) = self.ecology.spawn_policies.get_mut(policy_index) {
                *slot = policy;
            }
            report.spawned_labels.push(label);
            spawned_this_tick += 1;
            self.ecology.metrics.resources_spawned =
                self.ecology.metrics.resources_spawned.saturating_add(1);
        }
    }

    fn active_food_count(&self) -> usize {
        self.objects
            .values()
            .filter(|object| object.kind == WorldObjectKind::Food && !object.consumed)
            .count()
    }

    fn active_hazard_count(&self) -> usize {
        self.objects
            .values()
            .filter(|object| object.kind == WorldObjectKind::Hazard && !object.consumed)
            .count()
    }

    fn rebuild_ecology_metrics(&mut self) {
        let object_kinds = self
            .objects
            .values()
            .filter(|object| matches!(object.kind, WorldObjectKind::Food | WorldObjectKind::Hazard))
            .map(|object| {
                (
                    object.id.raw(),
                    (object.kind == WorldObjectKind::Food, object.consumed),
                )
            })
            .collect::<BTreeMap<_, _>>();
        self.ecology.rebuild_metrics(&object_kinds);
    }

    fn ambient_light_for_tick(&self) -> f32 {
        let phase = crate::ecology::cycle_phase(self.tick, self.ecology.config.cycle_length_ticks);
        if phase < 0.5 {
            0.85
        } else {
            0.35
        }
    }
}

fn write_world_object_signature(
    digest: &mut CanonicalDigestBuilder,
    object: &WorldObject,
) -> Result<(), ScaffoldContractError> {
    validate_persisted_object(object)?;
    digest.write_u64(object.id.raw());
    digest.write_utf8(&object.label);
    digest.write_u8(match object.kind {
        WorldObjectKind::Agent => 0,
        WorldObjectKind::Food => 1,
        WorldObjectKind::Hazard => 2,
        WorldObjectKind::Obstacle => 3,
        WorldObjectKind::Token => 4,
    });
    write_optional_u64(digest, object.organism_id.map(OrganismId::raw));
    write_vec3_bits(digest, object.position);
    write_f32_bits(digest, object.radius);
    write_f32_bits(digest, object.nutrition);
    write_f32_bits(digest, object.hazard_pain);
    write_optional_u32(digest, object.token_id);
    write_f32_bits(digest, object.social_affinity);
    match object.teacher_channel {
        Some(channel) => {
            digest.write_some();
            digest.write_u8(channel.raw());
        }
        None => digest.write_none(),
    }
    digest.write_bool(object.consumed);
    write_optional_u64(digest, object.carried_by.map(OrganismId::raw));

    let physical = object.grounded_physical;
    write_vec3_bits(digest, physical.velocity);
    for value in physical
        .color
        .into_iter()
        .chain(physical.material)
        .chain(physical.shape)
        .chain(physical.chemical)
    {
        write_f32_bits(digest, value);
    }
    write_f32_bits(digest, physical.surface_temperature);
    for value in physical.terrain {
        write_f32_bits(digest, value);
    }

    let tracking = object.tracking_provenance;
    digest.write_u16(tracking.schema_version);
    digest.write_u64(tracking.world_seed);
    digest.write_u32(tracking.zone_id);
    digest.write_u64(tracking.spawn_sequence);
    digest.write_u64(tracking.lineage_key);
    for word in object.tracking_key.0 {
        digest.write_u64(word);
    }
    Ok(())
}

fn write_f32_bits(digest: &mut CanonicalDigestBuilder, value: f32) {
    digest.write_u32(value.to_bits());
}

fn write_vec3_bits(digest: &mut CanonicalDigestBuilder, value: Vec3f) {
    write_f32_bits(digest, value.x);
    write_f32_bits(digest, value.y);
    write_f32_bits(digest, value.z);
}

fn write_optional_u32(digest: &mut CanonicalDigestBuilder, value: Option<u32>) {
    match value {
        Some(value) => {
            digest.write_some();
            digest.write_u32(value);
        }
        None => digest.write_none(),
    }
}

fn write_optional_u64(digest: &mut CanonicalDigestBuilder, value: Option<u64>) {
    match value {
        Some(value) => {
            digest.write_some();
            digest.write_u64(value);
        }
        None => digest.write_none(),
    }
}

fn grounded_identity_signature(object: &WorldObject) -> String {
    let physical = object.grounded_physical;
    let provenance = object.tracking_provenance;
    format!(
        concat!(
            "physical=",
            "{:.6},{:.6},{:.6};",
            "{:.6},{:.6},{:.6};",
            "{:.6},{:.6},{:.6};",
            "{:.6},{:.6},{:.6};",
            "{:.6},{:.6},{:.6};",
            "{:.6};{:.6},{:.6}|",
            "tracking={},{},{},{}"
        ),
        physical.velocity.x,
        physical.velocity.y,
        physical.velocity.z,
        physical.color[0],
        physical.color[1],
        physical.color[2],
        physical.material[0],
        physical.material[1],
        physical.material[2],
        physical.shape[0],
        physical.shape[1],
        physical.shape[2],
        physical.chemical[0],
        physical.chemical[1],
        physical.chemical[2],
        physical.surface_temperature,
        physical.terrain[0],
        physical.terrain[1],
        provenance.schema_version,
        provenance.zone_id,
        provenance.spawn_sequence,
        provenance.lineage_key,
    )
}

#[derive(Debug)]
pub struct HeadlessScenarioBuilder {
    world: HeadlessWorld,
    rng: DeterministicRng,
    error: Option<ScaffoldContractError>,
}

impl HeadlessScenarioBuilder {
    pub fn new(seed: u64) -> Self {
        Self {
            world: HeadlessWorld::new(seed),
            rng: DeterministicRng::new(seed),
            error: None,
        }
    }

    pub fn agent(mut self, label: &str, organism_id: OrganismId, position: Vec3f) -> Self {
        self.insert(SpawnSpec {
            label,
            kind: WorldObjectKind::Agent,
            organism_id: Some(organism_id),
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: None,
            social_affinity: 0.0,
            teacher_channel: None,
        });
        self
    }

    pub fn social_agent(
        mut self,
        label: &str,
        organism_id: OrganismId,
        position: Vec3f,
        affinity: f32,
    ) -> Self {
        self.insert(SpawnSpec {
            label,
            kind: WorldObjectKind::Agent,
            organism_id: Some(organism_id),
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: None,
            social_affinity: affinity.clamp(-1.0, 1.0),
            teacher_channel: None,
        });
        self
    }

    pub fn food(mut self, label: &str, position: Vec3f, nutrition: f32) -> Self {
        self.insert(SpawnSpec {
            label,
            kind: WorldObjectKind::Food,
            organism_id: None,
            position,
            nutrition,
            hazard_pain: 0.0,
            token_id: None,
            social_affinity: 0.0,
            teacher_channel: None,
        });
        self
    }

    pub fn hazard(mut self, label: &str, position: Vec3f, pain: f32) -> Self {
        self.insert(SpawnSpec {
            label,
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: pain,
            token_id: None,
            social_affinity: 0.0,
            teacher_channel: None,
        });
        self
    }

    pub fn obstacle(mut self, label: &str, position: Vec3f, radius: f32) -> Self {
        self.insert(SpawnSpec {
            label,
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: None,
            social_affinity: 0.0,
            teacher_channel: None,
        });
        if let Some(id) = self.world.entity_id(label) {
            if let Some(object) = self.world.objects.get_mut(&id.raw()) {
                object.radius = radius.max(0.1);
            }
        }
        self
    }

    pub fn token(mut self, label: &str, position: Vec3f, token_id: u32) -> Self {
        self.insert(SpawnSpec {
            label,
            kind: WorldObjectKind::Token,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: Some(token_id),
            social_affinity: 0.0,
            teacher_channel: None,
        });
        self
    }

    pub fn teacher_token(
        mut self,
        label: &str,
        position: Vec3f,
        token_id: u32,
        teacher_channel: TeacherPerceptionChannel,
    ) -> Self {
        self.insert(SpawnSpec {
            label,
            kind: WorldObjectKind::Token,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: Some(token_id),
            social_affinity: 0.0,
            teacher_channel: Some(teacher_channel),
        });
        self
    }

    pub fn grounded_physical(
        mut self,
        label: &str,
        properties: GroundedPhysicalProperties,
    ) -> Self {
        if self.error.is_some() {
            return self;
        }
        let Some(id) = self.world.entity_id(label) else {
            self.error = Some(ScaffoldContractError::InvalidId);
            return self;
        };
        if let Err(error) = self.world.set_grounded_physical_properties(id, properties) {
            self.error = Some(error);
        }
        self
    }

    pub fn ecology_config(mut self, config: EcologyConfig) -> Self {
        if self.error.is_some() {
            return self;
        }
        match self.world.ecology.clone().with_config(config) {
            Ok(ecology) => {
                if let Err(error) = self.world.configure_ecology(ecology) {
                    self.error = Some(error);
                }
            }
            Err(error) => self.error = Some(error),
        }
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn terrain_zone(
        mut self,
        id: u32,
        label: &str,
        kind: TerrainZoneKind,
        center: Vec3f,
        radius: f32,
        resource_bias: f32,
        hazard_pressure: f32,
    ) -> Self {
        if self.error.is_some() {
            return self;
        }
        match TerrainZone::new(
            EcologyZoneId(id),
            label,
            kind,
            center,
            radius,
            resource_bias,
            hazard_pressure,
        ) {
            Ok(zone) => {
                if let Err(error) = self.world.add_terrain_zone(zone) {
                    self.error = Some(error);
                }
            }
            Err(error) => self.error = Some(error),
        }
        self
    }

    pub fn track_resource(
        mut self,
        label: &str,
        zone_id: u32,
        regrow_after_ticks: u32,
        decay_after_ticks: u32,
    ) -> Self {
        if self.error.is_some() {
            return self;
        }
        let Some(object_id) = self.world.entity_id(label) else {
            self.error = Some(ScaffoldContractError::InvalidId);
            return self;
        };
        if let Err(error) = self.world.track_resource_lifecycle(
            object_id,
            EcologyZoneId(zone_id),
            regrow_after_ticks,
            decay_after_ticks,
        ) {
            self.error = Some(error);
        }
        self
    }

    pub fn resource_spawn_policy(
        mut self,
        label_prefix: &str,
        zone_id: u32,
        interval_ticks: u32,
        max_active: usize,
        nutrition: f32,
    ) -> Self {
        if self.error.is_some() {
            return self;
        }
        let policy = ResourceSpawnPolicy {
            label_prefix: label_prefix.to_string(),
            zone_id: EcologyZoneId(zone_id),
            interval_ticks,
            max_active,
            nutrition,
            next_spawn_tick: self.world.tick(),
            spawned_count: 0,
        };
        if let Err(error) = self.world.add_resource_spawn_policy(policy) {
            self.error = Some(error);
        }
        self
    }

    pub fn random_food(mut self, label: &str, nutrition: f32) -> Self {
        let position = self.random_position();
        self.insert(SpawnSpec {
            label,
            kind: WorldObjectKind::Food,
            organism_id: None,
            position,
            nutrition,
            hazard_pain: 0.0,
            token_id: None,
            social_affinity: 0.0,
            teacher_channel: None,
        });
        self
    }

    pub fn random_hazard(mut self, label: &str, pain: f32) -> Self {
        let position = self.random_position();
        self.insert(SpawnSpec {
            label,
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: pain,
            token_id: None,
            social_affinity: 0.0,
            teacher_channel: None,
        });
        self
    }

    pub fn build(self) -> Result<HeadlessWorld, ScaffoldContractError> {
        if let Some(error) = self.error {
            Err(error)
        } else {
            Ok(self.world)
        }
    }

    fn insert(&mut self, spec: SpawnSpec<'_>) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.world.insert_object(spec) {
            self.error = Some(error);
        }
    }

    fn random_position(&mut self) -> Vec3f {
        let x = self.rng.next_range(-3.0, 3.0);
        let y = self.rng.next_range(-3.0, 3.0);
        Vec3f::new(x, y, 0.0)
    }
}

pub struct HeadlessWorldCommand;

impl HeadlessWorldCommand {
    pub fn eat(
        organism_id: OrganismId,
        target: WorldEntityId,
    ) -> Result<ActionCommand, ScaffoldContractError> {
        Self::structured(
            organism_id,
            HeadlessActionIds::EAT,
            ActionKind::Interact,
            Some(target),
            None,
        )
    }

    pub fn approach(
        organism_id: OrganismId,
        target: WorldEntityId,
    ) -> Result<ActionCommand, ScaffoldContractError> {
        Self::structured(
            organism_id,
            HeadlessActionIds::APPROACH,
            ActionKind::Move,
            Some(target),
            None,
        )
    }

    pub fn rest(organism_id: OrganismId) -> Result<ActionCommand, ScaffoldContractError> {
        Self::structured(
            organism_id,
            ActionKind::Rest.canonical_id(),
            ActionKind::Rest,
            None,
            None,
        )
    }

    pub fn vocalize(organism_id: OrganismId) -> Result<ActionCommand, ScaffoldContractError> {
        Self::structured(
            organism_id,
            ActionKind::Vocalize.canonical_id(),
            ActionKind::Vocalize,
            None,
            None,
        )
    }

    pub fn idle(organism_id: OrganismId) -> Result<ActionCommand, ScaffoldContractError> {
        Self::structured(
            organism_id,
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            None,
            None,
        )
    }

    fn structured(
        organism_id: OrganismId,
        action_id: ActionId,
        kind: ActionKind,
        target_entity: Option<WorldEntityId>,
        target_position: Option<Vec3f>,
    ) -> Result<ActionCommand, ScaffoldContractError> {
        ActionCommand::structured(
            organism_id,
            action_id,
            kind,
            alife_core::ActionTarget::new(target_entity, target_position),
            Intensity::new(1.0)?,
            alife_core::DurationTicks::new(1),
            Confidence::new(0.9)?,
            0,
            None,
            None,
            None,
        )
    }
}

#[derive(Debug, Clone)]
pub struct HeadlessBrainHarness {
    world: Rc<RefCell<HeadlessWorld>>,
    telemetry: HeadlessTelemetry,
}

impl HeadlessBrainHarness {
    pub fn new(world: HeadlessWorld) -> Self {
        Self {
            world: Rc::new(RefCell::new(world)),
            telemetry: HeadlessTelemetry::default(),
        }
    }

    pub fn world(&self) -> Ref<'_, HeadlessWorld> {
        self.world.borrow()
    }

    pub const fn telemetry(&self) -> &HeadlessTelemetry {
        &self.telemetry
    }

    pub fn spawn_social_agent(
        &mut self,
        label: &str,
        organism_id: OrganismId,
        position: Vec3f,
        affinity: f32,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        self.world
            .borrow_mut()
            .spawn_social_agent(label, organism_id, position, affinity)
    }

    pub fn remove_agent_entity(
        &mut self,
        id: WorldEntityId,
    ) -> Result<WorldObject, ScaffoldContractError> {
        self.world.borrow_mut().remove_agent_entity(id)
    }

    pub fn tick_mind(
        &mut self,
        mind: &mut alife_core::CreatureMind,
        input: BrainTickInput,
    ) -> HeadlessBrainTick {
        let mut sensory = SharedSensoryAdapter {
            world: Rc::clone(&self.world),
        };
        let mut executor = SharedActionExecutor {
            world: Rc::clone(&self.world),
        };
        let mut observer = SharedOutcomeObserver {
            world: Rc::clone(&self.world),
        };
        let brain = mind.tick(input, &mut sensory, &mut executor, &mut observer);
        if let Some(patch) = &brain.experience_patch {
            self.telemetry.sealed_patches.push(patch.clone());
        }
        if let Some(record) = &brain.packed_record {
            self.telemetry.packed_records.push(record.clone());
        }
        let action_result = if brain.selected_action.is_some() {
            self.world.borrow().last_action_result.clone()
        } else {
            None
        };
        self.world.borrow_mut().advance_tick();
        HeadlessBrainTick {
            brain,
            action_result,
            sleep_transition: None,
            sleep_report: None,
        }
    }
}

impl crate::ActionLegalityChecker for HeadlessWorld {
    fn check_action(&self, action: &ActionCommand) -> crate::ActionLegality {
        if action
            .target_entity
            .is_some_and(|id| !self.objects.contains_key(&id.raw()))
        {
            return crate::ActionLegality::ImpossibleTarget;
        }
        if classify_action(action) == HeadlessAction::Eat {
            if let Some(target) = action.target_entity {
                let Some(object) = self.objects.get(&target.raw()) else {
                    return crate::ActionLegality::ImpossibleTarget;
                };
                if object.kind != WorldObjectKind::Food || object.consumed {
                    return crate::ActionLegality::BlockedByWorldState;
                }
            }
        }
        crate::ActionLegality::Legal
    }
}

#[derive(Clone)]
struct SharedSensoryAdapter {
    world: Rc<RefCell<HeadlessWorld>>,
}

impl ReferenceSensoryAdapter for SharedSensoryAdapter {
    fn gather_sensory(
        &mut self,
        request: ReferenceSensoryRequest,
    ) -> Result<SensorySnapshot, ScaffoldContractError> {
        self.world
            .borrow()
            .sensory_report(request.organism_id, request.tick)
            .map(|report| report.core_snapshot)
    }
}

#[derive(Clone)]
struct SharedActionExecutor {
    world: Rc<RefCell<HeadlessWorld>>,
}

impl ReferenceActionExecutor for SharedActionExecutor {
    fn execute_action(
        &mut self,
        command: &ActionCommand,
    ) -> Result<ReferenceActionExecution, ScaffoldContractError> {
        self.world
            .borrow_mut()
            .apply_command(command)
            .map(|result| result.execution)
    }
}

#[derive(Clone)]
struct SharedOutcomeObserver {
    world: Rc<RefCell<HeadlessWorld>>,
}

impl ReferenceOutcomeObserver for SharedOutcomeObserver {
    fn observe_outcome(
        &mut self,
        request: ReferenceOutcomeRequest<'_>,
    ) -> Result<ReferenceOutcomeObservation, ScaffoldContractError> {
        let world = self.world.borrow();
        let Some(result) = &world.last_action_result else {
            return Err(ScaffoldContractError::InvalidActionDecision);
        };
        if result.command.action_id != request.command.action_id
            || result.command.target_entity != request.command.target_entity
            || result.execution != *request.execution
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        Ok(result.observation.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeadlessAction {
    Idle,
    Rest,
    Inspect,
    Move,
    Approach,
    Flee,
    Eat,
    Grab,
    Vocalize,
}

enum RegisteredCommandMode {
    Legacy,
    Neural {
        speech_payload: Option<SpeechMotorPayload>,
        prompted: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoveIntent {
    Absolute,
    Approach,
    Flee,
}

fn classify_action(command: &ActionCommand) -> HeadlessAction {
    if command.action_id == HeadlessActionIds::EAT {
        HeadlessAction::Eat
    } else if command.action_id == HeadlessActionIds::APPROACH {
        HeadlessAction::Approach
    } else if command.action_id == HeadlessActionIds::FLEE {
        HeadlessAction::Flee
    } else if command.action_id == HeadlessActionIds::GRAB {
        HeadlessAction::Grab
    } else {
        match command.kind {
            ActionKind::Idle => HeadlessAction::Idle,
            ActionKind::Rest => HeadlessAction::Rest,
            ActionKind::Inspect => HeadlessAction::Inspect,
            ActionKind::Move => HeadlessAction::Move,
            ActionKind::Hold | ActionKind::Interact => HeadlessAction::Grab,
            ActionKind::Vocalize | ActionKind::Write | ActionKind::Gesture => {
                HeadlessAction::Vocalize
            }
        }
    }
}

fn motor_channel_order(channel: MotorChannel) -> u16 {
    match channel {
        MotorChannel::Locomotion => 0,
        MotorChannel::Orientation => 1,
        MotorChannel::Manipulation => 2,
        MotorChannel::Vocal => 3,
        MotorChannel::Posture => 4,
        MotorChannel::SpeciesSpecific(id) => 0x100 + u16::from(id),
    }
}

const VOCAL_CHANNEL_PAYLOAD_MAGIC_V1: u32 = 0x5348_5031;

fn decode_vocal_channel_payload(
    command: &ChannelCommand,
) -> Result<Option<(SpeechMotorPayload, bool)>, ScaffoldContractError> {
    let values = &command.payload.values;
    if values.is_empty() {
        return Ok(None);
    }
    if values.len() < 5 || values[0] != VOCAL_CHANNEL_PAYLOAD_MAGIC_V1 {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let speech_act = SpeechActKind::try_from_raw(
        u8::try_from(values[1]).map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?,
    )?;
    let prompted = match values[2] {
        0 => false,
        1 => true,
        _ => return Err(ScaffoldContractError::InvalidDecisionEvidence),
    };
    let confidence = Confidence::new(values[3] as f32 / 65_535.0)?;
    let tokens = values[4..]
        .iter()
        .map(|raw| {
            LanguageTokenId::new(
                u16::try_from(*raw).map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    SpeechMotorPayload::try_new(speech_act, tokens, confidence)
        .map(|payload| Some((payload, prompted)))
}

fn legacy_action_for_motor_channel(
    organism_id: OrganismId,
    command: &ChannelCommand,
) -> Result<ActionCommand, HeadlessMotorTransactionError> {
    let (action_id, kind, target) = match command.channel {
        MotorChannel::Locomotion => (
            if command.primitive == HeadlessActionIds::APPROACH
                || command.primitive == HeadlessActionIds::FLEE
            {
                command.primitive
            } else {
                ActionKind::Move.canonical_id()
            },
            ActionKind::Move,
            command
                .target
                .unwrap_or_else(|| alife_core::ActionTarget::new(None, Some(command.direction))),
        ),
        MotorChannel::Manipulation => (
            if command.primitive == HeadlessActionIds::EAT
                || command.primitive == HeadlessActionIds::GRAB
            {
                command.primitive
            } else {
                ActionKind::Interact.canonical_id()
            },
            if command.primitive == HeadlessActionIds::GRAB {
                ActionKind::Hold
            } else {
                ActionKind::Interact
            },
            command
                .target
                .unwrap_or_else(|| alife_core::ActionTarget::new(None, Some(command.direction))),
        ),
        MotorChannel::Vocal => (
            ActionKind::Vocalize.canonical_id(),
            ActionKind::Vocalize,
            alife_core::ActionTarget::new(None, None),
        ),
        MotorChannel::Posture => (
            ActionKind::Rest.canonical_id(),
            ActionKind::Rest,
            alife_core::ActionTarget::new(None, None),
        ),
        MotorChannel::Orientation | MotorChannel::SpeciesSpecific(_) => {
            return Err(HeadlessMotorTransactionError::UnsupportedChannel(
                command.channel,
            ));
        }
    };
    Ok(ActionCommand::structured(
        organism_id,
        action_id,
        kind,
        target,
        command.intensity,
        command.duration_ticks,
        command.confidence,
        0,
        None,
        None,
        None,
    )?)
}

fn combine_body_event(total: BodyEventDelta, event: BodyEventDelta) -> BodyEventDelta {
    BodyEventDelta {
        energy: (total.energy + event.energy).clamp(-1.0, 1.0),
        damage: (total.damage + event.damage).clamp(0.0, 1.0),
        temperature_stress: (total.temperature_stress + event.temperature_stress).clamp(0.0, 1.0),
        nutrition: (total.nutrition + event.nutrition).clamp(0.0, 1.0),
        social_contact: (total.social_contact + event.social_contact).clamp(0.0, 1.0),
        sleep_recovery: (total.sleep_recovery + event.sleep_recovery).clamp(0.0, 1.0),
        mating_opportunity: (total.mating_opportunity + event.mating_opportunity).clamp(0.0, 1.0),
    }
}

fn measured_motor_channel_observation(
    command: &ChannelCommand,
    result: &HeadlessActionResult,
) -> Result<Option<MeasuredChannelObservation>, ScaffoldContractError> {
    if command.channel != MotorChannel::Locomotion {
        return Ok(None);
    }
    let displacement = result.execution.physical.displacement;
    let measured_intensity = (distance(Vec3f::ZERO, displacement) / MOVE_STEP).clamp(0.0, 1.0);
    Ok(Some(MeasuredChannelObservation::new(
        command.channel,
        result.execution.succeeded,
        NormalizedScalar::new(measured_intensity)?,
        displacement,
    )?))
}

fn aggregate_motor_physical_outcome(
    executed: &[(Option<ChannelCommand>, HeadlessActionResult)],
) -> Result<PhysicalActionOutcome, ScaffoldContractError> {
    let mut displacement = Vec3f::ZERO;
    let mut contact = PhysicalContactKind::None;
    let mut target_entity = None;
    let mut collision_normal = None;
    let mut contact_priority = 0;
    let mut energy_cost = 0.0;

    for (_, result) in executed {
        let physical = result.execution.physical;
        displacement = Vec3f::new(
            displacement.x + physical.displacement.x,
            displacement.y + physical.displacement.y,
            displacement.z + physical.displacement.z,
        );
        energy_cost = (energy_cost + physical.energy_cost.raw()).clamp(0.0, 1.0);
        let priority = physical_contact_priority(physical.contact);
        if priority >= contact_priority && (priority > 0 || physical.target_entity.is_some()) {
            contact = physical.contact;
            target_entity = physical.target_entity;
            collision_normal = physical.collision_normal;
            contact_priority = priority;
        }
    }

    let outcome = PhysicalActionOutcome {
        contact,
        target_entity,
        displacement,
        collision_normal,
        energy_cost: NormalizedScalar::new(energy_cost)?,
    };
    outcome.validate_contract()?;
    Ok(outcome)
}

fn physical_contact_priority(contact: PhysicalContactKind) -> u8 {
    match contact {
        PhysicalContactKind::None => 0,
        PhysicalContactKind::Moved => 1,
        PhysicalContactKind::Touch => 2,
        PhysicalContactKind::Consumed => 3,
        PhysicalContactKind::Blocked => 4,
        PhysicalContactKind::Collision => 5,
    }
}

fn validate_persisted_object(object: &WorldObject) -> Result<(), ScaffoldContractError> {
    object.id.validate()?;
    if object.label.is_empty() {
        return Err(ScaffoldContractError::InvalidId);
    }
    if let Some(organism_id) = object.organism_id {
        organism_id.validate()?;
    }
    if let Some(carried_by) = object.carried_by {
        carried_by.validate()?;
    }
    object.position.validate()?;
    object.grounded_physical.validate_contract()?;
    object.tracking_provenance.validate_contract()?;
    if object.tracking_key != object.tracking_provenance.canonical_key() {
        return Err(ScaffoldContractError::InvalidId);
    }
    if !object.radius.is_finite() || object.radius <= 0.0 {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    for value in [object.nutrition, object.hazard_pain, object.social_affinity] {
        if !value.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
    }
    if !(0.0..=1.0).contains(&object.nutrition)
        || !(0.0..=1.0).contains(&object.hazard_pain)
        || !(-1.0..=1.0).contains(&object.social_affinity)
    {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct OutcomeProfile {
    homeostatic_delta: HomeostaticDelta,
    frustration: f32,
    pain: f32,
    energy: f32,
    prediction_error: f32,
    contradiction: bool,
    body_event: BodyEventDelta,
}

impl OutcomeProfile {
    fn idle() -> Self {
        Self::new(
            DriveDelta::zero(),
            EndocrineDelta::zero(),
            0.0,
            0.0,
            -0.01,
            0.05,
            false,
        )
    }

    fn rest() -> Self {
        Self::new(
            DriveDelta {
                fatigue: -0.35,
                brain_atp: 0.12,
                ..DriveDelta::zero()
            },
            EndocrineDelta {
                sleep_pressure: -0.1,
                serotonin: 0.05,
                ..EndocrineDelta::zero()
            },
            0.0,
            0.0,
            0.08,
            0.05,
            false,
        )
        .with_body_event(BodyEventDelta {
            sleep_recovery: 1.0,
            ..BodyEventDelta::zero()
        })
    }

    fn inspect() -> Self {
        Self::new(
            DriveDelta {
                curiosity: -0.03,
                brain_atp: -0.01,
                ..DriveDelta::zero()
            },
            EndocrineDelta::zero(),
            0.0,
            0.0,
            -0.01,
            0.1,
            false,
        )
    }

    fn food(nutrition: f32, pain: f32) -> Self {
        let nutrition = nutrition.clamp(0.0, 1.0);
        let pain = pain.clamp(0.0, 1.0);
        let energy = (nutrition * 0.5 - pain * 0.25).clamp(-1.0, 1.0);
        Self::new(
            DriveDelta {
                hunger: -nutrition,
                fear: pain * 0.5,
                pain: pain * 0.8,
                brain_atp: nutrition * 0.35,
                ..DriveDelta::zero()
            },
            EndocrineDelta {
                dopamine: (0.15 - pain * 0.2).clamp(-1.0, 1.0),
                cortisol: pain * 0.7,
                serotonin: 0.05,
                ..EndocrineDelta::zero()
            },
            0.0,
            pain,
            energy,
            (0.05 + pain * 0.75).clamp(0.0, 1.0),
            pain > 0.0,
        )
        .with_body_event(BodyEventDelta {
            energy,
            nutrition,
            damage: pain,
            ..BodyEventDelta::zero()
        })
    }

    fn movement() -> Self {
        Self::new(
            DriveDelta {
                brain_atp: -0.04,
                curiosity: 0.01,
                ..DriveDelta::zero()
            },
            EndocrineDelta::zero(),
            0.0,
            0.0,
            -0.04,
            0.08,
            false,
        )
    }

    fn hazard(pain: f32) -> Self {
        let pain = pain.clamp(0.0, 1.0);
        Self::new(
            DriveDelta {
                fear: pain * 0.45,
                pain,
                brain_atp: -0.08,
                ..DriveDelta::zero()
            },
            EndocrineDelta {
                adrenaline: pain * 0.25,
                cortisol: pain * 0.2,
                dopamine: -0.1,
                ..EndocrineDelta::zero()
            },
            0.25,
            pain,
            -0.08,
            0.8,
            true,
        )
        .with_body_event(BodyEventDelta {
            damage: pain,
            ..BodyEventDelta::zero()
        })
    }

    fn blocked() -> Self {
        Self::new(
            DriveDelta {
                pain: 0.05,
                brain_atp: -0.03,
                ..DriveDelta::zero()
            },
            EndocrineDelta {
                cortisol: 0.08,
                ..EndocrineDelta::zero()
            },
            0.45,
            0.05,
            -0.03,
            0.6,
            true,
        )
    }

    fn missing_affordance() -> Self {
        Self::new(
            DriveDelta {
                curiosity: 0.1,
                brain_atp: -0.02,
                ..DriveDelta::zero()
            },
            EndocrineDelta {
                cortisol: 0.1,
                dopamine: -0.05,
                ..EndocrineDelta::zero()
            },
            0.65,
            0.0,
            -0.02,
            0.85,
            true,
        )
    }

    fn invalid_target() -> Self {
        Self::new(
            DriveDelta {
                curiosity: 0.05,
                brain_atp: -0.01,
                ..DriveDelta::zero()
            },
            EndocrineDelta {
                cortisol: 0.08,
                ..EndocrineDelta::zero()
            },
            0.7,
            0.0,
            -0.01,
            0.9,
            true,
        )
    }

    fn grab() -> Self {
        Self::new(
            DriveDelta {
                brain_atp: -0.03,
                ..DriveDelta::zero()
            },
            EndocrineDelta::zero(),
            0.0,
            0.0,
            -0.03,
            0.1,
            false,
        )
    }

    fn vocalize() -> Self {
        Self::new(
            DriveDelta {
                loneliness: -0.02,
                brain_atp: -0.01,
                ..DriveDelta::zero()
            },
            EndocrineDelta {
                oxytocin: 0.03,
                ..EndocrineDelta::zero()
            },
            0.0,
            0.0,
            -0.01,
            0.1,
            false,
        )
    }

    fn social_contact(affinity: f32) -> Self {
        let affinity = affinity.clamp(-1.0, 1.0);
        if affinity >= 0.0 {
            Self::new(
                DriveDelta {
                    loneliness: -0.08 * affinity,
                    brain_atp: -0.02,
                    ..DriveDelta::zero()
                },
                EndocrineDelta {
                    oxytocin: 0.08 * affinity,
                    serotonin: 0.03 * affinity,
                    ..EndocrineDelta::zero()
                },
                0.02,
                0.0,
                -0.02,
                0.15,
                false,
            )
            .with_body_event(BodyEventDelta {
                social_contact: affinity.abs(),
                ..BodyEventDelta::zero()
            })
        } else {
            let fear = affinity.abs();
            Self::new(
                DriveDelta {
                    fear: 0.18 * fear,
                    pain: 0.02 * fear,
                    brain_atp: -0.04,
                    ..DriveDelta::zero()
                },
                EndocrineDelta {
                    adrenaline: 0.12 * fear,
                    cortisol: 0.10 * fear,
                    oxytocin: -0.04 * fear,
                    ..EndocrineDelta::zero()
                },
                0.20 * fear,
                0.02 * fear,
                -0.04,
                0.35,
                true,
            )
            .with_body_event(BodyEventDelta {
                social_contact: fear,
                ..BodyEventDelta::zero()
            })
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        drives: DriveDelta,
        hormones: EndocrineDelta,
        frustration: f32,
        pain: f32,
        energy: f32,
        prediction_error: f32,
        contradiction: bool,
    ) -> Self {
        Self {
            homeostatic_delta: HomeostaticDelta { drives, hormones },
            frustration,
            pain,
            energy,
            prediction_error,
            contradiction,
            body_event: BodyEventDelta {
                energy,
                ..BodyEventDelta::zero()
            },
        }
    }

    fn with_body_event(mut self, body_event: BodyEventDelta) -> Self {
        self.body_event = BodyEventDelta {
            energy: self.body_event.energy,
            ..body_event
        };
        self
    }

    fn with_social_contact(mut self, affinity: f32) -> Self {
        self.body_event.social_contact = affinity.clamp(-1.0, 1.0).abs();
        self
    }
}

#[derive(Debug, Clone, Copy)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    const fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0xA5A5_5A5A_D3C1_B2E1,
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    fn next_unit(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_unit()
    }
}

fn physical(
    contact: PhysicalContactKind,
    target_entity: Option<WorldEntityId>,
    displacement: Vec3f,
    energy_cost: f32,
) -> Result<PhysicalActionOutcome, ScaffoldContractError> {
    let outcome = PhysicalActionOutcome {
        contact,
        target_entity,
        displacement,
        collision_normal: None,
        energy_cost: NormalizedScalar::new(energy_cost.clamp(0.0, 1.0))?,
    };
    outcome.validate_contract()?;
    Ok(outcome)
}

fn distance(a: Vec3f, b: Vec3f) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn perception_cell(position: Vec3f) -> (i32, i32, i32) {
    let scale = 1.0 / HEADLESS_VISION_RADIUS;
    (
        (position.x * scale).floor() as i32,
        (position.y * scale).floor() as i32,
        (position.z * scale).floor() as i32,
    )
}

fn subtract(a: Vec3f, b: Vec3f) -> Vec3f {
    Vec3f::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn proximity_salience(distance: f32, radius: f32) -> f32 {
    if radius <= 0.0 {
        return 0.0;
    }
    (1.0 - distance / radius).clamp(0.0, 1.0)
}

fn step_toward(start: Vec3f, target: Vec3f, step: f32) -> Vec3f {
    let delta = subtract(target, start);
    let length = distance(start, target);
    if length <= step || length == 0.0 {
        target
    } else {
        Vec3f::new(
            start.x + delta.x / length * step,
            start.y + delta.y / length * step,
            start.z + delta.z / length * step,
        )
    }
}

fn step_away(start: Vec3f, target: Vec3f, step: f32) -> Vec3f {
    let delta = subtract(start, target);
    let length = distance(start, target);
    if length == 0.0 {
        Vec3f::new(start.x + step, start.y, start.z)
    } else {
        Vec3f::new(
            start.x + delta.x / length * step,
            start.y + delta.y / length * step,
            start.z + delta.z / length * step,
        )
    }
}

#[cfg(test)]
mod task_6_factorized_motor_tests {
    use super::*;

    const ORGANISM_ID: OrganismId = OrganismId(7);

    fn record(world_entity_id: WorldEntityId) -> WorldOrganismRecord {
        let genome = alife_core::CreatureGenome::early_mammal_founder(
            0xE10_3601,
            alife_core::FoundationGeneticIdentity::new(
                10,
                1,
                7,
                alife_core::BrainCapacityClass::N512_ID,
            )
            .unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let biochemistry = alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        WorldOrganismRecord::new(
            ORGANISM_ID,
            world_entity_id,
            genome,
            phenotype,
            biochemistry,
            Tick::ZERO,
        )
        .unwrap()
    }

    fn prepared_world() -> (
        HeadlessWorld,
        WorldEntityId,
        WorldEntityId,
        MotorCommandBundle,
    ) {
        let mut world = HeadlessScenarioBuilder::new(36_001)
            .agent("agent", ORGANISM_ID, Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        let agent = world.entity_id("agent").unwrap();
        let food = world.entity_id("food").unwrap();
        world.register_organism_record(record(agent)).unwrap();

        let manipulation = ChannelCommand::new(
            MotorChannel::Manipulation,
            HeadlessActionIds::EAT,
            Some(alife_core::ActionTarget::new(Some(food), None)),
            Vec3f::ZERO,
            Intensity::new(1.0).unwrap(),
            alife_core::DurationTicks::new(1),
            0.0,
            Confidence::new(0.9).unwrap(),
            0,
        )
        .unwrap();
        let locomotion = ChannelCommand::new(
            MotorChannel::Locomotion,
            HeadlessActionIds::APPROACH,
            Some(alife_core::ActionTarget::new(Some(food), None)),
            Vec3f::new(1.0, 0.0, 0.0),
            Intensity::new(1.0).unwrap(),
            alife_core::DurationTicks::new(1),
            0.0,
            Confidence::new(0.9).unwrap(),
            0,
        )
        .unwrap();
        let bundle = MotorCommandBundle::new(
            ORGANISM_ID,
            alife_core::ExperienceSequenceId::new(1).unwrap(),
            Tick::ZERO,
            vec![manipulation, locomotion],
        )
        .unwrap();
        (world, agent, food, bundle)
    }

    #[test]
    fn factorized_bundle_executes_as_one_joint_transaction_and_rolls_back() {
        let (mut world, agent, food, bundle) = prepared_world();
        let biology_before = *world
            .organism_registry()
            .get(ORGANISM_ID)
            .unwrap()
            .biochemistry();

        let receipt = world.apply_registered_motor_bundle(&bundle, agent).unwrap();

        assert_eq!(receipt.bundle, bundle);
        assert_eq!(receipt.outcome_tick, Tick::new(1));
        assert!(receipt.succeeded);
        assert_eq!(receipt.joint.joint_reward(), None);
        assert_eq!(receipt.joint.channel_observations.len(), 1);
        assert_eq!(receipt.channel_receipts.len(), 1);
        assert_eq!(
            receipt.channel_receipts[0].command.channel,
            MotorChannel::Locomotion
        );
        assert_eq!(receipt.biology_after.tick, Tick::new(1));
        assert_eq!(
            receipt.biology_after,
            biology_before
                .advance(
                    Tick::new(1),
                    receipt.body_event,
                    world
                        .organism_registry()
                        .get(ORGANISM_ID)
                        .unwrap()
                        .phenotype(),
                )
                .unwrap()
        );
        assert!(world.entity(food).unwrap().is_consumed());

        let (mut unsupported_world, unsupported_agent, _, _) = prepared_world();
        let unsupported_command = |channel| {
            ChannelCommand::new(
                channel,
                HeadlessActionIds::EAT,
                None,
                Vec3f::ZERO,
                Intensity::new(1.0).unwrap(),
                alife_core::DurationTicks::new(1),
                0.0,
                Confidence::new(0.9).unwrap(),
                0,
            )
            .unwrap()
        };
        let unsupported_bundle = MotorCommandBundle::new(
            ORGANISM_ID,
            alife_core::ExperienceSequenceId::new(2).unwrap(),
            Tick::ZERO,
            vec![unsupported_command(MotorChannel::Orientation)],
        )
        .unwrap();
        let unsupported_signature = unsupported_world.canonical_signature_digest().unwrap();
        let unsupported_objects = unsupported_world.object_snapshots();
        assert_eq!(
            unsupported_world.apply_registered_motor_bundle(&unsupported_bundle, unsupported_agent),
            Err(HeadlessMotorTransactionError::UnsupportedChannel(
                MotorChannel::Orientation,
            ))
        );
        assert_eq!(
            unsupported_world.canonical_signature_digest().unwrap(),
            unsupported_signature
        );
        assert_eq!(unsupported_world.object_snapshots(), unsupported_objects);
        assert_eq!(unsupported_world.tick(), Tick::ZERO);

        let species_bundle = MotorCommandBundle::new(
            ORGANISM_ID,
            alife_core::ExperienceSequenceId::new(3).unwrap(),
            Tick::ZERO,
            vec![unsupported_command(MotorChannel::SpeciesSpecific(9))],
        )
        .unwrap();
        assert_eq!(
            unsupported_world.apply_registered_motor_bundle(&species_bundle, unsupported_agent),
            Err(HeadlessMotorTransactionError::UnsupportedChannel(
                MotorChannel::SpeciesSpecific(9),
            ))
        );
        assert_eq!(
            unsupported_world.canonical_signature_digest().unwrap(),
            unsupported_signature
        );
        assert_eq!(unsupported_world.object_snapshots(), unsupported_objects);

        let (mut domain_world, domain_agent, domain_food, _) = prepared_world();
        let locomotion_with_eat_primitive = ChannelCommand::new(
            MotorChannel::Locomotion,
            HeadlessActionIds::EAT,
            Some(alife_core::ActionTarget::new(Some(domain_food), None)),
            Vec3f::new(1.0, 0.0, 0.0),
            Intensity::new(1.0).unwrap(),
            alife_core::DurationTicks::new(1),
            0.0,
            Confidence::new(0.9).unwrap(),
            0,
        )
        .unwrap();
        let domain_bundle = MotorCommandBundle::new(
            ORGANISM_ID,
            alife_core::ExperienceSequenceId::new(4).unwrap(),
            Tick::ZERO,
            vec![locomotion_with_eat_primitive],
        )
        .unwrap();
        let domain_receipt = domain_world
            .apply_registered_motor_bundle(&domain_bundle, domain_agent)
            .unwrap();
        assert_eq!(domain_receipt.bundle, domain_bundle);
        assert_eq!(domain_receipt.channel_receipts.len(), 1);
        assert_eq!(
            domain_receipt.channel_receipts[0].command.channel,
            MotorChannel::Locomotion
        );
        assert!(!domain_world.entity(domain_food).unwrap().is_consumed());

        let (mut failing_world, failing_agent, _, failing_bundle) = prepared_world();
        let before_signature = failing_world.canonical_signature_digest().unwrap();
        let before_objects = failing_world.object_snapshots();
        let before_record = failing_world
            .organism_registry()
            .get(ORGANISM_ID)
            .unwrap()
            .clone();
        failing_world.inject_post_action_failure();

        assert_eq!(
            failing_world.apply_registered_motor_bundle(&failing_bundle, failing_agent),
            Err(HeadlessMotorTransactionError::Contract(
                ScaffoldContractError::InvalidDecisionEvidence,
            ))
        );
        assert_eq!(
            failing_world.canonical_signature_digest().unwrap(),
            before_signature
        );
        assert_eq!(failing_world.object_snapshots(), before_objects);
        assert_eq!(
            failing_world.organism_registry().get(ORGANISM_ID),
            Some(&before_record)
        );
        assert_eq!(failing_world.tick(), Tick::ZERO);
    }

    #[test]
    fn factorized_bundle_causally_couples_neural_emission_into_organism_chemistry() {
        let (mut world, agent, _, bundle) = prepared_world();
        let before = *world
            .organism_registry()
            .get(ORGANISM_ID)
            .unwrap()
            .biochemistry();
        let neural = alife_core::NeuralEmissionFrame::new(
            Tick::ZERO,
            1,
            vec![alife_core::NeuralEmission::new(
                alife_core::NeuralEmissionClass::ExecutiveSustain,
                0.9,
                0.9,
            )
            .unwrap()],
        )
        .unwrap();

        let receipt = world
            .apply_registered_motor_bundle_with_neural_emission(&bundle, agent, &neural)
            .unwrap();

        assert_ne!(receipt.biology_after.graph_state(), before.graph_state());
        assert_eq!(
            receipt.biology_after.biochemical_work().neural_emitter_evaluations,
            1
        );
        assert!(
            receipt.biology_after.homeostasis.hormones.acetylcholine
                > before.homeostasis.hormones.acetylcholine
        );
    }
}

#[cfg(test)]
mod task_3_2a_tests {
    use super::*;

    const ORGANISM_ID: OrganismId = OrganismId(7);

    fn record(world_entity_id: WorldEntityId) -> WorldOrganismRecord {
        let genome = alife_core::CreatureGenome::early_mammal_founder(
            0xE10_32A1,
            alife_core::FoundationGeneticIdentity::new(
                10,
                1,
                7,
                alife_core::BrainCapacityClass::N512_ID,
            )
            .unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let biochemistry = alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        WorldOrganismRecord::new(
            ORGANISM_ID,
            world_entity_id,
            genome,
            phenotype,
            biochemistry,
            Tick::ZERO,
        )
        .unwrap()
    }

    #[test]
    fn inspect_preserves_safe_semantics_and_applies_target_hazard() {
        let mut safe_world = HeadlessScenarioBuilder::new(32_021)
            .agent("agent", ORGANISM_ID, Vec3f::ZERO)
            .obstacle("safe-target", Vec3f::new(1.0, 0.0, 0.0), 0.5)
            .build()
            .unwrap();
        let safe_agent = safe_world.entity_id("agent").unwrap();
        let safe_target = safe_world.entity_id("safe-target").unwrap();
        safe_world
            .register_organism_record(record(safe_agent))
            .unwrap();
        let safe_command = HeadlessWorldCommand::structured(
            ORGANISM_ID,
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            Some(safe_target),
            None,
        )
        .unwrap();

        let safe_receipt = safe_world
            .apply_registered_neural_command(&safe_command, safe_agent, Tick(1), None, false)
            .unwrap();

        assert!(safe_receipt.action_result.observation.success);
        assert_eq!(
            safe_receipt.action_result.observation.reward_valence.raw(),
            0.05
        );
        assert_eq!(safe_receipt.action_result.observation.pain_delta.raw(), 0.0);
        assert_eq!(
            safe_receipt
                .action_result
                .observation
                .homeostatic_delta
                .drives
                .curiosity,
            -0.03
        );
        assert_eq!(safe_receipt.action_result.body_event.damage, 0.0);
        assert_eq!(safe_receipt.biology_after.tick, Tick(1));

        let hazard_pain = 0.8;
        let mut hazard_world = HeadlessScenarioBuilder::new(32_022)
            .agent("agent", ORGANISM_ID, Vec3f::ZERO)
            .hazard("hazard-target", Vec3f::new(1.0, 0.0, 0.0), hazard_pain)
            .build()
            .unwrap();
        let hazard_agent = hazard_world.entity_id("agent").unwrap();
        let hazard_target = hazard_world.entity_id("hazard-target").unwrap();
        hazard_world
            .register_organism_record(record(hazard_agent))
            .unwrap();
        let hazard_command = HeadlessWorldCommand::structured(
            ORGANISM_ID,
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            Some(hazard_target),
            None,
        )
        .unwrap();

        let hazard_receipt = hazard_world
            .apply_registered_neural_command(&hazard_command, hazard_agent, Tick(1), None, false)
            .unwrap();

        assert!(hazard_receipt.action_result.observation.success);
        assert!(
            hazard_receipt
                .action_result
                .observation
                .reward_valence
                .raw()
                <= 0.0
        );
        assert_eq!(
            hazard_receipt.action_result.observation.pain_delta.raw(),
            hazard_pain
        );
        assert_eq!(
            hazard_receipt
                .action_result
                .observation
                .homeostatic_delta
                .drives
                .pain,
            hazard_pain
        );
        assert_eq!(hazard_receipt.action_result.body_event.damage, hazard_pain);
        assert_eq!(hazard_receipt.biology_after.tick, Tick(1));
        assert_ne!(hazard_receipt.biology_after, hazard_receipt.biology_before);
    }

    #[test]
    fn registered_transaction_rolls_back_late_failure_completely() {
        let mut world = HeadlessScenarioBuilder::new(32_005)
            .agent("agent", ORGANISM_ID, Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        let agent = world.entity_id("agent").unwrap();
        let food = world.entity_id("food").unwrap();
        world.register_organism_record(record(agent)).unwrap();
        world
            .apply_command(&HeadlessWorldCommand::idle(ORGANISM_ID).unwrap())
            .unwrap();
        world
            .emit_player_tokens(
                None,
                Vec3f::ZERO,
                vec![alife_core::LanguageTokenId::new(7).unwrap()],
            )
            .unwrap();

        let before_signature = world.canonical_signature_digest().unwrap();
        let before_objects = world.object_snapshots();
        let before_touched = world.last_touched_entities.clone();
        let before_last_action = world.last_action_result.clone();
        let before_speech = world.speech.snapshot();
        let before_record = world.organism_registry().get(ORGANISM_ID).unwrap().clone();
        world.inject_post_action_failure();

        let result = world.apply_registered_command(
            &HeadlessWorldCommand::eat(ORGANISM_ID, food).unwrap(),
            agent,
            Tick(1),
        );

        assert_eq!(result, Err(ScaffoldContractError::InvalidDecisionEvidence));
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            before_signature
        );
        assert_eq!(world.object_snapshots(), before_objects);
        assert_eq!(world.last_touched_entities, before_touched);
        assert_eq!(world.last_action_result, before_last_action);
        assert_eq!(world.speech.snapshot(), before_speech);
        assert_eq!(
            world.organism_registry().get(ORGANISM_ID),
            Some(&before_record)
        );
    }

    #[test]
    fn registered_neural_vocalize_failure_rolls_back_speech_and_world_state() {
        let mut world = HeadlessScenarioBuilder::new(32_014)
            .agent("agent", ORGANISM_ID, Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        let agent = world.entity_id("agent").unwrap();
        world.register_organism_record(record(agent)).unwrap();
        world
            .apply_command(&HeadlessWorldCommand::idle(ORGANISM_ID).unwrap())
            .unwrap();
        world
            .emit_player_tokens(
                None,
                Vec3f::ZERO,
                vec![alife_core::LanguageTokenId::new(7).unwrap()],
            )
            .unwrap();
        let command = HeadlessWorldCommand::vocalize(ORGANISM_ID).unwrap();
        let payload = alife_core::SpeechMotorPayload::try_new(
            alife_core::SpeechActKind::Declare,
            vec![alife_core::LanguageTokenId::new(9).unwrap()],
            alife_core::Confidence::new(0.9).unwrap(),
        )
        .unwrap();

        let before_signature = world.canonical_signature_digest().unwrap();
        let before_objects = world.object_snapshots();
        let before_speech = world.audible_utterances();
        let before_cooldown = world.last_creature_utterance_ticks.clone();
        let before_next_utterance_id = world.next_utterance_id;
        let before_touched = world.last_touched_entities.clone();
        let before_last_action = world.last_action_result.clone();
        let before_record = world.organism_registry().get(ORGANISM_ID).unwrap().clone();
        world.inject_post_action_failure();

        let result = world.apply_registered_neural_command(
            &command,
            agent,
            Tick(1),
            Some(payload.clone()),
            false,
        );

        assert_eq!(result, Err(ScaffoldContractError::InvalidDecisionEvidence));
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            before_signature
        );
        assert_eq!(world.object_snapshots(), before_objects);
        assert_eq!(world.audible_utterances(), before_speech);
        assert_eq!(world.last_creature_utterance_ticks, before_cooldown);
        assert_eq!(world.next_utterance_id, before_next_utterance_id);
        assert_eq!(world.last_touched_entities, before_touched);
        assert_eq!(world.last_action_result, before_last_action);
        assert_eq!(
            world.organism_registry().get(ORGANISM_ID),
            Some(&before_record)
        );
        assert!(world.entity_id("voice-token-7").is_none());
        assert!(world
            .audible_utterances()
            .iter()
            .flat_map(|utterance| utterance.tokens.iter())
            .all(|token| token.raw() != 9));

        world.injected_post_action_failure = false;
        let retry = world
            .apply_registered_neural_command(&command, agent, Tick(1), Some(payload), false)
            .unwrap();
        assert_eq!(
            retry
                .action_result
                .emitted_utterance
                .as_ref()
                .unwrap()
                .utterance_id,
            UtteranceId::new(2).unwrap()
        );
        assert_eq!(world.next_utterance_id, 3);
        assert_eq!(
            world.last_creature_utterance_ticks.get(&ORGANISM_ID.raw()),
            Some(&Tick::ZERO)
        );
        assert_eq!(retry.biology_after.tick, Tick(1));
    }

    const TASK_4_1_LOW_ORGANISM: OrganismId = OrganismId(7);
    const TASK_4_1_HIGH_ORGANISM: OrganismId = OrganismId(19);

    fn task_4_1_record(
        organism_id: OrganismId,
        world_entity_id: WorldEntityId,
    ) -> WorldOrganismRecord {
        let genome = alife_core::CreatureGenome::early_mammal_founder(
            0xE10_4100 + organism_id.raw(),
            alife_core::FoundationGeneticIdentity::new(
                10,
                1,
                7,
                alife_core::BrainCapacityClass::N512_ID,
            )
            .unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let biochemistry = alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        WorldOrganismRecord::new(
            organism_id,
            world_entity_id,
            genome,
            phenotype,
            biochemistry,
            Tick::ZERO,
        )
        .unwrap()
    }

    fn task_4_1_world_with_registration_order(order: &[OrganismId]) -> HeadlessWorld {
        let mut world = HeadlessScenarioBuilder::new(41_001)
            .agent("agent-low", TASK_4_1_LOW_ORGANISM, Vec3f::ZERO)
            .agent(
                "agent-high",
                TASK_4_1_HIGH_ORGANISM,
                Vec3f::new(0.5, 0.0, 0.0),
            )
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.6)
            .terrain_zone(
                1,
                "meadow",
                TerrainZoneKind::Meadow,
                Vec3f::ZERO,
                3.0,
                0.8,
                0.0,
            )
            .track_resource("food", 1, 1, 4)
            .build()
            .unwrap();

        let low_entity = world.entity_id("agent-low").unwrap();
        let high_entity = world.entity_id("agent-high").unwrap();
        for organism_id in order {
            let world_entity_id = match *organism_id {
                TASK_4_1_LOW_ORGANISM => low_entity,
                TASK_4_1_HIGH_ORGANISM => high_entity,
                _ => unreachable!("fixture only registers the two named organisms"),
            };
            world
                .register_organism_record(task_4_1_record(*organism_id, world_entity_id))
                .unwrap();
        }
        world
    }

    fn task_4_1_consume_and_prepare_resource(world: &mut HeadlessWorld) -> WorldEntityId {
        let food = world.entity_id("food").unwrap();
        let result = world
            .apply_command(&HeadlessWorldCommand::eat(TASK_4_1_LOW_ORGANISM, food).unwrap())
            .unwrap();
        assert!(result.execution.succeeded);
        assert_eq!(result.body_event.nutrition, 0.6);
        assert!(world.entity(food).unwrap().is_consumed());

        world
            .emit_player_tokens(
                None,
                Vec3f::new(-1.0, 0.0, 0.0),
                vec![alife_core::LanguageTokenId::new(42).unwrap()],
            )
            .unwrap();
        food
    }

    fn task_4_1_record_state(
        world: &HeadlessWorld,
        organism_id: OrganismId,
    ) -> WorldOrganismRecord {
        world.organism_registry().get(organism_id).unwrap().clone()
    }

    #[test]
    fn try_advance_tick_updates_all_registered_biology_and_resource_lifecycle_in_stable_order_and_rolls_back_late_failure(
    ) {
        let mut forward = task_4_1_world_with_registration_order(&[
            TASK_4_1_LOW_ORGANISM,
            TASK_4_1_HIGH_ORGANISM,
        ]);
        let mut reverse = task_4_1_world_with_registration_order(&[
            TASK_4_1_HIGH_ORGANISM,
            TASK_4_1_LOW_ORGANISM,
        ]);
        let forward_food = task_4_1_consume_and_prepare_resource(&mut forward);
        let reverse_food = task_4_1_consume_and_prepare_resource(&mut reverse);

        let before_low = task_4_1_record_state(&forward, TASK_4_1_LOW_ORGANISM);
        let before_high = task_4_1_record_state(&forward, TASK_4_1_HIGH_ORGANISM);
        let expected_low = before_low
            .biochemistry()
            .advance(
                Tick::new(1),
                alife_core::BodyEventDelta::zero(),
                before_low.phenotype(),
            )
            .unwrap();
        let expected_high = before_high
            .biochemistry()
            .advance(
                Tick::new(1),
                alife_core::BodyEventDelta::zero(),
                before_high.phenotype(),
            )
            .unwrap();

        let mut late_failure = forward.clone();
        let before_failure_tick = late_failure.tick();
        let before_failure_low = task_4_1_record_state(&late_failure, TASK_4_1_LOW_ORGANISM);
        let before_failure_high = task_4_1_record_state(&late_failure, TASK_4_1_HIGH_ORGANISM);
        let before_failure_objects = late_failure.object_snapshots();
        let before_failure_ecology = late_failure.ecology().clone();
        let before_failure_metrics = late_failure.ecology_metrics();
        let before_failure_speech = late_failure.audible_utterances();
        let before_failure_touched = late_failure.last_touched_entities.clone();
        let before_failure_last_action = late_failure.last_action_result.clone();
        let before_failure_signature = late_failure.canonical_signature_digest().unwrap();
        late_failure.inject_tick_late_failure_after_first_organism_for_test();

        assert_eq!(
            late_failure.try_advance_tick(),
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        );
        assert_eq!(late_failure.tick(), before_failure_tick);
        assert_eq!(
            task_4_1_record_state(&late_failure, TASK_4_1_LOW_ORGANISM),
            before_failure_low
        );
        assert_eq!(
            task_4_1_record_state(&late_failure, TASK_4_1_HIGH_ORGANISM),
            before_failure_high
        );
        assert_eq!(late_failure.object_snapshots(), before_failure_objects);
        assert_eq!(late_failure.ecology(), &before_failure_ecology);
        assert_eq!(late_failure.ecology_metrics(), before_failure_metrics);
        assert_eq!(late_failure.audible_utterances(), before_failure_speech);
        assert_eq!(late_failure.last_touched_entities, before_failure_touched);
        assert_eq!(late_failure.last_action_result, before_failure_last_action);
        assert_eq!(
            late_failure.canonical_signature_digest().unwrap(),
            before_failure_signature
        );

        assert_eq!(forward.try_advance_tick().unwrap(), Tick::new(1));
        assert_eq!(reverse.try_advance_tick().unwrap(), Tick::new(1));
        assert_eq!(
            forward
                .organism_registry()
                .get(TASK_4_1_LOW_ORGANISM)
                .unwrap()
                .biochemistry(),
            &expected_low
        );
        assert_eq!(
            forward
                .organism_registry()
                .get(TASK_4_1_HIGH_ORGANISM)
                .unwrap()
                .biochemistry(),
            &expected_high
        );
        assert_ne!(
            expected_low.homeostasis,
            before_low.biochemistry().homeostasis
        );
        assert_ne!(
            expected_high.homeostasis,
            before_high.biochemistry().homeostasis
        );

        for (world, food) in [(&forward, forward_food), (&reverse, reverse_food)] {
            assert!(!world.entity(food).unwrap().is_consumed());
            assert_eq!(world.entity(food).unwrap().nutrition, 0.6);
            assert_eq!(world.ecology_metrics().resources_regrown, 1);
            let resource = world
                .ecology()
                .resources
                .iter()
                .find(|resource| resource.object_id == food)
                .unwrap();
            assert_eq!(resource.consumed_at_tick, None);
            assert_eq!(resource.last_regrown_tick, Some(Tick::new(1)));
            assert!(!resource.low_salience_marker);
        }

        assert_eq!(
            forward.canonical_signature_digest().unwrap(),
            reverse.canonical_signature_digest().unwrap()
        );
        for organism_id in [TASK_4_1_LOW_ORGANISM, TASK_4_1_HIGH_ORGANISM] {
            assert_eq!(
                task_4_1_record_state(&forward, organism_id),
                task_4_1_record_state(&reverse, organism_id)
            );
        }
    }

    #[test]
    fn try_advance_tick_advances_development_age_health_and_death_in_stable_order_and_rolls_back_late_failure(
    ) {
        let mut forward = task_4_1_world_with_registration_order(&[
            TASK_4_1_LOW_ORGANISM,
            TASK_4_1_HIGH_ORGANISM,
        ]);
        let mut reverse = task_4_1_world_with_registration_order(&[
            TASK_4_1_HIGH_ORGANISM,
            TASK_4_1_LOW_ORGANISM,
        ]);
        let hazard_spec = WorldEditorSpawnSpec {
            label: "terminal-hazard".to_owned(),
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position: Vec3f::new(1.0, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 1.0,
            radius: 0.75,
            token_id: None,
        };
        let forward_hazard = forward.editor_spawn_object(hazard_spec.clone()).unwrap();
        let reverse_hazard = reverse.editor_spawn_object(hazard_spec).unwrap();

        for world in [&mut forward, &mut reverse] {
            for organism_id in [TASK_4_1_LOW_ORGANISM, TASK_4_1_HIGH_ORGANISM] {
                world
                    .organism_registry
                    .with_biology_mut(organism_id, |biology| {
                        biology.cadence.metabolism_ticks = 1;
                        biology.cadence.development_ticks = 1;
                        if organism_id == TASK_4_1_LOW_ORGANISM {
                            biology.body.health = 0.1;
                        }
                        Ok(())
                    })
                    .unwrap();
            }
        }

        let before_low = task_4_1_record_state(&forward, TASK_4_1_LOW_ORGANISM);
        let before_high = task_4_1_record_state(&forward, TASK_4_1_HIGH_ORGANISM);
        let next_tick = Tick::new(1);
        let expected_low_development = before_low
            .phenotype()
            .development_state_at(next_tick)
            .unwrap();
        let forward_low_entity = forward.entity_id("agent-low").unwrap();
        let reverse_low_entity = reverse.entity_id("agent-low").unwrap();
        let forward_action = forward
            .apply_registered_command(
                &HeadlessWorldCommand::approach(TASK_4_1_LOW_ORGANISM, forward_hazard).unwrap(),
                forward_low_entity,
                next_tick,
            )
            .unwrap();
        let reverse_action = reverse
            .apply_registered_command(
                &HeadlessWorldCommand::approach(TASK_4_1_LOW_ORGANISM, reverse_hazard).unwrap(),
                reverse_low_entity,
                next_tick,
            )
            .unwrap();
        for action in [&forward_action, &reverse_action] {
            assert!(action.action_result.execution.succeeded);
            assert_eq!(action.action_result.body_event.damage, 1.0);
            assert_eq!(action.biology_after.body.health, 0.0);
            assert!(action.biology_after.body.energy < before_low.biochemistry().body.energy);
        }

        let mut late_failure = forward.clone();
        let before_failure_tick = late_failure.tick();
        let before_failure_low = task_4_1_record_state(&late_failure, TASK_4_1_LOW_ORGANISM);
        let before_failure_high = task_4_1_record_state(&late_failure, TASK_4_1_HIGH_ORGANISM);
        let before_failure_objects = late_failure.object_snapshots();
        let before_failure_ecology = late_failure.ecology().clone();
        let before_failure_metrics = late_failure.ecology_metrics();
        let before_failure_speech = late_failure.audible_utterances();
        let before_failure_touched = late_failure.last_touched_entities.clone();
        let before_failure_last_action = late_failure.last_action_result.clone();
        let before_failure_signature = late_failure.canonical_signature_digest().unwrap();
        late_failure.inject_tick_late_failure_after_first_organism_for_test();

        assert_eq!(
            late_failure.try_advance_tick(),
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        );
        assert_eq!(late_failure.tick(), before_failure_tick);
        assert_eq!(
            task_4_1_record_state(&late_failure, TASK_4_1_LOW_ORGANISM),
            before_failure_low
        );
        assert_eq!(
            task_4_1_record_state(&late_failure, TASK_4_1_HIGH_ORGANISM),
            before_failure_high
        );
        assert_eq!(late_failure.object_snapshots(), before_failure_objects);
        assert_eq!(late_failure.ecology(), &before_failure_ecology);
        assert_eq!(late_failure.ecology_metrics(), before_failure_metrics);
        assert_eq!(late_failure.audible_utterances(), before_failure_speech);
        assert_eq!(late_failure.last_touched_entities, before_failure_touched);
        assert_eq!(late_failure.last_action_result, before_failure_last_action);
        assert_eq!(
            late_failure.canonical_signature_digest().unwrap(),
            before_failure_signature
        );

        assert_eq!(forward.try_advance_tick().unwrap(), next_tick);
        assert_eq!(reverse.try_advance_tick().unwrap(), next_tick);
        let forward_low = task_4_1_record_state(&forward, TASK_4_1_LOW_ORGANISM);
        let forward_high = task_4_1_record_state(&forward, TASK_4_1_HIGH_ORGANISM);
        assert_eq!(forward_low.age_at(next_tick).unwrap(), next_tick);
        assert_eq!(forward_high.age_at(next_tick).unwrap(), next_tick);
        assert_eq!(forward_low.biochemistry().development.age_ticks, next_tick);
        assert_eq!(
            forward_low.biochemistry().development.maturation,
            expected_low_development.maturation.raw()
        );
        assert_ne!(
            forward_low.biochemistry().development.maturation,
            before_low.biochemistry().development.maturation
        );
        assert_eq!(forward_low.biochemistry().body.health, 0.0);
        assert_eq!(
            forward_low.biochemistry().body.energy,
            forward_action.biology_after.body.energy
        );
        assert_eq!(forward_low.lifecycle().death_tick(), Some(next_tick));
        assert!(!forward_low.lifecycle().is_alive());
        assert!(forward_high.lifecycle().is_alive());
        assert_eq!(forward_high.biochemistry().development.age_ticks, next_tick);
        assert_eq!(
            forward_high.biochemistry().body.health,
            before_high.biochemistry().body.health
        );
        assert_eq!(
            forward_high.biochemistry().body.energy,
            before_high.biochemistry().body.energy
        );
        for (before, after) in [(&before_low, &forward_low), (&before_high, &forward_high)] {
            assert_eq!(after.archive(), before.archive());
            assert_eq!(after.world_entity_id(), before.world_entity_id());
        }
        assert_eq!(
            forward.canonical_signature_digest().unwrap(),
            reverse.canonical_signature_digest().unwrap()
        );
        for organism_id in [TASK_4_1_LOW_ORGANISM, TASK_4_1_HIGH_ORGANISM] {
            assert_eq!(
                task_4_1_record_state(&forward, organism_id),
                task_4_1_record_state(&reverse, organism_id)
            );
        }
    }

    #[test]
    fn try_advance_tick_applies_maximum_lifespan_in_stable_order_and_rolls_back() {
        let mut forward = task_4_1_world_with_registration_order(&[
            TASK_4_1_LOW_ORGANISM,
            TASK_4_1_HIGH_ORGANISM,
        ]);
        let mut reverse = task_4_1_world_with_registration_order(&[
            TASK_4_1_HIGH_ORGANISM,
            TASK_4_1_LOW_ORGANISM,
        ]);
        let _forward_food = task_4_1_consume_and_prepare_resource(&mut forward);
        let _reverse_food = task_4_1_consume_and_prepare_resource(&mut reverse);

        let before_low = task_4_1_record_state(&forward, TASK_4_1_LOW_ORGANISM);
        let before_high = task_4_1_record_state(&forward, TASK_4_1_HIGH_ORGANISM);
        let low_maximum =
            alife_core::PassiveBodyUpkeepPolicy::maximum_lifespan_ticks(before_low.phenotype());
        let high_maximum =
            alife_core::PassiveBodyUpkeepPolicy::maximum_lifespan_ticks(before_high.phenotype());
        assert_ne!(low_maximum, high_maximum);
        let minimum_maximum = low_maximum.min(high_maximum);
        assert!(minimum_maximum > 0);
        let next_tick = Tick::new(minimum_maximum);
        let current_tick = Tick::new(next_tick.raw().saturating_sub(1));
        let terminal_organism = if low_maximum < high_maximum {
            TASK_4_1_LOW_ORGANISM
        } else {
            TASK_4_1_HIGH_ORGANISM
        };
        let survivor_organism = if terminal_organism == TASK_4_1_LOW_ORGANISM {
            TASK_4_1_HIGH_ORGANISM
        } else {
            TASK_4_1_LOW_ORGANISM
        };

        for world in [&mut forward, &mut reverse] {
            world.tick = current_tick;
            for organism_id in [TASK_4_1_LOW_ORGANISM, TASK_4_1_HIGH_ORGANISM] {
                let phenotype = world
                    .organism_registry
                    .get(organism_id)
                    .unwrap()
                    .phenotype()
                    .clone();
                let biology = alife_core::BiochemistryState::new(&phenotype, current_tick).unwrap();
                world
                    .organism_registry
                    .with_biology_mut(organism_id, |current| {
                        *current = biology;
                        Ok(())
                    })
                    .unwrap();
            }
            if terminal_organism == TASK_4_1_HIGH_ORGANISM {
                let phenotype = world
                    .organism_registry
                    .get(TASK_4_1_LOW_ORGANISM)
                    .unwrap()
                    .phenotype()
                    .clone();
                let biology = alife_core::BiochemistryState::new(&phenotype, next_tick).unwrap();
                world
                    .organism_registry
                    .with_biology_mut(TASK_4_1_LOW_ORGANISM, |current| {
                        *current = biology;
                        Ok(())
                    })
                    .unwrap();
            }
        }

        let expected_low = task_4_1_record_state(&forward, TASK_4_1_LOW_ORGANISM);
        let expected_high = task_4_1_record_state(&forward, TASK_4_1_HIGH_ORGANISM);
        let expected_low_biology = if expected_low.biochemistry().tick == next_tick {
            *expected_low.biochemistry()
        } else {
            expected_low
                .biochemistry()
                .advance(
                    next_tick,
                    alife_core::BodyEventDelta::zero(),
                    expected_low.phenotype(),
                )
                .unwrap()
        };
        let expected_high_biology = if expected_high.biochemistry().tick == next_tick {
            *expected_high.biochemistry()
        } else {
            expected_high
                .biochemistry()
                .advance(
                    next_tick,
                    alife_core::BodyEventDelta::zero(),
                    expected_high.phenotype(),
                )
                .unwrap()
        };

        let mut late_failure = forward.clone();
        let before_failure_tick = late_failure.tick();
        let before_failure_low = task_4_1_record_state(&late_failure, TASK_4_1_LOW_ORGANISM);
        let before_failure_high = task_4_1_record_state(&late_failure, TASK_4_1_HIGH_ORGANISM);
        let before_failure_objects = late_failure.object_snapshots();
        let before_failure_ecology = late_failure.ecology().clone();
        let before_failure_metrics = late_failure.ecology_metrics();
        let before_failure_speech = late_failure.audible_utterances();
        let before_failure_touched = late_failure.last_touched_entities.clone();
        let before_failure_last_action = late_failure.last_action_result.clone();
        let before_failure_signature = late_failure.canonical_signature_digest().unwrap();
        late_failure.inject_tick_late_failure_after_first_organism_for_test();

        assert_eq!(
            late_failure.try_advance_tick(),
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        );
        assert_eq!(late_failure.tick(), before_failure_tick);
        assert_eq!(
            task_4_1_record_state(&late_failure, TASK_4_1_LOW_ORGANISM),
            before_failure_low
        );
        assert_eq!(
            task_4_1_record_state(&late_failure, TASK_4_1_HIGH_ORGANISM),
            before_failure_high
        );
        assert_eq!(late_failure.object_snapshots(), before_failure_objects);
        assert_eq!(late_failure.ecology(), &before_failure_ecology);
        assert_eq!(late_failure.ecology_metrics(), before_failure_metrics);
        assert_eq!(late_failure.audible_utterances(), before_failure_speech);
        assert_eq!(late_failure.last_touched_entities, before_failure_touched);
        assert_eq!(late_failure.last_action_result, before_failure_last_action);
        assert_eq!(
            late_failure.canonical_signature_digest().unwrap(),
            before_failure_signature
        );

        assert_eq!(forward.try_advance_tick().unwrap(), next_tick);
        assert_eq!(reverse.try_advance_tick().unwrap(), next_tick);
        let forward_low = task_4_1_record_state(&forward, TASK_4_1_LOW_ORGANISM);
        let forward_high = task_4_1_record_state(&forward, TASK_4_1_HIGH_ORGANISM);
        assert_eq!(forward_low.biochemistry(), &expected_low_biology);
        assert_eq!(forward_high.biochemistry(), &expected_high_biology);
        for record in [&forward_low, &forward_high] {
            assert_eq!(record.age_at(next_tick).unwrap(), next_tick);
            assert!(record.biochemistry().body.health > 0.0);
            assert!(record.biochemistry().body.energy > 0.0);
            assert_eq!(record.archive(), {
                if record.organism_id() == TASK_4_1_LOW_ORGANISM {
                    before_low.archive()
                } else {
                    before_high.archive()
                }
            });
            assert_eq!(record.world_entity_id(), {
                if record.organism_id() == TASK_4_1_LOW_ORGANISM {
                    before_low.world_entity_id()
                } else {
                    before_high.world_entity_id()
                }
            });
        }
        let terminal_record = task_4_1_record_state(&forward, terminal_organism);
        let survivor_record = task_4_1_record_state(&forward, survivor_organism);
        assert_eq!(terminal_record.lifecycle().death_tick(), Some(next_tick));
        assert!(!terminal_record.lifecycle().is_alive());
        assert!(survivor_record.lifecycle().is_alive());
        assert_eq!(
            forward.canonical_signature_digest().unwrap(),
            reverse.canonical_signature_digest().unwrap()
        );
        for organism_id in [TASK_4_1_LOW_ORGANISM, TASK_4_1_HIGH_ORGANISM] {
            assert_eq!(
                task_4_1_record_state(&forward, organism_id),
                task_4_1_record_state(&reverse, organism_id)
            );
        }
    }

    #[test]
    fn try_advance_tick_applies_passive_upkeep_in_stable_order_and_rolls_back() {
        let mut forward = task_4_1_world_with_registration_order(&[
            TASK_4_1_LOW_ORGANISM,
            TASK_4_1_HIGH_ORGANISM,
        ]);
        let mut reverse = task_4_1_world_with_registration_order(&[
            TASK_4_1_HIGH_ORGANISM,
            TASK_4_1_LOW_ORGANISM,
        ]);
        let _forward_food = task_4_1_consume_and_prepare_resource(&mut forward);
        let _reverse_food = task_4_1_consume_and_prepare_resource(&mut reverse);

        let baseline = task_4_1_record_state(&forward, TASK_4_1_HIGH_ORGANISM);
        let mut one_tick_cadence = baseline.biochemistry().cadence;
        one_tick_cadence.metabolism_ticks = 1;
        one_tick_cadence.development_ticks = 1;
        let expected_upkeep_cost = 0.7925_f32 / 2551.0_f32;
        let expected_upkeep = alife_core::PassiveBodyUpkeepPolicy::upkeep_event(
            baseline.phenotype(),
            one_tick_cadence,
            1,
        );
        assert!(((-expected_upkeep.energy) - expected_upkeep_cost).abs() <= 1.0e-7);
        let low_energy = expected_upkeep_cost - 1.0e-6;

        for world in [&mut forward, &mut reverse] {
            for organism_id in [TASK_4_1_LOW_ORGANISM, TASK_4_1_HIGH_ORGANISM] {
                world
                    .organism_registry
                    .with_biology_mut(organism_id, |biology| {
                        biology.cadence = one_tick_cadence;
                        if organism_id == TASK_4_1_LOW_ORGANISM {
                            biology.body.energy = low_energy;
                        }
                        Ok(())
                    })
                    .unwrap();
            }
        }

        let before_low = task_4_1_record_state(&forward, TASK_4_1_LOW_ORGANISM);
        let before_high = task_4_1_record_state(&forward, TASK_4_1_HIGH_ORGANISM);
        assert!((before_high.biochemistry().body.energy - 0.7925).abs() <= 1.0e-6);
        let next_tick = Tick::new(1);
        let expected_development = before_high
            .phenotype()
            .development_state_at(next_tick)
            .unwrap();

        let mut late_failure = forward.clone();
        let before_failure_tick = late_failure.tick();
        let before_failure_low = task_4_1_record_state(&late_failure, TASK_4_1_LOW_ORGANISM);
        let before_failure_high = task_4_1_record_state(&late_failure, TASK_4_1_HIGH_ORGANISM);
        let before_failure_objects = late_failure.object_snapshots();
        let before_failure_ecology = late_failure.ecology().clone();
        let before_failure_metrics = late_failure.ecology_metrics();
        let before_failure_speech = late_failure.audible_utterances();
        let before_failure_touched = late_failure.last_touched_entities.clone();
        let before_failure_last_action = late_failure.last_action_result.clone();
        let before_failure_signature = late_failure.canonical_signature_digest().unwrap();
        late_failure.inject_tick_late_failure_after_first_organism_for_test();

        assert_eq!(
            late_failure.try_advance_tick(),
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        );
        assert_eq!(late_failure.tick(), before_failure_tick);
        assert_eq!(
            task_4_1_record_state(&late_failure, TASK_4_1_LOW_ORGANISM),
            before_failure_low
        );
        assert_eq!(
            task_4_1_record_state(&late_failure, TASK_4_1_HIGH_ORGANISM),
            before_failure_high
        );
        assert_eq!(late_failure.object_snapshots(), before_failure_objects);
        assert_eq!(late_failure.ecology(), &before_failure_ecology);
        assert_eq!(late_failure.ecology_metrics(), before_failure_metrics);
        assert_eq!(late_failure.audible_utterances(), before_failure_speech);
        assert_eq!(late_failure.last_touched_entities, before_failure_touched);
        assert_eq!(late_failure.last_action_result, before_failure_last_action);
        assert_eq!(
            late_failure.canonical_signature_digest().unwrap(),
            before_failure_signature
        );

        assert_eq!(forward.try_advance_tick().unwrap(), next_tick);
        assert_eq!(reverse.try_advance_tick().unwrap(), next_tick);
        let forward_low = task_4_1_record_state(&forward, TASK_4_1_LOW_ORGANISM);
        let forward_high = task_4_1_record_state(&forward, TASK_4_1_HIGH_ORGANISM);
        assert!(
            (forward_high.biochemistry().body.energy
                - (before_high.biochemistry().body.energy - expected_upkeep_cost))
                .abs()
                <= 1.0e-6
        );
        assert!(forward_high.biochemistry().body.energy > 0.0);
        assert_eq!(forward_low.biochemistry().body.energy, 0.0);
        assert_eq!(forward_low.lifecycle().death_tick(), Some(next_tick));
        assert!(!forward_low.lifecycle().is_alive());
        assert!(forward_high.lifecycle().is_alive());
        for record in [&forward_low, &forward_high] {
            assert_eq!(record.age_at(next_tick).unwrap(), next_tick);
            assert_eq!(record.biochemistry().development.age_ticks, next_tick);
            assert_eq!(
                record.biochemistry().development.maturation,
                expected_development.maturation.raw()
            );
        }
        for (before, after) in [(&before_low, &forward_low), (&before_high, &forward_high)] {
            assert_eq!(after.archive(), before.archive());
            assert_eq!(after.world_entity_id(), before.world_entity_id());
        }
        assert_eq!(
            forward.canonical_signature_digest().unwrap(),
            reverse.canonical_signature_digest().unwrap()
        );
        for organism_id in [TASK_4_1_LOW_ORGANISM, TASK_4_1_HIGH_ORGANISM] {
            assert_eq!(
                task_4_1_record_state(&forward, organism_id),
                task_4_1_record_state(&reverse, organism_id)
            );
        }
    }
}

#[cfg(test)]
mod task_4_3a2_tests {
    use super::*;

    const MATERNAL_ID: OrganismId = OrganismId(7);
    const PATERNAL_ID: OrganismId = OrganismId(19);
    const FOUNDATION_ID: u64 = 10;
    const COMPATIBILITY_FAMILY_ID: u64 = 7;

    fn founder(seed: u64, compatibility_family_id: u64) -> alife_core::CreatureGenome {
        alife_core::CreatureGenome::early_mammal_founder(
            seed,
            alife_core::FoundationGeneticIdentity::new(
                FOUNDATION_ID,
                1,
                compatibility_family_id,
                alife_core::BrainCapacityClass::N512_ID,
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn prepared_world(
        paternal_position: Vec3f,
        paternal_compatibility_family_id: u64,
    ) -> (HeadlessWorld, Tick) {
        let maternal_genome = founder(0xE10_43A1, COMPATIBILITY_FAMILY_ID);
        let paternal_genome = founder(0xE10_43B3, paternal_compatibility_family_id);
        let maternal_phenotype = maternal_genome.express().unwrap();
        let paternal_phenotype = paternal_genome.express().unwrap();
        let reproduction_period =
            u64::from(alife_core::BiochemistryCadence::early_mammal().reproduction_ticks);
        let first_post_puberty_boundary =
            ((maternal_phenotype.development.puberty_tick.raw() / reproduction_period) + 1)
                * reproduction_period;
        let current_tick = Tick(first_post_puberty_boundary - 1);
        let next_tick = Tick(first_post_puberty_boundary);
        let maternal_biology = alife_core::BiochemistryState::new_with_age(
            &maternal_phenotype,
            current_tick,
            current_tick,
        )
        .unwrap();
        let paternal_biology = alife_core::BiochemistryState::new_with_age(
            &paternal_phenotype,
            current_tick,
            current_tick,
        )
        .unwrap();
        let mut world = HeadlessScenarioBuilder::new(43_002)
            .agent("parent-maternal", MATERNAL_ID, Vec3f::ZERO)
            .agent("parent-paternal", PATERNAL_ID, paternal_position)
            .build()
            .unwrap();
        let maternal_entity = world.entity_id("parent-maternal").unwrap();
        let paternal_entity = world.entity_id("parent-paternal").unwrap();
        world.tick = current_tick;
        world
            .register_organism_record(
                WorldOrganismRecord::new(
                    MATERNAL_ID,
                    maternal_entity,
                    maternal_genome,
                    maternal_phenotype,
                    maternal_biology,
                    Tick::ZERO,
                )
                .unwrap(),
            )
            .unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::new(
                    PATERNAL_ID,
                    paternal_entity,
                    paternal_genome,
                    paternal_phenotype,
                    paternal_biology,
                    Tick::ZERO,
                )
                .unwrap(),
            )
            .unwrap();
        for organism_id in [MATERNAL_ID, PATERNAL_ID] {
            world
                .organism_registry
                .with_biology_mut(organism_id, |biology| {
                    biology.homeostasis.drives.reproductive_drive = 1.0;
                    biology.homeostasis.hormones.developmental_hormone = 1.0;
                    Ok(())
                })
                .unwrap();
        }
        (world, next_tick)
    }

    fn records(world: &HeadlessWorld) -> Vec<WorldOrganismRecord> {
        let mut records = world
            .organism_registry()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        records.sort_unstable_by_key(|record| record.organism_id().raw());
        records
    }

    #[test]
    fn nearby_ready_compatible_parents_create_one_deterministic_newborn() {
        let (mut forward, next_tick) =
            prepared_world(Vec3f::new(0.5, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
        let (mut replay, replay_next_tick) =
            prepared_world(Vec3f::new(0.5, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
        let expected_child_id = OrganismId(forward.next_organism_id);
        let maternal_genome_id = forward
            .organism_registry()
            .get(MATERNAL_ID)
            .unwrap()
            .genome()
            .id;
        let paternal_genome_id = forward
            .organism_registry()
            .get(PATERNAL_ID)
            .unwrap()
            .genome()
            .id;

        assert_eq!(forward.try_advance_tick().unwrap(), next_tick);
        assert_eq!(replay.try_advance_tick().unwrap(), replay_next_tick);
        assert_eq!(forward.object_count(), 3);
        assert_eq!(forward.organism_registry().iter().count(), 3);
        let child = forward.organism_registry().get(expected_child_id).unwrap();
        let child_object = forward.entity(child.world_entity_id()).unwrap();
        assert_eq!(child_object.kind, WorldObjectKind::Agent);
        assert_eq!(child_object.organism_id, Some(expected_child_id));
        assert_eq!(
            child_object.label,
            format!("organism-{}", expected_child_id.raw())
        );
        assert_eq!(child_object.position, Vec3f::new(0.25, 0.0, 0.0));
        assert_eq!(
            child.genome().parent_genome_ids,
            vec![maternal_genome_id, paternal_genome_id]
        );
        assert_eq!(child.phenotype(), &child.genome().express().unwrap());
        assert_eq!(child.biochemistry().development.age_ticks, Tick::ZERO);
        assert_eq!(child.birth_tick(), next_tick);
        assert!(child.lifecycle().is_alive());
        assert_eq!(child.archive(), &Default::default());
        assert_ne!(child.genome().conception_seed, 0);
        assert_eq!(child.phenotype().lineage_id, child.genome().lineage_id);
        assert_eq!(records(&forward), records(&replay));
        assert_eq!(
            forward.canonical_signature_digest().unwrap(),
            replay.canonical_signature_digest().unwrap()
        );
    }

    #[test]
    fn derived_living_capacity_blocks_ready_conception() {
        let (mut world, next_tick) =
            prepared_world(Vec3f::new(0.5, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
        let mut ecology = world.ecology().clone();
        ecology.config = EcologyConfig {
            max_world_objects: 4,
            max_resource_records: 2,
            ..ecology.config
        };
        world.configure_ecology(ecology).unwrap();
        let before_next_organism_id = world.next_organism_id;
        let before_agent_ids = world.organism_entity_ids();

        assert_eq!(world.try_advance_tick().unwrap(), next_tick);
        assert_eq!(world.tick(), next_tick);
        assert_eq!(world.next_organism_id, before_next_organism_id);
        assert_eq!(world.organism_entity_ids(), before_agent_ids);
        assert_eq!(world.organism_registry().iter().count(), 2);
        for organism_id in [MATERNAL_ID, PATERNAL_ID] {
            assert_eq!(
                world
                    .organism_registry()
                    .get(organism_id)
                    .unwrap()
                    .biochemistry()
                    .tick,
                next_tick
            );
        }
    }

    #[test]
    fn terminal_death_frees_living_capacity_before_conception() {
        let (mut world, next_tick) =
            prepared_world(Vec3f::new(0.5, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
        let terminal_id = OrganismId(31);
        let terminal_genome = founder(0xE10_43C5, COMPATIBILITY_FAMILY_ID);
        let terminal_phenotype = terminal_genome.express().unwrap();
        let terminal_biology =
            BiochemistryState::new_with_age(&terminal_phenotype, world.tick(), world.tick())
                .unwrap();
        let terminal_entity = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "terminal-organism".to_string(),
                kind: WorldObjectKind::Agent,
                organism_id: Some(terminal_id),
                position: Vec3f::new(4.0, 0.0, 0.0),
                nutrition: 0.0,
                hazard_pain: 0.0,
                radius: 0.5,
                token_id: None,
            })
            .unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::new(
                    terminal_id,
                    terminal_entity,
                    terminal_genome,
                    terminal_phenotype,
                    terminal_biology,
                    Tick::ZERO,
                )
                .unwrap(),
            )
            .unwrap();
        world
            .organism_registry
            .with_biology_mut(terminal_id, |biology| {
                biology.body.energy = 0.0;
                Ok(())
            })
            .unwrap();
        let mut ecology = world.ecology().clone();
        ecology.config = EcologyConfig {
            max_world_objects: 6,
            max_resource_records: 3,
            ..ecology.config
        };
        world.configure_ecology(ecology).unwrap();
        let expected_child_id = OrganismId(world.next_organism_id);

        assert_eq!(
            world
                .organism_registry()
                .iter()
                .filter(|record| record.lifecycle().is_alive())
                .count(),
            3
        );
        assert_eq!(world.try_advance_tick().unwrap(), next_tick);
        let terminal = world.organism_registry().get(terminal_id).unwrap();
        assert_eq!(terminal.lifecycle().death_tick(), Some(next_tick));
        assert!(!terminal.lifecycle().is_alive());
        let child = world.organism_registry().get(expected_child_id).unwrap();
        assert_eq!(child.birth_tick(), next_tick);
        assert!(child.lifecycle().is_alive());
        assert_eq!(world.next_organism_id, expected_child_id.raw() + 1);
        assert_eq!(world.organism_registry().iter().count(), 4);
        assert_eq!(
            world
                .organism_registry()
                .iter()
                .filter(|record| record.lifecycle().is_alive())
                .count(),
            3
        );
    }

    #[test]
    fn distant_or_genetically_incompatible_parents_receive_no_conception() {
        for (position, compatibility_family_id) in [
            (Vec3f::new(10.0, 0.0, 0.0), COMPATIBILITY_FAMILY_ID),
            (Vec3f::new(0.5, 0.0, 0.0), COMPATIBILITY_FAMILY_ID + 1),
        ] {
            let (mut world, next_tick) = prepared_world(position, compatibility_family_id);
            assert_eq!(world.try_advance_tick().unwrap(), next_tick);
            assert_eq!(world.object_count(), 2);
            assert_eq!(world.organism_registry().iter().count(), 2);
            for organism_id in [MATERNAL_ID, PATERNAL_ID] {
                let record = world.organism_registry().get(organism_id).unwrap();
                assert_eq!(record.biochemistry().reproduction.mating_opportunity, 0.0);
                assert!(!record.biochemistry().reproduction.ready);
            }
        }

        let (mut dead_parent_world, next_tick) =
            prepared_world(Vec3f::new(0.5, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
        dead_parent_world
            .organism_registry
            .mark_dead(PATERNAL_ID, dead_parent_world.tick())
            .unwrap();
        assert_eq!(dead_parent_world.try_advance_tick().unwrap(), next_tick);
        assert_eq!(dead_parent_world.object_count(), 2);
        assert_eq!(
            dead_parent_world
                .organism_registry()
                .get(MATERNAL_ID)
                .unwrap()
                .biochemistry()
                .reproduction
                .mating_opportunity,
            0.0
        );
        assert_eq!(
            dead_parent_world
                .organism_registry()
                .get(PATERNAL_ID)
                .unwrap()
                .biochemistry()
                .reproduction
                .mating_opportunity,
            0.0
        );
    }

    #[test]
    fn late_child_spawn_failure_preserves_tick_objects_registry_and_allocators() {
        let (mut world, next_tick) =
            prepared_world(Vec3f::new(0.5, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
        let expected_child_id = OrganismId(world.next_organism_id);
        world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: format!("organism-{}", expected_child_id.raw()),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position: Vec3f::new(2.0, 0.0, 0.0),
                nutrition: 0.0,
                hazard_pain: 0.0,
                radius: 0.5,
                token_id: None,
            })
            .unwrap();
        let before_signature = world.canonical_signature_digest().unwrap();
        let before_tick = world.tick();
        let before_objects = world.object_snapshots();
        let before_records = records(&world);
        let before_next_entity_id = world.next_entity_id;
        let before_next_organism_id = world.next_organism_id;
        let before_next_spawn_sequence = world.next_spawn_sequence;

        assert_eq!(
            world.try_advance_tick(),
            Err(ScaffoldContractError::InvalidId)
        );
        assert_eq!(world.tick(), before_tick);
        assert_eq!(world.object_snapshots(), before_objects);
        assert_eq!(records(&world), before_records);
        assert_eq!(world.next_entity_id, before_next_entity_id);
        assert_eq!(world.next_organism_id, before_next_organism_id);
        assert_eq!(world.next_spawn_sequence, before_next_spawn_sequence);
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            before_signature
        );
        assert_eq!(next_tick, Tick(before_tick.raw() + 1));
    }
}
