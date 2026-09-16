"""Export the approved editable landscape props with Blender 5.1."""
import bpy
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/landscape'
bpy.ops.wm.open_mainfile(filepath=str(HERE / 'landscape.blend'))
for ob in list(bpy.context.scene.objects):
    if ob.type != 'MESH': continue
    bpy.ops.object.select_all(action='DESELECT')
    ob.select_set(True)
    bpy.context.view_layer.objects.active = ob
    bpy.ops.export_scene.gltf(filepath=str(HERE / (ob.name + '.glb')),
        export_format='GLB', use_selection=True, export_animations=False)
