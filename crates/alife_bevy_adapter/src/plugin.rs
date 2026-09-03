//! Bevy plugin wiring for identity and the opt-in reference adapter pipeline.

use alife_core::{AffordanceBits, ScaffoldContractError, Tick};
use bevy::prelude::{
    App, Commands, Entity, IntoScheduleConfigs, Or, ParamSet, Plugin, Query, Res, ResMut, Resource,
    SystemSet, Transform, Update, With,
};

use crate::{
    gather_sensory_from_observed_with_profile, plan_action_command, ActionAdapterContext,
    ActionSink, AdapterContractFailure, AdapterStage, AffordanceTags, BevyEntityMap, CreatureBody,
    LatestSensorySnapshot, ObservedBevyEntity, ObserverSensoryProfile, SensoryEmitter,
    TargetAdapterState,
};

/// Production-safe adapter wiring. It installs identity mapping only.
///
/// The canonical game owns simulation cadence, cognition, world actions, and outcomes.
/// Installing this plugin must not add a second tick loop or scan the render world.
#[derive(Debug, Default)]
pub struct AlifeBevyAdapterPlugin;

impl Plugin for AlifeBevyAdapterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BevyEntityMap>();
    }
}

/// Small opt-in adapter pipeline for examples and isolated integration tests.
///
/// It gathers derived sensory snapshots and translates queued action commands.
/// The host must set [`AdapterWorldTick`] and execute plans through its authoritative
/// world. This plugin never fabricates outcomes or advances simulation time.
#[derive(Debug, Default)]
pub struct AlifeReferenceAdapterPlugin;

impl Plugin for AlifeReferenceAdapterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BevyEntityMap>()
            .init_resource::<AdapterScheduleTrace>()
            .init_resource::<AdapterWorldTick>()
            .configure_sets(
                Update,
                (
                    AlifeBevyAdapterSet::GatherSensory,
                    AlifeBevyAdapterSet::PlanAction,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    gather_sensory_system.in_set(AlifeBevyAdapterSet::GatherSensory),
                    plan_action_system.in_set(AlifeBevyAdapterSet::PlanAction),
                ),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum AlifeBevyAdapterSet {
    GatherSensory,
    PlanAction,
}

#[derive(Debug, Clone, Default, PartialEq, Resource)]
pub struct AdapterScheduleTrace {
    stages: Vec<AlifeBevyAdapterSet>,
}

impl AdapterScheduleTrace {
    pub fn clear(&mut self) {
        self.stages.clear();
    }

    pub fn push(&mut self, stage: AlifeBevyAdapterSet) {
        self.stages.push(stage);
    }

    pub fn stages(&self) -> &[AlifeBevyAdapterSet] {
        &self.stages
    }
}

/// Tick supplied by the authoritative host for derived adapter snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub struct AdapterWorldTick(Tick);

impl Default for AdapterWorldTick {
    fn default() -> Self {
        Self(Tick::ZERO)
    }
}

impl AdapterWorldTick {
    pub const fn new(tick: Tick) -> Self {
        Self(tick)
    }

    pub const fn current(self) -> Tick {
        self.0
    }

    pub fn set(&mut self, tick: Tick) {
        self.0 = tick;
    }
}

type ObservedFilter = Or<(
    With<CreatureBody>,
    With<AffordanceTags>,
    With<SensoryEmitter>,
)>;

#[allow(clippy::type_complexity)]
pub fn gather_sensory_system(
    mut trace: ResMut<AdapterScheduleTrace>,
    tick: Res<AdapterWorldTick>,
    mut commands: Commands,
    mut map: ResMut<BevyEntityMap>,
    observed_query: Query<
        (
            Entity,
            &Transform,
            Option<&CreatureBody>,
            Option<&AffordanceTags>,
            Option<&SensoryEmitter>,
        ),
        ObservedFilter,
    >,
    creature_query: Query<(Entity, &CreatureBody, &Transform)>,
) {
    trace.clear();
    trace.push(AlifeBevyAdapterSet::GatherSensory);
    if creature_query.is_empty() {
        return;
    }

    let mut observed = Vec::new();
    for (entity, transform, body, tags, emitter) in &observed_query {
        let result = observed_entity(entity, transform, body, tags, emitter, &mut map);
        match result {
            Ok(value) => {
                commands.entity(entity).remove::<AdapterContractFailure>();
                if let Some(value) = value {
                    observed.push(value);
                }
            }
            Err(error) => record_failure(&mut commands, entity, AdapterStage::GatherSensory, error),
        }
    }

    for (entity, body, transform) in &creature_query {
        let result = body.validate().and_then(|body| {
            map.bind(entity, body.world_entity_id)?;
            gather_sensory_from_observed_with_profile(
                body.organism_id,
                tick.current(),
                body.world_entity_id,
                transform.translation,
                ObserverSensoryProfile {
                    vision_radius_meters: body.vision_radius_meters,
                    hearing_radius_meters: body.hearing_radius_meters,
                },
                &observed,
            )
        });
        match result {
            Ok(snapshot) => {
                commands
                    .entity(entity)
                    .insert(LatestSensorySnapshot(snapshot))
                    .remove::<AdapterContractFailure>();
            }
            Err(error) => {
                commands.entity(entity).remove::<LatestSensorySnapshot>();
                record_failure(&mut commands, entity, AdapterStage::GatherSensory, error);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn plan_action_system(
    mut trace: ResMut<AdapterScheduleTrace>,
    mut map: ResMut<BevyEntityMap>,
    mut set: ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                Option<&AffordanceTags>,
                Option<&CreatureBody>,
            ),
            Or<(With<AffordanceTags>, With<CreatureBody>)>,
        >,
        Query<(Entity, &CreatureBody, &Transform, &mut ActionSink)>,
    )>,
) {
    trace.push(AlifeBevyAdapterSet::PlanAction);
    if !set
        .p1()
        .iter()
        .any(|(_, _, _, sink)| sink.pending_command.is_some())
    {
        return;
    }

    let targets = {
        let query = set.p0();
        let mut targets = Vec::new();
        for (entity, transform, tags, body) in &query {
            let mut affordances = tags.map_or(AffordanceBits::NONE, |tags| tags.bits);
            if body.is_some() {
                affordances |= AffordanceBits::SOCIAL_AGENT;
            }
            if affordances == AffordanceBits::NONE {
                continue;
            }
            let world_id = match world_id_for_entity(entity, body, &mut map) {
                Ok(world_id) => world_id,
                Err(_) => continue,
            };
            targets.push(TargetAdapterState::new(
                entity,
                world_id,
                transform.translation,
                affordances,
            ));
        }
        targets
    };

    for (entity, body, transform, mut sink) in &mut set.p1() {
        let Some(command) = sink.pending_command.take() else {
            continue;
        };
        let mut context = ActionAdapterContext::new(
            entity,
            body.organism_id,
            body.world_entity_id,
            transform.translation,
        );
        context.movement_step_meters = body.movement_step_meters;
        context.targets = targets
            .iter()
            .copied()
            .filter(|target| target.entity != entity)
            .collect();
        match plan_action_command(&command, &context) {
            Ok(feedback) => {
                sink.last_plan = Some(feedback.plan);
                sink.last_failure = feedback.failure;
                sink.last_contract_error = None;
            }
            Err(error) => {
                sink.last_plan = None;
                sink.last_failure = None;
                sink.last_contract_error = Some(error.to_string());
            }
        }
    }
}

fn observed_entity(
    entity: Entity,
    transform: &Transform,
    body: Option<&CreatureBody>,
    tags: Option<&AffordanceTags>,
    emitter: Option<&SensoryEmitter>,
    map: &mut BevyEntityMap,
) -> Result<Option<ObservedBevyEntity>, ScaffoldContractError> {
    if let Some(body) = body {
        body.validate()?;
    }
    if let Some(tags) = tags {
        tags.validate()?;
    }
    if let Some(emitter) = emitter {
        emitter.validate()?;
    }
    let world_id = world_id_for_entity(entity, body, map)?;
    let mut affordances = tags.map_or(AffordanceBits::NONE, |tags| tags.bits);
    if body.is_some() {
        affordances |= AffordanceBits::SOCIAL_AGENT;
    }
    if affordances == AffordanceBits::NONE
        && emitter.and_then(|emitter| emitter.audible_token).is_none()
    {
        return Ok(None);
    }
    let mut observed =
        ObservedBevyEntity::new(entity, world_id, transform.translation, affordances);
    observed.forward = transform.rotation * bevy::prelude::Vec3::NEG_Z;
    if let Some(body) = body {
        observed.organism_id = Some(body.organism_id);
    }
    if let Some(tags) = tags {
        observed.nutrition = tags.nutrition;
        observed.hazard_pain = tags.hazard_pain;
    }
    if let Some(emitter) = emitter {
        observed.token_id = emitter.audible_token;
        observed.visual_salience_scale = emitter.visual_salience_scale;
        observed.smell_salience_scale = emitter.smell_salience_scale;
        observed.audible_radius_meters = emitter.audible_radius_meters;
    } else {
        observed.audible_radius_meters = 0.0;
    }
    Ok(Some(observed))
}

fn record_failure(
    commands: &mut Commands,
    entity: Entity,
    stage: AdapterStage,
    error: ScaffoldContractError,
) {
    commands.entity(entity).insert(AdapterContractFailure {
        stage,
        message: error.to_string(),
    });
}

fn world_id_for_entity(
    entity: Entity,
    body: Option<&CreatureBody>,
    map: &mut BevyEntityMap,
) -> Result<alife_core::WorldEntityId, ScaffoldContractError> {
    if let Some(body) = body {
        map.bind(entity, body.world_entity_id)?;
        Ok(body.world_entity_id)
    } else {
        map.get_or_allocate(entity)
    }
}
