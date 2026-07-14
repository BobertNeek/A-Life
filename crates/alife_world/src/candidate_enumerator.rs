//! Deterministic, score-free action candidates derived from one sensory report.

use std::collections::BTreeSet;

use alife_core::{
    ActionCandidate, ActionKind, ActionTarget, AffordanceBits, CandidateActionFamily,
    CandidateFeatureVector, CandidateObservationRef, Confidence, DurationTicks, NormalizedScalar,
    ScaffoldContractError, SensorProfile, Validate, Vec3f, CANDIDATE_FEATURE_COUNT,
    MAX_ACTION_CANDIDATES,
};

use crate::{
    headless::HEADLESS_CONTACT_RADIUS, HeadlessActionIds, HeadlessSensoryReport, VisibleWorldEntity,
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

const MAX_CANDIDATE_OBJECTS: usize = (MAX_ACTION_CANDIDATES - 1) / 5;
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

        let mut candidates = Vec::with_capacity(1 + visible.len() * 5);
        candidates.push(ActionCandidate::new(
            0,
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            CandidateActionFamily::Idle,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero(),
            Confidence::new(1.0)?,
            NormalizedScalar::new(0.0)?,
            DurationTicks::new(1),
            DurationTicks::new(1),
        )?);

        for (object_slot, entity) in visible.into_iter().enumerate() {
            let features = features_for(report, entity)?;
            let target_position = add(
                report.core_snapshot.observer_position,
                entity.relative_position,
            );
            let target = ActionTarget::new(Some(entity.id), Some(target_position));
            let observation = CandidateObservationRef::ObjectSlot(
                u16::try_from(object_slot)
                    .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
            );
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
            ] {
                candidates.push(ActionCandidate::new(
                    u16::try_from(candidates.len())
                        .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                    action_id,
                    kind,
                    family,
                    observation,
                    target,
                    features,
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
    let planar_length = entity.relative_position.x.hypot(entity.relative_position.y);
    let (bearing_sin, bearing_cos) = if planar_length > f32::EPSILON {
        (
            entity.relative_position.y / planar_length,
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
