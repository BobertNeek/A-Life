"""Blender 5.1: export the approved rig with grounded idle, walk and sleep clips.
Run: blender --background --python scripts/export_hearthling.py
"""
import bpy
import math
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/creatures/hearthling'
bpy.ops.wm.open_mainfile(filepath=str(HERE / 'hearthling-source.blend'))
rig = bpy.data.objects['Hearthling_Rig']
scene = bpy.context.scene
p = rig.pose.bones
scene.frame_set(1)
base = {b.name: (b.location.copy(), b.rotation_euler.copy(), b.scale.copy()) for b in p}
idle = rig.animation_data.action
idle.name = 'Hearthling_CuriousIdle'
idle.use_fake_user = True
for track in list(rig.animation_data.nla_tracks):
    rig.animation_data.nla_tracks.remove(track)
models = [o for o in scene.objects if o.type == 'MESH' and o.name.startswith('Hearthling')]

def ground(frame):
    scene.frame_set(frame)
    bpy.context.view_layer.update()
    deps = bpy.context.evaluated_depsgraph_get()
    low = min((ob.evaluated_get(deps).matrix_world @ v.co).z
              for ob in models for v in ob.evaluated_get(deps).data.vertices)
    p['root'].location.y -= low
    p['root'].keyframe_insert('location', frame=frame)

# Root local Y follows Blender world Z. Keep the approved foot contact through the loop.
for f in range(1, 74):
    ground(f)

for kind in ['Walk', 'Sleep']:
    rig.animation_data.action = None
    for frame in range(1, 74):
        scene.frame_set(frame)
        for b in p:
            loc, rot, scale = base[b.name]
            b.location = loc.copy(); b.rotation_euler = rot.copy(); b.scale = scale.copy()
        phase = 2 * math.pi * (frame - 1) / (24 if kind == 'Walk' else 72)
        if kind == 'Walk':
            p['pelvis'].location.y += .028 * (1 - math.cos(2 * phase))
            p['head'].rotation_euler.y *= .3
            for side, sign in [('L', 1), ('R', -1)]:
                wave = math.sin(phase) * sign
                # Foot controls use world-aligned local axes on this rig.
                ctrl = p['foot_control.' + side]
                local = ctrl.bone.matrix_local.to_3x3().inverted() @ Vector((0, .24 * wave, .10 * max(0, wave)))
                ctrl.location += local
                p['upper_arm.' + side].rotation_euler.x += .22 * wave
                p['forearm.' + side].rotation_euler.x += .10 * wave
            p['spine'].rotation_euler.z += .035 * math.sin(phase)
        else:
            # A tucked, seated sleep keeps feet and rump supporting the body;
            # broad cartoon ears must never become the creature's ground contact.
            p['pelvis'].location.y -= .32
            p['chest'].rotation_euler.x += .18
            p['head'].rotation_euler.x += .35
            p['head'].rotation_euler.y = -.04
            p['spine'].scale.y = 1 + .01 * math.sin(phase)
            for side in ['L', 'R']:
                p['eye.' + side].scale.y = .025
                p['forearm.' + side].rotation_euler.x += .65
                p['ear.' + side].rotation_euler.z *= .4
                p['ear.' + side].scale.y = .85
                ctrl = p['foot_control.' + side]
                ctrl.location += ctrl.bone.matrix_local.to_3x3().inverted() @ Vector((0, -.18, 0))
            for i in range(5):
                p[f'tail.{i:02d}'].rotation_euler.z += .18
        for b in p:
            b.keyframe_insert('location', frame=frame, group=b.name)
            b.keyframe_insert('rotation_euler', frame=frame, group=b.name)
            b.keyframe_insert('scale', frame=frame, group=b.name)
        ground(frame)
    rig.animation_data.action.name = 'Hearthling_' + kind
    rig.animation_data.action.use_fake_user = True

# Explicit NLA tracks give stable, named glTF clips across Blender versions.
rig.animation_data.action = None
for name in ['Hearthling_CuriousIdle', 'Hearthling_Walk', 'Hearthling_Sleep']:
    track = rig.animation_data.nla_tracks.new(); track.name = name
    strip = track.strips.new(name, 1, bpy.data.actions[name]); strip.name = name
    track.mute = True
rig.animation_data.action = idle
scene.frame_start = 1; scene.frame_end = 73; scene.frame_set(1)
bpy.ops.object.select_all(action='DESELECT')
for ob in models + [rig]: ob.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(HERE / 'hearthling.glb'), export_format='GLB',
    use_selection=True, export_animations=True, export_animation_mode='ACTIONS',
    export_force_sampling=True, export_skins=True, export_cameras=False, export_lights=False)
bpy.ops.wm.save_as_mainfile(filepath=str(HERE / 'hearthling.blend'))
print('HEARTHLING_GAME_EXPORT_COMPLETE', flush=True)
