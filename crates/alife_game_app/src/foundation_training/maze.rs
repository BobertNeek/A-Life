//! Layout-aware planning stays private; the demonstrator moves physically.
use alife_core::{ActionCandidate, CandidateActionFamily, PerceptionFrame, Vec3f, WorldEntityId};
use alife_world::HeadlessActionIds as Id;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
type Cell = (i32, i32);
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
        while self
            .route
            .get(self.next)
            .is_some_and(|t| (position.x - t.x).hypot(position.z - t.z) <= 0.22)
        {
            self.next += 1;
        }
        if let Some(t) = self.route.get(self.next) {
            let heading = 2.0
                * frame
                    .body()
                    .pose
                    .rotation
                    .y
                    .atan2(frame.body().pose.rotation.w);
            let error = ((t.z - position.z).atan2(t.x - position.x) - heading
                + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
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
fn neighbors((x, z): Cell) -> [Cell; 4] {
    [(x + 1, z), (x - 1, z), (x, z + 1), (x, z - 1)]
}
fn path(open: &BTreeSet<Cell>, start: Cell, goal: Cell) -> Vec<Cell> {
    let mut parent = BTreeMap::from([(start, start)]);
    let mut queue = VecDeque::from([start]);
    while let Some(cell) = queue.pop_front() {
        if cell == goal {
            break;
        }
        for next in neighbors(cell) {
            if open.contains(&next) && !parent.contains_key(&next) {
                parent.insert(next, cell);
                queue.push_back(next);
            }
        }
    }
    let mut result = vec![goal];
    while *result.last().unwrap() != start {
        result.push(parent[result.last().unwrap()]);
    }
    result.reverse();
    result
}
fn layout(seed: u64, rooms: i32) -> (BTreeSet<Cell>, Vec<Cell>) {
    // Random spanning trees vary junction connectivity. Even cells are rooms.
    let mut random = seed ^ 0x6d617a65;
    let mut visited = BTreeSet::from([(0, 0)]);
    let mut open = visited.clone();
    let mut frontier = vec![(0, 0)];
    while !frontier.is_empty() {
        random = random
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let index = if seed & 1 == 0 {
            frontier.len() - 1
        } else {
            (random as usize) % frontier.len()
        };
        let cell = frontier[index];
        let choices = neighbors((cell.0 / 2, cell.1 / 2))
            .into_iter()
            .map(|(x, z)| (2 * x, 2 * z))
            .filter(|&(x, z)| {
                x >= 0 && z >= 0 && x < 2 * rooms && z < 2 * rooms && !visited.contains(&(x, z))
            })
            .collect::<Vec<_>>();
        if choices.is_empty() {
            frontier.remove(index);
            continue;
        }
        let next = choices[((random >> 32) as usize) % choices.len()];
        visited.insert(next);
        open.insert(next);
        open.insert(((cell.0 + next.0) / 2, (cell.1 + next.1) / 2));
        frontier.push(next);
    }
    let goal = (2, 0); // Food is initially within sight even when its route is long.
    let mut leaves = visited
        .iter()
        .copied()
        .filter(|c| {
            *c != (0, 0)
                && *c != goal
                && neighbors(*c).iter().filter(|n| open.contains(n)).count() == 1
        })
        .collect::<Vec<_>>();
    leaves.sort_by_key(|&(x, z)| super::scenario_random(seed, (x + z * 17 + 41) as u64).to_bits());
    let mut route = Vec::new();
    for leaf in leaves.into_iter().take(2) {
        route.extend(path(&open, (0, 0), leaf).into_iter().skip(1));
        route.extend(path(&open, leaf, (0, 0)).into_iter().skip(1));
    }
    route.extend(path(&open, (0, 0), goal).into_iter().skip(1));
    (open, route)
}
pub(super) fn configure(
    world: &mut alife_world::HeadlessWorld,
    seed: u64,
    origin: Vec3f,
    food: WorldEntityId,
) -> Result<MazeLayout, Box<dyn std::error::Error>> {
    let stage = std::env::var("ALIFE_TRAINING_STAGE")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    let rooms = if stage >= 1 { 3 } else { 2 };
    let (open, route) = layout(seed, rooms);
    let axis = (super::scenario_random(seed, 21) - 0.5) * 120.0_f32.to_radians();
    let cell = 2.6 + 0.4 * super::scenario_random(seed, 22);
    let side = if seed & 8 == 0 { 1.0 } else { -1.0 };
    let position = |(x, z): Cell| {
        Vec3f::new(
            origin.x + cell * (x as f32 * axis.cos() - z as f32 * side * axis.sin()),
            origin.y,
            origin.z + cell * (x as f32 * axis.sin() + z as f32 * side * axis.cos()),
        )
    };
    if route.is_empty()
        || route
            .windows(2)
            .any(|p| (p[0].0 - p[1].0).abs() + (p[0].1 - p[1].1).abs() != 1)
    {
        return Err("disconnected maze route".into());
    }
    super::move_scenario_object(world, food, position((2, 0)))?;
    let mut walls = Vec::new();
    for x in -1..2 * rooms {
        for z in -1..2 * rooms {
            if !open.contains(&(x, z)) {
                let id = world.editor_spawn_object(alife_world::WorldEditorSpawnSpec {
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
                    radius: cell * 0.49,
                    token_id: None,
                })?;
                walls.push((id, position((x, z))));
            }
        }
    }
    Ok(MazeLayout {
        walls,
        demonstration_route: route.into_iter().map(position).collect(),
    })
}
#[cfg(test)]
mod tests {
    #[test]
    fn layouts_change_connectivity_and_keep_routes_connected() {
        let mut signatures = std::collections::BTreeSet::new();
        for seed in 0..16 {
            let (open, route) = super::layout(seed, 3);
            assert_eq!(route.last(), Some(&(2, 0)));
            assert!(route.iter().all(|c| open.contains(c)));
            assert!(route
                .windows(2)
                .all(|p| (p[0].0 - p[1].0).abs() + (p[0].1 - p[1].1).abs() == 1));
            signatures.insert(open);
        }
        assert!(
            signatures.len() > 8,
            "rotations cannot satisfy topology diversity"
        );
    }
}
