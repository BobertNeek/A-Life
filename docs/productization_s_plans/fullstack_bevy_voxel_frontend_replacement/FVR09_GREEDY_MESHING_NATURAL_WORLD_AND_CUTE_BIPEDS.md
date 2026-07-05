# FVR09 - Greedy Meshing, Natural World Textures, and Cute Bipedal Creature Redesign

Branch: `codex/FVR09-greedy-meshing-natural-world-cute-bipeds`

Prerequisites: FVR08 accepted or functionally complete enough that the production voxel frontend launches from the real backend.

Concurrency: Serial follow-up after FVR08. Do not run concurrently with FVR08 because this goal may change voxel mesh generation, material assets, creature renderer inputs, asset manifests, and performance validation.

Recommended model/reasoning: GPT-5.5 Pro High.

## Purpose

Upgrade the current production voxel frontend from functional but crude visuals into a performant and visually coherent game presentation.

The current state to replace:

- voxel terrain exists but is mostly primary colors;
- terrain lacks natural textures/material variation;
- creatures are very basic, currently resembling two cubes with a flat plane between them;
- creature silhouettes are not cute, bipedal, or expressive enough;
- voxel chunk rendering must be hardened with binary/greedy meshing concepts before the world gets richer textures and more visible detail.

This goal must leave the production frontend with:

- binary-mask/greedy-quad voxel chunk meshing or a verified equivalent;
- natural-looking voxel terrain materials and textures;
- cute bipedal creature meshes or instanced creature rigs;
- creature materials/textures that read clearly at isometric/orthographic distance;
- preserved `MinimumSettings30x30` and `MinSpecComfort1080p` performance gates;
- no mock world, no fake creatures, no placeholder art marked final.

## Non-negotiable constraints

- Do not put Bevy, wgpu, renderer handles, asset handles, or Bevy `Entity` values into `alife_core`.
- Do not make visuals authoritative over simulation, actions, learning, reproduction, creature state, or backend selection.
- Do not replace real creature/world/backend data with mock data to make the visuals look better.
- Do not import Bevy 0.13 demo code directly from TanTanDev/binary_greedy_mesher_demo. Treat it and cgerikj/binary-greedy-meshing as algorithmic references only.
- Do not add unlicensed assets. Every external asset must have source, author if known, license, digest, local path, and manifest entry.
- Keep the production default path real: production voxel frontend, real saved backend, real creatures, real profile budgets.
- Keep `MinimumSettings30x30`: 30 real creatures at 30 FPS on the minimum hardware class.
- Keep `MinSpecComfort1080p`: default comfortable profile on RTX 3050 8 GB / i7-3770K / Windows 10 / 1080p.
- Preserve future scale-up through profiles rather than hard-coded ceilings.

## Scope

Owned crates and files are expected to include, but are not limited to:

- `crates/alife_bevy_adapter/**`
- `crates/alife_game_app/**`
- `crates/alife_world/**` only for renderer-independent material IDs, texture/material references, saved profile metadata, or existing snapshot fields
- `crates/alife_gpu_backend/**` only if existing renderer/VFX shader packaging requires it; do not redesign neural runtime here
- `assets/**` or the existing production asset directory
- app bundle manifests, asset manifests, license manifests
- production launch scripts and validation commands
- docs under `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/`

Forbidden unless directly required by an existing boundary-preserving contract:

- `alife_core` renderer dependencies
- neural-runtime redesigns
- new mock simulations
- fake creature population generators outside production world config
- screenshot-only validation
- placeholder art marked as final

## Implementation requirements

### 1. Inspect the existing FVR08 result first

Before editing, inspect the current production voxel frontend and record:

- current voxel mesh path: naive cube meshes, culled faces, greedy quads, `bevy_voxel_world`, or internal mesh backend;
- current chunk size, mesh rebuild path, dirty-chunk handling, material assignment, and selection/raycast path;
- current creature renderer path: entities, meshes, sprite3d, instancing, materials, animation state;
- current terrain material system and asset manifest entries;
- current profile budgets for `MinimumSettings30x30`, `MinSpecComfort1080p`, `HighSpecScaleUp`, and `ResearchScale`;
- measured or recorded FVR08 performance evidence.

Add a short implementation note to the completion receipt. Do not stop at inspection; this goal must implement the improvements.

### 2. Add binary-mask / greedy-quad voxel meshing

Implement or harden a production voxel chunk meshing path based on binary greedy meshing principles.

Required behavior:

- chunk-local occupancy masks;
- face visibility masks for the six face directions;
- material-aware greedy rectangle merge so faces only merge when material/texture/AO/normal bucket compatibility is preserved;
- neighbor-border handling so chunk seams do not show missing or duplicate faces;
- dirty-chunk remeshing only, with bounded per-frame remesh budget;
- mesh-cache invalidation keyed by chunk coordinate, chunk version, material palette version, and profile-relevant mesh settings;
- selectable terrain coordinates preserved after meshing;
- ray/picking/collision path still resolves to stable world/chunk/tile coordinates, never Bevy IDs in saves/core;
- instrumentation for visible voxels, emitted faces, emitted quads, merge ratio, remesh time, dirty chunks, cached chunks, and skipped chunks.

If the current voxel backend is `bevy_voxel_world`, first determine whether it already provides equivalent greedy meshing and whether its exposed hooks are enough. If it cannot satisfy the above requirements, build an internal production mesh generation module behind the existing backend abstraction in the same goal.

Implementation guidance:

- Use Rust `u64` masks on CPU where helpful.
- For WGSL or GPU-facing masks, use portable `u32` low/high pairs rather than assuming 64-bit shader integer support.
- Use `trailing_zeros` / `mask &= mask - 1` style set-bit iteration on CPU hot loops.
- Avoid entity-per-face or entity-per-voxel designs in the production path.
- Prefer compact vertex/quad records and chunk-level mesh entities.
- Keep material buckets separate enough that texture seams, biome variation, and natural sides/tops remain correct.

Reference concepts only:

- TanTanDev `binary_greedy_mesher_demo` for Rust/Bevy reference structure;
- cgerikj `binary-greedy-meshing` for occupancy masks, face masks, greedy quads, and compact quad records.

Do not copy incompatible code blindly. Port the algorithm to the current repo, Bevy version, asset system, profile system, and save/runtime contracts.

### 3. Replace primary-color terrain with natural voxel materials

Implement a terrain material and texture pass that makes the world look natural without sacrificing voxel readability.

Required materials/textures:

- grass top with dirt/soil sides;
- soil/dirt variation;
- stone/rock;
- sand or dry soil;
- water or wet ground if water exists in the current world;
- decay/rot/corpse-influenced ground;
- food/resource vegetation;
- hazard material(s);
- at least one biome variation set if the world backend exposes biome/ecology zones.

Required visual behavior:

- remove primary-color terrain as the production default;
- keep high-contrast debug colors only behind explicit debug overlay modes;
- use texture atlas, array textures, material palette, or generated procedural textures suitable for Bevy 0.18;
- support top/side material differences for grass/dirt-style blocks;
- support per-biome tint or texture variation without exploding material count;
- add subtle randomness/variation by chunk/voxel coordinate where deterministic and cheap;
- keep `MinimumSettings30x30` readable even if texture resolution or material variation is reduced;
- preserve stable save semantics by storing material IDs and asset references, not renderer handles.

Asset policy:

- External assets are allowed only if permissively licensed and manifest-recorded.
- Procedural/generated textures are acceptable and preferred if they can be made coherent.
- If generated textures are used, commit the generator/config or deterministic source assets as appropriate.
- Update asset/license manifests with source/license/digest/path/usage/replacement policy.

### 4. Replace crude creatures with cute bipedal creatures

Implement a finished creature visual design that reads as cute, bipedal, and alive at the default camera distance.

The design should be stylized and cheap to render, not high-poly realistic.

Required creature silhouette:

- two legs/feet, clearly bipedal;
- rounded or softened body/head proportions;
- visible head/face or face-like front orientation;
- small arms or side appendages if feasible;
- distinct eyes or eye-like expression markers;
- readable front/back orientation;
- cute proportions: larger head, compact body, short limbs, soft colors, non-threatening posture;
- enough variation to distinguish species/state without becoming noisy.

Required runtime behavior:

- use real creature stable IDs and real creature/world state;
- preserve selection, hover, following camera, selected-creature panel, and save/load identity;
- render movement, idle, eating, fleeing, sleeping, death/corpse, birth/reproduction/offspring marker where those states exist;
- map drives/state into visual expression without mutating cognition: hunger, fear/danger, fatigue/sleep, valence, reproduction, social cue;
- support 30 creatures at 30 FPS under `MinimumSettings30x30`;
- support profile-scaled detail for higher profiles;
- avoid per-creature material churn and excessive entity hierarchies.

Acceptable implementation approaches:

- low-poly articulated mesh assembled from simple primitives but with rounded/cute proportions;
- generated mesh asset with a small number of submeshes/material slots;
- GPU-instanced creature mesh with per-instance attributes;
- sprite3d/billboard hybrid only if it reads as bipedal and cute from the production camera;
- a simple procedural rig with head/body/legs/arms if it remains performant.

Unacceptable result:

- two cubes with a flat plane between them;
- primary-color boxes with no expression;
- single flat billboard that does not read as bipedal;
- high-poly imported characters that break minimum settings performance;
- unlicensed character assets;
- visual-only mock creatures that do not correspond to real simulation entities.

### 5. Texture and material creature pass

Add creature materials/textures that make the bipedal creatures feel alive and natural.

Required:

- body material palette that is softer and more natural than primary colors;
- eye/face material or texture;
- state overlays or shader parameters for emotion/drive expression;
- simple species or lineage tinting if existing data supports it;
- selection/highlight material that does not obscure cute design;
- corpse/sleep/resource interaction visual states where supported.

Creature visuals must remain profile-aware:

- `MinimumSettings30x30`: low material count, simple animation, cheap expressions, no heavy shadows.
- `MinSpecComfort1080p`: default cute model, readable expressions, modest animation.
- `HighSpecScaleUp`: more variation, extra animation/detail allowed.

### 6. Preserve and improve performance gates

This goal is both a visual polish pass and a performance pass.

Required performance validation:

- `MinimumSettings30x30`, 30 real creatures, 30 FPS target;
- `MinSpecComfort1080p`, default population/world settings, comfortable 1080p target;
- population tiers: 1, 10, 30, 50, 100, 250, 500 real creatures where current production configs support them;
- record mesh stats: emitted quads vs naive visible faces, merge ratio, chunk remesh time, active chunks, cached chunks;
- record creature renderer stats: rendered creatures, draw calls or batches where available, material count, animation update cost;
- record selected backend, adapter identity, frame time/FPS, p95 if available, active chunk count, hot/warm/cold brain slot counts, VFX budget, internal render scale if reduced.

If the new art or creature design breaks `MinimumSettings30x30`, fix the implementation before completing. Do not accept performance regression as a known limitation.

### 7. Update production UX and debug visibility

Add UI/diagnostic visibility for the new systems:

- current mesher mode: naive/cull/greedy/equivalent;
- emitted quads and merge ratio;
- remesh queue size and per-frame remesh budget;
- terrain material atlas/palette version;
- creature visual profile and creature mesh/material version;
- profile-driven reductions currently active.

Debug overlays may show material IDs or chunk masks, but production mode should show natural materials by default.

## Required validation commands

Run the standard validation set unless the repo-specific scripts or FVR08 final acceptance changed it:

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

Run production graphical validation or the current FVR08-equivalent commands with at least:

```powershell
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p
```

If the actual production command names differ after FVR08, use the FVR08 final acceptance command names and record the exact commands used.

## Acceptance criteria

FVR09 is complete only when all of these are true:

- production voxel chunks use binary-mask/greedy-quad meshing or a measured equivalent;
- terrain no longer uses primary colors as the production default;
- terrain has natural materials/textures for grass/soil/stone/water-or-wet-ground/resources/hazards/decay as applicable;
- material IDs and asset references preserve save/load and do not serialize renderer handles;
- creatures are visibly cute and bipedal, not two cubes plus a flat plane;
- creature visuals are driven by real creature state and stable IDs;
- creature selection, hover, following, save/load, and inspector state still work;
- `MinimumSettings30x30` meets 30 real creatures at 30 FPS or better on the target hardware class;
- `MinSpecComfort1080p` remains the default comfortable profile;
- higher scale-up profiles remain available;
- external assets have complete license/source/digest manifest entries;
- no mocks, fake backends, fake populations, or visual-only stand-ins were added to product paths;
- `alife_core` remains free of Bevy/wgpu/renderer/asset dependencies;
- final docs record meshing mode, terrain material system, creature design, assets/licenses, performance evidence, and launch commands.

## Required documentation updates

Add or update:

- `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR09_COMPLETION.md`
- asset/license manifest docs or files used by the app;
- README or launch docs if production visuals/settings changed;
- `FINAL_ACCEPTANCE.md` addendum if FVR08 created it;
- any local `AGENTS.md` files if new architecture boundaries or asset rules were introduced.

`FVR09_COMPLETION.md` must include:

```text
Completion receipt
Plan: FVR09 - Greedy Meshing, Natural World Textures, and Cute Bipedal Creature Redesign
Branch:
Files changed:
Deleted/replaced crude visual files:
Mesher implementation:
Terrain material/texture implementation:
Creature mesh/material implementation:
Asset/license changes:
Saved-state/schema changes:
Commands run:
Hardware/GPU evidence:
Performance results:
MinimumSettings30x30 result:
MinSpecComfort1080p result:
Boundary invariants:
Deviations:
Known limitations:
```

`Known limitations` must not include unfinished owned work such as missing terrain textures, crude creature mesh, no greedy meshing, failed minimum floor, or unlicensed assets.

## Failure handling

If greedy meshing breaks selection, fix selection.

If terrain textures break save/load, fix material ID and asset-reference serialization.

If cute bipedal creatures break `MinimumSettings30x30`, reduce creature mesh/material complexity through profiles, batching, instancing, or animation LOD. Do not revert to two-cube creatures as the product default.

If external assets have unclear licenses, remove or replace them before completion.

If `bevy_voxel_world` prevents required meshing/material behavior, add an internal production chunk-mesh backend behind the existing renderer abstraction rather than deferring the improvement.

If performance cannot be measured on local hardware, record that honestly, but still provide the exact commands and do not claim the target was met.
