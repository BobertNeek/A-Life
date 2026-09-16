Hearthling, bipedal reference sculpt, revision 8
==============================================

`hearthling-source.blend` retains the revision-2 sculpt and expressive idle rig.
The exporter applies `scripts/refine_hearthling_face.py` with
`scripts/hearthling_head.py` and `scripts/hearthling_eyes.py`: an animal muzzle,
spherical green eyes, and rigid curved upper/lower eyelids. The globes retain their
shape during blinks. `scripts/hearthling_reference_sculpt.py` shortens the body
and limbs, broadens the paws, and authors a fox brush tail, cream coat markings,
pointed ears, and layered fur. `scripts/hearthling_fur_fusion.py` joins the fur
into the head, body, tail, and ears, then transfers colors and skin weights back
to the finished surfaces. It restores the eye apertures and head binding after
sculpting. The result remains an upright biped with two arms and two legs.
`scripts/bake_hearthling_fur.py` bakes directional fur relief to a packed 2048px
tangent normal atlas. The GLB uses solid geometry and standard PBR materials;
it does not require a hair renderer or procedural Blender shaders.
`scripts/refine_hearthling_reference_proportions.py` applies the matched-view
anatomy corrections: narrower cheek span with unchanged eye spacing, a shallower
rear skull, shorter and broader ears, compact body and palms, rounded toes, and
a higher tail. It adds orbital margins and normalizes deform weights. The asset
budget is 32,768 triangles, including separate curved eyelids and orbital rims.
`hearthling.blend` is the editable refined model with grounded walk and seated sleep clips. The shipping
`hearthling.glb` contains one skin and three named animations.

Revision 8 replaces the old head skin through
`scripts/refine_hearthling_expression.py`. The continuous head has a short,
broad muzzle, rounded supporting cheeks, a distinct chin and lower jaw, and
smaller fur tips. Almond-shaped openings frame the existing round eyes and
rotating blink shells. The user-approved face shape retains cream on the
muzzle and chin, with a darker orange-brown eye and forehead mask. The body,
ears, tail, and animation contract remain the revision-7 bipedal design.
`paint_reference_face()` reapplies this palette without changing geometry.
The forehead includes an amber center, tapered chestnut streaks, and directional
color grain inspired by the supplied fur reference. These are vertex colors;
the texture count, six material primitives, and triangle count are unchanged.
The eye globes and curved blink shells are uniformly 15% smaller around their
existing pivots. The surrounding apertures taper toward their corners for a
more almond-shaped outline. Pupils are another 15% smaller within the irises.
Both refinements retain the existing topology, weights, and animation clips;
the shipping GLB's open, half-blink, and closed-eye renders were checked again.

The export finishes with `scripts/optimize_hearthling_runtime.py`. It consolidates
the authored mesh objects into one skinned mesh with six material primitives,
preserving all 32,306 triangles, UVs, fur normals, weights, and animation clips.
The authoring blend retains the separate editable pieces. Runtime instances share
the fur normal at 1024px (the editable source retains its 2048px bake),
the animation graph and palette materials. Conservative animated bounds permit
frustum culling without cropping the ears or tail.

From the repository root, run Blender 5.2.1 LTS with `--background --python-exit-code 1 --python
scripts/export_hearthling.py`, then `python scripts/register_approved_art.py`
and `python scripts/check_hearthling_asset.py`.
Run Blender with `--background --python scripts/render_hearthling_states.py`
to reimport the shipping GLB and check grounding/render idle, walk, sleep, and blink.
The front camera also verifies that the eyes are visible when open and occluded
by the upper eyelids when closed. A half-blink render checks the transition.
This verifies the exported asset; it does not launch or verify the game runtime.

The authoring executable is
`C:/Users/PC/AppData/Local/Programs/Blender/blender-5.2.1-windows-x64/blender.exe`.
The `blender-character` skill's `review_character.py` can review the exported
authoring file's `Hearthling_Character` collection in a separate background
process. Keep neutral front, side, back, and three-quarter views consistent.
Revision 7 was also edited and checkpointed through Blender MCP protocol 7;
MCP is for bounded live edits, with baking/export/review in separate processes.
Revision 8 used matched front, side, three-quarter, clay, blink, and full-body
checks across six shape passes, followed by a palette pass through live MCP.
`scripts/review_hearthling_face.py -- --output <directory>` renders those
matched views from a loaded authoring checkpoint. The optimizer explicitly
selects `RuntimeColor` as the render color layer so both coat and iris colors
reach glTF `COLOR_0` after consolidation.

Revision-8 verification: the shipping GLB passes the 32,768-triangle/six-primitive
budget, skin/clip/color/UV checks, imported idle/walk/sleep grounding and open/
closed-eye occlusion checks. A 25-second Vulkan New Game smoke loaded all six
Hearthlings with the refreshed production manifest and exited successfully.
Captures and staged shape reviews are under
`target/artifacts/face-refinement-20260915/`. This asset pass does not claim to
resolve the separately measured GPU simulation stalls.

The graphics branch's renderer selects clips from the existing organism state.
Pause freezes animation. Coat, mass, ear/head proportions and tail size derive
from the existing appearance genome. All current organisms use this approved
Hearthling body. Reedkin and Stoneback remain concepts, not shipping models.
This revision is an asset update on `refine-hearthling-face-20260914`; it does not
reintroduce the historical graphics implementation into the current main branch.

The landscape source is `../../landscape/landscape.blend`. Export those props
with `scripts/export_landscape.py`, then refresh the same manifest receipts.
