"""Actual native positional stage of Gullglass Head; held native water and terrain on both sides."""

from pathlib import Path
import math, sys, json, struct
from importlib.util import spec_from_file_location, module_from_spec
import bpy
from mathutils import Vector

here = Path(__file__).resolve().parent
native, stage, shelf, hollow, field, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:6]
)
mode = sys.argv[-1]


def module(label, path):
    s = spec_from_file_location(label, path)
    m = module_from_spec(s)
    s.loader.exec_module(m)
    return m


surf = module("headland_skin", here / "material.py")


def grab(p):
    prior = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(p.resolve()))
    return [a for a in set(bpy.data.objects) - prior if a.type == "MESH"]


def cv(p):
    return Vector((p[0], -p[2], p[1]))


def linear(c):
    return ((c + 0.055) / 1.055) ** 2.4 if c > 0.04045 else c / 12.92


bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
sc.render.engine = "CYCLES"
sc.cycles.samples = 24
sc.cycles.seed = 18
sc.cycles.max_bounces = 3
sc.cycles.use_denoising = False
sc.render.threads_mode = "FIXED"
sc.render.threads = 1
sc.render.resolution_x = 840
sc.render.resolution_y = 500
sc.render.resolution_percentage = 100
sc.view_settings.view_transform = "AgX"
pr = json.loads((native / "coastal-daylight.json").read_text())
sc.world = bpy.data.worlds.new("Held sea daylight")
sc.world.use_nodes = True
bg = sc.world.node_tree.nodes["Background"]
bg.inputs[0].default_value = (*[linear(v) for v in pr["sky_fill_srgb"]], 1)
bg.inputs[1].default_value = 0.78
d = bpy.data.lights.new("Held soft daylight", "SUN")
d.color = [linear(v) for v in pr["sun_srgb"]]
d.energy = 2.2
d.angle = 0.138
ob = bpy.data.objects.new(d.name, d)
sc.collection.objects.link(ob)
ob.rotation_euler = cv(pr["sun_ray_direction"]).to_track_quat("-Z", "Y").to_euler()
ground = grab(stage / "ground.glb") + grab(stage / "near.glb")
old = module("hollow_coat", here.parent / "sunspindle-hollow/material.py")
old.coat(ground, hollow)
grab(hollow / "registered-guardian.glb")
for ob in grab(native / "marine-day.glb"):
    for k in (
        "visible_shadow",
        "visible_diffuse",
        "visible_glossy",
        "visible_transmission",
        "visible_volume_scatter",
    ):
        setattr(ob, k, False)
bpy.ops.mesh.primitive_plane_add(size=6000)
o = bpy.context.object
o.name = "Held native V1 sea level"
surf.sea(o, shelf)
orig = json.loads((native / "props.json").read_text())
crowns = json.loads((native / "grove-crowns.json").read_text())
sources = {}
for i, t in enumerate(orig):
    x, y, z = t["position"]
    dist = math.hypot(x - 397, z + 116)
    kind = t["kind"]
    radius = (
        240
        if kind in ("Conifer", "Broadleaf", "Sapling")
        else 130 if kind.startswith("Boulder") or kind in ("Cliff", "Scree") else 78
    )
    if dist > radius:
        continue
    use = kind == "Conifer" and str(i) in crowns
    lib = "grove-crowns" if use else kind
    key = "PineForm" + str(crowns[str(i)]["form"]) if use else kind
    if lib not in sources:
        sources[lib] = {q.name: q for q in grab(native / (lib + ".glb"))}
        for q in sources[lib].values():
            q.hide_render = True
    src = sources[lib][key + "_L" + str(2 if dist > 168 else 1 if dist > 77 else 0)]
    ins = src.copy()
    sc.collection.objects.link(ins)
    ins.hide_render = False
    ins.location = cv(t["position"])
    ins.rotation_mode = "XYZ"
    ins.rotation_euler = (0, 0, t["yaw"])
    ins.scale = (t["scale"],) * 3
raw = (native / "island-v1.bin").read_bytes()
w, _, ox, oz, step, _ = struct.unpack_from("<IIfffI", raw, 8)


def floor(x, z):
    a, b = (x - ox) / step, (z - oz) / step
    i, j = math.floor(a), math.floor(b)
    a -= i
    b -= j

    def at(i, j):
        return struct.unpack_from("<f", raw, 32 + 4 * (j * w + i))[0]

    s, t, v, u = at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)
    return s + (t - s) * a + (v - t) * b if a >= b else s + (v - u) * a + (u - s) * b


close = (409.6, floor(409.6, -97.0) + 1.04, -97.0)
poses = {
    "outside": [[485, 47, -147], [397, 10, -116], 119, 42],
    "wave-eye": [list(close), [403.25, 4.8, -116.8], 0, 29],
}
cams = {}
for name, (eye, aim, span, lens) in poses.items():
    c = bpy.data.cameras.new(name)
    o = bpy.data.objects.new(name, c)
    sc.collection.objects.link(o)
    o.location = cv(eye)
    o.rotation_euler = (cv(aim) - o.location).to_track_quat("-Z", "Y").to_euler()
    c.type = "ORTHO" if span else "PERSP"
    c.ortho_scale = max(span, 0.001)
    c.lens = lens
    c.clip_end = 3600
    cams[name] = o


def photos(side):
    (out / side).mkdir(parents=True, exist_ok=True)
    for name, cam in cams.items():
        sc.camera = cam
        sc.render.filepath = str(out / side / (name + ".png"))
        bpy.ops.render.render(write_still=True)


if mode != "after":
    photos("before")
surf.stone(ground, field)
photos("after")
(out / "photography.json").write_text(
    json.dumps(
        dict(
            method="Blender 4.3.2, one CPU thread, 840x500 at 24 Cycles samples; both sides use actual V1 shelf colour and 58m normal, identical native-position trees and lighting",
            cameras=poses,
            stock_sea_material=True,
            terrain_altered=False,
            native_gameplay_picture=False,
        ),
        indent=2,
    )
    + "\n"
)
