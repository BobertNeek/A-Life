"""A two-specimen botanical workbench portrait; not an island/game camera."""

import bpy, sys
from mathutils import Vector
from pathlib import Path

before, after, out = map(Path, sys.argv[sys.argv.index("--") + 1 :])
bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
for path, x in [(before, -0.67), (after, 0.67)]:
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path.resolve()))
    for ob in set(bpy.data.objects) - old:
        ob.hide_render = not ob.name.startswith("Fern_L0")
        ob.location.x = x
        ob.rotation_euler.z = 0.38
bpy.ops.mesh.primitive_plane_add(size=200)
m = bpy.data.materials.new("Workbench clay")
m.diffuse_color = (0.075, 0.084, 0.058, 1)
bpy.context.object.data.materials.append(m)


def aim(obj, at):
    obj.rotation_euler = (Vector(at) - obj.location).to_track_quat("-Z", "Y").to_euler()


bpy.ops.object.camera_add(location=(0, -3.2, 1.27))
cam = bpy.context.object
aim(cam, (0, 0, 0.39))
cam.data.type = "ORTHO"
cam.data.ortho_scale = 2.64
sc.camera = cam
for pos, e, col, sz in [
    ((-1.4, -1.7, 2.6), 200, (1, 0.88, 0.71), 2.4),
    ((1, 0.4, 2.1), 125, (0.68, 0.83, 1), 2.2),
]:
    bpy.ops.object.light_add(type="AREA", location=pos)
    lamp = bpy.context.object
    lamp.data.energy = e
    lamp.data.shape = "DISK"
    lamp.data.size = sz
    lamp.data.color = col
    aim(lamp, (0, 0, 0.27))
sc.world = bpy.data.worlds.new("Atelier ambient")
sc.world.color = (0.28, 0.30, 0.32)
sc.render.engine = "CYCLES"
sc.cycles.device = "CPU"
sc.cycles.samples = 20
sc.cycles.use_denoising = False
sc.render.resolution_x = 920
sc.render.resolution_y = 405
sc.render.resolution_percentage = 100
sc.render.filepath = str(out.resolve())
bpy.ops.render.render(write_still=True)
