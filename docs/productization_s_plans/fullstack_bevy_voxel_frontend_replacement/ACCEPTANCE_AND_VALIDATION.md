# Fullstack Bevy Voxel Frontend Replacement - Acceptance and Validation Gates

This file defines the non-negotiable validation standard for FVR01-FVR08. It exists to prevent Codex from substituting an alpha/smoke/mock/test-harness result for finished product work.

## Hardware acceptance target

```text
GPU: NVIDIA RTX 3050
VRAM: 8 GB
CPU: Intel Core i7-3770K
RAM: 32 GB DDR3
OS: Windows 10
Resolution: 1920x1080
Renderer: Bevy 0.18 desktop
Default view: voxel terrain backend
```

The i7-3770K is old enough that CPU-side per-agent and per-tile overhead must be treated as a primary risk. Prefer GPU instancing, chunk batching, dirty-region updates, cached meshes/materials, compact snapshots, and bounded UI/debug readback.

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
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --profile Rtx3050Balanced1080p --population 500 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile Rtx3050Balanced1080p
```

Commands named `smoke`, `alpha`, or `contract-only` may remain only as legacy regression commands and must not be the product acceptance path.

## Boundary gates

- `alife_core` must not depend on Bevy, wgpu, Avian, renderer crates, OS windowing crates, asset-loader crates, external art handles, or UI crates.
- `alife_world` must not persist Bevy `Entity`, Avian handles, wgpu handles, renderer handles, OS window handles, or asset-loader runtime handles.
- Save files persist stable IDs, schema versions, asset references/digests, chunk seeds/edits, materialized backend metadata, creature/brain summaries, and backend descriptors.
- GPU backend save state persists descriptors and validation receipts, not raw wgpu handles.
- UI/debug/renderer systems cannot issue direct actions, rewrite weights, inject rewards, bypass action arbitration, or mutate hidden cognition.

## Visual acceptance gates

The default launch must show:

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

## Performance gates

Target: 60 FPS at 1080p on the supplied RTX 3050 8GB / i7-3770K / Win10 machine for normal/default play.

Population tiers: 1, 10, 50, 100, 250, 500 real creatures.

For each tier, record:

- selected quality profile;
- selected backend;
- average FPS or frame time;
- p95 frame time if available;
- active chunk count;
- rendered creature count;
- VFX budget state;
- GPU neural/runtime timing if available;
- fallback/adaptive actions taken.

If 500 agents cannot hold 60 FPS, the app must automatically enter an adaptive profile and record the exact degraded budgets. That is acceptable only if normal/default play remains smooth and no measurement is fabricated.

## Persistence gates

Save/load must preserve:

- world seed and schema version;
- materialized chunk edits and chunk signatures;
- creature stable IDs and selected creature if still alive;
- resources/hazards/ecology state;
- brain/runtime backend descriptors;
- visual quality profile and UI settings;
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

FVR08 is complete only when a clean checkout can launch the production voxel frontend, load/create a real saved ALife world, render it attractively, interact with real creatures, use the GPU backend when validated, save/reload backend state, and produce honest performance/diagnostic receipts without requiring a human developer to fill in missing systems.
