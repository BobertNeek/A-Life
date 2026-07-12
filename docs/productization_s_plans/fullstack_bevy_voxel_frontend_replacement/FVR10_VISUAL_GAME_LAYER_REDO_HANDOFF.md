# FVR10/FVR11 Visual Game Layer Handoff

Date: 2026-07-11

Status: the FVR11 creature-stage terrain overhaul is implemented and accepted
for this production pass. The earlier flat-color terrain screenshots and the
procedural terrain strategy described by the original FVR10 handoff are
superseded by the evidence below.

## Current Production Result

The production Bevy voxel frontend now combines the existing real simulation,
persistence, and GPU runtime path with a display-only terrain presentation layer:

- compact generated PBR albedo, normal, and ORM atlases for eight terrain
  materials;
- 4x4 subdivided solid-tile tops with deterministic bounded interior relief and
  stable tile edges;
- distinct top, side, transition, and animated water material layers;
- three-segment irregular ecotones instead of full-width rectangular seams;
- wider material-aware hazard/decay blending without broad planar overlays;
- fourteen biome-compatible dressing species, including flowers, reeds,
  lichen rocks, fungi, leaf litter, alien ferns, crimson branching spires, and
  compact glow-bulb rosettes;
- deterministic clusters that avoid creature-occupied tiles;
- nonuniform per-instance silhouette variation and reusable lit prop meshes;
- warm directional light, TonyMcMapface tonemapping, cool ambient fill,
  profile-scaled fog and shadows, and minimum-profile contact grounding;
- restrained Hanabi VFX budgets, with CPU cuboid VFX hidden when GPU particles
  are active.

The minimum profile renders 64 dressing instances and two GPU VFX emitters. The
comfort profile renders 224 dressing instances and four emitters. Higher
profiles retain bounded scale-up caps rather than unbounded prop spawning.

## Accepted Screenshot Evidence

Both screenshots were regenerated from the final release executable at
1920x1080 and inspected at original resolution:

```text
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot.png
D:\A life\target\artifacts\fvr03\MinSpecComfort1080p_runtime_screenshot.png
```

The minimum capture is dated 2026-07-11 11:22:51 local time. It preserves 30
readable creatures, coherent paths and ledges, textured material regions, a
distinct fungal biome, and sparse ecology at the 30 FPS floor.

The comfort capture is dated 2026-07-11 11:23:25 local time. It adds denser
clustered flora, directional shadows, and stronger depth while preserving the
same readable world composition.

The `*_fvr05_*.png` captures in the same artifact directory are supplemental UX
evidence. They are not substitutes for the two required clean runtime images.

## Runtime Receipts

Both final launches exited successfully with empty stderr logs and reported:

- `selected_backend=GpuFull`
- `adapter='NVIDIA GeForce RTX 3050'`
- `backend_api=Vulkan`
- `fallback=None`
- `real_save_loaded=true`
- `mock_data_source=false`
- `voxel_roundtrip=true`
- `gpu_runtime_no_bulk_readback=true`

The fresh renderer diagnostics report:

| Profile | Measured local smoke FPS | Target | Dressing | GPU VFX emitters |
|---|---:|---:|---:|---:|
| `MinimumSettings30x30` | 117.71 | 30 | 64 | 2 |
| `MinSpecComfort1080p` | 154.24 | 60 | 224 | 4 |

These are local smoke measurements on the named machine, not broad hardware or
shipping-performance claims. Source receipts remain under ignored
`target/artifacts/fvr03/` and `target/artifacts/fvr06/`.

## Terrain Asset Manifest

The committed production atlas is compact. Exact regeneration uses
`terrain_material_generation.json`, `terrain_atlas_builder`, and the ignored
source sheet recorded by the generation config. A clean checkout can validate
and ship the committed atlases but cannot regenerate them without supplying an
equivalent permissively licensed source sheet:

| Asset | Bytes | FNV-1a digest |
|---|---:|---|
| `terrain_albedo_atlas.png` | 141823 | `fnv1a64:307543cd881b06f3` |
| `terrain_normal_atlas.png` | 97503 | `fnv1a64:6720600e3e924d51` |
| `terrain_orm_atlas.png` | 80083 | `fnv1a64:6dac4f6ae6d10c42` |

All three are recorded as generated A-Life source under the repository license.
The source sheet and runtime screenshots remain ignored generated artifacts and
must not be committed. Replacement assets must be generated in-project or have
an explicit permissive license, author, source, and replacement policy.

The Quirky Series creature OBJ/PNG derivatives are attributed in
`models/ATTRIBUTION.md` and individually manifested under `CC-BY-4.0`.
`validate-production-assets` reports 59 assets, 19 generated assets, 40 external
creature assets, 4817399 committed bytes, 43 final-art entries, zero unknown
licenses, zero rejected entries, zero final-art placeholders, and a largest file
of 452697 bytes.

## Architecture Boundaries

The terrain and creature renderers remain projections of authoritative world and
save state. They are display-only and cannot authorize actions, bypass motor
arbitration, mutate cognition, assign rewards, rewrite weights, or perform bulk
neural readback.

- `alife_core` remains free of Bevy, wgpu, renderer, mesh, and material types.
- `alife_world` remains the renderer-neutral owner of ecology, persistence, and
  heritable creature appearance data.
- `alife_game_app` owns Bevy meshes, materials, lighting, VFX, screenshots, and
  profile presentation budgets.
- Creature appearance inheritance and mutation remain simulation/persistence
  behavior; rendering only projects those genes.

No architecture decision record changed because this pass alters presentation
implementation, not authority or cross-crate ownership.

## Validation Receipt

The following commands passed on 2026-07-11:

```text
cargo fmt --all -- --check
cargo check -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --all-targets -j 1
cargo check --workspace --all-targets -j 1
cargo test -p alife_game_app --features "bevy-app voxel-backend" --lib terrain_ -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
cargo test -p alife_game_app --test app_shell lifecycle_lineage_birth_inherits_and_mutates_appearance_genes -j 1 -- --nocapture
cargo test -p alife_tools --bin terrain_atlas_builder -j 1 -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
```

The full FVR03 renderer integration suite passed 18 of 18 tests. The focused
terrain library suite passed 23 tests, the atlas builder passed four tests, and
the lineage appearance inheritance/mutation test passed.

## Maintenance Guidance

Preserve the following properties when extending this layer:

- terrain relief may move tile interiors but must preserve shared tile edges;
- transition geometry must remain segmented and materially compatible;
- dressing must remain deterministic, capped per profile, and absent from
  creature-occupied tiles;
- new props need grounded silhouettes rather than broad flat fans or unit-cube
  debug geometry;
- production materials should stay lit and atlas-backed;
- visual tests must be followed by fresh release screenshots and direct image
  inspection;
- target artifacts, source sheets, caches, and large generated files stay out
  of Git.

The current creature meshes are serviceable and architecture-safe but remain
less authored and expressive than the approved terrain treatment. Future
creature art may be replaced independently through the existing app-local
projection layer without changing simulation state, heredity, saves, or GPU
runtime authority.
