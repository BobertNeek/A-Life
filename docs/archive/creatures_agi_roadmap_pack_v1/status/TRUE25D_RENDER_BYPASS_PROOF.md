# True 2.5D Render Bypass Proof Addendum

Classification: CA44A extension addendum

Branch: `codex/true25d-render-bypass-proof-phase6`

This is a CA44A extension slice for the True 2.5D presentation goal. It does
not advance the CA roadmap. CA44 remains blocked until independent external
tester evidence exists. CA45 was not started. The next roadmap item remains
CA44 after evidence, not CA45.

## Objective

Tighten the Phase 3 viewport/render-bypass evidence so the default True 2.5D
Player View distinguishes:

- 60Hz presentation/render cadence;
- 20Hz authoritative CA13 simulation cadence;
- offscreen presentation entities with zero draw-call budget;
- headless procedural/data ledger updates that remain display/context only.

This addendum does not change simulation semantics or the CA13 scheduler.

## Implementation Summary

- Extended `GraphicalTrue25dViewportRenderBypass` receipts with:
  - render-extraction bypass flag;
  - presentation draw-call budget;
  - offscreen animation-update budget.
- Extended `GraphicalTrue25dRenderBypassSummaryResource` with:
  - `presentation_headless_tick_hz=60`;
  - `authoritative_sim_tick_hz=20`;
  - zero offscreen presentation draw budget;
  - zero offscreen animation-update budget;
  - explicit scheduler-unchanged receipt.
- Updated focused Bevy app-shell tests to assert:
  - offscreen True 2.5D entities are hidden;
  - offscreen entities carry zero presentation draw budget;
  - offscreen entities carry zero animation-update budget;
  - visible entities remain visible and budgeted;
  - the authoritative scheduler remains the existing 20Hz CA13 scheduler.

## Boundary

The addendum is a presentation/render-bypass proof. It is not a GPU draw-call
profiler and does not claim hardware counter evidence.

The 60Hz field is presentation/headless-render cadence only. The authoritative
simulation tick remains the existing CA13 20Hz scheduler. Changing the
authoritative simulation cadence would be a separate reviewed scheduler plan.

## Focused Evidence

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d_viewport_render_bypass -- --nocapture
```

Expected result: PASS.

## Invariant Checks

- `alife_core` unchanged.
- No Bevy, wgpu, renderer, app, or model-runtime dependency leaked into
  `alife_core`.
- No simulation authority changed.
- No action path changed.
- No CPU fallback weakening.
- No CPU shadow parity weakening.
- No full action-authoritative GPU claim.
- No semantic, teacher, topology, memory, UI, or GPU bypass.
- No active bulk neural readback.
- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media are intended for tracking.

## Known Limitations

- The receipt proves the app's presentation contract and Bevy visibility state;
  it does not read hardware GPU draw-call counters.
- Fog of war remains presentation-side and not an authoritative sensory
  visibility system.
- The authoritative fixed simulation tick remains 20Hz by design.
- This does not unblock CA44. Independent external tester evidence is still
  required before CA44 can move.

## Next

Roadmap continuation remains stopped. The next roadmap item remains CA44 after
independent external tester evidence is provided. Do not start CA45 from this
document.
