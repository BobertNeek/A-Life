"""Portable artist's desk: original V1 terrain cut, planted shingle and edge.

The lit gallery is produced with the real island, not this intentionally
small, neutral-material editing file. No full island, atlas or game required
to reopen its individual stones or the bent woodland ground-cover.
"""

from pathlib import Path
import json, math, struct, sys
import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from studio_woodland_edge_material import bind

surface, placements, cleaved, seated, growth, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :]
)
props = json.loads(placements.read_text())
notes = json.loads((growth / "woodland-source.json").read_text())
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.unit_settings.system = "METRIC"


def group(name):
    g = bpy.data.collections.new(name)
    scene.collection.children.link(g)
    return g


def imported(path, collection):
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path.resolve()))
    objs = set(bpy.data.objects) - old
    for obj in objs:
        for c in list(obj.users_collection):
            c.objects.unlink(obj)
        collection.objects.link(obj)
    return objs


def native(p):
    return (p[0], -p[2], p[1])


def inspectable(ob):
    # The original normals and UV seams stay split; vertex groups, not an
    # edit-mode mesh weld, let a sculptor select individual physical pieces.
    xyz = [tuple(round(v, 6) for v in a.co) for a in ob.data.vertices]
    parent = {v: v for v in xyz}

    def top(v):
        while parent[v] != v:
            parent[v] = parent[parent[v]]
            v = parent[v]
        return v

    for e in ob.data.edges:
        i, j = (top(xyz[n]) for n in e.vertices)
        if i != j:
            parent[i] = j
    parts = {}
    for i, v in enumerate(xyz):
        parts.setdefault(top(v), []).append(i)
    for i, k in enumerate(
        sorted(parts.values(), key=lambda ids: min(xyz[j][0] for j in ids))
    ):
        ob.vertex_groups.new(name=f"Clast {i+1:02}").add(k, 1.0, "REPLACE")


p = props[1303]
for path, label, hidden in [
    (cleaved, "Unseated alternative (toggle)", True),
    (seated, "22 planted fragments at original #1303", False),
]:
    g = group(label)
    root = bpy.data.objects.new("Original 1303 placement · " + label, None)
    g.objects.link(root)
    root.location = native(p["position"])
    root.rotation_euler.z = p["yaw"]
    root.scale = (p["scale"],) * 3
    for obj in imported(path, g):
        if obj.type == "MESH":
            inspectable(obj)
            obj.hide_render = obj.hide_viewport = not obj.name.endswith("_L0")
        if not obj.parent:
            obj.parent = root
    g.hide_viewport = g.hide_render = hidden
raw = surface.read_bytes()
w, d, x0, z0, s, _ = struct.unpack_from("<IIfffI", raw, 8)
heights = struct.unpack_from(f"<{w*d}f", raw, 32)
xmin, xmax, zmin, zmax = notes["bounds"]
xs = range(math.floor((xmin - x0) / s), math.ceil((xmax - x0) / s) + 1)
zs = range(math.floor((zmin - z0) / s), math.ceil((zmax - z0) / s) + 1)
verts = [native((x0 + i * s, heights[j * w + i], z0 + j * s)) for j in zs for i in xs]
faces = []
nx = len(xs)
for j in range(len(zs) - 1):
    for i in range(nx - 1):
        a = j * nx + i
        faces.extend(((a, a + nx + 1, a + 1), (a, a + nx, a + nx + 1)))
m = bpy.data.meshes.new("Original V1 triangle vertices")
m.from_pydata(verts, [], faces)
m.update()
piece = bpy.data.objects.new("True cropped V1 · neutral reference material", m)
group("Editable original terrain pitch").objects.link(piece)
mat = bpy.data.materials.new("Reference turf only · not the ingame material")
mat.use_nodes = True
mat.node_tree.nodes.get("Principled BSDF").inputs["Base Color"].default_value = (
    0.162,
    0.183,
    0.098,
    1,
)
m.materials.append(mat)
bind([piece], growth)
imported(growth / "woodland-edge.glb", group("Bent rushes and low woodland runners"))
markers = group("True landmarks · empties, not the gallery models")
for i in (111, 131, 2510, 1383, 2835, 3091):
    p = props[i]
    o = bpy.data.objects.new(f"{p['kind']} {i} · original", None)
    o.empty_display_type = "PLAIN_AXES"
    o.empty_display_size = 0.55
    o.location = native(p["position"])
    o.rotation_euler.z = p["yaw"]
    o.scale = (p["scale"],) * 3
    markers.objects.link(o)
focus = Vector(native((258, 41.9, -199)))
for a in bpy.context.screen.areas if bpy.context.screen else []:
    if a.type == "VIEW_3D":
        sp = a.spaces.active
        sp.region_3d.view_location = focus
        sp.region_3d.view_distance = 34
        sp.region_3d.view_rotation = Vector((-31, -11, -15)).to_track_quat("-Z", "Y")
        sp.shading.type = "MATERIAL"
scene["read me"] = (
    "Neutral editable true V1 cut; use gallery photos for the real terrain albedo, light and trees. World X/Z props, original placement yaw/scale, sole artwork; no game data or upgraded physics."
)
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(out.resolve()), compress=True)
