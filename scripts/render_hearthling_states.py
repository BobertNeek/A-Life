"""Import the shipping GLB, check grounding, and render state/face previews."""
import bpy
from pathlib import Path
from mathutils import Vector
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'target/artifacts/graphics'
OUT.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(ROOT / 'crates/alife_game_app/assets/creatures/hearthling/hearthling.blend'))
# Retain the authoring studio, but validate the exported geometry and animation.
for ob in list(bpy.context.scene.objects):
    if ob.name.startswith('Hearthling'):
        bpy.data.objects.remove(ob, do_unlink=True)
for action in list(bpy.data.actions):
    bpy.data.actions.remove(action)
bpy.ops.import_scene.gltf(filepath=str(ROOT / 'crates/alife_game_app/assets/creatures/hearthling/hearthling.glb'))
rig = bpy.data.objects['Hearthling_Rig']; scene = bpy.context.scene
scene.render.engine = 'BLENDER_EEVEE'
scene.render.resolution_x = scene.render.resolution_y = 600
scene.render.resolution_percentage = 100
camera = scene.camera
camera.location = (3.2, -6.5, 2.75)
camera.rotation_euler = (Vector((0, 0, 1.3)) - camera.location).to_track_quat('-Z','Y').to_euler()
camera.data.ortho_scale = 3.5
for name in ['Hearthling_CuriousIdle', 'Hearthling_Walk', 'Hearthling_Sleep']:
    rig.animation_data.action = next(a for a in bpy.data.actions if a.name.startswith(name))
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
rig.animation_data.action = next(a for a in bpy.data.actions if a.name.startswith('Hearthling_CuriousIdle'))
scene.render.resolution_x = scene.render.resolution_y = 900
for name, position, frame in [('face-front', (0,-6,2.14), 1),
                              ('face-three-quarter', (2.8,-6,2.55), 1),
                              ('face-half-blink', (0,-6,2.14), 26),
                              ('face-blink', (0,-6,2.14), 28)]:
    scene.frame_set(frame)
    camera.location = position
    camera.rotation_euler = (Vector((0,0,2.04))-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.ortho_scale = 1.85
    if name in ['face-front','face-blink']:
        deps=bpy.context.evaluated_depsgraph_get()
        for side in ['L','R']:
            globe=bpy.data.objects['Hearthling_Eyeball_'+side].evaluated_get(deps)
            center=sum((globe.matrix_world @ Vector(p) for p in globe.bound_box),Vector())/8
            hit,_,_,_,ob,_=scene.ray_cast(deps,camera.location,(center-camera.location).normalized())
            expected='Hearthling_Head' if name=='face-blink' else 'Hearthling_Eyeball_'+side
            assert hit and ob.name==expected, (name,side,ob.name if hit else 'no hit')
        print(name,'eye visibility PASS',flush=True)
    scene.render.filepath = str(OUT / (name + '.png'))
    bpy.ops.render.render(write_still=True)
print('SHIPPING_GLB_RENDER_CHECK_COMPLETE', flush=True)
