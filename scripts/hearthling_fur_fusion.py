"""Fuse coat clumps into the skin, preserving colors and skeletal weights."""
import bpy
import bmesh
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.geometry import barycentric_transform
from hearthling_head import EYE_X, EYE_WIDTH, EYE_UPPER, EYE_LOWER
import math


def fuse_coat(rig,body,fur):
    head=bpy.data.objects['Hearthling_Head']
    # Ear and tail fur use their own deforming surfaces. Head and body clumps
    # share the same coat material, and can become one continuous skin each.
    regions={}
    for v in fur.data.vertices:
        groups={fur.vertex_groups[g.group].name:g.weight for g in v.groups}
        if groups.get('head',0)>.96: region='head'
        elif any(n.startswith('tail.') for n in groups): region='tail'
        elif 'ear.L' in groups: region='ear.L'
        elif 'ear.R' in groups: region='ear.R'
        else: region='body'
        regions[v.index]=region

    targets=[('head',head,9500,.007),('body',body,8000,.009),
             ('tail',bpy.data.objects['Hearthling_Tail'],2600,.009),
             ('ear.L',bpy.data.objects['Hearthling_Ear_L'],1400,.005),
             ('ear.R',bpy.data.objects['Hearthling_Ear_R'],1400,.005)]
    for region,target,budget,voxel in targets:
        part=fur.copy(); part.data=fur.data.copy()
        bpy.context.collection.objects.link(part)
        for mod in list(part.modifiers): part.modifiers.remove(mod)
        bm=bmesh.new(); bm.from_mesh(part.data); bm.verts.ensure_lookup_table()
        bmesh.ops.delete(bm,geom=[v for v in bm.verts if regions[v.index]!=region],context='VERTS')
        bm.to_mesh(part.data); bm.free()
        # Retain target identity and the production material names.
        bpy.ops.object.select_all(action='DESELECT')
        target.select_set(True); part.select_set(True)
        bpy.context.view_layer.objects.active=target
        bpy.ops.object.join()
        _fuse_surface(target,voxel,budget,region=='head')

    bpy.data.objects.remove(fur,do_unlink=True)
    # Recompute the lid falloff on the finished head, not on the control cage.
    from hearthling_eyes import weight_head_lids
    weight_head_lids(head)


def _fuse_surface(ob,voxel,budget,eye_openings):
    donor=ob.data.copy(); donor.calc_loop_triangles()
    positions=[v.co.copy() for v in donor.vertices]
    triangles=[tuple(t.vertices) for t in donor.loop_triangles]
    tree=BVHTree.FromPolygons(positions,triangles,all_triangles=True)
    color=[c.color[:] for c in donor.color_attributes['CoatColor'].data]
    weights=[{ob.vertex_groups[g.group].name:g.weight for g in v.groups} for v in donor.vertices]
    if eye_openings:
        bm=bmesh.new(); bm.from_mesh(ob.data)
        bmesh.ops.holes_fill(bm,edges=[e for e in bm.edges if e.is_boundary],sides=0)
        bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
        bm.to_mesh(ob.data); bm.free()
    bpy.ops.object.select_all(action='DESELECT'); ob.select_set(True)
    bpy.context.view_layer.objects.active=ob
    remesh=ob.modifiers.new('Continuous fur sculpt','REMESH')
    remesh.mode='VOXEL'; remesh.voxel_size=voxel; remesh.use_smooth_shade=True
    bpy.ops.object.modifier_move_up(modifier=remesh.name)
    bpy.ops.object.modifier_apply(modifier=remesh.name)
    bm=bmesh.new(); bm.from_mesh(ob.data)
    for _ in range(5):
        bmesh.ops.smooth_vert(bm,verts=list(bm.verts),factor=.42,
                             use_axis_x=True,use_axis_y=True,use_axis_z=True)
    bm.to_mesh(ob.data); bm.free()
    if eye_openings:
        for sign in [-1,1]:
            vs=[]; fs=[]; sides=64
            for y in [-.9,-.095]:
                for i in range(sides):
                    a=2*math.pi*i/sides
                    vs.append((sign*EYE_X+EYE_WIDTH*math.cos(a),y,
                               2.075+(EYE_UPPER if math.sin(a)>=0 else EYE_LOWER)*math.sin(a)))
            fs=[(i,(i+1)%sides,(i+1)%sides+sides,i+sides) for i in range(sides)]
            fs.extend([tuple(reversed(range(sides))),tuple(range(sides,2*sides))])
            data=bpy.data.meshes.new('Eyelid aperture cutter'); data.from_pydata(vs,[],fs)
            cutter=bpy.data.objects.new('Eyelid aperture cutter',data); bpy.context.collection.objects.link(cutter)
            bm=bmesh.new(); bm.from_mesh(data)
            bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces)); bm.to_mesh(data); bm.free()
            boolean=ob.modifiers.new('Inset eye opening','BOOLEAN')
            boolean.operation='DIFFERENCE'; boolean.solver='EXACT'; boolean.object=cutter
            bpy.ops.object.modifier_move_up(modifier=boolean.name)
            bpy.ops.object.modifier_apply(modifier=boolean.name)
            bpy.data.objects.remove(cutter,do_unlink=True)
    ob.data.calc_loop_triangles()
    decimate=ob.modifiers.new('Sculpt triangle budget','DECIMATE')
    decimate.ratio=min(1,budget/len(ob.data.loop_triangles))
    bpy.ops.object.modifier_move_up(modifier=decimate.name)
    bpy.ops.object.modifier_apply(modifier=decimate.name)
    attr=ob.data.color_attributes.get('CoatColor')
    if attr: ob.data.color_attributes.remove(attr)
    attr=ob.data.color_attributes.new(name='CoatColor',type='FLOAT_COLOR',domain='POINT')
    for g in list(ob.vertex_groups): ob.vertex_groups.remove(g)
    for name in sorted({name for row in weights for name in row}): ob.vertex_groups.new(name=name)
    for v in ob.data.vertices:
        p,_,index,_=tree.find_nearest(v.co)
        ids=triangles[index]; a,b,c=(positions[i] for i in ids)
        w=barycentric_transform(p,a,b,c,Vector((1,0,0)),Vector((0,1,0)),Vector((0,0,1)))
        w=Vector(tuple(max(0,x) for x in w)); w/=sum(w)
        attr.data[v.index].color=tuple(sum(color[index][channel]*f for index,f in zip(ids,w)) for channel in range(4))
        combined={}
        for index,f in zip(ids,w):
            for name,weight in weights[index].items(): combined[name]=combined.get(name,0)+weight*f
        for name,weight in combined.items():
            if weight>.00001: ob.vertex_groups[name].add([v.index],weight,'REPLACE')
    for face in ob.data.polygons: face.use_smooth=True
    ob.data.update(); bpy.data.meshes.remove(donor)
