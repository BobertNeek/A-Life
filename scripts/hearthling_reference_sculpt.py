"""Bipedal fox proportions, sculpted fur masses, and a skinned bushy tail.

All geometry is authored in the retained source rig's rest coordinates, then
the same proportion map is applied to the mesh and skeleton before animation.
Fur is solid geometry with vertex colors, so the GLB needs no hair renderer.
"""
import math
import bpy
import bmesh
from mathutils import Vector
from hearthling_head import head_y
from hearthling_eyes import EYE_Y, EYE_Z, RADIUS

GOLD=(.69,.245,.036,1)
CREAM=(.94,.84,.66,1)
RUST=(.38,.102,.018,1)
BROWN=(.115,.034,.014,1)


def mix(a,b,t):
    return tuple(x*(1-t)+y*t for x,y in zip(a,b))


def clamp(t):
    return max(0,min(1,t))


def refine_reference_anatomy(rig,body,mesh):
    coat=body.data.materials[0]
    # Shared material and one mesh for the clumps keeps draw calls bounded.
    verts=[]; faces=[]; colors=[]; weights=[]

    def lock(a,b,c,width,depth,color,bone='head',normal=(0,-1,0)):
        """Swept flattened fur lock, with its broad root buried in the coat."""
        a,b,c=map(Vector,(a,b,c)); normal=Vector(normal).normalized()
        base=len(verts); sides=8
        profile=[.65,.94,1,.76,.36,.015]
        across=(b-a).normalized().cross(normal).normalized()
        for i,t in enumerate([0,.18,.4,.65,.86,1]):
            p=(1-t)**2*a+2*(1-t)*t*b+t*t*c
            axis=(2*(1-t)*(b-a)+2*t*(c-b)).normalized()
            # Parallel transport avoids a frame flip at the crest of a curl.
            across=(across-axis*across.dot(axis)).normalized()
            out=across.cross(axis).normalized()
            for j in range(sides):
                angle=2*math.pi*j/sides
                q=p+across*math.cos(angle)*width*profile[i]
                q+=out*math.sin(angle)*depth*profile[i]
                verts.append(tuple(q))
                # Broad warm root shading survives GLB export without nodes.
                colors.append(mix(tuple(k*.78 for k in color[:3])+(1,),color,.4+.6*t))
                weights.append({bone:1} if isinstance(bone,str) else bone.copy())
        for i in range(5):
            for j in range(sides):
                n=base+i*sides+j; m=base+i*sides+(j+1)%sides
                faces.append((n,m,m+sides,n+sides))
        faces.append(tuple(base+j for j in reversed(range(sides))))
        faces.append(tuple(base+40+j for j in range(sides)))

    # Layered cheek fans: roots follow the face, points extend the silhouette.
    for sign in [-1,1]:
        for i,(x,z,bx,bz,cx,cz) in enumerate([
            (.29,2.065,.44,2.09,.55,2.11),
            (.255,2.005,.44,2.025,.555,2.035),
            (.25,1.965,.46,1.95,.558,1.935),
            (.22,1.92,.42,1.85,.50,1.80),
            (.17,1.875,.335,1.795,.39,1.735),
        ]):
            a=(sign*x,head_y(sign*x,z,EYE_Y,EYE_Z,RADIUS)+.035,z)
            b=(sign*bx,-.11,bz)
            c=(sign*cx,.006,cz)
            lock(a,b,c,.078,.037,GOLD if i==0 else CREAM)
        # Rear golden cheek layers provide depth in side and rear views.
        for i in range(3):
            lock((sign*.36,.075,2.015-i*.078),
                 (sign*.46,.105,1.96-i*.075),
                 (sign*(.51-.018*i),.13,1.91-i*.075),.085,.036,RUST if i==2 else GOLD)

    # Staggered broad curls, with smaller layers continuing over the crown.
    for a,b,c,w in [
        ((-.20,.055,2.26),(-.19,-.09,2.52),(-.09,-.278,2.245),.098),
        ((-.05,.10,2.33),(.08,-.06,2.53),(.04,-.295,2.235),.103),
        ((.11,.105,2.29),(.26,-.08,2.43),(.16,-.26,2.235),.09),
        ((-.29,.03,2.26),(-.31,-.02,2.39),(-.20,-.235,2.29),.07),
        ((.23,.06,2.245),(.34,-.035,2.34),(.275,-.16,2.26),.065),
        ((-.15,.22,2.23),(-.24,.13,2.49),(-.06,-.02,2.47),.085),
        ((.03,.235,2.22),(.10,.17,2.49),(.17,.015,2.46),.082),
    ]:
        lock(a,b,c,w,.046,GOLD)

    # White ear furnishings lie in front of the pink membrane and follow ears.
    for side,sign in [('L',-1),('R',1)]:
        ear=rig.data.bones['ear.'+side]
        root=ear.head_local.copy(); axis=(ear.tail_local-root).normalized()
        across=Vector((-axis.z,0,axis.x))
        for i in range(5):
            p=root+axis*(.06+i*.052)
            p.y=.008
            a=p+across*(-.012*sign)
            b=p+axis*.035+across*(sign*.060); b.y=-.027
            c=p+axis*.060+across*(sign*(.105-i*.012)); c.y=-.025
            lock(a,b,c,.031,.019,CREAM,'ear.'+side)

    # Soft ruff covers the neck with descending overlapping cream fur.
    def body_y(x,z):
        hit,p,*_=body.ray_cast(Vector((x,-2,z)),Vector((0,1,0)))
        return p.y if hit else -.11

    for row in range(3):
        for col in range(5-row):
            x=(col-(4-row)/2)*.074
            z=1.755-row*.145+.018*math.sin(col*2+row)
            y=body_y(x,z)
            lock((x,y+.035,z),(x*1.06,body_y(x,z-.095)-.025,z-.095),
                 (x*1.10,body_y(x,z-.21)-.032,z-.21),.065,.022,CREAM,
                 {'head':.65,'chest':.35} if row==0 else 'chest')
    for sign in [-1,1]:
        for i in range(4):
            lock((sign*(.105+i*.032),.07,1.74-i*.085),
                 (sign*(.24+i*.025),.025,1.69-i*.09),
                 (sign*(.295+i*.026),.035,1.59-i*.088),
                 .075,.037,GOLD,'chest')
        # One shoulder and one outside-thigh lock echo the reference coat.
        lock((sign*.29,.01,1.40),(sign*.385,-.045,1.32),
             (sign*.40,-.05,1.20),.080,.036,GOLD,'upper_arm.'+('R' if sign>0 else 'L'))
        lock((sign*.31,.08,.76),(sign*.39,.03,.63),
             (sign*.355,-.005,.48),.086,.039,GOLD,'thigh.'+('R' if sign>0 else 'L'))

    for row in range(4):
        for col in range(5-row):
            x=(col-(4-row)/2)*.073
            z=1.34-row*.137+.02*math.sin(col*2.1+row)
            lock((x,body_y(x,z)+.025,z),
                 (x*1.025,body_y(x,z-.09)-.035,z-.09),
                 (x*.94,body_y(x*.94,z-.20)-.022,z-.20),
                 .058,.025,CREAM,'spine' if row<2 else 'pelvis')

    # Replace the lion-like tail with a broad curved fox brush on the same bones.
    for name in ['Hearthling_Tail','Hearthling_TailTuft']:
        bpy.data.objects.remove(bpy.data.objects[name],do_unlink=True)
    spine=[Vector(p) for p in [(0,.21,.71),(.24,.48,.68),(.57,.64,.89),
                              (.79,.64,1.22),(.86,.60,1.53),(.74,.56,1.79)]]
    bpy.ops.object.select_all(action='DESELECT'); rig.select_set(True)
    bpy.context.view_layer.objects.active=rig; bpy.ops.object.mode_set(mode='EDIT')
    for i in range(5):
        bone=rig.data.edit_bones[f'tail.{i:02d}']
        bone.head=spine[i]; bone.tail=spine[i+1]
    bpy.ops.object.mode_set(mode='OBJECT')

    def tail_at(t):
        u=clamp(t)*5; i=min(4,int(u)); f=u-i
        p0=spine[max(0,i-1)]; p1=spine[i]
        p2=spine[i+1]; p3=spine[min(5,i+2)]
        return .5*((2*p1)+(-p0+p2)*f+(2*p0-5*p1+4*p2-p3)*f*f+(-p0+3*p1-3*p2+p3)*f*f*f)

    def tail_weights(t):
        f=max(0,min(4,t*5-.5)); i=int(f); s=f-i
        return {f'tail.{i:02d}':1-s,f'tail.{min(4,i+1):02d}':s} if i<4 else {'tail.04':1}

    tv=[]; tf=[]; tc=[]; tw=[]; rings=29; sides=16
    for i in range(rings):
        t=i/(rings-1); p=tail_at(t)
        axis=(tail_at(min(1,t+.005))-tail_at(max(0,t-.005))).normalized()
        a=axis.cross(Vector((0,1,0))).normalized(); b=axis.cross(a).normalized()
        r=(.066*(1-t)+.31*math.sin(math.pi*t)**.8)*max(.015,min(1,(1-t)*10))
        for j in range(sides):
            angle=2*math.pi*j/sides
            radius=r*(1+.055*math.cos(5*angle+5*t))
            tv.append(tuple(p+radius*(a*math.cos(angle)+b*math.sin(angle)*.77)))
            cream=clamp((t-.61+.025*math.sin(angle*7))/.045)
            tc.append(mix(mix(RUST,GOLD,.65+.30*math.cos(angle)),CREAM,cream))
            tw.append(tail_weights(t))
        if i:
            for j in range(sides):
                n=i*sides+j; m=i*sides+(j+1)%sides
                tf.append((n-sides,m-sides,m,n))
    tf.extend([tuple(reversed(range(sides))),tuple(range((rings-1)*sides,rings*sides))])
    tail=mesh('Hearthling_Tail',tv,tf,coat,'tail.00')
    _color_and_bind(tail,tc,tw)
    for t in [.30,.43,.56,.70,.81]:
        p=tail_at(t); axis=(tail_at(t+.015)-tail_at(t-.015)).normalized()
        across=axis.cross(Vector((0,1,0))).normalized()
        r=.066*(1-t)+.31*math.sin(math.pi*t)**.8
        for sign in [-1,1]:
            normal=across*sign
            lock(p+normal*r*.72,p+normal*(r+.043)+axis*.055,
                 p+normal*(r+.04)+axis*.19,.067,.03,CREAM if t>.60 else GOLD,
                 tail_weights(t),normal=normal)

    # Front-facing tail layers keep the brush from reading as an inflated tube.
    for row,t in enumerate([.28,.43,.58,.73,.84]):
        p=tail_at(t); axis=(tail_at(t+.015)-tail_at(t-.015)).normalized()
        across=axis.cross(Vector((0,1,0))).normalized()
        r=.066*(1-t)+.31*math.sin(math.pi*t)**.8
        for col in [-1,0,1]:
            root=p+across*r*col*.43+Vector((0,-r*.70,0))
            mid=root+axis*.080+Vector((0,-.040,0))
            tip=root+axis*.225+across*.015*math.sin(row+col)
            lock(root,mid,tip,.079,.036,CREAM if t>.59 else GOLD,tail_weights(t))

    fur=mesh('Hearthling_SculptedFur',verts,faces,coat)
    _color_and_bind(fur,colors,weights)
    # Round the authored control cages, then retain only the useful silhouette.
    # Skin weights and vertex colors interpolate through both modifiers.
    bpy.ops.object.select_all(action='DESELECT'); fur.select_set(True)
    bpy.context.view_layer.objects.active=fur
    smooth=fur.modifiers.new('Rounded sculpted fur','SUBSURF'); smooth.levels=2
    bpy.ops.object.modifier_move_up(modifier=smooth.name)
    bpy.ops.object.modifier_apply(modifier=smooth.name)
    fur.data.calc_loop_triangles()
    simplify=fur.modifiers.new('Fur triangle budget','DECIMATE')
    simplify.ratio=min(1,7400/len(fur.data.loop_triangles))
    bpy.ops.object.modifier_move_up(modifier=simplify.name)
    bpy.ops.object.modifier_apply(modifier=simplify.name)
    for poly in fur.data.polygons: poly.use_smooth=True

    # Warm orange coat, ivory belly and socks, dark fingertip/toe accents.
    colors=body.data.color_attributes['CoatColor']
    for v in body.data.vertices:
        x,y,z=v.co
        belly=clamp((1.1-(x/.255)**2-((z-1.015)/.44)**2)/.3)*clamp((-y+.035)/.14)
        sock=clamp((.24-z)/.09)
        c=mix(GOLD,CREAM,max(belly,sock))
        toes=clamp((-y-.225)/.06)*clamp((.155-z)/.07)
        c=mix(c,BROWN,toes*.94)
        for side in ['L','R']:
            group=body.vertex_groups['hand.'+side].index
            weight=sum(g.weight for g in v.groups if g.group==group)
            c=mix(c,BROWN,weight*clamp((1.0-z)/.07))
        colors.data[v.index].color=c
    for ob in bpy.context.scene.objects:
        if ob.type=='MESH' and ob.name.startswith(('Hearthling_Finger','Hearthling_Thumb')):
            ob.data.materials.clear(); ob.data.materials.append(coat)
            attr=ob.data.color_attributes.get('CoatColor') or ob.data.color_attributes.new(name='CoatColor',type='FLOAT_COLOR',domain='POINT')
            for item in attr.data: item.color=BROWN

    for side in ['L','R']:
        ear=bpy.data.objects['Hearthling_Ear_'+side]
        bone=rig.data.bones['ear.'+side]
        axis=(bone.tail_local-bone.head_local).normalized()
        sums=[Vector((0,0,0,0)) for _ in ear.data.vertices]
        counts=[0]*len(sums)
        for poly in ear.data.polygons:
            color=Vector(ear.data.materials[poly.material_index].diffuse_color)
            for index in poly.vertices: sums[index]+=color; counts[index]+=1
        attr=ear.data.color_attributes.get('CoatColor') or ear.data.color_attributes.new(name='CoatColor',type='FLOAT_COLOR',domain='POINT')
        for v in ear.data.vertices:
            t=(v.co-bone.head_local).dot(axis)/bone.length
            color=tuple(sums[v.index]/max(1,counts[v.index]))
            attr.data[v.index].color=mix(color,BROWN,clamp((t-.67)/.30))
        ear.data.materials.clear(); ear.data.materials.append(coat)
        for poly in ear.data.polygons: poly.material_index=0

    from hearthling_fur_fusion import fuse_coat
    fuse_coat(rig,body,fur)

    # Broaden the original paw geometry around each wrist/ankle before scaling.
    for ob in bpy.context.scene.objects:
        if ob.type!='MESH' or not ob.name.startswith('Hearthling'): continue
        if ob==body or ob.name.startswith(('Hearthling_Finger','Hearthling_Thumb')):
            for v in ob.data.vertices:
                for side in ['L','R']:
                    name='hand.'+side
                    if name in ob.vertex_groups:
                        group=ob.vertex_groups[name].index
                        w=sum(g.weight for g in v.groups if g.group==group)
                        center=rig.data.bones[name].head_local
                        v.co=center+(v.co-center)*(1+.16*w)
                if ob==body and v.co.z<.32:
                    w=clamp((.32-v.co.z)/.18)
                    cx=.25 if v.co.x>0 else -.25
                    v.co.x=cx+(v.co.x-cx)*(1+.23*w)
        for v in ob.data.vertices: v.co=_proportions(v.co)
        ob.data.update()

    bpy.ops.object.select_all(action='DESELECT'); rig.select_set(True)
    bpy.context.view_layer.objects.active=rig; bpy.ops.object.mode_set(mode='EDIT')
    for bone in rig.data.edit_bones:
        bone.head=_proportions(bone.head); bone.tail=_proportions(bone.tail)
    bpy.ops.object.mode_set(mode='OBJECT')


def _proportions(p):
    x,y,z=p
    if z<=1.5:
        nz=.78*z; scale=1.06
    elif z>=1.75:
        nz=1.31+1.13*(z-1.75); scale=1.13
    else:
        t=(z-1.5)/.25
        nz=(2*t**3-3*t*t+1)*1.17+(t**3-2*t*t+t)*.25*.78
        nz+=(-2*t**3+3*t*t)*1.31+(t**3-t*t)*.25*1.13
        scale=1.06+.07*t*t*(3-2*t)
    return Vector((x*scale,y*scale,nz))


def _color_and_bind(ob,colors,weights):
    for group in list(ob.vertex_groups): ob.vertex_groups.remove(group)
    for name in sorted({name for row in weights for name in row}): ob.vertex_groups.new(name=name)
    attr=ob.data.color_attributes.new(name='CoatColor',type='FLOAT_COLOR',domain='POINT')
    for i,(color,row) in enumerate(zip(colors,weights)):
        attr.data[i].color=color
        for name,w in row.items():
            if w>0: ob.vertex_groups[name].add([i],w,'REPLACE')
    bm=bmesh.new(); bm.from_mesh(ob.data)
    bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
    bm.to_mesh(ob.data); bm.free(); ob.data.update()
    for poly in ob.data.polygons: poly.use_smooth=True
