"""Procedural mountainous art terrain; Blender 5.2.1, no simulation authority.

Run after export_landscape.py. Saves editable terrain and a complete GLB scene.
"""
import sys
import math
import random
import json
from pathlib import Path
import bpy
from mathutils import Vector, noise

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from landscape_detail import mesh, material, ground_detail
HERE=ROOT/'crates/alife_game_app/assets/landscape'
OUT=HERE/'highlands'
REVIEW=ROOT/'target/artifacts/terrain-v2'


def smooth(a,b,x):
    t=max(0,min(1,(x-a)/(b-a)))
    return t*t*(3-2*t)


def river_x(y): return 12*math.sin(y/52)+.08*y-14
def river_width(y): return 4.8+1.2*math.sin(y/37)+smooth(240,570,y)*9


def height(x,y):
    n=noise.noise_vector(Vector((x*.022,y*.022,3.7)))
    meadow=4.3+2.0*math.sin(x/28)*math.cos(y/38)+1.6*n.x
    left=(-155+25*math.sin(y/75))
    right=(155+23*math.sin(y/67))
    a=math.exp(-((x-left)/56)**2)*(55+48*math.exp(-((y-210)/140)**2))
    b=math.exp(-((x-right)/59)**2)*(55+62*math.exp(-((y-280)/180)**2))
    far=105*math.exp(-((y-510)/100)**2)*(.65+.35*math.sin(x/52)**2)
    mountains=max(a,b,far)*smooth(-100,110,y)
    detail=noise.noise_vector(Vector((x*.068,y*.068,17)))
    erosion=(12*n.z+5*detail.x-5*abs(math.sin(x*.13+y*.061+n.y*2)))
    land=meadow+mountains+erosion*smooth(9,45,mountains)
    # Rounded benches at the foot of the larger rock walls.
    land+=5*math.exp(-((abs(x)-80)/32)**2)*smooth(-50,130,y)
    d=abs(x-river_x(y)); w=river_width(y)
    return -1.2+(land+1.2)*smooth(w*.70,w+9,d)


def road_dist(x,y):
    return min(abs(x-(river_x(y)-17-4*math.sin(y/32))),
               abs(x-(-68+.16*y+9*math.sin(y/46))))


def slope(x,y):
    return math.hypot(height(x+.6,y)-height(x-.6,y),height(x,y+.6)-height(x,y-.6))/1.2


def terrain_color(x,y,z,grade):
    n=noise.noise_vector(Vector((x*.22,y*.22,8)))
    f=.85+.17*n.x+.07*math.sin(x/12+y/18)
    grass=(.16*f,.29*f,.037*f)
    stone=(.29*f,.32*f,.31*f)
    gravel=(.36*f,.31*f,.21*f)
    rock=smooth(.55,1.3,grade)*smooth(12,35,z)
    # Alternating mineral bands follow actual elevation and tilted bedding.
    band=.94+.06*smooth(-.2,.35,math.sin(z*.45+x*.035))
    stone=tuple(v*band for v in stone)
    color=tuple(grass[k]*(1-rock)+stone[k]*rock for k in range(3))
    bank=1-smooth(river_width(y)+2,river_width(y)+10,abs(x-river_x(y)))
    road=(1-smooth(.9,2.7,road_dist(x,y)))*(1-smooth(.4,.9,grade))
    mix=max(bank*.8,road*.85)
    color=tuple(color[k]*(1-mix)+gravel[k]*mix for k in range(3))
    snow=smooth(101,126,z+n.z*8)*(1-smooth(1,1.9,grade))
    return tuple(color[k]*(1-snow)+(.78,.85,.90)[k]*snow for k in range(3))


def camera(name,where,target,lens):
    data=bpy.data.cameras.new(name); ob=bpy.data.objects.new(name,data)
    bpy.context.scene.collection.objects.link(ob)
    ob.location=where; ob.rotation_euler=(Vector(target)-ob.location).to_track_quat('-Z','Y').to_euler()
    data.lens=lens; data.clip_end=3000
    return ob


def export_models(models):
    bpy.ops.object.select_all(action='DESELECT')
    for ob in models:ob.select_set(True)
    # The shader combines a texture with vertex paint. Explicitly retain the
    # active color layer: material auto-detection can omit it from this graph.
    bpy.ops.export_scene.gltf(filepath=str(OUT/'Mountain_Valley.glb'),
        export_format='GLB',use_selection=True,export_animations=False,
        export_vertex_color='ACTIVE')


def build():
    OUT.mkdir(exist_ok=True); REVIEW.mkdir(parents=True,exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(HERE/'landscape.blend'))
    props={o.name:o for o in bpy.context.scene.objects if o.type=='MESH'}
    for ob in list(bpy.context.scene.objects):
        if ob.type!='MESH': bpy.data.objects.remove(ob,do_unlink=True)
    for p in props.values():
        p.hide_render=True; p.hide_viewport=True
    rng=random.Random(20260915)
    nx,ny=320,420
    vs=[];fs=[];cs=[]
    for j in range(ny+1):
        y=-350+1050*j/ny
        for i in range(nx+1):
            x=-400+800*i/nx; z=height(x,y)
            vs.append((x,y,z));cs.append(terrain_color(x,y,z,slope(x,y)))
    for j in range(ny):
        for i in range(nx):
            a=j*(nx+1)+i;fs.append((a,a+1,a+nx+2,a+nx+1))
    ground=mesh('Highlands_continuous_terrain',vs,fs,material('Highlands | meadow and strata',(.2,.3,.06),True),cs)
    ground['authority']='Art mesh only; world heights/collision require explicit runtime integration'
    ground['dimensions_m']=[800,1050]
    # Exportable UVs allow the existing tiled ground texture to supply mid-scale detail.
    uv=ground.data.uv_layers.new(name='GroundUV')
    for loop in ground.data.loops:
        p=ground.data.vertices[loop.vertex_index].co
        uv.data[loop.index].uv=(p.x/6,p.y/6)
    ground_detail(ground.data.materials[0])
    water=mesh('Highlands_river', [(-400,-350,.25),(400,-350,.25),(400,700,.25),(-400,700,.25)],[(0,1,2,3)],material('Highlands | river blue',(.035,.22,.28)))
    water.data.materials[0].node_tree.nodes.get('Principled BSDF').inputs['Roughness'].default_value=.26
    placed=[]
    def place(key,x,y,s=1,angle=None):
        p=props[key];ob=p.copy();ob.data=p.data
        bpy.context.scene.collection.objects.link(ob)
        ob.name='Highlands_'+key
        ob.hide_viewport=False;ob.hide_render=False
        ob.location=(x,y,height(x,y)-.035);ob.scale=(s,)*3
        ob.rotation_euler.z=rng.random()*math.tau if angle is None else angle
        placed.append(ob)
        return ob
    for i in range(640):
        x=rng.uniform(-230,230);y=rng.uniform(-100,550)
        if height(x,y)<2 or slope(x,y)>.65 or road_dist(x,y)<4 or height(x,y)>65:continue
        if abs(x-river_x(y))<river_width(y)+8:continue
        if math.hypot(x-32,y+55)<17:continue
        place('Oak_variant_'+str(i%3+1),x,y,rng.uniform(.75,1.4))
    # Outcrops sit along highland shoulders; rotated fractured faces break the smooth slopes.
    for i in range(54):
        y=rng.uniform(0,440);x=rng.choice([-1,1])*rng.uniform(85,145)
        if slope(x,y)>1.5:continue
        outcrop=place('Cliff_outcrop',x,y,rng.uniform(.75,1.7))
        outcrop.location.z-=.45
    for i in range(1400):
        x=rng.uniform(-190,190);y=rng.uniform(-110,510)
        z=height(x,y)
        if z<.5 or road_dist(x,y)<2:continue
        # Scree collects below cliffs, with occasional isolated meadow stones.
        if z<14 and i%4:continue
        place('Boulder_variant_'+str(i%3+1),x,y,rng.uniform(.7,2.6))
    for i in range(2400):
        if i<1200:
            x=rng.uniform(5,65);y=rng.uniform(-88,-10)
        else:
            x=rng.uniform(-115,115);y=rng.uniform(-100,330)
        if height(x,y)<1 or slope(x,y)>.9 or road_dist(x,y)<1.7:continue
        key='Grass_clump' if i%9 else ('Fern_patch' if i%18 else 'Wildflower_patch')
        place(key,x,y,rng.uniform(.85,1.7))
    for x,y,s in [(25,-55,3.0),(42,-40,2.4),(17,-38,1.7),(52,-28,2.3)]:
        place('Boulder_variant_1',x,y,s)
        place('Fern_patch',x+1.7,y-1,1.9)
        place('Wildflower_patch',x+2,y-2,1.9)
    for x,y in [(32,-63),(36,-59),(29,-60),(38,-64)]:
        place('Fern_patch',x,y,1.5)
        place('Wildflower_patch',x+1,y-.7,1.5)
    for i in range(45):
        x=rng.uniform(-100,100);y=rng.uniform(-80,300)
        if height(x,y)>2 and slope(x,y)<.5:place('Berry_shrub',x,y,1.3)
    models=[ground,water]+placed
    collection=bpy.data.collections.new('Highlands_Export');bpy.context.scene.collection.children.link(collection)
    for ob in models:collection.objects.link(ob)
    scene=bpy.context.scene
    scene.render.engine='CYCLES';scene.cycles.samples=20;scene.cycles.use_denoising=True
    scene.render.resolution_x=1400;scene.render.resolution_y=900;scene.render.resolution_percentage=100
    scene.world=bpy.data.worlds.new('Highlands daylight');scene.world.use_nodes=True
    wn=scene.world.node_tree.nodes; bg=wn.get('Background');bg.inputs['Color'].default_value=(.10,.34,.78,1);bg.inputs['Strength'].default_value=.55
    sun_data=bpy.data.lights.new('Highlands afternoon','SUN');sun_data.energy=2.8;sun_data.angle=.06
    sun=bpy.data.objects.new('Highlands afternoon',sun_data);scene.collection.objects.link(sun)
    sun.rotation_euler=(.48,-.55,-.65)
    scene.view_settings.view_transform='AgX';scene.view_settings.exposure=.3
    wide=camera('Highlands_wide',(155,-220,112),(0,210,36),35)
    camera('Highlands_ground',(34,-70,height(34,-70)+3.2),(25,-37,height(25,-37)+1.3),32)
    scene.camera=wide
    export_models(models)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'mountain-valley.blend'))
    receipt={'blender':bpy.app.version_string,'terrain_vertices':len(vs),'terrain_triangles':len(fs)*2,
             'height_range_m':[min(v[2] for v in vs),max(v[2] for v in vs)],'instances':len(placed),
             'unique_prop_meshes':len(props),'dimensions_m':[800,1050],
             'runtime':'Art asset only; no world height, collision, navigation or main-branch integration'}
    (REVIEW/'build-receipt.json').write_text(json.dumps(receipt,indent=2))
    print('HIGHLANDS_EXPORTED',json.dumps(receipt),flush=True)


if __name__=='__main__':
    if '--export-only' in sys.argv:
        bpy.ops.wm.open_mainfile(filepath=str(OUT/'mountain-valley.blend'))
        export_models(bpy.data.collections['Highlands_Export'].objects)
    else:
        build()
