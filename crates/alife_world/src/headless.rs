//! v0 runtime scaffold: deterministic Bevy-independent headless world harness.
//!
//! This module owns simple world truth for P17 behavior tests. It implements
//! core sensory/action adapter traits without importing renderer, GPU, or ECS
//! concepts.

use std::{
    cell::{Ref, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use alife_core::{
    ActionCommand, ActionId, ActionKind, AffordanceBits, BrainTickInput, BrainTickOutput,
    Confidence, ContextStreams, DriveDelta, EndocrineDelta, ExperiencePatch, HeardToken,
    HomeostaticDelta, Intensity, LanguageContextSnapshot, NormalizedScalar, OrganismId,
    PhysicalActionOutcome, PhysicalContactKind, ReferenceActionExecution, ReferenceActionExecutor,
    ReferenceActionFailure, ReferenceOutcomeObservation, ReferenceOutcomeObserver,
    ReferenceOutcomeRequest, ReferenceSensoryAdapter, ReferenceSensoryRequest,
    ScaffoldContractError, SensoryChannels, SensorySnapshot, SignedValence,
    SleepConsolidationReport, SleepTransition, SleepTrigger, SocialAgentSnapshot,
    SocialProximityEntry, TeacherPerceptionChannel, Tick, Validate, Vec3f, WorldEntityId,
    MAX_HEARD_TOKENS, MAX_SOCIAL_AGENTS, SENSORY_AUDITORY_CHANNEL_COUNT,
    SENSORY_SMELL_CHANNEL_COUNT, SENSORY_TACTILE_CHANNEL_COUNT,
    SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT,
};

const DEFAULT_ENTITY_ID_START: u64 = 1;
const DEFAULT_VISION_RADIUS: f32 = 8.0;
const DEFAULT_HEARING_RADIUS: f32 = 6.0;
const CONTACT_RADIUS: f32 = 0.75;
const EAT_RADIUS: f32 = 1.25;
const MOVE_STEP: f32 = 1.0;
const MAX_VISIBLE_ENTITIES: usize = 16;

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
            WorldObjectKind::Token => AffordanceBits::GLYPH_OR_WRITING,
        }
    }

    fn blocks_position(&self, position: Vec3f) -> bool {
        self.kind == WorldObjectKind::Obstacle
            && !self.consumed
            && distance(self.position, position) <= self.radius.max(CONTACT_RADIUS)
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessActionResult {
    pub command: ActionCommand,
    pub execution: ReferenceActionExecution,
    pub observation: ReferenceOutcomeObservation,
    pub touched_entities: Vec<WorldEntityId>,
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

#[derive(Debug, Clone)]
pub struct HeadlessWorld {
    seed: u64,
    tick: Tick,
    next_entity_id: u64,
    objects: BTreeMap<u64, WorldObject>,
    labels: BTreeMap<String, WorldEntityId>,
    last_touched_entities: Vec<WorldEntityId>,
    last_action_result: Option<HeadlessActionResult>,
}

#[derive(Debug, Clone)]
pub(crate) struct HeadlessWorldPersistenceParts {
    pub seed: u64,
    pub tick: Tick,
    pub next_entity_id: u64,
    pub objects: Vec<WorldObject>,
    pub last_touched_entities: Vec<WorldEntityId>,
}

impl HeadlessWorld {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            tick: Tick::ZERO,
            next_entity_id: DEFAULT_ENTITY_ID_START,
            objects: BTreeMap::new(),
            labels: BTreeMap::new(),
            last_touched_entities: Vec::new(),
            last_action_result: None,
        }
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub const fn tick(&self) -> Tick {
        self.tick
    }

    pub fn advance_tick(&mut self) -> Tick {
        self.tick = Tick::new(self.tick.raw().saturating_add(1));
        self.tick
    }

    pub fn entity_id(&self, label: &str) -> Option<WorldEntityId> {
        self.labels.get(label).copied()
    }

    pub fn entity(&self, id: WorldEntityId) -> Option<&WorldObject> {
        self.objects.get(&id.raw())
    }

    pub fn stable_signature(&self) -> Vec<String> {
        self.objects
            .values()
            .map(|object| {
                format!(
                    "{}:{:?}:{}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{:?}:{:.3}:{:?}:{}:{:?}",
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
                    object.carried_by
                )
            })
            .collect()
    }

    pub(crate) fn persistence_parts(&self) -> HeadlessWorldPersistenceParts {
        HeadlessWorldPersistenceParts {
            seed: self.seed,
            tick: self.tick,
            next_entity_id: self.next_entity_id,
            objects: self.objects.values().cloned().collect(),
            last_touched_entities: self.last_touched_entities.clone(),
        }
    }

    pub(crate) fn from_persistence_parts(
        parts: HeadlessWorldPersistenceParts,
    ) -> Result<Self, ScaffoldContractError> {
        let mut objects = BTreeMap::new();
        let mut labels = BTreeMap::new();
        let mut max_id = 0_u64;
        for object in parts.objects {
            validate_persisted_object(&object)?;
            let raw_id = object.id.raw();
            if objects.contains_key(&raw_id) || labels.contains_key(&object.label) {
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
        for touched in &parts.last_touched_entities {
            touched.validate()?;
            if !objects.contains_key(&touched.raw()) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(Self {
            seed: parts.seed,
            tick: parts.tick,
            next_entity_id: parts.next_entity_id,
            objects,
            labels,
            last_touched_entities: parts.last_touched_entities,
            last_action_result: None,
        })
    }

    pub fn sensory_report(
        &self,
        organism_id: OrganismId,
        tick: Tick,
    ) -> Result<HeadlessSensoryReport, ScaffoldContractError> {
        organism_id.validate()?;
        let agent = self.agent_for(organism_id)?;
        let visible_entities = self.visible_entities_from(agent);
        let contact_entities = visible_entities
            .iter()
            .filter(|visible| visible.distance <= CONTACT_RADIUS)
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
            let salience = proximity_salience(visible.distance, DEFAULT_VISION_RADIUS);
            match visible.kind {
                WorldObjectKind::Food => {
                    visual[0] = visual[0].max(salience);
                    smell[0] = smell[0].max(salience);
                }
                WorldObjectKind::Hazard => {
                    visual[1] = visual[1].max(salience);
                    smell[1] = smell[1].max(salience);
                    pain = pain.max(proximity_salience(visible.distance, CONTACT_RADIUS * 2.0));
                }
                WorldObjectKind::Obstacle => {
                    visual[2] = visual[2].max(salience);
                    tactile[0] = tactile[0].max(if visible.distance <= CONTACT_RADIUS {
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
                                speaker_id: None,
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
            ambient_light: NormalizedScalar::new(0.8)?,
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
            let object = self
                .objects
                .values()
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

        Ok(HeadlessSensoryReport {
            core_snapshot,
            visible_entities,
            contact_entities,
            touched_entities: self.last_touched_entities.clone(),
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

    fn insert_object(
        &mut self,
        spec: SpawnSpec<'_>,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        spec.position.validate()?;
        if spec.label.is_empty() || self.labels.contains_key(spec.label) {
            return Err(ScaffoldContractError::InvalidId);
        }
        let id = WorldEntityId(self.next_entity_id);
        self.next_entity_id = self.next_entity_id.saturating_add(1);
        let object = WorldObject {
            id,
            label: spec.label.to_string(),
            kind: spec.kind,
            organism_id: spec.organism_id,
            position: spec.position,
            radius: CONTACT_RADIUS,
            nutrition: spec.nutrition.clamp(0.0, 1.0),
            hazard_pain: spec.hazard_pain.clamp(0.0, 1.0),
            token_id: spec.token_id,
            social_affinity: spec.social_affinity.clamp(-1.0, 1.0),
            teacher_channel: spec.teacher_channel,
            consumed: false,
            carried_by: None,
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
                self.finish_action(
                    *command,
                    true,
                    None,
                    physical(PhysicalContactKind::Touch, Some(target), Vec3f::ZERO, 0.02)?,
                    OutcomeProfile::inspect(),
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
            HeadlessAction::Vocalize => self.finish_action(
                *command,
                true,
                None,
                physical(PhysicalContactKind::None, None, Vec3f::ZERO, 0.02)?,
                OutcomeProfile::vocalize(),
                Vec::new(),
            ),
        }
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
        object.consumed = true;
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
            OutcomeProfile::food(nutrition),
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
                    && distance(object.position, destination) <= CONTACT_RADIUS
            })
            .map(|(id, _)| WorldEntityId(*id))
            .collect::<Vec<_>>();
        let hazard = touched.iter().find_map(|id| {
            self.objects
                .get(&id.raw())
                .filter(|object| object.kind == WorldObjectKind::Hazard)
                .map(|object| (*id, object.hazard_pain))
        });
        if let Some(agent) = self.objects.get_mut(&agent_id.raw()) {
            agent.position = destination;
        }
        let displacement = subtract(destination, start);
        let (profile, contact, target) = if let Some((hazard_id, pain)) = hazard {
            (
                OutcomeProfile::hazard(pain),
                PhysicalContactKind::Collision,
                Some(hazard_id),
            )
        } else {
            (
                OutcomeProfile::movement(),
                PhysicalContactKind::Moved,
                command.target_entity,
            )
        };
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
            SignedValence::new(profile.reward)?,
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
            touched_entities,
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

    fn visible_entities_from(&self, observer: &WorldObject) -> Vec<VisibleWorldEntity> {
        let mut visible = self
            .objects
            .values()
            .filter(|object| object.id != observer.id && !object.consumed)
            .filter_map(|object| {
                let distance = distance(observer.position, object.position);
                (distance <= DEFAULT_VISION_RADIUS).then_some(VisibleWorldEntity {
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
        let sleep_transition = if matches!(
            brain.selected_action.map(|command| command.kind),
            Some(ActionKind::Rest)
        ) && brain
            .experience_patch
            .as_ref()
            .is_some_and(|patch| patch.outcome().success)
        {
            mind.force_sleep(mind.current_tick(), SleepTrigger::ForcedRequest)
                .ok()
        } else {
            None
        };
        self.world.borrow_mut().advance_tick();
        HeadlessBrainTick {
            brain,
            action_result,
            sleep_transition,
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
    reward: f32,
    frustration: f32,
    pain: f32,
    energy: f32,
    prediction_error: f32,
    contradiction: bool,
}

impl OutcomeProfile {
    fn idle() -> Self {
        Self::new(
            DriveDelta::zero(),
            EndocrineDelta::zero(),
            0.0,
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
            0.1,
            0.0,
            0.0,
            0.08,
            0.05,
            false,
        )
    }

    fn inspect() -> Self {
        Self::new(
            DriveDelta {
                curiosity: -0.03,
                brain_atp: -0.01,
                ..DriveDelta::zero()
            },
            EndocrineDelta::zero(),
            0.05,
            0.0,
            0.0,
            -0.01,
            0.1,
            false,
        )
    }

    fn food(nutrition: f32) -> Self {
        let nutrition = nutrition.clamp(0.0, 1.0);
        Self::new(
            DriveDelta {
                hunger: -nutrition,
                brain_atp: nutrition * 0.35,
                ..DriveDelta::zero()
            },
            EndocrineDelta {
                dopamine: 0.15,
                serotonin: 0.05,
                ..EndocrineDelta::zero()
            },
            0.55 + nutrition * 0.4,
            0.0,
            0.0,
            nutrition * 0.5,
            0.05,
            false,
        )
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
            -0.35 - pain * 0.45,
            0.25,
            pain,
            -0.08,
            0.8,
            true,
        )
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
            -0.2,
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
            -0.35,
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
            -0.4,
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
            0.08,
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
            0.04,
            0.0,
            0.0,
            -0.01,
            0.1,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        drives: DriveDelta,
        hormones: EndocrineDelta,
        reward: f32,
        frustration: f32,
        pain: f32,
        energy: f32,
        prediction_error: f32,
        contradiction: bool,
    ) -> Self {
        Self {
            homeostatic_delta: HomeostaticDelta { drives, hormones },
            reward,
            frustration,
            pain,
            energy,
            prediction_error,
            contradiction,
        }
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
