"""Chubby raccoon-like feet; modifies only the lower body mesh.

Run refine_feet() with the accepted character loaded. The open ankle rings are
bridged into new foot surfaces, keeping the existing rig and upper-leg vertices.
New faces use a nondegenerate projection into the accepted atlas's neutral corner.
"""
import math
import bpy
import bmesh
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.geometry import barycentric_transform


def _smooth(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def refine_feet(triangles_per_foot=800):
    rig = bpy.data.objects['Hearthling_Rig']
    assert not rig.get('raccoon_foot_revision'), 'Use the pre-foot checkpoint'
    body = bpy.data.objects['Hearthling_Body']
    previous = rig.data.pose_position
    rig.data.pose_position = 'REST'
    bpy.context.view_layer.update()
    donor = body.data.copy()
    donor.calc_loop_triangles()
    positions = [v.co.copy() for v in donor.vertices]
    triangles = [tuple(t.vertices) for t in donor.loop_triangles]
    tree = BVHTree.FromPolygons(positions, triangles, all_triangles=True)
    donor_colors = [tuple(c.color) for c in donor.color_attributes['CoatColor'].data]
    names = {g.index:g.name for g in body.vertex_groups}
    donor_weights = [{names[g.group]:g.weight for g in v.groups} for v in donor.vertices]
    floor = min(v.co.z for v in donor.vertices)
    cut_body = floor + .225
    cut_foot = floor + .185
    white = [donor_colors[v.index] for v in donor.vertices
             if v.co.z < floor + .15 and min(donor_colors[v.index][:3]) > .45]
    cream = tuple(sum(c[k] for c in white)/len(white) for k in range(4)) if white else (.85,.76,.60,1)
    brown = (.115,.034,.014,1)
    coat = bpy.data.materials['Hearthling | vertex-colored coat']

    bm = bmesh.new()
    bm.from_mesh(body.data)
    region = bm.faces.layers.int.get('RaccoonFootRegion') or bm.faces.layers.int.new('RaccoonFootRegion')
    for f in bm.faces: f[region] = 0
    bmesh.ops.bisect_plane(bm, geom=list(bm.verts)+list(bm.edges)+list(bm.faces),
                          plane_co=(0,0,cut_body), plane_no=(0,0,1),
                          clear_inner=True, clear_outer=False)
    bm.to_mesh(body.data)
    bm.free()
    report = []
    for side, sign in [('L',-1),('R',1)]:
        x = rig.data.bones['foot.'+side].head_local.x
        parts = []

        def mass(label, center, scale, angle=0):
            bpy.ops.mesh.primitive_uv_sphere_add(segments=24, ring_count=16, location=center)
            ob = bpy.context.object
            ob.name = 'RaccoonFoot_'+side+'_'+label
            ob.scale = scale
            ob.rotation_euler.z = angle
            bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
            parts.append(ob)

        mass('heel',(x,-.010,floor+.060),(.085,.085,.085))
        mass('midfoot',(x,-.135,floor+.058),(.110,.132,.072))
        mass('instep',(x,.005,floor+.160),(.088,.107,.140))
        lengths = [.078,.092,.100,.090,.074]
        widths = [.029,.032,.033,.031,.028]
        for i, (off, length, width) in enumerate(zip([-.104,-.052,0,.052,.104],lengths,widths)):
            mass('toe'+str(i),(x+sign*off,-.246-length*.45,floor+.028),
                 (width,length,.045), sign*.10*off/.104)
        bpy.ops.object.select_all(action='DESELECT')
        for ob in parts: ob.select_set(True)
        bpy.context.view_layer.objects.active = parts[0]
        bpy.ops.object.join()
        foot = bpy.context.object
        remesh = foot.modifiers.new('Five connected toe volumes','REMESH')
        remesh.mode = 'VOXEL'
        remesh.voxel_size = .0055
        remesh.use_smooth_shade = True
        bpy.ops.object.modifier_apply(modifier=remesh.name)
        bm = bmesh.new()
        bm.from_mesh(foot.data)
        for _ in range(3):
            bmesh.ops.smooth_vert(bm, verts=list(bm.verts),factor=.30,
                                 use_axis_x=True,use_axis_y=True,use_axis_z=True)
        for v in bm.verts:
            v.co.z = max(floor, v.co.z)
        bm.to_mesh(foot.data)
        bm.free()
        foot.data.calc_loop_triangles()
        dec = foot.modifiers.new('Foot surface budget','DECIMATE')
        dec.ratio = min(1,(triangles_per_foot-60)/len(foot.data.loop_triangles))
        bpy.ops.object.modifier_apply(modifier=dec.name)
        bm = bmesh.new()
        bm.from_mesh(foot.data)
        bmesh.ops.bisect_plane(bm, geom=list(bm.verts)+list(bm.edges)+list(bm.faces),
                              plane_co=(0,0,cut_foot),plane_no=(0,0,1),
                              clear_inner=False,clear_outer=True)
        bm.to_mesh(foot.data)
        bm.free()
        foot.data.materials.clear()
        foot.data.materials.append(coat)
        tag = foot.data.attributes.new(name='RaccoonFootRegion',type='INT',domain='FACE')
        for p in foot.data.polygons: tag.data[p.index].value = 1 if side == 'L' else 2
        attr = foot.data.color_attributes.new(name='CoatColor',type='FLOAT_COLOR',domain='POINT')
        for name in names.values(): foot.vertex_groups.new(name=name)
        for v in foot.data.vertices:
            p,_,tid,_ = tree.find_nearest(v.co)
            ids = triangles[tid]
            a,b,c = (positions[j] for j in ids)
            bary = barycentric_transform(p,a,b,c,Vector((1,0,0)),Vector((0,1,0)),Vector((0,0,1)))
            bary = Vector(tuple(max(0,z) for z in bary))
            bary /= sum(bary)
            ankle = _smooth((v.co.z-floor-.115)/.065)
            old_color = [sum(donor_colors[j][k]*w for j,w in zip(ids,bary)) for k in range(4)]
            sole = 1-_smooth((v.co.z-floor-.020)/.045)
            color = [cream[k]*(1-sole)+brown[k]*sole for k in range(4)]
            attr.data[v.index].color = tuple(color[k]*(1-ankle)+old_color[k]*ankle for k in range(4))
            weights = {}
            for j,w in zip(ids,bary):
                for name,weight in donor_weights[j].items():
                    weights[name] = weights.get(name,0)+weight*w*ankle
            weights['foot.'+side] = weights.get('foot.'+side,0)+(1-ankle)
            total = sum(weights.values())
            for name,weight in weights.items():
                if weight > 1e-7: foot.vertex_groups[name].add([v.index],weight/total,'REPLACE')
        uv = foot.data.uv_layers.new(name=body.data.uv_layers.active.name)
        for loop in foot.data.loops:
            p = foot.data.vertices[loop.vertex_index].co
            uv.data[loop.index].uv = ((p.x-x+.16)/.32,(p.y+.42)/.50)
        foot.data.color_attributes.active_color = attr
        for p in foot.data.polygons: p.use_smooth=True
        bpy.ops.object.select_all(action='DESELECT')
        body.select_set(True)
        foot.select_set(True)
        bpy.context.view_layer.objects.active = body
        bpy.ops.object.join()
        bm = bmesh.new()
        bm.from_mesh(body.data)
        boundary = [e for e in bm.edges if e.is_boundary
                    and all(v.co.x*sign>0 and v.co.z < cut_body+.001 for v in e.verts)]
        assert boundary, ('Missing ankle boundaries',side)
        faces = bmesh.ops.bridge_loops(bm,edges=boundary)['faces']
        region = bm.faces.layers.int['RaccoonFootRegion']
        for f in faces:
            f[region] = 1 if side == 'L' else 2
            f.smooth = True
        bmesh.ops.recalc_face_normals(bm,faces=[f for f in bm.faces if all(v.co.z <= cut_body+.001 for v in f.verts)])
        bm.to_mesh(body.data)
        bm.free()
        report.append({'side':side,'bridge_faces':len(faces),'toe_count':5})
    body.data.update()
    # Flat normal-map area already present in the accepted 2048-square atlas.
    # Dominant-plane projection keeps every new triangle's UV area nonzero.
    # Inset 8..56 texels leaves an ample filtering margin inside its blue corner.
    tag = body.data.attributes['RaccoonFootRegion']
    uv = body.data.uv_layers.active
    for polygon in body.data.polygons:
        side = tag.data[polygon.index].value
        if not side: continue
        x = rig.data.bones['foot.'+('L' if side == 1 else 'R')].head_local.x
        drop = max(range(3),key=lambda k:abs(polygon.normal[k]))
        axes = [k for k in range(3) if k != drop]
        for i in polygon.loop_indices:
            p = body.data.vertices[body.data.loops[i].vertex_index].co
            q = ((p.x-x+.17)/.34,(p.y+.43)/.55,(p.z-floor)/.25)
            uv.data[i].uv = tuple((8+48*q[k])/2048 for k in axes)
    body.data.calc_loop_triangles()
    rig['raccoon_foot_revision'] = 1
    rig.data.pose_position = previous
    bpy.context.view_layer.update()
    bpy.data.meshes.remove(donor)
    return {'floor':floor,'cut_body':cut_body,'body_triangles_after':len(body.data.loop_triangles),
            'feet':report,'changed_region':'Hearthling_Body at or below ankle cut only',
            'note':'RaccoonFootRegion FACE INT tags 1=left,2=right; new UVs use existing neutral-blue corner 8..56/2048'}
