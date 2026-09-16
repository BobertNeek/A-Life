"""Export gameplay heights, obstacle proxies and render LODs from one saved sculpt."""
import bpy, json, math, struct
from pathlib import Path
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'crates/alife_game_app/assets/landscape/highlands'
WORLD=ROOT/'crates/alife_world/assets'
WORLD.mkdir(exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(HERE/'mountain-valley.blend'))
terrain=bpy.data.objects['Highlands_continuous_terrain']
nx,ny=320,420
assert len(terrain.data.vertices)==(nx+1)*(ny+1)
vertices=terrain.data.vertices
colors=terrain.data.color_attributes['LandscapeColor']
def source(i,j):return (ny-j)*(nx+1)+i
heights=[vertices[source(i,j)].co.z for j in range(ny+1) for i in range(nx+1)]
obstacles=[]; props=[]
for ob in bpy.data.collections['Highlands_Export'].objects:
    if ob==terrain or ob.name=='Highlands_river':continue
    key=next((n for n in ['Oak_variant_1','Oak_variant_2','Oak_variant_3',
        'Boulder_variant_1','Boulder_variant_2','Boulder_variant_3','Berry_shrub',
        'Grass_clump','Cliff_outcrop','Fern_patch','Wildflower_patch'] if ob.name.startswith('Highlands_'+n)),None)
    if not key:raise RuntimeError(ob.name)
    props.append({'kind':key,'position':[ob.location.x,ob.location.z,-ob.location.y],
                  'scale':ob.scale.x,'yaw':ob.rotation_euler.z})
    if key.startswith(('Boulder','Cliff','Oak')):
        bounds=[ob.matrix_world@Vector(c) for c in ob.bound_box]
        x0,x1=min(p.x for p in bounds),max(p.x for p in bounds)
        z0,z1=min(-p.y for p in bounds),max(-p.y for p in bounds)
        if key.startswith('Oak'):
            r=.58*ob.scale.x;x0=ob.location.x-r;x1=ob.location.x+r
            z0=-ob.location.y-r;z1=-ob.location.y+r
        obstacles.append((x0,z0,x1,z1,min(p.z for p in bounds),max(p.z for p in bounds)))
payload=bytearray(b'HLAND001')
payload.extend(struct.pack('<IIfffI',nx+1,ny+1,-400,-700,2.5,len(obstacles)))
payload.extend(struct.pack('<'+'f'*len(heights),*heights))
for row in obstacles:payload.extend(struct.pack('<6f',*row))
(WORLD/'highlands-v1.bin').write_bytes(payload)
(HERE/'props.json').write_text(json.dumps(props,separators=(',',':')))

chunks=[]
for j in range(0,ny,32):
    for i in range(0,nx,32):
        for lod,step in enumerate([1,2,4]):
            xs=list(range(i,min(i+32,nx)+1,step)); zs=list(range(j,min(j+32,ny)+1,step))
            vs=[];cs=[];ns=[];uvs=[];faces=[]
            for y in zs:
                for x in xs:
                    k=source(x,y);v=vertices[k]
                    vs.append(tuple(v.co));cs.append(tuple(colors.data[k].color));ns.append(tuple(v.normal))
                    uvs.append((v.co.x/6,v.co.y/6))
            for y in range(len(zs)-1):
                for x in range(len(xs)-1):
                    a=y*len(xs)+x;b=a+1;d=a+len(xs);c=d+1
                    faces.extend([(a,c,b),(a,d,c)])
            # Vertical skirts cover T-junctions where adjacent chunks select different LODs.
            w,h=len(xs),len(zs)
            edge=list(range(w))+[y*w+w-1 for y in range(1,h)]+list(range((h-1)*w+w-2,(h-1)*w-1,-1))+[y*w for y in range(h-2,0,-1)]
            lower=[]
            for index in edge:
                lower.append(len(vs));v=vs[index]
                vs.append((v[0],v[1],v[2]-20));cs.append(cs[index]);ns.append(ns[index]);uvs.append(uvs[index])
            for k,a in enumerate(edge):
                n=(k+1)%len(edge);b=edge[n];c=lower[n];d=lower[k]
                faces.extend([(a,b,c),(a,c,d)])
            name=f'Highlands_{i//32}_{j//32}_L{lod}'
            data=bpy.data.meshes.new(name);data.from_pydata(vs,[],faces);data.update()
            data.materials.append(terrain.data.materials[0])
            attr=data.color_attributes.new(name='LandscapeColor',type='BYTE_COLOR',domain='POINT')
            for row,color in zip(attr.data,cs):row.color=color
            uv=data.uv_layers.new(name='GroundUV')
            for loop in data.loops:uv.data[loop.index].uv=uvs[loop.vertex_index]
            for p in data.polygons:p.use_smooth=True
            data.normals_split_custom_set_from_vertices(ns)
            ob=bpy.data.objects.new(name,data);bpy.context.scene.collection.objects.link(ob)
            chunks.append(ob)
bpy.ops.object.select_all(action='DESELECT')
for ob in chunks:ob.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(HERE/'terrain-chunks.glb'),export_format='GLB',
    use_selection=True,export_animations=False,export_vertex_color='ACTIVE')
print(json.dumps({'runtime_chunks':len(chunks)//3,'lods':3,'collision_bytes':len(payload),
                  'obstacles':len(obstacles),'props':len(props),'coordinates':'Y-up, X/Z ground plane'}),flush=True)
