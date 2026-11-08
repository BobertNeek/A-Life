//! Private native source binding, reconstructed after environment loss.
//! Only the canonical CPU host calls this after actual selection and execution.
//! Fabricated unit receipts test physical admission, never neural provenance.
use alife_core::*;
use alife_world::{
    ActionPhase, HeadlessActionIds, HeadlessMotorChannelReceipt, HeadlessMotorTransactionReceipt,
    PreparedMotorChannelAdapter, WorldOrganismRecord,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceRejection {
    Contract,
    NonCpuOrigin,
    MissingMeasuredInterval,
    BindingMismatch(&'static str),
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
            Self::BindingMismatch(field) => field,
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

/// Compact provenance for a fresh diagnostic journal, never a restore input.
/// Commands are retained as requested and actually adapted, not reconstructed
/// after a failed life. This does not grant synthetic source admission.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedSourceAuditV4 {
    pub requested_channels: Vec<ChannelCommand>,
    pub executed_channels: Vec<CpuNeedChannelAuditV4>,
    pub before_record_equals_world: bool,
    pub measured_before_equals_world: bool,
    pub measured_after_equals_final_host: bool,
    pub world_after_equals_final_host: bool,
    pub biology_before: CpuNeedBiologyCostV4,
    pub biology_world_after: CpuNeedBiologyCostV4,
    pub biology_final_after: CpuNeedBiologyCostV4,
}
/// Numeric energy costs remain visible even when the ATP receptor is flat.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CpuNeedBiologyCostV4 {
    pub tick: Tick,
    pub body_energy: f32,
    pub organ_energy: [f32; ORGAN_KIND_COUNT],
    pub health: f32,
    pub injury: f32,
    pub fatigue: f32,
    pub brain_atp: f32,
    pub boredom: f32,
}
impl CpuNeedBiologyCostV4 {
    pub fn from_biology(b: &BiochemistryState) -> Self {
        Self {
            tick: b.tick,
            body_energy: b.body.energy,
            organ_energy: std::array::from_fn(|i| b.body.organs()[i].energy),
            health: b.body.health,
            injury: b.body.injury,
            fatigue: b.homeostasis.drives.fatigue,
            brain_atp: b.homeostasis.drives.brain_atp,
            boredom: b.homeostasis.drives.to_array()[9],
        }
    }
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedChannelAuditV4 {
    pub command: ChannelCommand,
    pub phase: String,
    pub physical: PhysicalActionOutcome,
    pub observation: Option<MeasuredChannelObservation>,
}
pub(super) fn source_audit(
    patch: &ExperiencePatch,
    receipt: &HeadlessMotorTransactionReceipt,
    before: &WorldOrganismRecord,
    after: &BiochemistryState,
) -> CpuNeedSourceAuditV4 {
    let measured = patch.outcome().measured_physiology.as_ref();
    CpuNeedSourceAuditV4 {
        requested_channels: receipt.bundle.channels.clone(),
        executed_channels: receipt
            .channel_receipts
            .iter()
            .map(|r| CpuNeedChannelAuditV4 {
                command: r.command.clone(),
                phase: format!("{:?}", r.phase),
                physical: r.physical,
                observation: r.observation,
            })
            .collect(),
        before_record_equals_world: before.biochemistry() == &receipt.biology_before,
        measured_before_equals_world: measured.is_some_and(|m| m.before == receipt.biology_before),
        measured_after_equals_final_host: measured.is_some_and(|m| &m.after == after),
        world_after_equals_final_host: &receipt.biology_after == after,
        biology_before: CpuNeedBiologyCostV4::from_biology(&receipt.biology_before),
        biology_world_after: CpuNeedBiologyCostV4::from_biology(&receipt.biology_after),
        biology_final_after: CpuNeedBiologyCostV4::from_biology(after),
    }
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
            } else if (candidate.kind == ActionKind::Move
                && candidate.action_id == HeadlessActionIds::NO_LOCOMOTION)
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
/// Check measured biology against both the world interval and final host charge.
/// The world receipt's earlier after-state must never replace the final state.
fn measured_interval_binding(
    measured: &MeasuredPhysiologyTransition,
    world_before: &BiochemistryState,
    world_after: &BiochemistryState,
    canonical_before: &BiochemistryState,
    canonical_after: &BiochemistryState,
) -> Result<(), SourceRejection> {
    if &measured.before != world_before {
        return Err(SourceRejection::BindingMismatch(
            "measured_before_world_receipt",
        ));
    }
    if world_before != canonical_before {
        return Err(SourceRejection::BindingMismatch(
            "world_before_native_record",
        ));
    }
    if &measured.after != canonical_after {
        return Err(SourceRejection::BindingMismatch(
            "measured_after_final_host_charge",
        ));
    }
    if measured.after.tick != world_after.tick {
        return Err(SourceRejection::BindingMismatch(
            "measured_after_world_tick",
        ));
    }
    Ok(())
}

/// Requested identity and physically adapted command are separate exact checks.
/// The same prepared adapter is used by canonical world execution; no approximate
/// duplicate of the body's adaptation equation is permitted here.
fn adapted_channel_binding(
    requested: &ChannelCommand,
    actual: &HeadlessMotorChannelReceipt,
    adapter: &PreparedMotorChannelAdapter,
) -> Result<(), SourceRejection> {
    let expected = adapter
        .adapt(requested)
        .map_err(|_| SourceRejection::BindingMismatch("body_adaptation_rejected"))?;
    if actual.command != expected {
        return Err(SourceRejection::BindingMismatch("adapted_channel_command"));
    }
    Ok(())
}

/// canonical_after is the actual host state after final cognitive charging,
/// not the world's earlier biology_after receipt. This private entry point is
/// called only after genuine canonical neural selection and physical execution.
pub(super) fn qualify_native_interval(
    patch: &ExperiencePatch,
    receipt: &HeadlessMotorTransactionReceipt,
    canonical_before: &WorldOrganismRecord,
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
        .ok_or(SourceRejection::BindingMismatch("missing_native_bundle"))?;
    for (matches, field) in [
        (bundle == &receipt.bundle, "selected_bundle_identity"),
        (
            patch.outcome().joint.as_ref() == Some(&receipt.joint),
            "joint_receipt_identity",
        ),
        (
            measured.before.source_genome_id == pre.genome_id,
            "before_genome",
        ),
        (measured.before.tick == pre.tick, "before_tick"),
        (
            measured.before.homeostasis == *pre.homeostasis(),
            "before_homeostasis",
        ),
        (
            receipt.outcome_tick == patch.outcome().outcome_tick,
            "outcome_tick",
        ),
        (
            bundle.sequence_id == patch.header().sequence_id,
            "bundle_sequence",
        ),
        (
            bundle.organism_id == patch.header().organism_id,
            "bundle_owner",
        ),
        (
            canonical_before.organism_id() == bundle.organism_id,
            "native_record_owner",
        ),
    ] {
        if !matches {
            return Err(SourceRejection::BindingMismatch(field));
        }
    }
    measured_interval_binding(
        measured,
        &receipt.biology_before,
        &receipt.biology_after,
        canonical_before.biochemistry(),
        canonical_after,
    )?;
    let adapter = PreparedMotorChannelAdapter::from_record(canonical_before)
        .map_err(|_| SourceRejection::BindingMismatch("body_adapter_preparation"))?;
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
    let expected_requested = channel_command_for_action(channel, &decision.selected_action)
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
        adapted_channel_binding(command, row, &adapter)?;
        if row.physical != outcomes[0].physical {
            return Err(SourceRejection::BindingMismatch("channel_physical_receipt"));
        }
        if command.channel == channel {
            if command != &expected_requested {
                return Err(SourceRejection::BindingMismatch(
                    "representative_requested_command",
                ));
            }
            let subject_matches = match subject {
                AssociationSubjectV1::Observed { entity, .. } => {
                    row.physical.target_entity == Some(entity)
                }
                AssociationSubjectV1::SelfAction {
                    action: SelfActionV1::Noop,
                    ..
                } => benign_noop(row),
                AssociationSubjectV1::SelfAction {
                    action: SelfActionV1::Rest,
                    ..
                } => {
                    row.physical.target_entity.is_none()
                        && row.physical.contact == PhysicalContactKind::None
                        && row.physical.displacement == Vec3f::ZERO
                        && row.physical.collision_normal.is_none()
                }
            };
            if !subject_matches {
                return Err(SourceRejection::BindingMismatch(
                    "representative_physical_subject",
                ));
            }
            if row.phase != ActionPhase::Completed {
                return Err(SourceRejection::BindingMismatch(
                    "representative_incomplete_phase",
                ));
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
    use alife_core::cognitive_work::{CognitiveWorkCostPolicy, CognitiveWorkCounters};
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
    fn calibration_record() -> WorldOrganismRecord {
        let mut config = alife_world::CanonicalNewGameConfig::phase3(0xB1D, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
        let asset = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile).unwrap();
        let game =
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
        let mut record = game
            .world
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .clone();
        let mut body = record.embodiment().clone();
        body.replace_calibration(
            record.biochemistry().tick,
            body.sensor_calibration().to_vec(),
            vec![-0.5; body.effector_controllability().len()],
            body.body_schema().to_vec(),
        )
        .unwrap();
        record.replace_embodiment_state(body).unwrap();
        record
    }

    // Synthetic physical receipts test this exact binding boundary only. They
    // are not neural selection envelopes and can never populate acquired state.
    #[test]
    fn adapted_rest_and_inspect_bind_without_weakening_requested_identity() {
        let record = calibration_record();
        let adapter = PreparedMotorChannelAdapter::from_record(&record).unwrap();
        for kind in [ActionKind::Rest, ActionKind::Inspect] {
            let target = (kind == ActionKind::Inspect)
                .then(|| ActionTarget::new(Some(WorldEntityId(777)), None));
            let requested = ChannelCommand::new(
                MotorChannel::Posture,
                kind.canonical_id(),
                target,
                Vec3f::ZERO,
                Intensity::new(0.8).unwrap(),
                DurationTicks::new(1),
                0.,
                Confidence::new(1.).unwrap(),
                0,
            )
            .unwrap();
            let mut actual = noop();
            actual.command = adapter.adapt(&requested).unwrap();
            assert_ne!(actual.command.intensity, requested.intensity);
            assert!(adapted_channel_binding(&requested, &actual, &adapter).is_ok());
            for mutate in [
                |c: &mut ChannelCommand| c.intensity = Intensity::new(0.99).unwrap(),
                |c: &mut ChannelCommand| c.primitive = ActionId(999),
                |c: &mut ChannelCommand| {
                    c.target = Some(ActionTarget::new(Some(WorldEntityId(888)), None))
                },
                |c: &mut ChannelCommand| c.channel = MotorChannel::Manipulation,
                |c: &mut ChannelCommand| c.duration_ticks = DurationTicks::new(2),
            ] {
                let mut tampered = actual.clone();
                mutate(&mut tampered.command);
                assert_eq!(
                    adapted_channel_binding(&requested, &tampered, &adapter),
                    Err(SourceRejection::BindingMismatch("adapted_channel_command"))
                );
            }
        }
    }
    #[test]
    fn adapted_noop_is_benign_but_zero_force_movement_is_not() {
        let record = calibration_record();
        let adapter = PreparedMotorChannelAdapter::from_record(&record).unwrap();
        let mut actual = noop();
        let requested = actual.command.clone();
        actual.command = adapter.adapt(&requested).unwrap();
        assert!(adapted_channel_binding(&requested, &actual, &adapter).is_ok());
        assert!(benign_noop(&actual));
        let mut force = requested.clone();
        force.primitive = HeadlessActionIds::APPROACH;
        force.intensity = Intensity::new(0.).unwrap();
        actual.command = adapter.adapt(&force).unwrap();
        assert!(adapted_channel_binding(&force, &actual, &adapter).is_ok());
        assert!(!benign_noop(&actual));
        assert!(adapted_channel_binding(&requested, &actual, &adapter).is_err());
    }
    #[test]
    fn measured_interval_requires_the_once_charged_final_host_state() {
        let mut record = calibration_record();
        let before = *record.biochemistry();
        let world_after = before;
        let work = CognitiveWorkCounters::new(100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
            .unwrap()
            .into_receipt()
            .unwrap();
        record
            .account_cognitive_work(work, CognitiveWorkCostPolicy::enabled(1.0e-6).unwrap())
            .unwrap();
        let charged_once = *record.biochemistry();
        assert_ne!(charged_once, world_after);
        let measured = MeasuredPhysiologyTransition::new(before, charged_once).unwrap();
        assert!(measured_interval_binding(
            &measured,
            &before,
            &world_after,
            &before,
            &charged_once
        )
        .is_ok());
        let omitted = MeasuredPhysiologyTransition::new(before, world_after).unwrap();
        assert_eq!(
            measured_interval_binding(&omitted, &before, &world_after, &before, &charged_once),
            Err(SourceRejection::BindingMismatch(
                "measured_after_final_host_charge"
            ))
        );
        record
            .account_cognitive_work(work, CognitiveWorkCostPolicy::enabled(1.0e-6).unwrap())
            .unwrap();
        assert_ne!(*record.biochemistry(), charged_once);
        let duplicated = MeasuredPhysiologyTransition::new(before, *record.biochemistry()).unwrap();
        assert_eq!(
            measured_interval_binding(&duplicated, &before, &world_after, &before, &charged_once),
            Err(SourceRejection::BindingMismatch(
                "measured_after_final_host_charge"
            ))
        );
    }
    #[test]
    fn noop_representative_requires_the_same_strict_physical_noop_as_other_channels() {
        // V4 applies this predicate to the primary receipt too. Identity alone
        // does not turn a costly or blocked movement into a stationary no-op.
        let original = noop();
        assert!(benign_noop(&original));
        for mutate in [
            |r: &mut HeadlessMotorChannelReceipt| r.command.primitive = HeadlessActionIds::APPROACH,
            |r: &mut HeadlessMotorChannelReceipt| {
                r.physical.energy_cost = NormalizedScalar::new(0.01).unwrap()
            },
            |r: &mut HeadlessMotorChannelReceipt| r.play_stimulation = 0.01,
            |r: &mut HeadlessMotorChannelReceipt| r.investigation = 0.01,
            |r: &mut HeadlessMotorChannelReceipt| r.physical.contact = PhysicalContactKind::Blocked,
            |r: &mut HeadlessMotorChannelReceipt| r.phase = ActionPhase::Preparation,
        ] {
            let mut altered = original.clone();
            mutate(&mut altered);
            assert!(!benign_noop(&altered));
        }
    }
    #[test]
    fn numeric_final_charge_trace_keeps_body_and_each_organ_visible() {
        let mut record = calibration_record();
        let before = CpuNeedBiologyCostV4::from_biology(record.biochemistry());
        let work = CognitiveWorkCounters::new(100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
            .unwrap()
            .into_receipt()
            .unwrap();
        record
            .account_cognitive_work(work, CognitiveWorkCostPolicy::enabled(1.0e-6).unwrap())
            .unwrap();
        let after = CpuNeedBiologyCostV4::from_biology(record.biochemistry());
        assert!(after.body_energy < before.body_energy);
        assert!(after
            .organ_energy
            .iter()
            .zip(before.organ_energy)
            .any(|(a, b)| *a < b));
    }
}
