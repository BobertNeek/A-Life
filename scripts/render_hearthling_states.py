"""Blender geometry/pose checks and small previews of the shipping GLB."""
import bpy, json
from pathlib import Path
from mathutils import Vector
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'target/artifacts/graphics'
bpy.ops.wm.open_mainfile(filepath=str(ROOT / 'crates/alife_game_app/assets/creatures/hearthling/hearthling.blend'))
rig = bpy.data.objects['Hearthling_Rig']; scene = bpy.context.scene
scene.render.engine = 'BLENDER_EEVEE'
scene.render.resolution_x = scene.render.resolution_y = 600
scene.render.resolution_percentage = 100
camera = scene.camera
camera.location = (3.2, -6.5, 2.75)
camera.rotation_euler = (Vector((0, 0, .9)) - camera.location).to_track_quat('-Z','Y').to_euler()
camera.data.ortho_scale = 4.0
for name in ['Hearthling_CuriousIdle', 'Hearthling_Walk', 'Hearthling_Sleep']:
    rig.animation_data.action = bpy.data.actions[name]
    scene.frame_set(7 if 'Walk' in name else 1)
    bpy.context.view_layer.update()
    deps = bpy.context.evaluated_depsgraph_get()
    pts = [o.evaluated_get(deps).matrix_world @ v.co for o in scene.objects
        if o.type == 'MESH' and o.name.startswith('Hearthling')
        for v in o.evaluated_get(deps).data.vertices]
    low = min(p.z for p in pts)
    assert abs(low) < .03, (name, low)
    print(name, 'ground contact', low, flush=True)
    scene.render.filepath = str(OUT / (name + '.png'))
    bpy.ops.render.render(write_still=True)
