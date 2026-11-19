# Eastern cove studio: a cool shelf below warm sand

The submerged floor changes the colour of eastern cove water before the surface
goes deep. The initial palette is intentionally mineral-quieter at an exposed
bottom and clearer above a sandy reach. A long swell, crosswind and finer
capillaries are painted into a single world-aligned 18 metre rough surface. No
paint changes the sea height, the water contact plane, the submerged floor or
any object. Do not add standing rocks or skerries above collision-free water
merely to improve the still.

`scripts/island_cove_optics.py` samples the actual two triangles in each original
V1 cell. Water outside the island's supplied bed fades to deep sea instead of
inheriting an endless turquoise ribbon. The shell/stone lookup is used only
while painting, not sampled by a new per-fragment shader. The Blender material
uses the saved shore and normal image files plus the ordinary sun and sky,
with no painted camera glint. `shelf-eye` is a 1.12 metre perspective artwork;
`shelf-guardian` looks down across the eastern reach. A studio still is not
evidence of a native material load or a moving wave.

Art board subject: `cove-water`, angles `shelf-eye, shelf-guardian`. The minimal
next-week native integration is two saved, mipped images on the existing sea
quad. Use the V1 chart only for the exact original terrain binding; keep the
world-derived material fallback for all other surfaces. Pan through shallow
foam, check roughness and normal amplitude in motion and in each graphics
preset, then use a warmed RTX 3050 frame capture. Do not promote this solely
from a static DCC plate.
