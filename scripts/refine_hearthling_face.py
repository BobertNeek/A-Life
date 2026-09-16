"""Reference-driven facial sculpt applied to the retained revision-2 source.

Imported by export_hearthling.py inside Blender. The head contains rotating lid
bones while retaining the production clip names and inherited-scale contract.
"""
import math
import bpy
from mathutils import Vector, Matrix


def refine_face():
    rig = bpy.data.objects['Hearthling_Rig']
    # Surface projections and authored coordinates must share the rest pose.
    # Sampling an already posed head leaves floating seams after skinning.
    rig.data.pose_position = 'REST'
    bpy.context.view_layer.update()
    gold = bpy.data.materials['Hearthling | warm ochre']
    gold.diffuse_color = (.69,.245,.036,1)
    gold.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value = gold.diffuse_color
    brow_material = bpy.data.materials['Hearthling | brows and tuft shadows']
    brow_material.diffuse_color=(.22,.067,.020,1)
    brow_material.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value=brow_material.diffuse_color
    cream = bpy.data.materials['Hearthling | cream']
    cream.diffuse_color = (.94, .84, .66, 1)
    cream.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value = cream.diffuse_color
    dark = bpy.data.materials['Hearthling | mouth and nose']
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

    # Preserve the coat and neck transition below the replacement head.
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
            lower=(x/.37)**2+((z-1.885)/.20)**2
            eye=min(((x-s*.192)/.18)**2+((z-2.069)/.215)**2 for s in [-1,1])
            field=min(lower,eye)
            weight=max(0,min(1,(1.30-field)/.55))*max(0,min(1,(-y-.12)/.12))
            original=colors.data[v.index].color
            colors.data[v.index].color=tuple(original[i]*(1-weight)+cream.diffuse_color[i]*weight for i in range(4))

    from hearthling_head import head_y
    from hearthling_eyes import EYE_Y,EYE_Z,RADIUS
    def muzzle_y(x,z):
        return head_y(x,z,EYE_Y,EYE_Z,RADIUS)-.002

    # Small rounded triangular animal nose, not a sharp upside-down pyramid.
    nose_y=muzzle_y(0,1.946)-.013
    nose=ellipsoid('Hearthling_Nose',(0,nose_y,1.946),(.064,.035,.032),dark,segments=20,rings=12)
    for v in nose.data.vertices:
        factor=.52+.48*max(0,min(1,(v.co.z-1.913)/.066))
        v.co.x *= factor
    lip=[]
    for i in range(21):
        u=-1+2*i/20; x=u*.125; z=1.880+.022*u*u-.009*math.sin(math.pi*abs(u))
        lip.append((x,muzzle_y(x,z),z))
    tube('Hearthling_ClosedSmile',lip,[.0018+.0027*math.sin(math.pi*i/20) for i in range(21)],dark)
    tube('Hearthling_NoseToLip',[(0,nose_y-.025,1.923),(0,muzzle_y(0,1.906),1.906),(0,muzzle_y(0,1.880),1.880)], [.0025,.0025,.002],dark)

    # Eye pivots follow the new compact facial layout; existing blink scales remain valid.
    bpy.ops.object.select_all(action='DESELECT'); rig.select_set(True); bpy.context.view_layer.objects.active=rig
    bpy.ops.object.mode_set(mode='EDIT')
    for sign,side in [(-1,'L'),(1,'R')]:
        bone=rig.data.edit_bones['eye.'+side]
        delta=Vector((sign*.192,-.306,2.079))-bone.head
        bone.head+=delta; bone.tail+=delta
        ear=rig.data.edit_bones['ear.'+side]
        pivot=ear.head.copy()
        rotation=(Matrix.Translation(pivot) @ Matrix.Rotation(-sign*math.radians(42),4,'Y')
                  @ Matrix.Scale(.93,4) @ Matrix.Translation(-pivot))
        ear.transform(rotation)
        bpy.data.objects['Hearthling_Ear_'+side].data.transform(rotation)
    bpy.ops.object.mode_set(mode='OBJECT')

    from hearthling_eyes import build_eyes
    build_eyes(rig,body,mesh,tube,cream,dark)
    from hearthling_head import head_y
    from hearthling_eyes import EYE_Y,EYE_Z,RADIUS
    def surface_y(x,z):
        return head_y(x,z,EYE_Y,EYE_Z,RADIUS)
    for sign,side in [(-1,'L'),(1,'R')]:
        x=sign*.192
        # Low brow follows the forehead instead of hovering over it.
        points=[(x+u*.108,surface_y(x+u*.108,2.238+.045*(1-u*u))-.003,2.238+.045*(1-u*u)) for u in [-1,-.75,-.5,-.25,0,.25,.5,.75,1]]
        tube('Hearthling_Brow_'+side,points,[.001,.009,.015,.018,.020,.018,.013,.007,.001],brow_material)
    from hearthling_reference_sculpt import refine_reference_anatomy
    refine_reference_anatomy(rig,body,mesh)
    rig['face_revision']=6
    rig.data.pose_position = 'POSE'
    bpy.context.view_layer.update()
