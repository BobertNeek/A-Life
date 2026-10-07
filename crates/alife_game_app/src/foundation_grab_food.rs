//! Offline acquisition evidence and separately reported curriculum credit.
//! World physiology, chemistry, candidates, and runtime inference are unchanged.
use crate::FoundationTrainingStep;
use alife_core::{
    ChannelCommand, MotorChannel, OrganismId, PhysicalActionOutcome, PhysicalContactKind, Validate,
    Vec3f, WorldEntityId,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub const GRAB_FOOD_CURRICULUM_VERSION: u16 = 1;
pub const GRAB_FOOD_ACQUISITION_REWARD: f32 = 0.25;

/// Safe, reachable, unheld food. No setup Grab or acquisition is executed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationGrabFoodSetup {
    pub target: WorldEntityId,
    pub organism: OrganismId,
    pub setup_tick: u64,
    pub initial_energy: f32,
    pub initial_hunger: f32,
    pub initial_health: f32,
    pub initial_sleeping: bool,
    pub initial_owner: Option<OrganismId>,
    pub initial_consumed: bool,
    pub sampling_seed: u64,
    pub sampling_attempts: u32,
    pub realized_body_position: Vec3f,
    pub realized_body_yaw: f32,
    pub food_position: Vec3f,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationGrabAcquisition {
    pub row: usize,
    pub organism: OrganismId,
    pub target: WorldEntityId,
    pub sequence_id: u64,
    pub decision_tick: u64,
    pub outcome_tick: u64,
    pub owner_before: Option<OrganismId>,
    pub owner_after: Option<OrganismId>,
    pub consumed_after: bool,
    pub selected_command: ChannelCommand,
    pub physical: PhysicalActionOutcome,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationCurriculumRewardRow {
    pub row: usize,
    pub physiological_reward: f32,
    pub curriculum_reward: f32,
    pub combined_reward: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FoodOwnership {
    owner: Option<OrganismId>,
    consumed: bool,
}
pub(crate) fn food_ownership(
    world: &alife_world::HeadlessWorld,
    target: WorldEntityId,
) -> Option<FoodOwnership> {
    world.entity(target).map(|food| FoodOwnership {
        owner: food.carried_by,
        consumed: food.consumed,
    })
}

/// Latches the first genuine learner acquisition in this newly prepared life.
/// Release/regrab and bootstrap-only events cannot create another loss credit.
#[derive(Debug, Default)]
pub(crate) struct GrabRewardGate {
    first_acquisition_row: Option<usize>,
}
impl GrabRewardGate {
    pub(crate) fn observe(&mut self, row: usize) {
        self.first_acquisition_row.get_or_insert(row);
    }
    pub(crate) fn reward(&self, row: usize, train_rows: usize) -> f32 {
        if row < train_rows && self.first_acquisition_row == Some(row) {
            GRAB_FOOD_ACQUISITION_REWARD
        } else {
            0.0
        }
    }
}

pub(crate) fn observe_grab_acquisition(
    setup: &FoundationGrabFoodSetup,
    before: Option<FoodOwnership>,
    world: &alife_world::HeadlessWorld,
    step: &FoundationTrainingStep,
    row: usize,
) -> Result<Option<FoundationGrabAcquisition>> {
    let after = food_ownership(world, setup.target);
    if before
        != Some(FoodOwnership {
            owner: None,
            consumed: false,
        })
        || after
            != Some(FoodOwnership {
                owner: Some(setup.organism),
                consumed: false,
            })
        || step.behavior.on_policy_log_probability().is_err()
    {
        return Ok(None);
    }
    let patch = &step.patch;
    patch.validate_contract()?;
    let evidence = patch.decision().neural_evidence()?;
    if step.frame.organism_id() != setup.organism
        || patch.header().organism_id != setup.organism
        || step.behavior.organism_id != setup.organism.raw()
        || step.behavior.tick != step.frame.tick().raw()
        || step.behavior.frame_digest != step.frame.frame_digest()
        || patch.pre_action().perception().frame_digest() != step.frame.frame_digest()
        || evidence.frame_digest != step.frame.frame_digest()
        || evidence.dispatch_generation != step.behavior.dispatch_generation
        || patch.outcome().outcome_tick != world.tick()
        || step.frame.tick().raw() < setup.setup_tick
    {
        return Err("Grab acquisition has inconsistent sealed learner identity/timing".into());
    }
    let Some(bundle) = patch.selected_bundle() else {
        return Ok(None);
    };
    let Some(command) = bundle.channels.iter().find(|command| {
        command.channel == MotorChannel::Manipulation
            && command.primitive == alife_world::HeadlessActionIds::GRAB
            && command.target.and_then(|target| target.entity) == Some(setup.target)
    }) else {
        return Ok(None);
    };
    let Some(candidate) = step
        .frame
        .candidates()
        .get(step.behavior.motor_indices[2] as usize)
    else {
        return Ok(None);
    };
    if candidate.action_id != command.primitive || candidate.target.entity != Some(setup.target) {
        return Err("Grab acquisition differs from sampled GPU manipulation action".into());
    }
    let Some(physical) = patch
        .outcome()
        .joint
        .as_ref()
        .and_then(|joint| {
            joint.channel_outcomes.iter().find(|outcome| {
                outcome.channel == MotorChannel::Manipulation
                    && outcome.physical.contact == PhysicalContactKind::Touch
                    && outcome.physical.target_entity == Some(setup.target)
            })
        })
        .map(|outcome| outcome.physical)
    else {
        return Ok(None);
    };
    Ok(Some(FoundationGrabAcquisition {
        row,
        organism: setup.organism,
        target: setup.target,
        sequence_id: patch.header().sequence_id.raw(),
        decision_tick: step.frame.tick().raw(),
        outcome_tick: patch.outcome().outcome_tick.raw(),
        owner_before: before.and_then(|state| state.owner),
        owner_after: after.and_then(|state| state.owner),
        consumed_after: after.is_some_and(|state| state.consumed),
        selected_command: command.clone(),
        physical,
    }))
}

pub(crate) fn curriculum_reward_row(
    row: usize,
    physiological_reward: f32,
    curriculum_reward: f32,
) -> Result<FoundationCurriculumRewardRow> {
    if !physiological_reward.is_finite()
        || !curriculum_reward.is_finite()
        || !(0.0..=GRAB_FOOD_ACQUISITION_REWARD).contains(&curriculum_reward)
    {
        return Err("invalid Grab curriculum reward components".into());
    }
    Ok(FoundationCurriculumRewardRow {
        row,
        physiological_reward,
        curriculum_reward,
        combined_reward: physiological_reward + curriculum_reward,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn released_and_regrabbed_food_gets_one_credit_per_life() {
        let mut gate = GrabRewardGate::default();
        gate.observe(2);
        gate.observe(5);
        assert_eq!(gate.reward(2, 7), GRAB_FOOD_ACQUISITION_REWARD);
        assert_eq!(gate.reward(5, 7), 0.0);
        assert_eq!(
            (0..7).map(|row| gate.reward(row, 7)).sum::<f32>(),
            GRAB_FOOD_ACQUISITION_REWARD
        );
    }
    #[test]
    fn bootstrap_acquisition_has_no_training_reward() {
        let mut gate = GrabRewardGate::default();
        gate.observe(4);
        assert_eq!((0..=4).map(|row| gate.reward(row, 4)).sum::<f32>(), 0.0);
    }
    #[test]
    fn no_learner_acquisition_has_no_curriculum_credit() {
        assert_eq!(GrabRewardGate::default().reward(0, 16), 0.0);
    }
    #[test]
    fn curriculum_credit_is_separate_from_measured_physiological_value() {
        let component = curriculum_reward_row(3, -0.07, GRAB_FOOD_ACQUISITION_REWARD).unwrap();
        assert_eq!(component.physiological_reward, -0.07);
        assert_eq!(component.curriculum_reward, 0.25);
        assert_eq!(component.combined_reward, 0.18);
        assert!(curriculum_reward_row(3, 0.0, 0.251).is_err());
    }
}
