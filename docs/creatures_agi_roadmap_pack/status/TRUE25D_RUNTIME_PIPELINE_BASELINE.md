# True 2.5D Runtime Pipeline Baseline

## Status

- Branch: `codex/true25d-runtime-pipeline-baseline`
- Scope: focused visual/runtime presentation baseline after CA44A.
- Roadmap position: post-CA44A visual pipeline hardening; no CA45 started.

## Completed In This Slice

- Locked the true 2.5D Player View camera contract to:
  - orthographic projection;
  - `FixedVertical(10.0)`;
  - transform position `(0.0, 12.0, 12.0)` looking at origin;
  - no runtime zoom/rotation mutation for the 3D presentation camera.
- Replaced the default true 2.5D rendered ground surface with a committed preprocessed repeating tile:
  - `crates/alife_game_app/assets/alpha_art_v1/ground_tile_repeat.png`;
  - Bevy `Plane3d` substrate;
  - repeat-wrapped sampler;
  - fixed UV repeat;
  - no synchronous runtime biome-texture generation for the default ground.
- Preserved the procedural chunk/terrain field as a CPU/data ledger:
  - generated without rendering;
  - materialized near active creature views;
  - no simulation or action authority changed.
- Added true 2.5D glTF scale normalization receipts for scene-root entities when glTF rendering is available.

## Focused Evidence

- `cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture`
  - Result: PASS, 5 passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded`
  - Result: PASS after warm build cache.
  - Runtime preflight selected `GpuPlastic` on `NVIDIA GeForce RTX 3050 api=Vulkan driver=581.80`.
  - Graphical smoke exited cleanly after 30 seconds.
- Forced fallback smoke:
  - Command: `$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded`
  - Result: PASS.
  - Runtime preflight selected `CpuReference` with `HardwareUnavailable` fallback and visible degraded status.

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
- `cargo test --workspace --all-features --all-targets`: PASS.

## Boundaries Preserved

- `alife_core` unchanged.
- No Bevy/wgpu/app dependency leaked into `alife_core`.
- CPU fallback unchanged.
- CPU shadow parity unchanged.
- No full action-authoritative GPU claim.
- No S12/G25/P37.
- No release tag.
- No screenshots, logs, target artifacts, model caches, or generated media tracked.

## Remaining Work In Active Goal

- Full dynamic post-processing render pass is not implemented in this slice:
  - low-resolution downsample pass remains to be implemented;
  - toon quantization pass remains to be implemented;
  - Sobel depth/normal outline pass remains to be implemented.
- Blender normalization/decimation has not been run in this slice; the runtime scale clamp now protects Bevy instantiation.
- Neurochemical visual feedback remains limited to the existing display-only pose/cue layer and should be expanded in a later slice.
- Headless chunk/render-bypass behavior remains covered by the existing procedural field ledger and should be profiled for draw-call evidence in a later slice.
