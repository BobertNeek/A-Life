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
camera.location = (3.2, -6.5, 2.5)
camera.rotation_euler = (Vector((0, 0, 1.18)) - camera.location).to_track_quat('-Z','Y').to_euler()
camera.data.ortho_scale = 3.15
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
for name, position, frame in [('face-front', (0,-6,1.82), 1),
                              ('face-three-quarter', (2.8,-6,2.2), 1),
                              ('face-half-blink', (0,-6,1.82), 26),
                              ('face-blink', (0,-6,1.82), 28)]:
    scene.frame_set(frame)
    camera.location = position
    camera.rotation_euler = (Vector((0,0,1.73))-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.ortho_scale = 1.85
    scene.render.filepath = str(OUT / (name + '.png'))
    bpy.ops.render.render(write_still=True)
    if name in ['face-front','face-blink']:
        deps=bpy.context.evaluated_depsgraph_get()
        for side in ['L','R']:
            merged=bpy.data.objects['Hearthling_Runtime'].evaluated_get(deps)
            eye_indices={i for p in merged.data.polygons
                if 'spherical eyes' in merged.data.materials[p.material_index].name for i in p.vertices}
            points=[merged.matrix_world@merged.data.vertices[i].co for i in eye_indices]
            points=[p for p in points if (p.x<0)==(side=='L')]
            center=Vector([(min(p[k] for p in points)+max(p[k] for p in points))/2 for k in range(3)])
            hit,_,_,face,ob,_=scene.ray_cast(deps,camera.location,(center-camera.location).normalized())
            def is_eye(ob,face):
                return 'spherical eyes' in ob.data.materials[ob.data.polygons[face].material_index].name
            assert hit and is_eye(ob,face)==(name=='face-front'), (name,side,'incorrect eye occlusion')
            if name=='face-blink':
                for dx in [-.085,0,.085]:
                    for dz in [-.040,0,.040]:
                        point=center+Vector((dx,0,dz))
                        hit,_,_,face,ob,_=scene.ray_cast(deps,camera.location,(point-camera.location).normalized())
                        assert hit and not is_eye(ob,face), (name,side,dx,dz,'exposed globe')
        print(name,'eye visibility PASS',flush=True)
print('SHIPPING_GLB_RENDER_CHECK_COMPLETE', flush=True)
