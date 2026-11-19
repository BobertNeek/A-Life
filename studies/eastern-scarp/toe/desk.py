"""One editable coastal cut, retaining both pocket LODs and the three new named optical surfaces."""

from pathlib import Path
import sys, bpy, runpy, json
from importlib.util import spec_from_file_location, module_from_spec

H = Path(__file__).resolve().parent
prior, base, upper, toe, cups, wash, out = map(
    lambda s: Path(s).resolve(), sys.argv[sys.argv.index("--") + 1 :][:7]
)
bpy.ops.wm.open_mainfile(filepath=str(prior))
sc = bpy.context.scene
ground = [
    o
    for o in sc.objects
    if o.type == "MESH" and o.data.name.startswith("Original V1 coastal diagonals")
]
if not ground:
    raise ValueError("Requires the unaltered Gullglass-Head.blend desk")
spec = spec_from_file_location("gullglass_optical_base", H.parent / "material.py")
surf = module_from_spec(spec)
spec.loader.exec_module(surf)
for p, f in [(H.parent / "outfold/material.py", upper), (H / "bind.py", toe)]:
    runpy.run_path(str(p))["bind"](ground, surf, base, f)
runpy.run_path(str(H.parent / "tideline/contact_material.py"))["bind"](
    ground, surf, cups
)
for lod in ("near", "middle"):
    grp = bpy.data.collections.new("Tide pockets / " + lod)
    sc.collection.children.link(grp)
    prev = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(cups / ("cups-" + lod + ".glb")))
    for o in set(bpy.data.objects) - prev:
        for c in list(o.users_collection):
            c.objects.unlink(o)
        grp.objects.link(o)
    grp.hide_viewport = grp.hide_render = lod != "near"
water = next(
    o
    for o in sc.objects
    if o.type == "MESH" and o.data.name.startswith("Genuine sea-level surface")
)
runpy.run_path(str(H.parent / "wash/material.py"))["bind"](water, surf, wash)
sc.camera = sc.objects["wave-eye"]
for scr in bpy.data.screens:
    for a in scr.areas:
        if a.type == "VIEW_3D":
            v = a.spaces.active
            v.region_3d.view_location = (408.3, 99.0, 1.6)
            v.region_3d.view_rotation = sc.camera.rotation_euler.to_quaternion()
            v.region_3d.view_distance = 3.4
            v.region_3d.view_perspective = "PERSP"
note = bpy.data.texts.new("Eastern shore / arrangement")
note.write(
    "Original native terrain and two original camera transforms. The reference sky and distant tree geometry belong to the full photographic setup (native roots are marked in the desk). The shallow wash is an explicitly new optical proposal on top of, not identical to, the stock water. No geometry, path or save changed.\n\n"
    + (toe / "toe.json").read_text()
    + "\n\n"
    + (wash / "wash.json").read_text()
)
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(out), compress=True)
