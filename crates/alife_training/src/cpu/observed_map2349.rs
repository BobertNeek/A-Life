//! Research-only experienced landmark graph. No World access, object kinds, goal oracle,
//! policy action, or host integration. Coordinates are self-odometric, not world coordinates.
use alife_core::{
    ExperienceSequenceId, GroundedObjectSlotV1, TrackedObjectId, Validate,
    MAX_GROUNDED_OBJECT_SLOTS,
};
use serde::{Deserialize, Serialize};
pub const NODE_CAP: usize = 32;
pub const EDGE_CAP: usize = 64;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Landmark {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_frame_verified2367: Option<bool>,
    pub id: TrackedObjectId,
    pub position: [f32; 2],
    pub confidence: f32,
    pub last_seen: u64,
    pub visits: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Traversal {
    pub from: u64,
    pub to: u64,
    pub displacement: [f32; 2],
    pub successes: u64,
    pub blocked: u64,
    pub last_sequence: u64,
}
impl Traversal {
    pub fn confidence(&self) -> f32 {
        self.successes as f32
            / (self
                .successes
                .saturating_add(self.blocked)
                .saturating_add(1)) as f32
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapWork {
    pub observed_validations: u64,
    pub node_visits: u64,
    pub edge_visits: u64,
    pub comparisons: u64,
    pub field_updates: u64,
    pub transforms: u64,
    pub validation_checks: u64,
    pub record_copies: u64,
}
impl MapWork {
    pub fn units(&self) -> u64 {
        self.observed_validations
            + self.node_visits
            + self.edge_visits
            + self.comparisons
            + self.field_updates
            + self.transforms
            + self.validation_checks
            + self.record_copies
    }
}
/// Fixed semantic event counters, not CPU instructions or measured speed. Includes failed
/// lookups and deterministic eviction scans. Host must bill once when this future API is used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisitNode {
    pub id: u64,
    pub position: [f32; 2],
    pub observed_signature: Vec<TrackedObjectId>,
    pub confidence: f32,
    pub last_sequence: u64,
    pub visits: u64,
    pub exit_attempts: [u64; 8],
    pub exit_blocks: [u64; 8],
}
/// Explicit experiment constants: anchor every two 0.2m body radii, loop association
/// within one body radius with at least one shared actually observed landmark.
pub const ANCHOR_SPACING: f32 = 0.4;
pub const LOOP_RADIUS: f32 = 0.2;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedMap {
    version: u8,
    sequence: u64,
    position: [f32; 2],
    yaw: f32,
    nodes: Vec<Landmark>,
    edges: Vec<Traversal>,
    visit_nodes: Vec<VisitNode>,
    last_visit: Option<u64>,
    next_visit: u64,
    contact_origin: [f32; 2],
    goal: Option<TrackedObjectId>,
    goal_set_at: u64,
}
/// Actual executed movement since the previous observation, in previous BODY coordinates.
/// A future host must authenticate this from native resolved motion, never a requested command.
#[derive(Debug, Clone, Copy)]
pub struct MeasuredMotion {
    pub displacement: [f32; 2],
    pub yaw_delta: f32,
    pub blocked: bool,
    pub zero_step_blocked: bool,
    pub executed_traversal: bool,
}
fn rotate(p: [f32; 2], yaw: f32) -> [f32; 2] {
    let (s, c) = yaw.sin_cos();
    [c * p[0] - s * p[1], s * p[0] + c * p[1]]
}
pub fn heading_bin(yaw: f32) -> usize {
    ((yaw.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * 8.0).floor() as usize).min(7)
}
fn finite(p: [f32; 2]) -> bool {
    p.iter().all(|x| x.is_finite())
}
impl Default for ObservedMap {
    fn default() -> Self {
        Self {
            version: 3,
            sequence: 0,
            position: [0.; 2],
            yaw: 0.,
            nodes: vec![],
            edges: vec![],
            visit_nodes: vec![],
            last_visit: None,
            next_visit: 1,
            contact_origin: [0.; 2],
            goal: None,
            goal_set_at: 0,
        }
    }
}
/// Unforgeable in sibling modules; never serialized and no mutable accessor.
/// Issued only by an observation that has already validated its result.
pub(super) struct ValidatedObservedMap(ObservedMap);
impl ValidatedObservedMap {
    pub(super) fn as_map(&self) -> &ObservedMap {
        &self.0
    }
    pub(super) fn into_inner(self) -> ObservedMap {
        self.0
    }
}
impl ObservedMap {
    /// Map v3 consumes actual V1 world-frame bearings through an explicit body offset.
    pub fn frame_version2367(&self) -> u8 {
        self.version
    }
    pub(super) fn with_world_slot_frame2367(&self) -> Result<Self, String> {
        self.validate()?;
        if self.version != 2 {
            return Err("frame upgrade requires legacy map2".into());
        }
        let mut next = self.clone();
        next.version = 3;
        next.validate()?;
        Ok(next)
    }
    pub(super) fn restore_frame_identity_for_check2367(&mut self) {
        self.version = 2;
    }
    pub fn landmark_position_verified2367(&self, n: &Landmark) -> bool {
        self.version == 2 || n.world_frame_verified2367 == Some(true)
    }

    pub(super) fn observe_validated(
        &self,
        sequence: ExperienceSequenceId,
        slots: &[GroundedObjectSlotV1],
        range_scale: f32,
        gaze_yaw: f32,
        motion: MeasuredMotion,
        selected_approach: Option<TrackedObjectId>,
    ) -> Result<(ValidatedObservedMap, MapWork), String> {
        let (map, mut work) = self.observe(
            sequence,
            slots,
            range_scale,
            gaze_yaw,
            motion,
            selected_approach,
        )?;
        // One bounded wrapper creation/move, no second map copy or validation.
        work.record_copies += 1;
        Ok((ValidatedObservedMap(map), work))
    }

    fn copy_units(&self) -> u64 {
        1 + self.nodes.len() as u64
            + self.edges.len() as u64
            + self.visit_nodes.len() as u64
            + self
                .visit_nodes
                .iter()
                .map(|n| n.observed_signature.len() as u64)
                .sum::<u64>()
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn current_visit(&self) -> Option<u64> {
        self.last_visit
    }
    pub fn position(&self) -> [f32; 2] {
        self.position
    }
    pub fn yaw(&self) -> f32 {
        self.yaw
    }
    pub fn nodes(&self) -> &[Landmark] {
        &self.nodes
    }
    pub fn edges(&self) -> &[Traversal] {
        &self.edges
    }
    pub fn visited_nodes(&self) -> &[VisitNode] {
        &self.visit_nodes
    }
    pub fn goal(&self) -> Option<TrackedObjectId> {
        self.goal
    }
    /// Host calls this ONLY after an actual selected Approach in this same bound frame.
    /// It cannot nominate an unseen ID or modify the pre-decision route preference.
    pub fn record_actual_approach(
        &self,
        sequence: ExperienceSequenceId,
        slots: &[GroundedObjectSlotV1],
        target: TrackedObjectId,
    ) -> Result<(Self, MapWork), String> {
        let mut work = MapWork {
            validation_checks: self.validate_counted()?,
            ..Default::default()
        };
        if sequence.raw() != self.sequence || slots.len() > MAX_GROUNDED_OBJECT_SLOTS {
            return Err("approach nomination chronology".into());
        }
        let mut found = false;
        for s in slots {
            work.observed_validations += 1;
            s.validate_contract().map_err(|e| format!("{e:?}"))?;
            work.comparisons += 1;
            found |= s.tracked_object_id == target;
        }
        if !found {
            return Err("approach nomination unseen".into());
        }
        let mut resident = false;
        for n in &self.nodes {
            work.node_visits += 1;
            work.comparisons += 1;
            resident |= n.id == target;
        }
        if !resident {
            return Err("approach nomination not retained".into());
        }
        work.record_copies += self.copy_units();
        let mut next = self.clone();
        next.goal = Some(target);
        next.goal_set_at = sequence.raw();
        work.field_updates += 2;
        Ok((next, work))
    }
    /// Host must authenticate `consumed` from actual resolved outcome and tracked-ID
    /// binding in this same decision frame. Absence/offscreen is never consumption.
    pub fn release_consumed_goal(
        &self,
        sequence: ExperienceSequenceId,
        consumed: TrackedObjectId,
    ) -> Result<(Self, MapWork), String> {
        let mut work = MapWork {
            validation_checks: self.validate_counted()?,
            ..Default::default()
        };
        if sequence.raw() != self.sequence {
            return Err("consumption chronology".into());
        }
        work.record_copies += self.copy_units();
        let mut next = self.clone();
        work.comparisons += 1;
        if next.goal == Some(consumed) {
            next.goal = None;
            work.field_updates += 1;
        }
        Ok((next, work))
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|e| e.to_string())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let x: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        x.validate()?;
        Ok(x)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.validate_counted().map(|_| ())
    }
    pub fn validate_counted(&self) -> Result<u64, String> {
        let mut checks = 1;
        if self.goal_set_at > self.sequence
            || !matches!(self.version, 2 | 3)
            || self.next_visit == 0
            || self.nodes.len() > NODE_CAP
            || self.edges.len() > EDGE_CAP
            || self.visit_nodes.len() > NODE_CAP
            || !finite(self.position)
            || !finite(self.contact_origin)
            || !self.yaw.is_finite()
        {
            return Err("invalid graph header".into());
        }
        for (i, n) in self.nodes.iter().enumerate() {
            checks += 1;
            if self.version == 3 {
                checks += 2;
                if n.world_frame_verified2367 == Some(false) {
                    return Err("invalid frame verification".into());
                }
            } else if n.world_frame_verified2367.is_some() {
                return Err("legacy map has new frame evidence".into());
            }
            checks += 1;
            if n.id.raw() == 0
                || !finite(n.position)
                || !n.confidence.is_finite()
                || !(0.0..=1.0).contains(&n.confidence)
                || n.last_seen > self.sequence
            {
                return Err("invalid node".into());
            }
            for x in &self.nodes[..i] {
                checks += 1;
                if x.id == n.id {
                    return Err("duplicate node".into());
                }
            }
        }
        for (i, e) in self.edges.iter().enumerate() {
            checks += 1;
            if e.from == e.to || !finite(e.displacement) || e.last_sequence > self.sequence {
                return Err("invalid edge".into());
            }
            let (mut from, mut to) = (false, false);
            for n in &self.visit_nodes {
                checks += 2;
                from |= n.id == e.from;
                to |= n.id == e.to;
            }
            if !from || !to {
                return Err("dangling edge".into());
            }
            for x in &self.edges[..i] {
                checks += 1;
                if x.from == e.from && x.to == e.to {
                    return Err("duplicate edge".into());
                }
            }
        }
        if let Some(id) = self.goal {
            let mut found = false;
            for n in &self.nodes {
                checks += 1;
                found |= n.id == id;
            }
            if !found {
                return Err("dangling goal".into());
            }
        }
        for (i, n) in self.visit_nodes.iter().enumerate() {
            checks += 1;
            if n.observed_signature.len() > MAX_GROUNDED_OBJECT_SLOTS
                || n.id == 0
                || n.id >= self.next_visit
                || !finite(n.position)
                || !n.confidence.is_finite()
                || !(0.0..=1.0).contains(&n.confidence)
                || n.last_sequence > self.sequence
            {
                return Err("invalid visit".into());
            }
            for x in &self.visit_nodes[..i] {
                checks += 1;
                if x.id == n.id {
                    return Err("duplicate visit".into());
                }
            }
            for (j, id) in n.observed_signature.iter().enumerate() {
                checks += 1;
                if id.raw() == 0 {
                    return Err("invalid signature".into());
                }
                for old in &n.observed_signature[..j] {
                    checks += 1;
                    if old == id {
                        return Err("duplicate signature".into());
                    }
                }
            }
            for i in 0..8 {
                checks += 1;
                if n.exit_blocks[i] > n.exit_attempts[i] {
                    return Err("impossible exit counts".into());
                }
            }
        }
        if let Some(id) = self.last_visit {
            let mut found = false;
            for n in &self.visit_nodes {
                checks += 1;
                found |= n.id == id;
            }
            if !found {
                return Err("dangling visit".into());
            }
        }
        Ok(checks)
    }
    /// Atomic pure update. Input slots are current observer-visible facts. range_scale converts
    /// normalized slot distance to meters; gaze_yaw is relative to current BODY heading.
    /// selected_approach is the actual current selected Approach target, not a nominated hidden ID.
    /// No selected target means no new goal; a previously observed goal may remain remembered.
    pub fn observe(
        &self,
        sequence: ExperienceSequenceId,
        slots: &[GroundedObjectSlotV1],
        range_scale: f32,
        gaze_yaw: f32,
        motion: MeasuredMotion,
        selected_approach: Option<TrackedObjectId>,
    ) -> Result<(Self, MapWork), String> {
        let initial_checks = self.validate_counted()?;
        if slots.len() > MAX_GROUNDED_OBJECT_SLOTS {
            return Err("too many observed slots".into());
        }
        if (motion.zero_step_blocked && !motion.blocked)
            || sequence.raw() <= self.sequence
            || !range_scale.is_finite()
            || range_scale <= 0.
            || !gaze_yaw.is_finite()
            || !finite(motion.displacement)
            || !motion.yaw_delta.is_finite()
        {
            return Err("invalid observation chronology/geometry".into());
        }
        let mut work = MapWork {
            validation_checks: initial_checks,
            ..Default::default()
        };
        for (i, s) in slots.iter().enumerate() {
            work.observed_validations += 1;
            s.validate_contract().map_err(|e| format!("{e:?}"))?;
            for p in &slots[..i] {
                work.comparisons += 1;
                if p.tracked_object_id == s.tracked_object_id {
                    return Err("duplicate observed ID".into());
                }
            }
        }
        if let Some(id) = selected_approach {
            let mut found = false;
            for s in slots {
                work.comparisons += 1;
                found |= s.tracked_object_id == id;
            }
            if !found {
                return Err("selected goal is not observed".into());
            }
        }
        work.record_copies += self.copy_units();
        let mut next = self.clone();
        next.sequence = sequence.raw();
        if next.goal.is_some() && sequence.raw().saturating_sub(next.goal_set_at) > 64 {
            next.goal = None;
            work.field_updates += 1;
        }
        let prior_yaw = next.yaw;
        let d = rotate(motion.displacement, next.yaw);
        work.transforms += 1;
        next.position = [next.position[0] + d[0], next.position[1] + d[1]];
        next.yaw += motion.yaw_delta;
        work.field_updates += 4;
        // A successful edge must contain only authenticated movement since its anchor.
        // Even a sub-spacing passive displacement breaks that continuity. Keep old
        // graph evidence, but establish a fresh exact-position anchor without reusing
        // a nearby old node (which could smuggle the passive segment into a later edge).
        let moved_x = motion.displacement[0] != 0.;
        let moved_y = motion.displacement[1] != 0.;
        let passive = !motion.executed_traversal;
        work.comparisons += 3;
        let break_traversal = passive && (moved_x || moved_y);
        if break_traversal {
            next.last_visit = None;
            work.field_updates += 1;
        }
        if let Some(id) = next.last_visit {
            for n in &mut next.visit_nodes {
                work.node_visits += 1;
                work.comparisons += 1;
                if n.id == id {
                    if (motion.executed_traversal && d[0] * d[0] + d[1] * d[1] > 1e-12)
                        || motion.zero_step_blocked
                    {
                        let bin = heading_bin(if d[0] * d[0] + d[1] * d[1] > 1e-12 {
                            d[1].atan2(d[0])
                        } else {
                            prior_yaw
                        });
                        n.exit_attempts[bin] = n.exit_attempts[bin].saturating_add(1);
                        if motion.blocked {
                            n.exit_blocks[bin] = n.exit_blocks[bin].saturating_add(1)
                        }
                        work.transforms += 1;
                        work.field_updates += 2;
                    }
                    break;
                }
            }
        }
        // Missing observations lower confidence, never delete evidence or imply a traversable route.
        for n in &mut next.nodes {
            work.node_visits += 1;
            let mut seen = false;
            for s in slots {
                work.comparisons += 1;
                seen |= s.tracked_object_id == n.id;
            }
            if !seen {
                n.confidence *= 0.99;
                work.field_updates += 1
            }
        }
        // Stable-ID insertion order prevents source roster ordering from changing eviction decisions.
        work.record_copies += slots.len() as u64;
        let mut order: Vec<_> = slots.iter().collect();
        order.sort_by(|a, b| {
            work.comparisons += 1;
            a.tracked_object_id.cmp(&b.tracked_object_id)
        });
        for s in order {
            let r = s.distance * range_scale;
            let relative = rotate([s.bearing[1] * r, s.bearing[0] * r], next.yaw + gaze_yaw);
            work.transforms += 1;
            let position = [
                next.position[0] + relative[0],
                next.position[1] + relative[1],
            ];
            let mut index = None;
            for (i, n) in next.nodes.iter().enumerate() {
                work.node_visits += 1;
                work.comparisons += 1;
                if n.id == s.tracked_object_id {
                    index = Some(i);
                    break;
                }
            }
            let i = if let Some(i) = index {
                i
            } else {
                if next.nodes.len() == NODE_CAP {
                    let mut victim = 0;
                    for i in 1..next.nodes.len() {
                        work.node_visits += 1;
                        work.comparisons += 1;
                        if (next.nodes[i].last_seen, next.nodes[i].id)
                            < (next.nodes[victim].last_seen, next.nodes[victim].id)
                        {
                            victim = i
                        }
                    }
                    let id = {
                        work.record_copies += (next.nodes.len() - victim - 1) as u64;
                        next.nodes.remove(victim).id
                    };
                    work.field_updates += 1;
                    if next.goal == Some(id) {
                        next.goal = None;
                        work.field_updates += 1
                    }
                }
                next.nodes.push(Landmark {
                    world_frame_verified2367: None,
                    id: s.tracked_object_id,
                    position,
                    confidence: s.confidence.raw(),
                    last_seen: sequence.raw(),
                    visits: 0,
                });
                work.field_updates += 5;
                next.nodes.len() - 1
            };
            let n = &mut next.nodes[i];
            work.comparisons += 1;
            if next.version == 3 {
                n.world_frame_verified2367 = Some(true);
                work.field_updates += 1;
            }
            n.position = position;
            n.confidence = s.confidence.raw();
            n.last_seen = sequence.raw();
            work.field_updates += 3;
        }
        if let Some(id) = selected_approach {
            if next.nodes.iter().any(|n| {
                work.node_visits += 1;
                work.comparisons += 1;
                n.id == id
            }) {
                next.goal = Some(id);
                next.goal_set_at = sequence.raw();
                work.field_updates += 2;
            }
        }
        work.record_copies += slots.len() as u64;
        let mut signature: Vec<_> = slots.iter().map(|s| s.tracked_object_id).collect();
        signature.sort_by(|a, b| {
            work.comparisons += 1;
            a.cmp(b)
        });
        let previous = next
            .last_visit
            .and_then(|id| {
                next.visit_nodes.iter().find(|n| {
                    work.node_visits += 1;
                    work.comparisons += 1;
                    n.id == id
                })
            })
            .cloned();
        let delta = previous.as_ref().map_or([0.; 2], |n| {
            [
                next.position[0] - n.position[0],
                next.position[1] - n.position[1],
            ]
        });
        let moved = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
        // Initial observed place, actual 0.4m traversal, or new blocked endpoint anchors a
        // visited pose. Zero-distance blocking updates the current node's self attempt below.
        let anchor =
            previous.is_none() || moved >= ANCHOR_SPACING || (motion.blocked && moved > 1e-6);
        if anchor {
            let mut existing = None;
            if !break_traversal {
                for n in &next.visit_nodes {
                    work.node_visits += 1;
                    work.comparisons += 1;
                    let d = [
                        n.position[0] - next.position[0],
                        n.position[1] - next.position[1],
                    ];
                    let close = d[0] * d[0] + d[1] * d[1] <= LOOP_RADIUS * LOOP_RADIUS;
                    let mut shared = false;
                    for a in &signature {
                        for b in &n.observed_signature {
                            work.comparisons += 1;
                            shared |= a == b;
                        }
                    }
                    if close
                        && shared
                        && Some(n.id) != next.last_visit
                        && existing.is_none_or(|id| n.id < id)
                    {
                        existing = Some(n.id)
                    }
                }
            }
            let id = if let Some(id) = existing {
                id
            } else {
                if next.visit_nodes.len() == NODE_CAP {
                    let victim = next
                        .visit_nodes
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, n)| (n.last_sequence, n.id))
                        .unwrap()
                        .0;
                    work.node_visits += next.visit_nodes.len() as u64;
                    work.record_copies += (next.visit_nodes.len() - victim - 1) as u64;
                    let old = next.visit_nodes.remove(victim).id;
                    next.edges.retain(|e| {
                        work.edge_visits += 1;
                        work.comparisons += 2;
                        work.record_copies += 1;
                        e.from != old && e.to != old
                    });
                    if next.last_visit == Some(old) {
                        next.last_visit = None
                    }
                }
                let id = next.next_visit;
                next.next_visit = next.next_visit.checked_add(1).ok_or("visit ID exhausted")?;
                next.visit_nodes.push(VisitNode {
                    id,
                    position: next.position,
                    observed_signature: {
                        work.record_copies += signature.len() as u64;
                        signature.clone()
                    },
                    confidence: 1.,
                    last_sequence: sequence.raw(),
                    visits: 0,
                    exit_attempts: [0; 8],
                    exit_blocks: [0; 8],
                });
                work.field_updates += 23;
                id
            };
            if let Some(n) = next.visit_nodes.iter_mut().find(|n| {
                work.node_visits += 1;
                work.comparisons += 1;
                n.id == id
            }) {
                n.visits = n.visits.saturating_add(1);
                n.last_sequence = sequence.raw();
                n.observed_signature = signature;
                n.confidence = 1.;
                work.field_updates += 4
            }
            if let Some(from) = next.last_visit {
                if from != id && motion.executed_traversal {
                    let mut index = None;
                    for (k, e) in next.edges.iter().enumerate() {
                        work.edge_visits += 1;
                        work.comparisons += 2;
                        if e.from == from && e.to == id {
                            index = Some(k);
                            break;
                        }
                    }
                    let k = if let Some(k) = index {
                        k
                    } else {
                        if next.edges.len() == EDGE_CAP {
                            let victim = next
                                .edges
                                .iter()
                                .enumerate()
                                .min_by_key(|(_, e)| (e.last_sequence, e.from, e.to))
                                .unwrap()
                                .0;
                            work.edge_visits += next.edges.len() as u64;
                            work.record_copies += (next.edges.len() - victim - 1) as u64;
                            next.edges.remove(victim);
                        }
                        next.edges.push(Traversal {
                            from,
                            to: id,
                            displacement: delta,
                            successes: 0,
                            blocked: 0,
                            last_sequence: sequence.raw(),
                        });
                        work.field_updates += 6;
                        next.edges.len() - 1
                    };
                    let e = &mut next.edges[k];
                    e.displacement = delta;
                    e.last_sequence = sequence.raw();
                    // Actual displacement is a visited link even if its final attempted step blocked.
                    e.successes = e.successes.saturating_add(1);
                    if motion.blocked {
                        e.blocked = e.blocked.saturating_add(1)
                    }
                    work.field_updates += 4;
                }
            }
            next.last_visit = Some(id);
            work.field_updates += 1;
        } else if motion.blocked {
            // No fictitious new edge for a zero-motion failed attempt. Lower local confidence.
            if let Some(id) = next.last_visit {
                if let Some(n) = next.visit_nodes.iter_mut().find(|n| {
                    work.node_visits += 1;
                    work.comparisons += 1;
                    n.id == id
                }) {
                    n.confidence *= 0.5;
                    work.field_updates += 1
                }
            }
        }
        for n in &mut next.visit_nodes {
            work.node_visits += 1;
            if Some(n.id) != next.last_visit {
                n.confidence *= 0.99;
                work.field_updates += 1
            }
        }
        work.validation_checks += next.validate_counted()?;
        Ok((next, work))
    }
}

#[cfg(test)]
mod passive_anchor_regression2364 {
    use super::*;
    use alife_core::Confidence;
    fn motion(d: [f32; 2], executed: bool) -> MeasuredMotion {
        MeasuredMotion {
            displacement: d,
            yaw_delta: 0.,
            blocked: false,
            zero_step_blocked: false,
            executed_traversal: executed,
        }
    }
    fn step(m: &ObservedMap, n: u64, d: [f32; 2], executed: bool) -> ObservedMap {
        m.observe(
            ExperienceSequenceId(n),
            &[],
            8.,
            0.,
            motion(d, executed),
            None,
        )
        .unwrap()
        .0
    }
    fn cold(m: &ObservedMap) -> ObservedMap {
        let bytes = m.encode().unwrap();
        let n = ObservedMap::decode(&bytes).unwrap();
        assert_eq!(bytes, n.encode().unwrap());
        n
    }
    #[test]
    fn exact_review_passive_point3_then_actual_point2_no_fake_half_edge() {
        for yaw in [0., std::f32::consts::FRAC_PI_2] {
            let mut initial = motion([0.; 2], false);
            initial.yaw_delta = yaw;
            let a = ObservedMap::default()
                .observe(ExperienceSequenceId(1), &[], 8., 0., initial, None)
                .unwrap()
                .0;
            let old = a.visited_nodes().to_vec();
            let b = cold(&step(&a, 2, [0.3, 0.], false));
            assert!(b.edges().is_empty());
            assert_eq!(b.visited_nodes()[0].position, old[0].position);
            assert_eq!(b.visited_nodes()[0].id, old[0].id);
            let c = cold(&step(&b, 3, [0.2, 0.], true));
            assert!(c.edges().is_empty());
            assert_eq!(c.visited_nodes().len(), 2);
        }
    }
    #[test]
    fn passive_point1_then_actual_point4_only_actual_displacement() {
        let a = step(&ObservedMap::default(), 1, [0.; 2], false);
        let b = cold(&step(&a, 2, [0.1, 0.], false));
        let c = step(&b, 3, [0.4, 0.], true);
        assert_eq!(c.edges().len(), 1);
        assert_eq!(c.edges()[0].from, b.current_visit().unwrap());
        assert_eq!(
            c.edges()[0].displacement[0].to_bits(),
            (c.position()[0] - b.position()[0]).to_bits()
        );
        assert!(c.edges()[0].displacement[0] < 0.41);
    }
    #[test]
    fn signed_zero_and_rotation_do_not_break_normal_anchor() {
        let a = step(&ObservedMap::default(), 1, [0.; 2], false);
        let mut z = motion([-0., 0.], false);
        z.yaw_delta = std::f32::consts::FRAC_PI_2;
        let (b, w) = a
            .observe(ExperienceSequenceId(2), &[], 8., 0., z, None)
            .unwrap();
        assert_eq!(a.current_visit(), b.current_visit());
        assert_eq!(b.visited_nodes().len(), 1);
        assert!(w.comparisons >= 3);
        let c = step(&cold(&b), 3, [0.5, 0.], true);
        assert_eq!(c.edges().len(), 1);
        assert_eq!(c.edges()[0].from, a.current_visit().unwrap());
        assert_eq!(c.edges()[0].successes, 1);
    }
    #[test]
    fn passive_near_identical_landmark_must_not_reuse_stale_anchor() {
        let a = step(&ObservedMap::default(), 1, [0.; 2], false);
        let b = step(&a, 2, [0.01, 0.], false);
        assert_ne!(a.current_visit(), b.current_visit());
        assert_eq!(b.visited_nodes().len(), 2);
        assert!(b.edges().is_empty());
        let c = step(&b, 3, [0.5, 0.], true);
        assert_eq!(c.edges()[0].from, b.current_visit().unwrap());
    }
    fn visible() -> GroundedObjectSlotV1 {
        GroundedObjectSlotV1 {
            slot_index: 0,
            tracked_object_id: TrackedObjectId(1),
            bearing: [0., 1.],
            distance: 0.5,
            relative_velocity: [0.; 3],
            color: [0.; 3],
            material: [0.; 3],
            shape: [0.; 3],
            chemical: [0.; 3],
            contact: 0.,
            proprioception: [0.; 2],
            temperature: 0.,
            terrain: [0.; 2],
            confidence: Confidence::new(1.).unwrap(),
        }
    }
    #[test]
    fn shared_landmark_cannot_loop_match_passive_near_anchor() {
        let slots = [visible()];
        let (a, _) = ObservedMap::default()
            .observe(
                ExperienceSequenceId(1),
                &slots,
                8.,
                0.,
                motion([0.; 2], false),
                None,
            )
            .unwrap();
        let (b, _) = a
            .observe(
                ExperienceSequenceId(2),
                &slots,
                8.,
                0.,
                motion([0.01, 0.], false),
                None,
            )
            .unwrap();
        assert_ne!(a.current_visit(), b.current_visit());
        assert_eq!(b.visited_nodes().len(), 2);
        assert!(b.edges().is_empty());
        let c = b
            .observe(
                ExperienceSequenceId(3),
                &slots,
                8.,
                0.,
                motion([0.5, 0.], true),
                None,
            )
            .unwrap()
            .0;
        assert_eq!(c.edges()[0].from, b.current_visit().unwrap());
    }
}

#[cfg(test)]
mod world_frame_regressions2367 {
    use super::*;
    use alife_core::Confidence;
    fn slot() -> GroundedObjectSlotV1 {
        GroundedObjectSlotV1 {
            slot_index: 0,
            tracked_object_id: TrackedObjectId(99),
            bearing: [0., 1.],
            distance: 0.5,
            relative_velocity: [0.; 3],
            color: [0.; 3],
            material: [0.; 3],
            shape: [0.; 3],
            chemical: [0.; 3],
            contact: 0.,
            proprioception: [0.; 2],
            temperature: 0.,
            terrain: [0.; 2],
            confidence: Confidence::new(1.).unwrap(),
        }
    }
    fn zero() -> MeasuredMotion {
        MeasuredMotion {
            displacement: [0.; 2],
            yaw_delta: 0.,
            blocked: false,
            zero_step_blocked: false,
            executed_traversal: false,
        }
    }
    fn fresh() -> ObservedMap {
        ObservedMap::default()
            .observe(
                ExperienceSequenceId(1),
                &[slot()],
                8.,
                0.,
                zero(),
                Some(TrackedObjectId(99)),
            )
            .unwrap()
            .0
    }
    // Synthetic old-wire fixture only. No retained organism or acquired state is edited.
    fn legacy() -> ObservedMap {
        let mut v: serde_json::Value = serde_json::from_slice(&fresh().encode().unwrap()).unwrap();
        v["version"] = serde_json::json!(2);
        for n in v["nodes"].as_array_mut().unwrap() {
            n.as_object_mut()
                .unwrap()
                .remove("world_frame_verified2367");
        }
        ObservedMap::decode(&serde_json::to_vec(&v).unwrap()).unwrap()
    }
    #[test]
    fn old_coordinates_migrate_unknown_without_reconstruction() {
        let old = legacy();
        let n = old.with_world_slot_frame2367().unwrap();
        assert_eq!(n.nodes(), old.nodes());
        assert_eq!(n.edges(), old.edges());
        assert_eq!(n.visited_nodes(), old.visited_nodes());
        assert_eq!(n.goal(), old.goal());
        assert!(!n.landmark_position_verified2367(&n.nodes()[0]));
        assert!(n.with_world_slot_frame2367().is_err());
        assert_eq!(
            super::super::goal_route2349::preference(&n, &[], 0., &[])
                .unwrap()
                .mode,
            "ExploreBacktrackFallback"
        );
        let cold = ObservedMap::decode(&n.encode().unwrap()).unwrap();
        assert_eq!(n.encode().unwrap(), cold.encode().unwrap());
    }
    #[test]
    fn rotation_reobservation_absence_and_revisit_keep_world_position() {
        let old = legacy();
        let upgraded = old.with_world_slot_frame2367().unwrap();
        let mut motion = zero();
        motion.yaw_delta = 1.;
        let (map, work) = upgraded
            .observe(ExperienceSequenceId(2), &[slot()], 8., -1., motion, None)
            .unwrap();
        assert_eq!(map.nodes()[0].position, old.nodes()[0].position);
        assert!(map.landmark_position_verified2367(&map.nodes()[0]));
        assert!(work.field_updates > 0);
        let absent = map
            .observe(ExperienceSequenceId(3), &[], 8., -1., zero(), None)
            .unwrap()
            .0;
        assert_eq!(absent.nodes()[0].position, map.nodes()[0].position);
        assert!(absent.landmark_position_verified2367(&absent.nodes()[0]));
        assert!(absent.nodes()[0].confidence < map.nodes()[0].confidence);
        let back = absent
            .observe(ExperienceSequenceId(4), &[slot()], 8., -1., zero(), None)
            .unwrap()
            .0;
        assert_eq!(back.nodes()[0].position, map.nodes()[0].position);
        assert_eq!(back.nodes()[0].confidence, 1.);
        let r = super::super::route_policy2349::preference(&map, &[slot()], -1., &[]).unwrap();
        assert!((r.target_heading_body.unwrap() + 1.).abs() < 1e-6);
    }
    #[test]
    fn corrected_world_bearing_scene_rotations_match() {
        let original = fresh();
        for phi in [-0.8_f32, 0.7, 2.1] {
            let mut s = slot();
            s.bearing = [phi.sin(), phi.cos()];
            let m = ObservedMap::default()
                .observe(ExperienceSequenceId(1), &[s], 8., -phi, zero(), None)
                .unwrap()
                .0;
            let a = m.nodes()[0].position;
            let b = original.nodes()[0].position;
            assert!((a[0] - b[0]).abs() < 2e-6 && (a[1] - b[1]).abs() < 2e-6);
        }
    }
    #[test]
    fn invalid_frame_evidence_and_legacy_relabel_rejected() {
        let f = fresh();
        assert_eq!(f.frame_version2367(), 3);
        let mut v: serde_json::Value = serde_json::from_slice(&f.encode().unwrap()).unwrap();
        v["version"] = serde_json::json!(2);
        assert!(ObservedMap::decode(&serde_json::to_vec(&v).unwrap()).is_err());
        v["version"] = serde_json::json!(3);
        v["nodes"][0]["world_frame_verified2367"] = serde_json::json!(false);
        assert!(ObservedMap::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    }
}
