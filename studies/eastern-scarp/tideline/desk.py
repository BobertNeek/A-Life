"""Compact on-site editable: original Gullglass desk, both exact LODs and the 1.04 m foothold."""

import bpy, csv, json, runpy, sys
from pathlib import Path
from mathutils import Vector

H = Path(__file__).resolve().parent
desk, cups, photography, out = [
    Path(a).resolve() for a in sys.argv[sys.argv.index("--") + 1 :][:4]
]
bpy.ops.wm.open_mainfile(filepath=str(desk))
sc = bpy.context.scene
contact = runpy.run_path(str(H / "contact_material.py"))
from importlib.util import spec_from_file_location, module_from_spec

sp = spec_from_file_location("gullglass_surface", H.parent / "material.py")
surf = module_from_spec(sp)
sp.loader.exec_module(surf)
ground = [
    ob
    for ob in sc.objects
    if ob.type == "MESH" and ob.data.name.startswith("Original V1 coastal diagonals")
]
if not ground:
    raise ValueError(
        "This desk requires the actual Gullglass V1 photo-cut, not a display plane"
    )
contact["bind"](ground, surf, cups)
for lod in ["near", "middle"]:
    group = bpy.data.collections.new("Tidal casts • " + lod + " LOD")
    sc.collection.children.link(group)
    prior = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(cups / ("cups-" + lod + ".glb")))
    for ob in set(bpy.data.objects) - prior:
        for c in list(ob.users_collection):
            c.objects.unlink(ob)
        group.objects.link(ob)
    group.hide_render = lod != "near"
    group.hide_viewport = lod != "near"


def cv(p):
    return Vector((p[0], -p[2], p[1]))


angles = json.loads((photography / "angles.json").read_text())
eye, aim = angles["standing_native_eye"], angles["close_target"]
cam = contact["close_camera"](sc, cv, eye, aim)
sc.camera = cam
sc.render.resolution_x = 840
sc.render.resolution_y = 500
sc.render.resolution_percentage = 100
for scr in bpy.data.screens:
    for ar in scr.areas:
        if ar.type == "VIEW_3D":
            v = ar.spaces.active
            v.region_3d.view_rotation = cam.rotation_euler.to_quaternion()
            v.region_3d.view_location = cv(aim)
            v.region_3d.view_distance = (cv(aim) - cv(eye)).length
            v.region_3d.view_perspective = "PERSP"
instruction = bpy.data.texts.new("Tide pocket field book")
instruction.write(
    "Original terrain cut from the V1 ground archive, upright true sea level and thirteen original conifer markers inherited from Gullglass Head. These are inanimate, decorative casts. The two LOD collections have identical placed origin, quaternion and radius for each form. Hide near BEFORE showing middle. The full sky, trees and distant coast are photo-stage assets, not part of this portable desk. Neither LOD has a gameplay binding.\n\n"
    + (cups / "shell-contacts.json").read_text()
    + "\n\n"
    + (cups / "shells.json").read_text()
)
sc["gameplay_registration"] = False
bpy.ops.file.pack_all()
out.parent.mkdir(exist_ok=True, parents=True)
bpy.ops.wm.save_as_mainfile(filepath=str(out), compress=True)
