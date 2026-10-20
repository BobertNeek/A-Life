//! Offline simulator rehearsal accounting. No observations, actions or rewards
//! are synthesized here; a host must execute each scheduled attempt.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RehearsalSource {
    Fresh,
    Success,
    DifficultFailure,
    Holdout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SceneVariation {
    Original,
    FoodLeft,
    FoodRight,
    FoodFarther,
    ObstacleLeft,
    ObstacleRight,
}
impl SceneVariation {
    pub fn for_attempt(attempt: usize) -> Self {
        [
            Self::FoodLeft,
            Self::FoodRight,
            Self::FoodFarther,
            Self::ObstacleLeft,
            Self::ObstacleRight,
        ][attempt % 5]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RehearsalLineage {
    pub rollout: u64,
    pub parent_rollout: Option<u64>,
    pub parent_snapshot: Option<u64>,
    pub world_seed: u64,
    pub sampling_seed: u64,
    pub policy_version: u64,
    pub foundation_digest: [u8; 32],
    pub source: RehearsalSource,
    pub variation: SceneVariation,
    /// A new stochastic rollout does not establish historical scene freshness.
    pub historically_fresh: bool,
}
impl RehearsalLineage {
    pub fn validate(&self) -> Result<(), &'static str> {
        let retry = matches!(
            self.source,
            RehearsalSource::Success | RehearsalSource::DifficultFailure
        );
        if self.world_seed == 0
            || self.sampling_seed == 0
            || self.foundation_digest == [0; 32]
            || retry != self.parent_snapshot.is_some()
            || retry != self.parent_rollout.is_some()
            || self
                .parent_rollout
                .is_some_and(|parent| parent >= self.rollout)
            || (retry && self.historically_fresh)
            || (!retry && self.variation != SceneVariation::Original)
        {
            return Err("inconsistent simulator rehearsal lineage");
        }
        Ok(())
    }
    pub fn require_policy(&self, version: u64, digest: [u8; 32]) -> Result<(), &'static str> {
        self.validate()?;
        if self.policy_version != version || self.foundation_digest != digest {
            return Err("snapshot policy changed; recollect with the current policy");
        }
        Ok(())
    }
}

/// Bounded stratified FIFO. One slot per (root scene, variation) avoids a common
/// easy scene crowding out other successful examples. Failures have a separate pool.
#[derive(Debug)]
pub struct DiverseRehearsalPool<T> {
    capacity: usize,
    entries: Vec<((u64, SceneVariation), T)>,
    cursor: usize,
}
impl<T> DiverseRehearsalPool<T> {
    pub fn new(capacity: usize) -> Result<Self, &'static str> {
        if !(1..=32).contains(&capacity) {
            return Err("pool capacity must be in 1..=32");
        }
        Ok(Self {
            capacity,
            entries: Vec::new(),
            cursor: 0,
        })
    }
    pub fn retain(&mut self, seed: u64, variation: SceneVariation, snapshot: T) {
        let key = (seed, variation);
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.0 == key) {
            entry.1 = snapshot;
        } else {
            if self.entries.len() == self.capacity {
                self.entries.remove(0);
            }
            self.entries.push((key, snapshot));
        }
    }
    pub fn next_example(&mut self) -> Option<&T> {
        if self.entries.is_empty() {
            return None;
        }
        let index = self.cursor % self.entries.len();
        self.cursor = self.cursor.wrapping_add(1);
        Some(&self.entries[index].1)
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A guaranteed fresh-scene reservation and a difficult-failure reservation.
/// Empty pools fall back to fresh scenes; pool availability never blocks work.
pub fn rehearsal_source(attempt: usize, control_fresh: bool) -> RehearsalSource {
    if control_fresh || attempt.is_multiple_of(4) {
        RehearsalSource::Fresh
    } else if attempt % 4 == 3 {
        RehearsalSource::DifficultFailure
    } else {
        RehearsalSource::Success
    }
}

/// Train and holdout seeds are disjoint within the run. This is deliberately
/// weaker than historically unseen admission, which needs the campaign ledger.
pub fn validate_scene_partition(train: &[u64], holdout: &[u64]) -> Result<(), &'static str> {
    let seen: BTreeSet<_> = train.iter().copied().collect();
    let held: BTreeSet<_> = holdout.iter().copied().collect();
    if train.is_empty()
        || holdout.is_empty()
        || seen.len() != train.len()
        || held.len() != holdout.len()
        || seen.contains(&0)
        || held.contains(&0)
        || !seen.is_disjoint(&held)
    {
        return Err("training and holdout scenes must be unique and disjoint");
    }
    Ok(())
}

/// No burn-in or bootstrap row receives an actor/value loss. A branch is one
/// independent sequence; callers may not join its GAE tail to another attempt.
pub fn rehearsal_loss_range(
    ticks: usize,
    burn_in: usize,
    terminal: bool,
) -> Result<std::ops::Range<usize>, &'static str> {
    let end = ticks
        .checked_sub(usize::from(!terminal))
        .ok_or("missing bootstrap")?;
    if ticks > crate::MAX_TRAINING_SEQUENCE_TICKS || burn_in >= end {
        return Err("attempt needs burn-in, train rows and a real bootstrap or terminal boundary");
    }
    Ok(burn_in..end)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lineage_never_promotes_historical_retry_or_stale_policy() {
        let mut line = RehearsalLineage {
            rollout: 2,
            parent_rollout: Some(1),
            parent_snapshot: Some(4),
            world_seed: 5,
            sampling_seed: 6,
            policy_version: 7400,
            foundation_digest: [1; 32],
            source: RehearsalSource::Success,
            variation: SceneVariation::FoodLeft,
            historically_fresh: false,
        };
        line.require_policy(7400, [1; 32]).unwrap();
        assert!(line.require_policy(7401, [1; 32]).is_err());
        assert!(line.require_policy(7400, [2; 32]).is_err());
        line.historically_fresh = true;
        assert!(line.validate().is_err());
        line.historically_fresh = false;
        line.parent_rollout = Some(2);
        assert!(line.validate().is_err());
    }
    #[test]
    fn capacity_deduplication_and_round_robin_preserve_diversity() {
        let mut pool = DiverseRehearsalPool::new(2).unwrap();
        pool.retain(1, SceneVariation::FoodLeft, 10);
        pool.retain(2, SceneVariation::FoodRight, 20);
        pool.retain(1, SceneVariation::FoodLeft, 11);
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.next_example(), Some(&11));
        assert_eq!(pool.next_example(), Some(&20));
        pool.retain(3, SceneVariation::ObstacleLeft, 30);
        assert_eq!(pool.len(), 2);
        assert!(!pool.entries.iter().any(|entry| entry.1 == 11));
        assert!(DiverseRehearsalPool::<()>::new(0).is_err());
    }
    #[test]
    fn fresh_and_failure_reservations_and_seed_partition_are_explicit() {
        assert_eq!(rehearsal_source(4, false), RehearsalSource::Fresh);
        assert_eq!(
            rehearsal_source(3, false),
            RehearsalSource::DifficultFailure
        );
        assert_eq!(rehearsal_source(1, false), RehearsalSource::Success);
        assert_eq!(rehearsal_source(1, true), RehearsalSource::Fresh);
        validate_scene_partition(&[1, 2], &[3, 4]).unwrap();
        assert!(validate_scene_partition(&[1, 2], &[2, 3]).is_err());
        assert!(validate_scene_partition(&[1, 1], &[3]).is_err());
        assert!(validate_scene_partition(&[1], &[0]).is_err());
    }
    #[test]
    fn sequence_masks_exclude_burn_in_and_bootstrap_at_boundaries() {
        assert_eq!(rehearsal_loss_range(8, 2, false).unwrap(), 2..7);
        assert_eq!(rehearsal_loss_range(8, 2, true).unwrap(), 2..8);
        assert!(rehearsal_loss_range(3, 2, false).is_err());
        assert!(rehearsal_loss_range(0, 0, false).is_err());
        assert!(rehearsal_loss_range(crate::MAX_TRAINING_SEQUENCE_TICKS + 1, 0, true).is_err());
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedSceneVariation {
    pub variation: SceneVariation,
    pub target: alife_core::WorldEntityId,
    pub food_before: alife_core::Vec3f,
    pub food_after: alife_core::Vec3f,
    pub obstacle: Option<alife_core::WorldEntityId>,
}

/// Branch the *complete* authoritative world. The original is immutable even if
/// validation fails. Only scenery changes: organism biology, identities, hands,
/// tracking, ecology, speech and clocks are inherited through HeadlessWorld::clone.
/// Neural/private-host checkpoint restoration remains the caller's responsibility.
pub fn perturb_rehearsal_world(
    world: &alife_world::HeadlessWorld,
    organism: alife_core::OrganismId,
    target: alife_core::WorldEntityId,
    variation: SceneVariation,
) -> Result<(alife_world::HeadlessWorld, AppliedSceneVariation), &'static str> {
    use alife_core::Vec3f;
    use alife_world::WorldObjectKind;
    let food = world.entity(target).ok_or("missing food target")?;
    let record = world
        .organism_registry()
        .get(organism)
        .ok_or("missing organism biology")?;
    let agent = world
        .entity(record.world_entity_id())
        .ok_or("missing learner")?;
    record
        .validate_contract()
        .map_err(|_| "invalid organism state")?;
    if !record.lifecycle().is_alive()
        || food.kind != WorldObjectKind::Food
        || food.consumed
        || food.carried_by.is_some()
        || world.player_held_object() == Some(target)
    {
        return Err("variation requires a live learner and unconsumed unheld food");
    }
    let dx = food.position.x - agent.position.x;
    let dz = food.position.z - agent.position.z;
    let norm = dx.hypot(dz);
    if !norm.is_finite() || norm < 0.1 {
        return Err("food direction is too short to vary safely");
    }
    let forward = (dx / norm, dz / norm);
    let side = (-forward.1, forward.0);
    let mut next = world.clone();
    let mut after = food.position;
    let mut obstacle = None;
    match variation {
        SceneVariation::Original => {}
        SceneVariation::FoodLeft | SceneVariation::FoodRight | SceneVariation::FoodFarther => {
            let (lateral, farther) = match variation {
                SceneVariation::FoodLeft => (agent.radius + food.radius + 0.35, 0.0),
                SceneVariation::FoodRight => (-(agent.radius + food.radius + 0.35), 0.0),
                _ => (0.0, 1.5),
            };
            after = Vec3f::new(
                after.x + lateral * side.0 + farther * forward.0,
                after.y,
                after.z + lateral * side.1 + farther * forward.1,
            );
            validate_placement(world, target, after, food.radius)?;
            next.editor_move_object(target, after)
                .map_err(|_| "food move rejected")?;
            after = next.entity(target).ok_or("food disappeared")?.position;
            let mut physical = food.grounded_physical;
            physical.velocity = Vec3f::ZERO;
            next.set_grounded_physical_properties(target, physical)
                .map_err(|_| "food geometry rejected")?;
        }
        SceneVariation::ObstacleLeft | SceneVariation::ObstacleRight => {
            let sign = if variation == SceneVariation::ObstacleLeft {
                1.0
            } else {
                -1.0
            };
            let position = Vec3f::new(
                agent.position.x + dx * 0.5 + side.0 * sign * (agent.radius + 0.5),
                agent.position.y,
                agent.position.z + dz * 0.5 + side.1 * sign * (agent.radius + 0.5),
            );
            validate_placement(world, target, position, 0.25)?;
            // Also exclude the target: an obstacle may not enclose the food.
            if (position.x - food.position.x).hypot(position.z - food.position.z)
                <= 0.25 + food.radius + 0.05
            {
                return Err("obstacle overlaps food");
            }
            let label = format!(
                "rehearsal-obstacle-{}-{}",
                world.tick().raw(),
                world.object_count()
            );
            obstacle = Some(
                next.editor_spawn_object(alife_world::WorldEditorSpawnSpec {
                    label,
                    kind: WorldObjectKind::Obstacle,
                    organism_id: None,
                    position,
                    nutrition: 0.0,
                    hazard_pain: 0.0,
                    radius: 0.25,
                    token_id: None,
                })
                .map_err(|_| "obstacle spawn rejected")?,
            );
        }
    }
    Ok((
        next,
        AppliedSceneVariation {
            variation,
            target,
            food_before: food.position,
            food_after: after,
            obstacle,
        },
    ))
}

fn validate_placement(
    world: &alife_world::HeadlessWorld,
    ignored: alife_core::WorldEntityId,
    position: alife_core::Vec3f,
    radius: f32,
) -> Result<(), &'static str> {
    use alife_world::WorldObjectKind;
    position.validate().map_err(|_| "nonfinite position")?;
    if !radius.is_finite() || radius <= 0.0 || position.x.abs() > 512.0 || position.z.abs() > 512.0
    {
        return Err("placement outside rehearsal bounds");
    }
    if let Some(terrain) = world.terrain() {
        for (x, z) in [
            (position.x, position.z),
            (position.x - radius, position.z),
            (position.x + radius, position.z),
            (position.x, position.z - radius),
            (position.x, position.z + radius),
        ] {
            if !terrain.walkable(x, z) {
                return Err("placement intersects terrain or its boundary");
            }
        }
    }
    for object in world.object_snapshots() {
        if object.id != ignored
            && !object.consumed
            && matches!(
                object.kind,
                WorldObjectKind::Agent | WorldObjectKind::Obstacle | WorldObjectKind::Hazard
            )
            && (position.x - object.position.x).hypot(position.z - object.position.z)
                <= radius + object.radius + 0.05
        {
            return Err("placement intersects body or solid scenery");
        }
    }
    Ok(())
}

#[cfg(test)]
mod world_tests {
    use super::*;
    use alife_core::{BrainScaleTier, FoundationWeightAsset, SensorProfile, Vec3f};
    use alife_world::{
        CanonicalNewGameConfig, HeadlessWorld, WorldEditorSpawnSpec, WorldObjectKind,
    };
    fn scene() -> (
        HeadlessWorld,
        alife_core::OrganismId,
        alife_core::WorldEntityId,
    ) {
        let mut config = CanonicalNewGameConfig::phase3(77_101, 1).unwrap();
        config.brain_class = BrainScaleTier::Nano512;
        config.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
        let asset = FoundationWeightAsset::builtin_nano512_v1(config.sensor_profile).unwrap();
        let mut world = alife_world::create_canonical_new_game(&config, &asset)
            .unwrap()
            .world;
        let organism = world
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .organism_id();
        let agent = world
            .entity(
                world
                    .organism_registry()
                    .get(organism)
                    .unwrap()
                    .world_entity_id(),
            )
            .unwrap()
            .position;
        let target = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "rehearsal-test-food".into(),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position: Vec3f::new(agent.x + 8.0, agent.y, agent.z),
                nutrition: 0.5,
                hazard_pain: 0.0,
                radius: 0.1,
                token_id: None,
            })
            .unwrap();
        (world, organism, target)
    }
    #[test]
    fn variations_branch_real_world_and_preserve_private_biology_and_parent() {
        let (world, organism, target) = scene();
        let parent = world.canonical_signature_digest().unwrap();
        let biology = serde_json::to_vec(world.organism_registry().get(organism).unwrap()).unwrap();
        for variant in [
            SceneVariation::FoodLeft,
            SceneVariation::FoodRight,
            SceneVariation::FoodFarther,
            SceneVariation::ObstacleLeft,
            SceneVariation::ObstacleRight,
        ] {
            let (mut child, applied) =
                perturb_rehearsal_world(&world, organism, target, variant).unwrap();
            assert_ne!(child.canonical_signature_digest().unwrap(), parent);
            assert_eq!(world.canonical_signature_digest().unwrap(), parent);
            assert_eq!(child.tick(), world.tick());
            assert_eq!(
                serde_json::to_vec(child.organism_registry().get(organism).unwrap()).unwrap(),
                biology
            );
            assert_eq!(applied.variation, variant);
            // Actual canonical metabolism advances in each branch; no fabricated outcomes.
            child.try_advance_tick().unwrap();
            assert_eq!(child.tick().raw(), world.tick().raw() + 1);
            assert_ne!(
                serde_json::to_vec(child.organism_registry().get(organism).unwrap()).unwrap(),
                biology
            );
        }
    }
    #[test]
    fn actual_near_grab_geometry_supports_all_five_variations() {
        let (mut world, organism, original) = scene();
        let agent = world
            .entity(
                world
                    .organism_registry()
                    .get(organism)
                    .unwrap()
                    .world_entity_id(),
            )
            .unwrap()
            .clone();
        world.editor_remove_object(original).unwrap();
        let target = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "near-food".into(),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position: Vec3f::new(agent.position.x + 0.5, agent.position.y, agent.position.z),
                nutrition: 0.5,
                hazard_pain: 0.0,
                radius: 0.45,
                token_id: None,
            })
            .unwrap();
        for variant in [
            SceneVariation::FoodLeft,
            SceneVariation::FoodRight,
            SceneVariation::FoodFarther,
            SceneVariation::ObstacleLeft,
            SceneVariation::ObstacleRight,
        ] {
            let (child, _) = perturb_rehearsal_world(&world, organism, target, variant).unwrap();
            assert_ne!(
                child.canonical_signature_digest().unwrap(),
                world.canonical_signature_digest().unwrap()
            );
        }
    }
    #[test]
    fn invalid_placements_and_nonfood_fail_without_mutating_parent() {
        let (mut world, organism, target) = scene();
        let food = world.entity(target).unwrap().position;
        world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "blocking-test".into(),
                kind: WorldObjectKind::Obstacle,
                organism_id: None,
                position: Vec3f::new(food.x, food.y, food.z + 1.0),
                nutrition: 0.0,
                hazard_pain: 0.0,
                radius: 0.5,
                token_id: None,
            })
            .unwrap();
        let digest = world.canonical_signature_digest().unwrap();
        assert!(
            perturb_rehearsal_world(&world, organism, target, SceneVariation::FoodLeft).is_err()
        );
        let agent = world
            .organism_registry()
            .get(organism)
            .unwrap()
            .world_entity_id();
        assert!(
            perturb_rehearsal_world(&world, organism, agent, SceneVariation::FoodRight).is_err()
        );
        assert_eq!(world.canonical_signature_digest().unwrap(), digest);
        assert!(validate_placement(&world, target, Vec3f::new(f32::NAN, 0.0, 0.0), 0.1).is_err());
        assert!(validate_placement(&world, target, Vec3f::new(513.0, 0.0, 0.0), 0.1).is_err());
    }
}

/// Timing for an action followed by possibly later canonical terminal biology.
/// The terminal tail is discounted from the action outcome, while GAE sees the
/// complete decision-to-death interval, matching ordinary collection semantics.
pub fn terminal_credit_timing(
    decision: u64,
    outcome: u64,
    terminal: u64,
    half_life: f32,
) -> Result<(f32, f32), &'static str> {
    if outcome <= decision || terminal < outcome || !half_life.is_finite() || half_life <= 0.0 {
        return Err("invalid terminal credit timing");
    }
    let elapsed = (terminal - decision) as f32 / 20.0;
    let gap_seconds = (terminal - outcome) as f64 / 20.0;
    let discount = (-std::f64::consts::LN_2 * gap_seconds / f64::from(half_life)).exp() as f32;
    Ok((elapsed, discount))
}
#[cfg(test)]
mod timing_tests {
    use super::*;
    #[test]
    fn delayed_death_changes_credit_discount_and_elapsed_time() {
        assert_eq!(
            terminal_credit_timing(10, 11, 11, 1.0).unwrap(),
            (0.05, 1.0)
        );
        let (elapsed, discount) = terminal_credit_timing(10, 11, 31, 1.0).unwrap();
        assert!((elapsed - 1.05).abs() < 1e-6);
        assert!((discount - 0.5).abs() < 1e-6);
        for (decision, outcome, terminal, half) in [
            (10, 10, 11, 1.0),
            (10, 11, 10, 1.0),
            (10, 11, 12, 0.0),
            (10, 11, 12, f32::NAN),
        ] {
            assert!(terminal_credit_timing(decision, outcome, terminal, half).is_err());
        }
    }
}
