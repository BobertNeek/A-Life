# Native live-reserve ingestion V1

`HeadlessWorld::apply_registered_live_bite_pair_v1` is an explicit world-only
research executor. It accepts two already selected, registered motor bundles.
The actor's Manipulation must be the existing Eat primitive targeting the exact
registered recipient. It does not choose a target or alter either bundle.
Ordinary Eat remains Food-only; no persistent rule is enabled.

At ordinary reachable, unobstructed contact, Eat removes at most 0.04 times its
embodied intensity from the recipient's mean organ reserve, bounded by actual
available reserve. The existing pro-rata debit preserves local reserve ratios.
The actor receives the removed mean amount as ordinary nutrition, spends 0.01
energy in effort, and tastes the contacted object's existing chemical sample.
The recipient receives the same bounded amount as ordinary damage. Inherited
resistance, chemistry, repair, and upkeep determine actual injury, pain, and
health consequences. There is no positive energy shortcut or authored reward.
The loss in summed organ reserves exceeds the maximum digestion gain; ordinary
clipping and metabolic costs can dissipate additional reserve.

The pair stages the entire world and runs both submitted bundles in actor-first
physical order. Contact therefore precedes the recipient's submitted move for
that interval. Each organism's biology advances once to `world_tick + 1`.
The whole-pair receipt includes the recipient's pre-debit biology, the measured
transfer, and both ordinary motor/biology receipts. Any error restores both
organisms and world state. The caller still owns the normal world boundary.
No pending effect or research configuration can escape this synchronous call.

Cold saves store the resulting ordinary reserve, injury, chemistry, taste, and
clock state. No save schema changes or hidden enabled rule need migration.
Reusing a same-interval pair rejects through ordinary registered tick authority.
After restore, ordinary Eat retains its original behavior.

## Scope and remaining boundary

This demonstrates **live reserve extraction**, not structural tissue ingestion,
autonomous hunting, or a kill-and-consume lifecycle. Dead and exhausted targets
provide no meal. Death still follows the existing lifecycle/archive behavior;
there is no carcass law. Existing Praise, Scold, Pet, Slap, Grab/release, and
NO_MANIPULATION are unchanged. No checkpoint, genome, weight, Adam state,
decoder, sensory feature, or symbolic prey label is modified.

The current CPU canonical host executes and learns from each organism's action
before executing the next organism's action. Its ordinary registered executor
has already advanced that organism's biology to the next tick. Calling this
paired executor behind that host would violate clock and private-state authority.
Future host integration must collect both selected motor bundles, own their
paired commit, and produce both corresponding activity/learning/private seals.
The V1 executor does not fabricate those seals or advance the world externally.
GPU/CPU-host integration and matched Contact20 brain trials remain separate work.

## Verification

The deterministic tests in
`crates/alife_world/src/headless/live_bite_v1/tests.rs` cover default Agent Eat
behavior; target/range/solid-contact controls; wrong identity, self, unregistered,
dead, and already advanced recipients; effector authority; finite extraction;
small/zero doses; recipient reserve/injury/pain/health; both rollback stages;
cold portable save continuation; and moving registered biological Agents.
The moving fixtures submit commands directly and prove executor capability only.

Run `cargo test -p alife_world --lib live_bite_v1 -- --nocapture` for the focused witness.
The crate suite and core boundary check exercise the unchanged ordinary routes.
These are native CPU world/biology tests, with no hosted-CI, GPU, or evolved-brain
acceptance claim.

The 2026-11-14 native run with Rust 1.100.0 passed all 11 focused tests. The
registered moving-Agent component fixture passed on seeds 91750, 91751, and
91752; it retained the recipient's submitted movement and advanced each biology
clock once. These commands were submitted by the harness, not chosen by a brain.

In the mature stationary component fixture (seed 91701, world tick 600), actual
mean reserve removal and received nutrition were both 0.04. Predator mean
energy changed from 0.50000000 to 0.50719833; recipient health changed from
1.00000000 to 0.98917335 and pain from 0.00000000 to 0.03200000. Ordinary chemistry,
damage resistance, metabolic effort, and upkeep produced these consequences.
Finite repeated extraction on three separate fixture seeds stayed below initial
donor reserves and admitted no further transfer after terminal exhaustion.

The complete `cargo test -p alife_world` run passed 698 tests with no failures;
three existing manual timing/soak checks remained ignored. `cargo clippy -p alife_world
--lib -- -D warnings`, crate formatting, `git diff --check`, and
`scripts/check_core_boundaries.sh` also passed in the fresh Cloud environment.
The branch was reconciled with main's source-only feedback diagnostic #352;
that change has no World-file overlap.
