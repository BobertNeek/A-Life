//! An informed demonstrator walks a plan through ordinary motor choices.
//! Its plan never enters the learner's perception or private prior.
use alife_core::{ActionCandidate, CandidateActionFamily, PerceptionFrame, Vec3f, WorldEntityId};
use alife_world::HeadlessActionIds as Id;
use std::collections::BTreeSet;

#[derive(Default)]
pub(super) struct MazeTeacher {
    pub route: Vec<Vec3f>,
    next: usize,
    centered: bool,
}
impl MazeTeacher {
    pub fn choose<'a>(
        &mut self,
        frame: &'a PerceptionFrame,
        food: WorldEntityId,
    ) -> Option<&'a ActionCandidate> {
        let action = |id| frame.candidates().iter().find(|c| c.action_id == id);
        if !self.centered {
            self.centered = true;
            return action(Id::LOOK_CENTER);
        }
        let position = frame.body().pose.translation;
        while let Some(target) = self.route.get(self.next) {
            if (position.x - target.x).hypot(position.z - target.z) > 0.22 {
                break;
            }
            self.next += 1;
        }
        if let Some(target) = self.route.get(self.next) {
            let gaze = 2.0
                * frame
                    .body()
                    .pose
                    .rotation
                    .y
                    .atan2(frame.body().pose.rotation.w);
            let angle = (target.z - position.z).atan2(target.x - position.x) - gaze;
            let error = (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            return action(if error > 0.18 {
                Id::TURN_LEFT
            } else if error < -0.18 {
                Id::TURN_RIGHT
            } else {
                Id::STEP_FORWARD
            });
        }
        frame
            .candidates()
            .iter()
            .find(|c| c.target.entity == Some(food) && c.family == CandidateActionFamily::Ingest)
            .or_else(|| {
                frame.candidates().iter().find(|c| {
                    c.target.entity == Some(food) && c.family == CandidateActionFamily::Approach
                })
            })
            .or_else(|| action(Id::LOOK_CENTER))
    }
}
pub(super) struct MazeLayout {
    pub walls: Vec<(WorldEntityId, Vec3f)>,
    pub demonstration_route: Vec<Vec3f>,
}
pub(super) fn configure(
    world: &mut alife_world::HeadlessWorld,
    seed: u64,
    origin: Vec3f,
    food: WorldEntityId,
) -> Result<MazeLayout, Box<dyn std::error::Error>> {
    let axis = (super::scenario_random(seed, 21) - 0.5) * 120.0_f32.to_radians();
    let cell = 3.0 + 0.4 * super::scenario_random(seed, 22);
    let side = if super::scenario_random(seed, 23) > 0.5 {
        1
    } else {
        -1
    };
    let mut open: BTreeSet<(i32, i32)> = [
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 2),
        (2, 2),
        (2, 1),
        (2, 0),
        (-1, 1),
        (3, 2),
    ]
    .into_iter()
    .collect();
    if seed & 2 != 0 {
        open.insert((-2, 1));
    }
    if seed & 4 != 0 {
        open.insert((4, 2));
    }
    let position = |(x, z): (i32, i32)| {
        Vec3f::new(
            origin.x + cell * (x as f32 * axis.cos() - (z * side) as f32 * axis.sin()),
            origin.y,
            origin.z + cell * (x as f32 * axis.sin() + (z * side) as f32 * axis.cos()),
        )
    };
    let mut reached = BTreeSet::from([(0, 0)]);
    let mut frontier = vec![(0, 0)];
    while let Some((x, z)) = frontier.pop() {
        for next in [(x + 1, z), (x - 1, z), (x, z + 1), (x, z - 1)] {
            if open.contains(&next) && reached.insert(next) {
                frontier.push(next);
            }
        }
    }
    if reached != open || !reached.contains(&(2, 0)) || cell - 1.1 <= 0.75 {
        return Err("maze is disconnected or too narrow".into());
    }
    super::move_scenario_object(world, food, position((2, 0)))?;
    let mut route = vec![(0, 1), (-1, 1)];
    if open.contains(&(-2, 1)) {
        route.extend([(-2, 1), (-1, 1)]);
    }
    route.extend([(0, 1), (0, 2), (1, 2), (2, 2), (3, 2)]);
    if open.contains(&(4, 2)) {
        route.extend([(4, 2), (3, 2)]);
    }
    route.extend([(2, 2), (2, 1), (2, 0)]);
    let mut walls = Vec::new();
    for x in -3..=5 {
        for z in -1..=3 {
            if !open.contains(&(x, z)) {
                let entity = world.editor_spawn_object(alife_world::WorldEditorSpawnSpec {
                    label: format!("maze-wall-{x}-{z}"),
                    kind: alife_world::WorldObjectKind::Obstacle,
                    organism_id: None,
                    position: Vec3f::new(
                        origin.x + 100.0 + x as f32 * 4.0,
                        origin.y,
                        origin.z + 100.0 + z as f32 * 4.0,
                    ),
                    nutrition: 0.0,
                    hazard_pain: 0.0,
                    radius: 1.1,
                    token_id: None,
                })?;
                walls.push((entity, position((x, z))));
            }
        }
    }
    Ok(MazeLayout {
        walls,
        demonstration_route: route.into_iter().map(position).collect(),
    })
}
