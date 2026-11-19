"""Matched Gullglass habitat photograph: one named layer arrangement, two archived stations.

The six parameters of the original held coastal stage stay unchanged. Independent
9b9999 controls predate all four additions: upper rock, near split joints, shells,
and the three actual submerged wash noses. The fork is Blender art, not native.
"""

from pathlib import Path
import bpy, json, sys, runpy

H = Path(__file__).resolve().parent
native, scene, shelf, hollow, base, cups, upper, toe, wash, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:10]
)
prepare_only = "setup-only" in sys.argv[sys.argv.index("--") + 11 :]
oldargs = sys.argv
sys.argv = [
    "blender",
    "--",
    *[str(x) for x in (native, scene, shelf, hollow, base, out)],
    "setup",
]
try:
    s = runpy.run_path(str(H.parent / "photo.py"), run_name="gullglass_composition")
finally:
    sys.argv = oldargs


def use(p):
    return runpy.run_path(str(p))["bind"]


ground, surf = s["ground"], s["surf"]
use(H.parent / "outfold/material.py")(ground, surf, base, upper)
use(H / "bind.py")(ground, surf, base, toe)
use(H.parent / "tideline/contact_material.py")(ground, surf, cups)
s["grab"](cups / "cups-near.glb")
water = s["sc"].objects.get("Held native V1 sea level")
if water is None:
    raise ValueError("Original water must remain in the composition")
use(H.parent / "wash/material.py")(water, surf, wash)
(out / "after").mkdir(parents=True, exist_ok=True)
for name, cam in () if prepare_only else s["cams"].items():
    s["sc"].camera = cam
    s["sc"].render.filepath = str(out / "after" / (name + ".png"))
    bpy.ops.render.render(write_still=True)
(out / "art-arrangement.json").write_text(
    json.dumps(
        dict(
            source="9b9999b592978e93d590609ec5daa402fe960494 / gallery/after",
            cameras=s["poses"],
            layers=[
                "Four disconnected face exposures",
                "1cm local relative-colour joints",
                "148 real-seated casts",
                "Three original below-sea wash noses",
            ],
            native_water_images_preserved=True,
            optical_water_modified_locally=True,
            native_game_capture=False,
            original_physical_surface=True,
        ),
        indent=2,
    )
    + "\n"
)
