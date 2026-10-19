# Local hands-on preparation and visible-target steering

Mode 2; independent R2 review. Base: PR98 merge `5d0a8c3f`.

## Authority and boundaries

Cassidy approved automatic legal pickup, automatic safe placement when switching
hands-on activities while protecting deliberate carrying, and local obstacle
avoidance. The latest clarification requires the destination to be actually
visible over, around, or through an obstacle. A hidden remembered destination
does not authorize an automatic corner detour. Learned intermediate movements
may restore visibility.

This implements world/body execution under AOA-WORLD-001/002/003,
AOA-SENSE-001/002, AOA-MOTOR-002/005, AOA-EMB-001/003, and AOA-TIME-001/003.
It does not amend the controlling v2.0 architecture.

Foundation weights, actor/value Adam, archived checkpoints, replay, seed ledgers,
and the continuity audit remain read-only. No training or learning evaluation.
Destinations, exploration, route knowledge, and persistence choices remain learned.
The subsequently approved narrow curriculum repairs add silent care controls,
EatReachableFood, Eat demonstrations without a Grab prerequisite, exact-target
nourishment acceptance, separate blocked-action reporting, and a care-first
next-run plan. They do not start training or settle broader qualification.

## Hands contract

- Eat and Play are hands-on activities. Look, Inspect, Approach, movement,
  orientation, speech, and inactivity never acquire objects implicitly.
- A mobile target is acquired through the same reach, effector, grip path,
  liveness, player/creature ownership, and single-grip capacity checks as Grab.
- Eat keeps its selected action identity: acquisition yields contact and zero
  nutrition; a later Eat consumes the same held target and supplies actual
  nourishment, taste, and ownership cleanup. No navigation occurs during Eat.
- Play acquires a mobile ball before activation. Acquisition is not play
  stimulation. A fixed activity toy is operated in place and cannot be carried.
- Explicit Grab acquisition records deliberate carrying; automatic activity
  acquisition records temporary activity possession. A different activity cannot
  silently discard an explicitly carried object. Explicit release remains a
  creature choice. Unknown legacy possession is protected as deliberate carry.
- When a different legal hands-on target needs the occupied activity grip,
  perform one measured placement transition first. Do not also activate,
  consume, or reward the requested new activity in that transition. A subsequent
  selected activity may acquire/use the target.
- Placement uses a bounded set of nearby positions, within reach, with support,
  solid path, object clearance, and legal terrain. Validate the new target before
  releasing the old one. No safe placement means failure without mutation.
- Physical transfer receipts expose the object, previous/new owner, positions,
  and transition. Staged pickup/placement must not manufacture nutrition, play
  stimulation, completed-action language labels, or repeat Grab credit.
- Grip provenance survives save/load and is included in canonical identity.

## Navigation contract

- Use fresh canonical grounded perception to establish a visible destination;
  derive its bearing from that observation, never from an unseen object's pose.
- Local safety sensing returns bounded nearest physical clearance rays, using
  the body's size and nearby geometry. The selector receives ranges and a chosen
  visible bearing, not the map, object kinds, food coordinates, or a route.
- Execute at most one ordinary bounded movement step per decision. Preserve
  terrain, collision, carry-clearance, effort, and measured displacement rules.
- No A*, flood fill, global waypoints, remembered hidden target tracking,
  wall-following route state, or semantic target selection.
- A hidden destination disables assistance. Explicit learned intermediate moves
  can still execute through ordinary physics, without automatic detours.
- Recheck visibility every decision. Loss of visibility pauses assistance on
  the next decision; each maneuver is just one physical step, with no queued
  remembered maneuver or authorization to steer around another corner.
- A small bounded non-progress allowance supports local side steps. Exhaustion
  returns Blocked until a meaningful new creature movement/goal choice, rather
  than silently continuing an unproductive detour. Persist this safety limit so
  a save/load cannot reset it into an unbounded solver. Persist the best observed
  distance and replenish only on meaningful net improvement beyond that reference;
  alternating outward/inward steps cannot refresh the budget.

## Implementation slices and checks

1. Shared hands preparation, transfer receipts, provenance, safe placement,
   legality/capacity, and updated Eat/Play/Grab execution.
2. Fresh visible-target gating, bounded body-clearance sensing and steering,
   measured movement, non-progress limit, and persistence/identity.
3. Source-bound documentation and focused CPU integration regressions.
4. Narrow approved care assessment/teacher repairs: silent versus prompted cue
   controls, reachable unheld Eat preparation/consumption, no acquisition bonus,
   exact-target nourishing success despite bumps, and care-first operator plan.

Acceptance tests cover legal ball/food acquisition; fixed toys; no acquisition
on observation/approach; real later nourishment; occupied activity transitions;
explicit carry protection; blocked/unsafe/owned/full-capacity cases with no
mutation; no duplicate possession or transition rewards; fresh save/load and
conservative legacy provenance; physical identity changes; visible low obstacles,
gaps and supported transparent barriers; opaque corners; visibility loss; hidden
geometry/target invariance; body and held-object clearance; bounded no-progress;
and registered factorized transaction rollback.

Run focused world tests first, affected app receipt/credit regressions where
needed, formatting, boundary/docs checks, independent R2 review, then exact-head
CI and eligible merge/post-merge checks. These are software regressions, not
learning evaluations or evidence of learned navigation competence.

## Bounded work and rollback

At most three diagnose/fix/check cycles per slice; broaden checks only for
changed code or an observed failure. No duplicate coding owner or learner job.
Review findings remain task data, not new authority. Stop on a genuine semantic
conflict with user intent or controlling architecture and report its evidence.

Rollback is a source revert. Preserve all original artifacts and retain their
existing qualification; no retrospective learning claim or artifact rewrite.
