"""Slim Blender-only setup with held V1 terrain, actual placed props and locked lamps."""

from pathlib import Path
from importlib.util import spec_from_file_location, module_from_spec
import bpy, sys, json, math, struct
from mathutils import Vector

here = Path(__file__).resolve().parent
native, scene_dir, art, hollow, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:5]
)
mode = sys.argv[-1]
site = json.loads((art / "site.json").read_text())


def load_at(p, name):
    s = spec_from_file_location(name, p)
    m = module_from_spec(s)
    s.loader.exec_module(m)
    return m


def grab(p):
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(p.absolute()))
    return [o for o in set(bpy.data.objects) - old if o.type == "MESH"]


def cv(v):
    return Vector((v[0], -v[2], v[1]))


def cam(name, pose):
    _, eye, aim, span, lens, _ = pose
    c = bpy.data.cameras.new(name)
    o = bpy.data.objects.new(name, c)
    sc.collection.objects.link(o)
    o.location = cv(eye)
    o.rotation_euler = (cv(aim) - o.location).to_track_quat("-Z", "Y").to_euler()
    c.type = "ORTHO" if span else "PERSP"
    c.ortho_scale = max(0.01, span)
    c.lens = lens
    c.clip_end = 3500
    return o


def materials(obs):
    return {
        s.material
        for o in obs
        for s in o.material_slots
        if s.material and s.material.use_nodes
    }


def skin(obs, base=False):
    # Shared world-space bedding runs out of the terrain and over the exposed
    # lip. Without it individually shaded blue-grey caps look like pavers.
    m = load_at(here.parent / "leeward-passage/material.py", "chart")
    m._bind_layer(obs, art, (), "site.json", "rim-ground.png", 12.0, 0.014, 0.33)
    for m in materials(obs):
        nt = m.node_tree
        tx = next(
            (
                n
                for n in nt.nodes
                if n.type == "TEX_IMAGE"
                and n.image
                and n.image.name.startswith("rim-ground")
            ),
            None,
        )
        if base and not tx:
            continue
        bs = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
        mask = None
        if tx:
            mask = nt.nodes.new("ShaderNodeTexImage")
            mask.image = bpy.data.images.load(
                str(art / "bedding-weight.png"), check_existing=True
            )
            mask.image.colorspace_settings.name = "Non-Color"
            mask.extension = "CLIP"
            nt.links.new(tx.inputs["Vector"].links[0].from_socket, mask.inputs[0])
        p = nt.nodes.new("ShaderNodeNewGeometry")
        vec = nt.nodes.new("ShaderNodeVectorMath")
        vec.operation = "MULTIPLY"
        vec.inputs[1].default_value = (0.40, 0.46, 0.24)
        nt.links.new(p.outputs["Position"], vec.inputs[0])

        def tex(n, linear=False):
            t = nt.nodes.new("ShaderNodeTexImage")
            t.image = bpy.data.images.load(str(art / n), check_existing=True)
            if linear:
                t.image.colorspace_settings.name = "Non-Color"
            nt.links.new(vec.outputs[0], t.inputs[0])
            return t.outputs["Color"]

        col = bs.inputs["Base Color"]
        mix = nt.nodes.new("ShaderNodeMixRGB")
        mix.blend_type = "MULTIPLY"
        mix.inputs[0].default_value = 0.58
        if col.is_linked:
            nt.links.new(col.links[0].from_socket, mix.inputs[1])
        nt.links.new(tex("mineral-grain.png"), mix.inputs[2])
        nt.links.new(mix.outputs[0], col)
        norm = bs.inputs["Normal"]
        b = nt.nodes.new("ShaderNodeBump")
        b.inputs["Distance"].default_value = 0.021
        b.inputs["Strength"].default_value = 0.44
        nt.links.new(tex("mineral-lip.png", True), b.inputs["Height"])
        if norm.is_linked:
            nt.links.new(norm.links[0].from_socket, b.inputs["Normal"])
        if mask:
            for socket, gain in [(mix.inputs[0], 0.48), (b.inputs["Strength"], 0.38)]:
                k = nt.nodes.new("ShaderNodeMath")
                k.operation = "MULTIPLY"
                k.inputs[1].default_value = gain
                nt.links.new(mask.outputs["Color"], k.inputs[0])
                nt.links.new(k.outputs[0], socket)
        nt.links.new(b.outputs[0], norm)


def desk():
    raw = (native / "island-v1.bin").read_bytes()
    W, D, ox, oz, cs, _ = struct.unpack_from("<IIfffI", raw, 8)
    x0, x1, z0, z1 = site["bounds"]
    xi, xj = math.floor((x0 - ox) / cs) - 1, math.ceil((x1 - ox) / cs) + 1
    zi, zj = math.floor((z0 - oz) / cs) - 1, math.ceil((z1 - oz) / cs) + 1
    nx = xj - xi + 1
    v = []
    f = []
    for z in range(zi, zj + 1):
        for x in range(xi, xj + 1):
            v.append(
                (
                    ox + x * cs,
                    -oz - z * cs,
                    struct.unpack_from("<f", raw, 32 + 4 * (z * W + x))[0],
                )
            )
            if z < zj and x < xj:
                k = (z - zi) * nx + x - xi
                f.extend(((k, k + nx + 1, k + 1), (k, k + nx, k + nx + 1)))
    g = bpy.data.meshes.new("V1 unchanged triangles")
    g.from_pydata(v, [], f)
    g.update()
    o = bpy.data.objects.new("Original V1 rim, stationed at real coordinates", g)
    sc.collection.objects.link(o)
    m = bpy.data.materials.new("Workshop earth plus new registered PBR layers")
    m.use_nodes = True
    m.node_tree.nodes.get("Principled BSDF").inputs["Base Color"].default_value = (
        0.29,
        0.33,
        0.21,
        1,
    )
    o.data.materials.append(m)
    return [o]


bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
sc.render.engine = "CYCLES"
sc.cycles.samples = 24
sc.cycles.use_denoising = False
sc.cycles.seed = 18
sc.cycles.max_bounces = 3
sc.render.resolution_x = 840
sc.render.resolution_y = 500
sc.render.resolution_percentage = 100
sc.render.threads_mode = "FIXED"
sc.render.threads = 1
sc.view_settings.view_transform = "AgX"
sc.world = bpy.data.worlds.new("Held coastal daylight")
sc.world.use_nodes = True
sun = bpy.data.lights.new("Held island daylight", "SUN")
sun.energy = 2.2
sun.angle = 0.138
ob = bpy.data.objects.new("Sun", sun)
sc.collection.objects.link(ob)
pr = json.loads((native / "coastal-daylight.json").read_text())


def lin(c):
    return pow((c + 0.055) / 1.055, 2.4) if c > 0.04045 else c / 12.92


sun.color = [lin(v) for v in pr["sun_srgb"]]
ob.rotation_euler = cv(pr["sun_ray_direction"]).to_track_quat("-Z", "Y").to_euler()
bg = sc.world.node_tree.nodes["Background"]
bg.inputs["Color"].default_value = (*[lin(v) for v in pr["sky_fill_srgb"]], 1)
bg.inputs["Strength"].default_value = 0.78
cameras = {t: cam(t, q) for t, q in site["stations"].items()}
sc.camera = cameras["upper-rim"]


def shots(side):
    (out / side).mkdir(exist_ok=True, parents=True)
    for t, cam_ in cameras.items():
        if mode == "final" and side == "before" and t == "upper-rim":
            continue
        sc.camera = cam_
        sc.render.filepath = str(out / side / (t + ".png"))
        bpy.ops.render.render(write_still=True)


if mode in ("desk", "setup-desk"):
    ground = desk()
else:
    ground = grab(scene_dir / "ground.glb") + grab(scene_dir / "near.glb")
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
    m = bpy.data.materials.new("Held sea reference")
    m.use_nodes = True
    m.node_tree.nodes.get("Principled BSDF").inputs["Base Color"].default_value = (
        0.038,
        0.15,
        0.17,
        1,
    )
    o.data.materials.append(m)
    held = load_at(here.parent / "sunspindle-hollow/material.py", "sunspindle")
    held.coat(ground, hollow)
    grab(hollow / "registered-guardian.glb")
    p = json.loads((native / "props.json").read_text())
    pine = json.loads((native / "grove-crowns.json").read_text())
    sources = {}
    for i, t in enumerate(p):
        x, y, z = t["position"]
        kind = t["kind"]
        dist = math.hypot(x - 247, z + 125)
        tree = kind in ("Broadleaf", "Sapling", "Conifer")
        if dist > (
            250
            if tree
            else 155 if kind.startswith("Boulder") or kind in ("Cliff", "Scree") else 85
        ):
            continue
        use = kind == "Conifer" and str(i) in pine
        key = "PineForm" + str(pine[str(i)]["form"]) if use else kind
        lib = "grove-crowns" if use else kind
        if lib not in sources:
            sources[lib] = {i.name: i for i in grab(native / (lib + ".glb"))}
            for b in sources[lib].values():
                b.hide_render = True
        level = 2 if dist > 165 else 1 if dist > 80 else 0
        source = sources[lib][key + "_L" + str(level)]
        ins = source.copy()
        ins.hide_render = False
        sc.collection.objects.link(ins)
        ins.location = cv(t["position"])
        ins.rotation_mode = "XYZ"
        ins.rotation_euler = (0, 0, t["yaw"])
        ins.scale = (t["scale"],) * 3
    if mode not in ("after", "setup"):
        shots("before")
skin(ground, True)
skin(grab(art / "fitted-outcrops.glb"))
grab(art / "wind-heath.glb")
if mode == "desk":
    sc.camera = cameras["upper-rim"]
    note = bpy.data.texts.new("Read me")
    note.write(
        "True V1 cut, both locked cameras, linked heath and originally split riven beds. Full habitat lamps, trees and valley field appear in the photographic study, not counterfeit backdrop meshes in this focused editable.\n"
        + json.dumps(site, indent=2)
    )
    for s in bpy.data.screens:
        for a in s.areas:
            if a.type == "VIEW_3D":
                a.spaces.active.region_3d.view_rotation = (
                    sc.camera.rotation_euler.to_quaternion()
                )
                a.spaces.active.region_3d.view_location = (245, 123, 42)
                a.spaces.active.region_3d.view_distance = 115
                a.spaces.active.region_3d.view_perspective = "ORTHO"
                a.spaces.active.overlay.show_overlays = False
                a.spaces.active.shading.type = "MATERIAL"
    bpy.ops.file.pack_all()
    bpy.ops.wm.save_as_mainfile(
        filepath=str(out / "Lanternback-Rim.blend"), compress=True
    )
elif mode not in ("setup", "setup-desk"):
    shots("after")
