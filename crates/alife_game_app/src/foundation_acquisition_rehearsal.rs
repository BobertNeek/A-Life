//! Opt-in offline rehearsal of a sealed, successful learner acquisition.
//! This adds no policy, action label, or reward to ordinary organism execution.
use std::path::Path;

use alife_core::{MotorChannel, PhysicalContactKind, ScaffoldContractError};
use alife_training::{
    train_recurrent_imitation, FoundationTrainer, ImitationExample, ImitationTarget,
    ImitationTrainingWindow, PpoTrainingState,
};

use crate::{
    FoundationGrabAcquisition, FoundationReplayBudget, FoundationReplayRecordRef,
    FoundationReplaySource,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MAX_REHEARSAL_EPOCHS: u32 = 512;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundationAcquisitionRehearsalReceipt {
    pub requested_epochs: u32,
    pub completed_epochs: u32,
    pub acquisition_row: Option<usize>,
    pub burn_in_rows: usize,
    pub loss_rows_per_epoch: usize,
    pub imitation_temperature: f32,
    pub action_losses: Vec<f32>,
    pub actor_optimizer_step_before: u32,
    pub actor_optimizer_step_after: u32,
    pub value_checkpoint_unchanged: bool,
}

pub(crate) fn validate_rehearsal_epochs(epochs: u32) -> Result<()> {
    if !(1..=MAX_REHEARSAL_EPOCHS).contains(&epochs) {
        return Err(format!(
            "acquisition rehearsal requires 1..={MAX_REHEARSAL_EPOCHS} offline epochs"
        )
        .into());
    }
    Ok(())
}

fn acquisition_target(
    behavior: &alife_gpu_backend::GpuTrainingRolloutReceipt,
    event: &FoundationGrabAcquisition,
) -> Result<ImitationExample> {
    behavior.on_policy_log_probability()?;
    let index = usize::from(behavior.motor_indices[2]);
    if event.organism.raw() != behavior.organism_id
        || event.decision_tick != behavior.tick
        || event.owner_before.is_some()
        || event.owner_after != Some(event.organism)
        || event.consumed_after
        || event.physical.contact != PhysicalContactKind::Touch
        || event.physical.target_entity != Some(event.target)
        || event.selected_command.channel != MotorChannel::Manipulation
        || event.selected_command.primitive != alife_world::HeadlessActionIds::GRAB
        || event
            .selected_command
            .target
            .and_then(|target| target.entity)
            != Some(event.target)
        || index >= behavior.forced_motor_slots.len()
        || index >= 32
        || behavior.forced_motor_slots[index] != 2
    {
        return Err(
            "acquisition rehearsal requires the exact sampled successful learner Grab".into(),
        );
    }
    let mut motors = [None; 6];
    motors[2] = Some(1u32 << index);
    let example = ImitationExample {
        candidate_count: u16::try_from(behavior.forced_motor_slots.len())?,
        representative_mask: behavior.representative_mask,
        motor_masks: behavior.motor_masks,
        forced_slots: behavior
            .forced_motor_slots
            .iter()
            .copied()
            .map(Some)
            .collect(),
        target: ImitationTarget {
            representative: None,
            motors,
        },
    };
    example.validate()?;
    Ok(example)
}

pub(crate) struct AcquisitionRehearsalContext<'a> {
    pub directory: &'a Path,
    pub references: &'a [FoundationReplayRecordRef],
    pub train_rows: usize,
    pub source: &'a FoundationReplaySource,
    pub phenotype: &'a alife_core::BrainPhenotype,
    pub budget: FoundationReplayBudget,
}

fn rehearsal_window(
    references: &[FoundationReplayRecordRef],
    train_rows: usize,
    row: usize,
) -> Result<(usize, usize)> {
    if row >= train_rows {
        return Err("bootstrap rows cannot become acquisition rehearsal losses".into());
    }
    let reference = references
        .get(row)
        .ok_or("acquisition row is outside replay")?;
    let segment_start = references[..row]
        .iter()
        .rposition(|previous| previous.segment != reference.segment)
        .map_or(0, |previous| previous + 1);
    Ok((row.saturating_sub(128).max(segment_start), row + 1))
}

pub(crate) fn rehearse_acquisition(
    trainer: &mut FoundationTrainer,
    value: &mut PpoTrainingState,
    epochs: u32,
    event: Option<&FoundationGrabAcquisition>,
    context: AcquisitionRehearsalContext<'_>,
) -> Result<FoundationAcquisitionRehearsalReceipt> {
    validate_rehearsal_epochs(epochs)?;
    let before = trainer.optimizer_step();
    let mut receipt = FoundationAcquisitionRehearsalReceipt {
        requested_epochs: epochs,
        completed_epochs: 0,
        acquisition_row: None,
        burn_in_rows: 0,
        loss_rows_per_epoch: 0,
        imitation_temperature: 1.0,
        action_losses: Vec::new(),
        actor_optimizer_step_before: before,
        actor_optimizer_step_after: before,
        value_checkpoint_unchanged: true,
    };
    let Some(event) = event else {
        return Ok(receipt);
    };
    let (burn_start, end) = rehearsal_window(context.references, context.train_rows, event.row)?;
    let value_before = serde_json::to_vec(&value.checkpoint(trainer.session())?)?;
    let losses = train_recurrent_imitation(trainer, value, 1, 1, epochs, 1.0, 1.0, |_| {
        let replay = crate::load_foundation_replay_window(
            context.directory,
            &context.references[burn_start..end],
            event.row - burn_start,
            context.source,
            context.phenotype,
            context.budget,
        )
        .map_err(|_| {
            alife_training::TrainingError::from(ScaffoldContractError::InvalidDecisionEvidence)
        })?;
        let patch = replay
            .patches
            .last()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if patch.header().sequence_id.raw() != event.sequence_id
            || patch.header().organism_id != event.organism
            || patch.outcome().outcome_tick.raw() != event.outcome_tick
            || !patch.selected_bundle().is_some_and(|bundle| {
                bundle
                    .channels
                    .iter()
                    .any(|command| command == &event.selected_command)
            })
            || !patch.outcome().joint.as_ref().is_some_and(|joint| {
                joint.channel_outcomes.iter().any(|outcome| {
                    outcome.channel == MotorChannel::Manipulation
                        && outcome.physical == event.physical
                })
            })
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let example = acquisition_target(
            replay
                .behavior
                .last()
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?,
            event,
        )
        .map_err(|_| {
            alife_training::TrainingError::from(ScaffoldContractError::InvalidDecisionEvidence)
        })?;
        Ok(ImitationTrainingWindow {
            speech_targets: vec![None; replay.behavior.len()],
            sequence: replay.sequence,
            examples: vec![example],
            episode_weight: 1.0,
        })
    })?;
    let after = trainer.optimizer_step();
    if after.checked_sub(before) != Some(epochs)
        || losses.len() != epochs as usize
        || losses.iter().any(|loss| !loss.is_finite())
        || serde_json::to_vec(&value.checkpoint(trainer.session())?)? != value_before
    {
        return Err(
            "acquisition rehearsal did not preserve value state or complete its actor updates"
                .into(),
        );
    }
    receipt.completed_epochs = epochs;
    receipt.acquisition_row = Some(event.row);
    receipt.burn_in_rows = event.row - burn_start;
    receipt.loss_rows_per_epoch = 1;
    receipt.action_losses = losses;
    receipt.actor_optimizer_step_after = after;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acquisition() -> (
        alife_gpu_backend::GpuTrainingRolloutReceipt,
        FoundationGrabAcquisition,
    ) {
        use alife_core::{
            ActionTarget, ChannelCommand, Confidence, DurationTicks, Intensity, NormalizedScalar,
            OrganismId, PerceptionFrameDigest, PhysicalActionOutcome, Vec3f, WorldEntityId,
        };
        let organism = OrganismId::new(1).unwrap();
        let target = WorldEntityId::new(2).unwrap();
        let behavior = alife_gpu_backend::GpuTrainingRolloutReceipt {
            organism_id: 1,
            tick: 123,
            dispatch_generation: 1,
            frame_digest: PerceptionFrameDigest([1; 4]),
            active_weight_generation: 1,
            sampling: alife_gpu_backend::GpuTrainingSamplingConfig {
                seed: 1,
                counter: 0,
                temperature: 32.0,
                demonstrator: None,
            },
            logits: vec![0.0; 3],
            decoder_inputs: vec![0.0; 72],
            decoder_input_stride: 24,
            representative_mask: 7,
            motor_masks: [2, 0, 0, 0, 0, 0],
            forced_motor_slots: vec![4, 0, 2],
            representative_index: 2,
            motor_indices: [1, u16::MAX, 2, u16::MAX, u16::MAX, u16::MAX],
            factor_log_probabilities: [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            behavior: alife_gpu_backend::GpuTrainingBehaviorKind::OnPolicy,
            joint_log_probability: Some(-1.0),
            policy_joint_log_probability: -1.0,
        };
        let event = FoundationGrabAcquisition {
            row: 4,
            organism,
            target,
            sequence_id: 5,
            decision_tick: 123,
            outcome_tick: 124,
            owner_before: None,
            owner_after: Some(organism),
            consumed_after: false,
            selected_command: ChannelCommand::new(
                MotorChannel::Manipulation,
                alife_world::HeadlessActionIds::GRAB,
                Some(ActionTarget::new(Some(target), None)),
                Vec3f::new(0.0, 0.0, 0.0),
                Intensity::new(1.0).unwrap(),
                DurationTicks::new(1),
                0.0,
                Confidence::new(0.9).unwrap(),
                0,
            )
            .unwrap(),
            physical: PhysicalActionOutcome {
                contact: PhysicalContactKind::Touch,
                target_entity: Some(target),
                displacement: Vec3f::new(0.0, 0.0, 0.0),
                collision_normal: None,
                energy_cost: NormalizedScalar::new(0.0).unwrap(),
            },
        };
        (behavior, event)
    }

    #[test]
    fn rehearsal_epoch_budget_is_bounded() {
        assert!(validate_rehearsal_epochs(0).is_err());
        assert!(validate_rehearsal_epochs(1).is_ok());
        assert!(validate_rehearsal_epochs(32).is_ok());
        assert!(validate_rehearsal_epochs(512).is_ok());
        assert!(validate_rehearsal_epochs(513).is_err());
        assert!(validate_rehearsal_epochs(u32::MAX).is_err());
    }

    #[test]
    fn only_the_successful_manipulation_is_supervised() {
        let (behavior, event) = acquisition();
        let example = acquisition_target(&behavior, &event).unwrap();
        assert_eq!(example.target.representative, None);
        assert_eq!(
            example.target.motors,
            [None, None, Some(4), None, None, None]
        );
        assert_eq!(example.motor_masks[2], 0); // Representative forcing remains sufficient support.
    }

    #[test]
    fn false_or_foreign_acquisitions_and_demonstrators_are_rejected() {
        let (mut behavior, event) = acquisition();
        let mut false_event = event.clone();
        false_event.owner_before = Some(event.organism);
        assert!(acquisition_target(&behavior, &false_event).is_err());
        false_event = event.clone();
        false_event.decision_tick += 1;
        assert!(acquisition_target(&behavior, &false_event).is_err());
        false_event = event.clone();
        false_event.consumed_after = true;
        assert!(acquisition_target(&behavior, &false_event).is_err());
        behavior.organism_id = 2;
        assert!(acquisition_target(&behavior, &event).is_err());
        behavior.organism_id = 1;
        behavior.behavior = alife_gpu_backend::GpuTrainingBehaviorKind::Demonstrator;
        assert!(acquisition_target(&behavior, &event).is_err());
    }

    #[test]
    fn rehearse_one_real_loss_row_with_bounded_same_segment_burn_in() {
        let references = (0..300)
            .map(|row| FoundationReplayRecordRef {
                record_index: row,
                segment: u64::from(row >= 220),
                tick: row,
                encoded_bytes: 1,
                digest: [1; 32],
            })
            .collect::<Vec<_>>();
        assert_eq!(rehearsal_window(&references, 299, 0).unwrap(), (0, 1));
        assert_eq!(rehearsal_window(&references, 299, 200).unwrap(), (72, 201));
        assert_eq!(rehearsal_window(&references, 299, 270).unwrap(), (220, 271));
        assert!(rehearsal_window(&references, 299, 299).is_err());
        assert!(rehearsal_window(&references, 301, 300).is_err());
    }
}
