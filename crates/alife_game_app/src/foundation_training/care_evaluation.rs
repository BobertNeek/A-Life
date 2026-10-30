//! In-memory preparation/control/observation for matched disposable care trials.
//!
//! This is not an accepted-Current evaluator or a GPU restore adapter. The caller
//! must qualify the source first and restore each fork's opaque private/sampler
//! bytes against the unmodified source world, then call `begin_scene`. It must
//! use normal `tick_outcome`, biology and within-trial
//! learning, including sleep, and reject sealed-learning failures. No optimizer,
//! accepted-store path, publication callback or final private-state export exists here.
use alife_core::{ExperiencePatch, OrganismId, Tick, Validate, Vec3f, WorldEntityId};
use alife_world::{HeadlessWorld, WorldObjectKind};

use super::{
    food_outcome::{ownership, FoodOutcomeObservation},
    FoundationFoodOutcome, Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CareEvaluationArm {
    NoAssistance,
    CueOnly,
    CueAndReposition,
}

/// A caller's explicit admission, not proof that opaque bytes were restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CareEvaluationStartBoundary {
    ExactPrivatePolicyAndSampler,
    RequiresLegacySleepOrPolicyAdoption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CareEvaluationFoodHistory {
    KnownUnassistedAndNeverCarried,
    AssistedOrPreviouslyCarried,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct CareEvaluationSetup {
    pub organism: OrganismId,
    pub target: WorldEntityId,
    /// Common held-out placement, applied once per arm after exact restore.
    pub held_out_food_position: Vec3f,
    pub world_ticks: u64,
    pub start_boundary: CareEvaluationStartBoundary,
    /// Covers all retained food, not just this target. Session-local absence of
    /// interventions cannot establish this qualification for a real Current.
    pub food_history: CareEvaluationFoodHistory,
}

/// Owned input copies. No runtime cloning or validity of opaque bytes is claimed.
pub struct CareEvaluationFork {
    pub world: HeadlessWorld,
    pub private_checkpoint_bytes: Vec<u8>,
    pub sampler_checkpoint_bytes: Vec<u8>,
    pub control: CareEvaluationControl,
}

/// Refuse confounds instead of clearing speech, grip, learning or saved biology.
/// The original world and bytes remain borrowed and unchanged, including on error.
pub fn prepare_care_evaluation_forks(
    source: &HeadlessWorld,
    private_checkpoint_bytes: &[u8],
    sampler_checkpoint_bytes: &[u8],
    setup: CareEvaluationSetup,
) -> Result<[CareEvaluationFork; 3]> {
    if setup.start_boundary != CareEvaluationStartBoundary::ExactPrivatePolicyAndSampler
        || setup.food_history != CareEvaluationFoodHistory::KnownUnassistedAndNeverCarried
        || private_checkpoint_bytes.is_empty()
        || sampler_checkpoint_bytes.is_empty()
        || !(1..=36_000).contains(&setup.world_ticks)
    {
        return Err("care evaluation requires a qualified exact start, known food history, private state, sampler and bounded world ticks".into());
    }
    source.validate_organism_bindings()?;
    let record = source
        .organism_registry()
        .get(setup.organism)
        .ok_or("evaluation organism missing")?;
    if !record.lifecycle().is_alive()
        || source.organism_registry().len() != 1
        || !source.audible_utterances().is_empty()
        || source.entity_id("nursery-teacher").is_some()
        || source.player_held_object().is_some()
        || source
            .object_snapshots()
            .iter()
            .any(|o| o.kind == WorldObjectKind::Food && o.carried_by.is_some())
    {
        return Err("care evaluation start contains speech, teacher, grip, missing life or another resident; source preserved".into());
    }
    if !source.entity(setup.target).is_some_and(|food| {
        food.kind == WorldObjectKind::Food
            && !food.consumed
            && food.carried_by.is_none()
            && food.nutrition > 0.0
            && food.hazard_pain == 0.0
    }) {
        return Err("care evaluation requires a loose, available safe food target".into());
    }
    let end_tick = source
        .tick()
        .raw()
        .checked_add(setup.world_ticks)
        .ok_or("evaluation world-tick bound overflow")?;
    setup.held_out_food_position.validate()?;
    let source_world_digest = source.canonical_signature_digest()?;
    let arms = [
        CareEvaluationArm::NoAssistance,
        CareEvaluationArm::CueOnly,
        CareEvaluationArm::CueAndReposition,
    ];
    Ok(arms.map(|arm| CareEvaluationFork {
        world: source.clone(),
        private_checkpoint_bytes: private_checkpoint_bytes.to_vec(),
        sampler_checkpoint_bytes: sampler_checkpoint_bytes.to_vec(),
        control: CareEvaluationControl::new(source, setup, arm, end_tick, source_world_digest),
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct CareEvaluationBiology {
    pub alive: bool,
    pub energy: f32,
    pub health: f32,
    pub injury: f32,
    pub hunger: f32,
    pub fatigue: f32,
    pub pain: f32,
    pub sleeping: bool,
}

fn biology(world: &HeadlessWorld, organism: OrganismId) -> Option<CareEvaluationBiology> {
    world.organism_registry().get(organism).map(|r| {
        let b = r.biochemistry();
        CareEvaluationBiology {
            alive: r.lifecycle().is_alive(),
            energy: b.body.energy,
            health: b.body.health,
            injury: b.body.injury,
            hunger: b.homeostasis.drives.hunger,
            fatigue: b.homeostasis.drives.fatigue,
            pain: b.homeostasis.drives.pain,
            sleeping: b.body.sleeping,
        }
    })
}

fn position(world: &HeadlessWorld, organism: OrganismId) -> Option<Vec3f> {
    world
        .organism_registry()
        .get(organism)
        .and_then(|r| world.entity(r.world_entity_id()))
        .map(|o| o.position)
}

fn distance(a: Vec3f, b: Vec3f) -> f64 {
    (f64::from(a.x) - f64::from(b.x))
        .hypot(f64::from(a.y) - f64::from(b.y))
        .hypot(f64::from(a.z) - f64::from(b.z))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CareEvaluationStop {
    WorldTickBound,
    TerminalDeath,
    SubjectUnavailable,
    EarlyCensoring,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CareEvaluationSummary {
    pub arm: CareEvaluationArm,
    pub start_tick: u64,
    pub end_tick: u64,
    pub stop: CareEvaluationStop,
    pub terminal_death_tick: Option<u64>,
    pub sealed_outcomes_observed: u64,
    pub world_ticks_without_sealed_outcome: u64,
    pub displacement_meters: f64,
    pub path_length_meters: f64,
    pub first_acquisition_latency_ticks: Option<u64>,
    pub first_nourished_meal_latency_ticks: Option<u64>,
    pub food: FoundationFoodOutcome,
    pub nourished_meal_energy_gain: f64,
    pub nourished_meal_hunger_reduction: f64,
    pub initial_biology: CareEvaluationBiology,
    pub final_biology: Option<CareEvaluationBiology>,
    pub cue_emissions: u64,
    pub cue_deferrals: u64,
    /// Includes initial and periodic arm-specific placement, not common setup.
    pub food_repositions: u64,
}

pub struct CareEvaluationControl {
    setup: CareEvaluationSetup,
    end_tick: u64,
    source_world_digest: alife_world::HeadlessWorldSignatureDigest,
    scene_started: bool,
    last_assistance_tick: Option<u64>,
    previous_food: Option<super::FoodOwnershipSnapshot>,
    initial_position: Vec3f,
    previous_position: Option<Vec3f>,
    summary: CareEvaluationSummary,
}

impl CareEvaluationControl {
    fn new(
        world: &HeadlessWorld,
        setup: CareEvaluationSetup,
        arm: CareEvaluationArm,
        end_tick: u64,
        source_world_digest: alife_world::HeadlessWorldSignatureDigest,
    ) -> Self {
        let initial_position = position(world, setup.organism).expect("admitted world binding");
        Self {
            setup,
            end_tick,
            source_world_digest,
            scene_started: false,
            last_assistance_tick: None,
            previous_food: ownership(world, setup.target),
            initial_position,
            previous_position: Some(initial_position),
            summary: CareEvaluationSummary {
                arm,
                start_tick: world.tick().raw(),
                end_tick: world.tick().raw(),
                stop: CareEvaluationStop::EarlyCensoring,
                terminal_death_tick: None,
                sealed_outcomes_observed: 0,
                world_ticks_without_sealed_outcome: 0,
                displacement_meters: 0.0,
                path_length_meters: 0.0,
                first_acquisition_latency_ticks: None,
                first_nourished_meal_latency_ticks: None,
                food: FoundationFoodOutcome::new(setup.organism, setup.target),
                nourished_meal_energy_gain: 0.0,
                nourished_meal_hunger_reduction: 0.0,
                initial_biology: biology(world, setup.organism).expect("admitted biology"),
                final_biology: biology(world, setup.organism),
                cue_emissions: 0,
                cue_deferrals: 0,
                food_repositions: 0,
            },
        }
    }

    /// After private/sampler restore, verify the unchanged world before applying
    /// the common held-out scene. This world check does not verify opaque private
    /// bytes. Legacy alignment, preparatory ticks or scene changes are refused.
    pub fn begin_scene(&mut self, world: &mut HeadlessWorld) -> Result<()> {
        if self.scene_started || world.canonical_signature_digest()? != self.source_world_digest {
            return Err("evaluation scene requires one unchanged exact source boundary".into());
        }
        let mut staged = world.clone();
        staged.editor_move_object(self.setup.target, self.setup.held_out_food_position)?;
        *world = staged;
        self.scene_started = true;
        Ok(())
    }

    /// Call before each ordinary runtime tick. A NoProgress outcome does not
    /// consume world-tick budget; repeated calls at that same tick are idempotent.
    /// There are no update/window-boundary placement or recovery-speech paths.
    pub fn before_tick(&mut self, world: &mut HeadlessWorld) -> Result<bool> {
        if !self.scene_started {
            return Err("evaluation scene must begin after exact private/sampler restore".into());
        }
        if world.tick().raw() != self.summary.end_tick {
            return Err("evaluation must observe every committed world tick".into());
        }
        if world.tick().raw() >= self.end_tick
            || !biology(world, self.setup.organism).is_some_and(|b| b.alive)
        {
            return Ok(false);
        }
        let tick = world.tick().raw();
        if self.summary.arm == CareEvaluationArm::NoAssistance
            || !(tick - self.summary.start_tick).is_multiple_of(24)
            || self.last_assistance_tick == Some(tick)
            || !world
                .entity(self.setup.target)
                .is_some_and(|o| !o.consumed && o.carried_by.is_none())
        {
            return Ok(true);
        }
        // Stage the physical intervention only, without ticking or changing the individual.
        let mut staged = world.clone();
        let (moved, emitted) = crate::foundation_training_cycle::evaluation_care_interventions(
            &mut staged,
            self.setup.organism,
            self.setup.target,
            self.summary.arm == CareEvaluationArm::CueAndReposition,
        )?;
        *world = staged;
        self.last_assistance_tick = Some(tick);
        self.summary.food_repositions += u64::from(moved);
        self.summary.cue_emissions += u64::from(emitted);
        self.summary.cue_deferrals += u64::from(!emitted);
        Ok(true)
    }

    /// Observe after a committed normal tick; `None` permits ordinary sleep and
    /// other non-capture ticks. Selected commands never count as body movement.
    pub fn observe_committed_tick(
        &mut self,
        world: &HeadlessWorld,
        patch: Option<&ExperiencePatch>,
    ) -> Result<()> {
        let tick = world.tick().raw();
        if !self.scene_started
            || self.summary.end_tick.checked_add(1) != Some(tick)
            || tick > self.end_tick
        {
            return Err(
                "evaluation observed a skipped, repeated or out-of-bound world tick".into(),
            );
        }
        let observation = FoodOutcomeObservation {
            organism: self.setup.organism,
            target: self.setup.target,
            decision_tick: Tick(self.summary.end_tick),
            outcome_tick: world.tick(),
            before: self.previous_food,
            after: ownership(world, self.setup.target),
        };
        if let Some(patch) = patch {
            patch.validate_contract()?;
            if !observation.matches_patch(patch) {
                return Err("evaluation requires the matching sealed world outcome".into());
            }
        }
        if let Some(patch) = patch {
            self.summary.sealed_outcomes_observed += 1;
            let events = self.summary.food.observe(observation, patch);
            let latency = tick - self.summary.start_tick;
            if events.acquisition.is_some() {
                self.summary
                    .first_acquisition_latency_ticks
                    .get_or_insert(latency);
            }
            if events.nourished {
                self.summary
                    .first_nourished_meal_latency_ticks
                    .get_or_insert(latency);
                if let Some(b) = patch.outcome().measured_physiology {
                    self.summary.nourished_meal_energy_gain +=
                        f64::from(b.energy_delta.raw().max(0.0));
                    self.summary.nourished_meal_hunger_reduction += f64::from(
                        (b.before.homeostasis.drives.hunger - b.after.homeostasis.drives.hunger)
                            .max(0.0),
                    );
                }
            }
        } else {
            self.summary.world_ticks_without_sealed_outcome += 1;
        }
        let current_position = position(world, self.setup.organism);
        if let (Some(previous), Some(current)) = (self.previous_position, current_position) {
            self.summary.path_length_meters += distance(previous, current);
            self.summary.displacement_meters = distance(self.initial_position, current);
        }
        self.previous_position = current_position;
        self.previous_food = observation.after;
        self.summary.final_biology = biology(world, self.setup.organism);
        self.summary.terminal_death_tick = world
            .organism_registry()
            .get(self.setup.organism)
            .and_then(|record| record.lifecycle().death_tick())
            .map(|tick| tick.raw());
        self.summary.end_tick = tick;
        Ok(())
    }

    /// Consumes only observations. Missing event latencies remain censored, with
    /// death/unavailability distinguished from exhausting the requested horizon.
    pub fn finish(mut self) -> CareEvaluationSummary {
        self.summary.stop = match self.summary.final_biology {
            None => CareEvaluationStop::SubjectUnavailable,
            Some(b) if !b.alive => CareEvaluationStop::TerminalDeath,
            Some(_) if self.summary.end_tick == self.end_tick => CareEvaluationStop::WorldTickBound,
            Some(_) => CareEvaluationStop::EarlyCensoring,
        };
        self.summary
    }
}

#[cfg(test)]
mod tests;
