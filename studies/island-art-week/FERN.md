# Woodland fern and feather moss: artist's workbench

The playable original forest places the ordinary Fern by stable location and
uses the same three meshes inside its existing 45 m visibility range. The old
Fern asset has folded opaque parasol tops. In two creature-eye modeling studies
the seven-frond opaque replacement was too skeletal edge-on (90c909d9), or
became a stack of noticeably triangular pinnae when widened (941d8eaa).
Neither of those versions is the selected plant.

The next plant is nine individually arched surfaces with actual roll, camber and
droop around one grounded crown. `scripts/island_fern_ink.py` draws four original
woodland prints into one 1024 x 512 image, each with alternating pairs of lobed
pinnae and converging veins. Transparent soil and air are visible between their
edges in both the Blender model and its glTF; this is not a camera-facing cross
billboard. The three LODs use the same cutout, and sides do not cast new
shadows. The fern's use of its unique material needs the corresponding scoped
island renderer edit; applying its mesh without this binding produces solid
green rectangles. The source atlas is packed in the editable native blend.

The separate moss layer no longer has buried lumpy oval meshes. Four square
branch prints trace the original V1 triangles, with small creeping leaf sprigs
and mineral soil separating damp patches. The forest margin in study 941d8eaa
is visibly improved. The pines, their rooted feet, gameplay objects, ground
contact/collision, opt-in V2 choice, world saves and creature systems are not
edited by this stage.

Both camera locks use ordinary V1 with its existing sun and lens at 1.04 m.
The studio's left side is pinned to the actual pre-fern native assets at
fc5d22dc. Still pending until next week: native material binding under all
quality settings, mip coverage and glancing-angle flicker, LOD movement,
plant-foot contact, and graphics timings. No game, QA or tests in this art lane
this week.
