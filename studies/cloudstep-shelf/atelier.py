"""Build the planned full V1 cloud-shelf artist photographs in Blender.

Optional, not executed by a build/CI. The same original copse roots, actual
pines and boulders, stock native sky and V1 sea remain on both sides. The
first scoped pass modifies only the locally registered upright rock contacts.
"""

import json, math, runpy, sys
from pathlib import Path
from importlib.util import spec_from_file_location, module_from_spec
import bpy
from mathutils import Vector

H = Path(__file__).resolve().parent
args = sys.argv[sys.argv.index("--") + 1 :]
seat = len(args) == 7 and args[-1] == "with-seated-shoulders"
show_forest = seat or (len(args) == 7 and args[-1] == "with-registered-floor")
if show_forest:
    args = args[:-1]
if len(args) != 6:
    raise ValueError(
        "NATIVE STAGED_TRUE_TERRAIN HELD_COPSE_FILE WATER_INPUTS SITE_OUTPUT PHOTOGRAPHS"
    )
native, land, copse, water, source, out = map(Path, args)


def mod(tag, file):
    spec = spec_from_file_location(tag, file)
    m = module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


surf = mod("held_coastal_optics", H.parent / "eastern-scarp/material.py")


def cv(p):
    return Vector((p[0], -p[2], p[1]))


def grab(p):
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(p.resolve()))
    return [a for a in set(bpy.data.objects) - old if a.type == "MESH"]


def srgb(a):
    return ((a + 0.055) / 1.055) ** 2.4 if a > 0.04045 else a / 12.92


bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
sc.render.engine = "CYCLES"
sc.cycles.samples = 24
sc.cycles.seed = 67
sc.cycles.max_bounces = 3
sc.cycles.use_denoising = False
sc.render.threads_mode = "FIXED"
sc.render.threads = 1
sc.render.resolution_x = 840
sc.render.resolution_y = 500
sc.render.resolution_percentage = 100
sc.view_settings.view_transform = "AgX"
day = json.loads((native / "coastal-daylight.json").read_text())
sc.world = bpy.data.worlds.new("Held actual coastal fill")
sc.world.use_nodes = True
bg = sc.world.node_tree.nodes["Background"]
bg.inputs[0].default_value = (*[srgb(v) for v in day["sky_fill_srgb"]], 1)
bg.inputs[1].default_value = 0.78
lamp = bpy.data.lights.new("Held actual sun direction", "SUN")
lamp.color = [srgb(v) for v in day["sun_srgb"]]
lamp.energy = 2.2
lamp.angle = 0.138
sun = bpy.data.objects.new(lamp.name, lamp)
sc.collection.objects.link(sun)
sun.rotation_euler = cv(day["sun_ray_direction"]).to_track_quat("-Z", "Y").to_euler()
ground = grab(land / "ground.glb") + grab(land / "near.glb")
# Do not replace the original 9,456 native root/forest-floor triangles with
# a blank baseline. The archived copse SHA is specified in underlay.py.
grab(copse)
for o in grab(native / "marine-day.glb"):
    for k in (
        "visible_shadow",
        "visible_diffuse",
        "visible_glossy",
        "visible_transmission",
        "visible_volume_scatter",
    ):
        setattr(o, k, False)
bpy.ops.mesh.primitive_plane_add(size=6000)
o = bpy.context.object
o.name = "Native y=0 ocean"
surf.sea(o, water)
props = json.loads((native / "props.json").read_text())
crowns = json.loads((native / "grove-crowns.json").read_text())
lib = {}
old_shoulders = []
for i, q in enumerate(props):
    x, y, z = q["position"]
    dist = math.hypot(x - 71.0, z + 379.0)
    kind = q["kind"]
    if dist > (
        265
        if kind in ("Conifer", "Broadleaf", "Sapling")
        else (185 if kind.startswith("Boulder") or kind in ("Cliff", "Scree") else 115)
    ):
        continue
    crown = crowns.get(str(i)) if kind == "Conifer" else None
    asset = "grove-crowns" if crown else kind
    if asset not in lib:
        lib[asset] = {v.name: v for v in grab(native / (asset + ".glb"))}
        for ob in lib[asset].values():
            ob.hide_render = True
    name = (
        ("PineForm" + str(crown["form"]) if crown else kind)
        + "_L"
        + str(0 if dist < 70 else 1 if dist < 156 else 2)
    )
    ob = lib[asset][name].copy()
    sc.collection.objects.link(ob)
    ob.hide_render = False
    ob.location = cv(q["position"])
    ob.rotation_mode = "XYZ"
    ob.rotation_euler = (0, 0, q["yaw"])
    ob.scale = (q["scale"],) * 3
    if i in (1335, 1695, 1847):
        old_shoulders.append(ob)
poses = json.loads((source / "views.json").read_text())
cams = {}
for k, q in poses.items():
    cam = bpy.data.cameras.new(k)
    cam.lens = q["lens"]
    cam.clip_end = 3600
    cam.type = "ORTHO" if q["span"] else "PERSP"
    cam.ortho_scale = max(0.001, q["span"])
    ob = bpy.data.objects.new(k, cam)
    sc.collection.objects.link(ob)
    ob.location = cv(q["eye"])
    ob.rotation_euler = (cv(q["aim"]) - ob.location).to_track_quat("-Z", "Y").to_euler()
    cams[k] = ob
for side in ("before", "after"):
    (out / side).mkdir(exist_ok=True, parents=True)
    if side == "after":
        mod("shelf_source_coat", H / "material.py").coat(ground, surf, source)
        if show_forest:
            mod("held_pine_duff", H / "forest.py").coat(ground, surf, source)
    husks = (
        grab(source / "cone-husks-eye.glb") if side == "after" and show_forest else []
    )
    new_ledge = {}
    if seat and side == "after":
        for ob in old_shoulders:
            ob.hide_render = True
        for lod in (0, 1):
            new_ledge[lod] = grab(source / ("slope-shoulders-L" + str(lod) + ".glb"))
        fmod = mod("cloudstep_optical_stone", H / "forest.py")
        fmod.tint(
            ground,
            surf,
            source / "shoulder-talus.png",
            json.loads((source / "shoulders.json").read_text())["talus"][
                "native_plan_xz"
            ],
            "Unbroken steep talus from the three original cliff seats",
        )
    for name, cam in cams.items():
        for lod, objs in new_ledge.items():
            for ob in objs:
                ob.hide_render = lod != int(name == "shelf-section")
        for ob in husks:
            ob.hide_render = name == "shelf-section"
        sc.camera = cam
        sc.render.filepath = str(out / side / (name + ".png"))
        bpy.ops.render.render(write_still=True)
(out / "capture-method.json").write_text(
    json.dumps(
        dict(
            native_gameplay=False,
            Blender_full_stage=True,
            held_stock_sky_water=True,
            original_terrain_and_props=True,
            source_copse_footprint_triangles=9456,
            only_change=(
                "optical contacts on mountain ground; original-needlefall glaze and small spent cones"
                if show_forest
                else "optical contact weathering on eligible mountain ground"
            ),
            visual_shoulder_seats=(list((1335, 1695, 1847)) if seat else []),
            poses=poses,
        ),
        indent=2,
    )
    + "\n"
)
