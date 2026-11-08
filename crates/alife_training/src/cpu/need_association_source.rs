//! Private native source binding, reconstructed after environment loss.
//! Only the canonical CPU host calls this after actual selection and execution.
//! Fabricated unit receipts test physical admission, never neural provenance.
use alife_core::*;
use alife_world::{
    ActionPhase, HeadlessActionIds, HeadlessMotorChannelReceipt, HeadlessMotorTransactionReceipt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceRejection {
    Contract,
    NonCpuOrigin,
    MissingMeasuredInterval,
    BindingMismatch,
    UnobservedRepresentative,
    MissingOrDuplicateChannel,
    AdditionalMeaningfulCommand,
}
impl SourceRejection {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Contract => "contract",
            Self::NonCpuOrigin => "non_cpu_origin",
            Self::MissingMeasuredInterval => "missing_measured_interval",
            Self::BindingMismatch => "physical_or_biology_binding",
            Self::UnobservedRepresentative => "unsupported_representative",
            Self::MissingOrDuplicateChannel => "channel_coverage",
            Self::AdditionalMeaningfulCommand => "additional_meaningful_command",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub(super) enum AssociationSubjectV1 {
    Observed {
        tracked: TrackedObjectId,
        entity: WorldEntityId,
    },
    SelfAction {
        organism: OrganismId,
        action: SelfActionV1,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum SelfActionV1 {
    Rest,
    Noop,
}

/// Association includes ambient contact and raw drift; it is not isolated credit.
#[derive(Debug)]
pub(super) struct IntervalAssociation {
    pub source_sequence: ExperienceSequenceId,
    pub source_frame_digest: PerceptionFrameDigest,
    pub subject: AssociationSubjectV1,
    pub representative_channel: MotorChannel,
    pub representative_physical: PhysicalActionOutcome,
    pub benign_noop_channels: usize,
    pub drive_before: [f32; 11],
    pub drive_after: [f32; 11],
    pub raw_delta: [f32; 11],
}
pub(super) fn candidate_subject(
    frame: &PerceptionFrame,
    candidate: &ActionCandidate,
) -> Result<AssociationSubjectV1, SourceRejection> {
    match candidate.observation {
        CandidateObservationRef::ObjectSlot(i) if candidate.kind == ActionKind::Inspect => {
            let slot = frame
                .grounded_object_slots()
                .get(usize::from(i))
                .ok_or(SourceRejection::UnobservedRepresentative)?;
            Ok(AssociationSubjectV1::Observed {
                tracked: slot.tracked_object_id,
                entity: candidate
                    .target
                    .entity
                    .ok_or(SourceRejection::UnobservedRepresentative)?,
            })
        }
        CandidateObservationRef::None if candidate.target == ActionTarget::NONE => {
            let action = if candidate.kind == ActionKind::Rest
                && candidate.action_id == ActionKind::Rest.canonical_id()
            {
                SelfActionV1::Rest
            } else if (candidate.kind == ActionKind::Idle
                && candidate.action_id == ActionKind::Idle.canonical_id())
                || (candidate.kind == ActionKind::Hold
                    && candidate.action_id == HeadlessActionIds::NO_POSTURE)
            {
                SelfActionV1::Noop
            } else {
                return Err(SourceRejection::UnobservedRepresentative);
            };
            Ok(AssociationSubjectV1::SelfAction {
                organism: frame.organism_id(),
                action,
            })
        }
        _ => Err(SourceRejection::UnobservedRepresentative),
    }
}
fn representative_channel(kind: ActionKind) -> MotorChannel {
    match kind {
        ActionKind::Idle
        | ActionKind::Hold
        | ActionKind::Rest
        | ActionKind::Inspect
        | ActionKind::Gesture => MotorChannel::Posture,
        ActionKind::Move => MotorChannel::Locomotion,
        ActionKind::Look => MotorChannel::Orientation,
        ActionKind::Interact | ActionKind::Write => MotorChannel::Manipulation,
        ActionKind::Vocalize => MotorChannel::Vocal,
    }
}
/// Exact no-op primitives only, not blocked or tiny movement. Selected command
/// intensity may be nonzero; measured locomotion intensity must be exactly zero.
fn benign_noop(receipt: &HeadlessMotorChannelReceipt) -> bool {
    let command = &receipt.command;
    let primitive_matches = match command.channel {
        MotorChannel::Locomotion => command.primitive == HeadlessActionIds::NO_LOCOMOTION,
        MotorChannel::Manipulation => command.primitive == HeadlessActionIds::NO_MANIPULATION,
        MotorChannel::Posture => command.primitive == HeadlessActionIds::NO_POSTURE,
        _ => false,
    };
    let physical = receipt.physical;
    let observation_matches = match (command.channel, receipt.observation) {
        (MotorChannel::Locomotion, Some(observation)) => {
            observation.channel == command.channel
                && observation.executed
                && observation.measured_intensity.raw() == 0.
                && observation.displacement == Vec3f::ZERO
        }
        (MotorChannel::Manipulation | MotorChannel::Posture, None) => true,
        _ => false,
    };
    primitive_matches
        && command.validate_contract().is_ok()
        && command.target.is_none()
        && command.direction == Vec3f::ZERO
        && command.payload.values.is_empty()
        && receipt.phase == ActionPhase::Completed
        && receipt.grip_transfers.is_empty()
        && receipt.play_stimulation == 0.
        && receipt.investigation == 0.
        && physical.contact == PhysicalContactKind::None
        && physical.target_entity.is_none()
        && physical.displacement == Vec3f::ZERO
        && physical.collision_normal.is_none()
        && physical.energy_cost.raw() == 0.
        && observation_matches
}
/// canonical_after is the actual host state after final cognitive charging,
/// not the world's earlier biology_after receipt.
pub(super) fn qualify_native_interval(
    patch: &ExperiencePatch,
    receipt: &HeadlessMotorTransactionReceipt,
    canonical_after: &BiochemistryState,
) -> Result<IntervalAssociation, SourceRejection> {
    patch
        .validate_contract()
        .map_err(|_| SourceRejection::Contract)?;
    let pre = patch.pre_action();
    if !matches!(
        pre.brain_evidence,
        PreActionBrainEvidence::NeuralClosedLoopCpu { .. }
    ) {
        return Err(SourceRejection::NonCpuOrigin);
    }
    let measured = patch
        .outcome()
        .measured_physiology
        .as_ref()
        .ok_or(SourceRejection::MissingMeasuredInterval)?;
    let bundle = patch
        .selected_bundle()
        .ok_or(SourceRejection::BindingMismatch)?;
    if bundle != &receipt.bundle
        || patch.outcome().joint.as_ref() != Some(&receipt.joint)
        || measured.before != receipt.biology_before
        || measured.before.source_genome_id != pre.genome_id
        || measured.before.tick != pre.tick
        || measured.before.homeostasis != *pre.homeostasis()
        || &measured.after != canonical_after
        || receipt.outcome_tick != patch.outcome().outcome_tick
        || measured.after.tick != receipt.biology_after.tick
        || bundle.sequence_id != patch.header().sequence_id
        || bundle.organism_id != patch.header().organism_id
    {
        return Err(SourceRejection::BindingMismatch);
    }
    let decision = patch.decision();
    let evidence = decision
        .neural_evidence()
        .map_err(|_| SourceRejection::Contract)?;
    let frame = pre.perception();
    let candidate = frame
        .candidates()
        .get(usize::from(evidence.candidate_index))
        .ok_or(SourceRejection::Contract)?;
    let subject = candidate_subject(frame, candidate)?;
    let channel = representative_channel(candidate.kind);
    let expected = channel_command_for_action(channel, &decision.selected_action)
        .map_err(|_| SourceRejection::Contract)?;
    if receipt.channel_receipts.len() != bundle.channels.len()
        || receipt.joint.channel_outcomes.len() != bundle.channels.len()
    {
        return Err(SourceRejection::MissingOrDuplicateChannel);
    }
    let mut primary = None;
    let mut benign = 0;
    for command in &bundle.channels {
        let matching = receipt
            .channel_receipts
            .iter()
            .filter(|r| r.command.channel == command.channel)
            .collect::<Vec<_>>();
        let outcomes = receipt
            .joint
            .channel_outcomes
            .iter()
            .filter(|r| r.channel == command.channel)
            .collect::<Vec<_>>();
        if matching.len() != 1 || outcomes.len() != 1 {
            return Err(SourceRejection::MissingOrDuplicateChannel);
        }
        let row = matching[0];
        if &row.command != command || row.physical != outcomes[0].physical {
            return Err(SourceRejection::BindingMismatch);
        }
        if command.channel == channel {
            let subject_matches = match subject {
                AssociationSubjectV1::Observed { entity, .. } => {
                    row.physical.target_entity == Some(entity)
                }
                AssociationSubjectV1::SelfAction { .. } => {
                    row.physical.target_entity.is_none()
                        && row.physical.contact == PhysicalContactKind::None
                        && row.physical.displacement == Vec3f::ZERO
                        && row.physical.collision_normal.is_none()
                }
            };
            if row.command != expected || !subject_matches || row.phase != ActionPhase::Completed {
                return Err(SourceRejection::BindingMismatch);
            }
            primary = Some(row.physical);
        } else if benign_noop(row) {
            benign += 1;
        } else {
            return Err(SourceRejection::AdditionalMeaningfulCommand);
        }
    }
    let drive_before = measured.before.homeostasis.drives.to_array();
    let drive_after = measured.after.homeostasis.drives.to_array();
    Ok(IntervalAssociation {
        source_sequence: patch.header().sequence_id,
        source_frame_digest: frame.frame_digest(),
        subject,
        representative_channel: channel,
        representative_physical: primary.ok_or(SourceRejection::MissingOrDuplicateChannel)?,
        benign_noop_channels: benign,
        drive_before,
        drive_after,
        raw_delta: std::array::from_fn(|i| drive_after[i] - drive_before[i]),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn noop() -> HeadlessMotorChannelReceipt {
        HeadlessMotorChannelReceipt {
            command: ChannelCommand::new(
                MotorChannel::Locomotion,
                HeadlessActionIds::NO_LOCOMOTION,
                None,
                Vec3f::ZERO,
                Intensity::new(1.).unwrap(),
                DurationTicks::new(1),
                0.,
                Confidence::new(1.).unwrap(),
                0,
            )
            .unwrap(),
            phase: ActionPhase::Completed,
            grip_transfers: Vec::new(),
            play_stimulation: 0.,
            investigation: 0.,
            observation: Some(
                MeasuredChannelObservation::new(
                    MotorChannel::Locomotion,
                    true,
                    NormalizedScalar::ZERO,
                    Vec3f::ZERO,
                )
                .unwrap(),
            ),
            physical: PhysicalActionOutcome {
                contact: PhysicalContactKind::None,
                target_entity: None,
                displacement: Vec3f::ZERO,
                collision_normal: None,
                energy_cost: NormalizedScalar::ZERO,
            },
        }
    }
    #[test]
    fn explicit_noop_with_normal_selected_intensity_is_benign() {
        assert!(benign_noop(&noop()));
    }
    #[test]
    fn blocked_movement_is_not_a_benign_noop() {
        let mut r = noop();
        r.command.primitive = HeadlessActionIds::APPROACH;
        assert!(!benign_noop(&r));
        r.command.primitive = HeadlessActionIds::NO_LOCOMOTION;
        r.physical.contact = PhysicalContactKind::Blocked;
        assert!(!benign_noop(&r));
    }
    #[test]
    fn any_displacement_or_energy_cost_rejects() {
        let mut r = noop();
        r.physical.displacement.x = f32::MIN_POSITIVE;
        assert!(!benign_noop(&r));
        r.physical.displacement = Vec3f::ZERO;
        r.physical.energy_cost = NormalizedScalar::new(0.001).unwrap();
        assert!(!benign_noop(&r));
    }
    #[test]
    fn missing_or_nonzero_measured_locomotion_rejects() {
        let mut r = noop();
        r.observation = None;
        assert!(!benign_noop(&r));
        r = noop();
        r.observation.as_mut().unwrap().measured_intensity = NormalizedScalar::new(0.001).unwrap();
        assert!(!benign_noop(&r));
    }
    #[test]
    fn other_biological_effects_rejects() {
        let mut r = noop();
        r.play_stimulation = 0.01;
        assert!(!benign_noop(&r));
        r = noop();
        r.investigation = 0.01;
        assert!(!benign_noop(&r));
    }
}
