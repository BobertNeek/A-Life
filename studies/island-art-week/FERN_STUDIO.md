Once the rooted forest and first tidal optics are published, prioritize visual
readability at encounter height, not another global tint. The stock Fern model
visible on the right in the frozen pine803 slope is a very obvious broken
asset: in profile it is a handful of folded umbrella heads on stalks. It is an
ordinary shared landscape model, not a pickup or simulation species. Preserve
its original 1.0 x 0.75 x 0.9 m bound/foot and all placements. Replace those
opaque folded tops with arched tapering fronds, forked leaflets grown along a
central rachis and soft fenestrated edges. The stock budgets are 700,294,154
triangles (L0/1/2). Derive from the modeled pocket ferns if that helps,
but size and silhouette should agree with the other original forest plants.
Use the same low slope and a real dense guardian thicket, while keeping the
already approved coastal shrub, grass, trees, rock and game lane unchanged.
No tests or user hardware before next week.

The existing `scripts/build_fern_fronds.py` (October connected-ribs pass) deliberately
retained the old folded leaflet silhouettes and devoted 300+ triangles to
tubular ribs. It already solved connectivity. Do **not** redo that as the
next change. Reuse its ground origin, source materials, baseline and three
historical extents, but now actually change the leaflet profile: narrow swept
pinnae, independent taper, low curling young foliage. An editable source
must reopen and re-export the selected new mesh; retain the earlier DCC
checkpoint so it is not silently overwritten by its old generator.

An initial seven-frond mesh recipe is already in `scripts/island_fern_fans.py`
and its author/reopen Blender writer is `scripts/studio_forest_fern.py`. Use
studio-board subject `forest-fern`, compare the actual existing Fern at the
right of pine803's `copse-eye`. Only the candidate fern is replaced in that
isolated study; select its art before installing any new native prop. No renderer
contract or test has run against this draft. If its central stalks still dominate
or the feathered tips vanish in the player lens, lower the rachis and redesign
the pinna mass instead of changing the whole forest's light.

Art read from the selected 1.04 m sunlit margin: bark and soil contact now
work, but the nearest moss cushions can still read as green oval pads. While
finishing the fern, use equal texture pixels per ground metre in both local
moss axes, vary the location within the moss micro-panel, and feather a few
outer leaves among the cushions. Keep it restricted to the sheltered pockets;
do not cover the entire sloping meadow in noise.
