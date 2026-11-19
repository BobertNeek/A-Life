"""Openable textured three-LOD artist mesh; never a game object or collider."""

from pathlib import Path
import sys
import bpy

source, target = map(Path, sys.argv[sys.argv.index("--") + 1 :])
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(source.resolve()))
for level in range(3):
    stone = bpy.data.objects[f"Cliff_L{level}"]
    stone["artist_subject"] = (
        "Faulted coastal scarp at the sixteen original V1 Cliff placements"
    )
    stone["geometry_authority"] = (
        "Display mesh only. Original static world obstacle and A-Life terrain retained."
    )
    stone.hide_set(level != 0)
    stone.hide_render = level != 0
bpy.ops.file.pack_all()
bpy.ops.object.select_all(action="DESELECT")
bpy.data.objects["Cliff_L0"].select_set(True)
bpy.context.view_layer.objects.active = bpy.data.objects["Cliff_L0"]
bpy.ops.wm.save_as_mainfile(filepath=str(target.resolve()), compress=True)
