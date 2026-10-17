"""Portable CoastalShrub-only folded-leaf source reload/export, Blender 4.3.2.

Normal export loads the editable source without repeating the sculpt. The guarded
--author reconstruction reads the retained historical two-species native source;
it never overwrites that source or exports BerryBush. No runtime code, placement,
registration or other asset is changed. Run register_approved_art.py afterward.
"""
import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'crates/alife_game_app/assets/landscape/island'
SOURCE = ASSETS / 'CoastalShrub.leaf-sprays.blend'
HISTORICAL_SOURCE = ASSETS / 'island-biological-shrubs.blend'
HISTORICAL_SHA256 = 'cf1ab181da8b8131b575ce98492837835a40e4ccb44b0e7c8020d2ddcfdeba89'
sys.path.insert(0, str(ROOT / 'scripts'))
from compact_glb_colors import compact_colors

def digest(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def bounds(m): return [[f(v.co[k] for v in m.vertices) for k in range(3)] for f in (min,max)]
def colors(m):
    attr = m.color_attributes.active_color
    sums = [Vector((0,0,0)) for _ in m.vertices]; counts = [0]*len(m.vertices)
    for loop, c in zip(m.loops, attr.data):
        sums[loop.vertex_index] += Vector(c.color[:3]); counts[loop.vertex_index] += 1
    return [s / n for s,n in zip(sums, counts)]
def components(m, cs):
    graph=[set() for v in m.vertices]
    for edge in m.edges:
        a,b=edge.vertices;graph[a].add(b);graph[b].add(a)
    left=set(range(len(m.vertices)));result=[]
    while left:
        start=min(left);left.remove(start);pending=[start];group=[]
        while pending:
            i=pending.pop();group.append(i)
            for j in graph[i]:
                if j in left:left.remove(j);pending.append(j)
        color=sum((cs[i] for i in group), Vector())/len(group)
        category='wood' if color.x > color.y*1.2 else 'leaf'
        result.append((sorted(group),category))
    return result
def mesh_signature(m, subset=None):
    ids=set(range(len(m.vertices))) if subset is None else set(subset)
    cs=colors(m)
    # Order-independent triangle corner position/color records, with native
    # float32 coordinate bytes. This verifies stems and the untouched species.
    rec=[]
    for p in m.polygons:
        if set(p.vertices).issubset(ids):
            rec.append(sorted(tuple((*m.vertices[i].co,*cs[i])) for i in p.vertices))
    return hashlib.sha256(json.dumps(sorted(rec),separators=(',',':')).encode()).hexdigest()
def metrics(ob):
    m=ob.data;cs=colors(m);groups=components(m,cs)
    wood=[i for ids,k in groups if k=='wood' for i in ids]
    attachments=[]
    for ids,kind in groups:
        if kind!='wood':continue
        if ob.name.endswith('_L0'):
            tip_ids=sorted(ids,key=lambda i:m.vertices[i].co.z)[-5:]
            attachments.append(sum((m.vertices[i].co for i in tip_ids),Vector())/5)
        else:attachments.append(max((m.vertices[i].co for i in ids),key=lambda v:v.z).copy())
    blade_roots=[m.vertices[min(ids)].co for ids,k in groups if ob.get('prototype_pass') and k=='leaf' and len(ids) in (5,8)]
    root_error=max((min((root-attachment).length for attachment in attachments) for root in blade_roots),default=None)
    return {'name':ob.name,'vertices':len(m.vertices),'triangles':sum(len(p.vertices)-2 for p in m.polygons),
        'bounds_xyz':bounds(m),'location':list(ob.location),'rotation':list(ob.rotation_euler),'scale':list(ob.scale),
        'material_names':[x.name for x in m.materials],'color_attribute':m.color_attributes.active_color.name,
        'wood_signature':mesh_signature(m,wood),'mesh_signature':mesh_signature(m),
        'wood_components':sum(k=='wood' for _,k in groups),'leaf_components':sum(k=='leaf' for _,k in groups),
        'folded_blade_roots_checked':len(blade_roots),'leaf_root_attachment_max_error_m':root_error,
        'zero_area_faces':sum(p.area<1e-10 for p in m.polygons)}

# Blade directions share a windward/back pair and three longer leeward tips.
# L1 merges the lower two leeward blades, preserving the same spray vocabulary.
DIRECTIONS=[(-.27,-.075,-.065),(-.17,.145,.035),(.29,-.115,-.050),(.34,.012,.065),(.23,.145,.095)]
def make_spray(lo,hi,attachment,level,index):
    vs=[];fs=[];tags=[]
    selected=[0,1,2,3,4] if level==0 else [0,1,3,4]
    for blade in selected:
        d=Vector(DIRECTIONS[blade])
        angle=.17*math.sin(index*2.399)
        d.x,d.y=d.x*math.cos(angle)-d.y*math.sin(angle),d.x*math.sin(angle)+d.y*math.cos(angle)
        base=Vector((0,0,0))
        direction=d-base
        across=Vector((-direction.y,direction.x,0)).normalized()
        # All leaves are solid opaque folded geometry with no alpha/textures.
        width=[.063,.071,.080,.078,.068][blade]
        start=len(vs);vs.append(base);tags.append((blade,0,0))
        stations=(.30,.70) if level==0 else (.43,)
        for t in stations:
            p=base+direction*t+Vector((0,0,.045*math.sin(math.pi*t)))
            half=width*(math.sin(math.pi*t)**.8)
            for side in (-1,0,1):
                q=p+across*half*side
                q.z+=half*(.40 if side==0 else -.12)
                # A mild individual twist, never a flat oval crown.
                q.z+=side*.012*math.sin(index*.8+blade)
                vs.append(q);tags.append((blade,t,side))
        tip_index=len(vs);vs.append(d);tags.append((blade,1,0))
        fs.extend([(start,start+1,start+2),(start,start+2,start+3)])
        if level==0:
            fs.extend([(start+1,start+4,start+5),(start+1,start+5,start+2),
                       (start+2,start+5,start+6),(start+2,start+6,start+3)])
            last=start+4
        else:last=start+1
        fs.extend([(last,tip_index,last+1),(last+1,tip_index,last+2)])
    # Fit each crown's exact pre-existing envelope. Stem endpoints, all twelve
    # attachment positions, wind sweep, and each LOD's exact AABB stay fixed.
    newlo=[min(v[k] for v in vs) for k in range(3)];newhi=[max(v[k] for v in vs) for k in range(3)]
    for v in vs:
        for k in range(3):
            # Piecewise envelope mapping fixes the branch's exact attachment
            # while retaining all six original crown extrema. The first draft
            # used a plain fit and exposed branch tips; this corrects it.
            v[k]=attachment[k]+v[k]*((hi[k]-attachment[k])/newhi[k] if v[k]>=0 else (attachment[k]-lo[k])/-newlo[k])
    return vs,fs,tags

def author(ob,level):
    old=ob.data;oldbounds=bounds(old);cs=colors(old);groups=components(old,cs);verts=[];faces=[];paint=[];smooth=[]
    leaf_index=0;attachment=None
    for ids,category in groups:
        start=len(verts)
        if category=='wood':
            if level==0:
                tip_ids=sorted(ids,key=lambda i:old.vertices[i].co.z)[-5:]
                attachment=sum((old.vertices[i].co for i in tip_ids),Vector())/5
            else:attachment=max((old.vertices[i].co for i in ids),key=lambda v:v.z).copy()
            mapping={i:start+j for j,i in enumerate(ids)}
            verts.extend(tuple(old.vertices[i].co) for i in ids);paint.extend(tuple(cs[i]) for i in ids)
            for p in old.polygons:
                if set(p.vertices).issubset(ids):faces.append(tuple(mapping[i] for i in p.vertices));smooth.append(p.use_smooth)
        else:
            lo=[min(old.vertices[i].co[k] for i in ids) for k in range(3)];hi=[max(old.vertices[i].co[k] for i in ids) for k in range(3)]
            assert attachment is not None and all(lo[k]<attachment[k]<hi[k] for k in range(3))
            vs,fs,tags=make_spray(lo,hi,attachment,level,leaf_index);verts.extend(tuple(v) for v in vs)
            assert all(v==attachment for v,(_,t,_) in zip(vs,tags) if t==0)
            faces.extend(tuple(start+i for i in f) for f in fs);smooth.extend([True]*len(fs))
            for v,(blade,t,side) in zip(vs,tags):
                # Sample the existing sea-glass paint by proximity, then add a
                # restrained ridge/edge variation using only the same palette.
                nearest=sorted(ids,key=lambda i:(old.vertices[i].co-v).length_squared)[:3]
                original=sum((cs[i] for i in nearest),Vector())/len(nearest)
                factor=(1.045 if side==0 else .89)*(.96+.055*t)
                paint.append(tuple(min(1,max(0,x*factor)) for x in original))
            leaf_index+=1
    mesh=bpy.data.meshes.new(ob.name+' prototype');mesh.from_pydata(verts,[],faces);mesh.update()
    for mat in old.materials:mesh.materials.append(mat)
    attr=mesh.color_attributes.new(name='ArtColor',type='FLOAT_COLOR',domain='CORNER')
    for loop,c in zip(mesh.loops,attr.data):c.color=(*paint[loop.vertex_index],1)
    mesh.color_attributes.active_color=attr
    for p,s in zip(mesh.polygons,smooth):p.use_smooth=s
    ob.data=mesh
    if old.users==0:bpy.data.meshes.remove(old)
    mesh.name=ob.name
    ob['prototype_pass']='2026-10-16 pointed folded-leaf sprays; isolated review, not integrated'
    ob['art_owner']='CoastalShrub.leaf-sprays.blend'
    assert bounds(mesh)==oldbounds,(ob.name,bounds(mesh),oldbounds)

def export_kind(kind,path):
    obs=[bpy.data.objects[f'{kind}_L{i}'] for i in range(3)];state=[]
    bpy.ops.object.select_all(action='DESELECT')
    for ob in obs:
        state.append((ob,ob.location.copy(),ob.hide_get(),ob.hide_render))
        ob.location=(0,0,0);ob.hide_set(False);ob.hide_render=False;ob.select_set(True)
    bpy.ops.export_scene.gltf(filepath=str(path),export_format='GLB',use_selection=True,export_animations=False,export_vertex_color='ACTIVE')
    compact_colors(path)
    for ob,loc,hidden,render_hidden in state:ob.location=loc;ob.hide_set(hidden);ob.hide_render=render_hidden


def main():
    assert bpy.app.version == (4, 3, 2), 'Use the reviewed Blender 4.3.2 exporter'
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--author', action='store_true',
                        help='Reconstruct from exact retained historical source')
    parser.add_argument('--output-dir', type=Path, default=ASSETS,
                        help='Write only CoastalShrub.glb here; source is never resaved on normal export')
    args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    if args.author:
        assert digest(HISTORICAL_SOURCE) == HISTORICAL_SHA256, 'Historical shrub source changed'
        bpy.ops.wm.open_mainfile(filepath=str(HISTORICAL_SOURCE))
        for level in (0, 1):
            author(bpy.data.objects[f'CoastalShrub_L{level}'], level)
        for level in range(3):
            ob = bpy.data.objects[f'CoastalShrub_L{level}']
            ob.location = (0, 0, 0)
            ob.hide_set(level != 0)
            ob.hide_render = level != 0
        bpy.context.preferences.filepaths.save_version = 0
        bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE))
    else:
        bpy.ops.wm.open_mainfile(filepath=str(SOURCE))
    for level in range(3):
        ob = bpy.data.objects[f'CoastalShrub_L{level}']
        assert tuple(ob.location) == (0., 0., 0.)
        assert tuple(ob.rotation_euler) == (0., 0., 0.)
        assert tuple(ob.scale) == (1., 1., 1.)
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    target = output / 'CoastalShrub.glb'
    export_kind('CoastalShrub', target)
    print('COASTAL_LEAF_SPRAYS_EXPORT', json.dumps({
        'source': SOURCE.relative_to(ROOT).as_posix(),
        'source_sha256': digest(SOURCE), 'runtime_sha256': digest(target),
        'runtime_bytes': target.stat().st_size,
        'lods': [metrics(bpy.data.objects[f'CoastalShrub_L{i}']) for i in range(3)],
        'other_assets_exported': False, 'live_game': False, 'physical_gpu': False}), flush=True)


if __name__ == '__main__':
    main()
