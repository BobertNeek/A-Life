"""Measured, reusable island assets built from approved September 30 sheets.

Blender CLI: --background --factory-startup --python scripts/build_island_assets.py
Each prop has an origin at ground level, applied scale, one shared vertex-colour
material and three mesh LODs. The editable source includes reference-image empties.
"""
import json
import math
import random
import sys
from pathlib import Path
import bpy
from mathutils import Vector, noise

sys.path.insert(0, str(Path(__file__).resolve().parent))
from island_art_common import (ROOT, REVIEW, material, mesh, setup_scene, camera,
                              render, export, reference, fit_dimensions, blueprint)

OUT = ROOT / 'crates/alife_game_app/assets/landscape/island'
TARGETS = {
    'Broadleaf': (6, 5.5, 8), 'Conifer': (4, 3.8, 12), 'Sapling': (2.8, 2.5, 4),
    'BerryBush': (1.8, 1.6, 1.3), 'CoastalShrub': (1.3, 1.1, .9),
    'Grass': (.6, .5, .45), 'Fern': (1, .9, .75), 'Flowers': (.7, .6, .6),
    'BoulderA': (2.4, 2, 1.8), 'BoulderB': (1.8, 1.5, 1.2),
    'BoulderC': (1.2, 1, .75), 'Cliff': (8, 4, 12), 'Scree': (4, 3, .6),
}


class Geometry:
    def __init__(self):
        self.v, self.f, self.c = [], [], []
        self.crowns = []

    def tube(self, points, radii, color, sides=8):
        base = len(self.v)
        for j, (p, r) in enumerate(zip(points, radii)):
            p = Vector(p)
            d = Vector(points[min(j+1, len(points)-1)]) - Vector(points[max(0, j-1)])
            d.normalize()
            u = d.cross(Vector((0, 1, 0))).normalized()
            if u.length < .1: u = Vector((1, 0, 0))
            w = d.cross(u).normalized()
            for i in range(sides):
                a = i*math.tau/sides
                self.v.append(tuple(p+r*(u*math.cos(a)+w*math.sin(a))))
                self.c.append(tuple(c*(.83+.17*math.cos(a)) for c in color))
        for j in range(len(points)-1):
            for i in range(sides):
                a=base+j*sides+i; b=base+j*sides+(i+1)%sides
                self.f.append((a,b,b+sides,a+sides))
        self.f += [tuple(base+i for i in reversed(range(sides))),
                   tuple(base+(len(points)-1)*sides+i for i in range(sides))]

    def crown(self, center, size, color, seed, subdivisions=1):
        import bmesh
        bm=bmesh.new()
        bmesh.ops.create_icosphere(bm, subdivisions=subdivisions+1, radius=1)
        bm.verts.ensure_lookup_table()
        base=len(self.v)
        for v in bm.verts:
            n=noise.noise_vector(v.co*3+Vector((seed,seed*.1,0))).x
            p=Vector(center)+Vector(tuple(v.co[k]*size[k]*(1+.13*n) for k in range(3)))
            self.v.append(tuple(p));self.c.append(tuple(c*(.8+.18*n+.13*v.co.z) for c in color))
        for f in bm.faces:self.f.append(tuple(base+v.index for v in f.verts))
        self.crowns.append(list(range(base, len(self.v))))
        bm.free()

    def leaf(self, base, tip, width, color, segments=3):
        base,tip=Vector(base),Vector(tip);d=tip-base
        across=d.cross(Vector((0,0,1))).normalized()
        if across.length<.1:across=Vector((1,0,0))
        k=len(self.v)
        for j in range(segments+1):
            t=j/segments;p=base+d*t+Vector((0,0,math.sin(math.pi*t)*d.length*.18))
            half=width*math.sin(math.pi*t)**.8+.001
            for side in (-1,0,1):
                self.v.append(tuple(p+across*half*side+Vector((0,0,half*(1-abs(side))*.28))))
                self.c.append(tuple(c*(.8+.2*t-.1*abs(side)) for c in color))
        for j in range(segments):
            for side in range(2):
                a=k+j*3+side;self.f.append((a,a+1,a+4,a+3))


def tree(kind):
    rng=random.Random(122+list(TARGETS).index(kind));g=Geometry()
    h=TARGETS[kind][2];r={'Broadleaf':.225,'Conifer':.175,'Sapling':.09}[kind]
    g.tube([(0,0,0),(.02,.04,h*.15),(-.08,.04,h*.4),(0,0,h*.8)],
           [r*1.4,r,r*.8,r*.2],(.24,.15,.065),10)
    if kind=='Conifer':
        for j in range(10):
            z=1.2+j*1.0;radius=1.9*(1-j/11)**.7
            for branch in range(6):
                a=branch*math.tau/6+j*.65;d=Vector((math.cos(a),math.sin(a),0))
                tip=d*radius+Vector((0,0,z-.35))
                g.tube([(0,0,z),tuple(tip)],[.065,.012],(.24,.15,.07),5)
                for k in range(2):
                    center=d*radius*(.40+k*.36)+Vector((0,0,z-.12-k*.1))
                    g.crown(center,(radius*.4,radius*.36,.26),(.09,.21,.075),j*7+branch,0)
        g.crown((0,0,11.5),(.22,.22,.6),(.12,.25,.09),19,0)
    else:
        for j in range(22 if kind=='Broadleaf' else 14):
            a=j*2.399;rad=(.7+.3*(j%3))*h*.27
            z=h*(.55+.28*rng.random());center=(math.cos(a)*rad,math.sin(a)*rad,z)
            g.tube([(0,0,h*.32),tuple(Vector(center)*Vector((.8,.8,1)))],
                   [r*.55,.018],(.24,.16,.07),6)
            g.crown(center,(h*.14,h*.13,h*.13),(.24,.38,.07),j+11,1)
            # Small folded leaf clusters break the canopy outline without alpha cards.
            for k in range(14):
                b=k*2.399;rad=h*.14
                p=Vector(center)+Vector((math.cos(b)*rad,math.sin(b)*rad,rng.uniform(-.3,.4)))
                g.leaf(p,p+Vector((math.cos(b)*.22,math.sin(b)*.22,.08)),.075,(.28,.43,.08),2)
        g.crown((0,0,h*.86),(h*.22,h*.19,h*.16),(.29,.43,.08),26,1)
    return g


def foliage(kind):
    rng=random.Random(312+list(TARGETS).index(kind));g=Geometry()
    if kind in ('BerryBush','CoastalShrub'):
        for j in range(12):
            a=j*2.399;rad=.4+.2*rng.random();z=.45+.35*rng.random()
            center=(math.cos(a)*rad,math.sin(a)*rad,z)
            g.tube([(0,0,0),center],[.028,.009],(.25,.16,.07),5)
            g.crown(center,(.33,.3,.3),(.21,.33,.065) if kind=='BerryBush' else (.23,.34,.19),j,1)
            if kind=='BerryBush':
                for k in range(3):
                    g.crown(Vector(center)+Vector((.15*math.cos(k*2),-.22,z*.05)),(.035,)*3,(.22,.08,.17),k,0)
    elif kind=='Fern':
        for j in range(7):
            a=j*2.399;d=Vector((math.cos(a),math.sin(a),0))
            for k in range(1,8):
                t=k/8;p=d*t*.55+Vector((0,0,.12+.42*math.sin(t*2)))
                for s in (-1,1):
                    tip=p+Vector((-d.y,d.x,0))*s*.18*math.sin(t*math.pi)+d*.08
                    g.leaf(p,tip,.035,(.14,.32,.06),2)
    else:
        for j in range(24):
            a=rng.random()*math.tau;radius=rng.random()*.25
            base=(radius*math.cos(a),radius*math.sin(a),0)
            tip=Vector(base)+Vector((math.cos(a)*.2,math.sin(a)*.2,rng.uniform(.25,.5)))
            g.leaf(base,tip,rng.uniform(.016,.032),(.24,.4,.06))
        if kind=='Flowers':
            for j in range(7):
                a=j*2.399;center=Vector((.25*math.cos(a),.25*math.sin(a),.38+.15*rng.random()))
                g.tube([(center.x,center.y,0),center],[.008,.005],(.12,.27,.05),4)
                for k in range(6):
                    a=k*math.tau/6
                    g.leaf(center,center+Vector((.085*math.cos(a),.085*math.sin(a),0)),.03,
                           (.89,.84,.64) if j%2 else (.59,.26,.59),2)
                g.crown(center,(.02,)*3,(.75,.45,.05),j,0)
    return g


def rock(kind):
    rng=random.Random(411+list(TARGETS).index(kind));g=Geometry()
    blocks=7 if kind=='Cliff' else (22 if kind=='Scree' else 3)
    for j in range(blocks):
        if kind=='Cliff':
            center=((j%4-1.5)*1.7,(j//4-.5)*1.4,0);size=(1.0,.95,rng.uniform(6,11))
        elif kind=='Scree':
            center=(rng.uniform(-2,2),rng.uniform(-1.5,1.5),0);size=(rng.uniform(.12,.4),rng.uniform(.1,.35),rng.uniform(.12,.5))
        else:
            center=(0,0,0) if not j else (rng.uniform(-.7,.7),rng.uniform(-.4,.4),0)
            size=(1-j*.2,.8-j*.15,1.4-j*.45)
        base=len(g.v);sides=9;rings=6
        radial=[rng.uniform(.82,1.15) for _ in range(sides)]
        for k in range(rings):
            t=k/(rings-1)
            for i in range(sides):
                a=i*math.tau/sides
                taper=(.65+.35*math.sin(t*math.pi*.8))*(1-.1*(k%2))
                x=center[0]+math.cos(a)*size[0]*radial[i]*taper+t*.15
                y=center[1]+math.sin(a)*size[1]*radial[i]*taper
                z=size[2]*t*(1+.06*math.sin(a*3+j))
                g.v.append((x,y,z))
                f=.85+.15*rng.random();moss=k==rings-1 and i%3==0
                g.c.append((.19*f,.29*f,.07*f) if moss else (.37*f,.35*f,.3*f))
        for k in range(rings-1):
            for i in range(sides):
                a=base+k*sides+i;b=base+k*sides+(i+1)%sides;g.f.append((a,b,b+sides,a+sides))
        g.f.extend([tuple(base+i for i in reversed(range(sides))),tuple(base+(rings-1)*sides+i for i in range(sides))])
    return g


def overview_tree(near, geometry, kind, collection):
    """Simplify connected canopy envelopes, rather than erasing leaf shells.

    Preserve the authored crown groups and conifer tiers at the existing L2
    triangle budget. Tiny leaves and branches inside crowns need no far mesh.
    """
    import bmesh
    groups = {}
    for index, indices in enumerate(geometry.crowns):
        if index == len(geometry.crowns)-1:
            key = 'top'
        elif kind == 'Conifer':
            key = index // 12
        else:
            center = sum((near.data.vertices[i].co for i in indices), Vector()) / len(indices)
            key = (center.x >= 0, center.y >= 0)
        groups.setdefault(key, []).extend(indices)
    # The first four rings are the original trunk, with ten vertices per ring.
    groups['trunk'] = list(range(40))
    pieces = []
    colors = near.data.color_attributes['ArtColor'].data
    for key, indices in groups.items():
        bm = bmesh.new()
        source = bm.verts.layers.int.new('source')
        for i in indices:
            v = bm.verts.new(near.data.vertices[i].co)
            v[source] = i
        bmesh.ops.convex_hull(bm, input=list(bm.verts), use_existing_faces=False)
        unused = [v for v in bm.verts if not v.link_faces]
        if unused:bmesh.ops.delete(bm, geom=unused, context='VERTS')
        bm.verts.ensure_lookup_table();bm.verts.index_update()
        vertices = [tuple(v.co) for v in bm.verts]
        tint = [tuple(colors[v[source]].color[:3]) for v in bm.verts]
        faces = [tuple(v.index for v in f.verts) for f in bm.faces]
        bm.free()
        piece = mesh('Overview crown envelope', vertices, faces, near.data.materials[0],
                     tint, key != 'trunk', collection)
        triangles = sum(len(p.vertices)-2 for p in piece.data.polygons)
        budget = (16 if kind == 'Sapling' else 18) if key == 'trunk' else (8 if kind == 'Sapling' else 14)
        bpy.ops.object.select_all(action='DESELECT')
        piece.select_set(True);bpy.context.view_layer.objects.active=piece
        dec=piece.modifiers.new('Preserve complete crown envelope','DECIMATE')
        dec.ratio=min(1, budget / triangles)
        bpy.ops.object.modifier_apply(modifier=dec.name)
        piece.select_set(False);pieces.append(piece)
    bpy.ops.object.select_all(action='DESELECT')
    for piece in pieces:piece.select_set(True)
    bpy.context.view_layer.objects.active=pieces[0]
    bpy.ops.object.join()
    distant=pieces[0];distant.name=kind+'_L2';distant.data.name=distant.name
    distant.select_set(False)
    budget={'Broadleaf':88,'Conifer':182,'Sapling':57}[kind]
    assert sum(len(p.vertices)-2 for p in distant.data.polygons) <= budget
    return distant


def build():
    OUT.mkdir(parents=True,exist_ok=True);REVIEW.mkdir(parents=True,exist_ok=True)
    for ob in list(bpy.data.objects):bpy.data.objects.remove(ob,do_unlink=True)
    scene=setup_scene();source=bpy.data.collections.new('Island_Asset_Blueprints');scene.collection.children.link(source)
    refs=bpy.data.collections.new('Approved_Image_References');scene.collection.children.link(refs)
    for filename in ('trees','bushes-ground-cover','rocks-cliffs','terrain-mountains'):
        reference(ROOT/f'target/artifacts/art-review/2026-09-30-island-blueprints-v1/{filename}-blueprint.png',filename,refs)
    mat=material('Island shared painted surface',(.3,.4,.13),True)
    receipt={};near=[];far=[]
    positions={'Broadleaf':(-7,3,0),'Conifer':(0,3,0),'Sapling':(6,3,0),
               'BerryBush':(-7,-3,0),'CoastalShrub':(-4,-3,0),'Grass':(-1,-3,0),'Fern':(1,-3,0),'Flowers':(3,-3,0),
               'BoulderA':(-7,-7,0),'BoulderB':(-3,-7,0),'BoulderC':(0,-7,0),'Cliff':(13,4,0),'Scree':(6,-7,0)}
    for kind,target in TARGETS.items():
        g=tree(kind) if kind in ('Broadleaf','Conifer','Sapling') else rock(kind) if kind.startswith('Boulder') or kind in ('Cliff','Scree') else foliage(kind)
        ob=mesh(kind+'_L0',g.v,g.f,mat,g.c,kind in ('Broadleaf','Conifer','Sapling','BerryBush','CoastalShrub'),source);fit_dimensions(ob,target)
        lod=ob.copy();lod.data=ob.data.copy();lod.name=kind+'_L1';lod.data.name=lod.name;source.objects.link(lod)
        bpy.context.view_layer.objects.active=lod;lod.select_set(True)
        dec=lod.modifiers.new('Distant silhouette','DECIMATE');dec.ratio=.22 if kind in ('Broadleaf','Conifer','Sapling') else .45
        bpy.ops.object.modifier_apply(modifier=dec.name)
        lod.select_set(False)
        if kind in ('Broadleaf','Conifer','Sapling'):
            distant=overview_tree(ob,g,kind,source)
        else:
            distant=ob.copy();distant.data=ob.data.copy();distant.name=kind+'_L2';distant.data.name=distant.name;source.objects.link(distant)
            bpy.context.view_layer.objects.active=distant;distant.select_set(True)
            dec=distant.modifiers.new('Overview silhouette','DECIMATE');dec.ratio=.20
            bpy.ops.object.modifier_apply(modifier=dec.name);distant.select_set(False)
        export([ob,lod,distant],OUT/(kind+'.glb'),export_animations=False)
        distant.location=positions[kind];distant.hide_render=True;distant.hide_set(True)
        ob.location=positions[kind];lod.location=positions[kind];lod.hide_render=True;lod.hide_set(True)
        ob['origin']='ground centre; Z-up; scale applied';ob['blueprint_dimensions_m']=target
        near.append(ob);far.append(lod)
        receipt[kind]={'width_depth_height_m':target,'near_triangles':sum(len(p.vertices)-2 for p in ob.data.polygons),
                       'far_triangles':sum(len(p.vertices)-2 for p in lod.data.polygons),
                       'overview_triangles':sum(len(p.vertices)-2 for p in distant.data.polygons)}
    scene.camera=camera('Asset library',(27,-38,26),(2,0,4),32)
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'island-assets.blend'))
    if '--no-review' not in sys.argv:
        render(scene.camera,REVIEW/'asset-library-material.png',(1400,1000))
    views=[('front',camera('Asset Front',(2,-48,6),(2,0,6),32)),
           ('three-quarter',camera('Asset Three Quarter',(27,-38,26),(2,0,4),32))]
    if '--no-review' not in sys.argv:blueprint(near,'island-assets',REVIEW,views)
    (OUT/'asset-specification.json').write_text(json.dumps({'blender':bpy.app.version_string,'units':'metres','assets':receipt},indent=2)+'\n')
    print('ISLAND_ASSETS',json.dumps(receipt),flush=True)


if __name__=='__main__':build()
