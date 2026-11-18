"""Make the family album's porcelain species cameo from the approved game model.
blender --background --python scripts/render_hearthling_cameo.py
The cameo is deliberately unpainted: it never asserts an individual's coat color.
"""

from pathlib import Path

import bpy
from mathutils import Vector

root = Path(__file__).resolve().parents[1]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(
    filepath=str(
        root / "crates/alife_game_app/assets/creatures/hearthling/hearthling.glb"
    )
)
scene = bpy.context.scene
scene.render.engine = "CYCLES"
scene.cycles.samples = 16
scene.cycles.use_denoising = True
scene.render.resolution_x = 256
scene.render.resolution_y = 256
scene.render.resolution_percentage = 100
scene.render.film_transparent = True
scene.render.image_settings.file_format = "PNG"
scene.render.image_settings.color_mode = "RGBA"
scene.view_settings.view_transform = "AgX"


def enamel(name, rgb, roughness):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes.get("Principled BSDF")
    b.inputs["Base Color"].default_value = (*rgb, 1)
    b.inputs["Roughness"].default_value = roughness
    return m


pearl = enamel("Celadon porcelain", (0.58, 0.72, 0.61), 0.42)
ink = enamel("Deep green enamel", (0.065, 0.13, 0.13), 0.26)
inlay = enamel("Warm shell", (0.83, 0.64, 0.42), 0.42)
for obj in scene.objects:
    if obj.type != "MESH":
        continue
    for slot in obj.material_slots:
        label = (obj.name + " " + (slot.material.name if slot.material else "")).lower()
        if "eyes" in label:
            # Preserve the approved mesh's measured sclera/iris/pupil contrast.
            original = slot.material
            shader = next(
                (n for n in original.node_tree.nodes if n.type == "BSDF_PRINCIPLED"),
                None,
            )
            if shader:
                shader.inputs["Roughness"].default_value = 0.30
        elif "mouth" in label or "nose" in label or "brows" in label:
            slot.material = ink
        elif "ochre" in label:
            slot.material = inlay
        else:
            slot.material = pearl


def aim(obj, target):
    obj.rotation_euler = (
        (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()
    )


def softbox(name, xyz, power, color, size):
    b = bpy.data.lights.new(name, "AREA")
    b.energy = power
    b.color = color
    b.shape = "DISK"
    b.size = size
    obj = bpy.data.objects.new(name, b)
    scene.collection.objects.link(obj)
    obj.location = xyz
    aim(obj, (0, 0, 1.45))


softbox("Soft daylight", (-2.7, -3.2, 4.4), 260.0, (1.0, 0.89, 0.72), 3.0)
softbox("Celadon fill", (2.4, -2.4, 2.6), 110.0, (0.66, 0.82, 1.0), 2.8)
softbox("Shell rim", (0, 2.0, 3.0), 160.0, (1.0, 0.80, 0.60), 2.0)
world = bpy.data.worlds.new("Ambient")
scene.world = world
world.color = (0.22, 0.24, 0.27)
cam = bpy.data.cameras.new("Album portrait")
cam.type = "ORTHO"
cam.ortho_scale = 1.65
obj = bpy.data.objects.new("Album portrait", cam)
scene.collection.objects.link(obj)
obj.location = (1.1, -4.8, 2.10)
aim(obj, (0, 0, 1.52))
scene.camera = obj
scene.render.filepath = str(root / "assets/ui/kinship/cameo.png")
bpy.ops.render.render(write_still=True)
