"""Editable traceable shore planting, both instanced LODs and the real root-level viewing position."""

import bpy, sys, runpy, json
from pathlib import Path
from mathutils import Vector
from importlib.util import spec_from_file_location, module_from_spec

H = Path(__file__).resolve().parent
prior, lace, out = map(
    lambda p: Path(p).resolve(), sys.argv[sys.argv.index("--") + 1 :][:3]
)
bpy.ops.wm.open_mainfile(filepath=str(prior))
s = spec_from_file_location("gullglass_original", H.parent / "material.py")
surf = module_from_spec(s)
s.loader.exec_module(surf)
sc = bpy.context.scene
ground = [
    a
    for a in sc.objects
    if a.type == "MESH" and a.data.name.startswith("Original V1 coastal diagonals")
]
if not ground:
    raise ValueError("Requires the archived coastal physical cut")
runpy.run_path(str(H / "material.py"))["bind"](ground, surf, lace)
for energy, name in [
    ("near", "bladderlace-near.glb"),
    ("middle", "bladderlace-middle.glb"),
]:
    grp = bpy.data.collections.new("Bladderlace original roots • " + energy)
    sc.collection.children.link(grp)
    prior = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(lace / name))
    for o in set(bpy.data.objects) - prior:
        for c in list(o.users_collection):
            c.objects.unlink(o)
        grp.objects.link(o)
    grp.hide_render = grp.hide_viewport = energy == "middle"
pose = json.loads((lace / "photos/lace-cameras.json").read_text())


def cv(p):
    return Vector((p[0], -p[2], p[1]))


cam = bpy.data.cameras.new("Genuine tide pocket")
cam.lens = pose["camera_lens_mm"]
cam.clip_end = 3400
o = bpy.data.objects.new(cam.name, cam)
sc.collection.objects.link(o)
o.location = cv(pose["eye"])
o.rotation_euler = (cv(pose["aim"]) - o.location).to_track_quat("-Z", "Y").to_euler()
sc.camera = o
for scr in bpy.data.screens:
    for a in scr.areas:
        if a.type == "VIEW_3D":
            v = a.spaces.active
            v.region_3d.view_location = cv(pose["aim"])
            v.region_3d.view_rotation = o.rotation_euler.to_quaternion()
            v.region_3d.view_distance = (cv(pose["aim"]) - o.location).length
            v.region_3d.view_perspective = "PERSP"
            v.overlay.show_overlays = False
n = bpy.data.texts.new("Original coastal roots / editing note")
n.write(
    "35 noninteractive frond roots in seven separated pockets on actual native V1. Full near and middle collections are provided for editing. Show ONLY ONE full collection at a time. The actual 1.04m photographic stage displays five near and eleven middle representative roots and an optical holdfast glaze at distance. No game objects, prey, brains or collisions altered.\n\n"
    + (lace / "lace-sites.json").read_text()
    + "\n"
    + (lace / "lace-budget.json").read_text()
)
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(out), compress=True)
