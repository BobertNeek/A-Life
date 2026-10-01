//! Deterministic, score-free action candidates derived from one sensory report.

use std::collections::BTreeSet;

use alife_core::{
    ActionCandidate, ActionKind, ActionTarget, AffordanceBits, CandidateActionFamily,
    CandidateFeatureVector, CandidateObservationRef, Confidence, DurationTicks, NormalizedScalar,
    ScaffoldContractError, SensorProfile, Validate, Vec3f, CANDIDATE_FEATURE_COUNT,
    MAX_ACTION_CANDIDATES,
};

use crate::{
    headless::HEADLESS_CONTACT_RADIUS, GroundedSensingFrame, HeadlessActionIds,
    HeadlessSensoryReport, VisibleWorldEntity,
};

pub const HEADLESS_VISION_RADIUS: f32 = 8.0;
pub const CANDIDATE_FEATURE_BEARING_SIN_LANE: usize = 0;
pub const CANDIDATE_FEATURE_BEARING_COS_LANE: usize = 1;
pub const CANDIDATE_FEATURE_DISTANCE_LANE: usize = 2;
pub const CANDIDATE_FEATURE_RELATIVE_VELOCITY_X_LANE: usize = 3;
pub const CANDIDATE_FEATURE_RELATIVE_VELOCITY_Y_LANE: usize = 4;
pub const CANDIDATE_FEATURE_RELATIVE_VELOCITY_Z_LANE: usize = 5;
pub const CANDIDATE_FEATURE_AFFORDANCE_START_LANE: usize = 6;
pub const CANDIDATE_FEATURE_AFFORDANCE_COUNT: usize = 10;
pub const CANDIDATE_FEATURE_CONTACT_LANE: usize = 16;
pub const CANDIDATE_FEATURE_EVIDENCE_LANE: usize = 17;
pub const CANDIDATE_FEATURE_RESERVED_START_LANE: usize = 18;
// Same Contact head, distinct motor operation. The feature describes the
// attempted action, never a hidden object kind or successful outcome.
fn interaction_features(
    mut features: CandidateFeatureVector,
    action: alife_core::ActionId,
) -> CandidateFeatureVector {
    if action == HeadlessActionIds::PLAY {
        features.0[alife_core::CONTACT_ACTIVATION_FEATURE_LANE] = 1.0;
    }
    features
}

const INTRINSIC_CANDIDATE_COUNT: usize = 3;
const MAX_CANDIDATE_OBJECTS: usize = (MAX_ACTION_CANDIDATES - INTRINSIC_CANDIDATE_COUNT) / 6;
const KNOWN_AFFORDANCE_MASK: u32 = (1 << CANDIDATE_FEATURE_AFFORDANCE_COUNT) - 1;

pub trait CandidateEnumerator {
    fn enumerate_candidates(
        &self,
        report: &HeadlessSensoryReport,
        profile: SensorProfile,
    ) -> Result<Vec<ActionCandidate>, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeadlessCandidateEnumerator;

impl CandidateEnumerator for HeadlessCandidateEnumerator {
    fn enumerate_candidates(
        &self,
        report: &HeadlessSensoryReport,
        profile: SensorProfile,
    ) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
        if profile != SensorProfile::PrivilegedAffordanceV1 {
            return Err(ScaffoldContractError::SensorProfileMismatch);
        }
        validate_report(report)?;

        let mut visible = report.visible_entities.iter().collect::<Vec<_>>();
        visible.sort_by(|left, right| {
            left.distance
                .total_cmp(&right.distance)
                .then_with(|| left.id.raw().cmp(&right.id.raw()))
        });
        visible.truncate(MAX_CANDIDATE_OBJECTS);

        let mut candidates = Vec::with_capacity(INTRINSIC_CANDIDATE_COUNT + visible.len() * 6);
        push_intrinsic_candidates(&mut candidates)?;

        for entity in visible {
            let features = features_for(report, entity)?;
            let target_position = add(
                report.core_snapshot.observer_position,
                entity.relative_position,
            );
            let target = ActionTarget::new(Some(entity.id), Some(target_position));
            let observation = CandidateObservationRef::None;
            for (action_id, kind, family, effort) in [
                (
                    ActionKind::Inspect.canonical_id(),
                    ActionKind::Inspect,
                    CandidateActionFamily::Inspect,
                    0.1,
                ),
                (
                    HeadlessActionIds::APPROACH,
                    ActionKind::Move,
                    CandidateActionFamily::Approach,
                    0.3,
                ),
                (
                    HeadlessActionIds::FLEE,
                    ActionKind::Move,
                    CandidateActionFamily::Avoid,
                    0.3,
                ),
                (
                    HeadlessActionIds::EAT,
                    ActionKind::Interact,
                    CandidateActionFamily::Ingest,
                    0.2,
                ),
                (
                    HeadlessActionIds::GRAB,
                    ActionKind::Interact,
                    CandidateActionFamily::Contact,
                    0.2,
                ),
                (
                    HeadlessActionIds::PLAY,
                    ActionKind::Interact,
                    CandidateActionFamily::Contact,
                    0.2,
                ),
            ] {
                candidates.push(ActionCandidate::new(
                    u16::try_from(candidates.len())
                        .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                    action_id,
                    kind,
                    family,
                    observation,
                    target,
                    interaction_features(features, action_id),
                    Confidence::new(1.0)?,
                    NormalizedScalar::new(effort)?,
                    DurationTicks::new(1),
                    DurationTicks::new(1),
                )?);
            }
        }
        Ok(candidates)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroundedCandidateEnumerator;

impl GroundedCandidateEnumerator {
    pub fn enumerate_candidates(
        &self,
        grounded: &GroundedSensingFrame,
        profile: SensorProfile,
    ) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
        if !matches!(
            profile,
            SensorProfile::GroundedObjectSlotsV1 | SensorProfile::GroundedTerrainVisionV1
        ) {
            return Err(ScaffoldContractError::SensorProfileMismatch);
        }
        if grounded.slots().len() != grounded.transports().len()
            || grounded.slots().len() > alife_core::MAX_GROUNDED_OBJECT_SLOTS
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }

        let terrain_vision = profile == SensorProfile::GroundedTerrainVisionV1;
        let object_limit = if terrain_vision {
            // Leave room for locomotion, body/head orientation, and an explicit
            // hold-gaze command within the frozen 32-candidate foundation ABI.
            3
        } else {
            MAX_CANDIDATE_OBJECTS
        };
        let mut candidates = Vec::with_capacity(
            INTRINSIC_CANDIDATE_COUNT
                + grounded.slots().len().min(object_limit) * 6
                + 3
                + if terrain_vision { 7 } else { 0 },
        );
        push_intrinsic_candidates(&mut candidates)?;

        if terrain_vision {
            for (action_id, kind, family, bearing_sin, bearing_cos) in [
                (
                    HeadlessActionIds::STEP_FORWARD,
                    ActionKind::Move,
                    CandidateActionFamily::Approach,
                    0.0,
                    1.0,
                ),
                (
                    HeadlessActionIds::TURN_LEFT,
                    ActionKind::Look,
                    CandidateActionFamily::Inspect,
                    1.0,
                    0.0,
                ),
                (
                    HeadlessActionIds::TURN_RIGHT,
                    ActionKind::Look,
                    CandidateActionFamily::Inspect,
                    -1.0,
                    0.0,
                ),
            ] {
                let mut features = [0.0; CANDIDATE_FEATURE_COUNT];
                features[CANDIDATE_FEATURE_BEARING_SIN_LANE] = bearing_sin;
                features[CANDIDATE_FEATURE_BEARING_COS_LANE] = bearing_cos;
                // Distinguish body primitives from head gaze and object descriptors.
                features[CANDIDATE_FEATURE_RESERVED_START_LANE + 1] = 1.0;
                candidates.push(ActionCandidate::new(
                    u16::try_from(candidates.len())
                        .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                    action_id,
                    kind,
                    family,
                    CandidateObservationRef::None,
                    ActionTarget::NONE,
                    CandidateFeatureVector(features).with_intrinsic_action_basis(kind),
                    Confidence::new(1.0)?,
                    NormalizedScalar::new(0.1)?,
                    DurationTicks::new(1),
                    DurationTicks::new(1),
                )?);
            }
            for (action_id, bearing_sin, bearing_cos) in [
                (HeadlessActionIds::LOOK_LEFT, 1.0, 0.0),
                (HeadlessActionIds::LOOK_RIGHT, -1.0, 0.0),
                (HeadlessActionIds::LOOK_CENTER, 0.0, 1.0),
                (HeadlessActionIds::HOLD_GAZE, 0.0, 0.0),
            ] {
                // The GPU decoder scores features, not ActionId. Without a
                // distinct feature vectors the gaze choices would tie forever.
                let mut look_features = [0.0; CANDIDATE_FEATURE_COUNT];
                look_features[CANDIDATE_FEATURE_BEARING_SIN_LANE] = bearing_sin;
                look_features[CANDIDATE_FEATURE_BEARING_COS_LANE] = bearing_cos;
                look_features[CANDIDATE_FEATURE_RESERVED_START_LANE] = 1.0;
                candidates.push(ActionCandidate::new(
                    u16::try_from(candidates.len())
                        .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                    action_id,
                    ActionKind::Look,
                    CandidateActionFamily::Inspect,
                    CandidateObservationRef::None,
                    ActionTarget::NONE,
                    CandidateFeatureVector(look_features)
                        .with_intrinsic_action_basis(ActionKind::Look),
                    Confidence::new(1.0)?,
                    NormalizedScalar::new(0.02)?,
                    DurationTicks::new(1),
                    DurationTicks::new(1),
                )?);
            }
        }

        for (slot, transport) in grounded
            .slots()
            .iter()
            .zip(grounded.transports())
            .take(object_limit)
        {
            if slot.slot_index != transport.slot_index {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
            let features = slot.candidate_features()?;
            let target = ActionTarget::new(
                Some(transport.transport_entity),
                Some(transport.target_position),
            );
            let observation = CandidateObservationRef::ObjectSlot(slot.slot_index);
            for (action_id, kind, family, effort) in [
                (
                    ActionKind::Inspect.canonical_id(),
                    ActionKind::Inspect,
                    CandidateActionFamily::Inspect,
                    0.1,
                ),
                (
                    HeadlessActionIds::APPROACH,
                    ActionKind::Move,
                    CandidateActionFamily::Approach,
                    0.3,
                ),
                (
                    HeadlessActionIds::FLEE,
                    ActionKind::Move,
                    CandidateActionFamily::Avoid,
                    0.3,
                ),
                (
                    HeadlessActionIds::EAT,
                    ActionKind::Interact,
                    CandidateActionFamily::Ingest,
                    0.2,
                ),
                (
                    HeadlessActionIds::GRAB,
                    ActionKind::Interact,
                    CandidateActionFamily::Contact,
                    0.2,
                ),
                (
                    HeadlessActionIds::PLAY,
                    ActionKind::Interact,
                    CandidateActionFamily::Contact,
                    0.2,
                ),
            ] {
                candidates.push(ActionCandidate::new(
                    u16::try_from(candidates.len())
                        .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                    action_id,
                    kind,
                    family,
                    observation,
                    target,
                    interaction_features(features, action_id),
                    slot.confidence,
                    NormalizedScalar::new(effort)?,
                    DurationTicks::new(1),
                    DurationTicks::new(1),
                )?);
            }
        }
        // A visible object must not force movement, rest, or manipulation. These
        // targetless choices remain in the existing motor channels.
        candidates.push(ActionCandidate::new(
            u16::try_from(candidates.len())
                .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
            HeadlessActionIds::NO_LOCOMOTION,
            ActionKind::Move,
            CandidateActionFamily::Approach,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Move),
            Confidence::new(1.0)?,
            NormalizedScalar::new(0.0)?,
            DurationTicks::new(1),
            DurationTicks::new(1),
        )?);
        candidates.push(ActionCandidate::new(
            u16::try_from(candidates.len())
                .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
            HeadlessActionIds::NO_MANIPULATION,
            ActionKind::Interact,
            CandidateActionFamily::Contact,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Interact),
            Confidence::new(1.0)?,
            NormalizedScalar::new(0.0)?,
            DurationTicks::new(1),
            DurationTicks::new(1),
        )?);
        candidates.push(ActionCandidate::new(
            u16::try_from(candidates.len())
                .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
            HeadlessActionIds::NO_POSTURE,
            ActionKind::Hold,
            CandidateActionFamily::Other,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Hold),
            Confidence::new(1.0)?,
            NormalizedScalar::new(0.0)?,
            DurationTicks::new(1),
            DurationTicks::new(1),
        )?);
        Ok(candidates)
    }
}

fn push_intrinsic_candidates(
    candidates: &mut Vec<ActionCandidate>,
) -> Result<(), ScaffoldContractError> {
    for (kind, family, effort) in [
        (ActionKind::Idle, CandidateActionFamily::Idle, 0.0),
        (ActionKind::Rest, CandidateActionFamily::Rest, 0.05),
        (ActionKind::Vocalize, CandidateActionFamily::Other, 0.05),
    ] {
        candidates.push(ActionCandidate::new(
            u16::try_from(candidates.len())
                .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
            kind.canonical_id(),
            kind,
            family,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero().with_intrinsic_action_basis(kind),
            Confidence::new(1.0)?,
            NormalizedScalar::new(effort)?,
            DurationTicks::new(1),
            DurationTicks::new(1),
        )?);
    }
    Ok(())
}

fn validate_report(report: &HeadlessSensoryReport) -> Result<(), ScaffoldContractError> {
    report
        .core_snapshot
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    report
        .ecology
        .validate()
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    let mut ids = BTreeSet::new();
    for entity in &report.visible_entities {
        entity
            .id
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
        entity
            .relative_position
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
        let measured_distance = length(entity.relative_position);
        if !entity.distance.is_finite()
            || entity.distance < 0.0
            || (entity.distance - measured_distance).abs() > 1e-5
            || entity.distance > HEADLESS_VISION_RADIUS
            || entity.affordances.raw() & !KNOWN_AFFORDANCE_MASK != 0
            || !ids.insert(entity.id.raw())
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
    }
    let mut contact_ids = BTreeSet::new();
    for contact_id in &report.contact_entities {
        contact_id
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
        let Some(visible) = report
            .visible_entities
            .iter()
            .find(|entity| entity.id == *contact_id)
        else {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        };
        if !contact_ids.insert(contact_id.raw()) || visible.distance > HEADLESS_CONTACT_RADIUS {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
    }
    let geometric_contact_ids = report
        .visible_entities
        .iter()
        .filter(|entity| entity.distance <= HEADLESS_CONTACT_RADIUS)
        .map(|entity| entity.id.raw())
        .collect::<BTreeSet<_>>();
    if contact_ids != geometric_contact_ids {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    Ok(())
}

fn features_for(
    report: &HeadlessSensoryReport,
    entity: &VisibleWorldEntity,
) -> Result<CandidateFeatureVector, ScaffoldContractError> {
    debug_assert_eq!(CANDIDATE_FEATURE_RESERVED_START_LANE, 18);
    debug_assert_eq!(CANDIDATE_FEATURE_COUNT, 24);
    // Match the grounded profile and Y-up game space, preserving [sin, cos].
    let planar_length = entity.relative_position.x.hypot(entity.relative_position.z);
    let (bearing_sin, bearing_cos) = if planar_length > f32::EPSILON {
        (
            entity.relative_position.z / planar_length,
            entity.relative_position.x / planar_length,
        )
    } else {
        (0.0, 1.0)
    };
    let mut values = [0.0_f32; CANDIDATE_FEATURE_COUNT];
    values[CANDIDATE_FEATURE_BEARING_SIN_LANE] = bearing_sin;
    values[CANDIDATE_FEATURE_BEARING_COS_LANE] = bearing_cos;
    values[CANDIDATE_FEATURE_DISTANCE_LANE] = entity.distance / HEADLESS_VISION_RADIUS;
    values[CANDIDATE_FEATURE_RELATIVE_VELOCITY_X_LANE] = 0.0;
    values[CANDIDATE_FEATURE_RELATIVE_VELOCITY_Y_LANE] = 0.0;
    values[CANDIDATE_FEATURE_RELATIVE_VELOCITY_Z_LANE] = 0.0;
    for lane_offset in 0..CANDIDATE_FEATURE_AFFORDANCE_COUNT {
        let bit = AffordanceBits(1 << lane_offset);
        values[CANDIDATE_FEATURE_AFFORDANCE_START_LANE + lane_offset] =
            if entity.affordances.contains(bit) {
                1.0
            } else {
                0.0
            };
    }
    values[CANDIDATE_FEATURE_CONTACT_LANE] = if report.contact_entities.contains(&entity.id) {
        1.0
    } else {
        0.0
    };
    values[CANDIDATE_FEATURE_EVIDENCE_LANE] = 1.0;
    let features = CandidateFeatureVector(values);
    features.validate()?;
    Ok(features)
}

fn add(left: Vec3f, right: Vec3f) -> Vec3f {
    Vec3f::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn length(value: Vec3f) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}

#[cfg(test)]
mod founder_basis_tests {
    use super::*;
    use crate::HeadlessScenarioBuilder;
    use alife_core::{HomeostaticSnapshot, OrganismId, Tick};

    #[test]
    fn intrinsic_choices_have_learning_support_without_changing_object_evidence() {
        let mut world = HeadlessScenarioBuilder::new(60_011)
            .agent("observer", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(3.0, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        let frame = world
            .perception_frame_draft(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        for candidate in frame.candidates() {
            if candidate.target == ActionTarget::NONE {
                assert_eq!(
                    candidate.features.0[CandidateFeatureVector::INTRINSIC_ACTION_BASIS_LANE],
                    1.0
                );
            } else if let CandidateObservationRef::ObjectSlot(index) = candidate.observation {
                let slot = frame
                    .grounded_object_slots()
                    .iter()
                    .find(|s| s.slot_index == index)
                    .unwrap();
                assert_eq!(
                    candidate.features.0[CandidateFeatureVector::INTRINSIC_ACTION_BASIS_LANE],
                    slot.terrain[1]
                );
                assert_eq!(candidate.features.0[15], 0.0, "distant taste stays absent");
            }
        }
        assert!(frame
            .sensory()
            .channels
            .smell_chemistry
            .iter()
            .any(|v| *v > 0.0));
        let rest = frame
            .candidates()
            .iter()
            .find(|c| c.family == CandidateActionFamily::Rest)
            .unwrap();
        let vocal = frame
            .candidates()
            .iter()
            .find(|c| c.kind == ActionKind::Vocalize)
            .unwrap();
        let hold = frame
            .candidates()
            .iter()
            .find(|c| c.action_id == HeadlessActionIds::NO_POSTURE)
            .unwrap();
        assert_eq!(vocal.family, hold.family);
        assert_ne!(vocal.features, hold.features);
        // d(logit)/d(weight) = motor * feature is now nonzero for a nonzero
        // motor activation, so both offline and local action learning can act.
        assert_ne!(
            0.5 * rest.features.0[CandidateFeatureVector::INTRINSIC_ACTION_BASIS_LANE],
            0.0
        );
        let mut forged = frame.candidates().to_vec();
        forged[usize::from(rest.candidate_index)].features.0[22] = 1.0;
        assert_eq!(
            alife_core::PerceptionFrameDraft::new(
                frame.organism_id(),
                frame.tick(),
                frame.sensor_profile(),
                frame.sensory().clone(),
                frame.body(),
                *frame.homeostasis(),
                forged,
                frame.profile_provenance(),
                frame.grounded_object_slots().to_vec(),
            ),
            Err(ScaffoldContractError::InvalidPerceptionFrame),
            "the action descriptor cannot fabricate a different opportunity",
        );
    }
}
