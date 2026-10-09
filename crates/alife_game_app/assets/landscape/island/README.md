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
