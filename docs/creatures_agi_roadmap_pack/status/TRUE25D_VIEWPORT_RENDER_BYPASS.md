# True 2.5D Viewport Render Bypass

Plan context: post-CA44A True 2.5D runtime pipeline hardening
Branch: `codex/true25d-render-bypass-phase3`

## Objective

Add explicit evidence that the default True 2.5D Player View can keep
procedural world chunks as headless CPU/data context while preventing offscreen
presentation entities from participating in the Bevy render pass.

## Implementation Summary

- Added `GraphicalTrue25dViewportRenderBypass` receipts to renderable True 2.5D
  presentation entities.
- Added `GraphicalTrue25dRenderBypassSummaryResource` with counts for:
  - renderable True 2.5D entities;
  - visible entities inside the locked camera viewport;
  - hidden/bypassed offscreen entities;
  - ledger-only headless True 2.5D assets;
  - active procedural chunks and materialized headless tiles.
- The bypass contract uses the locked orthographic camera constants:
  - `FixedVertical(10.0)`;
  - expected 16:9 horizontal view width;
  - a small presentation margin to avoid edge popping.
- Offscreen True 2.5D presentation entities receive `Visibility::Hidden`.
- The static repeated ground plane remains visible as the primary player
  surface.
- Procedural terrain/content generation remains data-ledger only and
  display/context only.

## Cadence Note

The render target remains the existing CA13 `60Hz` render cadence. The fixed
simulation/headless tick cadence remains the existing CA13 `20Hz` scheduler.
This slice does not change gameplay scheduling semantics and does not claim a
new 60Hz simulation tick. Changing the authoritative fixed tick cadence would
require a separate reviewed scheduler plan.

CA44A extension addendum
`docs/creatures_agi_roadmap_pack/status/TRUE25D_RENDER_BYPASS_PROOF.md`
tightens the presentation-side proof fields. It records the 60Hz presentation
headless cadence separately from the 20Hz authoritative simulation cadence and
adds explicit zero offscreen presentation draw/animation budgets. It still does
not advance the CA roadmap, does not change CA13 scheduler semantics, and does
not claim GPU profiler counters.

## Invariant Checks

- `alife_core` unchanged.
- No Bevy, wgpu, renderer, or app dependency added to `alife_core`.
- No simulation authority changed.
- No action path changed.
- Procedural terrain/content cannot emit actions or rewrite weights.
- CPU fallback unchanged.
- CPU shadow parity unchanged.
- No full action-authoritative GPU runtime claim.
- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media are intended for tracking.

## Focused Evidence

Focused commands:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d_viewport_render_bypass -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture
cargo test -p alife_world --test procedural_chunks procedural_world_travel -- --nocapture
cargo test -p alife_game_app --test app_shell ca44a_procedural_world_travel_smoke_streams_seeded_chunks_without_rendering -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results:

- Render-bypass focused test: PASS.
- True 2.5D focused test group: PASS, 6 tests.
- Procedural travel tests: PASS.
- Procedural travel app-shell smoke test: PASS.
- Default graphical smoke: PASS; selected `GpuPlastic` on
  `NVIDIA GeForce RTX 3050 api=Vulkan driver=581.80`; graphical smoke exited
  cleanly.
- Forced fallback graphical smoke: PASS; selected `CpuReference` with
  `HardwareUnavailable` fallback and visible degraded status.

## Validation Results

- `cargo fmt --all -- --check`: PASS.
- `cargo check --workspace --all-targets`: PASS.
- `cargo test --workspace --all-targets`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`: PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`: PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`: PASS.
- `cargo tree -p alife_core`: PASS; no Bevy/wgpu/app dependency leak.
- `cargo check --workspace --all-features --all-targets`: PASS.
- `cargo test --workspace --all-features --all-targets`: PASS after rerun with
  `CARGO_BUILD_JOBS=1`. The first all-features test attempt failed in MSVC
  `link.exe` with `LNK1000`/access violation while linking the all-features
  `alife_game_app` test binary; the single-job rerun passed without Rust test
  failures.
- `graphify update .`: PASS; refreshed ignored `graphify-out` graph data.

## Known Limitations

- This is a visibility/render-pass/draw-budget receipt, not a GPU draw-call
  profiler. It proves offscreen True 2.5D presentation entities are marked
  hidden before rendering and carry an explicit zero presentation draw budget;
  it does not capture hardware GPU draw-call counters.
- Fog of war remains presentation-side and not an authoritative sensory
  visibility system.
- The fixed headless simulation tick remains `20Hz`.
