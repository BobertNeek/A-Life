# Weatherlip: the two stone heels

[Matched three-station folio](gallery/Weatherlip-Stone-Seats.jpg) · [Editable before/after workbench](gallery/Weatherlip-Stone-Seats.blend) · [Full-size original photographs](gallery/locked-photographs.zip). The [earlier complete cove and source](../gallery) remain unchanged.

At the locked rock-toe camera the dark stone now has a tapered, broken heel meeting sloped sand instead of a flat black gap. From the beach the warm boulder’s base is visibly wider at the bed and its grain reads as the local shore rock; its downhill face still casts a broad, dark shadow. The guardian view stays essentially the same: this is contact and material refinement, not a changed coastline. All three are Blender photographs, not gameplay.

## What actually changed

Only the specific *visual* bodies of the two already-placed compound erratics, BoulderB #1785 at the northern rock toe and BoulderA #1619 at the sheltered bench. The next stone uphill, #1472, is an unchanged control. No old landmark is hidden or translated. The original rock parent translation, uniform scale and yaw and the original three component identities are fixed. All three model levels retain each original component's projected convex footprint. The original top height is held. The downhill shoulder tips gently into the inclined bed and its lower faces rake back to a genuine supporting heel. Two keyed GLBs, not a global BoulderA/B replacement.

The wet side has the cove's own quartz grain worked into its opaque iron/mineral skin. The cool, dark stone and warmer stone stay distinct. Salt staining is weighted by distance to the real ground under each vertex, not a horizontal black band. The new material's occlusion channel is neutral white. Existing light, GI, shadow casting, camera, V1 soil, cove textures, plants and loose chips are unchanged. There is **no** dirt skirt, extra shadow mesh, altered sun, extra ground or below-collider toe.

## Geometric evidence, not a game test

Tabulated in [contact.json](gallery/contact.json). For each source body and level, the modeller samples a 67 × 67 lattice restricted to its old projected outline. It intersects the lower convex shell with the actual V1 triangles, using the saved prop's yaw and scale. The percentage below is the sampled fraction whose underside meets/intersects V1 or lies within 1.5 cm of it. It is not a claim about simulated contact or friction.

| Original instance | Native solid | Largest component, before → after | Reachable under the box-floor constraint |
| :-- | --: | --: | --: |
| Dark #1785 | 1664 | 32% → 50% | 52% |
| Warm #1619 | 1547 | 14% → 40% | 43% |

More than half of a large top-heavy block's plan can still cantilever outward over real downhill air because its fixed collision floor stands above the native slope. Closing every dark pixel would require changing the placement/solid or raising ground, neither of which this project does. Horizontal support agreement, actual saved box extents, original parent transforms, the reachable fraction, and per-body clearance statistics for all levels are included in the receipt. The two near models together are 546 triangles versus the original 624. The mid forms total 406 versus 280 and far forms 236 versus 124, because each keeps its original footprint and supported heel; this is not a frame-time claim.

## Reopen the study

The portable .blend is a **separate copy** of the complete existing cove edit, with switchable true old/new rock collections at native coordinates. The original empty-wire AABBs can be switched on for comparison and never render. The folio and its six lossless originals are the full-island photographs; the neutral isolated desk has no counterfeit tree or ocean panorama.

The candidate GLBs in `gallery/` expect the adjacent `coastal-erratic/` texture directory. `heel.py` reads the real source hulls and physical floor; `rind.py` produces the shared, direction-mapped coastal material; `atelier_seat.py` imports exactly the existing finished cove art and changes only those two model keys. It does not rebake vegetation or geology. The bearing calculation uses only the original triangle terrain, not surface chips or the existing visual plant scatter. An incidental source limitation in the frozen prior cove: the old art-scatter broad exclusion reads its box columns differently from V1’s Xmin/Zmin/Xmax/Zmax order; it is not an obstacle-clearance proof. No greenery or stone-chip edits are hidden in this comparison. No renderer binding, V1 default, collision, save, gameplay system or brain has been touched. No game, build, test or user PC was used.

