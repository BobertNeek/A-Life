"""Non-destructive, independent Gullglass desk: original V1 surface and new exposed rock."""

from pathlib import Path
import runpy, bpy, sys, json
from importlib.util import spec_from_file_location, module_from_spec

h = Path(__file__).resolve().parent
prior, base, assets, out = map(
    lambda a: Path(a).resolve(), sys.argv[sys.argv.index("--") + 1 :][:4]
)
bpy.ops.wm.open_mainfile(filepath=str(prior))
spec = spec_from_file_location("gullglass_colour", h.parent / "material.py")
surf = module_from_spec(spec)
spec.loader.exec_module(surf)
sc = bpy.context.scene
ground = [
    a
    for a in sc.objects
    if a.type == "MESH" and a.data.name.startswith("Original V1 coastal diagonals")
]
if not ground:
    raise ValueError("Expected the held original V1 coastal desk")
runpy.run_path(str(h / "material.py"))["bind"](ground, surf, base, assets)
cam = sc.objects.get("outside")
if not cam:
    raise ValueError("Missing held guardian")
sc.camera = cam
for scr in bpy.data.screens:
    for area in scr.areas:
        if area.type == "VIEW_3D":
            v = area.spaces.active
            v.region_3d.view_rotation = cam.rotation_euler.to_quaternion()
            v.region_3d.view_location = (398, 115, 11)
            v.region_3d.view_distance = 94
            v.region_3d.view_perspective = "ORTHO"
n = bpy.data.texts.new("The flank in genuine elevation")
n.write(
    "The original V1 triangles are unchanged beneath four deliberately separate shallow colour/roughness and joint-normal windows. No cuts in the skyline. This is not a native game photograph, bypass route, displacement or physics replacement.\n\n"
    + (assets / "outfold.json").read_text()
    + "\n\n"
    + (assets / "flow.json").read_text()
)
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(out), compress=True)
