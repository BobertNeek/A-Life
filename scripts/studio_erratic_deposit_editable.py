"""Modeler's native-scale desk for the separate A/B/C deposits.

Original and revised forms stay separate and reopen with their actual
metre-scale minerals. Presentation offsets live on parent empties only.
"""

from pathlib import Path
import sys
import bpy
from mathutils import Euler

raw = sys.argv[sys.argv.index("--") + 1 :]
previous, revised, out = map(Path, raw[:3])
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.unit_settings.system = "METRIC"
scene.unit_settings.scale_length = 1.0
active = None
for source, tag in (
    (previous, "Original height bands"),
    (revised, "Glacial rub faces"),
):
    master = bpy.data.collections.new(tag)
    scene.collection.children.link(master)
    for i, kind in enumerate(("BoulderA", "BoulderB", "BoulderC")):
        group = bpy.data.collections.new(tag + " | " + kind)
        master.children.link(group)
        offset = bpy.data.objects.new(kind + " · optional bench position", None)
        group.objects.link(offset)
        offset.location.x = (i - 1) * 6.5
        offset.empty_display_size = 0.30
        old = set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str((source / (kind + ".glb")).resolve()))
        for o in set(bpy.data.objects) - old:
            for collection in list(o.users_collection):
                collection.objects.unlink(o)
            group.objects.link(o)
            if not o.parent:
                o.parent = offset
            level = next((k for k in range(3) if f"_L{k}" in o.name), None)
            if level is not None:
                o.name = f"{tag} | {kind} | L{level}"
                o.hide_viewport = level != 0
                o.hide_render = level != 0
                if level == 0 and kind == "BoulderB" and source == revised:
                    active = o
    if source == previous:
        master.hide_viewport = True
        master.hide_render = True
bpy.ops.object.select_all(action="DESELECT")
if active:
    active.select_set(True)
    bpy.context.view_layer.objects.active = active
for area in bpy.context.screen.areas if bpy.context.screen else []:
    if area.type == "VIEW_3D":
        sp = area.spaces.active
        sp.region_3d.view_location = (0, 0, 1.05)
        sp.region_3d.view_distance = 15.8
        sp.region_3d.view_rotation = Euler((1.25, 0, -0.15)).to_quaternion()
        sp.shading.type = "MATERIAL"
scene["guide"] = (
    "Native metres. Three separate rock bodies in each A/B/C model. Original forms and all LODs are in the outliner. Gallery parents alone are offset. The appearance study applies no collision or terrain changes."
)
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(out.resolve()), compress=True)
