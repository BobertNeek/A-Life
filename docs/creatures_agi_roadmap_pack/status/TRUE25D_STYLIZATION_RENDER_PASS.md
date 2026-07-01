# True 2.5D Stylization Render Pass

Plan context: post-CA44A True 2.5D runtime pipeline hardening
Branch: `codex/true25d-stylization-render-pass-phase4`

## Objective

Make the default True 2.5D Player View use a concrete GPU-backed
stylization shader path instead of only a manifest-level art-direction
contract.

## Implementation Summary

- Added `crates/alife_gpu_backend/shaders/true25d_stylization_postprocess.wgsl`.
- Added the shader to `crates/alife_game_app/app_bundle_manifest.json`.
- Updated the True 2.5D asset manifest so the shader stack is no longer
  marked contract-only.
- Added strict manifest validation that rejects a contract-only True 2.5D
  shader stack.
- Added a feature-gated Bevy postprocess plugin in `alife_game_app`:
  - embeds the WGSL shader source;
  - attaches stylization settings to the locked True 2.5D player camera;
  - registers a Core3d render graph node after tonemapping;
  - samples the current postprocess color target;
  - samples the main view depth texture after explicitly enabling
    `TEXTURE_BINDING` on the True 2.5D `Camera3d` depth texture usage.
- Added `GraphicalTrue25dStylizationRenderPassResource` as the runtime
  evidence receipt.

## Stylization Contract

Runtime pass:

- low-resolution pixel-step sampling at `320x240`;
- four-band toon quantization;
- depth Sobel silhouette outline using the Bevy view depth texture;
- luminance Sobel fallback for internal color edges;
- display-only, no action authority, no hidden gameplay state.

Deferred:

- normal-buffer Sobel edge detection. This slice did not add a Bevy normal
  prepass or G-buffer dependency. The current pass uses depth and luminance,
  not normals.

## Runtime Issue Found And Fixed

The first live graphical smoke failed because Bevy's default 3D depth texture
was allocated as `RENDER_ATTACHMENT` only. Binding it as a shader texture
caused a wgpu validation error:

```text
TextureView usage flags do not contain required usage flags TEXTURE_BINDING
```

The fix is to set the True 2.5D camera `depth_texture_usages` to
`RENDER_ATTACHMENT | TEXTURE_BINDING` before the core pipeline prepares the
view depth texture.

## Invariant Checks

- `alife_core` unchanged.
- No Bevy, wgpu, renderer, app, or model-runtime dependency added to
  `alife_core`.
- No simulation authority changed.
- No action path changed.
- No UI, shader, semantic, teacher, memory, topology, or GPU path can emit
  actions directly.
- CPU fallback unchanged.
- CPU shadow parity unchanged.
- No active bulk neural readback added.
- No full action-authoritative GPU runtime claim.
- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media are intended for tracking.

## Focused Evidence

Commands:

```powershell
cargo test -p alife_game_app true_25d_assets -- --nocapture
cargo test -p alife_game_app ca12_app_bundle_manifest_discovers_assets_shaders_and_placeholder_art -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture
cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground --scenario gpu-alpha --gpu-mode static-plastic-cpu-shadow-guarded --view-mode player --smoke-seconds 5
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results:

- True 2.5D asset validation tests: PASS, 4 tests.
- App bundle shader discovery test: PASS; six WGSL shader assets are listed
  and discovered.
- True 2.5D Bevy app-shell tests: PASS, 7 tests.
- Short graphical smoke: PASS after enabling sampleable depth texture usage.
- Default graphical smoke: PASS; selected `GpuPlastic` on local
  NVIDIA RTX 3050/Vulkan, fallback `None`, and exited cleanly.
- Forced CPU fallback graphical smoke: PASS; selected `CpuReference`,
  fallback `HardwareUnavailable`, showed degraded fallback status, and exited
  cleanly.

## Validation Results

- `cargo fmt --all -- --check`: PASS after formatting the branch.
- `cargo check --workspace --all-targets`: PASS.
- `cargo test --workspace --all-targets`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`:
  PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`:
  PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`:
  PASS.
- `cargo tree -p alife_core`: PASS; `alife_core` remains free of Bevy, wgpu,
  renderer, app, and model-runtime dependencies.
- `cargo check --workspace --all-features --all-targets`: PASS.
- `cargo test --workspace --all-features --all-targets`: PASS with
  `CARGO_BUILD_JOBS=1` to avoid the known MSVC all-features linker flake.

## Known Limitations

- Normal-buffer Sobel is not active yet.
- The pass is a postprocess stylization layer only. It does not change world,
  sensory, action, learning, or GPU neural semantics.
- The current shader pass does not make any full action-authoritative GPU
  runtime claim.
- Blender was not used in this phase; the fixed Blender path is relevant to
  the separate mesh/asset calibration slice, not this shader-pass validation.
- Runtime-generated build outputs under `target/` remain untracked.

## Next Plan

Continue this goal with the remaining True 2.5D visual pipeline work. Do not
resume CA roadmap execution from this status document.
