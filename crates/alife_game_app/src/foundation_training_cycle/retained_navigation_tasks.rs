//! Public-world tasks for retained lives. No teacher actions, body relocation, or private-state edits.
//! MovingPrey is an externally propelled edible Food proxy, NOT autonomous prey ecology.
use crate::foundation_training::food_outcome::{
    ownership, FoodOutcomeObservation, FoodOwnershipSnapshot, FoundationFoodOutcome,
};
use alife_core::{ExperiencePatch, OrganismId, Tick, Vec3f, WorldEntityId};
use alife_world::{HeadlessWorld, WorldEditorSpawnSpec, WorldObjectKind};
use serde::{Deserialize, Serialize};
type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    LCorner,
    TwoTurnMaze,
    MovingPrey,
    /// Native world-owned reactive/wandering edible critter, not a neural organism.
    AutonomousPrey,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub kind: TaskKind,
    /// Public geometry rotation relative to the actor's unchanged initial body heading.
    pub rotation_radians: f32,
    pub mirror: bool,
    /// Used by MovingPrey and AutonomousPrey, horizontal world metres per actual native interval (not metres/second).
    pub prey_speed_per_tick: f32,
}
impl TaskSpec {
    pub(crate) fn validate(&self) -> Result<()> {
        if !self.rotation_radians.is_finite()
            || !self.prey_speed_per_tick.is_finite()
            || !(0.0..=0.08).contains(&self.prey_speed_per_tick)
            || (matches!(self.kind, TaskKind::MovingPrey | TaskKind::AutonomousPrey)
                && self.prey_speed_per_tick <= 0.0)
            || (self.kind == TaskKind::AutonomousPrey && self.prey_speed_per_tick < 0.001)
        {
            return Err("invalid retained navigation task parameters".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RelocationReceipt {
    pub entity: WorldEntityId,
    pub before: Vec3f,
    pub after: Vec3f,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MotionReceipt {
    pub entity: WorldEntityId,
    pub tick: Tick,
    pub before: Vec3f,
    pub after: Vec3f,
    pub velocity: Vec3f,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskState {
    pub spec: TaskSpec,
    pub actor: OrganismId,
    pub target: WorldEntityId,
    pub walls: Vec<WorldEntityId>,
    pub relocations: Vec<RelocationReceipt>,
    pub preserved_held_entities: Vec<WorldEntityId>,
    pub origin: Vec3f,
    pub goal_position: Vec3f,
    pub geometry_yaw: f32,
    pub start_tick: Tick,
    pub last_tick: Tick,
    pub elapsed_ticks: u64,
    pub food_outcome: FoundationFoodOutcome,
    pub first_capture_tick: Option<Tick>,
    /// Native ownership telemetry, not proof of nourishment.
    pub held_ticks: u64,
    pub longest_held_ticks: u64,
    pub release_count: u64,
    pub first_meal_tick: Option<Tick>,
    pub first_traversal_tick: Option<Tick>,
    pub path_length: f32,
    pub minimum_target_distance: f32,
    pub crossed_wall_outside_span: bool,
    pub maze_gates_completed: u8,
    pub maze_route_invalid: bool,
    pub last_target_motion: Option<MotionReceipt>,
    pub autonomous_controller_seed: Option<u64>,
    previous_actor_position: Vec3f,
    previous_ownership: Option<FoodOwnershipSnapshot>,
    motion_direction: f32,
    consecutive_held_ticks: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskObservation {
    pub kind: TaskKind,
    pub tick: Tick,
    pub elapsed_ticks: u64,
    pub target: WorldEntityId,
    pub actor_position: Vec3f,
    pub target_position: Vec3f,
    pub distance: f32,
    pub goal_distance: f32,
    pub minimum_distance: f32,
    pub path_length: f32,
    pub captured: bool,
    pub native_target_consumed: bool,
    pub food_outcome: FoundationFoodOutcome,
    pub traversed: bool,
    pub maze_gates_completed: u8,
    pub maze_route_invalid: bool,
    pub first_capture_tick: Option<Tick>,
    /// Native ownership telemetry, not proof of nourishment.
    pub held_ticks: u64,
    pub longest_held_ticks: u64,
    pub release_count: u64,
    pub first_meal_tick: Option<Tick>,
    pub first_traversal_tick: Option<Tick>,
    pub target_motion: Option<MotionReceipt>,
    pub autonomous_critter: Option<alife_world::AutonomousCritterState>,
}
fn distance(a: Vec3f, b: Vec3f) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}
fn position(origin: Vec3f, yaw: f32, forward: f32, lateral: f32) -> Vec3f {
    let (s, c) = yaw.sin_cos();
    Vec3f::new(
        origin.x + forward * c - lateral * s,
        origin.y,
        origin.z + forward * s + lateral * c,
    )
}
fn local(origin: Vec3f, yaw: f32, p: Vec3f) -> [f32; 2] {
    let (s, c) = yaw.sin_cos();
    let x = p.x - origin.x;
    let z = p.z - origin.z;
    [x * c + z * s, z * c - x * s]
}
fn own(world: &HeadlessWorld, actor: OrganismId) -> Result<alife_world::WorldObject> {
    let mut all = world
        .object_snapshots()
        .into_iter()
        .filter(|o| o.organism_id == Some(actor));
    let first = all.next().ok_or("missing task actor")?;
    if all.next().is_some() || first.consumed {
        return Err("invalid task actor identity".into());
    }
    Ok(first)
}
fn wall_layout() -> Vec<[f32; 2]> {
    // Connected discs form a genuine L: direct start-to-goal line intersects the vertical leg.
    let mut walls = (-3..=3).map(|i| [1.5, i as f32 * 0.5]).collect::<Vec<_>>();
    walls.extend((1..=3).map(|i| [1.5 + i as f32 * 0.5, 1.5]));
    walls
}
// Enclosed 10m x 6m corridor with two alternating baffles: upper gap first,
// lower gap second. The perimeter prevents a straight outside-wall shortcut.
fn maze_layout() -> Vec<[f32; 2]> {
    let mut walls = Vec::new();
    for i in -3..=17 {
        walls.push([i as f32 * 0.5, -3.0]);
        walls.push([i as f32 * 0.5, 3.0]);
    }
    for i in -5..=5 {
        walls.push([-1.5, i as f32 * 0.5]);
        walls.push([8.5, i as f32 * 0.5]);
    }
    for i in -6..=2 {
        walls.push([2.0, i as f32 * 0.5]);
    }
    for i in -2..=6 {
        walls.push([5.0, i as f32 * 0.5]);
    }
    walls
}
// Receipts use actual actor motion, not intent or proximity to the target.
// Mirror is undone by the caller. Backtracking is allowed; skipped gates and
// an exterior route cannot earn a traversal even if the target gets consumed.
fn score_maze_segment(gates: &mut u8, invalid: &mut bool, p: [f32; 2], q: [f32; 2]) {
    if p.iter().chain(q.iter()).any(|x| !x.is_finite())
        || [p, q]
            .iter()
            .any(|v| v[0] < -1.5 || v[0] > 8.5 || v[1].abs() > 3.0)
    {
        *invalid = true;
        return;
    }
    if q[0] <= p[0] {
        return;
    }
    // Score the geometric openings, not an assumed body envelope. Native motion
    // already enforces obstacle clearance using this world's locomotion limits.
    for (index, x, low, high) in [(0u8, 2.0, 1.0, 3.0), (1u8, 5.0, -3.0, -1.0)] {
        if p[0] < x && q[0] >= x {
            let z = p[1] + (q[1] - p[1]) * (x - p[0]) / (q[0] - p[0]);
            if z <= low || z >= high || *gates < index {
                *invalid = true;
            } else if *gates == index {
                *gates += 1;
            }
        }
    }
}
fn spawn(
    world: &mut HeadlessWorld,
    label: String,
    kind: WorldObjectKind,
    p: Vec3f,
    radius: f32,
    nutrition: f32,
) -> Result<WorldEntityId> {
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label,
            kind,
            organism_id: None,
            position: p,
            nutrition,
            hazard_pain: 0.0,
            radius,
            token_id: None,
        })
        .map_err(|e| e.to_string())
}
/// Atomic public scene setup on a clone. No organism pose, physiology, genetics, private state,
/// sampling stream, speech, teacher channel, or learned history is rewritten.
pub(crate) fn configure(
    world: &mut HeadlessWorld,
    actor: OrganismId,
    spec: TaskSpec,
) -> Result<TaskState> {
    spec.validate()?;
    let actor_before = own(world, actor)?;
    let origin = actor_before.position;
    let yaw = actor_before.body_yaw + spec.rotation_radians;
    let sign = if spec.mirror { -1.0 } else { 1.0 };
    let mut staged = world.clone();
    let all = world.object_snapshots();
    let player_held = world.player_held_object();
    let mut relocations = Vec::new();
    let mut held = Vec::new();
    for o in &all {
        let carries = o
            .organism_id
            .is_some_and(|id| all.iter().any(|item| item.carried_by == Some(id)));
        if o.carried_by.is_some() || o.activity_hold || player_held == Some(o.id) || carries {
            held.push(o.id);
            continue;
        }
        if o.organism_id == Some(actor) || o.consumed {
            continue;
        }

        if distance(origin, o.position) <= 10.0
            && o.organism_id.is_none()
            && o.teacher_channel.is_none()
            && o.label != "nursery-teacher"
            && matches!(o.kind, WorldObjectKind::Food | WorldObjectKind::Obstacle)
        {
            let park = position(origin, yaw, -24.0 - (relocations.len() as f32) * 1.5, 12.0);
            staged
                .editor_move_object(o.id, park)
                .map_err(|e| format!("public relocation {}: {e}", o.id.raw()))?;
            staged
                .editor_move_object(o.id, park)
                .map_err(|e| e.to_string())?; // explicit public STOP
            relocations.push(RelocationReceipt{entity:o.id,before:o.position,after:staged.entity(o.id).ok_or("relocation lost object")?.position,reason:"public classroom relocation; object, ownership history and teacher channel retained".into()});
        }
    }
    let mut walls = Vec::new();
    if matches!(spec.kind, TaskKind::LCorner | TaskKind::TwoTurnMaze) {
        let layout = if spec.kind == TaskKind::TwoTurnMaze {
            maze_layout()
        } else {
            wall_layout()
        };
        for (i, p) in layout.iter().enumerate() {
            walls.push(spawn(
                &mut staged,
                format!("retained-navigation-{}-wall-{i}", world.tick().raw()),
                WorldObjectKind::Obstacle,
                position(origin, yaw, p[0], sign * p[1]),
                0.35,
                0.0,
            )?);
        }
    }
    let target_local = if spec.kind == TaskKind::TwoTurnMaze {
        [7.5, 0.0]
    } else if spec.kind == TaskKind::LCorner {
        [4.0, 0.0]
    } else {
        [3.0, -1.5 * sign]
    };
    let target = spawn(
        &mut staged,
        format!("retained-navigation-{}-target", world.tick().raw()),
        WorldObjectKind::Food,
        position(origin, yaw, target_local[0], target_local[1]),
        0.3,
        0.6,
    )?;
    let autonomous_controller_seed = if spec.kind == TaskKind::AutonomousPrey {
        let seed = (world.seed()
            ^ world.tick().raw().wrapping_mul(0x9e3779b97f4a7c15)
            ^ target.raw().wrapping_mul(0xd1b54a32d192ed03))
            | 1;
        staged
            .enable_autonomous_critter(target, seed, spec.prey_speed_per_tick, 4.0)
            .map_err(|e| e.to_string())?;
        Some(seed)
    } else {
        None
    };
    if own(&staged, actor)? != actor_before {
        return Err("task setup altered actor".into());
    }
    let minimum = distance(origin, staged.entity(target).ok_or("target")?.position);
    let state = TaskState {
        spec,
        actor,
        target,
        walls,
        relocations,
        preserved_held_entities: held,
        origin,
        goal_position: staged.entity(target).ok_or("target")?.position,
        geometry_yaw: yaw,
        start_tick: world.tick(),
        last_tick: world.tick(),
        elapsed_ticks: 0,
        food_outcome: FoundationFoodOutcome::new(actor, target),
        first_capture_tick: None,
        held_ticks: 0,
        longest_held_ticks: 0,
        release_count: 0,
        consecutive_held_ticks: 0,
        first_meal_tick: None,
        first_traversal_tick: None,
        path_length: 0.0,
        minimum_target_distance: minimum,
        crossed_wall_outside_span: false,
        maze_gates_completed: 0,
        maze_route_invalid: false,
        last_target_motion: None,
        autonomous_controller_seed,
        previous_actor_position: origin,
        previous_ownership: ownership(&staged, target),
        motion_direction: sign,
    };
    *world = staged;
    Ok(state)
}
fn lateral_step(z: f32, direction: f32, speed: f32) -> (f32, f32) {
    let mut d = direction;
    if z >= 1.5 {
        d = -1.0;
    } else if z <= -1.5 {
        d = 1.0;
    }
    let mut next = z + d * speed;
    if z.abs() <= 1.5 {
        if next > 1.5 {
            next = 3.0 - next;
            d = -1.0;
        } else if next < -1.5 {
            next = -3.0 - next;
            d = 1.0;
        }
    }
    (next, d)
}
/// Call exactly once AFTER each actual native tick, with that tick's sealed patch when captured.
/// Scores the just-finished interval, then moves only the free proxy for the NEXT interval.
/// None means no captured patch (e.g. sleep), not permission to infer a meal from proximity.
pub(crate) fn advance_after_actual_tick(
    world: &mut HeadlessWorld,
    actor: OrganismId,
    state: &mut TaskState,
    patch: Option<&ExperiencePatch>,
) -> Result<TaskObservation> {
    if actor != state.actor
        || world.tick().raw()
            != state
                .last_tick
                .raw()
                .checked_add(1)
                .ok_or("tick overflow")?
    {
        return Err("task requires exactly one actual next world tick".into());
    }
    if let Some(p) = patch {
        if p.header().phase != alife_core::ExperiencePatchPhase::Sealed
            || p.header().organism_id != actor
            || p.header().world_tick != state.last_tick
            || p.outcome().outcome_tick != world.tick()
        {
            return Err("task patch is not this actor/interval".into());
        }
    }
    let a = own(world, actor)?;
    let target = world
        .entity(state.target)
        .ok_or("task target disappeared")?
        .clone();
    let mut next = state.clone();
    next.elapsed_ticks += 1;
    let held_now = target.carried_by == Some(actor) && !target.consumed;
    if held_now {
        next.held_ticks += 1;
        next.consecutive_held_ticks += 1;
        next.longest_held_ticks = next.longest_held_ticks.max(next.consecutive_held_ticks);
    } else {
        next.consecutive_held_ticks = 0;
    }
    // Consumption is a separate outcome, not a release. Counts observed ownership
    // transitions even when a captured decision patch is unavailable.
    if state
        .previous_ownership
        .is_some_and(|p| p.owner == Some(actor) && !p.consumed)
        && !held_now
        && !target.consumed
    {
        next.release_count += 1;
    }
    next.path_length += distance(state.previous_actor_position, a.position);
    next.minimum_target_distance = next
        .minimum_target_distance
        .min(distance(a.position, target.position));
    if let Some(p) = patch {
        next.food_outcome.observe(
            FoodOutcomeObservation {
                organism: actor,
                target: state.target,
                decision_tick: state.last_tick,
                outcome_tick: world.tick(),
                before: state.previous_ownership,
                after: ownership(world, state.target),
            },
            p,
        );
    }
    if next.food_outcome.ordinary_acquisition_events > 0 && next.first_capture_tick.is_none() {
        next.first_capture_tick = Some(world.tick());
    }
    if next.food_outcome.nourished_meal_events > 0 && next.first_meal_tick.is_none() {
        next.first_meal_tick = Some(world.tick());
    }
    let p = local(
        state.origin,
        state.geometry_yaw,
        state.previous_actor_position,
    );
    let q = local(state.origin, state.geometry_yaw, a.position);
    if state.spec.kind == TaskKind::LCorner
        && (p[0] - 1.5) * (q[0] - 1.5) <= 0.0
        && (q[0] - p[0]).abs() > 1e-6
    {
        let z = p[1] + (q[1] - p[1]) * (1.5 - p[0]) / (q[0] - p[0]);
        if z.abs() > 2.05 {
            next.crossed_wall_outside_span = true;
        }
    }
    if state.spec.kind == TaskKind::LCorner
        && next.crossed_wall_outside_span
        && distance(a.position, state.goal_position) <= 0.75
        && next.first_traversal_tick.is_none()
    {
        next.first_traversal_tick = Some(world.tick());
    }
    if state.spec.kind == TaskKind::TwoTurnMaze {
        let sign = if state.spec.mirror { -1.0 } else { 1.0 };
        score_maze_segment(
            &mut next.maze_gates_completed,
            &mut next.maze_route_invalid,
            [p[0], sign * p[1]],
            [q[0], sign * q[1]],
        );
        if next.maze_gates_completed == 2
            && !next.maze_route_invalid
            && distance(a.position, state.goal_position) <= 0.75
            && next.first_traversal_tick.is_none()
        {
            next.first_traversal_tick = Some(world.tick());
        }
    }
    next.previous_actor_position = a.position;
    next.previous_ownership = ownership(world, state.target);
    next.last_tick = world.tick();
    next.last_target_motion = None;
    let scored = observe(world, actor, &next)?;
    if state.spec.kind == TaskKind::MovingPrey
        && !target.consumed
        && target.carried_by.is_none()
        && !target.activity_hold
        && world.player_held_object() != Some(target.id)
    {
        let p = local(state.origin, state.geometry_yaw, target.position);
        let (z, d) = lateral_step(p[1], state.motion_direction, state.spec.prey_speed_per_tick);
        let requested = position(state.origin, state.geometry_yaw, p[0], z);
        world
            .editor_move_object(target.id, requested)
            .map_err(|e| e.to_string())?;
        let after = world.entity(target.id).ok_or("motion target")?;
        next.motion_direction = d;
        next.last_target_motion = Some(MotionReceipt {
            entity: target.id,
            tick: world.tick(),
            before: target.position,
            after: after.position,
            velocity: after.grounded_physical.velocity,
        });
    }
    let mut result = scored;
    result.target_motion = next.last_target_motion.clone();
    *state = next;
    Ok(result)
}
pub(crate) fn observe(
    world: &HeadlessWorld,
    actor: OrganismId,
    state: &TaskState,
) -> Result<TaskObservation> {
    if actor != state.actor {
        return Err("task actor mismatch".into());
    }
    let a = own(world, actor)?;
    let t = world.entity(state.target).ok_or("task target")?;
    Ok(TaskObservation {
        kind: state.spec.kind,
        tick: world.tick(),
        elapsed_ticks: state.elapsed_ticks,
        target: state.target,
        actor_position: a.position,
        target_position: t.position,
        distance: distance(a.position, t.position),
        goal_distance: distance(a.position, state.goal_position),
        minimum_distance: state.minimum_target_distance,
        path_length: state.path_length,
        captured: state.first_capture_tick.is_some(),
        native_target_consumed: t.consumed,
        food_outcome: state.food_outcome.clone(),
        traversed: state.first_traversal_tick.is_some(),
        maze_gates_completed: state.maze_gates_completed,
        maze_route_invalid: state.maze_route_invalid,
        first_capture_tick: state.first_capture_tick,
        held_ticks: state.held_ticks,
        longest_held_ticks: state.longest_held_ticks,
        release_count: state.release_count,
        first_meal_tick: state.first_meal_tick,
        first_traversal_tick: state.first_traversal_tick,
        target_motion: state.last_target_motion.clone(),
        autonomous_critter: world.autonomous_critter(state.target).cloned(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use alife_world::HeadlessScenarioBuilder;
    #[test]
    fn l_corner_really_blocks_direct_line_and_has_clear_detour() {
        let walls = wall_layout();
        assert!(walls.iter().any(|p| p[1] == 0.0));
        // Physical body radius0.2 plus wall0.35; use stricter0.95 margin in witness.
        for segment in [
            ([0.0, 0.0], [0.0, -2.7]),
            ([0.0, -2.7], [4.0, -2.7]),
            ([4.0, -2.7], [4.0, 0.0]),
        ] {
            for i in 0..=100 {
                let u = i as f32 / 100.0;
                let p = [
                    segment.0[0] + u * (segment.1[0] - segment.0[0]),
                    segment.0[1] + u * (segment.1[1] - segment.0[1]),
                ];
                assert!(walls
                    .iter()
                    .all(|w| (p[0] - w[0]).hypot(p[1] - w[1]) > 0.95));
            }
        }
    }
    #[test]
    fn bounded_motion_reflects_without_release_teleport() {
        for z in [-3.0, -1.5, -1.49, 0.0, 1.49, 1.5, 3.0] {
            for d in [-1.0, 1.0] {
                let (n, _) = lateral_step(z, d, 0.02);
                assert!((n - z).abs() <= 0.02001);
                if z.abs() <= 1.5 {
                    assert!(n.abs() <= 1.5);
                }
            }
        }
    }
    #[test]
    fn public_configuration_keeps_actor_and_produces_native_velocity() {
        let mut w = HeadlessScenarioBuilder::new(7)
            .agent("subject", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        let actor = own(&w, OrganismId(1)).unwrap();
        let mut s = configure(
            &mut w,
            OrganismId(1),
            TaskSpec {
                kind: TaskKind::MovingPrey,
                rotation_radians: 0.7,
                mirror: false,
                prey_speed_per_tick: 0.02,
            },
        )
        .unwrap();
        assert_eq!(own(&w, OrganismId(1)).unwrap(), actor);
        w.advance_tick();
        let r = advance_after_actual_tick(&mut w, OrganismId(1), &mut s, None).unwrap();
        let m = r.target_motion.unwrap();
        assert!(distance(m.before, m.after) > 0.0199);
        assert_eq!(
            m.velocity,
            Vec3f::new(
                m.after.x - m.before.x,
                m.after.y - m.before.y,
                m.after.z - m.before.z
            )
        );
        assert_eq!(s.food_outcome.nourished_meal_events, 0);
        assert!(advance_after_actual_tick(&mut w, OrganismId(1), &mut s, None).is_err());
    }
    #[test]
    fn native_lcorner_setup_is_zero_step_and_preserves_passive_teacher() {
        let mut w = HeadlessScenarioBuilder::new(9)
            .agent("subject", OrganismId(1), Vec3f::ZERO)
            .agent("nursery-teacher", OrganismId(2), Vec3f::ZERO)
            .food("old-food", Vec3f::new(0.3, 0.0, 0.0), 0.4)
            .build()
            .unwrap();
        let before = own(&w, OrganismId(1)).unwrap();
        let teacher = own(&w, OrganismId(2)).unwrap();
        let tick = w.tick();
        let state = configure(
            &mut w,
            OrganismId(1),
            TaskSpec {
                kind: TaskKind::LCorner,
                rotation_radians: 0.7,
                mirror: true,
                prey_speed_per_tick: 0.0,
            },
        )
        .unwrap();
        assert_eq!(w.tick(), tick);
        assert_eq!(own(&w, OrganismId(1)).unwrap(), before);
        assert_eq!(state.walls.len(), 10);
        for (id, p) in state.walls.iter().zip(wall_layout()) {
            let actual = w.entity(*id).unwrap();
            assert_eq!(actual.kind, WorldObjectKind::Obstacle);
            assert!(
                distance(
                    actual.position,
                    position(state.origin, state.geometry_yaw, p[0], -p[1])
                ) < 1e-5
            );
            assert!(distance(actual.position, state.origin) < 4.0);
        }
        let after = own(&w, OrganismId(2)).unwrap();
        assert_eq!(teacher.teacher_channel, after.teacher_channel);
        assert_eq!(teacher.id, after.id);
        assert_eq!(after, teacher);
        assert!(!state.relocations.iter().any(|r| r.entity == teacher.id));
        assert_eq!(state.food_outcome.nourished_meal_events, 0);
        assert!(!observe(&w, OrganismId(1), &state).unwrap().traversed);
    }
    #[test]
    fn two_turn_maze_has_blocked_direct_route_and_clear_alternating_route() {
        let walls = maze_layout();
        assert!(walls.contains(&[2.0, 0.0]) && walls.contains(&[5.0, 0.0]));
        let route = [
            [0.0, 0.0],
            [0.0, 2.0],
            [3.5, 2.0],
            [3.5, -2.0],
            [7.5, -2.0],
            [7.5, 0.0],
        ];
        let (mut gates, mut invalid) = (0, false);
        for segment in route.windows(2) {
            score_maze_segment(&mut gates, &mut invalid, segment[0], segment[1]);
            for i in 0..=100 {
                let u = i as f32 / 100.0;
                let p = [
                    segment[0][0] + u * (segment[1][0] - segment[0][0]),
                    segment[0][1] + u * (segment[1][1] - segment[0][1]),
                ];
                assert!(walls.iter().all(|w| (p[0] - w[0]).hypot(p[1] - w[1]) > 0.9));
            }
        }
        assert_eq!(gates, 2);
        assert!(!invalid);
    }
    #[test]
    fn two_turn_score_rejects_direct_skip_and_outside_routes() {
        for route in [
            vec![[0.0, 0.0], [7.5, 0.0]],
            vec![[3.5, -2.0], [7.5, -2.0]],
            vec![[0.0, 0.0], [0.0, 4.0], [7.5, 4.0], [7.5, 0.0]],
        ] {
            let (mut gates, mut invalid) = (0, false);
            for segment in route.windows(2) {
                score_maze_segment(&mut gates, &mut invalid, segment[0], segment[1]);
            }
            assert!(invalid);
        }
    }
    #[test]
    fn two_turn_setup_is_rotated_mirrored_and_does_not_move_either_agent() {
        for mirror in [false, true] {
            let mut w = HeadlessScenarioBuilder::new(19)
                .agent("subject", OrganismId(1), Vec3f::ZERO)
                .agent("nursery-teacher", OrganismId(2), Vec3f::ZERO)
                .build()
                .unwrap();
            let before = [
                own(&w, OrganismId(1)).unwrap(),
                own(&w, OrganismId(2)).unwrap(),
            ];
            let tick = w.tick();
            let state = configure(
                &mut w,
                OrganismId(1),
                TaskSpec {
                    kind: TaskKind::TwoTurnMaze,
                    rotation_radians: 0.7,
                    mirror,
                    prey_speed_per_tick: 0.0,
                },
            )
            .unwrap();
            assert_eq!(w.tick(), tick);
            assert_eq!(own(&w, OrganismId(1)).unwrap(), before[0]);
            assert_eq!(own(&w, OrganismId(2)).unwrap(), before[1]);
            assert_eq!(state.walls.len(), maze_layout().len());
            let sign = if mirror { -1.0 } else { 1.0 };
            for (id, p) in state.walls.iter().zip(maze_layout()) {
                assert!(
                    distance(
                        w.entity(*id).unwrap().position,
                        position(state.origin, state.geometry_yaw, p[0], sign * p[1])
                    ) < 1e-5
                );
            }
            assert!(!observe(&w, OrganismId(1), &state).unwrap().traversed);
        }
    }
    #[test]
    fn autonomous_prey_moves_only_in_native_tick_and_preserves_agents() {
        let mut w = HeadlessScenarioBuilder::new(23)
            .agent("subject", OrganismId(1), Vec3f::ZERO)
            .agent("nursery-teacher", OrganismId(2), Vec3f::ZERO)
            .build()
            .unwrap();
        let agents = [
            own(&w, OrganismId(1)).unwrap(),
            own(&w, OrganismId(2)).unwrap(),
        ];
        let mut state = configure(
            &mut w,
            OrganismId(1),
            TaskSpec {
                kind: TaskKind::AutonomousPrey,
                rotation_radians: 0.0,
                mirror: false,
                prey_speed_per_tick: 0.02,
            },
        )
        .unwrap();
        assert!(state.walls.is_empty());
        assert!(state.autonomous_controller_seed.is_some());
        assert_eq!(own(&w, OrganismId(1)).unwrap(), agents[0]);
        assert_eq!(own(&w, OrganismId(2)).unwrap(), agents[1]);
        let before = w.entity(state.target).unwrap().position;
        w.advance_tick();
        let after_native = w.entity(state.target).unwrap().position;
        assert!(distance(before, after_native) > 0.019);
        let observed = advance_after_actual_tick(&mut w, OrganismId(1), &mut state, None).unwrap();
        assert_eq!(w.entity(state.target).unwrap().position, after_native);
        assert!(observed.target_motion.is_none());
        assert_eq!(
            observed.autonomous_critter.unwrap().last_mode,
            alife_world::CritterMode::Flee
        );
        assert_eq!(observed.food_outcome.nourished_meal_events, 0);
        assert_eq!(observed.held_ticks, 0);
        assert_eq!(observed.release_count, 0);
        assert!(!observed.captured);
    }
    #[test]
    fn ownership_metrics_distinguish_grab_release_from_consumption() {
        use alife_world::{HeadlessActionIds, HeadlessWorldCommand};
        let actor = OrganismId(1);
        let mut w = HeadlessScenarioBuilder::new(31)
            .agent("subject", actor, Vec3f::ZERO)
            .build()
            .unwrap();
        let mut s = configure(
            &mut w,
            actor,
            TaskSpec {
                kind: TaskKind::AutonomousPrey,
                rotation_radians: 0.0,
                mirror: false,
                prey_speed_per_tick: 0.02,
            },
        )
        .unwrap();
        // Test-only scene setup. Production setup never moves the learner or
        // injects these actions; they exercise the native grab toggle contract.
        w.editor_move_object(s.target, Vec3f::new(0.0, 0.0, 0.6))
            .unwrap();
        let grab = alife_core::ActionCommand::structured(
            actor,
            HeadlessActionIds::GRAB,
            alife_core::ActionKind::Hold,
            alife_core::ActionTarget::new(Some(s.target), None),
            alife_core::Intensity::new(1.0).unwrap(),
            alife_core::DurationTicks::new(1),
            alife_core::Confidence::new(0.9).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap();
        assert!(w.apply_command(&grab).unwrap().execution.succeeded);
        for _ in 0..2 {
            w.advance_tick();
            advance_after_actual_tick(&mut w, actor, &mut s, None).unwrap();
        }
        assert_eq!(
            (s.held_ticks, s.longest_held_ticks, s.release_count),
            (2, 2, 0)
        );
        assert!(w.apply_command(&grab).unwrap().execution.succeeded);
        w.advance_tick();
        advance_after_actual_tick(&mut w, actor, &mut s, None).unwrap();
        assert_eq!(s.release_count, 1);
        assert!(w.apply_command(&grab).unwrap().execution.succeeded);
        w.advance_tick();
        advance_after_actual_tick(&mut w, actor, &mut s, None).unwrap();
        assert_eq!((s.held_ticks, s.longest_held_ticks), (3, 2));
        let eat = HeadlessWorldCommand::eat(actor, s.target).unwrap();
        assert!(w.apply_command(&eat).unwrap().execution.succeeded);
        w.advance_tick();
        let r = advance_after_actual_tick(&mut w, actor, &mut s, None).unwrap();
        assert!(r.native_target_consumed);
        assert_eq!(r.release_count, 1);
        // Without the sealed action patch, ownership alone cannot claim a
        // measured acquisition or a nourished meal.
        assert_eq!(r.first_capture_tick, None);
        assert_eq!(r.food_outcome.nourished_meal_events, 0);
    }
    #[test]
    fn autonomous_speed_validation_matches_controller_range() {
        for (speed, valid) in [
            (0.0, false),
            (0.0005, false),
            (0.001, true),
            (0.08, true),
            (0.081, false),
        ] {
            let spec = TaskSpec {
                kind: TaskKind::AutonomousPrey,
                rotation_radians: 0.0,
                mirror: false,
                prey_speed_per_tick: speed,
            };
            assert_eq!(spec.validate().is_ok(), valid);
        }
    }
    #[test]
    fn maze_gate_score_does_not_assume_default_body_clearance() {
        // With a native body radius of 0.1, z=1.50 clears the baffle's
        // radius-0.35 endpoint by 0.05. It is inside the real opening.
        let (mut gates, mut invalid) = (0, false);
        score_maze_segment(&mut gates, &mut invalid, [1.8, 1.5], [2.2, 1.5]);
        score_maze_segment(&mut gates, &mut invalid, [4.8, -1.5], [5.2, -1.5]);
        assert_eq!(gates, 2);
        assert!(!invalid);
        for z in [0.99, 1.0, 3.0, 3.01] {
            let (mut gates, mut invalid) = (0, false);
            score_maze_segment(&mut gates, &mut invalid, [1.8, z], [2.2, z]);
            assert!(invalid);
            assert_eq!(gates, 0);
        }
    }
    #[test]
    fn coordinates_are_rotation_translation_equivariant() {
        for yaw in [0.0, 0.7, -1.5] {
            let origin = Vec3f::new(7.0, 0.0, -3.0);
            let p = position(origin, yaw, 4.0, -2.0);
            let q = local(origin, yaw, p);
            assert!((q[0] - 4.0).abs() < 1e-5 && (q[1] + 2.0).abs() < 1e-5);
        }
    }
}
