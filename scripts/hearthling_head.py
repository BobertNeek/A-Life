"""Continuous head topology with two constrained eye openings."""
import math
import bmesh
from mathutils import Vector
from mathutils.geometry import delaunay_2d_cdt


def head_radius(x,z):
    nz=(z-2.045)/(.34 if z>=2.045 else .31)
    return (x/(.45*(1-.15*max(0,-nz))))**2+nz*nz


def head_y(x,z,eye_y,eye_z,eye_radius):
    y=.025-.345*math.sqrt(max(0,1-head_radius(x,z)))
    for sign in [-1,1]:
        dx=x-sign*.192; dz=z-eye_z
        height=.077 if dz>=0 else .061
        rho=math.sqrt((dx/.112)**2+(dz/height)**2)
        radial=dx*dx+dz*dz
        if rho<1.75:
            bx=dx/max(1,rho); bz=dz/max(1,rho)
            boundary=eye_y-math.sqrt(max(.0001,eye_radius**2-bx*bx-bz*bz))-.009
            t=max(0,min(1,(rho-1)/.75)); blend=t*t*(3-2*t)
            y=boundary*(1-blend)+y*blend
            if radial<eye_radius**2:
                surface=eye_y-math.sqrt(eye_radius**2-radial)-.009-.008*math.sin(math.pi*t)
                y=min(y,surface)
    return y


def make_head(body, mesh, cream, eye_y, eye_z, eye_radius):
    points=[]; edges=[]; loops=[]; count=64

    def outline(a):
        s=math.sin(a)
        return (.45*math.cos(a)*(1-.15*max(0,-s)), 2.045+(.34 if s>=0 else .31)*s)

    def add_loop(coords):
        ids=list(range(len(points),len(points)+len(coords)))
        points.extend(Vector(p) for p in coords)
        edges.extend((ids[i],ids[(i+1)%len(ids)]) for i in range(len(ids)))
        loops.append(ids)

    add_loop([outline(2*math.pi*i/count) for i in range(count)])
    for sign in [-1,1]:
        for scale in [1,1.3,1.6]:
            add_loop([(sign*.192+.112*scale*math.cos(2*math.pi*i/48),
                       eye_z+(.077 if math.sin(2*math.pi*i/48)>=0 else .061)*scale*math.sin(2*math.pi*i/48)) for i in range(48)])

    def in_eye(x,z,margin=1):
        dz=z-eye_z; h=.077 if dz>=0 else .061
        return any(((x-sign*.192)/(.112*margin))**2+(dz/(h*margin))**2<1 for sign in [-1,1])

    for ix in range(46):
        for iz in range(34):
            x=-.45+ix*.02; z=1.725+iz*.020
            if head_radius(x,z)<.965 and not in_eye(x,z,1.02): points.append(Vector((x,z)))
    verts2,_,triangles,orig,_,_=delaunay_2d_cdt(points,edges,[],0,.00001,True)
    verts=[]; faces=[]
    for point in verts2:
        x,z=point
        y=head_y(x,z,eye_y,eye_z,eye_radius)
        verts.append((x,y,z))
    for tri in triangles:
        x=sum(verts2[i].x for i in tri)/3
        z=sum(verts2[i].y for i in tri)/3
        if head_radius(x,z)<=1.001 and not in_eye(x,z,.998): faces.append(tuple(tri))

    # Sew the back of the head directly to the front outline.
    mapping={source:i for i,items in enumerate(orig) for source in items}
    previous=[mapping[i] for i in loops[0]]
    for ring in range(1,13):
        a=ring*math.pi/2/13; scale=math.cos(a)
        current=[]
        for j in range(count):
            ox,oz=outline(2*math.pi*j/count)
            current.append(len(verts))
            verts.append((ox*scale,.025+.295*math.sin(a),2.045+(oz-2.045)*scale))
        for j in range(count):
            n=(j+1)%count
            faces.append((previous[j],current[j],current[n],previous[n]))
        previous=current
    pole=len(verts); verts.append((0,.320,2.045))
    for j in range(count): faces.append((previous[j],pole,previous[(j+1)%count]))
    head=mesh('Hearthling_Head',verts,faces,body.data.materials[0])
    colors=head.data.color_attributes.new(name='CoatColor',type='FLOAT_COLOR',domain='POINT')
    gold=(.64,.31,.055,1)
    for v in head.data.vertices:
        x,y,z=v.co
        lower=(x/.39)**2+((z-1.91)/.19)**2
        eye=min(((x-sign*.192)/.18)**2+((z-2.075)/.185)**2 for sign in [-1,1])
        w=max(0,min(1,(1.45-min(lower,eye))/.60))*max(0,min(1,(-y+.04)/.18))
        colors.data[v.index].color=tuple(gold[i]*(1-w)+cream.diffuse_color[i]*w for i in range(4))

    # Retain the existing torso/neck and its weights; the new head covers this cut.
    bm=bmesh.new(); bm.from_mesh(body.data)
    bmesh.ops.delete(bm,geom=[v for v in bm.verts if v.co.z>1.70],context='VERTS')
    for v in bm.verts:
        if v.co.z>1.58:
            t=min(1,(v.co.z-1.58)/.12)
            v.co.x*=1-.45*t
            v.co.y*=1-.3*t
    boundary=list({v for e in bm.edges if e.is_boundary for v in e.verts})
    assert boundary and all(v.co.z>1.60 for v in boundary), 'Expected one neck opening'
    boundary.sort(key=lambda v:math.atan2(v.co.y-.02,v.co.x))
    deform=bm.verts.layers.deform.verify()
    coat=bm.verts.layers.float_color.get('CoatColor')
    previous=boundary
    for z,rx,ry in [(1.73,.125,.115),(1.80,.145,.13)]:
        current=[]
        for source in boundary:
            angle=math.atan2(source.co.y-.02,source.co.x)
            v=bm.verts.new((rx*math.cos(angle),.02+ry*math.sin(angle),z))
            v[deform][body.vertex_groups['head'].index]=1
            if coat is not None: v[coat]=source[coat]
            current.append(v)
        for j in range(len(boundary)):
            n=(j+1)%len(boundary)
            bm.faces.new((previous[j],previous[n],current[n],current[j]))
        previous=current
    bm.faces.new(previous)
    transition=[v for v in bm.verts if v.co.z>1.55]
    for _ in range(3):
        bmesh.ops.smooth_vert(bm,verts=transition,factor=.35,
                             use_axis_x=True,use_axis_y=True,use_axis_z=True)
    bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
    bm.to_mesh(body.data); bm.free(); body.data.update()
    for poly in body.data.polygons: poly.use_smooth=True
    return head
