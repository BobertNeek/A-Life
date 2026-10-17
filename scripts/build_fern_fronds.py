"""Isolated, reversible Fern frond-rib candidate. Blender 4.3.2, CPU only.
Normal export reopens the editable candidate source. --author is a guarded one-time
reconstruction of the exact shipping Fern, with folded leaf silhouettes retained
and triangle budget reallocated from redundant leaflet segmentation to frond ribs.
The normal export writes only Fern and its specification fields. Run the
approved registrar afterward to refresh the existing manifest digest.
"""
import argparse,hashlib,json,math,sys
from pathlib import Path
import bpy
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'crates/alife_game_app/assets/landscape/island'
BASE=HERE/'fern-connected-fronds/Fern.baseline.glb'
PIN='20d1ca87a71f8adc6ea9cd2b0ca8356cf4beec4a4ae145f2b8224b0b972dabb6'
sys.path.insert(0,str(ROOT/'scripts'))
from compact_glb_colors import compact_colors
from build_island_assets import foliage as plant
SOURCE=HERE/'fern-connected-fronds.blend'

def bounds(points):
 return [min(p[k] for p in points) for k in range(3)],[max(p[k] for p in points) for k in range(3)]

def author():
 assert hashlib.sha256(BASE.read_bytes()).hexdigest()==PIN
 bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
 bpy.ops.import_scene.gltf(filepath=str(BASE))
 originals=[bpy.data.objects['Fern_L'+str(l)] for l in range(3)]
 material=originals[0].data.materials[0]
 # Keep the imported shared opaque painted surface exactly; no new material.
 old_bounds=[bounds([v.co for v in ob.data.vertices]) for ob in originals]
 attr=originals[0].data.color_attributes.active_color
 lookup={}
 for loop,col in zip(originals[0].data.loops,attr.data):
  lookup[tuple(round(c,6) for c in originals[0].data.vertices[loop.vertex_index].co)]=tuple(col.color[:3])
 g=plant('Fern');lo,hi=bounds(g.v)
 def fit(p):return Vector(tuple((p[k]-lo[k])/(hi[k]-lo[k])*(old_bounds[0][1][k]-old_bounds[0][0][k])+old_bounds[0][0][k] for k in range(3)))
 def old_color(p):return lookup.get(tuple(round(c,6) for c in p),(.07,.26,.11))
 def root(j,k):
  t=k/8;d=Vector((math.cos(j*2.399),math.sin(j*2.399),0))
  return fit(d*t*.55+Vector((0,0,.12+.42*math.sin(t*2))))
 configs=[(list(range(7)),list(range(1,8)),True),(list(range(7)),[2,4,6,7],False),(list(range(7)),[4,7],False)]
 for level,(fronds,rungs,folded) in enumerate(configs):
  verts=[];faces=[];colors=[]
  def add(p,c):verts.append(tuple(p));colors.append(tuple(c));return len(verts)-1
  smooth_faces=[]
  for j in fronds:
   # A continuous tapered triangular-section rib follows the exact authored
   # leaflet attachment roots. Its first ring begins at the ground crown.
   points=[Vector((0,0,0))]+[root(j,k) for k in rungs]
   rings=[]
   for n,p in enumerate(points):
    tangent=(points[min(n+1,len(points)-1)]-points[max(0,n-1)]).normalized()
    u=tangent.cross(Vector((0,0,1))).normalized()
    if u.length<.1:u=Vector((1,0,0))
    w=tangent.cross(u).normalized();t=n/(len(points)-1)
    radius=(.012*(1-t)+.0035*t)*(1 if level==0 else 1.08)
    color=(.047+.055*t,.20+.13*t,.070+.04*t)
    rings.append([add(p+radius*(u*math.cos(a*math.tau/3)+w*math.sin(a*math.tau/3)),color) for a in range(3)])
   rib_start=len(faces)
   for lower,upper in zip(rings,rings[1:]):
    for a in range(3):
     b=(a+1)%3;faces.extend([(lower[a],lower[b],upper[b]),(lower[a],upper[b],upper[a])])
   faces.extend([tuple(reversed(rings[0])),tuple(rings[-1])])
   smooth_faces.extend(range(rib_start,len(faces)))
   # Every folded leaflet shares its base vertex with its rib ring. This is
   # actual native topology connectivity, not just floating intersecting pieces.
   for rung_index,k in enumerate(rungs):
    for side in range(2):
     start=(j*14+(k-1)*2+side)*9
     old=[fit(Vector(v)) for v in g.v[start:start+9]]
     l,r,t=[add(old[n],old_color(old[n])) for n in (3,5,7)]
     c=add(old[4],old_color(old[4])) if folded else None
     b=rings[rung_index+1][side]
     if folded:
      # One flat polygon per wing. Export triangulation uses 4 triangles;
      # shared wing normals plus smooth rib normals bound exported vertices.
      faces.extend([(b,l,t,c),(b,c,t,r)])
     else:faces.extend([(b,l,t),(b,t,r)])
  # Exact historical per-LOD AABBs, including their tiny overview lower bound.
  newlo,newhi=bounds(verts);targetlo,targethi=old_bounds[level]
  verts=[tuple(targetlo[k]+(p[k]-newlo[k])/(newhi[k]-newlo[k])*(targethi[k]-targetlo[k]) for k in range(3)) for p in verts]
  data=bpy.data.meshes.new('Fern_L'+str(level)+' candidate');data.from_pydata(verts,[],faces);data.update()
  for index in smooth_faces:data.polygons[index].use_smooth=True
  data.materials.append(material)
  paint=data.color_attributes.new(name='ArtColor',type='BYTE_COLOR',domain='POINT')
  for col,c in zip(paint.data,colors):col.color=(*c,1)
  data.color_attributes.active_color=paint
  ob=originals[level];old=ob.data;ob.data=data;bpy.data.meshes.remove(old);data.name='Fern_L'+str(level)
  ob['art_owner']='fern-connected-fronds.blend';ob['art_pass']='2026-10-16 connected frond ribs, bounded folded leaflets'
  ob['baseline_sha256']=PIN;ob['baseline_bounds_xyz']=[*targetlo,*targethi]
  ob['origin_contract']='ground-centred metres; Z-up native / Y-up GLB'
  ob.hide_set(level!=0);ob.hide_render=level!=0
 bpy.context.preferences.filepaths.save_version=0
 bpy.context.scene.unit_settings.system='METRIC';bpy.context.scene.unit_settings.scale_length=1
 bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE))

def export():
 bpy.ops.object.select_all(action='DESELECT');objs=[]
 for level in range(3):
  ob=bpy.data.objects['Fern_L'+str(level)];assert tuple(ob.location)==(0.,0.,0.) and tuple(ob.scale)==(1.,1.,1.) and tuple(ob.rotation_euler)==(0.,0.,0.)
  ob.hide_set(False);ob.hide_render=False;ob.select_set(True);objs.append(ob)
 bpy.ops.export_scene.gltf(filepath=str(HERE/'Fern.glb'),export_format='GLB',use_selection=True,export_animations=False,export_cameras=False,export_lights=False,export_vertex_color='ACTIVE')
 compact_colors(HERE/'Fern.glb')
 spec=HERE/'asset-specification.json';metadata=json.loads(spec.read_text());metadata['assets']['Fern'].update(near_triangles=700,far_triangles=294,overview_triangles=154,editable_override='fern-connected-fronds.blend',export_recipe='scripts/build_fern_fronds.py',override_blender='4.3.2');spec.write_text(json.dumps(metadata,indent=2)+'\n')
 print('FERN_FRONDS_EXPORT',json.dumps({'triangles':[sum(len(p.vertices)-2 for p in ob.data.polygons) for ob in objs],'native_vertices':[len(ob.data.vertices) for ob in objs],'source':str(SOURCE)}))

if __name__=='__main__':
 assert bpy.app.version==(4,3,2)
 p=argparse.ArgumentParser();p.add_argument('--author',action='store_true');a=p.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
 if a.author:author()
 else:bpy.ops.wm.open_mainfile(filepath=str(SOURCE))
 export()
