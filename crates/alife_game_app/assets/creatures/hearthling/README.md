Approved Hearthling, revision 2
==============================

`hearthling-source.blend` is the approved sculpt and expressive idle rig.
`hearthling.blend` adds grounded walk and seated sleep clips. The shipping
`hearthling.glb` contains one skin, 36 meshes, and three named animations.

From the repository root, run Blender 5.1 with `--background --python
scripts/export_hearthling.py`, then `python scripts/register_approved_art.py`
and `python scripts/check_hearthling_asset.py`.

The production renderer selects clips from the existing organism state.
Pause freezes animation. Coat, mass, ear/head proportions and tail size derive
from the existing appearance genome. All current organisms use this approved
Hearthling body. Reedkin and Stoneback remain concepts, not shipping models.

The landscape source is `../../landscape/landscape.blend`. Export those props
with `scripts/export_landscape.py`, then refresh the same manifest receipts.
