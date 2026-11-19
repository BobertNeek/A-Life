"""Lock the review camera and light for terrain GLBs (Blender, CPU; not a gameplay capture)."""

import argparse
import json
import struct
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

cli = argparse.ArgumentParser(description=__doc__)
cli.add_argument("glb", type=Path)
cli.add_argument("out", type=Path)
cli.add_argument("--wide-only", action="store_true")
cli.add_argument("--creature-height", action="store_true")
cli.add_argument(
    "--tidal-review", action="store_true", help="Two fixed views of the eastern strand"
)
cli.add_argument(
    "--strandline", type=Path, help="Optional actual strand GLB for matched art review"
)
cli.add_argument(
    "--handoff",
    action="store_true",
    help="hold the camera at the river head and vary only surface LOD",
)
cli.add_argument("--surface", type=Path)
cli.add_argument("--placements", type=Path)
cli.add_argument("--prop-library", type=Path)
args = cli.parse_args(sys.argv[sys.argv.index("--") + 1 :])
args.out.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.render.engine = "CYCLES"
scene.cycles.device = "CPU"
scene.cycles.samples = 12
scene.cycles.use_denoising = False
scene.cycles.seed = 27
scene.render.threads_mode = "FIXED"
scene.render.threads = 2
scene.render.resolution_x, scene.render.resolution_y = 1000, 600
scene.render.resolution_percentage = 100
scene.view_settings.view_transform = "AgX"
scene.world = bpy.data.worlds.new("Overcast blue skylight")
scene.world.use_nodes = True
bg = scene.world.node_tree.nodes["Background"]
bg.inputs["Color"].default_value = (0.51, 0.66, 0.77, 1)
bg.inputs["Strength"].default_value = 0.62
bpy.ops.import_scene.gltf(filepath=str(args.glb.resolve()))
chunks = [o for o in bpy.data.objects if o.type == "MESH"]
assert len(chunks) == 360
sun = bpy.data.lights.new("Low warm sun", "SUN")
sun.energy, sun.angle = 2.2, math.radians(8)
sun.color = (1.0, 0.89, 0.75)
light = bpy.data.objects.new("Sun", sun)
scene.collection.objects.link(light)
light.rotation_euler = Vector((0.90, 0.52, -0.71)).to_track_quat("-Z", "Y").to_euler()
# Neutral sea stand-in to expose the actual shoreline. No plants, nursery props,
# engine lighting or other scenery; the A/B is strictly the shipping terrain art.
bpy.ops.mesh.primitive_plane_add(size=8000)
ocean = bpy.context.object
mat = bpy.data.materials.new("Sea swatch")
mat.use_nodes = True
p = mat.node_tree.nodes["Principled BSDF"]
p.inputs["Base Color"].default_value = (0.025, 0.13, 0.18, 1)
p.inputs["Roughness"].default_value = 0.57
ocean.data.materials.append(mat)
cam = bpy.data.cameras.new("Locked lens")
cam.clip_end = 9000
obj = bpy.data.objects.new("Locked camera", cam)
scene.collection.objects.link(obj)
scene.camera = obj


# Coordinates below are in the game's Y-up space; Blender is Z-up.
def native(p):
    return Vector((p[0], -p[2], p[1]))


shots = [
    ("wide", (705, 795, 975), (0, 27, -12), 1120, 36, 2),
    ("estuary", (190, 54, 440), (4, 20, 236), 0, 35, 0),
    ("headland", (-480, 51, -72), (-265, 30, -212), 0, 36, 0),
]
if args.creature_height or args.tidal_review:
    root = Path(__file__).resolve().parents[1]
    library = (
        args.prop_library or root / "crates/alife_game_app/assets/landscape/island"
    )
    raw = (
        args.surface or root / "crates/alife_world/assets/island-v1.bin"
    ).read_bytes()
    width, depth, ox, oz, spacing, _ = struct.unpack_from("<IIfffI", raw, 8)
    heights = struct.unpack_from(f"<{width*depth}f", raw, 32)

    def ground(x, z):
        u, v = (x - ox) / spacing, (z - oz) / spacing
        i, j = math.floor(u), math.floor(v)
        u, v = u - i, v - j
        a = j * width + i
        p, q, r, s = (
            heights[a],
            heights[a + 1],
            heights[a + width + 1],
            heights[a + width],
        )
        return (
            p + (q - p) * u + (r - q) * v if u >= v else p + (r - s) * u + (s - p) * v
        )

    x, z = 99.9, 256.9
    focus = (x, ground(x, z) + 0.88, z)
    aim = (98.3, ground(98.3, 253.1) + 0.045, 253.1)
    shots = [("creature-height", focus, aim, 0, 30, 0)]
    if args.tidal_review:
        x, z = 344.8, 114.8
        shots = [
            (
                "strand-eye",
                (x, ground(x, z) + 0.86, z),
                (347.05, ground(347.05, 110.86) + 0.16, 110.86),
                0,
                32,
                0,
            ),
            (
                "strand-reach",
                (334.4, ground(334.4, 139.3) + 4.2, 139.3),
                (348.2, 1.05, 104.8),
                0,
                43,
                0,
            ),
        ]
    props = json.loads((args.placements or library / "props.json").read_text())
    models = {}
    for prop in props:
        px, py, pz = prop["position"]
        kind = prop["kind"]
        if math.hypot(px - x, pz - z) > 65:
            continue
        if kind not in models:
            old = set(bpy.data.objects)
            bpy.ops.import_scene.gltf(filepath=str(library / f"{kind}.glb"))
            items = set(bpy.data.objects) - old
            models[kind] = next(o for o in items if o.name == kind + "_L0")
            for o in items:
                o.hide_render = True
        instance = models[kind].copy()
        scene.collection.objects.link(instance)
        instance.hide_render = False
        if kind in ("Grass", "Fern", "Flowers", "BerryBush", "CoastalShrub"):
            py = ground(px, pz)
        instance.location = native((px, py, pz))
        instance.rotation_mode = "XYZ"
        instance.rotation_euler = (0, 0, prop["yaw"])
        instance.scale = (prop["scale"],) * 3
if args.strandline:
    bpy.ops.import_scene.gltf(filepath=str(args.strandline.resolve()))
if args.wide_only:
    shots = shots[:1]
if args.handoff:
    shots = [
        (f"handoff-{lod}", (250, 245, 10), (33, 109, -198), 350, 36, lod)
        for lod in (0, 1, 2)
    ]
for name, pos, aim, ortho, lens, lod in shots:
    for ob in chunks:
        ob.hide_render = not ob.name.endswith(f"_L{lod}")
    obj.location = native(pos)
    obj.rotation_euler = (
        (native(aim) - obj.location).to_track_quat("-Z", "Y").to_euler()
    )
    cam.type = "ORTHO" if ortho else "PERSP"
    cam.ortho_scale, cam.lens = max(0.01, ortho), lens
    scene.render.filepath = str((args.out / f"{name}.png").resolve())
    bpy.ops.render.render(write_still=True)
