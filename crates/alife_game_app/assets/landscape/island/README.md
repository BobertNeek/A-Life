# Approved island asset pack

Blender 5.2.1 LTS sources use metres, Z-up, and ground-centred prop origins.
`island-assets.blend` contains 13 reusable assets, three mesh LODs, dimension
guides, and the approved image blueprints. `island-terrain.blend` contains the
1000 x 1200 metre island, 240 metre peaks, ocean, river, and placed shared props.
The `Measured_Blueprint_Guides` collection is for authoring, not export.

The 2026-10-09 BerryBush and CoastalShrub overrides are owned by
`island-biological-shrubs.blend` (Blender 4.3.2), reconstructed from the exact
committed runtime GLBs before the pass. It contains six editable LOD meshes,
ground-centred origins, and the shared `ArtColor` surface. The original Blender
5.2 library, its embedded references, terrain source and all other props remain
byte-retained. For these two species, use the override source rather than the
older copies in `island-assets.blend` or the terrain authoring scene.

After editing the override, run
`blender --background --factory-startup --python-exit-code 1 --python scripts/build_island_biological_shrubs.py`
and then `python scripts/register_approved_art.py`. Run this export after a full
base-library rebuild as well. The optional `--author-from` bootstrap accepts
only the exact pre-pass `9a4e3b3d` GLBs, preventing repeated sculpt application.
The normal command exports the current native edits, preserving both bounds and
triangle ceilings. `scripts/check_island_biological_shrubs.py` checks those
contracts, manifest digests and retention of all other art against that revision.

BerryBush exposes larger orchid fruit clusters against natural green foliage;
its overview uses grouped fruit pods at the original face budget. CoastalShrub
has layered, wind-swept leaves with restrained sea-glass growth tips. Runtime
paths, names, one-material/one-primitive LOD bindings and placement data are
unchanged. See [the source-bound review](../../../../../docs/reviews/2026-10-09-island-biological-shrubs.md)
for isolated CPU-rendered before/after evidence; game-rendering acceptance is unrun.

The 2026-10-09 Fern color-only override is owned by
`fern-readability.blend` (Blender 4.3.2), reconstructed from the exact committed
`ec5bb040` runtime GLB. Use this editable override for Fern rather than its older
copies in `island-assets.blend` or the terrain authoring scene. The original
Blender 5.2 sources remain byte-retained; this does not claim Blender 5.2 source
compatibility. Geometry, normals, topology, bounds, pivots, material parameters,
three LODs (784 / 352 / 156 triangles) and the 97,100-byte runtime size are retained.
Only normalized RGB vertex-color bytes change, giving folded leaflets jade-green
ridges, darker edges and restrained growth/frond variation.

After editing this override, use the explicit Blender 4.3.2 executable:
`blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/build_fern_readability.py`
then `python scripts/register_approved_art.py` and
`python scripts/check_fern_readability.py`. Run this override export after a
full base-library rebuild as well. The shipped source reload/re-export is
byte-identical to the candidate. The guarded adapter transfers only colors into
`fern-readability/Fern.baseline.glb`, the exact pre-pass source-layout baseline;
it rejects changed positions, triangle order or ambiguous color seams. Blender
4.3's intermediate export can round split normals and duplicate near-LOD
vertices, so it is kept only under `target/artifacts/fern-readability/export`.
It is never shipped or registered. This color-only recipe does not support
geometry edits. The optional `--author-from` bootstrap accepts only that exact
baseline, preventing repeated paint application. The checker proves binary
color-only retention; it does not render or establish in-game acceptance.

The 2026-10-09 Sapling color-only override is owned by
`sapling-color.blend` (Blender 4.3.2), reconstructed from the exact pre-pass
runtime GLB. Use this override for Sapling instead of its older copies in
`island-assets.blend` or the terrain authoring scene. The canopy gains restrained
jade/sea-glass foliage and warmer olive new growth; bark is unchanged. Geometry,
normals, topology, bounds, pivots, material parameters, LOD budgets
(3,124 / 672 / 56 triangles) and the 127,320-byte runtime size are retained.
The original Blender 5.2 sources remain untouched; 5.2 compatibility is untested.

After editing this override, use the explicit Blender 4.3.2 executable:
`blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/build_sapling_color.py`
then `python scripts/register_approved_art.py` and
`python scripts/check_sapling_color.py`. Run this override after a full
base-library rebuild too. Normal export loads the editable source; the optional
`--author-from` bootstrap accepts only `sapling-color/Sapling.baseline.glb`,
preventing repeated paint application. The guarded adapter requires exact
triangle-corner positions, original materials/nodes, unambiguous colors and
pinned per-corner normals from an unchanged 4.3.2 round-trip. It transfers only
RGB bytes into the original layout. Native intermediate exports stay under
`target/artifacts/sapling-color/export` and are never shipped or registered.
This recipe supports color edits only. Source reload/export reproduces the
shipping GLB byte-for-byte. The checker proves retention; in-game visual
acceptance and physical GPU/performance testing are unrun.

The Conifer central-leader override is owned by `conifer-leader-repair.blend`
(Blender 4.3.2), reconstructed from the exact PR24 runtime GLB. This integration
was reconstructed on 2026-10-12 from the audited 2026-10-09 standalone repair;
it is not an exact recovery of the unavailable earlier repository patch.
Use this override for Conifer rather than its older copies in
`island-assets.blend` or the terrain authoring scene. The original multi-asset
sources and generators stay unchanged. The repair extends only the existing
trunk cap into the unchanged top foliage shell at all three LODs. This proves
geometric overlap, not topological welding. Bounds, horizontal coordinates,
foliage, indices, colors, materials, nodes, pivots and 3,456 / 668 / 172 triangle
budgets are unchanged; the runtime file remains 112,240 bytes.

To reproduce the exact shipping GLB without Blender, run
`python scripts/repair_conifer_leader.py`; it uses the Python standard library
and the pinned `conifer-leader/Conifer.baseline.glb` plus audited float32 patch.
`python scripts/derive_conifer_leader.py` independently reproduces it using
NumPy. Neither command runs the monolithic generator.

For a source reload and export, use the explicit Blender 4.3.2 executable:
`blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/build_conifer_leader.py`
then `python scripts/register_approved_art.py` and
`python scripts/check_conifer_leader.py`. Run the override after a full base
library rebuild too. `--author-from-baseline` reconstructs only this audited
Conifer source. The source adapter checks transforms, colors and topology,
reads its current positions, independently recalculates incident normals and
retains the baseline binary layout. The complete export-and-check workflow
rejects unreviewed source identity or coordinate changes; they require a new
deliberate recipe and review. Native diagnostic exports
and receipts stay under `target/artifacts/conifer-leader/` and are not shipped.
Run the checker in normal Python mode; optimized mode is explicitly rejected.
The checker verifies exact geometry/field retention, source identity and only
this registration change against PR24; it does not establish whole-app or
all-other-payload validation. It requires the pinned PR24 Git manifest, or an
explicit `--baseline-manifest` copy with that exact SHA256. The registrar
preserves this recorded provenance while refreshing asset digests.

The separate historical PR20 + PR23 + PR24 composed checker remains pinned to
its original four-asset profile. It deliberately rejects this later Conifer
asset and source addition. The separate `check_composed_conifer_graphics.py`
profile covers that fixed five-asset composition. It intentionally rejects
the later six saved refinements below. Both historical profiles remain
unchanged; use `check_consolidated_island_art.py` for the current all-repairs
composition rather than loosening either historical gate.
Blender 5.2, whole-island startup, production rendering, LOD transitions,
performance, GPU and training acceptance remain untested for this integration.


The saved 2026-10-09 Flowers, Grass, BoulderA/B/C and Cliff refinements are owned
by the six editable Blender 4.3.2 sources under `finished-island-refinements/`.
Use those sources instead of their older copies in `island-assets.blend` or the
terrain authoring scene. The original Blender 5.2 sources remain unchanged.
Flowers have broader orchid/ivory petals and amber centres; Grass has fuller
folded blades and growing tips. The boulders have distinct basalt fractures and
moss, while Cliff uses a restrained sea-gray palette.

Their exact Oct 4 source-pack layout baselines differ from the Oct 1 runtime
assets that were still installed. `lineage.json` pins both versions separately.
The reconciliation retains every installed L0/L1/L2 AABB, dimension, pivot,
node/mesh name, shared material and triangle budget. The earlier source-pack
sculpt changes and the five later plant/boulder geometry refinements are
intentional; only Cliff's Oct 9 pass is color-only versus its source-pack
baseline. Do not describe all six as color-only versus the old installed pack.
The historical baseline guard is retained as evidence, not silently removed.

After editing these overrides, use Blender 4.3.2:
`blender --background --threads 2 --factory-startup --python-exit-code 1 --python scripts/build_finished_island_assets.py`
then `python scripts/register_approved_art.py` and
`python scripts/check_finished_island_assets.py`. The guarded adapter loads
current native edits, preserves exact source-pack JSON/index layout and bounds,
and writes normalized byte colors. It never reapplies a sculpt/paint pass.
Native intermediate exports remain under `target/artifacts/finished-island-refinements/export`
and are neither shipped nor registered. Unchanged native sources reproduce all
six delivered runtime GLBs byte-for-byte. Source edits require a reviewed new
fixed profile; the present checks deliberately reject changed delivered bytes.

After any full base-library rebuild, re-export the shrub, Fern, Sapling,
Conifer and six-finished-asset overrides before refreshing registrations.
For the complete current art tree and all 48 manifest records, run
`python scripts/check_consolidated_island_art.py --output target/artifacts/consolidated-island-art/check.json`.
This fixed successor profile preserves the approved Conifer record, checks the
six explicit manifest deltas and audits every art/world file. The historical
four/five-asset checkers are unchanged and belong to their pinned snapshots.
See [the reconciliation review](../../../../../docs/reviews/2026-10-14-six-finished-island-assets.md).
CPU source reload/export and neutral-lighting comparison do not establish
in-game visual acceptance, GPU timing or performance.

The island is the sole playable map. The reusable Highlands prop library remains
available as art; its old terrain loader and save fallback have been retired.
Current saves include their terrain data and locomotion limits, and restoration
verifies that data against the saved binding without selecting another map.

Runtime terrain has 120 chunks with 20 metre skirts. All LODs use the same
vertex samples as the authoritative `alife_world/assets/island-v1.bin`.
Near/middle/overview triangle totals are 276480 / 76800 / 23040. The 3716 prop
instances share meshes and materials. Collision uses 1716 world-owned proxies.
Ground cover culls at 45 metres; LOD uses camera focus and viewport coverage.
Terrain zoom spans a 9.8 to 1600 metre vertical view on every graphics profile.
Camera clearance preserves the ground focus, and the two-triangle ocean extends
past the widest view. Overview shadows switch off above a 240 metre view.
Normalized byte colors reduce file size without changing heights or normals.
The island heightfield and placements remain authoritative.

The 2026-10-15 placement corrections lower exactly five proven exposed floating
rocks, keeping their XZ, scale, yaw and approved meshes:

- `1319` BoulderC: Y `155.7075371101759` -> `154.78496851454122`, proxy `1310`
- `1325` BoulderA: Y `65.20034210344221` -> `64.74313004876842`, proxy `1316`
- `1364` BoulderA: Y `5.288538399551772` -> `5.163427675697716`, proxy `1348`
- `1541` BoulderC: Y `85.85161233125542` -> `85.708212317397`, proxy `1489`
- `1730` BoulderC: Y `137.25853498797284` -> `137.1313965994088`, proxy `1631`

The original correction for 1319 is retained exactly. The analytic authoring
height left these instances above the sampled near terrain. Exact triangle-overlap
contact gives shallow support: 2–4.75 cm for 1319, 2–8.34 cm for 1325's visible
L0/L1, and 2–3.55 cm for the other new rocks. Meshes, terrain samples, all other
placements and all other collision proxies are unchanged. Only each matching
proxy's vertical bounds move with its instance. Hidden geometric candidates
1659/1620 and tiny-gap cases are deliberately unchanged.

BoulderA 1325's approved L2 simplification has a different support footprint.
Its unused L2 retains a 36.84 cm gap; forcing it down would overbury visible
L0/L1 by 40–47 cm. Production never displays non-tree L2: solid props require
view height <240 m and distance <210 m, while L2 requires height >350 m or
distance >240 m. The actual unchanged cull/LOD policy is extracted into a pure
helper and locked by a renderer boundary regression. No quality setting changes
these prop thresholds. This correction does not claim that 1325 L2 is grounded.

`scripts/island_placement_corrections.py` supplies the same guarded corrections
to the base authoring recipe. To reapply them to the existing source without a
full rebuild, use source-matched Blender 5.2.1 LTS:
`blender --background --threads 2 --factory-startup --python-exit-code 1 --python scripts/repair_island_boulder_placement.py`
then run `python scripts/register_approved_art.py` and
`python -m unittest discover -s scripts -p test_island_boulder_placement.py`.
The repair saves and reloads the native source, comparing every object transform,
mesh topology/paint/UV and packed image, apart from the reviewed locations. The
fixed consolidated profile includes the source, placement and proxy updates.
The delivered native file uses lossless Zstandard level 19 compression to fit
repository upload tooling. Its decompressed edited Blender 5.2.1 stream is
unchanged by that packaging (SHA256
`7bb87ce5c78592911b9123ef8e81fe307da37c3130a37f8a75201995d900c28d`)
and reloads normally in Blender. Ordinary authoring and the idempotent
corrections need only Blender itself. These CPU checks do not establish
in-game visual acceptance or GPU performance.

Changing proxies changes the baked terrain binding. New games and their saves
use the corrected placements. Older saves retain their own exact terrain and
creature state, but the existing renderer selects generic heightfield art for
that older binding instead of current baked island props. Tests cover both
original-main and one-rock-fix saved terrain. This correction does not migrate
old saves or change training checkpoints.

Rebuild from the repository root, using the explicit Blender 5.2.1 executable:

1. `blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/build_island_assets.py -- --no-review`
2. `blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/build_island_terrain.py -- --no-review`
3. `blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/build_wizard_hand.py -- --no-review`
4. `blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/finalize_island_blueprints.py`
5. `python scripts/register_approved_art.py`
6. `python scripts/check_island_art.py`

Omit `--no-review` to render material and actual polygon-edge reviews. Original
approved concepts must be present under the dated `target/artifacts/art-review`
folders when rebuilding from scratch; copies are embedded in the delivered
Blender files. Measurements and budgets are also in the specification JSON.

Verification on 2026-10-01: Blender source reload retained all five hand poses;
asset validation accepted all 218 registrations. Rendered terrain samples matched
the authoritative heightfield within 0.000001 metres. Native mouse checks covered
9.8 -> 1600 -> 9.8 metre zoom, unchanged ground focus, current-position picking,
creature and ball carry, grounded release, and edge scrolling.

The optimized Balanced1080p/Vulkan run with eight creatures on an RTX 3050
measured 330 frames in 60.18 seconds (about 5.5 FPS). Simulation tick work consumed
95.7% of recorded frame time. The remaining renderer/present/uninstrumented
residual averaged 6.82 ms per frame; that is not an isolated GPU render timing.
The existing performance recorder also opens its speech and lineage capture UI.
This run does not establish a graphics regression baseline or larger-population
performance. Receipt: `target/artifacts/island-hand-implementation/island-hand-performance.json`,
implementation revision `c050013d5cefe4760b019e46cb0ebb37d774f300`.

The 2026-10-01 presentation refinement preserves near/middle tree geometry and
all placements. Overview crowns now simplify connected envelopes of the authored
crown groups; conifers retain their separate bough tiers. Triangle counts are
88 / 172 / 56 for Broadleaf / Conifer / Sapling, within the previous 88 / 182 / 57
budgets. Across four CPU silhouette views, coverage of the near silhouette is
93-96% / 94-97% / 80-90%, respectively. Compare against the `e76e9e12` asset pack
with `scripts/check_island_canopies.py --baseline <old-island-folder> --candidate
crates/alife_game_app/assets/landscape/island --output <local-receipt-folder>`.

Island runtime water uses the same two-triangle sea plane. A one-time 1024-square
depth-color/foam texture samples the selected authoritative surface; a bounded
dry-land distance mask fades shallow channel tint offshore. A 128-square normal
tile repeats every 32 metres. Both textures have complete mip chains, with a
combined RGBA payload of 5,679,784 bytes. They use ordinary StandardMaterial,
without texture updates or additional geometry per frame. The old Highlands
water material is retained. Terrain heights, collision proxies, camera/hand
logic, Hearthling art, physiology, cognition, and headless scenarios are unchanged.

Local paused Vulkan captures at 1280 x 720 compare the same eight-creature seed,
overview, shoreline, grove, and 9.8 -> 1600 -> 9.8 metre views. These are visual
checks, not simulation or GPU timing measurements. They do not establish an FPS
gain. The reference's cliff profiles and denser forest placement remain separate
work; this refinement does not alter the authoritative terrain or add vegetation.
