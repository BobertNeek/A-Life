"""On-site coastal ground photograph, never a detached specimen on a tabletop."""

from pathlib import Path
import json, sys, runpy, bpy
from mathutils import Vector

here = Path(__file__).resolve().parent
native, scene, shelf, hollow, cliff, cups, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:7]
)
args = sys.argv
sys.argv = [
    "blender",
    "--",
    *[str(p) for p in (native, scene, shelf, hollow, cliff, out)],
    "setup",
]
try:
    s = runpy.run_path(str(here.parent / "photo.py"), run_name="held_gullglass")
finally:
    sys.argv = args
sc = s["sc"]
cv = s["cv"]
surf = s["surf"]
base_cam = s["cams"]["wave-eye"]
eye = s["poses"]["wave-eye"][0]
target = [408.50, s["floor"](408.50, -99.72) + 0.055, -99.72]
contact = s["module"]("tidal_ground_material", here / "contact_material.py")
o = contact.close_camera(sc, cv, eye, target)


def shot(name, cam, side):
    (out / side).mkdir(exist_ok=True, parents=True)
    sc.camera = cam
    sc.render.filepath = str(out / side / (name + ".png"))
    bpy.ops.render.render(write_still=True)


shot("foothold", o, "before")
contact.bind(s["ground"], surf, cups)
s["grab"](cups / "cups-near.glb")
shot("wave-eye", base_cam, "after")
shot("foothold", o, "after")
(out / "angles.json").write_text(
    json.dumps(
        dict(
            standing_native_eye=eye,
            close_target=target,
            close_lens_mm=33,
            hold="9b9999b592978e93d590609ec5daa402fe960494",
            real_terrain=True,
            sea_images_held=True,
            not_native=True,
        ),
        indent=2,
    )
    + "\n"
)
