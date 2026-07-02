# True 2.5D Headless Chunk Continuity Proof

Classification: CA44A extension slice

Branch: `codex/true25d-headless-chunk-continuity-proof`

This is a CA44A extension for the active True 2.5D presentation/runtime goal.
It does not advance the CA roadmap. CA44 remains blocked until independent
external tester evidence exists. CA45 was not started. The next roadmap item
remains CA44 after evidence, not CA45.

## Objective

Tighten Phase 3 evidence by proving the seeded procedural chunk field can
stream around creature anchors without requiring rendering while the live brain
loop continues ticking and sealing patches.

This slice does not change simulation semantics or scheduler cadence.

## Implementation Summary

- Added `True25dHeadlessChunkContinuitySummary`.
- Added `run_true25d_headless_chunk_continuity_smoke`.
- Added CLI command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- true25d-headless-continuity-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

- Added focused regression test:
  `true25d_headless_chunk_continuity_smoke_keeps_brain_ticks_running_without_render_authority`.
- The smoke composes:
  - seeded procedural world travel/chunk streaming evidence;
  - CA44A live tick stability evidence.

## Evidence Captured

Focused command result:

```text
schema=alife.ca44a.true25d_headless_chunk_continuity.v1
seed=4242
stable=1
steps=6
unique_chunks=138
max_active=25
no_render=true
zero_draw_budget=0
ticks=128/128
mind_delta=128
world_delta=128
sealed=128
packed=128
first_invalid=None
auth_hz=20
presentation_hz=60
goal_hz=60
claim_60hz_sim=false
action_authority=false
weight_authority=false
cpu_shadow=true
```

## Boundary And Scheduler Note

The active goal requests 60Hz headless continuation. Current A-Life CA13
authority remains:

- 60Hz presentation/render cadence;
- 20Hz authoritative simulation/headless brain tick cadence.

This slice preserves that invariant and records `claim_60hz_sim=false`.
Changing the authoritative simulation scheduler to 60Hz would require a
separate reviewed scheduler plan. This proof therefore strengthens Phase 3
headless continuity evidence but does not close the 60Hz simulation-cadence gap.

## Invariant Checks

- `alife_core` unchanged.
- No Bevy, wgpu, renderer, app, or model-runtime dependency leaked into
  `alife_core`.
- Procedural chunks remain CPU/data context.
- Procedural chunks cannot emit actions.
- Procedural chunks cannot rewrite weights.
- The new smoke does not change action authority.
- CPU fallback unchanged.
- CPU shadow parity unchanged.
- No full action-authoritative GPU claim.
- No semantic, teacher, topology, memory, UI, or GPU bypass.
- No active bulk neural readback.
- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media are intended for tracking.

## Focused Evidence

```powershell
cargo test -p alife_game_app --test app_shell true25d_headless_chunk_continuity -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- true25d-headless-continuity-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Results:

- Focused test: PASS.
- CLI smoke: PASS.

## Known Limitations

- This is still not hardware GPU draw-call counter evidence. It proves the
  app-level zero offscreen presentation draw-budget contract and headless
  generation/tick continuity.
- The authoritative fixed simulation tick remains 20Hz.
- This does not unblock CA44. Independent external tester evidence is still
  required before CA44 can move.

## Next

Continue the active True 2.5D goal with another CA44A extension slice only if
explicitly instructed by the goal runner. Do not resume CA roadmap execution
from this document.
