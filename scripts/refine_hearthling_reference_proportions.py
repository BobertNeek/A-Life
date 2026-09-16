"""Reference landmark pass on the finished Hearthling sculpt, Blender 5.2 LTS."""
import math
import bpy
import bmesh
from mathutils import Vector,Matrix


def refine_reference_proportions():
    rig=bpy.data.objects['Hearthling_Rig']
    if rig.get('reference_landmarks_revision',0)>=7:
        raise RuntimeError('Reference proportions already applied to this checkpoint')
    previous=rig.data.pose_position; rig.data.pose_position='REST'
    bpy.context.view_layer.update()
    head=bpy.data.objects['Hearthling_Head']
    before=(min(v.co.x for v in head.data.vertices),max(v.co.x for v in head.data.vertices))
    for v in head.data.vertices:
        x,y,z=v.co
        # Keep the eye apertures and eye spacing intact while reducing the wings.
        if abs(x)>.39: v.co.x=math.copysign(.39+.52*(abs(x)-.39),x)
        if y>.04: v.co.y=.04+(y-.04)*.67
        if z>1.98 and y>.035:
            v.co.z-=.060*_smooth((z-1.98)/.12)*_smooth((y-.035)/.1)

    # Wider, shorter ear triangles with the attachment point held fixed.
    ear_maps={}
    for side in ['L','R']:
        bone=rig.data.bones['ear.'+side]
        pivot=bone.head_local.copy(); axis=(bone.tail_local-pivot).normalized()
        across=Vector((-axis.z,0,axis.x)).normalized(); depth=axis.cross(across).normalized()
        basis=Matrix((axis,across,depth)).transposed()
        linear=basis @ Matrix.Diagonal((.86,1.15,1.18)) @ basis.transposed()
        ear_maps[side]=(pivot,linear)
        for v in bpy.data.objects['Hearthling_Ear_'+side].data.vertices:
            v.co=pivot+linear@(v.co-pivot)

    body=bpy.data.objects['Hearthling_Body']
    for ob in bpy.context.scene.objects:
        if ob.type!='MESH' or not ob.name.startswith('Hearthling'): continue
        if ob==body or ob.name.startswith(('Hearthling_Finger','Hearthling_Thumb')):
            for v in ob.data.vertices:
                for side in ['L','R']:
                    name='hand.'+side
                    if name not in ob.vertex_groups: continue
                    idx=ob.vertex_groups[name].index
                    w=sum(g.weight for g in v.groups if g.group==idx)
                    if w<=0: continue
                    bone=rig.data.bones[name]; axis=(bone.tail_local-bone.head_local).normalized()
                    offset=v.co-bone.head_local; along=axis*offset.dot(axis)
                    v.co=bone.head_local+along*(1-.18*w)+(offset-along)*(1+.12*w)
        if ob==body:
            color=body.data.color_attributes['CoatColor']
            for v in body.data.vertices:
                x,y,z=v.co
                if z<.18 and y<-.12:
                    foot=math.copysign(.265,x)
                    field=sum(math.exp(-((x-(foot+j*.078))/.038)**2-((y+.285)/.095)**2) for j in [-1,0,1])
                    v.co.z+=.030*field*_smooth(z/.065)*(1-_smooth((z-.135)/.045))
                    toe=max(math.exp(-((x-(foot+j*.078))/.04)**4-((y+.315)/.10)**4) for j in [-1,0,1])
                    w=toe*_smooth((.15-z)/.05)
                    current=color.data[v.index].color
                    color.data[v.index].color=tuple(current[i]*(1-w)+(.115,.034,.014,1)[i]*w for i in range(4))
        if ob.name=='Hearthling_Tail':
            bm=bmesh.new(); bm.from_mesh(ob.data)
            selected=[v for v in bm.verts if v.co.z>.60]
            for _ in range(7): bmesh.ops.smooth_vert(bm,verts=selected,factor=.38,use_axis_x=True,use_axis_y=True,use_axis_z=True)
            bm.to_mesh(ob.data); bm.free()
            for v in ob.data.vertices: v.co.z+=_tail_lift(v.co.z)
        for v in ob.data.vertices: v.co=_body_map(v.co)
        ob.data.update()

    bpy.ops.object.select_all(action='DESELECT'); rig.select_set(True)
    bpy.context.view_layer.objects.active=rig; bpy.ops.object.mode_set(mode='EDIT')
    for bone in rig.data.edit_bones:
        if bone.name.startswith('ear.'):
            pivot,linear=ear_maps[bone.name[-1]]
            bone.head=pivot+linear@(bone.head-pivot); bone.tail=pivot+linear@(bone.tail-pivot)
        if bone.name.startswith('tail.'):
            bone.head.z+=_tail_lift(bone.head.z); bone.tail.z+=_tail_lift(bone.tail.z)
        bone.head=_body_map(bone.head); bone.tail=_body_map(bone.tail)
    bpy.ops.object.mode_set(mode='OBJECT')
    rig['reference_landmarks_revision']=7
    rig.data.pose_position=previous; bpy.context.view_layer.update()
    finish_reference_face()
    print({'reference_revision':7,'head_width_before':before[1]-before[0],
           'head_width_after':max(v.co.x for v in head.data.vertices)-min(v.co.x for v in head.data.vertices),
           'anatomy':'upright biped; two arms; two legs; shortened palms; rounded toes'})


def _smooth(t):
    t=max(0,min(1,t)); return t*t*(3-2*t)


def _body_map(p):
    x,y,z=p
    x*=.87+.13*_smooth((z-.62)/.28)
    z-=.070*_smooth((z-.62)/.55)
    return Vector((x,y,z))


def _tail_lift(z):
    return .30*_smooth((z-.56)/.86)


def finish_reference_face():
    rig=bpy.data.objects['Hearthling_Rig']
    if rig.get('reference_finish_revision',0)>=7: raise RuntimeError('Face finish already applied')
    previous=rig.data.pose_position; rig.data.pose_position='REST'
    for ob in bpy.context.scene.objects:
        if ob.type!='MESH' or not ob.name.startswith(('Hearthling_Head','Hearthling_Nose','Hearthling_ClosedSmile')): continue
        for v in ob.data.vertices:
            x,y,z=v.co
            if y<-.16:
                pads=sum(math.exp(-((x-s*.082)/.075)**2-((z-1.44)/.055)**2) for s in [-1,1])
                v.co.y-=.015*pads
        ob.data.update()
    body=bpy.data.objects['Hearthling_Body']
    for v in body.data.vertices:
        x,y,z=v.co
        w=math.exp(-((z-1.16)/.09)**2)*_smooth((.29-abs(x))/.13)
        v.co.x*=1+.20*w; v.co.y*=1+.10*w
    body.data.update()
    head=bpy.data.objects['Hearthling_Head']; coat=head.data.color_attributes['CoatColor']
    for v in head.data.vertices:
        x,y,z=v.co
        w=math.exp(-(x/.22)**4-((z-1.435)/.11)**4)*_smooth((-y-.15)/.20)*.35
        old=coat.data[v.index].color
        coat.data[v.index].color=tuple(old[i]*(1-w)+(1,.91,.76,1)[i]*w for i in range(4))
    # A thin orbital margin follows the fixed aperture; the curved lids close inside it.
    from hearthling_head import head_y,EYE_X,EYE_WIDTH,EYE_UPPER
    from hearthling_eyes import EYE_Y,EYE_Z,RADIUS
    dark=bpy.data.materials['Hearthling | brows and tuft shadows']
    for sign,side in [(-1,'L'),(1,'R')]:
        points=[]
        for j in range(19):
            angle=math.pi*j/18
            x=sign*EYE_X+EYE_WIDTH*1.022*math.cos(angle)
            z=EYE_Z+EYE_UPPER*1.022*math.sin(angle)
            y=head_y(x,z,EYE_Y,EYE_Z,RADIUS)-.004
            points.append(Vector((x*1.13,y*1.13,1.31+1.13*(z-1.75)-.07)))
        vs=[]; fs=[]; sides=6
        for i,p in enumerate(points):
            axis=(points[min(18,i+1)]-points[max(0,i-1)]).normalized()
            a=axis.cross(Vector((0,1,0))).normalized(); b=axis.cross(a)
            r=.001+.0035*math.sin(math.pi*i/18)
            for j in range(sides): vs.append(tuple(p+r*(a*math.cos(j*math.tau/sides)+b*math.sin(j*math.tau/sides))))
            if i:
                for j in range(sides):
                    n=i*sides+j; m=i*sides+(j+1)%sides
                    fs.append((n-sides,m-sides,m,n))
        fs.extend([tuple(reversed(range(sides))),tuple(range(len(vs)-sides,len(vs)))])
        data=bpy.data.meshes.new('Hearthling_OrbitalRim_'+side); data.from_pydata(vs,[],fs); data.materials.append(dark)
        ob=bpy.data.objects.new(data.name,data); bpy.context.scene.collection.objects.link(ob)
        review=bpy.data.collections.get('Hearthling_Character_Review')
        if review: review.objects.link(ob)
        group=ob.vertex_groups.new(name='head'); group.add(list(range(len(vs))),1,'REPLACE')
        mod=ob.modifiers.new('Head skin','ARMATURE'); mod.object=rig; ob.parent=rig
        for poly in data.polygons: poly.use_smooth=True
    deform={b.name for b in rig.data.bones if b.use_deform}
    normalized=0
    for ob in bpy.context.scene.objects:
        if ob.type!='MESH' or not ob.name.startswith('Hearthling'): continue
        for v in ob.data.vertices:
            row=[(g.group,g.weight) for g in v.groups if ob.vertex_groups[g.group].name in deform]
            total=sum(w for _,w in row)
            if total<=1e-8: raise RuntimeError(f'Unweighted vertex: {ob.name}:{v.index}')
            if abs(total-1)>1e-5:
                for group,w in row: ob.vertex_groups[group].add([v.index],w/total,'REPLACE')
                normalized+=1
    rig['reference_finish_revision']=7; rig.data.pose_position=previous
    bpy.context.view_layer.update()
    print({'reference_face_finish':7,'normalized_vertices':normalized})
