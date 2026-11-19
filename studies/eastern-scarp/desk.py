"""Portable native-height coast cut, original water textures, editable vertical material."""

from pathlib import Path
from importlib.util import spec_from_file_location, module_from_spec
import bpy, math, struct, sys, json
from mathutils import Vector

here = Path(__file__).resolve().parent
native, shelf, field, photo = map(Path, sys.argv[sys.argv.index("--") + 1 :][:4])
sp = spec_from_file_location("edit_skin", here / "material.py")
surf = module_from_spec(sp)
sp.loader.exec_module(surf)
bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
raw = (native / "island-v1.bin").read_bytes()
w, _, ox, oz, step, _ = struct.unpack_from("<IIfffI", raw, 8)
a, b, c, d = json.loads((field / "face.json").read_text())["plan"]
i0, i1 = math.floor((a - ox) / step) - 2, math.ceil((b - ox) / step) + 4
j0, j1 = math.floor((c - oz) / step) - 2, math.ceil((d - oz) / step) + 2
n = i1 - i0 + 1
v = []
faces = []
for j in range(j0, j1 + 1):
    for i in range(i0, i1 + 1):
        k = (j - j0) * n + i - i0
        v.append(
            (
                ox + i * step,
                -oz - j * step,
                struct.unpack_from("<f", raw, 32 + 4 * (j * w + i))[0],
            )
        )
        if j < j1 and i < i1:
            faces.extend(((k, k + n + 1, k + 1), (k, k + n, k + n + 1)))
m = bpy.data.meshes.new("Original V1 coastal diagonals")
m.from_pydata(v, [], faces)
m.update()
ob = bpy.data.objects.new("Gullglass Head / actual exposed and submerged ground", m)
sc.collection.objects.link(ob)
base = bpy.data.materials.new("Break in the native face / local editable grade")
base.use_nodes = True
base.node_tree.nodes.get("Principled BSDF").inputs["Base Color"].default_value = (
    0.29,
    0.30,
    0.22,
    1.0,
)
m.materials.append(base)
surf.stone([ob], field)
# Only the water inside the desk. The photo scene has a full horizon, not a
# floating seascape or a placed blue backdrop within this editable study.
p0, p1 = ox + i0 * step, ox + i1 * step
q0, q1 = oz + j0 * step, oz + j1 * step
mesh = bpy.data.meshes.new("Genuine sea-level surface")
mesh.from_pydata(
    [(p0, -q0, 0), (p1, -q0, 0), (p1, -q1, 0), (p0, -q1, 0)], [], [(3, 2, 1, 0)]
)
mesh.update()
water = bpy.data.objects.new("Water level zero, native maps", mesh)
sc.collection.objects.link(water)
surf.sea(water, shelf)
manifest = json.loads((field / "shore.json").read_text())
for name, q in manifest["landmarks"].items():
    x, y, z = q["position"]
    o = bpy.data.objects.new(
        "Original " + q["kind"] + " " + name + " at recorded root", None
    )
    sc.collection.objects.link(o)
    o.location = (x, -z, y)
    o.empty_display_size = 0.35
for name, (eye, aim, span, lens) in json.loads(
    (photo / "photography.json").read_text()
)["cameras"].items():
    c = bpy.data.cameras.new(name)
    o = bpy.data.objects.new(name, c)
    sc.collection.objects.link(o)
    o.location = (eye[0], -eye[2], eye[1])
    o.rotation_euler = (
        (Vector((aim[0], -aim[2], aim[1])) - o.location)
        .to_track_quat("-Z", "Y")
        .to_euler()
    )
    c.type = "ORTHO" if span else "PERSP"
    c.ortho_scale = max(span, 0.01)
    c.lens = lens
    if name == "outside":
        sc.camera = o
note = bpy.data.texts.new("What this desk preserves")
note.write(
    "The edit contains real native V1 terrain triangles, unclipped real sea bathymetry inside the desk extent, unchanged native shelf albedo/normal, the side-elevation pigment and mask, and the two photographic cameras. Original tree origins are marked; the full tree meshes, sky and distant island appear in the photographs, not this compact edit.\n\n"
    + json.dumps(manifest, indent=2)
)
for scr in bpy.data.screens:
    for a in scr.areas:
        if a.type == "VIEW_3D":
            v = a.spaces.active
            v.overlay.show_overlays = False
            v.shading.type = "MATERIAL"
            v.region_3d.view_rotation = sc.camera.rotation_euler.to_quaternion()
            v.region_3d.view_location = (397, 116, 10)
            v.region_3d.view_distance = 104
            v.region_3d.view_perspective = "ORTHO"
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(photo / "Gullglass-Head.blend"), compress=True)
