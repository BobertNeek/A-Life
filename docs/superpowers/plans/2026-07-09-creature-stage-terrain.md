# Creature-Stage Terrain Implementation Plan

**Status:** Completed and validated on 2026-07-11. See
`docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_GAME_LAYER_REDO_HANDOFF.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the flat, noisy production voxel terrain with the approved lush alien habitat while preserving the real voxel world, save path, backend selection, and renderer authority boundaries.

**Architecture:** Convert the existing persistent tile samples into an app-only terrain sample map, build softened top/cliff/transition/water mesh layers in batches, bind a compact generated PBR atlas through Bevy `StandardMaterial`, and place deterministic biome-aware dressing clusters. Split these responsibilities out of `production_voxel_renderer.rs`; the renderer orchestrates them but never feeds visual state back into the simulation.

**Tech Stack:** Rust 2024 workspace, Bevy `=0.18.0`, wgpu `=27.0.1`, `image = 0.25.10`, `bevy_voxel_world = 0.16.0`, PNG assets, PowerShell validation on Windows 10.

## Global Constraints

- The approved target is `C:\Users\PC\.codex\generated_images\019f2a54-ead6-76d1-a32a-51fb7a56cc1a\exec-dbc9b242-0643-4c0b-a9a5-83616781b2c6.png`.
- Target hardware is RTX 3050 8 GB, i7-3770K, Windows 10, 1920x1080.
- `MinimumSettings30x30` must preserve the existing 30-creature floor and use the same macro terrain identities as the comfort profile.
- `MinSpecComfort1080p` is the default visual acceptance profile.
- No mock simulation, fake backend, renderer-owned action/cognition authority, or renderer types in `alife_core`/`alife_world`.
- Do not add new production work with alpha naming.
- Do not copy Spore assets, shaders, UI, terrain meshes, or textures. The approved image is a quality reference only.
- Every committed production asset remains at most 512 KiB and the manifest total remains at most 8 MiB.
- Generated source sheets, Blender caches, and screenshot artifacts stay outside Git.
- Tests are necessary but do not establish visual acceptance; both fresh production screenshots must be inspected.
- Use `-j 1` for Bevy-heavy checks on this machine.

---

## File Map

Create:

- `crates/alife_game_app/src/production_terrain.rs` - app-only terrain contracts, sample map, roles, and scene summary.
- `crates/alife_game_app/src/terrain_mesh.rs` - deterministic softened terrain mesh generation.
- `crates/alife_game_app/src/terrain_materials.rs` - atlas layout, opaque material specs, and shared asset handles.
- `crates/alife_game_app/src/terrain_water.rs` - translucent water material, marker, and display-only animation.
- `crates/alife_game_app/src/terrain_dressing.rs` - clustered ecological props and reusable low-poly meshes.
- `crates/alife_game_app/src/terrain_lighting.rs` - profile-specific camera atmosphere, ambient fill, sun, and contact-grounding policy.
- `crates/alife_tools/src/bin/terrain_atlas_builder.rs` - developer-only atlas postprocessor.
- `crates/alife_game_app/assets/production_voxel_v1/terrain/terrain_material_generation.json` - committed prompt, slot order, and map-generation values.
- `crates/alife_game_app/assets/production_voxel_v1/terrain/terrain_albedo_atlas.png` - 4x4 top/side material atlas.
- `crates/alife_game_app/assets/production_voxel_v1/terrain/terrain_normal_atlas.png` - matching tangent-space normal atlas.
- `crates/alife_game_app/assets/production_voxel_v1/terrain/terrain_orm_atlas.png` - matching AO/roughness/metallic atlas.

Modify:

- `crates/alife_game_app/src/lib.rs` - feature-gated module wiring and public test contracts.
- `crates/alife_game_app/src/production_voxel_renderer.rs` - remove confetti detail generation and delegate terrain, dressing, water, and lighting.
- `crates/alife_game_app/src/bevy_shell.rs` - initialize required image assets in dry-run tests if needed and keep production asset root unchanged.
- `crates/alife_game_app/Cargo.toml` - enable `bevy/tonemapping_luts` in `bevy-app`.
- `crates/alife_tools/Cargo.toml` - add the existing `image = 0.25.10` PNG dependency for the developer-only atlas builder.
- `crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json` - register generation config and the three runtime atlases with exact size/digest metadata.
- `crates/alife_game_app/tests/fvr03_voxel_renderer.rs` - replace weak color-count checks with terrain layer, material, clustering, and authority checks.
- `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_GAME_LAYER_REDO_HANDOFF.md` - point future work at the accepted terrain implementation and fresh screenshots.

Do not modify:

- `crates/alife_core/**`
- `crates/alife_world/**`
- production save schema or appearance inheritance
- Antigravity's creature OBJ/texture files

---

### Task 1: Introduce The App-Only Terrain Contract

**Files:**

- Create: `crates/alife_game_app/src/production_terrain.rs`
- Modify: `crates/alife_game_app/src/lib.rs:127`
- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs:129`
- Test: `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`

**Interfaces:**

- Consumes: `VoxelTileCoord`, existing `Fvr03ProductionVoxelMaterialKind`, height, resource bias, hazard pressure, and deterministic visual bucket.
- Produces: `ProductionTerrainSampleMap`, `Fvr11TerrainSurfaceRole`, `Fvr11ProductionTerrainLayer`, and `Fvr11ProductionTerrainSceneResource`.

- [x] **Step 1: Write the failing terrain-contract test**

Add these imports and test:

```rust
use alife_game_app::{
    Fvr11ProductionTerrainLayer, Fvr11ProductionTerrainSceneResource,
    Fvr11TerrainSurfaceRole, FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION,
};

#[test]
fn fvr11_terrain_contract_is_display_only() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let scene = app
        .world()
        .resource::<Fvr11ProductionTerrainSceneResource>();
    assert_eq!(scene.visual_version, FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION);
    assert!(scene.sample_count > 0);
    assert!(scene.display_only);
    assert!(scene.no_renderer_authority_over_world_actions_or_cognition);
}
```

- [x] **Step 2: Run the test and verify the missing contract fails compilation**

Run:

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer fvr11_terrain_contract_is_display_only -j 1 -- --nocapture
```

Expected: compile failure naming one or more missing `Fvr11` terrain symbols.

- [x] **Step 3: Add the terrain contract module**

Create the module with these exact public contracts:

```rust
use std::collections::BTreeMap;

use alife_world::VoxelTileCoord;
use bevy::prelude::{Component, Resource};

use crate::Fvr03ProductionVoxelMaterialKind;

pub const FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION: &str =
    "fvr11-lush-creature-stage-terrain-v1";

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProductionTerrainSample {
    pub tile: VoxelTileCoord,
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub center_x: f32,
    pub center_z: f32,
    pub height: f32,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
    pub visual_bucket: u8,
}

pub(crate) type ProductionTerrainSampleMap =
    BTreeMap<VoxelTileCoord, ProductionTerrainSample>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TerrainAtlasUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerrainAtlasLayout {
    pub tile_size: u32,
    pub gutter: u32,
    pub columns: u32,
    pub rows: u32,
}

impl TerrainAtlasLayout {
    pub const PRODUCTION: Self = Self {
        tile_size: 64,
        gutter: 2,
        columns: 4,
        rows: 4,
    };

    pub fn slot_rect(self, slot: u8) -> TerrainAtlasUvRect {
        assert!(u32::from(slot) < self.columns * self.rows);
        let cell = self.tile_size + self.gutter * 2;
        let atlas_width = self.columns * cell;
        let atlas_height = self.rows * cell;
        let column = u32::from(slot) % self.columns;
        let row = u32::from(slot) / self.columns;
        let x = column * cell + self.gutter;
        let y = row * cell + self.gutter;
        TerrainAtlasUvRect {
            min: [x as f32 / atlas_width as f32, y as f32 / atlas_height as f32],
            max: [
                (x + self.tile_size) as f32 / atlas_width as f32,
                (y + self.tile_size) as f32 / atlas_height as f32,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fvr11TerrainSurfaceRole {
    Top,
    Cliff,
    Transition,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr11ProductionTerrainLayer {
    pub role: Fvr11TerrainSurfaceRole,
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub source_tile_count: usize,
    pub display_only: bool,
    pub no_renderer_authority_over_world_actions_or_cognition: bool,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Fvr11ProductionTerrainSceneResource {
    pub visual_version: &'static str,
    pub sample_count: usize,
    pub top_layer_count: usize,
    pub cliff_layer_count: usize,
    pub transition_edge_count: usize,
    pub water_layer_count: usize,
    pub confetti_detail_quad_count: usize,
    pub display_only: bool,
    pub no_renderer_authority_over_world_actions_or_cognition: bool,
}
```

Wire `production_terrain` under `#[cfg(feature = "bevy-app")]` in `lib.rs` and re-export its public contracts.

- [x] **Step 4: Replace the material-bucket vector with one sample map**

In `spawn_fvr03_chunk_tiles`, replace the current
`BTreeMap<MaterialKind, Vec<Fvr03BatchedTerrainTile>>` output with
`ProductionTerrainSampleMap`. Insert each sample by stable `VoxelTileCoord` and
carry resource/hazard values through unchanged. Do not remove hidden tile
entities used for selection.

- [x] **Step 5: Add a temporary contract resource from the existing spawn path**

Until Task 3 replaces the mesh, insert the scene resource with measured existing
counts and `confetti_detail_quad_count` equal to the current generated detail
quad count. The Task 1 test only proves the contract and authority boundary;
Task 3 adds the stricter geometry acceptance assertions.

- [x] **Step 6: Run formatting and the focused test**

Run:

```powershell
cargo fmt --all -- --check
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer fvr11_terrain_contract_is_display_only -j 1 -- --nocapture
```

Expected: the test passes after proving the app-only terrain contract is present,
truthful, populated, and display-only.

- [x] **Step 7: Commit the contract**

```powershell
git add crates/alife_game_app/src/lib.rs crates/alife_game_app/src/production_terrain.rs crates/alife_game_app/src/production_voxel_renderer.rs crates/alife_game_app/tests/fvr03_voxel_renderer.rs
git commit -m "Add production terrain visual contract"
```

---

### Task 2: Build And Register The Compact Terrain Atlas

**Files:**

- Create: `crates/alife_tools/src/bin/terrain_atlas_builder.rs`
- Create: `crates/alife_game_app/assets/production_voxel_v1/terrain/terrain_material_generation.json`
- Create: the three `terrain_*_atlas.png` files listed in the file map
- Modify: `crates/alife_tools/Cargo.toml`
- Modify: `crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json`
- Test: `crates/alife_game_app/src/production_assets.rs`

**Interfaces:**

- Consumes: one uncommitted 4x4 image-generation source sheet and committed generation config.
- Produces: three 272x272 PNG atlases with 16 64x64 slots plus two-pixel gutters.

- [x] **Step 1: Write the failing production-asset test**

Add a unit test in `production_assets.rs`:

```rust
#[test]
fn fvr11_terrain_atlases_are_manifested_and_compact() {
    let path = default_production_asset_manifest_path();
    let manifest: ProductionVoxelAssetManifest =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let required = [
        "terrain-albedo-atlas",
        "terrain-normal-atlas",
        "terrain-orm-atlas",
    ];
    for asset_id in required {
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.asset_id == asset_id)
            .unwrap_or_else(|| panic!("missing {asset_id}"));
        assert!(entry.final_art);
        assert!(!entry.placeholder);
        assert!(entry.generated);
        assert!(!entry.external);
        assert!(entry.size_bytes <= FVR07_MAX_COMMITTED_ASSET_BYTES);
        assert!(entry.local_path.starts_with(
            "crates/alife_game_app/assets/production_voxel_v1/terrain/"
        ));
    }
    validate_production_assets(&path).unwrap();
}
```

- [x] **Step 2: Run the test and verify the missing entries fail**

```powershell
cargo test -p alife_game_app production_assets::tests::fvr11_terrain_atlases_are_manifested_and_compact -j 1 -- --nocapture
```

Expected: failure containing `missing terrain-albedo-atlas`.

- [x] **Step 3: Add the generation config**

The JSON must define this stable row-major slot order:

```json
{
  "schema": "alife.fvr11.terrain_material_generation.v1",
  "tile_size": 64,
  "gutter": 2,
  "grid_columns": 4,
  "grid_rows": 4,
  "source_prompt": "Create a strict 4x4 orthographic swatch sheet of original seamless stylized alien game-terrain materials. Slots in row-major order: moss grass top, rooted grass side, worn soil top, clay soil side, rich resource ground top, rooted resource side, dark crimson fungal hazard top, hazard earth side, humus leaf-litter top, dark decay side, lichen stone top, layered stone side, turquoise shallow water top, wet bank side, warm sand top, compact sand side. Each cell is edge-to-edge material only, flat neutral lighting, no objects crossing cells, no text, no borders, no logos, no watermark.",
  "slots": [
    { "id": "safe-grass-top", "normal_strength": 0.45, "roughness": 0.86 },
    { "id": "safe-grass-side", "normal_strength": 0.55, "roughness": 0.92 },
    { "id": "soil-top", "normal_strength": 0.52, "roughness": 0.94 },
    { "id": "soil-side", "normal_strength": 0.62, "roughness": 0.96 },
    { "id": "resource-top", "normal_strength": 0.50, "roughness": 0.78 },
    { "id": "resource-side", "normal_strength": 0.56, "roughness": 0.86 },
    { "id": "hazard-top", "normal_strength": 0.58, "roughness": 0.72 },
    { "id": "hazard-side", "normal_strength": 0.64, "roughness": 0.82 },
    { "id": "decay-top", "normal_strength": 0.48, "roughness": 0.90 },
    { "id": "decay-side", "normal_strength": 0.54, "roughness": 0.94 },
    { "id": "stone-top", "normal_strength": 0.72, "roughness": 0.92 },
    { "id": "stone-side", "normal_strength": 0.84, "roughness": 0.96 },
    { "id": "water-top", "normal_strength": 0.30, "roughness": 0.18 },
    { "id": "water-side", "normal_strength": 0.36, "roughness": 0.26 },
    { "id": "sand-top", "normal_strength": 0.46, "roughness": 0.88 },
    { "id": "sand-side", "normal_strength": 0.52, "roughness": 0.92 }
  ]
}
```

- [x] **Step 4: Implement the developer-only atlas builder**

Add `image = { version = "0.25.10", default-features = false, features = ["png"] }`
to `alife_tools`. The binary accepts exactly three positional arguments in this
order: source sheet PNG path, generation config JSON path, and output directory
path.

Its algorithm is deterministic:

1. crop the source into the configured 4x4 grid,
2. resize every crop to 64x64 with Lanczos3,
3. blend opposite edges across eight pixels so each tile wraps,
4. place each tile in row-major order with a two-pixel extruded gutter,
5. derive tangent-space normals from wrapped luminance central differences,
6. pack AO in R, configured roughness with luminance variation in G, and zero metallic in B,
7. write the three named PNG files and print exact byte sizes.

Use these exact helpers so later tasks can test the algorithm independently:

```rust
fn make_wrapped_tile(source: &image::RgbaImage, size: u32, blend: u32) -> image::RgbaImage;
fn derive_normal_map(source: &image::RgbaImage, strength: f32) -> image::RgbaImage;
fn derive_orm_map(source: &image::RgbaImage, roughness: f32) -> image::RgbaImage;
fn extrude_into_atlas(
    atlas: &mut image::RgbaImage,
    tile: &image::RgbaImage,
    column: u32,
    row: u32,
    gutter: u32,
);
```

- [x] **Step 5: Generate the source sheet with the image-generation skill**

Use the committed `source_prompt`, the approved terrain blueprint as the style
reference, and the built-in image generation path. Treat the generated sheet as
an intermediate; do not commit it. Set `ALIFE_TERRAIN_SOURCE_SHEET` to the exact
path returned by the image tool, then copy it to the stable ignored path:

```powershell
$sourceSheet = $env:ALIFE_TERRAIN_SOURCE_SHEET
if (-not $sourceSheet -or -not (Test-Path -LiteralPath $sourceSheet)) {
    throw "ALIFE_TERRAIN_SOURCE_SHEET must name the image-generation output"
}
$generatedDir = "target\generated_art\production_voxel_v1"
New-Item -ItemType Directory -Force -Path $generatedDir | Out-Null
Copy-Item -LiteralPath $sourceSheet -Destination "$generatedDir\terrain_source_sheet.png"
```

- [x] **Step 6: Build the runtime atlases**

Run from the workspace root:

```powershell
cargo run -p alife_tools --bin terrain_atlas_builder -- "target\generated_art\production_voxel_v1\terrain_source_sheet.png" "crates\alife_game_app\assets\production_voxel_v1\terrain\terrain_material_generation.json" "crates\alife_game_app\assets\production_voxel_v1\terrain"
```

Expected: three 272x272 PNG files, each nonempty and no larger than 262144 bytes.

- [x] **Step 7: Register exact manifest metadata**

Add one manifest entry per atlas. Use `usage_category` `terrain-materials`,
license `A-Life-Generated-Source`, `source`
`generated:openai-imagegen-and-deterministic-atlas-builder`, `final_art: true`,
`placeholder: false`, and generator config path
`crates/alife_game_app/assets/production_voxel_v1/terrain/terrain_material_generation.json`.

Compute each `fnv1a64` digest using `PortableAssetDigest::for_file` in a small
temporary unit test or a one-shot extension to the atlas builder's printed
receipt. Copy the exact digest and byte count into the manifest, then remove any
temporary test.

- [x] **Step 8: Run the asset tests**

```powershell
cargo test -p alife_game_app production_assets::tests::fvr11_terrain_atlases_are_manifested_and_compact -j 1 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
```

Expected: both commands pass; the validation receipt reports `unknown_license=0`,
`rejected=0`, and `no_large_artifacts=true`.

- [x] **Step 9: Commit the atlas pipeline and final assets**

```powershell
git add crates/alife_tools/Cargo.toml crates/alife_tools/src/bin/terrain_atlas_builder.rs crates/alife_game_app/assets/production_voxel_v1/terrain crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json crates/alife_game_app/src/production_assets.rs Cargo.lock
git commit -m "Add compact production terrain material atlas"
```

---

### Task 3: Replace Greedy Slabs And Confetti With Layered Terrain Meshes

**Files:**

- Create: `crates/alife_game_app/src/terrain_mesh.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs:1818`
- Test: `crates/alife_game_app/src/terrain_mesh.rs`
- Test: `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`

**Interfaces:**

- Consumes: `&ProductionTerrainSampleMap`, tile stride, and `TerrainAtlasLayout`.
- Produces: `TerrainMeshBuild` containing batched top, cliff, transition, and water layers plus measured statistics.

- [x] **Step 1: Write failing pure mesh-builder tests**

Test a fixed 3x3 sample map containing grass, soil, stone, and water. Assert:

```rust
assert_eq!(first.stats, second.stats);
let first_receipts = first
    .layers
    .iter()
    .map(|layer| (layer.role, layer.material, layer.source_tile_count))
    .collect::<Vec<_>>();
let second_receipts = second
    .layers
    .iter()
    .map(|layer| (layer.role, layer.material, layer.source_tile_count))
    .collect::<Vec<_>>();
assert_eq!(first_receipts, second_receipts);
assert!(first.layers.iter().any(|layer| layer.role == Fvr11TerrainSurfaceRole::Top));
assert!(first.layers.iter().any(|layer| layer.role == Fvr11TerrainSurfaceRole::Cliff));
assert!(first.layers.iter().any(|layer| layer.role == Fvr11TerrainSurfaceRole::Transition));
assert!(first.layers.iter().any(|layer| layer.role == Fvr11TerrainSurfaceRole::Water));
assert_eq!(first.stats.confetti_detail_quads, 0);
assert!(first.stats.transition_edges > 0);
assert!(first.stats.max_vertices_per_source_tile <= 40);
```

Also inspect every generated mesh for `POSITION`, `NORMAL`, `UV_0`, `TANGENT`,
and `COLOR` attributes.

Add the failing integration test `fvr11_terrain_contract_is_display_only_and_layered`
at this task, after the contract-only test is already green. It must assert:

```rust
assert_eq!(scene.confetti_detail_quad_count, 0);
assert!(scene.top_layer_count >= 7);
assert!(scene.cliff_layer_count >= 3);
assert!(scene.transition_edge_count > 0);

let mut query = app.world_mut().query::<&Fvr11ProductionTerrainLayer>();
let roles = query
    .iter(app.world())
    .map(|layer| layer.role)
    .collect::<BTreeSet<_>>();
assert!(roles.contains(&Fvr11TerrainSurfaceRole::Top));
assert!(roles.contains(&Fvr11TerrainSurfaceRole::Cliff));
assert!(roles.contains(&Fvr11TerrainSurfaceRole::Transition));
assert!(roles.contains(&Fvr11TerrainSurfaceRole::Water));
```

- [x] **Step 2: Run the tests and verify the builder is missing**

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" terrain_mesh::tests -j 1 -- --nocapture
```

Expected: compile failure for missing `terrain_mesh` interfaces.

- [x] **Step 3: Implement the mesh data types**

Use these exact app-only types:

```rust
pub(crate) struct TerrainMeshLayer {
    pub role: Fvr11TerrainSurfaceRole,
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub mesh: Mesh,
    pub source_tile_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TerrainMeshStats {
    pub source_tiles: usize,
    pub top_quads: usize,
    pub cliff_quads: usize,
    pub transition_edges: usize,
    pub water_quads: usize,
    pub confetti_detail_quads: usize,
    pub max_vertices_per_source_tile: usize,
}

pub(crate) struct TerrainMeshBuild {
    pub layers: Vec<TerrainMeshLayer>,
    pub stats: TerrainMeshStats,
}
```

- [x] **Step 4: Implement neighbor-aware top geometry**

For each sample, calculate four corner heights from the average of adjacent
non-water sample heights. Clamp smoothing to `0.20` world units so logical
height bands remain legible. Use central differences for corner normals.

Emit one top quad per sample into a batch keyed by material. Map local tile UVs
into the correct atlas slot; deterministically rotate or mirror the UV corners
from `visual_bucket` without leaving the slot gutter. Vertex color varies only
within +/-6% of the material tint.

- [x] **Step 5: Implement cliffs and transition rims**

For every east/south boundary, emit a cliff quad when the height difference is
at least `0.24`. Use the higher tile's side atlas slot and flat outward normals.

For material changes with height difference below `0.24`, emit a narrow
`0.12 * tile_stride` transition strip on the dominant tile. Grass-to-stone uses
the grass-side slot, grass-to-soil uses the soil-side slot, and water boundaries
use the wet-bank slot. Never emit both directions for one edge.

- [x] **Step 6: Implement water as a separate surface layer**

Water top quads sit `0.035` world units above their source height, use the water
top slot, and do not emit opaque top geometry for the same sample. Wet-bank
cliffs still cover the sides.

- [x] **Step 7: Generate tangents and finish each batch**

Insert positions, normals, UVs, colors, and indices, then call:

```rust
mesh.generate_tangents()
    .expect("FVR11 terrain mesh needs valid tangents for normal mapping");
```

Return no empty layers. Keep each material/role in one mesh so repeated tiles do
not become individual Bevy entities.

- [x] **Step 8: Replace the old terrain spawn path**

Delete these functions and their calls from `production_voxel_renderer.rs`:

```text
fvr09_material_greedy_prisms
fvr10_tiles_can_merge
fvr10_append_colored_terrain_cuboid
fvr10_append_terrain_surface_detail
fvr10_append_terrain_side_detail
fvr10_terrain_detail_color
fvr10_terrain_side_detail_color
```

Retain any hash/palette helper still used elsewhere. Spawn each returned layer
with `Fvr11ProductionTerrainLayer`. Keep one
`Fvr03ProductionVoxelTerrainBatch` marker per material's top layer for backward
compatibility with existing diagnostics.

- [x] **Step 9: Run the pure and integration terrain tests**

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" terrain_mesh::tests -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer fvr11_terrain_contract_is_display_only_and_layered -j 1 -- --nocapture
```

Expected: both pass, including `confetti_detail_quad_count == 0`.

- [x] **Step 10: Commit the mesh replacement**

```powershell
git add crates/alife_game_app/src/lib.rs crates/alife_game_app/src/terrain_mesh.rs crates/alife_game_app/src/production_voxel_renderer.rs crates/alife_game_app/tests/fvr03_voxel_renderer.rs
git commit -m "Replace noisy terrain slabs with layered chunk meshes"
```

---

### Task 4: Bind PBR Terrain Materials And Animated Water

**Files:**

- Create: `crates/alife_game_app/src/terrain_materials.rs`
- Create: `crates/alife_game_app/src/terrain_water.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs`
- Test: `crates/alife_game_app/src/terrain_materials.rs`
- Test: `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`

**Interfaces:**

- Consumes: production atlas asset paths and `Fvr03ProductionVoxelMaterialKind`.
- Produces: `TerrainMaterialLibrary`, `TerrainAtlasLayout`, `Fvr11ProductionTerrainMaterialContract`, and `Fvr11AnimatedWaterMaterial`.

- [x] **Step 1: Write failing material-spec tests**

Assert all eight terrain materials have top and side atlas slots, all slots are
unique, and all runtime map paths are package-relative:

```rust
let specs = production_terrain_material_specs();
assert_eq!(specs.len(), 8);
assert_eq!(specs.iter().map(|spec| spec.top_slot).collect::<BTreeSet<_>>().len(), 8);
assert_eq!(specs.iter().map(|spec| spec.side_slot).collect::<BTreeSet<_>>().len(), 8);
for spec in specs {
    assert!(spec.base_color_path.starts_with("production_voxel_v1/terrain/"));
    assert!(spec.normal_path.starts_with("production_voxel_v1/terrain/"));
    assert!(spec.orm_path.starts_with("production_voxel_v1/terrain/"));
}
```

- [x] **Step 2: Implement atlas and material specifications**

Define:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProductionTerrainMaterialSpec {
    pub kind: Fvr03ProductionVoxelMaterialKind,
    pub top_slot: u8,
    pub side_slot: u8,
    pub base_tint: [f32; 4],
    pub perceptual_roughness: f32,
    pub base_color_path: &'static str,
    pub normal_path: &'static str,
    pub orm_path: &'static str,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Fvr11ProductionTerrainMaterialContract {
    pub material_count: usize,
    pub atlas_dimensions: [u32; 2],
    pub base_color_path: &'static str,
    pub normal_path: &'static str,
    pub orm_path: &'static str,
    pub real_assets_requested: bool,
    pub display_only: bool,
}

pub(crate) struct TerrainMaterialLibrary {
    pub top: BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    pub side: BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    pub transition: BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    pub water: Handle<StandardMaterial>,
}
```

- [x] **Step 3: Load production atlas handles**

When `AssetServer` exists, request the three paths once and share those handles
across all material instances. When dry-run tests omit `AssetServer`, create
untextured fallback materials but set `real_assets_requested=false`; never claim
textures loaded in a dry run.

Opaque top/side materials use:

```rust
StandardMaterial {
    base_color: Color::srgba(tint[0], tint[1], tint[2], tint[3]),
    base_color_texture: albedo.clone(),
    normal_map_texture: normal.clone(),
    metallic_roughness_texture: orm.clone(),
    occlusion_texture: orm.clone(),
    perceptual_roughness: spec.perceptual_roughness,
    metallic: 0.0,
    unlit: false,
    ..default()
}
```

- [x] **Step 4: Add the water module, material, and animation marker**

Use `AlphaMode::Blend`, `perceptual_roughness: 0.18`, `reflectance: 0.42`,
`clearcoat: 0.35`, `clearcoat_perceptual_roughness: 0.12`, and `cull_mode: None`.
Create `terrain_water.rs` and add this exact resource:

```rust
#[derive(Debug, Clone, Resource)]
pub(crate) struct Fvr11AnimatedWaterMaterial {
    pub handle: Handle<StandardMaterial>,
    pub phase: f32,
}
```

Update only its `uv_transform.translation` plus a +/-2% blue-green tint pulse in
an `Update` system. The system must not read or write world simulation state.

- [x] **Step 5: Replace white vertex-color-only terrain materials**

Use the top, side, transition, and water handle selected by each mesh layer. Keep
vertex colors as restrained macro tint modulation; texture maps carry microdetail.

- [x] **Step 6: Strengthen the integration test**

In dry-run integration tests, assert the contract reports eight materials and
three exact asset paths. In the real screenshot run, inspect Bevy asset-load logs
and fail completion if any terrain atlas produces a missing-asset warning.

- [x] **Step 7: Run tests and commit**

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" terrain_materials::tests -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" terrain_water::tests -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer fvr11_terrain_contract_is_display_only_and_layered -j 1 -- --nocapture
git add crates/alife_game_app/src/lib.rs crates/alife_game_app/src/terrain_materials.rs crates/alife_game_app/src/terrain_water.rs crates/alife_game_app/src/production_voxel_renderer.rs crates/alife_game_app/tests/fvr03_voxel_renderer.rs
git commit -m "Bind production PBR terrain materials and water"
```

---

### Task 5: Replace Uniform Cuboid Dressing With Biome Clusters

**Files:**

- Create: `crates/alife_game_app/src/terrain_dressing.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs:890`
- Test: `crates/alife_game_app/src/terrain_dressing.rs`
- Test: `crates/alife_game_app/tests/fvr03_voxel_renderer.rs:729`

**Interfaces:**

- Consumes: `ProductionTerrainSampleMap`, creature occupied tiles, profile dressing cap.
- Produces: deterministic `ProductionTerrainDressingSpawn` records, shared mesh/material libraries, and existing display-only dressing markers.

- [x] **Step 1: Write failing clustering tests**

Test a fixed biome map and occupied-tile set. Assert:

```rust
assert_eq!(first, second);
assert!(first.iter().all(|spawn| !occupied.contains(&spawn.tile)));
assert!(first.iter().all(|spawn| spawn.display_only));
assert!(first.iter().all(|spawn| spawn.no_renderer_authority_over_actions_or_cognition));
assert!(first.iter().any(|spawn| spawn.kind == Fvr07ProductionDressingKind::ReedCluster));
assert!(first.iter().any(|spawn| spawn.kind == Fvr07ProductionDressingKind::HazardFungus));
assert!(first.iter().any(|spawn| spawn.kind == Fvr07ProductionDressingKind::LichenRock));
```

- [x] **Step 2: Extend dressing kinds without breaking stable existing names**

Keep `LeafPatch`, `MushroomCluster`, `PebbleCluster`, `NestMarker`,
`FoodResource`, and `CorpseMarker`. Add:

```rust
FlowerPatch,
ReedCluster,
LichenRock,
HazardFungus,
DeadLeafPatch,
```

Use this exact internal spawn record:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProductionTerrainDressingSpawn {
    pub kind: Fvr07ProductionDressingKind,
    pub tile: VoxelTileCoord,
    pub translation: Vec3,
    pub scale: Vec3,
    pub yaw_radians: f32,
    pub cluster_id: u32,
    pub display_only: bool,
    pub no_renderer_authority_over_actions_or_cognition: bool,
}
```

- [x] **Step 3: Implement deterministic cluster planning**

Select anchors from material-compatible tiles using coordinate hash and local
resource/hazard values. Enforce:

- no occupied creature tile,
- no more than one cluster anchor per 2x2 sampled-tile neighborhood,
- grass/resource clusters contain 2-5 child spawns,
- stone clusters contain 1-3 rocks/lichen props,
- water edges contain 2-4 reed spawns,
- hazard clusters contain 2-5 dark fungal props,
- soil paths receive at most sparse pebble/leaf edge dressing,
- total entities never exceed the profile's existing dressing cap.

- [x] **Step 4: Build reusable low-poly prop meshes**

Implement shared mesh handles for tapered grass blades, radial broad leaves,
five-petal flowers, faceted mushroom caps, reeds, irregular lichen rocks, fungal
caps, and leaf-litter clusters. Use triangles/tapered prisms, not unit cubes.
Insert vertex colors and valid normals. Reuse handles so Bevy can instance
repeated props.

- [x] **Step 5: Make prop materials lit**

Replace `unlit: true` dressing materials with rough lit materials. Use a white
base color when vertex colors provide the palette; set roughness between 0.68
and 0.92. Hazard fungus may use restrained emissive red no brighter than 0.08.

- [x] **Step 6: Remove props spawned directly on creature tiles**

Delete the current creature-index loop that places nests, food, leaves, or
mushrooms on the exact creature tile. Select the nearest compatible unoccupied
neighbor instead. This preserves visual grounding and prevents prop/creature
occlusion.

- [x] **Step 7: Update the old visual-dressing test**

Replace the assertion that hero materials are unlit with assertions that at
least 24 composite meshes have more than 24 vertices, at least 12 are upright,
all hero materials are lit, and no dressing marker shares a tile with a creature
marker.

- [x] **Step 8: Run tests and commit**

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" terrain_dressing::tests -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer fvr10_scene_dressing_uses_composite_vertical_props_not_unit_debug_cubes -j 1 -- --nocapture
git add crates/alife_game_app/src/lib.rs crates/alife_game_app/src/terrain_dressing.rs crates/alife_game_app/src/production_voxel_renderer.rs crates/alife_game_app/tests/fvr03_voxel_renderer.rs
git commit -m "Add clustered biome-aware terrain dressing"
```

---

### Task 6: Add Creature-Stage Lighting, Atmosphere, And Grounding

**Files:**

- Create: `crates/alife_game_app/src/terrain_lighting.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs:5247`
- Modify: `crates/alife_game_app/Cargo.toml`
- Test: `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`

**Interfaces:**

- Consumes: `Fvr03ProductionVoxelRendererSettings` profile flags.
- Produces: camera atmosphere bundle, sun configuration, and `Fvr11ProductionTerrainLightingMarker` evidence.

- [x] **Step 1: Write the failing profile-lighting test**

Build minimum and comfort dry-run apps and query the camera/light markers:

```rust
assert_eq!(minimum.tonemapping, "tony-mc-mapface");
assert!(!minimum.directional_shadows);
assert!(minimum.contact_grounding);
assert!(comfort.directional_shadows);
assert_eq!(comfort.shadow_cascades, 2);
assert!(comfort.distance_fog);
assert!(comfort.cool_ambient_fill);
assert!(comfort.display_only);
assert!(comfort.no_renderer_authority_over_world_actions_or_cognition);
```

- [x] **Step 2: Enable Bevy's production tonemapping LUTs**

Add `"bevy/tonemapping_luts"` to the `bevy-app` feature list. Do not change the
pinned Bevy version.

- [x] **Step 3: Implement the camera atmosphere bundle**

Replace `Tonemapping::None` with `Tonemapping::TonyMcMapface`. Add camera-local:

```rust
AmbientLight {
    color: Color::srgb(0.56, 0.68, 0.78),
    brightness: 105.0,
    affects_lightmapped_meshes: true,
}
```

For comfort and higher profiles, add:

```rust
DistanceFog {
    color: Color::srgba(0.18, 0.30, 0.28, 0.30),
    directional_light_color: Color::srgba(0.95, 0.78, 0.52, 0.22),
    directional_light_exponent: 18.0,
    falloff: FogFalloff::Linear { start: 34.0, end: 88.0 },
}
```

Minimum uses a weaker alpha of 0.12 and starts at 42.0.

- [x] **Step 4: Configure the warm sun and comfort shadows**

Use illuminance `7600.0`, warm color `Color::srgb(1.0, 0.91, 0.74)`, and the
existing direction. Enable shadows for comfort and higher, but not minimum.
Attach:

```rust
CascadeShadowConfigBuilder {
    num_cascades: 2,
    minimum_distance: 0.1,
    maximum_distance: 90.0,
    first_cascade_far_bound: 28.0,
    overlap_proportion: 0.18,
}
.build()
```

- [x] **Step 5: Add minimum-profile contact grounding**

When directional shadows are disabled, spawn one shared soft circular shadow mesh
under each creature and large dressing anchor. Use a dark green-brown transparent
material, `AlphaMode::Blend`, `unlit: true`, and no pickability. Mark every entity
display-only. Keep the diameter below 70% of one sampled tile so shadows do not
become grid overlays.

- [x] **Step 6: Run tests and commit**

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer fvr11_profile_lighting_preserves_minimum_floor_and_comfort_depth -j 1 -- --nocapture
cargo check -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --all-targets -j 1
git add crates/alife_game_app/Cargo.toml crates/alife_game_app/src/lib.rs crates/alife_game_app/src/terrain_lighting.rs crates/alife_game_app/src/production_voxel_renderer.rs crates/alife_game_app/tests/fvr03_voxel_renderer.rs Cargo.lock
git commit -m "Add production terrain lighting and atmosphere"
```

---

### Task 7: Run The Real Screenshot Loop And Finish Visual Polish

**Files:**

- Modify: `crates/alife_game_app/src/production_terrain.rs`
- Modify: `crates/alife_game_app/src/terrain_mesh.rs`
- Modify: `crates/alife_game_app/src/terrain_materials.rs`
- Modify: `crates/alife_game_app/src/terrain_water.rs`
- Modify: `crates/alife_game_app/src/terrain_dressing.rs`
- Modify: `crates/alife_game_app/src/terrain_lighting.rs`
- Modify: `crates/alife_game_app/assets/production_voxel_v1/terrain/terrain_material_generation.json`
- Modify only when the screenshot comparison identifies a material-map defect: the three committed terrain atlas PNG files.
- Modify: `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_GAME_LAYER_REDO_HANDOFF.md`
- Generate only under `target/artifacts/fvr03/`: required runtime screenshots and performance receipts.

**Interfaces:**

- Consumes: the full production launch path, real save, backend selection, approved visual blueprint.
- Produces: fresh minimum/comfort screenshots, honest backend/performance receipts, and final validation evidence.

- [x] **Step 1: Run focused code validation before launching**

```powershell
cargo fmt --all -- --check
cargo test -p alife_game_app --features "bevy-app voxel-backend" terrain_mesh::tests -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" terrain_materials::tests -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" terrain_water::tests -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" terrain_dressing::tests -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
```

Expected: all pass before visual acceptance work begins.

- [x] **Step 2: Build the release executable once**

```powershell
cargo build -p alife_game_app --release --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -j 1
```

Expected: `target\release\alife_game_app.exe` is updated successfully.

- [x] **Step 3: Capture the minimum profile**

```powershell
target\release\alife_game_app.exe production-voxel --profile MinimumSettings30x30 --population 30 --resolution 1920x1080 --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan --record-performance
```

Inspect with Computer Use and `view_image`:

```text
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot.png
```

Reject the result if it has rectangular confetti, hard debug-color mats,
featureless slabs, missing atlas textures, floating creatures, unreadable paths,
or terrain clutter obscuring creatures.

- [x] **Step 4: Capture the comfort profile**

```powershell
target\release\alife_game_app.exe production-voxel --profile MinSpecComfort1080p --resolution 1920x1080 --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan --record-performance
```

Inspect:

```text
D:\A life\target\artifacts\fvr03\MinSpecComfort1080p_runtime_screenshot.png
```

Compare side-by-side with the approved blueprint. Require coherent broad
materials, readable paths, mossy ledges, clustered flora, a distinct fungal
hazard, water depth when visible, contact grounding, and stronger depth than the
minimum profile.

- [x] **Step 5: Iterate against visible discrepancies**

Change one visual variable group at a time in this order:

1. camera framing and terrain scale,
2. macro palette and top/side separation,
3. cliff/transition geometry,
4. lighting and fog,
5. dressing density and silhouette,
6. texture contrast and water response.

After each change, rebuild the release binary, recapture both required profiles,
and compare again. Do not use passing tests as a reason to stop a visibly weak
iteration.

- [x] **Step 6: Run the full required validation gate**

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets -j 1
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

Expected: all pass. If any command is unavailable or fails, preserve the exact
command and error; do not claim a pass.

- [x] **Step 7: Audit real runtime receipts**

Confirm both profile receipts record:

- `real_save_loaded=true`
- `mock_data_source=false`
- `voxel_roundtrip=true`
- actual `selected_backend`, `backend_api`, and `fallback`
- actual frame-time/performance values
- no missing terrain assets

CPU fallback may be reported for visual validation but is not GPU performance
evidence.

- [x] **Step 8: Update the visual handoff**

Record the accepted terrain strategy, exact screenshot paths, backend receipts,
validation results, remaining creature-quality caveat, and the explicit statement
that the renderer remains display-only.

- [x] **Step 9: Commit the accepted terrain pass**

```powershell
git add crates/alife_game_app crates/alife_tools Cargo.lock docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_GAME_LAYER_REDO_HANDOFF.md docs/superpowers
git status --short
git commit -m "Complete creature-stage terrain visual overhaul"
```

Before committing, inspect `git status --short` and unstage any generated
`target/` artifacts or unrelated files.

- [x] **Step 10: Request review, push, and integrate safely**

Use `superpowers:requesting-code-review`, fix actionable findings, rerun the
validation gate, then:

```powershell
git fetch origin
git rebase origin/main
git push -u origin codex/FVR11-creature-stage-terrain
git switch main
git pull --ff-only origin main
git merge --no-ff codex/FVR11-creature-stage-terrain -m "Merge creature-stage terrain visual overhaul"
git push origin main
```

If `origin/main` or the feature branch contains parallel work, inspect and
integrate it; never force-push or wipe unrelated work.

---

## Completion Audit

The goal is not complete until all of these are evidenced together:

- the approved mockup remains the visual comparison target,
- old confetti functions and runtime detail quads are gone,
- terrain uses real compact atlas assets with valid manifest metadata,
- visible geometry has coherent top/cliff/transition/water layers,
- dressing is clustered and biome-compatible,
- minimum and comfort profiles both render correctly at 1920x1080,
- comfort is visibly richer while minimum stays readable,
- fresh screenshots have been inspected through Computer Use and `view_image`,
- real save/backend/fallback receipts are honest,
- all required validation commands pass or exact failures are reported,
- `alife_core` and `alife_world` remain renderer-free,
- no generated source sheets or screenshot artifacts are committed,
- the feature branch is reviewed, pushed, and merged without erasing parallel work.
