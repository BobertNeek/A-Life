"""Leaf, grouped sprig and distant bough: the three same-scale editable shapes.
This is a studio artist sheet, not a native screenshot or performance run.
"""

import sys
from pathlib import Path
from mathutils import Vector
import bpy

args = sys.argv[sys.argv.index("--") + 1 :]
source, output = map(Path, args)
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(source.resolve()))
scene = bpy.context.scene
for i in range(3):
    t = bpy.data.objects["Sapling_L" + str(i)]
    t.location = (3.75 * (i - 1), 0, 0)
    t.rotation_euler.z = 0.28
bpy.ops.mesh.primitive_plane_add(size=200)
mat = bpy.data.materials.new("Slate clay")
mat.diffuse_color = (0.11, 0.13, 0.087, 1)
bpy.context.object.data.materials.append(mat)


def aim(o, p):
    o.rotation_euler = (Vector(p) - o.location).to_track_quat("-Z", "Y").to_euler()


bpy.ops.object.camera_add(location=(0, -13.2, 4.8))
cam = bpy.context.object
aim(cam, (0, 0, 1.87))
cam.data.type = "ORTHO"
cam.data.ortho_scale = 12.3
scene.camera = cam
for p, power, c, size in [
    ((-5, -8, 11), 2280, (1, 0.88, 0.73), 6),
    ((4, 2, 8), 1360, (0.67, 0.82, 1), 7),
]:
    bpy.ops.object.light_add(type="AREA", location=p)
    l = bpy.context.object
    l.data.energy = power
    l.data.color = c
    l.data.shape = "DISK"
    l.data.size = size
    aim(l, (0, 0, 1.8))
scene.world = bpy.data.worlds.new("Coastal ambient")
scene.world.color = (0.26, 0.34, 0.37)
scene.render.engine = "CYCLES"
scene.cycles.device = "CPU"
scene.cycles.samples = 16
scene.cycles.use_denoising = False
scene.render.resolution_x = 1040
scene.render.resolution_y = 450
scene.render.resolution_percentage = 100
scene.render.filepath = str(output.resolve())
bpy.ops.render.render(write_still=True)
