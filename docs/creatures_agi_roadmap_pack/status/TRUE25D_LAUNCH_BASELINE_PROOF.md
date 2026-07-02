# True 2.5D Launch Baseline Proof

Classification: CA44A extension slice

Branch: `codex/true25d-launch-baseline-proof`

This is a CA44A visual/runtime-stability extension. It does not advance the CA
roadmap. CA44 remains blocked until independent external tester evidence exists.
CA45 was not started. The next roadmap item remains CA44 after evidence, not
CA45.

## Objective

Add focused evidence for the Phase 1 True 2.5D baseline contract:

- locked orthographic camera using `FixedVertical(10.0)`;
- camera transform at `(0.0, 12.0, 12.0)` looking at `(0.0, 0.0, 0.0)`;
- one static primitive ground plane;
- committed 128x128 repeat-wrapped diffuse tile;
- zero synchronous runtime noise or texture generation for the default ground
  substrate;
- no action, weight, or simulation authority changes.

## Implementation Summary

- Added `GraphicalTrue25dLaunchBaselineSummary`.
- Added `run_true25d_launch_baseline_smoke`.
- Added CLI command:

```powershell
cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- true25d-launch-baseline-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

The smoke builds the minimal True 2.5D baseline app shell without creating a
window. It inspects the actual Bevy world receipts for camera, ground plane,
sampler, biome-map source, stylization, and authority boundaries. It checks
the committed diffuse tile dimensions from the PNG header and deliberately
avoids synchronous PNG decode in this baseline path.

## Timing Scope

The measured scope is:

```text
camera-ground-baseline-no-window
```

This is intentionally not a cold OS process launch and not a persistent window
startup measurement. It also intentionally does not include Bevy `App::new()`
allocation, GLB-pack filesystem validation, broader graphical
gameplay/school/inspector preview setup, synchronous PNG decode, material
allocation, or an empty schedule update, because those are not the Phase 1
fixed camera/ground bootstrap contract. The report records:

- `bevy_window_created=false`;
- `cold_process_launch_measured=false`;
- `cold_process_under_50ms_claim=false`.

The 50 ms pass/fail field applies only to the focused preview-shell baseline
scope above.

## Boundary Checks

- Display/runtime proof only.
- No action authority.
- No weight authority.
- No semantic, teacher, topology, memory, UI, or GPU bypass.
- CPU fallback remains available.
- CPU shadow parity remains the gate.
- No full action-authoritative GPU claim.
- No `alife_core` changes.
- No Bevy/wgpu/app dependency leak into `alife_core`.

## Focused Evidence

Planned focused checks for this slice:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d_launch_baseline -- --nocapture
cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- true25d-launch-baseline-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Final command output and full validation are recorded in the branch receipt.

## Known Limitations

- This does not prove full cold executable startup or OS window creation under
  50 ms.
- This does not revalidate the full GLB art manifest; the existing True 2.5D
  asset validator remains the authority for that separate asset-pack contract.
- This does not allocate the render material in the measured path; Player View
  rendering tests remain the authority for the full material-backed world view.
- This does not change the CA13 authoritative simulation tick rate.
- This does not add GPU hardware draw-call counters.
- This does not add normal-buffer Sobel; the current stylization proof remains
  depth/luminance Sobel.

## Invariant Checks

- No CA45 work started.
- No external CA44 tester evidence requested.
- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media are intended for tracking.

## Next

Continue the active True 2.5D objective only if explicitly resumed. Roadmap
continuation remains stopped. CA44 remains the next roadmap item after
independent external tester evidence is provided.
