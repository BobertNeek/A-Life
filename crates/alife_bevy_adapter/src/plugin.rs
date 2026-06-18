//! v0 scaffold: Bevy plugin scheduling for the adapter causal order.

use alife_core::{AffordanceBits, Tick};
use bevy::prelude::{
    App, Commands, Entity, IntoScheduleConfigs, ParamSet, Plugin, Query, Res, ResMut, Resource,
    SystemSet, Transform, Update,
};

use crate::{
    execute_action_command, gather_sensory_from_observed, ActionAdapterContext, ActionSink,
    AffordanceTags, BevyEntityMap, CreatureBody, LatestSensorySnapshot, ObservedBevyEntity,
    PatchTelemetry, SensoryEmitter, TargetAdapterState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum AlifeBevyAdapterSet {
    GatherSensory,
    CpuBrainTick,
    ExecuteAction,
    MeasureOutcome,
    SealPatch,
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

    pub fn advance(&mut self) -> Tick {
        self.0 = Tick::new(self.0.raw().saturating_add(1));
        self.0
    }
}

#[derive(Debug, Default)]
pub struct AlifeBevyAdapterPlugin;

impl Plugin for AlifeBevyAdapterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BevyEntityMap>()
            .init_resource::<PatchTelemetry>()
            .init_resource::<AdapterScheduleTrace>()
            .init_resource::<AdapterWorldTick>()
            .configure_sets(
                Update,
                (
                    AlifeBevyAdapterSet::GatherSensory,
                    AlifeBevyAdapterSet::CpuBrainTick,
                    AlifeBevyAdapterSet::ExecuteAction,
                    AlifeBevyAdapterSet::MeasureOutcome,
                    AlifeBevyAdapterSet::SealPatch,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    gather_sensory_system.in_set(AlifeBevyAdapterSet::GatherSensory),
                    cpu_brain_tick_system.in_set(AlifeBevyAdapterSet::CpuBrainTick),
                    execute_action_system.in_set(AlifeBevyAdapterSet::ExecuteAction),
                    measure_outcome_system.in_set(AlifeBevyAdapterSet::MeasureOutcome),
                    seal_patch_system.in_set(AlifeBevyAdapterSet::SealPatch),
                ),
            );
    }
}

#[allow(clippy::type_complexity)]
pub fn gather_sensory_system(
    mut trace: ResMut<AdapterScheduleTrace>,
    tick: Res<AdapterWorldTick>,
    mut commands: Commands,
    mut map: ResMut<BevyEntityMap>,
    observed_query: Query<(
        Entity,
        &Transform,
        Option<&CreatureBody>,
        Option<&AffordanceTags>,
        Option<&SensoryEmitter>,
    )>,
    creature_query: Query<(Entity, &CreatureBody, &Transform)>,
) {
    trace.clear();
    trace.push(AlifeBevyAdapterSet::GatherSensory);

    let observed = observed_query
        .iter()
        .filter_map(|(entity, transform, body, tags, emitter)| {
            let mut affordances = tags.map_or(AffordanceBits::NONE, |tags| tags.bits);
            if body.is_some() {
                affordances |= AffordanceBits::SOCIAL_AGENT;
            }
            if affordances == AffordanceBits::NONE
                && emitter.and_then(|value| value.audible_token).is_none()
            {
                return None;
            }
            let world_id = world_id_for_entity(entity, body, &mut map)?;

            let mut observed =
                ObservedBevyEntity::new(entity, world_id, transform.translation, affordances);
            if let Some(body) = body {
                observed = observed.with_organism(body.organism_id);
                observed.vision_radius_meters = body.vision_radius_meters;
                observed.hearing_radius_meters = body.hearing_radius_meters;
            }
            if let Some(tags) = tags {
                observed.nutrition = tags.nutrition;
                observed.hazard_pain = tags.hazard_pain;
            }
            if let Some(emitter) = emitter {
                observed.token_id = emitter.audible_token;
                observed.hearing_radius_meters = emitter.audible_radius_meters;
            }
            Some(observed)
        })
        .collect::<Vec<_>>();

    for (entity, body, transform) in &creature_query {
        let _ = map.bind(entity, body.world_entity_id);
        if let Ok(snapshot) = gather_sensory_from_observed(
            body.organism_id,
            tick.current(),
            body.world_entity_id,
            transform.translation,
            &observed,
        ) {
            commands
                .entity(entity)
                .insert(LatestSensorySnapshot(snapshot));
        }
    }
}

pub fn cpu_brain_tick_system(mut trace: ResMut<AdapterScheduleTrace>) {
    trace.push(AlifeBevyAdapterSet::CpuBrainTick);
}

#[allow(clippy::type_complexity)]
pub fn execute_action_system(
    mut trace: ResMut<AdapterScheduleTrace>,
    mut map: ResMut<BevyEntityMap>,
    mut set: ParamSet<(
        Query<(
            Entity,
            &Transform,
            Option<&AffordanceTags>,
            Option<&CreatureBody>,
        )>,
        Query<(Entity, &CreatureBody, &mut Transform, &mut ActionSink)>,
    )>,
) {
    trace.push(AlifeBevyAdapterSet::ExecuteAction);
    let targets = {
        let mut targets = Vec::new();
        for (entity, transform, tags, body) in &set.p0() {
            let mut affordances = tags.map_or(AffordanceBits::NONE, |tags| tags.bits);
            if body.is_some() {
                affordances |= AffordanceBits::SOCIAL_AGENT;
            }
            if affordances == AffordanceBits::NONE {
                continue;
            }
            let Some(world_id) = world_id_for_entity(entity, body, &mut map) else {
                continue;
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

    for (entity, body, mut transform, mut sink) in &mut set.p1() {
        let Some(command) = sink.pending_command.take() else {
            continue;
        };
        let mut context =
            ActionAdapterContext::new(entity, body.world_entity_id, transform.translation);
        context.movement_step_meters = body.movement_step_meters;
        context.targets = targets
            .iter()
            .copied()
            .filter(|target| target.entity != entity)
            .collect();
        match execute_action_command(&command, &context) {
            Ok(feedback) => {
                if feedback.execution.succeeded {
                    transform.translation += feedback.plan.displacement * command.intensity.raw();
                }
                sink.last_execution = Some(feedback.execution);
                sink.last_failure = feedback.failure;
            }
            Err(_) => {
                sink.last_execution = None;
                sink.last_failure = None;
            }
        }
    }
}

pub fn measure_outcome_system(mut trace: ResMut<AdapterScheduleTrace>) {
    trace.push(AlifeBevyAdapterSet::MeasureOutcome);
}

pub fn seal_patch_system(
    mut trace: ResMut<AdapterScheduleTrace>,
    mut tick: ResMut<AdapterWorldTick>,
) {
    trace.push(AlifeBevyAdapterSet::SealPatch);
    let _ = tick.advance();
}

fn world_id_for_entity(
    entity: Entity,
    body: Option<&CreatureBody>,
    map: &mut BevyEntityMap,
) -> Option<alife_core::WorldEntityId> {
    if let Some(body) = body {
        let _ = map.bind(entity, body.world_entity_id);
        Some(body.world_entity_id)
    } else {
        map.get_or_allocate(entity).ok()
    }
}
