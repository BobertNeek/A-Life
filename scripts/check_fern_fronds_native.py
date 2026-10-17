"""Source topology/connectivity check for isolated Fern candidate."""
import json,sys,math
from pathlib import Path
import bpy
ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'crates/alife_game_app/assets/landscape/island'
OUT=ROOT/'target/artifacts/fern-fronds'
OUT.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(HERE/'fern-connected-fronds.blend'))
records=[]
for level,expected in enumerate((7,7,7)):
 ob=bpy.data.objects['Fern_L'+str(level)];m=ob.data
 assert ob.data.name==ob.name
 assert tuple(ob.location)==(0,0,0) and tuple(ob.rotation_euler)==(0,0,0) and tuple(ob.scale)==(1,1,1)
 assert len(m.materials)==1 and m.materials[0].name=='Island shared painted surface'
 assert m.color_attributes.active_color.name=='ArtColor'
 graph=[set() for v in m.vertices]
 for e in m.edges:a,b=e.vertices;graph[a].add(b);graph[b].add(a)
 assert all(graph),'No unused source vertices'
 left=set(range(len(m.vertices)));components=[]
 while left:
  start=min(left);left.remove(start);pending=[start];seen=[]
  while pending:
   i=pending.pop();seen.append(i)
   for j in graph[i]:
    if j in left:left.remove(j);pending.append(j)
  components.append(seen)
 assert len(components)==expected,(level,len(components))
 # Every component is one entire rooted rib + all its paired leaflets, not
 # separately floating leaflet islands. Ring base is within 2cm of crown.
 assert all(min(m.vertices[i].co.z for i in c)<.02 for c in components)
 assert all(any(m.vertices[i].co.z>.50 for i in c) for c in components)
 assert all(p.area>1e-10 for p in m.polygons),'Degenerate native face'
 tris=sum(len(p.vertices)-2 for p in m.polygons)
 records.append({'lod':level,'triangles':tris,'native_vertices':len(m.vertices),'connected_rooted_fronds':len(components),'unused_vertices':0,'zero_area_polygons':0,'identity_transform':True,'single_shared_material':True,'paint':'ArtColor'})
result={'result':'PASS','blender':bpy.app.version_string,'source':'fern-connected-fronds.blend','lods':records,'limit':'Connectivity is established in native topology. Runtime glTF may split shading vertices at flat leaflet/rib normal boundaries.'}
(OUT/'native-topology-check.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
