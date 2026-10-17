"""Matched actual runtime-GLB CPU previews. No repository writes."""
import sys,argparse
from pathlib import Path
import bpy
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];HERE=ROOT/'crates/alife_game_app/assets/landscape/island';OUT=ROOT/'target/artifacts/fern-fronds/review'
sys.path.insert(0,str(ROOT/'scripts'))
from island_art_common import setup_scene,camera,material,mesh,render
p=argparse.ArgumentParser();p.add_argument('--candidate',action='store_true');a=p.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False);setup_scene();s=bpy.context.scene;s.cycles.device='CPU';s.cycles.samples=24;s.cycles.use_denoising=False
path=HERE/'Fern.glb' if a.candidate else HERE/'fern-connected-fronds/Fern.baseline.glb'
bpy.ops.import_scene.gltf(filepath=str(path));obs={o.name.rsplit('.',1)[0]:o for o in bpy.data.objects if o.type=='MESH'}
mesh('Review ground',[(-5,-5,-.015),(5,-5,-.015),(5,5,-.015),(-5,5,-.015)],[(0,1,2,3)],material('Review ground',(.10,.15,.12)))
for view,eye in [('close',(1.3,-1.8,1.2)),('reverse',(-1.3,1.5,1.0))]:
 cam=camera('Matched fern '+view,eye,(0,0,.34),1.4)
 for level in range(3 if view=='close' else 1):
  for name,ob in obs.items():ob.hide_render=name!='Fern_L'+str(level)
  render(cam,OUT/(('candidate' if a.candidate else 'baseline')+'-'+view+'-L'+str(level)+'.png'),(640,640))
