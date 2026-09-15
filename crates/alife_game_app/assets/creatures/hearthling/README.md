Hearthling, balanced face and proportions, revision 5
===================================================

`hearthling-source.blend` retains the revision-2 sculpt and expressive idle rig.
The exporter applies `scripts/refine_hearthling_face.py` with
`scripts/hearthling_head.py` and `scripts/hearthling_eyes.py`: one continuous head
surface with eye openings, spherical green eyes, and rotating upper/lower lids.
The globes retain their shape during blinks. The muzzle, cheeks, and subtle fur
contour now share the head surface. The separate muzzle and cheek pieces are
removed. A shorter tapered neck and ears reduced by 14 percent balance the head.
The forelock and existing body/limb animations remain.
`hearthling.blend` is the editable refined model with grounded walk and seated sleep clips. The shipping
`hearthling.glb` contains one skin and three named animations.

From the repository root, run Blender 5.1 with `--background --python
scripts/export_hearthling.py`, then `python scripts/register_approved_art.py`
and `python scripts/check_hearthling_asset.py`.
Run Blender with `--background --python scripts/render_hearthling_states.py`
to reimport the shipping GLB and check grounding/render idle, walk, sleep, and blink.
The front camera also verifies that the eyes are visible when open and occluded
by the head's lids when closed. A half-blink render checks the transition.
This verifies the exported asset; it does not launch or verify the game runtime.

The production renderer selects clips from the existing organism state.
Pause freezes animation. Coat, mass, ear/head proportions and tail size derive
from the existing appearance genome. All current organisms use this approved
Hearthling body. Reedkin and Stoneback remain concepts, not shipping models.

The landscape source is `../../landscape/landscape.blend`. Export those props
with `scripts/export_landscape.py`, then refresh the same manifest receipts.
