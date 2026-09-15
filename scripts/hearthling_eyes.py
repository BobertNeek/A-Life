"""Recessed spherical eyes and rotating eyelids; Blender authoring only."""
import math
import bpy
import bmesh


EYE_Y = -.170
EYE_Z = 2.075
RADIUS = .133
UPPER_ANGLE = .68
LOWER_ANGLE = -.53


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
            bone.head = (sign*.192, EYE_Y, EYE_Z)
            bone.tail = (sign*.192, EYE_Y, EYE_Z+.1)
            bone.parent = rig.data.edit_bones['head']
    bpy.ops.object.mode_set(mode='OBJECT')

    from hearthling_head import make_head, head_y
    head=make_head(body,mesh,cream,EYE_Y,EYE_Z,RADIUS)
    for sign, side in [(-1, 'L'), (1, 'R')]:
        x=sign*.192
        verts = []; faces = []; colors = []
        pupil_angle = math.asin(.050/RADIUS)
        iris_angle = math.asin(.086/RADIUS)
        # Extra rings at material boundaries keep a crisp limbal ring and pupil.
        angles = [0, .12, .24, pupil_angle-.008, pupil_angle+.008,
                  .48, .55, iris_angle-.025, iris_angle, iris_angle+.014,
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

        # A restrained upper lid edge; both ends stay attached to the corners.
        points=[]; ws=[]
        for i in range(33):
            a=math.pi*i/32; dx=.112*math.cos(a); dz=.077*math.sin(a)
            dy=head_y(x+dx,EYE_Z+dz,EYE_Y,EYE_Z,RADIUS)-EYE_Y-.001
            points.append((x+dx,EYE_Y+dy,EYE_Z+dz))
            ws.append(min(1,math.atan2(dz,-dy)/UPPER_ANGLE))
        lash=tube('Hearthling_LidEdge_'+side,points,[.0015+.002*math.sin(math.pi*i/32) for i in range(33)],dark,sides=8)
        group=lash.vertex_groups.new(name='lid_upper.'+side)
        for i,w in enumerate(ws):
            ids=list(range(i*8,(i+1)*8))
            lash.vertex_groups['head'].add(ids,1-w,'REPLACE')
            group.add(ids,w,'REPLACE')
    # Blend the lid rotations into the surrounding continuous head surface.
    for side in ['L','R']:
        for part in ['upper','lower']:
            name='lid_'+part+'.'+side
            if name not in head.vertex_groups: head.vertex_groups.new(name=name)
    for v in head.data.vertices:
        if v.co.z<=1.79: continue
        for group in head.vertex_groups: group.remove([v.index])
        influence=0
        for sign,side in [(-1,'L'),(1,'R')]:
            dx=v.co.x-sign*.192; dz=v.co.z-EYE_Z
            height=.077 if dz>=0 else .061
            radius=math.sqrt((dx/.112)**2+(dz/height)**2)
            radial_weight=max(0,min(1,1-(radius-1)/.65))**1.5
            depth_weight=max(0,min(1,(EYE_Y-v.co.y)/.06))
            part='upper' if dz>=0 else 'lower'
            angle=UPPER_ANGLE if dz>=0 else -LOWER_ANGLE
            phi=abs(math.atan2(dz,max(.001,EYE_Y-v.co.y)))
            w=min(1,phi/angle)*radial_weight*depth_weight
            if w>0:
                head.vertex_groups['lid_'+part+'.'+side].add([v.index],w,'REPLACE')
                influence+=w
        head.vertex_groups['head'].add([v.index],max(0,1-influence),'REPLACE')
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
