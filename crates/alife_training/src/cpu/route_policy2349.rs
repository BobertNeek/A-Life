//! Explicit innate graph comparator, NOT a weight-learned brain. No forced action.
//! Visible landmark directions are exploration hypotheses, not claims of free space.
use super::observed_map2349::{heading_bin, ObservedMap};
use alife_core::{ActionCandidate, GroundedObjectSlotV1, TrackedObjectId, Validate};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteReason {
    KnownGoalRoute,
    UntriedObservedDirection,
    BacktrackTraversedLink,
    LeastTriedObservedDirection,
    NoEvidence,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteWork {
    pub validation_checks: u64,
    pub node_visits: u64,
    pub edge_visits: u64,
    pub slot_visits: u64,
    pub comparisons: u64,
    pub transforms: u64,
    pub candidate_visits: u64,
    pub writes: u64,
}
impl RouteWork {
    pub fn units(&self) -> u64 {
        self.validation_checks
            + self.node_visits
            + self.edge_visits
            + self.slot_visits
            + self.comparisons
            + self.transforms
            + self.candidate_visits
            + self.writes
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutePreference {
    pub version: u8,
    pub reason: RouteReason,
    pub target_heading_body: Option<f32>,
    pub observed_landmark: Option<TrackedObjectId>,
    pub traversed_node: Option<u64>,
    pub candidate_bias: Vec<f32>,
    pub work: RouteWork,
}
fn angle(x: f32) -> f32 {
    (x + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}
/// Single declared innate gain, no grid. Add in native f32 order before output
/// authentication AND replay. This module does not itself sample or execute.
pub const ROUTE_GAIN: f32 = 4.0;
pub fn preference(
    map: &ObservedMap,
    slots: &[GroundedObjectSlotV1],
    gaze_yaw: f32,
    candidates: &[ActionCandidate],
) -> Result<RoutePreference, String> {
    let validation = map.validate_counted()?;
    preference_inner(map, slots, gaze_yaw, candidates, validation)
}
/// Opt-in optimized path only; legacy public route retains validation and tariff.
pub(super) fn preference_validated(
    map: &super::observed_map2349::ValidatedObservedMap,
    slots: &[GroundedObjectSlotV1],
    gaze_yaw: f32,
    candidates: &[ActionCandidate],
) -> Result<RoutePreference, String> {
    preference_inner(map.as_map(), slots, gaze_yaw, candidates, 1)
}
fn preference_inner(
    map: &ObservedMap,
    slots: &[GroundedObjectSlotV1],
    gaze_yaw: f32,
    candidates: &[ActionCandidate],
    validation: u64,
) -> Result<RoutePreference, String> {
    // Validated path counts one token access; all slot/candidate checks remain below.
    let mut work = RouteWork {
        validation_checks: validation,
        ..Default::default()
    };
    if slots.len() > alife_core::MAX_GROUNDED_OBJECT_SLOTS
        || candidates.len() > 32
        || !gaze_yaw.is_finite()
    {
        return Err("invalid route inputs".into());
    }
    let mut current = None;
    for n in map.visited_nodes() {
        work.node_visits += 1;
        work.comparisons += 1;
        if Some(n.id) == map.current_visit() {
            current = Some(n);
            break;
        }
    }
    // Rank each currently observed direction by actual outgoing movement attempts,
    // blocked fraction, then recognized remembered-goal tie preference, stable ID.
    let mut best: Option<(u64, u64, u32, bool, u64, f32, TrackedObjectId)> = None;
    for s in slots {
        work.slot_visits += 1;
        work.validation_checks += 1;
        s.validate_contract().map_err(|e| format!("{e:?}"))?;
        work.comparisons += 1;
        if s.distance <= 0.0 {
            continue;
        }
        let heading = angle(s.bearing[0].atan2(s.bearing[1]) + gaze_yaw);
        let bin = heading_bin(heading + map.yaw());
        work.transforms += 2;
        let attempts = current.map_or(0, |n| n.exit_attempts[bin]);
        let blocked = current.map_or(0, |n| n.exit_blocks[bin]);
        let not_goal = map.goal() != Some(s.tracked_object_id);
        let uncertainty = ((1.0 - s.confidence.raw()) * 1024.0) as u32;
        let key = (
            attempts,
            blocked,
            uncertainty,
            not_goal,
            s.tracked_object_id.raw(),
        );
        work.comparisons += 1;
        if best
            .as_ref()
            .is_none_or(|b| key < (b.0, b.1, b.2, b.3, b.4))
        {
            best = Some((
                attempts,
                blocked,
                uncertainty,
                not_goal,
                s.tracked_object_id.raw(),
                heading,
                s.tracked_object_id,
            ));
            work.writes += 1
        }
    }
    let mut reason = RouteReason::NoEvidence;
    let mut heading = None;
    let mut landmark = None;
    let mut traversed = None;
    if let Some(b) = best.as_ref().filter(|b| b.0 == 0) {
        reason = RouteReason::UntriedObservedDirection;
        heading = Some(b.5);
        landmark = Some(b.6);
    } else if let Some(current) = current {
        // Only reverse a link genuinely traversed into this node. This is a conservative
        // backtrack hypothesis, not proof the reverse direction remains unblocked.
        let mut chosen: Option<(u32, u64, u64, f32)> = None;
        for e in map.edges() {
            work.edge_visits += 1;
            work.comparisons += 2;
            if e.to != current.id || e.successes == 0 {
                continue;
            }
            for n in map.visited_nodes() {
                work.node_visits += 1;
                work.comparisons += 1;
                if n.id != e.from {
                    continue;
                }
                let dx = n.position[0] - map.position()[0];
                let dy = n.position[1] - map.position()[1];
                work.transforms += 1;
                if dx * dx + dy * dy <= 1e-12 {
                    continue;
                }
                // Blocked counts penalize links; fewer visits prioritizes less repeated routes.
                let uncertainty =
                    ((1.0 - (e.confidence() * n.confidence).clamp(0.0, 1.0)) * 1024.0) as u32;
                let key = (uncertainty, n.visits, n.id);
                work.comparisons += 1;
                if chosen.as_ref().is_none_or(|c| key < (c.0, c.1, c.2)) {
                    chosen = Some((key.0, key.1, key.2, angle(dy.atan2(dx) - map.yaw())));
                    work.writes += 1
                }
            }
        }
        if let Some(c) = chosen {
            reason = RouteReason::BacktrackTraversedLink;
            heading = Some(c.3);
            traversed = Some(c.2)
        }
    }
    if heading.is_none() {
        if let Some(b) = best {
            reason = RouteReason::LeastTriedObservedDirection;
            heading = Some(b.5);
            landmark = Some(b.6)
        }
    }
    let mut bias = vec![0.; candidates.len()];
    work.writes += candidates.len() as u64;
    for (i, c) in candidates.iter().enumerate() {
        work.candidate_visits += 1;
        work.validation_checks += 1;
        c.validate_contract().map_err(|e| format!("{e:?}"))?;
        if c.candidate_index as usize != i {
            return Err("unaligned candidate index".into());
        }
        if let Some(h) = heading {
            let v = match c.action_id.raw() {
                105 => ROUTE_GAIN * h.cos(),
                604 => ROUTE_GAIN * (h / std::f32::consts::PI),
                605 => -ROUTE_GAIN * (h / std::f32::consts::PI),
                _ => 0.,
            };
            work.comparisons += 1;
            work.transforms += 1;
            bias[i] = v;
            work.writes += 1;
        }
    }
    Ok(RoutePreference {
        version: 1,
        reason,
        target_heading_body: heading,
        observed_landmark: landmark,
        traversed_node: traversed,
        candidate_bias: bias,
        work,
    })
}
