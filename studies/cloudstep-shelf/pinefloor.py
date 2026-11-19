"""Pineglass Floor: true upland clearing above the original cloud-shelf boulder.
A high eye pixel is not a reason to put pine duff in an open clearing.
Source-only soil, cryptogam and loose flake artwork, never collisions or food.
"""
from pathlib import Path
import csv,json,math,struct,sys
import numpy as np
from PIL import Image
HERE=Path(__file__).resolve().parent
sys.path[:0]=[str(HERE),str(HERE.parents[1]/"scripts"),str(HERE.parent/"lanternback-rim")]
from island_inlet_ground import Inlet,ease
from island_erratic_glb import streams
from finished_island_glb import save
from strata import nz
from mountainskin import plates
BOX=(73.5,86.5,-418.5,-404.0)

def lookup(image,box,x,z):
    a,b,c,d=box
    h,w=image.shape[:2];i=np.clip(((x-a)*w/(b-a)).astype(int),0,w-1);j=np.clip(((z-c)*h/(d-c)).astype(int),0,h-1)
    return image[j,i]

def environment(land,folder,x,z):
    y,gx,gz=land.floor(x,z);grade=np.hypot(gx,gz)
    # Short terrain samples: the broad axial clearing sits legitimately ABOVE
    # native BoulderC #1330; we do not send that boulder's debris uphill.
    curv=(land.floor(x+.85,z)[0]+land.floor(x-.85,z)[0]+land.floor(x,z+.85)[0]+land.floor(x,z-.85)[0]-4*y)/.85
    broad=nz(x,z,2.46,178)-.5;mid=nz(x,z,.72,388)-.5;sm=nz(x,z,.19,445)-.5
    micro=nz(x,z,.041,601)-.5
    protected=json.loads((folder/'held-copse.json').read_text())['box_xz']
    shed=np.asarray(Image.open(folder/'forest-shed-gain.png'))
    duff=np.clip((128-lookup(shed,protected,x,z)[...,2].astype('f'))/36,0,1)
    keep=lookup(np.asarray(Image.open(folder/'held-copse-coverage.png')),protected,x,z)>0
    edge=ease(0,.80,np.minimum(x-BOX[0],BOX[1]-x))*ease(0,.80,np.minimum(z-BOX[2],BOX[3]-z))
    exposure=ease(.04,.74,.49+.41*broad+.76*mid+.46*sm-.92*curv+.45*grade-.43*duff)
    lens=exposure*(.64+.36*ease(.12,.62,.39+.72*broad+.43*mid))
    # No camera / player coordinates in the material: the geological strike
    # belongs to the shelf. Original cone deposition remains spatially sparse.
    axis=.83*x-.55*z+.23*y;cross=.55*x+.83*z-.14*y
    j,c=plates(axis/0.31+.24*mid,cross/0.143+.22*sm)
    split=(1-ease(.018,.118,j))*(.19+.81*ease(.28,.66,.49+.65*sm+.55*micro))
    # Thin warm organics in true sheltered pockets; mineral belly in the open.
    fern=duff*edge
    surface=edge*(1-.43*keep)
    peb=lens*(.36+.58*c+.31*sm+.29*micro-.76*split)
    occluded=ease(.02,.18,curv)*(.48+.34*mid)
    moss=ease(.28,.78,.43+.87*broad+.58*mid-.52*grade+.69*curv)*(1-ease(.63,.94,duff))
    gain=np.stack((
        1+surface*(.23+.66*peb-.22*occluded-.30*moss)+.25*fern,
        1+surface*(.095+.43*peb-.15*occluded+.09*moss)-.10*fern,
        1+surface*(-.035+.31*peb-.20*occluded-.04*moss)-.18*fern),-1)
    # Microstructure affects albedo and a separate optical relief height, not
    # the V1 triangles. Exposed grains need shadows of their own at this scale.
    rough=np.clip(.86-.1*occluded+.09*sm,0,1)
    relief=np.clip(.48+.18*peb+.09*sm+.065*micro-.18*split,.03,.98)
    return gain,rough,relief,edge,exposure,moss,duff,keep,y,gx,gz

def make(native,folder):
    land=Inlet(native/'island-v1.bin');a,b,c,d=BOX;W,H=680,758
    albedo=np.empty((H,W,3),'B');normal=np.empty((H,W,3),'B')
    for j in range(0,H,20):
        stop=min(j+20,H)
        x,z=np.meshgrid(a+(np.arange(W)+.5)*(b-a)/W,c+(np.arange(j,stop)+.5)*(d-c)/H)
        g,rough,relief,edge,*_=environment(land,folder,x,z)
        albedo[j:stop]=np.uint8(np.clip(np.rint(g*112),0,255))
        normal[j:stop]=np.uint8(np.rint(255*np.stack((rough,relief,.64*edge),-1)))
    Image.fromarray(albedo).save(folder/'pineglass-mineral.png',optimize=True)
    Image.fromarray(normal).save(folder/'pineglass-response.png',optimize=True)
    return placement(native,folder,land)

def obstacles(native):
    raw=(native/'island-v1.bin').read_bytes();w,d=struct.unpack_from('<II',raw,8)
    return np.frombuffer(raw,dtype='<f4',offset=32+w*d*4).reshape(-1,6)

def placement(native,folder,land):
    a,b,c,d=BOX;r=np.random.default_rng(563750);x=r.uniform(a+.2,b-.2,2800);z=r.uniform(c+.2,d-.2,2800)
    _,_,_,e,open_,moss,duff,copse,y,dx,dz=environment(land,folder,x,z)
    obstacle=obstacles(native)
    relevant=obstacle[(obstacle[:,0]<=b+.3)&(obstacle[:,2]>=a-.3)&(obstacle[:,1]<=d+.3)&(obstacle[:,3]>=c-.3)]
    free=np.ones(len(x),bool)
    for xmin,zmin,xmax,zmax,yl,yh in relevant:
        free&=~((xmin-.24<x)&(x<xmax+.24)&(zmin-.24<z)&(z<zmax+.24))
    roots=[];score=np.maximum(moss*(.73+.27*open_),open_*(.65+.35*(1-duff)))
    ok=free&(~copse)&(e>.65)&(np.hypot(dx,dz)<.45)
    # A few actual xeric colonies gathered into coherent neighbourhoods.
    # Sorting with site distance discourages evenly spaced identical dots.
    sectors=[]
    for k in np.argsort(-score+r.uniform(-.08,.08,len(x))):
        if not ok[k] or any(np.hypot(x[k]-xx,z[k]-zz)<r.uniform(1.4,2.7) for xx,zz in sectors):continue
        sectors.append((x[k],z[k]))
        if len(sectors)==12:break
    for mi,(cx,cz) in enumerate(sectors):
        nearby=np.flatnonzero(ok & (np.hypot(x-cx,z-cz)<r.uniform(.53,.97)))
        r.shuffle(nearby);taken=[]
        for k in nearby:
            if any(np.hypot(x[k]-x[h],z[k]-z[h])<.123 for h in taken):continue
            typ='shelf lichen' if moss[k]>.36 and r.random()<.73 else 'weathered flake'
            rr=r.uniform(.032,.084) if typ=='shelf lichen' else r.uniform(.025,.073)
            roots.append([mi,typ,float(x[k]),float(y[k]),float(z[k]),rr,r.uniform(-math.pi,math.pi),float(dx[k]),float(dz[k])])
            taken.append(k)
            if len(taken)>=r.integers(6,14):break
    with (folder/'pineglass-roots.csv').open('w',newline='') as f:
        w=csv.writer(f);w.writerow(['colony','kind','x','v1_y','z','radius_m','yaw','native_dx','native_dz']);w.writerows(roots)
    return save_floor(land,folder,roots,len(sectors),len(relevant))

def save_floor(land,folder,roots,ncolonies,nobstacles):
    P=[];C=[];F=[];rng=np.random.default_rng(91677)
    def patch(cx,cz,rad,angle,lichen):
        sides=14 if lichen else 8
        phase=rng.uniform(0,6.28);h=rng.uniform(.007,.020) if lichen else rng.uniform(.008,.026)
        ox=rad*np.cos(angle)*.1;oz=rad*np.sin(angle)*.1;start=len(P)
        base=np.array([.205,.251,.186] if lichen else [.244,.21,.169])*rng.uniform(.8,1.22)
        for ring in range(3):
            for j in range(sides):
                t=2*math.pi*j/sides+phase
                lobes=1+.12*math.sin(3*t+phase)+(.16*math.cos(5*t-phase) if lichen else .07*np.sin(2*t))
                width=(.085,.66,1.)[ring]*rad*lobes;xx=np.cos(t)*width;zz=np.sin(t)*width*(.68 if lichen else .59)
                wx=cx+xx*np.cos(angle)-zz*np.sin(angle)+ox*(1-ring*.5);wz=cz+xx*np.sin(angle)+zz*np.cos(angle)+oz*(1-ring*.5)
                yy=land.floor(wx,wz)[0]
                rise=((.81,1.,.38) if lichen else (1.,.80,-.13))[ring]*h + (.21*np.sin(t*4)*h if ring==2 and lichen else 0.)
                P.append((wx,yy+rise,wz))
                tint=(.78,1.,1.21)[ring] if lichen else (1.05,1.05,.73)[ring]
                C.append(base*tint*rng.uniform(.93,1.08))
                if ring:
                    a=start+(ring-1)*sides+j;b=start+(ring-1)*sides+(j+1)%sides
                    F.extend([(a,a+sides,b),(a+sides,b+sides,b)])
        F.extend((start,start+j+1,start+j) for j in range(1,sides-1))
    for _,form,x,y,z,rr,yaw,dx,dz in roots:
        if form=='shelf lichen':
            for j in range(rng.integers(2,5)):
                t=yaw+j*2.39;off=rr*rng.uniform(.16,.57);patch(x+off*np.cos(t),z+off*np.sin(t),rr*rng.uniform(.37,.73),t,True)
        else:patch(x,z,rr,yaw,False)
    p=np.asarray(P,'<f4');c=np.asarray(C,'<f4');tri=np.array(F,'<u2')
    face=np.cross(p[tri[:,1]]-p[tri[:,0]],p[tri[:,2]]-p[tri[:,0]])
    down=face[:,1]<0;tri[down]=tri[down][:,[0,2,1]];face[down]*=-1
    n=np.zeros_like(p)
    for j in range(3):np.add.at(n,tri[:,j],face)
    n/=np.maximum(1e-9,np.linalg.norm(n,axis=1,keepdims=True))
    doc=dict(asset=dict(version='2.0',generator='Cloudstep / clearing mineral field'),scene=0,scenes=[dict(nodes=[0])],
             nodes=[dict(name='Pineglass near shelf only',mesh=0)],meshes=[],accessors=[],bufferViews=[],
             materials=[dict(name='Worn mineral fines and sage lichens',doubleSided=True,pbrMetallicRoughness=dict(metallicFactor=0,roughnessFactor=.91))])
    buf=bytearray();emit=streams(doc,buf)
    attrs=dict(POSITION=emit(p,5126,'VEC3',bounds=True),NORMAL=emit(n,5126,'VEC3'),COLOR_0=emit(c,5126,'VEC3'))
    doc['meshes']=[dict(primitives=[dict(material=0,attributes=attrs,indices=emit(tri.ravel(),5123,'SCALAR',indices=True))])]
    doc['buffers']=[dict(byteLength=len(buf))];save(doc,buf,folder/'pineglass-eye.glb')
    info=dict(native_plan_xz=BOX,neutral=112,metres_per_texel=[13/680,14.5/758],artist_colonies=ncolonies,
              rooted_sites=len(roots),held_native_obstacles_in_window=nobstacles,original_copse_footprint_respected=True,
              placement_depends_on_camera=False,draw_once_body_triangles=len(tri),guardian_triangles=0,game_objects=0)
    (folder/'pineglass.json').write_text(json.dumps(info,indent=2)+'\n')
    return info
if __name__=='__main__':print(make(*(Path(v) for v in sys.argv[1:])))
