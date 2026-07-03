# Fullstack Bevy Voxel Frontend Replacement - Acceptance and Validation Gates

This file defines the non-negotiable validation standard for FVR01-FVR08. It exists to prevent Codex from substituting an alpha/smoke/mock/test-harness result for finished product work.

## Hardware acceptance target

```text
Minimum comfortable supported spec, not a stretch benchmark:
GPU: NVIDIA RTX 3050
VRAM: 8 GB
CPU: Intel Core i7-3770K
RAM: 32 GB DDR3
OS: Windows 10
Resolution: 1920x1080
Renderer: Bevy 0.18 desktop
Default view: voxel terrain backend
Default profile: MinSpecComfort1080p
Minimum settings floor: MinimumSettings30x30, 30 real creatures at 30 FPS
```

The i7-3770K is old enough that CPU-side per-agent and per-tile overhead must be treated as a primary risk. Prefer GPU instancing, chunk batching, dirty-region updates, cached meshes/materials, compact snapshots, bounded UI/debug readback, async/background preparation where safe, and profile-driven budgets.

The minimum hardware class has two acceptance thresholds:

1. `MinimumSettings30x30` is the hard playability floor. It must run 30 real creatures at 30 FPS with real simulation, real saves, visible voxel terrain, creature selection, essential overlays, essential UI, and backend selection/fallback diagnostics. It may reduce internal render scale, chunk radius, shadows, VFX density, overlay resolution, label density, and nonessential update cadence. It may not substitute mocks or remove core gameplay.
2. `MinSpecComfort1080p` is the default comfortable profile for Cassidy's machine. It should target smooth 1080p play with all core features enabled and conservative budgets.

Scale-up must remain available through explicit profiles rather than architectural rewrites. Neither the RTX 3050 target nor the 30-creature floor may become a ceiling.

## Profile acceptance model

Codex must implement and validate profile-driven scaling. Exact numeric budgets may be tuned by measurement, but the named profiles and their semantics are required.

| Profile | Purpose | Acceptance posture |
|---|---|---|
| `MinimumSettings30x30` | Hard minimum-settings floor on the minimum hardware class | 30 real creatures at 30 FPS, real voxel world, real saves, real backend selection/fallback, essential UI/overlays, conservative chunk/render/VFX/neural budgets, no mocks. |
| `MinSpecComfort1080p` | Default on RTX 3050 8 GB / i7-3770K / Win10 / 1080p | Smooth playable 60 FPS target for normal/default gameplay, all core visual/gameplay features enabled, conservative chunk/creature/VFX/neural budgets. |
| `Balanced1080p` | Same hardware or modestly stronger hardware with measured headroom | Increases view distance, creature density, VFX, overlays, and hot/warm brain slots only when timing budget allows. |
| `HighSpecScaleUp` | Stronger desktop GPUs/CPUs | Larger worlds, higher population, denser VFX, longer draw distance, richer shadows/materials, and more hot/warm neural residency. |
| `ResearchScale` | Non-default experiments and long soaks | May exceed comfort/FPS targets, but must preserve schemas, saves, ABI compatibility, no-readback rules, and honest diagnostics. |

No subsystem may hard-code the minimum spec as a permanent ceiling. Chunk radius, mesh budgets, VFX particle caps, overlay resolution, hot/warm/cold brain slots, population density, draw distance, neural cadence, internal render scale, material complexity, label density, and debug sampling must be profile-controlled or derived from measured budgets.

## Global validation commands

Codex should run the narrow relevant commands first, then the standard set. On Windows, prefer PowerShell wrappers over bare bash.

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

If all-features tests hit a known local MSVC linker/resource issue, Codex must record the exact failure, run a narrower equivalent matrix, and not claim the full command passed.

## Required production commands by the end of FVR08

Names may differ if FVR00/FVR01 selects better names, but the finished app must expose equivalent production commands:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --profile MinSpecComfort1080p --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --profile HighSpecScaleUp --population 500 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p
```

Commands named `smoke`, `alpha`, or `contract-only` may remain only as legacy regression commands and must not be the product acceptance path.

## Boundary gates

- `alife_core` must not depend on Bevy, wgpu, Avian, renderer crates, OS windowing crates, asset-loader crates, external art handles, or UI crates.
- `alife_world` must not persist Bevy `Entity`, Avian handles, wgpu handles, renderer handles, OS window handles, or asset-loader runtime handles.
- Save files persist stable IDs, schema versions, asset references/digests, chunk seeds/edits, materialized chunk metadata, profile metadata, creature/brain summaries, and backend descriptors.
- GPU backend save state persists descriptors and validation receipts, not raw wgpu handles.
- UI/debug/renderer systems cannot issue direct actions, rewrite weights, inject rewards, bypass action arbitration, or mutate hidden cognition.

## Visual acceptance gates

The `MinimumSettings30x30` launch must show:

- stylized voxel terrain generated from real saved `alife_world` chunks;
- streamed chunks with conservative LOD/draw-distance policy;
- readable biome/material variation, even if simplified;
- selectable terrain/chunk/tile coordinates;
- 30 visible real creatures with stable ID selection;
- basic creature animations and expression state;
- resource/hazard/ecology visualization at reduced density/resolution if needed;
- essential overlays for important ALife fields;
- usable UI and settings;
- licensed/generated final assets, not placeholder claims.

The default `MinSpecComfort1080p` launch must show:

- stylized voxel terrain generated from real saved `alife_world` chunks;
- streamed chunks with visible LOD/draw-distance policy;
- readable biome/material variation;
- lighting/shadows or coherent stylized substitutes;
- selectable terrain/chunk/tile coordinates;
- visible real creatures with stable ID selection;
- creature animations and expression state;
- resource/hazard/ecology visualization;
- overlays for important ALife fields;
- polished UI and settings;
- licensed/generated final assets, not placeholder claims.

All core gameplay/inspection features must remain enabled in both minimum and comfort profiles. The minimum profile may reduce density, resolution, radius, particle counts, shadow quality, label density, and update cadence, but it may not remove real simulation, creature interaction, save/load, backend diagnostics, voxel terrain, or essential overlays.

## GPU/runtime gates

The production app may select GPU runtime only when the real probe and validation pass. It must report:

- adapter/backend identity;
- selected neural backend mode;
- renderer profile;
- fallback reason or `None`;
- frame timing;
- chunk/creature/VFX budgets;
- compact neural/action readback size;
- CPU shadow parity status for bounded validation;
- saved backend descriptor version.

Active gameplay must not perform bulk neural readback. Large diagnostics belong at frame, manual validation, sleep/export, or performance-report boundaries.

The RTX 3050 machine must be expected to select the GPU path when validation succeeds. If it falls back to CPU, the app must show an explicit degradation reason and keep at least the `MinimumSettings30x30` floor playable through conservative CPU/GPU/render budgets.

## Performance gates

Hard floor: `MinimumSettings30x30` must sustain playable 30 FPS with 30 real creatures on the supplied RTX 3050 8 GB / i7-3770K / Win10 machine, using real world/core/gpu/app paths and no mock systems.

Comfort target: `MinSpecComfort1080p` targets comfortable 60 FPS at 1080p on the same machine for the default gameplay population/world settings chosen by FVR00/FVR08.

Population tiers: 1, 10, 30, 50, 100, 250, 500 real creatures.

Minimum-settings floor acceptance:

- The 30-creature tier must run at 30 FPS or better under `MinimumSettings30x30`.
- The profile must include real creatures, voxel terrain, essential overlays, UI, save/load, and GPU runtime selection/fallback diagnostics.
- The profile may use reduced internal render scale or lower quality, but it must remain visually readable and playable.
- Failure to meet 30 creatures at 30 FPS on the minimum hardware class is a release blocker for FVR08.

Minimum-spec comfort acceptance:

- The default profile must sustain smooth play for the normal/default population and world settings chosen by FVR00/FVR08.
- The default profile must include real creatures, voxel terrain, overlays, UI, VFX, save/load, and GPU runtime selection/fallback diagnostics.
- Population tiers above the default comfort setting must remain runnable through adaptive quality and residency budgets, not separate mock modes.
- The 500-creature tier is a scale-up/benchmark gate. On the minimum spec, it may use visible adaptive reductions if needed, but it must not crash, hang, allocate unbounded memory, or require manual code changes.

For each tier, record:

- selected quality profile;
- selected backend;
- average FPS or frame time;
- p95 frame time if available;
- active chunk count;
- rendered creature count;
- hot/warm/cold brain slot counts;
- internal render scale if not native 1080p;
- VFX budget state;
- GPU neural/runtime timing if available;
- fallback/adaptive actions taken.

If 500 agents cannot hold 60 FPS on minimum spec, the app must automatically enter an adaptive profile and record the exact degraded budgets. That is acceptable only if normal/default play remains smooth and the 30-creature/30-FPS floor is met. On stronger machines, `HighSpecScaleUp` and `ResearchScale` must be able to raise the relevant budgets without schema or architecture rewrites.

## Persistence gates

Save/load must preserve:

- world seed and schema version;
- materialized chunk edits and chunk signatures;
- creature stable IDs and selected creature if still alive;
- resources/hazards/ecology state;
- brain/runtime backend descriptors;
- visual quality profile and UI settings;
- minimum-settings and scale-up profile metadata and budget overrides;
- asset manifest references and digests;
- performance/backend validation receipt metadata.

A reload must not depend on Bevy entity IDs or wgpu handles being stable.

## Asset/license gates

Every external asset must have:

- source URL or origin note;
- license name;
- license text or committed license reference;
- author/creator when available;
- local path;
- digest;
- usage category;
- replacement policy.

Unclear, missing, copyleft-incompatible, or non-redistributable assets are rejected. Generated assets must record generator source/config where practical.

## No-mock rule

Do not add a mock simulation, mock backend, fake population, fake world, or fake GPU availability path to pass these gates. Production validation may use small real saves/configs, but they must exercise the real core/world/gpu/app path.

## Completion standard

FVR08 is complete only when a clean checkout can launch the production voxel frontend, load/create a real saved ALife world, render it attractively, interact with real creatures, use the GPU backend when validated, save/reload backend state, meet the 30-creature/30-FPS minimum-settings floor, run comfortably on the default minimum-spec profile, expose scale-up profiles for stronger hardware, and produce honest performance/diagnostic receipts without requiring a human developer to fill in missing systems.
