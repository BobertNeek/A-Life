# FVR10 Completion

Status: FVR10 complete.

## Scope

FVR10 treats the FVR09 result as a visible art-direction failure and fixes the
actual production screenshots without changing simulation authority. The
referenced plan file was absent in this worktree:

```text
docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_ART_DIRECTION_SCREENSHOT_OVERHAUL.md
```

The active goal text and `FVR10_VISUAL_AUDIT.md` controlled this pass.

## Before Evidence

The audit recorded the pre-overhaul screenshot paths:

```text
target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot.png
target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot_fvr05_world_inspector.png
```

Those files are ignored generated artifacts and were overwritten by final
record-performance captures. The pre-overhaul findings are preserved in
`FVR10_VISUAL_AUDIT.md`: flat RGBA slabs, metadata-only texture labels, cuboid
creature stacks, visible debug UI, and descriptor JSON mislabeled as final art.

Visual blueprint used for this pass:

```text
C:\Users\PC\.codex\generated_images\019f2a54-ead6-76d1-a32a-51fb7a56cc1a\ig_0b7e337a4edf39e4016a4b1a4f64548197b95b3b0e7cb15ea4.png
```

## After Evidence

Final clean product screenshots:

```text
target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot.png
target/artifacts/fvr03/MinimumSettings30x30_runtime_screenshot.png
```

Diagnostic screenshots are also generated after the clean product capture:

```text
target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot_fvr05_world_inspector.png
target/artifacts/fvr03/MinimumSettings30x30_runtime_screenshot_fvr05_world_inspector.png
```

Reproduction commands:

```powershell
target\release\alife_game_app.exe production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan --record-performance
target\release\alife_game_app.exe production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan --record-performance
```

## Terrain Implementation

FVR10 binds visible terrain variation into the production mesh instead of relying
on JSON slot names. Production acceptance profiles now sample terrain at stride
2, emit `Mesh::ATTRIBUTE_COLOR`, prevent variation-incompatible greedy merges,
and use stronger procedural per-face color variation with explicit top/side
separation.

Renderer evidence:

| Profile | Tile Mesh Count | Palette | Emitted Quads | Merge Ratio |
|---|---:|---|---:|---:|
| `MinSpecComfort1080p` | 6400 | `fvr10-visible-surface-variation-v1` | 28818 | 1.333 |
| `MinimumSettings30x30` | 2304 | `fvr10-visible-surface-variation-v1` | 10446 | 1.323 |

## Creature Implementation

Creatures now use generated low-poly biped rigs under
`fvr10-readable-cute-biped-rig-v1` with brighter display materials, larger
face markers, camera-facing eyes/mouth, rounded body/head/legs/feet/arms, and
state-driven expression/animation. The renderer keeps the real stable creature
IDs and read-only state projection.

## Presentation

Product screenshots start with menu, settings, and overlays hidden. The capture
sequence writes a clean product screenshot first, then captures FVR05 diagnostic
panels. The default orthographic camera is closer for the two acceptance
profiles, and display-only hero dressing is denser around the creature cluster.

## Asset And License Policy

No external art assets or large generated artifacts were committed. The
production asset manifest keeps generated/project JSON descriptors licensed and
digested, but descriptor-only JSON entries are no longer mislabeled as final
visible art.

## Performance Evidence

Hardware evidence machine: NVIDIA RTX 3050 8 GB, Intel i7-3770K class target,
Windows, 1920x1080.

| Profile | Target FPS | Measured FPS | Backend | Fallback | Creatures | Screenshot |
|---|---:|---:|---|---|---:|---|
| `MinimumSettings30x30` | 30 | 184.52 | `GpuFull` Vulkan | None | 30 | `target/artifacts/fvr03/MinimumSettings30x30_runtime_screenshot.png` |
| `MinSpecComfort1080p` | 60 | 173.38 | `GpuFull` Vulkan | None | 30 | `target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot.png` |

Both GPU gameplay receipts report 30 proposals, 30 CPU shadow parity checks, 0
parity failures, `no_active_bulk_readback=true`, and no full-action-authority
claim beyond the guarded CPU-shadow product path.

## Save/Load Evidence

Both save validation commands passed from the fresh release binary:

```powershell
target\release\alife_game_app.exe validate-production-save --profile MinSpecComfort1080p --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan
target\release\alife_game_app.exe validate-production-save --profile MinimumSettings30x30 --population 30 --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan
```

Receipts reported `real_save_loaded=true`, `mock_data_source=false`,
`voxel_roundtrip=true`, `selected_backend=GpuFull`, `fallback=None`, and
`gpu_runtime_no_bulk_readback=true`.

## Boundary Invariants

- No Bevy, wgpu, renderer, UI, window, or asset handles were added to `alife_core`.
- `alife_world` save/world truth remains stable-ID based and renderer independent.
- Renderer/VFX/dressing/UI remain display-only and cannot issue hidden actions,
  bypass arbitration, inject rewards, mutate weights, or mutate cognition.
- No mock simulation, fake backend, fake GPU availability, or fake population
  path was added.
- Large generated artifacts remain under ignored `target/artifacts/`.

## Validation

Fresh validation passed:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo check --workspace --all-targets` | Passed |
| `cargo test --workspace --all-targets --quiet` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo check --workspace --all-features --all-targets` | Passed |
| `cargo test --workspace --all-features --all-targets --no-run` | Passed |
| `cargo test -p alife_game_app --all-features --all-targets --quiet` | Passed |
| `cargo test -p alife_core -p alife_world -p alife_gpu_backend -p alife_bevy_adapter -p alife_school -p alife_semantic -p alife_tools --all-features --all-targets --quiet` | Passed |
| `cargo test -p alife_game_app --features "bevy-app voxel-backend production-assets vfx-hanabi" --test fvr03_voxel_renderer fvr10_ -- --nocapture` | Passed, 5 tests |
| `target\release\alife_game_app.exe production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan --record-performance` | Passed |
| `target\release\alife_game_app.exe production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan --record-performance` | Passed |
| `target\release\alife_game_app.exe validate-production-save --profile MinSpecComfort1080p --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan` | Passed |
| `target\release\alife_game_app.exe validate-production-save --profile MinimumSettings30x30 --population 30 --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan` | Passed |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1` | Passed; `alife_core boundary checks passed` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1` | Passed |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1` | Passed |

All-features note:

The monolithic command below timed out after 20 minutes without reporting a test
failure:

```powershell
cargo test --workspace --all-features --all-targets --quiet
```

The same gate was then split into `--no-run`, `alife_game_app` all-features
tests, and all remaining package all-features tests. Those split commands
passed with zero failures and cover the same workspace packages and all-feature
test targets.

FVR10 acceptance statement:

FVR10 replaces the screenshot-visible FVR09 art-direction failure with bound
terrain vertex-color variation, denser voxel terrain sampling, readable generated
cute biped creature rigs, clean product screenshots, and honest asset metadata
while preserving the hard minimum profile, default comfort profile, real
save/load, real backend selection/fallback, no mocks, no renderer authority over
cognition/actions, and no renderer dependencies in `alife_core`.
