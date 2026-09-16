"""Continuous reference face for the revision-7 bipedal rig, Blender 5.2.

Replace only the head skin and its facial landmarks before baking the coat.
Eyes, rotating lids, body, inherited bones and production clips are retained.
"""
import math
import bpy
import bmesh
from mathutils import Vector, Matrix

EYE_X,EYE_Y,EYE_Z,EYE_R=.21696,-.1921,1.60725,.15029


def smooth(t):
    t=max(0.,min(1.,t)); return t*t*(3-2*t)


def refine_expression():
    rig=bpy.data.objects['Hearthling_Rig']
    assert rig.get('reference_finish_revision')==7
    assert not rig.get('expression_revision'), 'Open the unmodified checkpoint first'
    previous=rig.data.pose_position; rig.data.pose_position='REST'
    coat=bpy.data.materials['Hearthling | vertex-colored coat']
    dark=bpy.data.materials['Hearthling | mouth and nose']
    brow_mat=bpy.data.materials['Hearthling | brows and tuft shadows']
    collection=bpy.data.collections.get('Hearthling_Character') or bpy.data.collections.new('Hearthling_Character')
    if collection.name not in bpy.context.scene.collection.children: bpy.context.scene.collection.children.link(collection)
    removed=('Hearthling_Head','Hearthling_Nose','Hearthling_NoseToLip','Hearthling_ClosedSmile',
             'Hearthling_Brow_','Hearthling_OrbitalRim_')
    for ob in list(bpy.context.scene.objects):
        if ob.type=='MESH' and ob.name.startswith(removed): bpy.data.objects.remove(ob,do_unlink=True)

    def mesh(name,vs,fs,mat,bone=None):
        data=bpy.data.meshes.new(name); data.from_pydata(vs,[],fs); data.update()
        ob=bpy.data.objects.new(name,data); collection.objects.link(ob); data.materials.append(mat)
        if bone: bind(ob,bone)
        return ob

    def bind(ob,bone='head'):
        group=ob.vertex_groups.new(name=bone); group.add(list(range(len(ob.data.vertices))),1,'REPLACE')
        arm=ob.modifiers.new('Reference face skin','ARMATURE'); arm.object=rig; ob.parent=rig
        for p in ob.data.polygons: p.use_smooth=True

    def ellipsoid(name,center,scale,mat,segments=40,rings=24):
        bpy.ops.mesh.primitive_uv_sphere_add(segments=segments,ring_count=rings)
        ob=bpy.context.object; ob.name=name
        ob.data.transform(Matrix.Translation(center)@Matrix.Diagonal((*scale,1)))
        ob.data.materials.append(mat)
        return ob

    def tube(name,points,radii,mat,bone='head',sides=8):
        points=[Vector(p) for p in points]; vs=[]; fs=[]
        for i,p in enumerate(points):
            axis=(points[min(i+1,len(points)-1)]-points[max(0,i-1)]).normalized()
            a=axis.cross(Vector((0,1,0))).normalized(); b=axis.cross(a).normalized()
            for j in range(sides):
                angle=j*math.tau/sides
                vs.append(tuple(p+radii[i]*(a*math.cos(angle)+b*math.sin(angle))))
            if i:
                for j in range(sides):
                    n=i*sides+j; m=i*sides+(j+1)%sides; fs.append((n-sides,m-sides,m,n))
        fs.extend([tuple(reversed(range(sides))),tuple(range(len(vs)-sides,len(vs)))])
        return mesh(name,vs,fs,mat,bone)

    def fur_lock(a,b,c,width,depth):
        a,b,c=map(Vector,(a,b,c)); vs=[]; fs=[]; sides=12
        across=(b-a).normalized().cross(Vector((0,-1,0))).normalized()
        for i in range(9):
            t=i/8; p=(1-t)**2*a+2*(1-t)*t*b+t*t*c
            axis=((1-t)*(b-a)+t*(c-b)).normalized()
            # Transport the frame through the curl instead of flipping the
            # cross-section when its tangent passes through the front axis.
            across=(across-axis*across.dot(axis)).normalized(); out=across.cross(axis).normalized()
            radius=max(.005,(1-t)**.60)*(.80+.3*math.sin(t*math.pi))
            for j in range(sides):
                angle=j*math.tau/sides
                vs.append(tuple(p+radius*(across*width*math.cos(angle)+out*depth*math.sin(angle))))
            if i:
                for j in range(sides):
                    n=i*sides+j; m=i*sides+(j+1)%sides; fs.append((n-sides,m-sides,m,n))
        fs.extend([tuple(reversed(range(sides))),tuple(range(len(vs)-sides,len(vs)))])
        return mesh('FaceFur',vs,fs,coat)

    # Intersecting anatomical masses become one skin, rather than separate
    # cheeks/pads sitting on the skull. The muzzle is broad and short in profile.
    parts=[ellipsoid('FaceSkull',(0,.015,1.664),(.438,.31,.317),coat),
           ellipsoid('FaceJaw',(0,-.105,1.435),(.295,.23,.143),coat),
           ellipsoid('FaceChin',(0,-.245,1.363),(.179,.150,.091),coat),
           ellipsoid('FaceNeck',(0,.025,1.285),(.158,.157,.175),coat)]
    for s in [-1,1]:
        parts.append(ellipsoid('FaceCheek',(s*.263,-.102,1.493),(.166,.213,.153),coat))
        parts.append(ellipsoid('FaceWhiskerPad',(s*.107,-.290,1.457),(.184,.139,.109),coat))
        for i in range(3):
            z=1.535-i*.056
            parts.append(fur_lock((s*.36,.0,z),(s*.45,-.035,z-.003),
                                  (s*(.497-i*.012),-.008,z+.030),.043,.046))
    for a,b,c,w in [
        ((-.12,.07,1.916),(-.16,-.10,2.079),(-.10,-.235,1.932),.066),
        ((.015,.09,1.94),(.030,-.075,2.118),(.055,-.240,1.925),.073),
        ((.15,.075,1.908),(.21,-.08,2.038),(.175,-.225,1.914),.060),
        ((-.24,.09,1.864),(-.27,-.015,1.980),(-.215,-.18,1.907),.054),
    ]:
        parts.append(fur_lock(a,b,c,w,.030))
    bpy.ops.object.select_all(action='DESELECT')
    for ob in parts: ob.select_set(True)
    bpy.context.view_layer.objects.active=parts[0]; bpy.ops.object.join()
    head=bpy.context.object; head.name='Hearthling_Head'
    if head.name not in collection.objects: collection.objects.link(head)
    remesh=head.modifiers.new('Unified face anatomy','REMESH'); remesh.mode='VOXEL'; remesh.voxel_size=.0055
    bpy.ops.object.modifier_apply(modifier=remesh.name)
    bm=bmesh.new(); bm.from_mesh(head.data)
    for _ in range(18):
        bmesh.ops.smooth_vert(bm,verts=list(bm.verts),factor=.40,use_axis_x=True,use_axis_y=True,use_axis_z=True)
    bm.to_mesh(head.data); bm.free()
    # Smooth the bridge onto the globe curvature, then cut almond apertures.
    for v in head.data.vertices:
        x,y,z=v.co
        if y>-.07: continue
        for sign in [-1,1]:
            dx=x-sign*EYE_X; dz=z-EYE_Z; radial=dx*dx+dz*dz
            rho=math.sqrt((dx/.127)**2+(dz/.111)**2)
            if rho<1.85:
                w=1-smooth((rho-1.05)/.80)
                surface=EYE_Y-math.sqrt(max(.0002,EYE_R**2-radial))-.024
                v.co.y=min(v.co.y,v.co.y*(1-w)+surface*w)
    head.data.update()
    for sign in [-1,1]:
        sides=64; vs=[]
        for y in [-.9,-.060]:
            for i in range(sides):
                a=i*math.tau/sides; dx=.128*math.cos(a)
                dz=(.096 if math.sin(a)>0 else .082)*math.sin(a)+sign*.08*dx
                vs.append((sign*EYE_X+dx,y,EYE_Z+dz))
        fs=[(i,(i+1)%sides,(i+1)%sides+sides,i+sides) for i in range(sides)]
        fs.extend([tuple(reversed(range(sides))),tuple(range(sides,2*sides))])
        cutter=mesh('FaceAperture',vs,fs,coat)
        bm=bmesh.new(); bm.from_mesh(cutter.data); bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces)); bm.to_mesh(cutter.data); bm.free()
        boolean=head.modifiers.new('Eye aperture','BOOLEAN'); boolean.operation='DIFFERENCE'; boolean.solver='EXACT'; boolean.object=cutter
        bpy.context.view_layer.objects.active=head; bpy.ops.object.modifier_apply(modifier=boolean.name)
        bpy.data.objects.remove(cutter,do_unlink=True)
    head.data.calc_loop_triangles()
    dec=head.modifiers.new('Head triangle budget','DECIMATE'); dec.ratio=min(1,8600/len(head.data.loop_triangles))
    bpy.ops.object.modifier_apply(modifier=dec.name)
    bind(head)
    paint_reference_face()
    head.data.update(); bpy.context.view_layer.update()

    def surface(x,z):
        hit,p,*_=head.ray_cast(Vector((x,-2,z)),Vector((0,1,0)))
        assert hit, ('Missing facial surface',x,z)
        return p.y

    nose=ellipsoid('Hearthling_Nose',(0,surface(0,1.492)-.019,1.492),(.066,.038,.033),dark,24,14)
    for v in nose.data.vertices:
        v.co.x*=.64+.36*smooth((v.co.z-1.459)/.066)
    bind(nose)
    if nose.name not in collection.objects: collection.objects.link(nose)
    points=[]
    for i in range(33):
        u=-1+2*i/32; x=u*.147
        z=1.410+.026*u*u-.006*math.sin(math.pi*abs(u))
        points.append((x,surface(x,z)-.003,z))
    tube('Hearthling_ClosedSmile',points,[.001+.0026*math.sin(math.pi*i/32) for i in range(33)],dark)
    points=[(0,surface(0,z)-.003,z) for z in [1.466,1.449,1.430,1.410]]
    tube('Hearthling_NoseToLip',points,[.0024]*len(points),dark)

    for sign,side in [(-1,'L'),(1,'R')]:
        points=[]
        for i in range(33):
            a=math.pi*i/32; dx=.128*math.cos(a)
            x=sign*EYE_X+dx; z=EYE_Z+.096*math.sin(a)+sign*.08*dx
            y=surface(sign*EYE_X+dx*1.03,EYE_Z+(z-EYE_Z)*1.04)-.002
            points.append((x,y,z))
        tube('Hearthling_OrbitalRim_'+side,points,[.001+.0045*math.sin(math.pi*i/32) for i in range(33)],brow_mat)
        points=[]
        for i in range(17):
            u=-1+2*i/16; x=sign*EYE_X+u*.109; z=1.756+.037*(1-u*u)-sign*.014*u
            points.append((x,surface(x,z)-.004,z))
        tube('Hearthling_Brow_'+side,points,[.001+.009*math.sin(math.pi*i/16) for i in range(17)],brow_mat)
        edge=bpy.data.objects['Hearthling_LidEdge_'+side]
        center=Vector((sign*EYE_X,EYE_Y,EYE_Z))
        for v in edge.data.vertices: v.co=center+(v.co-center)*.985
        bm=bmesh.new(); bm.from_mesh(edge.data)
        bmesh.ops.delete(bm,geom=[v for v in bm.verts if abs(v.co.x-sign*EYE_X)>.112],context='VERTS')
        bmesh.ops.holes_fill(bm,edges=[e for e in bm.edges if e.is_boundary],sides=0)
        bm.to_mesh(edge.data); bm.free()
        # Change the color-ring geometry on the sphere; keep its skin clearance.
        eye=bpy.data.objects['Hearthling_Eyeball_'+side]
        pupil=math.asin(.067/.133); iris=math.asin(.097/.133)
        new_pupil=math.asin(.063/.133); new_iris=math.asin(.102/.133)
        for v in eye.data.vertices:
            d=v.co-center; theta=math.acos(max(-1,min(1,-d.y/EYE_R))); phi=math.atan2(d.z,d.x)
            if theta<=pupil: t=theta*new_pupil/pupil
            elif theta<=iris: t=new_pupil+(theta-pupil)*(new_iris-new_pupil)/(iris-pupil)
            else: t=new_iris+(theta-iris)*(math.pi-new_iris)/(math.pi-iris)
            v.co=center+Vector((EYE_R*math.sin(t)*math.cos(phi),-EYE_R*math.cos(t),EYE_R*math.sin(t)*math.sin(phi)))
        eye.data.update()
    shader=bpy.data.materials['Hearthling | spherical eyes'].node_tree.nodes.get('Principled BSDF')
    shader.inputs['Roughness'].default_value=.23; shader.inputs['Coat Weight'].default_value=.20
    # The full exporter recreates this palette material during the fur bake.
    gold_mat=bpy.data.materials.get('Hearthling | warm ochre')
    if gold_mat and gold_mat!=coat:
        head.data.materials.append(gold_mat)
        for p in head.data.polygons:
            if p.center.z>1.91: p.material_index=1
    refine_eye_proportions()
    rig['expression_revision']=8; rig.data.pose_position=previous
    bpy.context.view_layer.update()
    print('REFERENCE_EXPRESSION_REVISION_8',flush=True)


def refine_eye_proportions():
    """Reduce globe/lids together and taper the surrounding eye openings."""
    rig=bpy.data.objects['Hearthling_Rig']
    assert not rig.get('smaller_almond_eyes'), 'Eye proportions already applied'
    previous=rig.data.pose_position; rig.data.pose_position='REST'
    scale=.85

    def orbital_map(p, force=False):
        x,y,z=p; sign=-1 if x<0 else 1
        center=Vector((sign*EYE_X,EYE_Y,EYE_Z))
        d=Vector(p)-center
        rho=math.sqrt((d.x/.128)**2+((d.z-sign*.08*d.x)/.100)**2)
        weight=1. if force else (1-smooth((rho-1.15)/.85))*smooth((-.03-y)/.12)
        if weight<=0: return Vector(p)
        q=center+d*scale
        u=min(1,abs(d.x)/.128)
        # Flatten the arcs toward both corners without narrowing their span.
        height=q.z-EYE_Z-sign*.08*(q.x-sign*EYE_X)
        q.z-=height*(.055+.19*u*u)
        radial=(q.x-sign*EYE_X)**2+(q.z-EYE_Z)**2
        if y<EYE_Y and radial<(EYE_R*scale)**2:
            q.y=min(q.y,EYE_Y-math.sqrt((EYE_R*scale)**2-radial)-.012)
        return Vector(p).lerp(q,weight)

    for name in ['Hearthling_Head','Hearthling_OrbitalRim_L','Hearthling_OrbitalRim_R']:
        ob=bpy.data.objects[name]
        for v in ob.data.vertices: v.co=orbital_map(v.co,name!='Hearthling_Head')
        ob.data.update()
    for sign,side in [(-1,'L'),(1,'R')]:
        center=Vector((sign*EYE_X,EYE_Y,EYE_Z))
        for part in ['Eyeball','UpperEyelid','LowerEyelid','LidEdge']:
            ob=bpy.data.objects['Hearthling_'+part+'_'+side]
            for v in ob.data.vertices: v.co=center+(v.co-center)*scale
            ob.data.update()
    # Uniform scaling about the existing lid pivot preserves the blink arc.
    rig['smaller_almond_eyes']=True
    rig['eye_globe_scale']=scale
    shrink_pupils()
    rig.data.pose_position=previous; bpy.context.view_layer.update()


def shrink_pupils():
    """Expose more iris by reducing pupil radius 15%, keeping the eye spherical."""
    rig=bpy.data.objects['Hearthling_Rig']
    assert not rig.get('pupil_radius_scale'), 'Pupil reduction already applied'
    radius=EYE_R*rig.get('eye_globe_scale',1.)
    old=math.asin(.063/.133); new=math.asin(.063*.85/.133)
    iris=math.asin(.102/.133)
    for sign,side in [(-1,'L'),(1,'R')]:
        center=Vector((sign*EYE_X,EYE_Y,EYE_Z))
        eye=bpy.data.objects['Hearthling_Eyeball_'+side]
        for v in eye.data.vertices:
            d=v.co-center; theta=math.acos(max(-1,min(1,-d.y/radius)))
            if theta>=iris: continue
            phi=math.atan2(d.z,d.x)
            t=theta*new/old if theta<=old else new+(theta-old)*(iris-new)/(iris-old)
            v.co=center+Vector((radius*math.sin(t)*math.cos(phi),-radius*math.cos(t),radius*math.sin(t)*math.sin(phi)))
        eye.data.update()
    rig['pupil_radius_scale']=.85


def paint_reference_face():
    """Approved orange-brown eye/forehead mask, cream only on muzzle and chin."""
    head=bpy.data.objects['Hearthling_Head']
    attr=head.data.color_attributes.get('CoatColor') or head.data.color_attributes.new(name='CoatColor',type='FLOAT_COLOR',domain='POINT')
    gold=(.69,.245,.036,1); russet=(.37,.108,.022,1); cream=(.94,.84,.66,1)
    for v in head.data.vertices:
        x,y,z=v.co
        eyes=min(((x-s*EYE_X)/.183)**2+((z-1.640)/.170)**2 for s in [-1,1])
        forehead=(x/.150)**2+((z-1.815)/.205)**2
        dark=(1-smooth((min(eyes,forehead)-.55)/1.05))*smooth((.09-y)/.23)
        base=tuple(gold[k]*(1-dark)+russet[k]*dark for k in range(4))
        # Amber crown and tapered russet strands follow the reference fur flow.
        # Vertex paint adds readable variation without another texture or draw.
        front=smooth((.035-y)/.20)
        zone=smooth((z-1.695)/.09)*(1-smooth((abs(x)-.29)/.12))*front
        center=math.exp(-(x/(.054+.10*smooth((z-1.73)/.27)))**2)
        amber=(.86,.367,.070,1)
        lift=.72*center*zone
        base=tuple(base[k]*(1-lift)+amber[k]*lift for k in range(4))
        strand_x=abs(x)-(.048+.76*(z-1.740))
        feather=.006*math.sin(z*185+x*32)+.003*math.sin(z*317-x*61)
        stripe=math.exp(-((strand_x+feather)/.032)**2)
        stripe*=smooth((z-1.715)/.065)*(1-smooth((z-1.965)/.09))*zone*.90
        chestnut=(.105,.024,.005,1)
        base=tuple(base[k]*(1-stripe)+chestnut[k]*stripe for k in range(4))
        flow=x*(1.0+1.5*smooth((z-1.72)/.30))
        grain=math.sin(flow*167+z*17+.8*math.sin(z*42))
        grain+=.45*math.sin(flow*269-z*29)
        gain=1+.13*grain*zone
        base=tuple(min(1,base[k]*gain) for k in range(3))+(1,)
        muzzle=(x/.323)**2+((z-1.431)/.143)**2
        ivory=(1-smooth((muzzle-.80)/.52))*smooth((.04-y)/.20)
        attr.data[v.index].color=tuple(base[k]*(1-ivory)+cream[k]*ivory for k in range(4))
    # Rotating lid shells use the same warm mask, including when closed.
    lid=bpy.data.materials['Hearthling | cream']
    lid.diffuse_color=russet
    lid.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value=russet
    head.data.update()


if __name__=='__main__': refine_expression()
