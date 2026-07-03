# Fullstack Bevy Voxel Frontend Replacement Plan Pack

Status: Codex Goal Mode implementation pack.

Repository target: `BobertNeek/A-Life`.

Hardware target supplied by Cassidy:

```text
Minimum comfortable spec, not a stretch target:
GPU: NVIDIA RTX 3050
VRAM: 8 GB
CPU: Intel Core i7-3770K
RAM: 32 GB DDR3
OS: Windows 10
Resolution target: 1920x1080
Platform: desktop only
```

## Controlling decision

Replace the old ugly frontend with a finished Bevy 0.18 fullstack game frontend whose default player view is a stylized voxel world with real ALife backend integration, GPU-backed neural/runtime scaling, saved backend state, production UI, production assets, and finished desktop validation. Reuse existing code only when it is cheaper and safer than replacement; lean toward replacement.

Cassidy's RTX 3050 / i7-3770K system is the minimum comfortable supported machine for the default desktop experience. The default `MinSpecComfort1080p` profile must run smoothly on that hardware without requiring the user to manually disable core features. Stronger machines may unlock larger worlds, more hot/warm brain slots, longer view distances, denser VFX, higher creature counts, larger brain classes, and research-scale modes through explicit scale-up profiles. The implementation must not hard-code the minimum spec as an architectural ceiling.

This pack deliberately rejects the previous pattern of alpha plans, practice attempts, smoke-only slices, manifest-only art contracts, mock data sources, and future-human cleanup. FVR00 is the only scaffolding/review pass. Every FVR plan after FVR00 must leave its owned subsystem finished, integrated, saved, validated, and enabled in the production path.

## Fixed architectural constraints

- Keep `alife_core` engine-independent. No Bevy, Avian, wgpu, renderer handles, OS window handles, external asset handles, or Bevy `Entity` values may enter `alife_core`.
- Keep `alife_world` renderer-independent except for explicit adapter-facing snapshot/export contracts.
- Keep Bevy, Avian, voxel rendering, particles, UI, picking, asset loading, and graphical settings inside `alife_bevy_adapter` and `alife_game_app`.
- Keep `alife_gpu_backend` responsible for wgpu/WGSL neural runtime work. It may expose compact production outputs and saved backend telemetry, but it must not become the game-world authority.
- CPU reference behavior remains the correctness oracle. The finished desktop app should select the GPU path on Cassidy's RTX 3050 when the real hardware probe and validation pass; CPU fallback is allowed only with explicit diagnostics.
- Active gameplay must not synchronously bulk-read neural buffers, weight buffers, lobe buffers, or per-synapse state. Compact action summaries, selected-creature debug snapshots, frame-bound diagnostics, sleep/export diagnostics, and saved performance receipts are allowed.
- No mockups, no mock simulation, no fake backend, no placeholder art-as-final, no visual-only lies. If an existing command or status file says alpha/smoke, Codex may preserve it only as a regression command; new production entrypoints must use production names and real data.
- External assets are allowed only if their license is committed and recorded in the asset manifest with source, license, digest, and local path. Prefer CC0, MIT, Apache-2.0, BSD, Zlib, or public-domain assets.
- Desktop only: Windows 10 first, Linux acceptable. No web/mobile work in this pack.

## Scalability rule

The minimum spec profile must be comfortable, but the architecture must remain scale-up capable. Codex must implement all performance-sensitive systems as profile-driven budgets, not single constants:

- `MinSpecComfort1080p`: default supported profile for RTX 3050 8 GB and i7-3770K. Smooth 1080p play, bounded chunk radius, GPU-instanced creatures, adaptive VFX, compact overlays, and conservative hot-brain residency.
- `Balanced1080p`: same feature set with moderate increases when timing headroom is available.
- `HighSpecScaleUp`: increased chunk radius, creature population, VFX density, hot/warm brain slots, and larger brain-class budgets on stronger GPUs/CPUs.
- `ResearchScale`: non-default experimental mode for large worlds, long soaks, larger brains, and high population studies. It may miss consumer-play comfort targets but must keep save/schema/ABI compatibility.

Data layouts must be sparse, chunked, class-bucketed, dirty-region based, and saved with profile metadata so later scale-up does not require rewriting world, brain, renderer, or save architecture.

## Bevy/ecosystem baseline

Use Bevy `0.18.0` because the current workspace already pins it and the preferred ecosystem crates align with it. Required crate direction:

- `bevy_voxel_world`: default voxel terrain backend if compatible; otherwise build a production internal chunk-mesh fallback in the same plan, without deferral.
- `bevy_sprite3d` or custom GPU-instanced billboard/mesh renderer: creature bodies, items, corpses, signs, social/emotional markers.
- Bevy core picking: selection, hover, tile/chunk/creature hit tests.
- `bevy_asset_loader`: production loading states and asset collections.
- `bevy_hanabi` or equivalent Bevy 0.18-compatible GPU VFX: pheromones, spores, dust, sleep/consolidation signals, decay fields.
- `avian3d`: optional presentation-facing collision/spatial query layer only; it must not become ecological truth.
- `bevy-inspector-egui` / `bevy_egui` / perf UI: debug and settings UI behind features and product debug modes.

## Plan sequence

FVR00 is the single scaffolding/review pass. It creates the exact demolition map and final file/API map, then stops. FVR01-FVR08 are completion-grade implementation plans.

| Plan | Title | Finished result |
|---|---|---|
| FVR00 | One-pass repo audit and replacement blueprint | Codex knows what to delete/reuse, where to write, and what exact validations prove completion. |
| FVR01 | Production launcher, dependency cutover, and frontend demolition | Old graphical frontend paths are retired or routed through the new production app; Bevy 0.18 voxel stack dependencies and feature flags are wired. |
| FVR02 | Real persistent world backend and chunk/snapshot contracts | `alife_world` owns saved procedural voxel chunk truth, ecology/resource layers, profile metadata, and adapter snapshots with no renderer contamination. |
| FVR03 | Finished default voxel world renderer | The player sees a stylized, streamed, selectable voxel world at 1080p with LOD, materials, lighting, camera, persistence-backed chunks, and min-spec/scale-up profile budgets. |
| FVR04 | GPU-scaled creatures, animation, selection, and expression | Real creatures render through GPU-friendly batching/instancing with expression data from core/world state, min-spec comfort budgets, and scale-up population profiles. |
| FVR05 | Production game UX, overlays, and inspectors | The frontend has real menus, camera, settings, debug overlays, brain/world inspection, profile controls, and no debug authority leaks. |
| FVR06 | Full gameplay GPU backend integration and saved runtime state | The neural/GPU runtime is selected and persisted as a real gameplay backend on RTX 3050 when validation passes, with CPU oracle fallback only as diagnosed fallback and scale-up profiles for stronger hardware. |
| FVR07 | Art, assets, VFX, audio-visual polish, and license manifest | The game is visually coherent, uses licensed/generated assets, GPU VFX, stylized materials, and adaptive quality budgets instead of placeholder art claims. |
| FVR08 | Final replacement hardening, packaging, and acceptance | The old ugly frontend is fully replaced; desktop package and validation prove comfortable 1080p operation on the minimum spec and preserve later scale-up. |

## Definition of done for the whole pack

The default desktop launch opens a polished 1080p Bevy game using the real saved ALife backend, a voxel terrain world, visible creatures, resource/ecology state, camera controls, selection, overlays, persistence, GPU-backed rendering/VFX, GPU neural/runtime selection on RTX 3050 when available, adaptive quality scaling, and explicit performance/diagnostic receipts. The app should be playable and inspectable without a human developer finishing missing systems.

The `MinSpecComfort1080p` profile must be the default on Cassidy's system and must run comfortably as the minimum supported spec. Higher profiles must scale up through data-driven budgets rather than forks or rewrites.

The system must preserve core architecture boundaries and save enough backend state to resume world, creatures, brain residency/backend mode, visual settings, chunk materialization, profile selection, and asset manifest references.

## Required final validation posture

Every plan after FVR00 must end with a completion receipt containing:

```text
Completion receipt
Plan: FVRxx - <title>
Branch:
Files changed:
Deleted/replaced old frontend files:
Public APIs changed:
Saved-state/schema changes:
Assets/licenses changed:
Commands run:
Hardware/GPU evidence:
Results:
Boundary invariants:
Deviations:
Known limitations:
```

`Known limitations` must be empty or restricted to explicit non-scope platform notes. It must not contain unfinished owned work.
