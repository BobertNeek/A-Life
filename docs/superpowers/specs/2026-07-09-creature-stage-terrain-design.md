# Creature-Stage Terrain Visual Design

Date: 2026-07-09

Status: approved visual direction; implementation is not yet complete

## Goal

Replace the current noisy, flat terrain presentation with an original lush alien
habitat whose visible material richness, environmental depth, and playful
readability meet the quality bar of the Spore Creature Stage reference. Preserve
the existing production simulation, voxel world, save/load path, backend
selection, and renderer-authority boundaries.

Approved visual blueprint:

```text
C:\Users\PC\.codex\generated_images\019f2a54-ead6-76d1-a32a-51fb7a56cc1a\exec-dbc9b242-0643-4c0b-a9a5-83616781b2c6.png
```

The blueprint is a quality and composition target, not a source asset. The
implementation must be original and must not copy proprietary Spore assets,
textures, terrain meshes, shaders, UI, or identifiable set dressing.

## Current Failure

The current renderer does not fail because it lacks enough colors. It fails
because visual detail has no hierarchy:

- `fvr10_append_terrain_surface_detail` distributes two or three small flat
  quads over nearly every sampled cell, producing rectangular confetti.
- `fvr10_append_terrain_side_detail` adds more flat strips without communicating
  roots, strata, erosion, moisture, or material structure.
- material `top_texture` and `side_texture` fields are labels only; no terrain
  texture images are bound to the production materials.
- greedy material prisms read as large rectangular slabs with hard biome seams.
- default comfort lighting uses no shadows, no tonemapping, no atmospheric
  depth, and unlit environmental dressing.
- dressing is uniformly scattered and cuboid-based instead of forming coherent
  ecological clusters around material boundaries and resource/hazard signals.

The old surface-detail and side-detail quad generation will be removed rather
than retained underneath the new layer.

## Approaches Considered

### Palette-only cleanup

Keep the current greedy prisms and replace the bright flecks with subtler vertex
colors. This is low risk, but it cannot produce the approved depth, material
identity, transition quality, water, or ecological composition. Rejected.

### Authored square terrain-tile pack

Replace each logical tile with a pre-authored mesh and texture set. This can look
good in close-up, but the current world scale would expose repeated square tiles,
increase entity and draw-call pressure, and couple visual variety to a large asset
set. Rejected as the primary representation; small authored or generated props
remain acceptable.

### Hybrid voxel-derived terrain renderer

Project the real voxel samples into chunk-local softened terrain meshes, use a
compact set of tileable material maps, render water separately, and place
deterministic biome-aware prop clusters. This keeps logical truth unchanged while
providing the visual hierarchy shown in the blueprint. Selected.

## Architecture

The implementation remains a display-only projection:

```text
alife_world persistent voxel snapshot
    -> renderer-neutral tile/material/resource/hazard samples
    -> alife_game_app terrain visual samples
    -> chunk mesh builder + terrain material library + ecology dressing planner
    -> Bevy mesh/material/light/water entities
```

`alife_world` remains authoritative for terrain samples, object positions,
actions, outcomes, and persistence. `alife_game_app` may derive visual normals,
blend masks, texture coordinates, prop transforms, lighting, and water animation,
but those values never feed back into simulation, cognition, action legality,
reward, learning, or genetics. No Bevy, wgpu, render-pipeline, material, mesh, or
window types may enter `alife_core` or `alife_world`.

No ADR change is required because this design changes only the existing
renderer-owned projection and retains all architecture decisions.

## Module Boundaries

The terrain work must not expand `production_voxel_renderer.rs` into a larger
god object. Move focused responsibilities into app-only modules:

### `production_terrain.rs`

Owns renderer-neutral app-side visual records and orchestration:

- `ProductionTerrainSample`
- `ProductionTerrainProfile`
- `ProductionTerrainSceneSummary`
- conversion from existing tile summaries to terrain visual input
- spawn orchestration and display-only authority markers

It consumes the existing persistent snapshot and does not resample or mutate
world truth.

### `terrain_mesh.rs`

Owns chunk-local mesh generation:

- top surfaces with shared world-space UV scale
- softened/chamfered plateau borders
- explicit cliff and bank side geometry
- deterministic macro color variation
- neighbor-aware material transition skirts
- mesh statistics and bounded vertex/index budgets

The logical tile footprint remains selectable through the existing hidden
`Fvr03ProductionVoxelTerrainTile` entities. The visible mesh may soften the
projection without changing tile identity, height truth, or interaction lookup.

### `terrain_materials.rs`

Owns Bevy material construction and production asset references:

- grass/moss
- worn soil/path
- resource-rich foliage ground
- fungal hazard
- decay/humus
- lichen stone
- sand/bank
- water surface and wet bank

Each opaque ground material uses Bevy 0.18 `StandardMaterial` with a compact
tileable `base_color_texture`, `normal_map_texture`,
`metallic_roughness_texture`, and `occlusion_texture`. Top and cliff/bank
surfaces may use distinct material handles so rooted soil, rock strata, and wet
banks do not share stretched top textures. UVs use world scale, not one `[0,1]`
rectangle per merged prism.

### `terrain_dressing.rs`

Owns deterministic, biome-aware ecological composition:

- grass tufts and low leaf clusters
- broad-leaf plants and flowers
- mushrooms and hazard fungal caps
- reeds near water
- moss/lichen and pebble groups on stone
- sparse dead leaves and humus details in decay zones

Dressing placement uses existing material, resource-bias, hazard-pressure, and
tile coordinates only. It creates clusters with exclusion radii around creatures
and paths instead of uniform per-cell scattering. Repeated meshes share handles
for Bevy instancing. All dressing entities retain display-only/no-authority
markers.

### `terrain_lighting.rs`

Owns camera-adjacent presentation:

- warm directional key light
- cool ambient/sky fill
- low-cost shadows for `MinSpecComfort1080p` and above
- a cheaper non-shadowed fallback for `MinimumSettings30x30`
- production tonemapping and restrained color grading
- subtle distance/height fog for scene separation
- contact grounding for creatures and larger props when shadow quality is off

Lighting may improve visibility but must not hide material boundaries or turn the
default screenshot into a dark cinematic scene.

### `terrain_water.rs`

Owns the display-only water surface:

- a separate translucent surface above the bank mesh
- blue-green depth gradient and restrained specular response
- slow deterministic ripple motion
- reeds and wet-bank dressing at boundaries
- no reflection probe or simulation dependency on the minimum profile

Water animation is visual time only and has no world authority.

## Visual Language

### Macro composition

- Terrain reads first as broad coherent landforms, paths, shelves, pools, and
  biome patches.
- Creatures remain the focal layer; the ground supports them instead of
  competing with their silhouettes.
- Material transitions are irregular but readable. There are no hard debug-color
  rectangles and no per-cell checkerboard.
- Height changes use ledges, banks, roots, moss, and strata to communicate depth.

### Material detail

- Grass uses two or three related moss/leaf tones plus sparse clustered plants.
- Soil reads as compressed warm earth with pebbles and worn path margins.
- Stone uses cooler lichen-gray strata, chipped borders, and moss on selected
  upper edges.
- Resource terrain is greener and denser, but resource props remain individually
  readable.
- Hazard terrain uses dark crimson fungal growth and clustered caps rather than a
  uniformly red floor.
- Decay uses humus, leaf litter, and desaturated olive-brown detail.
- Water uses depth, specular highlights, and bank vegetation rather than opaque
  blue blocks.

### Detail hierarchy

1. broad material and height masses,
2. path, ledge, bank, and biome transitions,
3. clustered medium props,
4. restrained texture and normal-map microdetail.

Microdetail must never recreate the current confetti failure. At normal 1080p
camera distance, no single terrain texel or flat decal should dominate a logical
tile.

## Profiles And Performance

Target hardware remains RTX 3050 8 GB, i7-3770K, Windows 10, 1920x1080.

### `MinimumSettings30x30`

- preserve the existing 30-creature floor and target frame rate
- use the same macro mesh and material identities
- reduce normal-map sampling, shadowing, water layers, and dressing density
- keep paths, biome boundaries, height, and material distinctions intact
- use contact grounding when directional shadows are disabled

### `MinSpecComfort1080p`

- default approved composition target
- enable low-cost directional shadows, normal/roughness detail, clustered
  vegetation, water surface, tonemapping, and atmospheric depth
- maintain the existing production performance recording and honest fallback
  receipt

Higher profiles increase dressing diversity, shadow quality, water quality, and
distance detail without changing material IDs or world data.

## Asset And License Policy

- Terrain assets must be original A-Life-generated assets or permissively
  licensed local assets with committed source, author, license, and URL metadata.
- Do not copy or extract assets from Spore or any other commercial game.
- Keep the terrain material set compact. Do not commit Blender caches, source
  renders, texture-generation dumps, or large intermediate files.
- Final runtime textures and meshes must be referenced by
  `production_asset_manifest.json`, include exact byte size and digest, and use
  package-relative runtime paths.
- Generated blueprint and validation screenshots stay under generated-image or
  `target/artifacts` paths and are not runtime dependencies.

## Error Handling

- Missing mandatory terrain assets cause a clear production-asset validation
  error before the scene claims readiness.
- The minimum profile may use a deterministic generated fallback material only
  when the manifest explicitly allows it and the runtime receipt records it.
- A failed optional water or atmosphere feature degrades to the same world mesh
  without changing simulation state.
- Mesh budget overflow reports the material/chunk and truncates optional dressing
  before it removes navigation or terrain surfaces.

## Testing

Headless tests prove architecture and data invariants, not subjective beauty:

- old confetti surface/side detail generation is absent
- terrain meshes bind real production material assets
- top and side UV density remains world-scaled across differently sized regions
- every visible material has a distinct material identity
- transition geometry is neighbor-aware and deterministic
- dressing is clustered, biome-compatible, and excludes occupied creature tiles
- water is a separate display-only projection
- all terrain, dressing, lighting, and water markers deny action/cognition
  authority
- `alife_core` and `alife_world` remain free of renderer types
- both production profiles stay within explicit mesh/dressing budgets

Visual acceptance requires fresh real-runtime screenshots from both:

```powershell
cargo run -p alife_game_app --features "bevy-app voxel-backend production-assets vfx-hanabi" --bin alife_game_app -j 1 -- production-voxel --profile MinimumSettings30x30 --resolution 1920x1080 --record-performance
cargo run -p alife_game_app --features "bevy-app voxel-backend production-assets vfx-hanabi" --bin alife_game_app -j 1 -- production-voxel --profile MinSpecComfort1080p --resolution 1920x1080 --record-performance
```

The screenshots are compared directly with the approved blueprint for:

- coherent broad material masses,
- no rectangular confetti,
- readable paths and material transitions,
- convincing stone ledges and banks,
- clustered vegetation and fungal hazard dressing,
- water depth and wet-bank separation when water is visible,
- contact grounding and scene depth,
- creature silhouettes remaining visually dominant,
- no diagnostic panels in the default product view.

Passing tests without passing this screenshot review is not completion.

## Validation Gate

Run at minimum:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets -j 1
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

Record the real backend selection, fallback state, save-load status, screenshot
paths, and performance receipt. CPU fallback may be reported honestly for visual
validation, but it is not GPU performance evidence.

## Acceptance

This design is complete only when the production screenshots visibly approach
the approved blueprint at normal viewing size, the minimum profile remains
readable, the comfort profile is measurably richer, all required validation
passes or exact failures are recorded, and renderer visuals remain display-only.
