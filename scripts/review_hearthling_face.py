"""Matched modest-cost face, clay, blink and full-body renders of a checkpoint."""
import argparse
import sys
from pathlib import Path
import bpy
from mathutils import Vector

parser=argparse.ArgumentParser()
parser.add_argument('--output',required=True)
parser.add_argument('--quick',action='store_true')
args=parser.parse_args(sys.argv[sys.argv.index('--')+1:])
out=Path(args.output)
if not out.is_absolute(): out=Path(__file__).resolve().parents[1]/out
out.mkdir(parents=True,exist_ok=True)
scene=bpy.context.scene; rig=bpy.data.objects['Hearthling_Rig']; camera=scene.camera
rig.animation_data.action=next(a for a in bpy.data.actions if a.name.startswith('Hearthling_CuriousIdle'))
scene.render.engine='BLENDER_EEVEE'
scene.render.resolution_x=scene.render.resolution_y=640
scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG'
views=[('front',(0,-6,1.82),1,1.85,1.73),('three-quarter',(2.8,-6,2.2),1,1.85,1.73),
       ('side',(6,0,1.73),1,1.85,1.73),('half-blink',(0,-6,1.82),26,1.85,1.73),
       ('blink',(0,-6,1.82),28,1.85,1.73),('body',(2.1,-6.5,2.5),1,3.1,1.12)]
if args.quick: views=views[:2]
for name,pos,frame,scale,height in views:
    scene.frame_set(frame)
    camera.location=pos
    camera.rotation_euler=(Vector((0,0,height))-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.ortho_scale=scale
    scene.render.filepath=str(out/(name+'.png'))
    bpy.ops.render.render(write_still=True)
if not args.quick:
    clay=bpy.data.materials.new('Face review neutral clay'); clay.diffuse_color=(.45,.45,.45,1)
    scene.view_layers[0].material_override=clay
    rig.data.pose_position='REST'
    for name,pos in [('clay-front',(0,-6,1.7)),('clay-side',(6,0,1.7)),('clay-three-quarter',(2.8,-6,1.9))]:
        camera.location=pos
        camera.rotation_euler=(Vector((0,0,1.65))-camera.location).to_track_quat('-Z','Y').to_euler()
        camera.data.ortho_scale=1.85
        scene.render.filepath=str(out/(name+'.png'))
        bpy.ops.render.render(write_still=True)
print('FACE_REVIEW_COMPLETE',flush=True)
