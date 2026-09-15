"""Reference-driven facial sculpt applied to the retained revision-2 source.

Imported by export_hearthling.py inside Blender. All added geometry uses the
existing head/eye bones, so the production clip and inherited-scale contracts stay intact.
"""
import math
import bpy
from mathutils import Vector, Matrix
from mathutils.kdtree import KDTree


def refine_face():
    rig = bpy.data.objects['Hearthling_Rig']
    # Surface projections and authored coordinates must share the rest pose.
    # Sampling an already posed head leaves floating seams after skinning.
    rig.data.pose_position = 'REST'
    bpy.context.view_layer.update()
    gold = bpy.data.materials['Hearthling | warm ochre']
    brow_material = bpy.data.materials['Hearthling | brows and tuft shadows']
    cream = bpy.data.materials['Hearthling | cream']
    cream.diffuse_color = (.93, .80, .56, 1)
    cream.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value = cream.diffuse_color
    dark = bpy.data.materials['Hearthling | mouth and nose']
    white = bpy.data.materials['Hearthling | warm eye whites']
    pupil = bpy.data.materials['Hearthling | pupil']
    glint = bpy.data.materials['Hearthling | catchlight']
    iris = bpy.data.materials['Hearthling | amber iris']
    iris.diffuse_color = (.18, .29, .055, 1)
    iris.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value = iris.diffuse_color
    iris.node_tree.nodes.get('Principled BSDF').inputs['Roughness'].default_value = .26

    prefixes = ('Hearthling_Muzzle','Hearthling_Nose','Hearthling_Eye','Hearthling_Iris',
                'Hearthling_Pupil','Hearthling_UpperLid','Hearthling_Brow','Hearthling_Tuft_',
                'Hearthling_OpenSmile','Hearthling_LowerLip','Hearthling_Tongue')
    for ob in list(bpy.context.scene.objects):
        if ob.type == 'MESH' and ob.name.startswith(prefixes):
            bpy.data.objects.remove(ob, do_unlink=True)

    def bind(ob, bone='head'):
        group = ob.vertex_groups.new(name=bone)
        group.add(list(range(len(ob.data.vertices))), 1, 'REPLACE')
        modifier = ob.modifiers.new('Expressive skeleton', 'ARMATURE')
        modifier.object = rig
        ob.parent = rig
        for face in ob.data.polygons:
            face.use_smooth = True
        return ob

    def mesh(name, verts, faces, material, bone='head'):
        data=bpy.data.meshes.new(name)
        data.from_pydata(verts, [], faces); data.update()
        ob=bpy.data.objects.new(name,data); bpy.context.collection.objects.link(ob)
        data.materials.append(material)
        return bind(ob,bone)

    def ellipsoid(name, center, scale, material, bone='head', segments=24, rings=14):
        bpy.ops.mesh.primitive_uv_sphere_add(segments=segments, ring_count=rings)
        ob=bpy.context.object; ob.name=name
        ob.data.transform(Matrix.Translation(center) @ Matrix.Diagonal((*scale,1)))
        ob.data.materials.append(material)
        return bind(ob,bone)

    def tube(name, points, radii, material, bone='head', sides=8):
        points=[Vector(p) for p in points]; verts=[]; faces=[]
        for i,p in enumerate(points):
            axis=(points[min(i+1,len(points)-1)]-points[max(0,i-1)]).normalized()
            a=axis.cross(Vector((0,1,0))).normalized()
            if a.length < .1: a=axis.cross(Vector((1,0,0))).normalized()
            b=axis.cross(a).normalized()
            for j in range(sides):
                angle=j*2*math.pi/sides
                verts.append(tuple(p+radii[i]*(math.cos(angle)*a+math.sin(angle)*b)))
        for i in range(len(points)-1):
            for j in range(sides):
                a=i*sides+j; b=i*sides+(j+1)%sides
                faces.append((a,b,b+sides,a+sides))
        faces += [tuple(reversed(range(sides))),tuple(range(len(verts)-sides,len(verts)))]
        return mesh(name,verts,faces,material,bone)

    # Broader cheek contour and a continuous cream mask on the connected sculpt.
    body=bpy.data.objects['Hearthling_Body']
    colors=body.data.color_attributes['CoatColor']
    for v in body.data.vertices:
        x,y,z=v.co
        if z < 1.64: continue
        if z > 1.79:
            for group in body.vertex_groups: group.remove([v.index])
            body.vertex_groups['head'].add([v.index],1,'REPLACE')
        cheek=math.exp(-((z-1.94)/.22)**2)
        v.co.x *= (1+.09*cheek)*(1-.16*math.exp(-((z-1.73)/.13)**2))
        if y < -.12:
            # Recess the eye centers; surrounding geometry forms the socket.
            socket=sum(math.exp(-((x-s*.192)/.12)**2-((z-2.079)/.145)**2) for s in [-1,1])
            v.co.y += .035*socket
            lower=(x/.37)**2+((z-1.885)/.20)**2
            eye=min(((x-s*.192)/.18)**2+((z-2.069)/.215)**2 for s in [-1,1])
            field=min(lower,eye)
            weight=max(0,min(1,(1.30-field)/.55))*max(0,min(1,(-y-.12)/.12))
            original=colors.data[v.index].color
            colors.data[v.index].color=tuple(original[i]*(1-weight)+cream.diffuse_color[i]*weight for i in range(4))

    # A single blended muzzle replaces the old broad projecting smile plate.
    pads=[ellipsoid('Hearthling_Muzzle',(-.074,-.329,1.889),(.119,.080,.076),cream),
          ellipsoid('Hearthling_MuzzlePad',(.074,-.329,1.889),(.119,.080,.076),cream),
          ellipsoid('Hearthling_Chin',(0,-.307,1.838),(.173,.076,.073),cream)]
    bpy.ops.object.select_all(action='DESELECT')
    for ob in pads:
        ob.modifiers.clear(); ob.vertex_groups.clear(); ob.parent=None; ob.select_set(True)
    bpy.context.view_layer.objects.active=pads[0]
    bpy.ops.object.join(); muzzle=bpy.context.object
    remesh=muzzle.modifiers.new('Unified soft muzzle','REMESH'); remesh.mode='VOXEL'; remesh.voxel_size=.012
    bpy.ops.object.modifier_apply(modifier=remesh.name)
    smooth=muzzle.modifiers.new('Soft cheek transitions','SMOOTH'); smooth.factor=.7; smooth.iterations=5
    bpy.ops.object.modifier_apply(modifier=smooth.name)
    reduce=muzzle.modifiers.new('Game mesh budget','DECIMATE'); reduce.ratio=.55
    bpy.ops.object.modifier_apply(modifier=reduce.name)
    bind(muzzle)

    def muzzle_y(x,z):
        hit,p,_,_=muzzle.ray_cast(Vector((x,-2,z)),Vector((0,1,0)))
        return p.y-.003 if hit else -.383

    # Small rounded triangular animal nose, not a sharp upside-down pyramid.
    nose=ellipsoid('Hearthling_Nose',(0,-.418,1.946),(.054,.033,.033),dark,segments=20,rings=12)
    for v in nose.data.vertices:
        factor=.52+.48*max(0,min(1,(v.co.z-1.913)/.066))
        v.co.x *= factor
    lip=[]
    for i in range(21):
        u=-1+2*i/20; x=u*.112; z=1.861+.022*u*u
        lip.append((x,muzzle_y(x,z),z))
    tube('Hearthling_ClosedSmile',lip,[.0018+.0027*math.sin(math.pi*i/20) for i in range(21)],dark)
    tube('Hearthling_NoseToLip',[(0,-.441,1.922),(0,-.413,1.891),(0,muzzle_y(0,1.873),1.873),(0,muzzle_y(0,1.861),1.861)], [.003,.003,.003,.002],dark)

    # Eye pivots follow the new compact facial layout; existing blink scales remain valid.
    bpy.ops.object.select_all(action='DESELECT'); rig.select_set(True); bpy.context.view_layer.objects.active=rig
    bpy.ops.object.mode_set(mode='EDIT')
    for sign,side in [(-1,'L'),(1,'R')]:
        bone=rig.data.edit_bones['eye.'+side]
        delta=Vector((sign*.192,-.306,2.079))-bone.head
        bone.head+=delta; bone.tail+=delta
        ear=rig.data.edit_bones['ear.'+side]
        pivot=ear.head.copy()
        rotation=Matrix.Translation(pivot) @ Matrix.Rotation(-sign*math.radians(22),4,'Y') @ Matrix.Translation(-pivot)
        ear.transform(rotation)
        bpy.data.objects['Hearthling_Ear_'+side].data.transform(rotation)
    bpy.ops.object.mode_set(mode='OBJECT')

    body.data.update()
    tree=KDTree(len(body.data.vertices))
    for v in body.data.vertices: tree.insert(v.co,v.index)
    tree.balance()
    def coat_color(point):
        nearest=tree.find_n(Vector(point),4)
        weights=[1/max(distance,.0001)**2 for _,_,distance in nearest]
        return tuple(sum(colors.data[item[1]].color[c]*w for item,w in zip(nearest,weights))/sum(weights) for c in range(4))
    def surface_y(x,z):
        hit,p,_,_=body.ray_cast(Vector((x,-2,z)),Vector((0,1,0)))
        return p.y-.002 if hit else -.12

    for sign,side in [(-1,'L'),(1,'R')]:
        eye_bone='eye.'+side; x=sign*.192; z=2.079
        ellipsoid('Hearthling_EyeWhite_'+side,(x,-.279,z),(.117,.061,.127),white,eye_bone)
        iris_ob=ellipsoid('Hearthling_Iris_'+side,(x+.004,-.328,z+.002),(.086,.018,.098),iris,eye_bone,40,16)
        attr=iris_ob.data.color_attributes.new(name='IrisColor',type='FLOAT_COLOR',domain='POINT')
        for v in iris_ob.data.vertices:
            dx=(v.co.x-x-.004)/.086; dz=(v.co.z-z-.002)/.098
            radius=min(1,math.sqrt(dx*dx+dz*dz)); angle=math.atan2(dz,dx)
            ray=.92+.08*math.sin(angle*37)+.04*math.cos(angle*71)
            edge=max(0,min(1,(radius-.77)/.23))
            base_color=(.22,.34,.067); rim=(.035,.065,.012)
            attr.data[v.index].color=tuple((base_color[c]*(1-edge)+rim[c]*edge)*ray for c in range(3))+(1,)
        shader=iris.node_tree.nodes.get('Principled BSDF')
        color_node=iris.node_tree.nodes.get('Iris radial color')
        if color_node is None:
            color_node=iris.node_tree.nodes.new('ShaderNodeVertexColor'); color_node.name='Iris radial color'; color_node.layer_name='IrisColor'
            iris.node_tree.links.new(color_node.outputs['Color'],shader.inputs['Base Color'])
        ellipsoid('Hearthling_Pupil_'+side,(x+.004,-.342,z+.004),(.055,.009,.064),pupil,eye_bone)
        ellipsoid('Hearthling_EyeGlint_'+side,(x-.020,-.352,z+.043),(.012,.003,.014),glint,eye_bone,12,8)
        # A broad skin surface joins the eye opening to the face. Its inner edge
        # follows the blink bone while the outer edge stays attached to the head.
        # This closes an actual lid over the eye instead of flattening a loose rim.
        verts=[]; faces=[]; count=48; rings=8
        for r in range(rings):
            t=r/(rings-1)
            for i in range(count):
                a=2*math.pi*i/count
                px=x+( .105+.067*t)*math.cos(a)
                pz=z+( .115+.068*t)*math.sin(a)
                blend=t*t*(3-2*t)
                py=-.356*(1-blend)+(surface_y(px,pz)+.003)*blend
                verts.append((px,py,pz))
                if r<rings-1:
                    k=r*count+i; n=r*count+(i+1)%count
                    faces.append((k,k+count,n+count,n))
        lid=mesh('Hearthling_LidSurface_'+side,verts,faces,body.data.materials[0])
        lid_colors=lid.data.color_attributes.new(name='CoatColor',type='FLOAT_COLOR',domain='POINT')
        for i,point in enumerate(verts):
            t=(i//count)/(rings-1)
            sampled=coat_color(point)
            lid_colors.data[i].color=tuple(cream.diffuse_color[c]*(1-t)+sampled[c]*t for c in range(4))
        head_group=lid.vertex_groups['head']; eye_group=lid.vertex_groups.new(name=eye_bone)
        for r in range(rings):
            weight=(1-r/(rings-1))**1.3
            ids=list(range(r*count,(r+1)*count))
            head_group.add(ids,1-weight,'REPLACE'); eye_group.add(ids,weight,'REPLACE')
        points=[(x+.105*math.cos(i*math.pi/32),-.358,z+.115*math.sin(i*math.pi/32)) for i in range(33)]
        tube('Hearthling_Lash_'+side,points,[.002+.0025*math.sin(math.pi*i/32) for i in range(33)],dark,eye_bone,6)
        # Low brow follows the forehead instead of hovering over it.
        points=[(x+u*.108,surface_y(x+u*.108,2.246+.018*(1-u*u))-.002,2.246+.018*(1-u*u)) for u in [-1,-.75,-.5,-.25,0,.25,.5,.75,1]]
        tube('Hearthling_Brow_'+side,points,[.001,.007,.011,.014,.015,.013,.009,.005,.001],brow_material)
        tufts=[]
        for i in range(3):
            z0=1.97-i*.053
            tip_z=[2.035,1.935,1.82][i]
            points=[(sign*.30,-.08,z0),(sign*.39,-.10,z0),(sign*(.46-i*.008),-.08,(z0+tip_z)/2),(sign*(.53-i*.018),-.015,tip_z)]
            tufts.append(tube('Hearthling_CheekTuft_'+side+str(i),points,[.09,.081,.042,.001],cream,sides=16))
        bpy.ops.object.select_all(action='DESELECT')
        for ob in tufts:
            ob.modifiers.clear(); ob.vertex_groups.clear(); ob.parent=None; ob.select_set(True)
        bpy.context.view_layer.objects.active=tufts[0]
        bpy.ops.object.join(); cheek=bpy.context.object
        remesh=cheek.modifiers.new('Sculpted cheek fur','REMESH'); remesh.mode='VOXEL'; remesh.voxel_size=.010
        bpy.ops.object.modifier_apply(modifier=remesh.name)
        smooth=cheek.modifiers.new('Blend clump roots','SMOOTH'); smooth.factor=.8; smooth.iterations=6
        bpy.ops.object.modifier_apply(modifier=smooth.name)
        reduce=cheek.modifiers.new('Game mesh budget','DECIMATE'); reduce.ratio=.35
        bpy.ops.object.modifier_apply(modifier=reduce.name)
        bind(cheek)

    # Broad swept forelock with tapered ends; no cone-shaped forehead spikes.
    for i,(a,b,c,d) in enumerate([
        ((-.10,-.045,2.33),(-.075,-.13,2.39),(.065,-.20,2.37),(.10,-.26,2.29)),
        ((.01,-.055,2.32),(.095,-.13,2.37),(.14,-.22,2.31),(.08,-.285,2.25)),
        ((-.13,-.095,2.31),(-.16,-.17,2.36),(-.11,-.25,2.30),(-.14,-.285,2.25)),
    ]):
        # Cubic Bezier gives a continuous curl and broad, soft volume.
        points=[]; radii=[]
        for j in range(13):
            t=j/12
            points.append(tuple((1-t)**3*Vector(a)+3*(1-t)**2*t*Vector(b)+3*(1-t)*t*t*Vector(c)+t**3*Vector(d)))
            radii.append(.068*(1-t)**.65+.001)
        tube('Hearthling_Tuft_'+str(i),points,radii,gold,sides=12)
    rig['face_revision']=3
    rig.data.pose_position = 'POSE'
    bpy.context.view_layer.update()
