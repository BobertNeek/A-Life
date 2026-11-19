"""Portable Blender editing desk, not a substitute for the full-island camera plates.

Call from Blender, after the cropped macro pigment and official study output
exist. Models in the final argument must be the original native island pack;
the optional comparison sheet is a packed image, never a viewport backdrop.
"""

from pathlib import Path
import csv
import json, math, struct, sys
import bpy
from mathutils import Vector

here = Path(__file__).resolve().parent
sys.path.insert(0, str(here))
from material import bind
from cameras import frame

surface, propsfile, detail, pigment, lib, folio, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :]
)
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.unit_settings.system = "METRIC"


def group(name):
    c = bpy.data.collections.new(name)
    scene.collection.children.link(c)
    return c


def attach(ob, c):
    for old in list(ob.users_collection):
        old.objects.unlink(ob)
    c.objects.link(ob)


def imp(path):
    existing = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path.resolve()))
    return set(bpy.data.objects) - existing


def native(p):
    return (p[0], -p[2], p[1])


raw = surface.read_bytes()
w, h, x0, z0, s, n = struct.unpack_from("<IIfffI", raw, 8)
a = struct.unpack_from(f"<{w*h}f", raw, 32)


def ground(x, z):
    u = (x - x0) / s
    v = (z - z0) / s
    i = int(math.floor(u))
    j = int(math.floor(v))
    u -= i
    v -= j
    aa, bb, cc, dd = (
        a[j * w + i],
        a[j * w + i + 1],
        a[(j + 1) * w + i + 1],
        a[(j + 1) * w + i],
    )
    return (
        aa + (bb - aa) * u + (cc - bb) * v
        if u >= v
        else aa + (cc - dd) * u + (dd - aa) * v
    )


# Native cut bounds align on its own grid, not a remeshed profile.
xl, xr, zt, zb = 43.75, 165.625, 250.0, 478.125
ii = range(round((xl - x0) / s), round((xr - x0) / s) + 1)
jj = range(round((zt - z0) / s), round((zb - z0) / s) + 1)
verts = [native((x0 + s * i, a[j * w + i], z0 + s * j)) for j in jj for i in ii]
faces = []
nx = len(ii)
for j in range(len(jj) - 1):
    for i in range(nx - 1):
        k = j * nx + i
        faces.extend(((k, k + nx + 1, k + 1), (k, k + nx, k + nx + 1)))
m = bpy.data.meshes.new("Real triangles · V1 / not a new island")
m.from_pydata(verts, [], faces)
m.update()
ob = bpy.data.objects.new("01 · Original dry lip and southern strand", m)
group("01 · Ordinary V1 cut").objects.link(ob)
uv = m.uv_layers.new(name="Native macro crop")
for p in m.polygons:
    for i in p.loop_indices:
        x, y, z = verts[m.loops[i].vertex_index]
        uv.data[i].uv = ((x - xl) / (xr - xl), (zb + y) / (zb - zt))
base = bpy.data.materials.new("Native macro pigment crop + optional studio wash")
base.use_nodes = True
t = base.node_tree.nodes.new("ShaderNodeTexImage")
t.image = bpy.data.images.load(str(pigment.resolve()))
p = base.node_tree.nodes.get("Principled BSDF")
base.node_tree.links.new(t.outputs["Color"], p.inputs["Base Color"])
p.inputs["Roughness"].default_value = 0.9
m.materials.append(base)
bind([ob], detail)
g = group("02 · Wind growth, gravel and hinged shells")
for o in imp(detail / "reach-detail.glb"):
    attach(o, g)
nurses = group("02b · Connected dune scrub bodies")
for o in imp(detail / "reach-scrub.glb"):
    attach(o, nurses)
props = json.loads(propsfile.read_text())
# The same real ground witnesses as in the photographed view, grouped by
# native model. Real yaw, uniform scale and original ground placement.
stock = group("03 · Original reusable witnesses")
stock.hide_viewport = stock.hide_render = True
installed = group("04 · Original placed witnesses")
refs = {}
nearby = [
    (i, q)
    for i, q in enumerate(props)
    if xl + 1 < q["position"][0] < xr - 1
    and zt + 1 < q["position"][2] < zb - 1
    and q["kind"]
    in ("Broadleaf", "Sapling", "BoulderA", "BoulderB", "BoulderC", "Scree")
]
for idx, q in nearby:
    name = q["kind"]
    if name not in refs:
        got = imp(lib / (name + ".glb"))
        refs[name] = next(o for o in got if o.type == "MESH" and o.name == name + "_L1")
        for model in got:
            if model != refs[name]:
                bpy.data.objects.remove(model, do_unlink=True)
            else:
                attach(model, stock)
    model = refs[name]
    inst = bpy.data.objects.new(f"{name} #{idx} · old world placement", model.data)
    installed.objects.link(inst)
    inst.location = native(q["position"])
    inst.rotation_euler.z = q["yaw"]
    inst.scale = (q["scale"],) * 3
# Keep the route an optional editor guide, not visible in the pictures.
guide = group("05 · Route curve / toggle in editor")
g = bpy.data.curves.new("Surveyed dry guide, not a game path", "CURVE")
g.dimensions = "3D"
sp = g.splines.new("POLY")
with (pigment.parent / "stations.csv").open(newline="") as f:
    pts = [
        (float(row["native_x"]), float(row["native_z"])) for row in csv.DictReader(f)
    ]
sp.points.add(len(pts) - 1)
for k, (x, z) in enumerate(pts):
    sp.points[k].co = (*native((x, ground(x, z) + 0.08, z)), 1)
obj = bpy.data.objects.new("218.88 m surveyed centre", g)
guide.objects.link(obj)
guide.hide_render = True
guide.hide_viewport = True
cameras = group("06 · Three matched full-island camera stations")
for name in ("reach-guardian", "reach-meadow", "reach-strand"):
    _, e, t, ortho, mm, _ = frame(name, ground)
    cam = bpy.data.cameras.new(name)
    cam.clip_end = 2000
    cam.type = "ORTHO" if ortho else "PERSP"
    cam.ortho_scale = max(0.1, ortho)
    cam.lens = mm
    c = bpy.data.objects.new(name, cam)
    cameras.objects.link(c)
    c.location = native(e)
    c.rotation_euler = (
        (Vector(native(t)) - c.location).to_track_quat("-Z", "Y").to_euler()
    )
    if name == "reach-guardian":
        scene.camera = c
# The editing slab is deliberately local. No rough ocean plane or painted
# backdrop is introduced in place of the real whole-island artist photos.
for scr in bpy.data.screens:
    for area in scr.areas:
        if area.type == "VIEW_3D":
            s = area.spaces.active
            s.region_3d.view_location = Vector((103, -371, 5))
            s.region_3d.view_distance = 210
            s.region_3d.view_rotation = Vector((-172, 77, -147)).to_track_quat(
                "-Z", "Y"
            )
            s.shading.type = "MATERIAL"
photo = bpy.data.images.load(str(folio.resolve()))
photo.name = "Shellback Reach · matched full-island photographs"
photo.use_fake_user = True
photo.pack()
work = bpy.data.workspaces.get("Rendering")
if work:
    work.name = "Full-island photographs"
    for scr in work.screens:
        for area in scr.areas:
            if area.type == "IMAGE_EDITOR":
                area.spaces.active.image = photo
text = bpy.data.texts.new("Start here · Shellback Reach")
text.write(
    "Layout: editable detail organized in eight longitudinal parts, literal cropped V1 triangles, real-position trees and stones, exact camera stations. The cropped editor uses the native macro pigment, not the full island's close material, sea or lighting. The Full-island photographs workspace contains all three matched frames on the actual island. Only the optional pigment, ground tooth and cosmetic leaves/valves/shingle change. No collider, V1 terrain identity, native registration, placement or save was edited. This is artwork, not a playable trail.\n"
)
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(out.resolve()), compress=True)
