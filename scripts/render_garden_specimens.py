"""Photograph the four shipping lantern-pear meshes with a single lens and scale.
Blender --background --python scripts/render_garden_specimens.py
"""

from pathlib import Path

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets/ui/garden_survey"
OUTPUT.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.render.engine = "CYCLES"
scene.cycles.samples = 24
scene.cycles.use_denoising = True
scene.render.resolution_x = scene.render.resolution_y = 192
scene.render.resolution_percentage = 100
scene.render.film_transparent = True
scene.render.image_settings.color_mode = "RGBA"
scene.view_settings.view_transform = "AgX"
scene.world = bpy.data.worlds.new("Warm room")
scene.world.color = (0.38, 0.38, 0.38)


def aim(obj, target):
    obj.rotation_euler = (
        (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()
    )


for pos, power, size in [((-2, -3, 4), 390, 3), ((3, 1, 2), 180, 2.0)]:
    bpy.ops.object.light_add(type="AREA", location=pos)
    light = bpy.context.object
    light.data.energy = power
    light.data.shape = "DISK"
    light.data.size = size
    aim(light, (0, 0, 0.4))
bpy.ops.object.camera_add(location=(1.7, -2.5, 1.75))
scene.camera = bpy.context.object
aim(scene.camera, (0, 0, 0.45))
scene.camera.data.type = "ORTHO"
scene.camera.data.ortho_scale = 1.58
for stage in ("seed", "bud", "bloom", "ripe"):
    path = ROOT / f"crates/alife_game_app/assets/care/orchard/{stage}.glb"
    bpy.ops.import_scene.gltf(filepath=str(path))
    objects = list(bpy.context.selected_objects)
    scene.render.filepath = str(OUTPUT / f"{stage}.png")
    bpy.ops.render.render(write_still=True)
    for obj in objects:
        bpy.data.objects.remove(obj, do_unlink=True)
