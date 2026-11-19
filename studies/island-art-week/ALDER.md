# Next island art: a maritime alder instead of eight green plates

The finished conifer crown is not the whole forest. The stock V1 object list
contains 691 Broadleaf and 421 Conifer; the original 691 still share October's
one hardwood mesh. In the existing Broadleaf close model, eight closed shallow
icospheres are 0.76–1.10 m high around 1.5–2.8 m wide. Four bent opaque
triangles per leaf, many 0.6–0.8 m long, ride those forms. The single-tree native
asset portrait shows four horizontal paddles held up on otherwise bare rods.
This is not a retint or a rationale for moving 691 gameplay objects.

Keep the original bole, flare and actual fork locations. The L0 source has
eight individual load-bearing woody branches, each 52 triangles, from 2.18–
3.44 m up the existing stem. At those original tips, grow fewer architectural
forks that arch toward and past their neighbours in different planes. Willow
or alder-like ovate sprays join along the fork and taper into its gap, not at
an identical height. Lose the full solid orchard discs in the near mesh;
replace the guardian mass with overlapping non-horizontal irregular lobes.
Several branch ends droop, but avoid a patchwork of suspended flat alpha cards.
Existing 6 x 5.5 x 8 m scale, axes, base and all collision and save anchors stay
exact. Never place floating saturated flecks over an empty mesh to fake volume.

The historical native source and the 4462/916/86 triangle budget remain
available in the old branch. Capture the seated, unshifted existing Broadleaf
1202 near the low eastern cove (317.1, 8.3, 139.9; 1010 is just behind), plus
its same stand from above to review the interplay of tree volume, negative
space and shoreline. Use one reproducible editable .blend first; do not
install a family of variants until that candidate reads materially better in
both views. No native acceptance tests this week; do not overwrite the
separate gameplay lane's village willow assets.

The first full native-asset Blender board at `94b624b7` (44473796744)
removed the tiered solid leaves nicely at 1.04 m, but the 146 m-wide guardian
composition exposed a thinned, almost winter canopy. Do not install that file;
its exported mesh names also need the stable native `Broadleaf_L0/1/2`
contract. The new author preserves the original woody normals as well as
coordinates, rolls small stem-supported clusters through the hollow canopy
and gives the middle model a few dappled opaque *interior* leaf masses. Edges
remain the finer real leaf silhouettes. The near forms do not gain a whole
opaque roof. Photograph both cameras again; one convincing close model is
not enough when the guardian view is the main playing perspective.

Authoring re-export after selection: open the later `alder-boughs.blend` to
edit branches or its packed native leaf painting. Run
`blender -b -t 1 --python-exit-code 1 -P scripts/studio_coastal_alder.py -- crates/alife_game_app/assets/landscape/island`
to export those edits; do not pass `--author-from` when opening an existing
artist file. The older October `broadleaf-canopy.blend` is kept for comparison.
Similarly the new fern reopens with `scripts/studio_forest_fern.py` and
`fern-sori.blend`. The historical whole-library generator overwrites ordinary
prop outputs, so re-export selected overrides after intentionally rebuilding that
library, then refresh the approved-art receipts. Static counts are not GPU costs.

The first opaque mid-distance volume corrected the missing canopy density but
turned almost black in the guardian image. Correct winding exposed a second
real problem: the shape was too crystalline even when lit. It was rejected.
The current middle crown uses four original square leaf-cluster paintings,
their contour drawn only by overlapping individual leaves. Nonparallel curved
structures carry those printed clusters through the crown. No opaque pyramids
or full-size green caps are installed into the near or middle models.

Retained studio board: `f9af064dc905106c387ff69fa7ad760732f79236`; Blender-only art run **44475423297**. The coast uses its actual 691 Broadleaf transforms. One close model has 4206 triangles, middle 1140, distant 98, versus the earlier 4462/916/86. One embedded texture and one cutout family replace the leaf and wood finish together; this does not measure sampler/shadow cost. No game or tests in this week.
