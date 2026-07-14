# FVR10/FVR11 Visual Game Layer Handoff

Date: 2026-07-14

Status: the FVR11 creature-stage terrain overhaul and the heritable modular
creature assembly pass are implemented and accepted. The earlier flat-color
terrain and whole-animal creature meshes are superseded by the evidence below.

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

## Modular Creature Assembly

Creature appearance save schema v2 adds five stable family sources: head,
torso, arms, legs, and tail/back. Schema-v1 saves migrate all five sources from
the former coherent species value. Schema-v2 roundtrips preserve each source,
founders remain coherent, offspring inherit per slot, and bounded catalog-aware
mutation may alter compatible parts. This data stays renderer-neutral in
`alife_world`.

The append-only catalog currently reserves these stable IDs:

| ID | Family | Compatibility |
|---:|---|---|
| 0 | `colobus` | mammalian, long-arm, plume-tail |
| 1 | `gecko` | compact, scaled, long-tail |
| 2 | `herring` | aquatic, fin-arm, tailless |
| 3 | `inkfish` | aquatic, tentacle-arm, soft-body |
| 4 | `muskrat` | mammalian, compact, aquatic |
| 5 | `pudu` | mammalian, heavy-torso, short-tail |
| 6 | `sparrow` | compact, wing-arm, plume-tail |
| 7 | `taipan` | scaled, long-body, tailless |

New families are data additions with new IDs; existing IDs must never be
renumbered or repurposed. A synthetic ninth-family test proves that catalog
growth requires no renderer match arm.

The deterministic builder owns source normalization, structured OBJ parsing,
cut-plane triangle partitioning, UV/normal interpolation, socket-local
rebasing, output validation, and staging-before-copy behavior. Boundary-
crossing triangles are clipped into complete surface fragments instead of
being stretched across an anatomical part envelope. Each original source
triangle retains one deterministic primary provenance owner even when its
surface fragments render in adjacent slots; the builder separately records the
complete fragment-slot set and validates that the primary owner belongs to it.
Runtime loading then fits each UV-preserving part into an upright anatomical
envelope while preserving the authored socket-local origin for every attached
part, and assembles a Bevy hierarchy:

```text
ProductionCreatureAssemblyRoot (stable ID, selection, animation, display-only)
  ProductionCreaturePartMarker x7 (head, torso, paired arms/legs, tail/back)
  ProductionCreatureJoinCoverMarker x6 (textured ruffs/tufts/cuffs)
  layered sclera/iris/pupil/highlight face children (non-impostor LOD)
  hidden affordance marker
```

Meshes are cached by `(family, lod, slot)` and materials by family/palette/
pattern/expression bucket. Thirty creatures therefore reuse 56 part mesh
handles instead of allocating per-creature copies. The renderer does not own
appearance genes, mutation, actions, cognition, or world authority.

Exact maintenance commands are:

```text
cargo run -p alife_tools --bin creature_part_builder -- analyze --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json --family <id> --lod compact --json target/artifacts/creature_parts/<family>_analysis.json
cargo run -p alife_tools --bin creature_part_builder -- build --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json --family <id> --staging target/generated_art/creature_parts/staging
cargo run -p alife_tools --bin creature_part_builder -- preview --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json --family <id> --lod compact --output target/artifacts/creature_parts/<family>_compact.png
cargo run -p alife_tools --bin creature_part_builder -- validate --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json
```

The committed generated pack contains 24 named-part OBJ files (6,377,739
bytes) and 24 socket manifests (38,552 bytes); the largest generated OBJ is
498,351 bytes, below the enforced 512 KiB per-file limit. OBJ coordinates,
UVs, and normals use deterministic six-decimal text precision to keep clipped
surface fragments compact without visible-scale quantization. All 24 LOD
packs validate. Whole source OBJs live under the
developer-source directory and are excluded from runtime packaging. Generated
parts retain Omabuarts Quirky Series source UVs and `CC-BY-4.0` attribution;
textures remain manifested production dependencies. Source and generated
records include origin, author, digest, generation status, and replacement
policy. Generated manifest asset IDs use append-only numeric family IDs, so a
display-label rename cannot change package identity. Unknown-license, rejected,
and placeholder-final counts are zero.

## Accepted Screenshot Evidence

Both screenshots were regenerated from the final release executable at
1920x1080 and inspected at original resolution:

```text
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot.png
D:\A life\target\artifacts\fvr03\MinSpecComfort1080p_runtime_screenshot.png
```

The minimum capture is dated 2026-07-14 06:31:09 local time. It preserves 30
textured modular creatures across active and prone simulation poses, coherent
paths and ledges, textured material regions, a distinct fungal biome, and
sparse ecology at the 30 FPS floor. Upright creatures have bounded torsos and
layered sclera/iris/pupil eyes; the former oversized triangle sheets and black
bead eyes are absent.

The comfort capture is dated 2026-07-14 06:31:15 local time. It adds denser
clustered flora, directional shadows, and stronger depth while preserving
separated biped limbs, source textures, and the same readable world composition.

The `*_fvr05_*.png` captures in the same artifact directory are supplemental UX
evidence. They are not substitutes for the two required clean runtime images.

## Runtime Receipts

Both final GPU-required launches exited successfully and their receipts report:

- `requested_policy=gpu-required`
- `selected_backend=GpuAuthoritative`
- `authoritative=true`
- `unavailable_reason=None`
- `adapter='NVIDIA GeForce RTX 3050'`
- `backend_api=Vulkan`
- `active_creatures=30`
- `finite_rejections=0`
- `no_active_bulk_readback=true`
- `compact_readback_bytes=1440`

The launch signatures identify the real P34 fixture save and asset manifest;
the production path does not construct a mock simulation or a fallback neural
backend.

The fresh renderer diagnostics report:

| Profile | Measured local smoke FPS | Target | Dressing | GPU VFX emitters |
|---|---:|---:|---:|---:|
| `MinimumSettings30x30` | 190.31 | 30 | 64 | 2 |
| `MinSpecComfort1080p` | 151.19 | 60 | 224 | 4 |

Both renderer diagnostics report 30 assembly roots, 210 part entities, 180
join covers, eight represented source families, 56 shared mesh handles,
`creature_visual_profile=modular-heritable-part-assembly-v1`, and
`production_visuals_display_only=true`.

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
`validate-production-assets` reports 75 assets, 67 generated assets, 8 external
texture assets, 6,747,108 committed bytes, 59 final-art entries, zero unknown
licenses, zero rejected entries, zero final-art placeholders, and a largest file
of 498,351 bytes.

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

The following commands passed on 2026-07-14:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets -j 1
cargo build -p alife_game_app --release --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -j 1
cargo test -p alife_world appearance -- --nocapture
cargo test -p alife_world --test save_load_roundtrip -- --nocapture
cargo test -p alife_tools creature_part_builder -- --nocapture
cargo test -p alife_game_app creature_part_catalog -j 1 -- --nocapture
cargo test -p alife_game_app creature_part_genetics -j 1 -- --nocapture
cargo test -p alife_game_app creature_assembly -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
cargo test -p alife_game_app --test app_shell lifecycle_lineage_birth_inherits_and_mutates_appearance_genes -j 1 -- --nocapture
cargo test -p alife_game_app --features gpu-runtime --test gpu_closed_loop_policy -j 1 -- --nocapture
cargo test -p alife_game_app --test no_cpu_shadow_runtime -j 1 -- --nocapture
cargo test -p alife_game_app --bin alife_game_app production_asset_validation_command_remains_available -j 1 -- --nocapture
cargo run -p alife_tools --bin creature_part_builder -- validate --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

The exact production release build also passed with the full shipping feature
set. The full FVR03 renderer integration suite passed 19 of 19 tests under the
low-debug, non-incremental test profile. The nine GPU policy tests, the no-CPU-
shadow boundary test, and the lineage appearance inheritance/mutation test also
passed.

After the no-conflict merge, `main` was verified byte-identical to the reviewed
feature tree and passed fmt, workspace check, core/docs gates, GPU policy,
lineage, no-CPU-shadow, part-pack validation, production-asset validation, the
release build, and both GPU-required profile launches. A redundant post-merge
rerun of the 19-test FVR03 renderer binary timed out after 1,204 seconds while
rebuilding the identical main-checkout target and returned no test result; it
is not represented as a second pass.

## Maintenance Guidance

Preserve the following properties when extending this layer:

- terrain relief may move tile interiors but must preserve shared tile edges;
- transition geometry must remain segmented and materially compatible;
- dressing must remain deterministic, capped per profile, and absent from
  creature-occupied tiles;
- new props need grounded silhouettes rather than broad flat fans or unit-cube
  debug geometry;
- production materials should stay lit and atlas-backed;
- preserve append-only family IDs and validate a new family before committing;
- build through staging and inspect compact previews before replacing a pack;
- keep whole source OBJs out of runtime packaging while retaining attribution;
- keep part fitting, sockets, joins, and materials in `alife_game_app`;
- visual tests must be followed by fresh release screenshots and direct image
  inspection;
- target artifacts, source sheets, caches, and large generated files stay out
  of Git.

Future source meshes can be added by registering a new catalog family,
calibrating its transform/cut profile, generating all three LOD packs, and
validating the production manifest. No save schema or renderer code change is
required when the new family follows the existing slot/socket contract.
