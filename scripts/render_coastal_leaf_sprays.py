"""Portable matched CPU-only imported-GLB previews, including real authored placements.

blender -b -t 4 --factory-startup --python-exit-code 1 --python scripts/render_coastal_leaf_sprays.py
Append -- --only-close for first visual iteration, --patch for actual Island patch.
"""
import bpy, sys, json, math, argparse, struct, hashlib
from pathlib import Path
from mathutils import Vector
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'crates/alife_game_app/assets/landscape/island'
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--only-close',action='store_true');p.add_argument('--patch',action='store_true')
p.add_argument('--output',type=Path,default=ROOT/'target/artifacts/coastal-leaf-sprays/review')
a=p.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
assert bpy.app.version==(4,3,2), 'Use Blender 4.3.2'
OUT=a.output.resolve();OUT.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
s=bpy.context.scene;s.render.engine='CYCLES';s.cycles.device='CPU';s.cycles.samples=24;s.cycles.use_denoising=False
s.render.resolution_x=640;s.render.resolution_y=640;s.render.resolution_percentage=100
s.render.image_settings.file_format='PNG';s.view_settings.view_transform='AgX'
s.world=bpy.data.worlds.new('Review world');s.world.use_nodes=True
bg=s.world.node_tree.nodes.get('Background');bg.inputs['Color'].default_value=(.18,.24,.29,1);bg.inputs['Strength'].default_value=.65
sun=bpy.data.lights.new('Review sunlight','SUN');sun.energy=2.4;sun.angle=.12
o=bpy.data.objects.new('Review sunlight',sun);s.collection.objects.link(o);o.rotation_euler=(.45,-.5,-.5)
fill=bpy.data.lights.new('Review fill','AREA');fill.energy=400;fill.shape='DISK';fill.size=9
o=bpy.data.objects.new('Review fill',fill);s.collection.objects.link(o);o.location=(0,-5,7)
o.rotation_euler=(Vector((0,0,.5))-o.location).to_track_quat('-Z','Y').to_euler()
camera=bpy.data.cameras.new('Matched camera');camera.type='ORTHO';camera.ortho_scale=1.75
cam=bpy.data.objects.new('Matched camera',camera);s.collection.objects.link(cam);s.camera=cam
inputs=[HERE/'coastal-leaf-sprays/CoastalShrub.baseline.glb',HERE/'CoastalShrub.glb']
if a.patch:inputs += [HERE/'props.json',HERE/'terrain-chunks.glb',ROOT/'crates/alife_world/assets/island-v1.bin']+sorted(p for p in HERE.glob('*.glb') if p.name!='CoastalShrub.glb')
receipt={'source_files':{p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs},'renderer':'Cycles CPU','blender':bpy.app.version_string,'samples':24,'denoise':False,'view_transform':'AgX','live_game':False,'physical_gpu':False,'renders':[]}
def load(path):
    old=set(bpy.data.objects);bpy.ops.import_scene.gltf(filepath=str(path))
    return [o for o in bpy.data.objects if o not in old and o.type=='MESH']
def point_camera(eye,target,scale):
    cam.location=eye;cam.rotation_euler=(Vector(target)-cam.location).to_track_quat('-Z','Y').to_euler();camera.ortho_scale=scale
def render(filename,eye,target,scale,lod,variant):
    point_camera(eye,target,scale);s.render.filepath=str(OUT/filename);bpy.ops.render.render(write_still=True)
    receipt['renders'].append({'file':filename,'variant':variant,'eye':list(eye),'target':list(target),'ortho_scale':scale,'lod':lod,'resolution':[s.render.resolution_x,s.render.resolution_y]})
libraries={}
for variant,path in [('baseline',HERE/'coastal-leaf-sprays/CoastalShrub.baseline.glb'),('candidate',HERE/'CoastalShrub.glb')]:
    obs=load(path)
    libraries[variant]=sorted(obs,key=lambda o:o.name.split('_L')[-1][0])
    for o in obs:o.hide_render=True
if not a.patch:
    ground=bpy.data.materials.new('Review ground');ground.use_nodes=True
    shader=ground.node_tree.nodes.get('Principled BSDF');shader.inputs['Base Color'].default_value=(.055,.068,.075,1);shader.inputs['Roughness'].default_value=.88
    bpy.ops.mesh.primitive_plane_add(size=200,location=(0,0,-.025));bpy.context.object.data.materials.append(ground)
    views=[('close',(1.1,-2.1,1.7),[0])] if a.only_close else [('close',(1.1,-2.1,1.7),[0,1,2]),('reverse',(-1.1,2.1,1.5),[0]),('side',(2.5,0,.9),[0])]
    for view,eye,levels in views:
        for level in levels:
            for variant in ('baseline','candidate'):
                for v,obs in libraries.items():
                    for i,o in enumerate(obs):o.hide_render=v!=variant or i!=level
                render(f'{variant}-{view}-L{level}.png',eye,(0,0,.4),1.75,level,variant)
else:
    props=json.loads((HERE/'props.json').read_text());cover={'Grass','Flowers','Fern','CoastalShrub','BerryBush'}
    best=max((p for p in props if p['kind']=='CoastalShrub'),key=lambda p:sum(1 for q in props if q['kind']=='CoastalShrub' and abs(p['position'][0]-q['position'][0])<12 and abs(p['position'][2]-q['position'][2])<12))
    focus=Vector((best['position'][0],-best['position'][2],best['position'][1]))
    terrain=load(HERE/'terrain-chunks.glb')
    for o in terrain:
        cs=[o.matrix_world@Vector(c) for c in o.bound_box];cx=sum(c.x for c in cs)/8;cy=sum(c.y for c in cs)/8
        o.hide_render=not(o.name.endswith('_L0') and abs(cx-focus.x)<120 and abs(cy-focus.y)<120)
    raw=(ROOT/'crates/alife_world/assets/island-v1.bin').read_bytes()
    w,d,ox,oz,spacing,_=struct.unpack_from('<IIfffI',raw,8);heights=struct.unpack_from('<'+'f'*(w*d),raw,32)
    def ground_height(x,z):
        u=(x-ox)/spacing;v=(z-oz)/spacing;i=min(w-2,max(0,int(u)));j=min(d-2,max(0,int(v)));u-=i;v-=j
        h00=heights[j*w+i];h10=heights[j*w+i+1];h01=heights[(j+1)*w+i];h11=heights[(j+1)*w+i+1]
        return h00+(h10-h00)*u+(h11-h10)*v if u>=v else h00+(h11-h01)*u+(h01-h00)*v
    focus.z=ground_height(focus.x,-focus.y)
    libs={};instances={'baseline':[],'candidate':[]};records=[]
    for i,p in enumerate(props):
        x,y,z=p['position']
        if abs(x-focus.x)>35 or abs(-z-focus.y)>35:continue
        kind=p['kind']
        if kind!='CoastalShrub' and kind not in libs:
            obs=load(HERE/(kind+'.glb'))
            for o in obs:o.hide_render=True
            libs[kind]=next(o for o in obs if o.name.split('.')[0]==kind+'_L0')
        variants=('baseline','candidate') if kind=='CoastalShrub' else ('both',)
        for variant in variants:
            src=libraries[variant][0] if kind=='CoastalShrub' else libs[kind]
            o=src.copy();s.collection.objects.link(o);o.hide_render=False
            o.location=(x,-z,ground_height(x,z) if kind in cover else y);o.rotation_mode='XYZ';o.rotation_euler=(0,0,p['yaw']);o.scale=(p['scale'],)*3
            if kind=='CoastalShrub':instances[variant].append(o)
        records.append({'index':i,**p,'effective_z_up_height_m':ground_height(x,z) if kind in cover else y})
    # Move local fill with the patch; sunlight and world stay exactly matched.
    fill_ob=bpy.data.objects['Review fill'];fill_ob.location=focus+Vector((0,-5,7))
    fill_ob.rotation_euler=(focus+Vector((0,0,.5))-fill_ob.location).to_track_quat('-Z','Y').to_euler()
    s.render.resolution_x=1200;s.render.resolution_y=900
    for variant in ('baseline','candidate'):
        for v,obs in instances.items():
            for o in obs:o.hide_render=v!=variant
        for distance,scale in [('authored-patch',18),('authored-near',6)]:
            render(f'{variant}-{distance}.png',focus+Vector((10,-16,16)),focus+Vector((0,0,.4)),scale,0,variant)
    receipt.update({'authored_focus_z_up':list(focus),'placements':records,'grounding':'Actual L0 terrain chunks and authored XZ/yaw/uniform scale; cover near-triangle grounding matches static runtime code. No placement files modified.','placement_count_in_patch':len(records),'shrub_instances_in_patch':len(instances['baseline'])})
(OUT/('patch-render-receipt.json' if a.patch else 'close-render-receipt.json')).write_text(json.dumps(receipt,indent=2)+'\n')
print('RENDER_REVIEW',json.dumps(receipt),flush=True)
