# Food transform guard audit — 2026-10-19

Decision: leave the runtime unchanged. Against main
`a2b30e14d9d149b4af8a97e2b5821cfe21815590`, exact bit guards can avoid some
CPU bookkeeping and matrix arithmetic, but ordinary unchanged food meshes
already avoid downstream change-driven extraction. The measured small-population
saving does not justify a runtime patch. This is a fresh investigation, not a
reconstruction of the unavailable preserved patch.

## Work actually avoided

`live_food_projection.rs:61` assigns retained care-object translation on changed
presentation frames; `:162–163` assigns X/Z every rendered update. Merely obtaining
Bevy's `Mut<Transform>` does not mark it changed; mutable dereferencing does.
No application consumer requiring unconditional care-object change ticks was
found. The existing invocation-local lazy carrier-root scan remains useful and
separate from these guards.

The root Bevy package is 0.18.0, but `Cargo.lock` resolves transform, ECS and PBR
to **0.18.1**, with `std` enabled in the production frontend:

- [Mutable dereferencing](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_ecs/src/change_detection/traits.rs#L420)
  marks even identical assignments changed.
- [Transform propagation](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_transform/src/systems.rs#L109)
  marks dirty trees and rewrites root globals. Guards can skip those operations,
  plus child matrix calculation/comparison, while static-tree optimization is
  active. The queries still scan. Above the default 30% dynamic threshold,
  hierarchy propagation runs regardless of these guards.
- Crucially, [child propagation already uses `set_if_neq`](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_transform/src/systems.rs#L566).
  Identical finite mesh-child globals retain their ticks. Thus the
  [GPU extraction filter](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_pbr/src/render/mesh.rs#L1491)
  already skips those unchanged meshes when its other inputs are unchanged.
  [CPU mesh extraction](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_pbr/src/render/mesh.rs#L1346)
  and visibility checks run independently; guards do not eliminate them.

Guards also retain snapshot/object lookups, terrain-center calculation and
carrier-offset construction. On a new snapshot, sync can restore canonical X/Z
before alignment reapplies a nonzero carrier offset. Both intermediate changes
must retain their ticks even if the final pose matches the previous frame.
Any future guard must compare final float **bits**, preserving signed-zero
assignments and NaN payload changes, without clearing another writer's tick.

## Bounded CPU probe

A throwaway Rust release probe used Bevy ECS/transform 0.18.1, one core, a
single-thread schedule/task pool, one transform child per care root (matching the care mesh hierarchy),
and three static standalone transforms per root. It compared unconditional X/Z
writes with exact bit guards followed by Bevy's real transform systems. Seven
alternating-order samples were warmed up; medians are microseconds per iteration:

| Stationary care roots | Unconditional | Guarded |
| ---: | ---: | ---: |
| 3 | 0.328 | 0.209 |
| 16 | 0.985 | 0.425 |
| 256 | 13.413 | 4.200 |
| 4,096 | 210.452 | 65.522 |

With static-tree optimization disabled, three stationary roots measured
0.220 versus 0.222 microseconds. The probe observed root global writes fall
from N to zero only with the optimization active; unchanged finite child global
writes were already zero in both variants. This excludes app lookup/terrain
costs, rendering and GPU execution; it is **not a game-frame or FPS measurement**.
Rust/Cargo 1.98.1 ran on the saved cloud Xeon Platinum 8573C environment.
The normal new-game ecology seeds three care objects (`alife_world/src/new_game.rs:304–317`);
players may add more. Revisit only if actual gameplay profiling at a larger
population identifies this CPU work as material.

The provisional patch passed all 16 food-projection CPU tests (13 existing and
three experimental), including bitwise legacy parity, signed zero, NaN payloads,
external/intermediate change ticks and real Bevy root/child propagation. All 30
synthetic motion/static-mode cases matched the source-derived change counts.
The guards and experimental tests were then removed. Only this note remains;
no biology, learning, curriculum, training/evaluation, art, prepared desktop
executable or runtime source is changed.

Final formatting, static core boundaries, whitespace and documentation checks
passed (77 documentation assertions). Reproducer, results, rejected patch and
experimental test log remain in the saved cloud environment under
`target/artifacts/food-transform-guard-audit-20261019/` (untracked investigation
evidence). No game performance measurement or runtime acceptance gate is claimed.
