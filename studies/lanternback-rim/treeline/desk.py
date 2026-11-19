"""Small standalone Blender edit: the real 35×51 m bank, 36 roots and two artwork levels."""

import bpy, struct, math, sys, json
from pathlib import Path
from importlib.util import spec_from_file_location, module_from_spec
from mathutils import Vector

native, art, out = map(Path, sys.argv[sys.argv.index("--") + 1 :][:3])
here = Path(__file__).resolve().parent
bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
facts = json.loads((art / "lee.json").read_text())
survey = json.loads((art / "sightline.json").read_text())
a, b, c, d = facts["bounds"]
if (art / "bedding/bedding.json").exists():
    ra, rb, rc, rd = json.loads((art / "bedding/site.json").read_text())["bounds"]
    a, b, c, d = min(a, ra), max(b, rb), min(c, rc), max(d, rd)
raw = (native / "island-v1.bin").read_bytes()
w, _, ox, oz, step, _ = struct.unpack_from("<IIfffI", raw, 8)
xl, xr, zl, zr = (
    math.floor((a - ox) / step) - 1,
    math.ceil((b - ox) / step) + 1,
    math.floor((c - oz) / step) - 1,
    math.ceil((d - oz) / step) + 1,
)
row = xr - xl + 1
verts = []
faces = []
for j in range(zl, zr + 1):
    for i in range(xl, xr + 1):
        verts.append(
            (
                ox + i * step,
                -oz - j * step,
                struct.unpack_from("<f", raw, 32 + 4 * (j * w + i))[0],
            )
        )
        k = (j - zl) * row + i - xl
        if i < xr and j < zr:
            faces.extend(((k, k + row + 1, k + 1), (k, k + row, k + row + 1)))
mesh = bpy.data.meshes.new("Original diagonal topology · V1")
mesh.from_pydata(verts, [], faces)
mesh.update()
ground = bpy.data.objects.new("The unchanged windward bank", mesh)
sc.collection.objects.link(ground)
mat = bpy.data.materials.new("Ground wash on exact native elevations")
mat.use_nodes = True
mat.node_tree.nodes.get("Principled BSDF").inputs["Base Color"].default_value = (
    0.25,
    0.27,
    0.16,
    1,
)
mesh.materials.append(mat)
sp = spec_from_file_location("coat", here.parents[1] / "leeward-passage/material.py")
coat = module_from_spec(sp)
sp.loader.exec_module(coat)
if (art / "bedding/bedding.json").exists():
    coat._bind_layer(
        [ground], art / "bedding", (), "site.json", "rim-ground.png", 12.0, 0.014, 0.33
    )
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(
        filepath=str((art / "bedding/fitted-outcrops.glb").resolve())
    )
    rock = [ob for ob in set(bpy.data.objects) - before if ob.type == "MESH"]
    coat._bind_layer(
        rock, art / "bedding", (), "site.json", "rim-ground.png", 12.0, 0.014, 0.33
    )
coat._bind_layer([ground], art, (), "lee.json", "lee-soil.png", 30, 0.0025, 0.2)
for level in ("middle", "near"):
    coll = bpy.data.collections.new("Rooted forms · " + level)
    sc.collection.children.link(coll)
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str((art / ("lee-" + level + ".glb")).resolve()))
    for ob in set(bpy.data.objects) - before:
        for c in list(ob.users_collection):
            c.objects.unlink(ob)
        coll.objects.link(ob)
    coll.hide_viewport = coll.hide_render = level == "middle"
for idx, r in survey["distinct_roots"].items():
    o = bpy.data.objects.new("Held spruce " + idx + " · recorded root only", None)
    sc.collection.objects.link(o)
    x, y, z = r["position"]
    o.location = (x, -z, y)
    o.empty_display_size = 0.4
cam = bpy.data.cameras.new("58m guardian framing")
o = bpy.data.objects.new("Survey camera", cam)
sc.collection.objects.link(o)
o.location = (275, 149, 73)
o.rotation_euler = (
    (Vector((242.5, 170.9, 50)) - o.location).to_track_quat("-Z", "Y").to_euler()
)
cam.type = "ORTHO"
cam.ortho_scale = 58
sc.camera = o
txt = bpy.data.texts.new("Read the bank")
txt.write(
    "This is a focused native-height editing volume, not the full distant photo scene. The original four trees are location empties; they remain real native trees in the photographic pairs. A second collection holds the exact same rooted figures at a reduced detail level. No native asset registration or physics.\n\n"
    + json.dumps(facts, indent=2)
)
for s in bpy.data.screens:
    for ar in s.areas:
        if ar.type == "VIEW_3D":
            v = ar.spaces.active
            v.overlay.show_overlays = False
            v.shading.type = "MATERIAL"
            v.region_3d.view_distance = 44
            v.region_3d.view_location = (242.5, 170.9, 50)
            v.region_3d.view_rotation = o.rotation_euler.to_quaternion()
            v.region_3d.view_perspective = "ORTHO"
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(out / "Spruce-Lee.blend"), compress=True)
