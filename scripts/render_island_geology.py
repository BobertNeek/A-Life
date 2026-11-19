"""Lock the review camera and light for terrain GLBs (Blender, CPU; not a gameplay capture)."""

import argparse
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

cli = argparse.ArgumentParser(description=__doc__)
cli.add_argument("glb", type=Path)
cli.add_argument("out", type=Path)
cli.add_argument(
    "--handoff",
    action="store_true",
    help="hold the camera at the river head and vary only surface LOD",
)
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
