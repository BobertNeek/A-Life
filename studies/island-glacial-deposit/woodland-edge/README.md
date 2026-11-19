# The woodland dripline and seated glacial shingle

[1.04m: same 22 fragments, unseated / seated](talus-eye.jpg) · [same deposit at guardian height](erratic-reach.jpg) · [through the woodland gap](woodland-gap.jpg) · [the woodland edge at a lower guardian height](woodland-edge.jpg)

At the low #1303 camera, the old row of hanging front-left stones has settled onto the slope. Individual stone faces still read angular and the adjacent large boulder keeps its separate original downhill gap. The guardian silhouette changes little, as it should for low shingle. At the woodland gap, the bare foreground now has overlapping low leaf clusters leading to the two original spruce; from above, darker needle earth runs into separated mineral and green patches instead of a hard forest boundary. An earlier tall, straight-runners draft (04608607) read like seedlings in rows and was not selected. The revised foreground still gathers in a few bands, so the 24.47 m edge is an optional local art source rather than a recommendation to scatter it all over the island.

## What actually changed in these photographs

The first pair keeps the same ordinary V1 terrain, cameras, adjacent weathered 1383 boulder, growth, mineral texture and 22 carved rock shapes. Only each separate loose fragment at **#1303** is tilted, placed and partly buried against its own local V1 triangles, with uniform fitting kept inside each original horizontal extent. A single original transform remains. The V1 generator produces no collision volume for Scree. No native model, collision, terrain, lighting or save was edited. Research art for this single anchor, not a promise about the other 133.

The second pair keeps the same completed seating on BOTH sides. Original conifers **111/131**, fern **2510**, boulder **1383**, scree **1303** and other stock geometry stay anchored. Only the right side has the continuous local duff-to-mineral wash and the low communities of actual bent rushes and connected runners. It is not a larger collection of individual ground decals. A 704² world chart washes the *existing native terrain* in the DCC; the original surface and its shading normals are unchanged. The artist plant model is 42,642 triangles at this one location. Nothing has a hitbox, food value, or new saved state.

## Reopen the source

[Packed and editable Blender workshop](woodland-edge.blend) contains a cropped literal V1 triangle surface, clast selections split without welding their UV seams, a complete alternative, true landmark empties, and editable understory. Its floor is neutral by design; the comparative photos above are from the actual terrain, the released tree/fern models, real sky and coastal daylight. [Rooted woodland model](woodland-edge.glb) · [bed colour/coverage](woodland-bed.png) · [seated local shingle, beside the original mineral images](../talus/ScreeSeat1303.glb) · [precise authoring/source data](artist-source.json). Regenerate via `scripts/island_talus_contact.py`, `scripts/island_woodland_edge.py` and `scripts/studio_woodland_margin_editable.py`.

## Native boundary

No native binding, hardware run, gameplay, CI or tests were commissioned for this gallery. Before any future native adaptation, a changed or player-sculpted terrain must never retain a stale static dressing; the ordinary V1 SHA and actual footprint would need a safe binding or fallback. Keep the published original main and unchanged V1 semantics. No FPS inference from these shots. [The local binding boundary, without an implementation or claim](BOUNDARY.md).
