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
world maps under `geology/` and rebuild the middle and far representations
from the existing near mesh. NumPy, SciPy and Pillow are required. Then run
`python scripts/register_approved_art.py` and `python scripts/test_island_geology.py`.
The finish does not write to `island-v1.bin`, prop placements, or saves.
Keep the two PNGs alongside the GLB when copying it between art tools.

Pigment follows catchments, tidal elevation, exposure and soil depth cues.
Crevices and broken sills live in the corresponding tangent-space normal map,
not in the collision mesh. A separate fine roughness/cavity tile breaks up
close highlights without painting repeating dark dots into the overview. The
three levels use one standard glTF material, one primitive per chunk/level and
the same world-aligned addresses. No new draw pass or per-frame terrain work.

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
