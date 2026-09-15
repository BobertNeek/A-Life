"""Recessed spherical eyes and rotating eyelids; Blender authoring only."""
import math
import bpy
import bmesh
from hearthling_head import EYE_X, EYE_WIDTH, EYE_UPPER, EYE_LOWER


EYE_Y = -.170
EYE_Z = 2.075
RADIUS = .133
UPPER_ANGLE = 1.07
LOWER_ANGLE = -1.02


def build_eyes(rig, body, mesh, tube, cream, dark):
    # The globe is one curved surface. Iris and pupil are color regions on it,
    # not shallow disks stacked in front of a flattened white ellipsoid.
    material = bpy.data.materials.new('Hearthling | spherical eyes')
    material.use_nodes = True
    shader = material.node_tree.nodes.get('Principled BSDF')
    shader.inputs['Roughness'].default_value = .16
    shader.inputs['IOR'].default_value = 1.4
    shader.inputs['Coat Weight'].default_value = .45
    color = material.node_tree.nodes.new('ShaderNodeVertexColor')
    color.layer_name = 'EyeColor'
    material.node_tree.links.new(color.outputs['Color'], shader.inputs['Base Color'])

    bpy.ops.object.select_all(action='DESELECT')
    rig.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode='EDIT')
    for sign, side in [(-1, 'L'), (1, 'R')]:
        for part in ['upper', 'lower']:
            bone = rig.data.edit_bones.new('lid_' + part + '.' + side)
            bone.head = (sign*EYE_X, EYE_Y, EYE_Z)
            bone.tail = (sign*EYE_X, EYE_Y, EYE_Z+.1)
            bone.parent = rig.data.edit_bones['head']
    bpy.ops.object.mode_set(mode='OBJECT')

    from hearthling_head import make_head, head_y
    head=make_head(body,mesh,cream,EYE_Y,EYE_Z,RADIUS)
    for sign, side in [(-1, 'L'), (1, 'R')]:
        x=sign*EYE_X
        verts = []; faces = []; colors = []
        pupil_angle = math.asin(.067/RADIUS)
        iris_angle = math.asin(.097/RADIUS)
        # Extra rings at material boundaries keep a crisp limbal ring and pupil.
        angles = [0, .12, .24, pupil_angle-.008, pupil_angle+.008,
                  .54, .60, iris_angle-.025, iris_angle, iris_angle+.014,
                  .82, 1.0, 1.2, 1.4, 1.6, 1.85, 2.1, 2.4, 2.7, math.pi]
        segments = 48
        for theta in angles:
            for j in range(segments):
                phi = j*2*math.pi/segments
                dx = RADIUS*math.sin(theta)*math.cos(phi)
                dz = RADIUS*math.sin(theta)*math.sin(phi)
                dy = -RADIUS*math.cos(theta)
                verts.append((x+dx, EYE_Y+dy, EYE_Z+dz))
                if theta < pupil_angle:
                    c = (.004,.007,.004,1)
                elif theta <= iris_angle:
                    t = (theta-pupil_angle)/(iris_angle-pupil_angle)
                    ray = .88+.08*math.sin(phi*31)+.04*math.cos(phi*19)
                    edge = max(0,(t-.72)/.28)
                    c = tuple(((.24,.34,.075)[k]*(1-edge)+(.022,.045,.009)[k]*edge)*ray for k in range(3))+(1,)
                else:
                    c = (.84,.82,.68,1)
                colors.append(c)
        for i in range(len(angles)-1):
            for j in range(segments):
                a=i*segments+j; b=i*segments+(j+1)%segments
                faces.append((a,a+segments,b+segments,b))
        globe = mesh('Hearthling_Eyeball_'+side, verts, faces, material)
        attr = globe.data.color_attributes.new(name='EyeColor', type='FLOAT_COLOR', domain='POINT')
        for item,c in zip(attr.data,colors): item.color=c
        # Weld the polar rings so the pupil center has one smooth normal.
        bm=bmesh.new(); bm.from_mesh(globe.data)
        bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.000001)
        bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
        bm.to_mesh(globe.data); bm.free(); globe.data.update()

        # Rigid curved shells retain clearance from the globe throughout a blink.
        # Blending a head surface between two rotations cuts through the sphere.
        for part,latitude,offset,radius in [('Upper',.40,-.37,RADIUS+.005),('Lower',-.35,.32,RADIUS+.004)]:
            vs=[]; fs=[]; segments=32
            for ring in range(6):
                t=ring/5; lat=latitude+(math.copysign(math.pi/2,latitude)-latitude)*t
                for j in range(segments):
                    a=2*math.pi*j/segments
                    dy=-radius*math.cos(lat)*math.sin(a); dz=radius*math.sin(lat)
                    vs.append((x+radius*math.cos(lat)*math.cos(a),
                               EYE_Y+dy*math.cos(offset)-dz*math.sin(offset),
                               EYE_Z+dy*math.sin(offset)+dz*math.cos(offset)))
                if ring:
                    for j in range(segments):
                        a=(ring-1)*segments+j; b=(ring-1)*segments+(j+1)%segments
                        fs.append((a,b,b+segments,a+segments))
            lid=mesh('Hearthling_'+part+'Eyelid_'+side,vs,fs,cream,'lid_'+part.lower()+'.'+side)
            bm=bmesh.new(); bm.from_mesh(lid.data)
            bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.000001)
            bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
            bm.to_mesh(lid.data); bm.free()
            for poly in lid.data.polygons: poly.use_smooth=True
        points=[]
        for i in range(33):
            a=math.pi*i/32; radius=RADIUS+.006
            dy=-radius*math.cos(.40)*math.sin(a); dz=radius*math.sin(.40)
            points.append((x+radius*math.cos(.40)*math.cos(a),
                           EYE_Y+dy*math.cos(-.37)-dz*math.sin(-.37),
                           EYE_Z+dy*math.sin(-.37)+dz*math.cos(-.37)))
        tube('Hearthling_LidEdge_'+side,points,[.0015+.002*math.sin(math.pi*i/32) for i in range(33)],dark,'lid_upper.'+side,sides=8)
    weight_head_lids(head)
    bpy.context.view_layer.update()


def weight_head_lids(head):
    # Facial skin stays on the skull; independent lids move behind its openings.
    for group in list(head.vertex_groups): head.vertex_groups.remove(group)
    group=head.vertex_groups.new(name='head')
    group.add(list(range(len(head.data.vertices))),1,'REPLACE')
    for poly in head.data.polygons: poly.use_smooth=True
    bpy.context.view_layer.update()


def animate_lids(rig, frame):
    """Bake existing blink/sleep intent into lid rotation, leaving globes round."""
    for side in ['L','R']:
        closed=max(0,min(1,(1-rig.pose.bones['eye.'+side].scale.y)/.94))
        for part,angle in [('upper',UPPER_ANGLE),('lower',LOWER_ANGLE)]:
            bone=rig.pose.bones['lid_'+part+'.'+side]
            bone.rotation_mode='XYZ'
            bone.rotation_euler.x=closed*angle
            bone.keyframe_insert('rotation_euler',frame=frame,group=bone.name)
