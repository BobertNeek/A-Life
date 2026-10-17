import sys,json,math,argparse,struct
from pathlib import Path
import bpy
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from island_art_common import setup_scene,camera,material,mesh,render
HERE=ROOT/'crates/alife_game_app/assets/landscape/island'
OUT=ROOT/'target/artifacts/fern-fronds/review'
p=argparse.ArgumentParser();p.add_argument('--patch',action='store_true');p.add_argument('--close',action='store_true');p.add_argument('--candidate',action='store_true');a=p.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
def load(path):
 old=set(bpy.data.objects);bpy.ops.import_scene.gltf(filepath=str(path));return [o for o in bpy.data.objects if o not in old and o.type=='MESH']
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
setup_scene();s=bpy.context.scene;s.cycles.samples=16;s.cycles.device='CPU';s.cycles.use_denoising=False
if not a.patch:
 ground=mesh('Review ground',[(-20,-20,-.015),(20,-20,-.015),(20,20,-.015),(-20,20,-.015)],[(0,1,2,3)],material('Review ground',(.10,.15,.12)))
 for kind in ['Grass','Flowers','Fern','BerryBush','CoastalShrub','Scree']:
  obs=load((HERE/'Fern.glb' if a.candidate else HERE/'fern-connected-fronds/Fern.baseline.glb') if kind=='Fern' else HERE/(kind+'.glb'));chosen=next(o for o in obs if o.name.rsplit('.',1)[0]==kind+'_L0')
  for o in obs:o.hide_render=o!=chosen
  dims=chosen.dimensions;w=max(dims.x,dims.y,dims.z)*1.5
  cam=camera(kind+' camera',(w*.8,-w,w*.65),(0,0,dims.z*.4),w)
  render(cam,OUT/(kind+'-L0.png'),(480,480))
  for o in obs:bpy.data.objects.remove(o,do_unlink=True)
else:
 props=json.loads((HERE/'props.json').read_text());cover={'Grass','Flowers','Fern','CoastalShrub','BerryBush'}
 best=max((p for p in props if p['kind']=='Fern'),key=lambda p:sum(1 for q in props if q['kind'] in cover and abs(p['position'][0]-q['position'][0])<13 and abs(p['position'][2]-q['position'][2])<13))
 print('BEST',best,flush=True)
 focus=Vector((best['position'][0],-best['position'][2],best['position'][1]))
 # 23-28m authored patch. Keep whole exact chunks, deleting only fully distant terrain.
 terrain=load(HERE/'terrain-chunks.glb')
 for o in terrain:
  cs=[o.matrix_world@Vector(c) for c in o.bound_box];cx=sum(c.x for c in cs)/8;cy=sum(c.y for c in cs)/8
  o.hide_render=not(o.name.endswith('_L0') and abs(cx-focus.x)<120 and abs(cy-focus.y)<120)
 raw=(ROOT/'crates/alife_world/assets/island-v1.bin').read_bytes()
 w,d,ox,oz,spacing,_=struct.unpack_from('<IIfffI',raw,8);heights=struct.unpack_from('<'+'f'*(w*d),raw,32)
 def ground_height(x,z):
  u=(x-ox)/spacing;v=(z-oz)/spacing;i=min(w-2,max(0,int(u)));j=min(d-2,max(0,int(v)));u-=i;v-=j
  h00=heights[j*w+i];h10=heights[j*w+i+1];h01=heights[(j+1)*w+i];h11=heights[(j+1)*w+i+1]
  # Mesh triangles (00,01,11), (00,11,10), shared diagonal u=v.
  return h00+(h10-h00)*u+(h11-h10)*v if u>=v else h00+(h11-h01)*u+(h01-h00)*v
 libraries={};records=[]
 for i,p in enumerate(props):
  x,y,z=p['position']
  if abs(x-focus.x)>42 or abs(-z-focus.y)>42:continue
  kind=p['kind']
  if kind not in libraries:
   obs=load((HERE/'Fern.glb' if a.candidate else HERE/'fern-connected-fronds/Fern.baseline.glb') if kind=='Fern' else HERE/(kind+'.glb'))
   for o in obs:o.hide_render=True
   libraries[kind]=next(o for o in obs if o.name.rsplit('.',1)[0]==kind+'_L0')
  o=libraries[kind].copy();s.collection.objects.link(o);o.hide_render=False;o.location=(x,-z,ground_height(x,z) if kind in cover else y);o.rotation_euler=(0,0,p['yaw']);o.scale=(p['scale'],)*3
  records.append({'index':i,**p})
 cam=camera('Authored ground cover patch',focus+Vector((10,-16,16)),focus+Vector((0,0,.4)),24 if not a.close else 12)
 render(cam,OUT/('candidate' if a.candidate else 'baseline')/('authored-patch'+('-close' if a.close else '')+'.png'),(1200,900))
 (OUT/(('candidate' if a.candidate else 'baseline')+'-patch-receipt.json')).write_text(json.dumps({'focus':list(focus),'placements':records,'render':'Actual L0 GLBs and terrain L0 chunks, authored XZ/yaw/scale, current runtime near-triangle plant grounding. Candidate changes only Fern geometry. Offline CPU Cycles, not an engine capture.'},indent=2))
