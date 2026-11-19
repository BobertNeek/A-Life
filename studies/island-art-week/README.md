# Island art week

Single working branch for Qualia's coastal habitat. The original terrain
material and geometry pass is PR #412. Its separate [physical ridge
study](../island-ridge-v2/README.md) changes both render and collision for
opt-in testing, not the default map or current saves. No native gameplay or
user-PC performance test has been run during this art week.

## From the dry bank to the windbreak

Water sorts the old constant-height pebble string into grouped anchor stones
and smaller satellites. Eleven reed patches and the wet wrack occupy sheltered
outer shores. The next layer grows only on dry, sufficiently gentle banks with
a real exposed shelf toward water: 82 separate wind-shaped stands, 332 rooted
sprays/clumps. They add olive and silver folded leaves, dry seed combs and
actual short shadows, not glowing point sprites. Existing trees and solid
obstacles keep their original placements, collision and pick behaviour.

The near shore uses five shared finishes. 240 patches on the *whole* island
contain 225,408 triangles; the heaviest is 8,604. The approved eye-height
area selects 28 before frustum culling, 35,058 triangles versus 23,855
previously. The bank selects 45,666 versus 26,497. These are static geometry
counts, not GPU timing. The wet sand is 3 mm off the exact surface, shingle
at most 4.21 cm high, folded plants below 1.10 m. The wash lies 9 mm above
water, not above the submerged floor. Each decoration vertex sits above
identical V1 and opt-in V2 underlying terrain. [Independent vertex and file
receipt](coast-validation.json).

The warm sun and cool fill are an island-only profile. Linear fog begins
beyond the camera's ground focus: zooming the orthographic camera out no
longer puts the habitat itself under a fully developed green veil. Leaving
the island restores the original camera and sun state. Rebuilding an island
scene acquires its replacement sun. The hosted island CPU/ECS suite passed
32 tests including restore/rebuild and camera replay, at `255d80a` (run
44440093415); this is not a GPU or gameplay gate.

## Art evidence and reproduction

[Composed coast A/B](cove-showcase.jpg) · [neutral 86 cm control](coast-eye.jpg) · [neutral bank control](coast-bank.jpg).

The shore-to-massif composition uses the real imported terrain, existing prop
LODs, near-only strandline patches, and the [native water inputs](water-inputs/README.md).
Both sides use the exact same opt-in V2 physical surface, byte-compared to the frozen compatibility fixture. The diagnostic controls use default V1. It is rendered in CPU Blender 4.3.2 (hosted run 44442858406). The same lens is used on both sides; daylight
and atmosphere intentionally change on the revised side. The two established
shore controls hold camera *and light* fixed and still use a neutral sea plane.

Reproduce the matched plates with [.github/workflows/island-art-study.yml](../../.github/workflows/island-art-study.yml).
The earlier beach baseline is pinned to `810e4f872680dc569affa6c8d2e3c31f79caa86c`;
the workflow verifies its SHA-256 before comparison. It verifies the new
geometry separately first with `scripts/verify_island_strandline.py`.

**Prior checkpoint:** [eye-height before/after](strand-eye.jpg),
[bank before/after](strand-reach.jpg), [earlier geometry](strand-validation.json).
This was the first wet film and broken lace pass, before the windbreaks.

**Art verdict and next iteration.** The first air study obscured the mountain;
this one holds its profile and leaves the close plants readable. The near lawn
still reads like coloured gravel and the far trees like isolated pins. Next
give turf, exposed stone and sand *different* close-scale relief instead of
stamping the same ground tile everywhere, then group plant contact/litter by
real rooted habitat. Keep the existing collision and test at the same camera.
A more saturated sky alone would not solve that floor.

**Next week:** [disposable profile, same native camera, contacts and timing](NEXT_WEEK.md).
No established save or guarded learner belongs in a review checkout.

## Turfs, lee meadows and grove openings

[Locked coast comparison](habitat-cove.jpg) · [86 cm sand control](habitat-strand.jpg) · [bank control](habitat-bank.jpg).

Hosted CPU Blender 4.3.2, run **44446749282**. The camera, physical terrain,
solar direction, water maps and prior prop placements match on both sides.
The showcase uses the V2 physical study; the two neutral locks use actual
default V1. Two private camera scouts by the tree trunk hid the main ridge,
so the open shoreline frame stays the lock. These are not screenshots or
performance readings from the game.

One shared near glTF material distinguishes wind-laid turf, ribbed shore sand,
fractured stone and shaded leaf litter using 51 rotation/reflection-equivalent
padded transition charts. An actual tree-density field grades understory and
openings across the near and far meshes. The real 175,104 affected triangles
in 76 land-adjacent chunks keep their ordered corners, skirts and smooth
normals. The UV seams increase these chunks from 92,492 to 189,392 stored
vertices, not draw calls. The near PBR atlas costs at most around 66.3 MiB
for three RGBA8 textures with a complete mip chain before compression
(2688×2352 colour and normal, 672×588 ORM); no GPU allocation was measured.

In the near coast, 82 lee-side windbreaks gain groups of curved, fixed
grass sheaves plus folded specimen plants. The six-material shore asset
contains 276,228 triangles in 271 spatial batches, largest 9,220 triangles.
The open-bank focus selects 42,538 versus 35,058 strand triangles before
frustum culling, 31 potential patches. The new sheaves reach at most 0.560 m;
all underlying V1 and V2 heights matched. The first ultra-soft texture and
thin, sparse isolated tussocks were rejected. The current grass/shingle
are materially different, but a few cutouts read as square hedges in profile
and the uniform sky still makes the ridge feel flat. Work next on those two
visible weaknesses, not another density increase over the entire island.

[Substrate geometry](habitat-validation.json) · [shore bounds](salt-meadow-validation.json) · [view budgets](habitat-view-budgets.json).

### Marine coast: actual sky and softer grass beds

[Matched coast before/after](marine-cove.jpg). Run **44448257829** used Blender
4.3.2 on CPU with the same 1.12 m perspective, sun, real prop positions, water
maps and V2 terrain. The neutral 86 cm and bank controls deliberately **do not**
get the new sky. The earlier photographed beach is held to predecessor
`4bf90a3438bc160665580ba711c44c1c96526879` by SHA-256 checks.

One 183,844-byte, inward-facing GLB makes altitude colour, broken low clouds
and high streaks visible around the ridge. Its 4,128 triangles and 1536×384
texture form a static layer beyond the playable island, with no per-frame
weather work. The PBR habitat and cloud artwork are the actual assets consumed
by the new bindings. Feathering the contours of individual grass pockets
reduced the maximum new sward height to 0.493 m without widening visibility
ranges. Some distant pines still repeat a narrow, pointed shape along the
saddle; the mountain-to-woodland transition and contact of stone with soil
are the next focus.

The previous 27c3b1c habitat-binding checkpoint passed all 34 hosted native
island CPU/ECS tests (run **44448104499**), including late-loading replacement
and allowing original terrain and props to instantiate without the optional
pack. Neither a CPU test nor an art plate qualifies the native sky or GPU
frame costs. Those remain next week's disposable-profile checks.

### Exposed windbreaks and sheltered pine leaders

[Guardian A/B](pines-guardian.jpg) · [creature A/B](pines-cove.jpg) ·
[geometry/proxy checks](grove-validation.json). Run **44450003730**, CPU
Blender 4.3.2. The original trunk and foliage mesh remains unchanged below
3.2 m in all 24 LOD/form combinations. The same fixed anchors and collision
proxies support 135 original leaders and 286 variants selected by canopy
exposure. No extra visible draw calls, triangles per tree, or extra trees.
The optional pack is 696 KiB; originals stay visible while loading. The
changes read on the guardian shoulder. At the low camera, some thin trunks
still line up and the stones seem set on top of smooth turf. Shape-specific
mineral scree and rooted terrain are the next art pass, not a different sky.

### Grounded mineral shoulder

CPU Blender 4.3.2 study **44453469724** holds both cameras, daylight, terrain,
water, crowns and their source positions fixed on the alternate massif. A
clay/albedo/normal decomposition traced the conspicuous repeated cuts on the
eastern lower shoulder to the old height-dependent material coordinates, not
physical relief. Two independent ground-plane axes and broken weathered joints
remove the false contours. From 1.03 m the old full polygon outlines on the
apron give way to angular shale in soil; the distant cove is a control, not
the main evidence of this pass.

Sixteen existing cliffs and 210 boulder/scree sources seed bounded fall-line
fans. In V1, 24,822 m² (4.46% of its above-water area) have strong exposed
mineral weight. Only 55 close habitat charts are needed, with the same atlas
dimensions, 76 near meshes and 175,104 ordered original triangles. A common
PBR skin serves all five rock shapes. The solid models' 6,512 triangles over
fifteen LODs receive no new faces; inner pinched rings are smoothed inside
every original block's convex hull. All hull vertices, entire feet, vertical
positions, proxy extremes at all 574 original rock rotations and all triangle
windings were checked. Scree keeps its earlier geometry. The original flat
boulder B at (254.68, -177.67) touches the uphill slope. Its original downhill
lip overhangs by up to 45 cm; this material does not change the heightfield or
move its collider to disguise that. The rock may still look too block-like
and several existing pine crowns read as spinning plates at eye height.

[Mesh/contact checks](rock-geometry-validation.json) · [terrain checks](rock-ground-validation.json)
· [terrain cost](rock-ground-budget.json). No frame-time or gameplay claim.
The material binding uses two reusable surface handles (plants and stones)
instead of accidentally texturing every prop with the first plant's material.
A separate render/byte check reproduces the final opaque RGBA packing and
rock tangent-normal convention before selecting the final review plates.

Final renderer-matched [guardian](mineral-guardian.jpg), [1.03 m creature eye](mineral-creature.jpg)
and [unchanged wide camera](mineral-cove.jpg) come from **44454766679**. The
first exact-byte job caught sub-micro-unit differences between local and hosted
rock UVs/normals (positions and RGBA were identical). The five checked-in
GLBs now use the actual hosted producer behind these renders. This is a
reproducibility correction, not a claim that the first byte gate passed.

### Week checkpoint: real maritime spruce boughs

The old conifers were stacks of separate level plates. The [1.03 m stone-eye comparison](bough-creature.jpg)
and [dense hillside from above](bough-guardian.jpg) show a different mesh, not a rescale: each spruce
has 25 to 29 staggered bough attachments, drooping forked branch sprays, broken lower edges and
an asymmetric central needle mass. Twelve locally sized/wind-oriented forms cover the original
421 pines; no new tree entity, placement, rotation, size or proxy was introduced. Branches share
one opaque material (aligned needle pigment, normal and roughness); no cutout overdraw or extra
per-tree draw. The native loader retains the source pines until all 36 mesh variants and the
shared surface are available, then changes only the correct pine entities, once.

The [geometry and projected silhouette check](bough-validation.json) compares eight angles per
form at each of three LODs. Near 2,985–3,433 versus original 3,456 triangles; middle 555–627
versus 668; far 158 versus 172. Level 1 and 2 retain at least 82.0% / 78.9% of near silhouette area
at these angles, not a promise of zero in-motion pop. The unbent source bark through 3.2 m
is bit-identical in local position, normal and color; all 421 physical trunk boxes are unchanged.
The bound asset and these two plates are the exact output of hosted Blender 4.3.2 CPU study
44456932447 at recipe 4547e3358d98f6e66c0e1d752339010fb8ab8c7f. These are offline poses on
the separate V2 study surface. Neither they nor the CPU ECS tests are a live game or FPS result.

**Review correction:** the old guardian camera reused the grid-width variable as its lens width.
That left tree placements and the matched canopy change intact, but could misplace grounded
small plants and select the wrong LOD for background chunks. The new frozen review grid
requires an exact physical triangle match at all 3,716 placements and checks every chunk
at three locked spans. The eye among pine 434 now uses the same local chunk gate as the
native unoccupied terrain, rather than drawing the entire island at maximum detail. Keep
the earlier plates as historical art comparisons, not native vegetation-contact evidence.

### Art studio: forked, feathered island spruce

The [1.02 m view from inside the boughs](spruce-under-canopy.jpg) and
[guardian view of the same woods](spruce-guardian.jpg) show the actual V1 island.
Tree transforms, underlying terrain, water, light and camera are identical on
the two sides. The old discs and the later giant opaque paddle leaves are
replaced by real woody armatures with forked, folded banks of needles. Each
close bank has small transparent spaces between needlelets, not a green post
behind all the branches. Dense fill follows the individual limbs only at the
middle distance; it is omitted at creature height.

The [editable, packed Blender source](../../crates/alife_game_app/assets/landscape/island/grove-boughs.blend)
includes all 12 forms and 3 LODs. By default the 3 collections of close forms
are visible in the workbench. The game still reads the GLB and the established
tree index, never the artist's arrangement. [Export counts](spruce-art-budget.json)
are authored geometry, not frame-time measurements. One clipped material is no
guarantee of unchanged fragment or shadow cost.

Original comparison source: `c6a3e18b36a8460307f525a0a69993da5c4a1cc9`.
Studio source: `042d628780eee2c8b22d63247c2b3ca1f4bc2ee0`; paired
1000×600 Blender 4.3.2 CPU stills at 80 samples and the actual editable file
came from art run **44461673484**. No game, PC, compatibility suite or FPS run
was performed for the new assets. The gallery is not native acceptance.

### Rooted woodland at creature scale

[The locked slope](sheltered-slope.jpg) and [the sunlit fern hollow](sunlit-margin.jpg) were made by Blender 4.3.2 CPU art run **44466525999** at artist source `a4ffdc232aa0b5d3b0ea3c1fa32641b8752b15da`. The left image is the installed feathered-spruce checkpoint, the right changes the visible bark, root flares and woodland floor, not the tree positions or walkable island. [How it was drawn and what remains to check next week](WOODLAND.md). No game/PC/performance test in this studio-only week.

### The sheltered tidal shelf

[Original bands versus sand, near and guardian waters](COVE.md). Stock V1 only; all other bindings retain live depth-derived water color. Same plane, no simulation or gameplay changes. No native or performance testing this week.

### Woodland fern and creeping moss

[At eye level beside pine 803](divided-fern-slope.jpg) · [sunlit forest floor](feather-moss-margin.jpg) · [the two actual models on the artist’s workbench](fern-botanical.png) · [authoring and next-week checks](FERN.md). The previous 0.75 m fern with folded umbrella tops has nine tapering fronds, a solid crown and original two-sided cutout ink. The moss is ground-tracing vegetation and scattered shoots rather than solid green humps. The existing 278 ordinary ferns keep their original positions, bounds and ground anchor. 906/414/258 model triangles per LOD; one masked material, with no changes to the other props. Blender-only board 43ff89ef1be32e8c592b2b8fa3cf2fdc4e5655c7 (art run **44472178781**), not a screenshot or performance test.
