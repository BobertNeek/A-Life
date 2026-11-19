# Silverleaf on ordinary woodland till

[The low moraine](moraine-soil.jpg) · [The three-metre rock view](erratic-foot.jpg) · [Three-pocket study model](silverleaf-till.glb).

An optional wild ground layer at the original glacial deposits 1570, 1383 and 1675. The change is small but legible: folded paired leaves follow actual bronze runners. There are sparse spent heads, not berries or food affordances. The open dirt stays visible. The close camera especially shows how the blades differ from the taller woodrush. Treat the light disconnected-looking terminal leaf at upper left and the sparse linear rhythm as reasons to refine rather than blanket the island. NOT bound in the game and no new ecology or save data.

## Actual comparison, not a re-captioned bare site

Both left frames are the original 1:1 shaded woodrush-and-till baseline from CPU Blender source `addc37af57ae337caeef2f890c9e8b9e6c5ffe88`, run 44509691508, BEFORE the optional soil UV change. Both right frames come from source `a2ee8448beba1dc9f126976f165b2a29ec8e3a38`, run 44512107562, WITH the real silverleaf. The two renders used the same 1000×600 locked lenses, 80 samples, daylight, native terrain/pigment and revised rock; only the border and caption are added. The prior run's visible baseline file and current run's before-runners asset have identical SHA-256 `d83f3f87efff383133035551cda9cdc12c3b0c2af824edbc4efa74daa503e563`. The automatically assembled before frame at a2ee8448 **omitted the entire till** and is not used in this exhibit. The studio baseline argument is corrected in this checkpoint for future work.

Sculpt source: `scripts/island_erratic_silverleaf.py`, composed through `scripts/island_erratic_train.py`. Including all earlier rush and stones: 1570 2,761→5,525 tris; 1383 2,878→6,794; 1675 2,683→6,261. Total 8,322→18,580 for three optional near pockets, not for all 424 rocks. This is offline artwork, not measured runtime or a playable asset assertion.
