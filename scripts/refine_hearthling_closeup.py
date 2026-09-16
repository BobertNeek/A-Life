"""Reference nose, orbital rims and smile; preserve approved eye dimensions."""
import math
import bpy
import bmesh
from mathutils import Vector


def refine_closeup():
    rig = bpy.data.objects['Hearthling_Rig']
    assert not rig.get('closeup_reference_revision'), 'Open the retained checkpoint first'
    assert rig.get('iris_radius_scale') == .75
    previous = rig.data.pose_position
    rig.data.pose_position = 'REST'
    head = bpy.data.objects['Hearthling_Head']
    dark = bpy.data.materials['Hearthling | mouth and nose']

    def smooth(t):
        t = max(0., min(1., t))
        return t*t*(3-2*t)

    def surface(x, z):
        hit, p, *_ = head.ray_cast(Vector((x, -2, z)), Vector((0, 1, 0)))
        assert hit, (x, z)
        return p.y

    for side in ['L', 'R']:
        rim = bpy.data.objects['Hearthling_OrbitalRim_'+side]
        vertices = list(rim.data.vertices)
        for start in range(0, len(vertices), 8):
            row = vertices[start:start+8]
            center = sum((v.co for v in row), Vector())/len(row)
            for v in row:
                v.co = center+(v.co-center)*1.5
                v.co.y -= .0015
        rim.data.materials.clear()
        rim.data.materials.append(dark)
        brow = bpy.data.objects['Hearthling_Brow_'+side]
        brow.data.materials.clear()
        brow.data.materials.append(dark)
        rim.data.update()

    # Reuse the existing brow material for a polished, vertex-painted nose.
    # This retains the six-material budget and keeps fur relief off it.
    nose = bpy.data.objects['Hearthling_Nose']
    material = bpy.data.materials['Hearthling | brows and tuft shadows']
    shader = next(n for n in material.node_tree.nodes if n.type == 'BSDF_PRINCIPLED')
    shader.inputs['Roughness'].default_value = .28
    shader.inputs['Coat Weight'].default_value = .18
    color_node = material.node_tree.nodes.new('ShaderNodeVertexColor')
    color_node.layer_name = 'CoatColor'
    material.node_tree.links.new(color_node.outputs['Color'], shader.inputs['Base Color'])
    nose.data.materials.clear()
    nose.data.materials.append(material)
    bpy.ops.object.select_all(action='DESELECT')
    nose.select_set(True)
    bpy.context.view_layer.objects.active = nose
    sub = nose.modifiers.new('Smooth nostril topology', 'SUBSURF')
    sub.levels = 1
    bpy.ops.object.modifier_move_up(modifier=sub.name)
    bpy.ops.object.modifier_apply(modifier=sub.name)
    center = Vector((0, -.4155453, 1.492))
    for v in nose.data.vertices:
        d = v.co-center
        v.co = center+Vector((d.x*1.24, d.y*1.14, d.z*1.12))
    attr = nose.data.color_attributes.new(name='CoatColor', type='FLOAT_COLOR', domain='POINT')
    for v in nose.data.vertices:
        x, y, z = v.co
        depression = max(math.exp(-((x-s*.033)/.012)**2-((z-1.480)/.009)**2) for s in [-1, 1])
        depression *= smooth((-.42-y)/.022)
        v.co.y += .006*depression
        color = (.125, .038, .016, 1)
        inner = (.004, .001, .0005, 1)
        shade = smooth(depression/.64)
        attr.data[v.index].color = tuple(color[k]*(1-shade)+inner[k]*shade for k in range(4))
    nose.data.update()

    # One curve authors the lip volumes, recessed mouth groove and dark seam.
    # Smooth away the old pad/chin intersection so it cannot form a second mouth.
    def mouth_z(x):
        u = x/.163
        return 1.410+.035*u*u-.006*math.sin(math.pi*min(1, abs(u)))

    bm = bmesh.new()
    bm.from_mesh(head.data)
    edges = [e for e in bm.edges if all(abs(v.co.x)<.205 and
             1.36<v.co.z<1.48 and v.co.y<-.30 for v in e.verts)]
    bmesh.ops.subdivide_edges(bm, edges=edges, cuts=1, use_grid_fill=True)
    bm.to_mesh(head.data)
    bm.free()
    for v in head.data.vertices:
        x,y,z = v.co
        weight = (1-smooth((abs(x)-.17)/.085))*smooth((z-1.32)/.04)
        weight *= (1-smooth((z-1.477)/.043))*smooth((-.265-y)/.065)
        if weight <= 0:
            continue
        distance = z-mouth_z(x)
        end = 1-smooth((abs(x)-.145)/.030)
        base = -.422+5.0*(z-1.450)**2+.90*x*x
        groove = .0035*math.exp(-(distance/.007)**2)
        lower_lip = -.0015*math.exp(-((distance+.015)/.011)**2)
        upper_lip = -.001*math.exp(-((distance-.013)/.010)**2)
        target = base+end*(groove+lower_lip+upper_lip)
        v.co.y = y*(1-weight)+target*weight
    head.data.update()
    bpy.context.view_layer.update()

    smile = bpy.data.objects['Hearthling_ClosedSmile']
    vertices = list(smile.data.vertices)
    for start in range(0, len(vertices), 8):
        row = vertices[start:start+8]
        center = sum((v.co for v in row), Vector())/len(row)
        x = center.x*1.10
        z = mouth_z(x)
        target = Vector((x, surface(x, z)-.0007, z))
        for v in row:
            v.co = target+(v.co-center)*.60
    smile.data.update()
    stem = bpy.data.objects['Hearthling_NoseToLip']
    vertices = list(stem.data.vertices)
    for start in range(0, len(vertices), 8):
        row = vertices[start:start+8]
        center = sum((v.co for v in row), Vector())/len(row)
        target = Vector((0, surface(0, center.z)-.0007, center.z))
        for v in row:
            v.co = target+(v.co-center)*.60
    stem.data.update()

    for side in ['L', 'R']:
        ear = bpy.data.objects['Hearthling_Ear_'+side]
        bone = rig.data.bones['ear.'+side]
        axis = (bone.tail_local-bone.head_local).normalized()
        attr = ear.data.color_attributes['CoatColor']
        for v in ear.data.vertices:
            t = (v.co-bone.head_local).dot(axis)/bone.length
            color = attr.data[v.index].color[:]
            ivory = color[0] > .70 and color[1] > .50
            tip = smooth((t-.57)/.36)*(.25 if ivory else .94)
            attr.data[v.index].color = tuple(color[k]*(1-tip)+(.075, .016, .006, 1)[k]*tip for k in range(4))

    # Preserve eyelids. Reallocate smooth body surfaces for nose and mouth
    # topology, then inspect existing walk/sleep deformations.
    models = [o for o in bpy.context.scene.objects if o.type == 'MESH' and o.name.startswith('Hearthling')]
    def triangles(ob):
        ob.data.calc_loop_triangles()
        return len(ob.data.loop_triangles)
    total = sum(triangles(o) for o in models)
    body = bpy.data.objects['Hearthling_Body']
    before = triangles(body)
    if total > 32700:
        bpy.ops.object.select_all(action='DESELECT')
        body.select_set(True)
        bpy.context.view_layer.objects.active = body
        dec = body.modifiers.new('Nose detail budget', 'DECIMATE')
        dec.ratio = (before-(total-32700))/before
        bpy.ops.object.modifier_move_up(modifier=dec.name)
        bpy.ops.object.modifier_apply(modifier=dec.name)
        total -= before-triangles(body)
    assert total <= 32768, total
    rig['closeup_reference_revision'] = 9
    rig.data.pose_position = previous
    bpy.context.view_layer.update()
    print({'closeup_revision': 9, 'triangles': total, 'eye_sizes': 'preserved'}, flush=True)


if __name__ == '__main__':
    refine_closeup()
