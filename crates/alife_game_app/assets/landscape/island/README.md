# Island assets

`island-assets.blend` and `island-terrain.blend` retain the base authoring sources.
`props.json` binds runtime placement and collision. Exported GLBs, source models
and production manifest digests must be kept together.

When rebuilding the base library, apply species overrides afterward: connected
Fern from `scripts/build_fern_fronds.py`, CoastalShrub from
`scripts/build_coastal_leaf_sprays.py`, Broadleaf from
`scripts/build_broadleaf_canopy.py`, and Conifer from
`scripts/build_conifer_leader.py`. The Fern and CoastalShrub exporters target
different species; older shrub exports must precede the CoastalShrub override.
The Conifer exporter reuses the retained NumPy geometric derivation with exact
input/output hashes. Use the compatible Blender version recorded in each exporter.

Refresh registrations with `python scripts/register_approved_art.py`, then run
the game app's `validate-production-assets` command. Historical audits and render
receipts remain in Git history; the removed snapshot validators are not current
gates. Inspect new exports visually before shipping them.

### Terrain finish: salt meadow and tidecut stone

The base Blender file still owns the full-resolution island. After a base export,
run `python scripts/build_island_geology.py` to bake the two lossless 2048px
world maps and the three 768px close-ground maps under `geology/` and rebuild the middle and far representations
from the existing near mesh. NumPy, SciPy and Pillow are required. Then run
`python scripts/register_approved_art.py` and `python scripts/test_island_geology.py`.
The finish does not write to `island-v1.bin`, prop placements, or saves.
Keep these PNGs alongside the GLB when copying it between art tools.

Pigment follows catchments, tidal elevation, exposure and soil depth cues.
Crevices and broken sills live in the corresponding tangent-space normal map,
not in the collision mesh. A separate fine roughness/cavity tile breaks up
close highlights without painting repeating dark dots into the overview. The
near level trades the overview material for separate gravel chips, curled leaf
litter, broken stems and soil clods painted with corresponding normals and
roughness. The 4.7 m tile uses continuous, warped world UVs (~6 mm per texel);
each vertex multiplies it by its original linear island pigment. The middle
and far meshes keep the large maps. The renderer swaps the two standard glTF
materials along with its existing mesh handoff. Still one primitive per
chunk/level and no extra draw pass.

An absolute sine in the original northern heightfield produced repeated fins.
The bake detects that phase where it is actually present and removes its local
lighting and albedo emphasis. It does not lower peaks, alter triangle positions
or remove genuine skyline teeth. An offline trial capped at 0.75 m, using only
vertices with no adjacent walkable triangle, changed the wide skyline by at
most one pixel. Baking out the source wave could change that outline by seven
pixels but would move the underlying world by up to 10.2 m. A real contour
redesign needs a versioned heightfield, not invisible terrain under creatures.

The source is 321 × 385 samples. On the shipping overview, sampling the actual
indexed mesh at every source lattice point reduces the largest height mismatch
from 92.51 m to 2.77 m; the corresponding middle mismatch falls from 66.89 m
to 0.76 m. Most of the initial error was not this severe. Far mean error is
0.086 m (previously 0.339); within 2 m of sea level, far RMS error is 0.102 m
(previously 2.255). The number of samples on the wrong side of
sea level falls from 612 to 33 far and 160 to 10 middle. The near mesh does not
move. Far costs 43,946 triangles instead of 23,040, middle 104,318 instead of
76,800; near remains 276,480. These are full-island counts including skirts,
not measured frame times.

[Wide comparison](../../../../../docs/media/island-geology/wide.jpg) ·
[Low headland](../../../../../docs/media/island-geology/headland.jpg) ·
[Estuary](../../../../../docs/media/island-geology/estuary.jpg).
These locked CPU Blender studies isolate the real terrain GLBs using a neutral
sea plane; they are not game, RTX, or performance captures. Rebuild with
`blender -b -t 2 --python-exit-code 1 -P scripts/render_island_geology.py -- <GLB> <output-directory>`.
Append `--handoff` to render the same view of the river head at each LOD.

[Marked ridge, matched crop](../../../../../docs/media/island-geology/marked-ridge.jpg) ·
[Creature-height ground, dressed habitat](../../../../../docs/media/island-geology/creature-height.jpg).
For the dressed view append `--creature-height`. The perspective lens is 88 cm
above the source surface. It uses approved production prop GLBs and their
placements, but the lighting and camera are Blender, not Bevy.


### Eastern strand and island tide line

`strandline.glb` is display-only shingle, shell, wrack, reed, sand film and shallow
wash. Its spatially bounded patches are fitted to the actual near triangles and
culled with their parent terrain chunks. Rebuild with
`python scripts/build_island_strandline.py`, then run
`python scripts/verify_island_strandline.py crates/alife_game_app/assets/landscape/island/strandline.glb`
and refresh the production manifest. Never treat this layer as a collision mesh or
new water level. See the matched [island art studies](../../../../../studies/island-art-week/README.md).

### Rooted habitat substrate and salt meadow (weekly art branch)

Rebuild in this order: `python scripts/build_island_geology.py`,
`python scripts/build_island_habitats.py --rock-ground --out crates/alife_game_app/assets/landscape/island`,
`python scripts/build_island_strandline.py`, then
`python scripts/register_approved_art.py`. The habitat recipe starts its
shared macro colour from the original generated surface, so running it twice
cannot darken the coppices twice. All commands are art-only.

The near extension uses 76 existing land-adjacent chunks. Islands of turf,
dry sand, talus and grove loam cross each other inside 6.25 m PBR charts;
the other submerged chunks keep their original mesh. The same physical near
triangles, skirts and normals remain intact. A broad tree-density colour
field grades both the near vertex pigment and the shared middle/far map.
The 8-pixel gutters and filtered quarter-size ORM avoid wasting three
full-size texture allocations on recess light and roughness.

The packed `strandline.glb` has 50 m spatial batches for gravel, wash,
reed, windbreak and salt sward. Short non-billboard curved cutouts make
continuous grass beds; low rooted folded geometry and heath flowers break
their outline. The original progress-critical asset never waits on the
optional close habitat pack: a late pack is bound only to near mesh handles.
Normal terrain selection and all placements, saves and collision are unchanged.

`python scripts/build_island_marine_sky.py` bakes the separate visual-only
2800 m marine sky. It uses the palette's real solar direction, broad cloud
banks and high thin streaks in continuous azimuth. The game loads the
same interior mesh and image shown in the Blender visual study, with an
unlit non-fogged material and shadow casting/receiving disabled; it does
not change global sun, ambient intensity, wind or weather.

The weekly stone finish consumes the five original approved `BoulderA/B/C`,
`Scree` and `Cliff` GLBs from checkpoint `c6a3e18b36a8460307f525a0a69993da5c4a1cc9`.
Materialize these source files in a separate directory and invoke
`python scripts/build_island_minerals.py --source <original-directory> --out <fresh-directory>`.
The source bodies and their individual shell feet are not new colliders.
The verifier checks every rock's placed, rotated extents and each original
convex support point. Copy its five GLBs and `minerals/` alongside the island;
do not feed those finished GLBs into the generator as if they were sources.
The terrain recipe uses two independent directions across exposed rock, then
the same local deposit mask from actual slopes and rocks at all three LODs.
Both middle/far `geology/pigment.png` and `geology/relief.png` must be present.
