"""Build approved island, its render LODs and authoritative geometry together.

One sample grid supplies Blender near meshes, physics and save terrain payloads.
No visual-only height adjustment or map-specific locomotion policy is introduced.
"""
import json
import math
import random
import struct
import sys
from pathlib import Path
import bpy
from mathutils import Vector, noise

sys.path.insert(0,str(Path(__file__).resolve().parent))
from island_art_common import ROOT, REVIEW, material, mesh, setup_scene, camera, render, export, reference

OUT=ROOT/'crates/alife_game_app/assets/landscape/island'
NX,NY=320,384
SPACING=3.125
ORIGIN=(-500,-600)


def smooth(a,b,x):
    t=max(0,min(1,(x-a)/(b-a)))
    return t*t*(3-2*t)


def river_x(y):return 22+33*math.sin(y/110)+.13*(-y)


def height(x,y):
    theta=math.atan2(y/535,x/445)
    coast=.87+.045*math.sin(theta*5+.6)+.03*math.sin(theta*9)
    radius=math.hypot(x/445,y/535)
    # Irregular coastal shape; continuous sea floor under an unbounded ocean.
    inland=(coast-radius)*440
    n=noise.noise_vector(Vector((x*.013,y*.013,31)))
    meadow=8+7*math.sin(x/79)*math.cos(y/87)+3*n.x
    hills=25*math.exp(-((x+120)/200)**2-((y-60)/310)**2)
    peaks=[(-100,205,206,92,135),(28,275,228,76,102),(118,180,160,92,128)]
    ridge=max(h*max(0,1-math.hypot((x-cx)/sx,(y-cy)/sy)/2.5)**1.65 for cx,cy,h,sx,sy in peaks)
    detail=noise.noise_vector(Vector((x*.052,y*.052,7)))
    ridge+=smooth(25,120,ridge)*(17*n.z+10*detail.x-14*abs(math.sin(x*.083+y*.06)))
    land=meadow+hills+ridge
    # Coastal shelves with rock headlands and broad southern beaches.
    rocky=smooth(-40,70,y)*(.6+.4*math.sin(theta*3+1)**2)
    shelf=20*rocky*smooth(3,12,inland)*(1-smooth(22,70,inland))
    land=land*smooth(-3,35,inland)+shelf
    land=-12+(land+12)*smooth(-30,4,inland)
    # A connected river from the mountain lake through the southern clearing.
    if y<200:
        width=6+5*(1-smooth(-400,100,y))
        dist=abs(x-river_x(y))
        riverbed=-1.8+max(0,y-25)*.12
        land=riverbed+(land-riverbed)*smooth(width*.6,width+8,dist)
    return land


def grade(x,y):return math.hypot(height(x+1,y)-height(x-1,y),height(x,y+1)-height(x,y-1))/2


def color(x,y,h,g):
    n=noise.noise_vector(Vector((x*.08,y*.08,5))).x
    grass=(.27,.39,.085);sand=(.61,.50,.30);stone=(.39,.37,.34);snow=(.79,.83,.83)
    shade=.92+.1*n
    rock=smooth(.6,1.5,g)*smooth(12,50,h)
    beach=1-smooth(1,8,h)
    col=tuple((grass[k]*(1-rock)+stone[k]*rock)*(1-beach)+sand[k]*beach for k in range(3))
    frost=smooth(164,211,h)*(1-smooth(1.3,2.6,g))
    col=tuple((col[k]*(1-frost)+snow[k]*frost)*shade for k in range(3))
    if h<0:col=(.27,.39,.36)
    return col


def build():
    for ob in list(bpy.data.objects):bpy.data.objects.remove(ob,do_unlink=True)
    scene=setup_scene();land=bpy.data.collections.new('Island_Continuous_Terrain');scene.collection.children.link(land)
    props_collection=bpy.data.collections.new('Island_Shared_Prop_Placements');scene.collection.children.link(props_collection)
    refs=bpy.data.collections.new('Approved_Image_References');scene.collection.children.link(refs)
    reference(ROOT/'target/artifacts/art-review/2026-09-30-terrain-hand-v3/island-terrain-review.png','Approved island',refs)
    reference(ROOT/'target/artifacts/art-review/2026-09-30-island-blueprints-v1/terrain-mountains-blueprint.png','Terrain dimensions',refs)
    vs=[];cs=[];heights=[]
    for j in range(NY+1):
        z=ORIGIN[1]+j*SPACING;y=-z
        for i in range(NX+1):
            x=ORIGIN[0]+i*SPACING;h=height(x,y)
            vs.append((x,y,h));heights.append(h);cs.append(color(x,y,h,grade(x,y)))
    # Peak target exact at sampled maximum without raising shoreline.
    highest=max(heights);factor=240/highest
    heights=[h*factor if h>0 else h for h in heights]
    vs=[(v[0],v[1],h) for v,h in zip(vs,heights)]
    fs=[]
    for j in range(NY):
        for i in range(NX):
            a=j*(NX+1)+i;b=a+1;d=a+NX+1;c=d+1;fs.extend([(a,c,b),(a,d,c)])
    mat=material('Island painted ground',(.3,.4,.1),True)
    # Existing ordinary glTF tiled normal/albedo detail, no runtime procedural shader.
    from landscape_detail import ground_detail
    ground_detail(mat)
    terrain=mesh('Island_continuous_terrain',vs,fs,mat,cs,True,land)
    # UV metre tiling accompanies the exported vertex tint.
    uv=terrain.data.uv_layers.new(name='GroundUV')
    for loop in terrain.data.loops:
        v=terrain.data.vertices[loop.vertex_index].co;uv.data[loop.index].uv=(v.x/6,v.y/6)
    terrain['width_depth_m']=[1000,1200];terrain['sea_level_m']=0;terrain['peak_height_m']=240
    water=mesh('Island ocean',[(-5000,-5000,0),(5000,-5000,0),(5000,5000,0),(-5000,5000,0)],[(0,1,2,3)],
               material('Island blue ocean',(.025,.24,.33),roughness=.28))
    # Reuse library data, keep the scenery file economical and editable.
    with bpy.data.libraries.load(str(OUT/'island-assets.blend'),link=False) as (source,destination):
        destination.objects=[n for n in source.objects if n.endswith('_L0')]
    library={ob.name[:-3]:ob for ob in destination.objects if ob}
    placements=[];obstacles=[];rng=random.Random(93026)
    def place(kind,x,y,scale):
        if math.hypot(x-80,y+250)<18:return
        h=height(x,y);h=h*factor if h>0 else h
        ob=library[kind].copy();ob.data=library[kind].data;props_collection.objects.link(ob)
        ob.name='Island_'+kind+'_'+str(len(placements));ob.location=(x,y,h)
        ob.scale=(scale,)*3;ob.rotation_euler.z=rng.random()*math.tau
        placements.append({'kind':kind,'position':[x,h,-y],'scale':scale,'yaw':ob.rotation_euler.z})
        if kind in ('Broadleaf','Conifer','Sapling'):
            r={'Broadleaf':.225,'Conifer':.175,'Sapling':.09}[kind]*scale
            obstacles.append([x-r,-y-r,x+r,-y+r,h,h+3*scale])
        elif kind.startswith('Boulder') or kind=='Cliff':
            bounds=[ob.matrix_world@Vector(v) for v in ob.bound_box]
            # Update matrix explicitly: no dependency on a viewport refresh.
            from mathutils import Matrix
            matrix=Matrix.Translation(ob.location)@ob.rotation_euler.to_matrix().to_4x4()@Matrix.Diagonal((*ob.scale,1))
            bounds=[matrix@Vector(v) for v in ob.bound_box]
            obstacles.append([min(p.x for p in bounds),min(-p.y for p in bounds),max(p.x for p in bounds),max(-p.y for p in bounds),h,max(p.z for p in bounds)])
    for _ in range(6000):
        x=rng.uniform(-420,420);y=rng.uniform(-485,485);h=height(x,y);g=grade(x,y)
        if h<7 or h>155 or g>.7 or abs(x-river_x(y))<20:continue
        forest=(math.sin(x*.032+y*.011)+math.cos(y*.028))>-.05
        if not forest:continue
        kind='Conifer' if h>55 or y>50 else ('Sapling' if rng.random()<.17 else 'Broadleaf')
        place(kind,x,y,rng.uniform(.8,1.25))
    for _ in range(900):
        x=rng.uniform(-430,430);y=rng.uniform(-480,470);h=height(x,y)
        if h<1 or h>170:continue
        kind='Cliff' if grade(x,y)>.9 and h>15 and rng.random()<.15 else rng.choice(['BoulderA','BoulderB','BoulderC','Scree'])
        place(kind,x,y,rng.uniform(.75,1.4))
    for _ in range(6000):
        x=rng.uniform(-330,330);y=rng.uniform(-410,330);h=height(x,y)
        if h<4 or h>90 or grade(x,y)>.7 or abs(x-river_x(y))<13:continue
        kind=rng.choices(['Grass','Fern','Flowers','BerryBush','CoastalShrub'],[40,10,7,3,3])[0]
        if kind in ('Grass','Fern','Flowers') and rng.random()>.35:continue
        place(kind,x,y,rng.uniform(.8,1.35))
    for _ in range(650):
        x=rng.uniform(15,145);y=rng.uniform(-315,-190)
        if height(x,y)>4 and grade(x,y)<.7 and abs(x-river_x(y))>13:
            place(rng.choice(['Grass','Grass','Fern','Flowers']),x,y,rng.uniform(.9,1.3))
    for ob in library.values():
        if not ob.users_collection:bpy.data.objects.remove(ob)
    OUT.mkdir(parents=True,exist_ok=True)
    (OUT/'props.json').write_text(json.dumps(placements,separators=(',',':'))+'\n',newline='\n')
    payload=bytearray(b'HLAND001')
    payload.extend(struct.pack('<IIfffI',NX+1,NY+1,*ORIGIN,SPACING,len(obstacles)))
    payload.extend(struct.pack('<'+'f'*len(heights),*heights))
    for box in obstacles:payload.extend(struct.pack('<6f',*box))
    (ROOT/'crates/alife_world/assets/island-v1.bin').write_bytes(payload)
    wide=camera('Island god view',(720,-990,820),(0,30,35),1450)
    scene.camera=wide
    camera('Island creature view',(78,-280,height(78,-280)*factor+2.4),(40,200,75),None,24)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'island-terrain.blend'))
    chunks=[];counts=[0,0,0];chunk_span=32
    for j in range(0,NY,chunk_span):
        for i in range(0,NX,chunk_span):
            for lod,step in enumerate((1,2,4)):
                xs=list(range(i,min(i+chunk_span,NX)+1,step));zs=list(range(j,min(j+chunk_span,NY)+1,step))
                cv=[];cc=[];cn=[];uvs=[];cf=[]
                for z in zs:
                    for x in xs:
                        k=z*(NX+1)+x;v=terrain.data.vertices[k]
                        cv.append(tuple(v.co));cc.append(cs[k]);cn.append(tuple(v.normal));uvs.append((v.co.x/6,v.co.y/6))
                w,h=len(xs),len(zs)
                for z in range(h-1):
                    for x in range(w-1):
                        a=z*w+x;b=a+1;d=a+w;c=d+1;cf.extend([(a,c,b),(a,d,c)])
                edge=list(range(w))+[z*w+w-1 for z in range(1,h)]+list(range((h-1)*w+w-2,(h-1)*w-1,-1))+[z*w for z in range(h-2,0,-1)]
                lower=[]
                for k in edge:
                    lower.append(len(cv));v=cv[k];cv.append((v[0],v[1],v[2]-20));cc.append(cc[k]);cn.append(cn[k]);uvs.append(uvs[k])
                for k,a in enumerate(edge):
                    n=(k+1)%len(edge);cf.extend([(a,edge[n],lower[n]),(a,lower[n],lower[k])])
                ob=mesh(f'Island_{i//32}_{j//32}_L{lod}',cv,cf,mat,cc,True)
                uv=ob.data.uv_layers.new(name='GroundUV')
                for loop in ob.data.loops:uv.data[loop.index].uv=uvs[loop.vertex_index]
                ob.data.normals_split_custom_set_from_vertices(cn)
                chunks.append(ob);counts[lod]+=len(cf)
    export(chunks,OUT/'terrain-chunks.glb',export_animations=False)
    for ob in chunks:ob.hide_render=True
    receipt={'blender':bpy.app.version_string,'width':NX+1,'depth':NY+1,'origin_xz':ORIGIN,'spacing_m':SPACING,
             'chunks_xz':[math.ceil(NX/32),math.ceil(NY/32)],'triangles_by_lod':counts,'skirt_depth_m':20,
             'props':len(placements),'collision_proxies':len(obstacles),'sea_level_m':0,'peak_height_m':max(heights),
             'new_game_origin_xz':[80,250]}
    (OUT/'terrain-specification.json').write_text(json.dumps(receipt,indent=2)+'\n')
    if '--no-review' not in sys.argv:
        render(wide,REVIEW/'island-god-view.png',(1400,1000))
        render(bpy.data.objects['Island creature view'],REVIEW/'island-creature-view.png',(1400,900))
    print('ISLAND_TERRAIN',json.dumps(receipt),flush=True)


if __name__=='__main__':build()
