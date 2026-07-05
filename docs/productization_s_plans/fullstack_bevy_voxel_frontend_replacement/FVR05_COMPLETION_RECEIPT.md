# FVR05 Completion Receipt - Production UX, Overlays, And Inspectors

Date: 2026-07-05

Branch: `codex/fvr05-production-ux-debug-layer`

## Scope Completed

FVR05 adds the production UX and debug inspection layer on top of the real Bevy
0.18 voxel frontend. The work stays in `alife_game_app`; no Bevy, wgpu,
renderer, UI, or windowing types were added to `alife_core`.

Implemented surfaces:

- Top production runtime bar with profile, backend/API, adapter, FPS target,
  frame budget, run/pause state, and save label.
- Left production control rail for play/pause, save, load, create-world
  artifact, menu/settings visibility, profile preference, speed, overlays, and
  live runtime stats.
- Right inspector with Creature, Tile, World, and GPU tabs.
- Bottom overlay toolbar for Resources, Danger, Pheromones, Energy, Age,
  Fertility, Territory, Neural, Residency, BackendTiming, ChunkBoundaries,
  LodBudget, and Persistence overlays.
- Footer status bar with camera controls, chunk/LOD/resident-byte summary,
  backend, UX settings path, and stable scene signature.
- Keyboard model:
  `Space/P` pause, `[`/`]` speed, `M` menu, `G` settings, `H` overlays,
  `Tab` inspector tab, `Q` preferred next-launch profile, `S` save real
  production runtime state plus UX settings, `L` validate/load saved runtime
  state and UX settings, `N` create a real production world-save artifact,
  `1-9/B/C/D/V` overlay toggles, `O/I/F` existing camera/follow controls.

## Persistence

New app-local UX schema:

```text
alife.fvr05.production_ux.v1
schema_version=1
```

The UX settings document persists only stable production values: selected and
preferred profile IDs, inspector tab, overlay labels, camera label, pause/speed,
panel visibility, selected stable ID raw value, save/config paths, backend
descriptor, and validation receipt. It rejects engine-local tokens such as
Bevy, wgpu, renderer handles, window handles, material handles, and ECS entity
tokens.

P34/FVR02 world save schemas were not changed. The production save path still
uses stable IDs, voxel backend descriptors, profile metadata, and real save
validation. Runtime UI save/create actions materialize production save artifacts
under `target/artifacts/fvr05/` and do not serialize Bevy entities or wgpu
handles.

## Debug Authority

FVR05 debug and inspector surfaces are read-only projections.

Authority statement:

```text
read_only=true actions_blocked=true arbitration_bypass_blocked=true rewards_blocked=true weights_blocked=true cognition_blocked=true bulk_readback_blocked=true
```

The UI can toggle visibility, pause presentation animation, adjust presentation
speed, select stable references, write validated production save artifacts, and
persist UI preferences. It cannot issue direct actions, bypass action
arbitration, inject rewards, mutate weights, mutate hidden cognition, or perform
bulk active neural readback.

## Visual Evidence

Blueprint used:

```text
D:\A life\target\artifacts\fvr05\fvr05_production_ux_blueprint.png
```

Recorded production screenshots:

```text
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot_fvr05_menu_settings_creature.png
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot_fvr05_tile_inspector.png
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot_fvr05_world_inspector.png
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot_fvr05_gpu_panel.png
```

These screenshots show the production top bar, left controls/settings rail,
right inspector states, bottom overlay toolbar, real voxel terrain, and real
creature projections.

## Runtime Evidence

Production dry run:

```powershell
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --profile MinimumSettings30x30 --population 30 --dry-run
```

Result: passed. Backend selected `GpuPlastic`; adapter `NVIDIA GeForce RTX
3050`; API `Vulkan`; fallback `None`; real save loaded; mock data source
`false`; voxel roundtrip `true`; UX schema `alife.fvr05.production_ux.v1`.

Production visual/performance screenshot run:

```powershell
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --profile MinimumSettings30x30 --population 30 --record-performance
```

Result: passed and wrote the screenshot sequence above plus:

```text
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_renderer_diagnostics.json
```

Production save validation:

```powershell
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
```

Result: passed with `GpuPlastic`, `NVIDIA GeForce RTX 3050`, `Vulkan`, fallback
`None`, real save loaded, voxel renderer tokens saved `false`, and UX authority
receipt included.

## Validation Receipt

Passed:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test -p alife_game_app --features "bevy-app gpu-runtime voxel-backend debug-tools" fvr05 -- --nocapture
cargo test -p alife_game_app --features "bevy-app gpu-runtime voxel-backend debug-tools" ux -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
C:\Users\PC\.local\bin\graphify.exe update .
```

Note: the plan-pack wording referenced a `debug-ui` feature name, but this repo
defines `debug-tools`; the validation used the existing feature name.

Focused FVR05 test coverage:

- Overlay catalog includes every required production debug surface.
- UX settings roundtrip preserves profile/overlays and rejects engine-local
  tokens.
- Debug authority report blocks direct actions, reward injection, weight
  mutation, hidden cognition mutation, arbitration bypass, and bulk neural
  readback.
- Renderer overlay toggles do not change the stable scene simulation signature.

## Progress Index

No `docs/progress/` directory exists in this branch, so no progress/index file
required an update.

## FVR06 Readiness

FVR06 can start without more UX/debug-layer planning after this branch is
merged.
