"""The lifted lips of present ground + glaucous plants rooted in its shelter."""
from pathlib import Path
from collections import defaultdict, Counter
import json, math, struct
import numpy as np
from finished_island_glb import save
from island_erratic_glb import streams
from shore import fields

class Piece:
    def __init__(self):
        self.xyz=[]; self.rgb=[]; self.tri=[]
    def draw(self,pts,cols,tris):
        a=len(self.xyz); self.xyz.extend(pts)
        self.rgb.extend(np.broadcast_to(cols,(len(pts),3)))
        for t in tris:
            t=list(t); p=np.array([pts[i] for i in t]); cross=np.cross(p[1]-p[0],p[2]-p[0])
            if cross[1]<0:t[2],t[1]=t[1],t[2]
            self.tri.append(tuple(a+i for i in t))

def export(parts,file):
    doc=dict(asset=dict(version='2.0',generator='Qualia / Weatherlip Bight native-coordinate art'),
             scene=0,scenes=[dict(nodes=[])],nodes=[],meshes=[],buffers=[],bufferViews=[],accessors=[],
             materials=[dict(name='West bight / weathered living pigment',doubleSided=True,
                            pbrMetallicRoughness=dict(metallicFactor=0.,roughnessFactor=.88))])
    buf=bytearray(); emit=streams(doc,buf); total=0
    for label,g in sorted(parts.items()):
        if not g.tri:continue
        p=np.asarray(g.xyz,dtype='<f4'); pivot=(p.min(0)+p.max(0))*.5
        p-=pivot; tt=np.asarray(g.tri,dtype=np.uint32); n=np.zeros_like(p)
        fn=np.cross(p[tt[:,1]]-p[tt[:,0]],p[tt[:,2]]-p[tt[:,0]])
        for k in range(3): np.add.at(n,tt[:,k],fn)
        n/=np.maximum(1e-12,np.linalg.norm(n,axis=1,keepdims=True))
        c=np.clip(np.asarray(g.rgb)/255,0,1)
        l=np.where(c<=.04045,c/12.92,((c+.055)/1.055)**2.4)
        rgba=np.column_stack((np.uint8(np.rint(l*255)),np.full(len(p),255,'B')))
        ttype=5123 if len(p)<65536 else 5125
        at=dict(POSITION=emit(p,5126,'VEC3',bounds=True),NORMAL=emit(n,5126,'VEC3'),
                COLOR_0=emit(rgba,5121,'VEC4',normalized=True))
        ii=emit(tt.ravel().astype('<u2' if ttype==5123 else '<u4'),ttype,'SCALAR',indices=True)
        i=len(doc['nodes']); doc['nodes'].append(dict(mesh=i,name=label,translation=pivot.tolist()))
        doc['meshes'].append(dict(name=label,primitives=[dict(attributes=at,indices=ii,material=0)]))
        doc['scenes'][0]['nodes'].append(i);total+=len(tt)
    doc['buffers']=[dict(byteLength=len(buf))]; save(doc,buf,file)
    return dict(triangles=total,groups=len(doc['nodes']))

def build(b,source,folder):
    R=np.random.default_rng(592610)
    raw=Path(source).read_bytes()
    W,D,_,_,_,cnt=struct.unpack_from('<IIfffI',raw,8)
    boxes=np.frombuffer(raw,'<f4',cnt*6,32+4*W*D).reshape(cnt,6)
    boxes=boxes[(boxes[:,0]<-280)&(boxes[:,1]>-430)&(boxes[:,2]<239)&(boxes[:,3]>-125)]
    mineral=defaultdict(Piece); plants=defaultdict(Piece); count=Counter()
    def screened(x,z,margin=.18):
        return np.any((boxes[:,0]-margin<x)&(x<boxes[:,1]+margin)&(boxes[:,2]-margin<z)&(z<boxes[:,3]+margin))
    def points(p,lift):
        y=b.floor(p[:,0],p[:,1])[0]+lift
        return np.column_stack((p[:,0],y,p[:,1]))
    def colour(t,s): return np.asarray(t)*s
    def plate(x,z,length,width,angle,tall,base):
        g=mineral[f"Broken beds / {int((z+140)//27):02d}"]
        a=np.array([math.cos(angle),math.sin(angle)]); n=np.array([-a[1],a[0]])
        # 6–8 corner seam, no centre spike. Buried perimeter plus two flatchested
        # shoulders; even the larger pieces stay near-flush with V1.
        local=np.array([[-.96,-.21],[-.73,-.57],[.11,-.5],[.78,-.37],[.96,.04],[.58,.47],[-.28,.50],[-.77,.29]])
        local+=R.uniform(-.09,.09,(8,2));xz=[x,z]+local@np.stack((a*length/2,n*width))
        inner=[x,z]+(xz-[x,z])*.86 + n*width*.035
        yedge=np.array([.30,.61,.79,.95,.65,.31,.22,.20])*tall
        xyz=np.row_stack((points(xz,-.017),points(inner,yedge),points(np.array([[x,z]+a*length*.20,[x,z]-a*length*.26]),np.array([tall*.82,tall*.77]))))
        t=[]
        for j in range(8):
            k=(j+1)%8;t.extend(((j,k,8+k),(j,8+k,8+j)))
        t.extend(((8,9,17),(9,10,17),(10,16,17),(10,11,16),(11,12,16),(12,13,16),(13,14,16),(14,17,16),(14,15,17),(15,8,17)))
        c=np.asarray(base)*R.uniform(.90,1.07)
        g.draw(xyz,np.row_stack(([c*.61]*8,[c*float(v) for v in R.uniform(.93,1.05,8)],[c*1.075,c*.99])),t)
    def blade(g,center,vec,length,height,width,ink):
        n=np.array([-vec[1],vec[0]])
        root=b.floor(*center)[0]-.012
        curve=R.uniform(-.32,.32); p=center
        xy=np.array([p-.2*width*n,p+.2*width*n,p+.32*length*vec-width*n,
                     p+.32*length*vec+width*n,p+.32*length*vec,p+.78*length*vec+.45*width*n+curve*n*length*.30,
                     p+length*vec+curve*length*n])
        up=np.array([0,0,.61,.61,.67,1,.77])*height+root
        floor=b.floor(xy[:,0],xy[:,1])[0]
        up[2:]=np.maximum(up[2:],floor[2:]+.008)
        xyz=np.column_stack((xy[:,0],up,xy[:,1]));c=np.asarray(ink)
        g.draw(xyz,c*np.array([.58,.65,.90,.98,1.09,.97,1.025])[:,None],
               ((0,1,4),(0,4,2),(1,3,4),(2,4,5),(4,3,5),(2,5,6),(5,3,6)))
    def cushion(g,p,size,kind):
        az=R.uniform(-math.pi,math.pi)
        ink=dict(fescue=(136,153,128),glass=(153,168,131),thrift=(121,141,102),wrack=(102,105,70))[kind]
        for j in range(R.integers(5,9)):
            a=az+j*2.39996;v=np.array([math.cos(a),math.sin(a)])
            S=size*R.uniform(.72,1.14)
            if kind=='fescue':l,h,w=.56,.48,.032
            elif kind=='thrift':l,h,w=.36,.18,.048
            elif kind=='glass':l,h,w=.39,.22,.085
            else:l,h,w=.78,.055,.060
            blade(g,p+R.uniform(-.040,.040,2)*S,v,l*S,h*S,w*S,np.asarray(ink)*R.uniform(.92,1.05))
        if kind=='thrift':
            # A few shell-pink thrift heads, not pixels on every plant.
            for j in range(R.integers(1,4)):
                c=p+R.uniform(-.08,.08,2)*size
                hh=b.floor(*c)[0]; ht=size*R.uniform(.22,.36)
                top=c+(.061*size)*np.array([.4+math.cos(az+j),.5+math.sin(az+j)])
                rr=.009*size
                g.draw([(c[0]-rr,hh-.015,c[1]),(c[0]+rr,hh-.015,c[1]),(top[0]+rr*.5,hh+ht,top[1]),(top[0]-rr*.5,hh+ht,top[1])],(136,149,105),((0,1,2),(0,2,3)))
                s=.039*size; ang=np.arange(6)*math.pi/3+az
                xx=top[0]+s*np.cos(ang); zz=top[1]+s*np.sin(ang)
                xyz=list(zip(xx,[hh+ht]*6,zz))+[(top[0],hh+ht+s*.65,top[1])]
                ccol=np.vstack(([colour((201,162,153),R.uniform(.92,1.06)) for q in range(6)],(223,196,172)))
                g.draw(xyz,ccol,[(j,(j+1)%6,6) for j in range(6)])
    # Exposed fingers follow dipping strata; the flatter stretch keeps fewer,
    # smaller pieces instead of being outlined in loose gravel.
    zz=R.uniform(-104,219,1900); dd=R.uniform(-2,27,len(zz));xx=b.waterx(zz)+dd
    ds,hh,ss,_,_,_,rock,_,_,cov=fields(b,xx,zz)
    for i in range(len(zz)):
        x,z=xx[i],zz[i]
        if R.random()>(.06+.88*rock[i])*cov[i] or hh[i]<-.22:continue
        if screened(x,z,.05):continue
        angle=1.18+R.normal(0,.18)+(.40 if z>62 else 0)
        for j in range(R.integers(2,6)):
            sz=R.uniform(.29,1.25)*R.uniform(.78,1.30)*(1+.18*min(ss[i],1.3))
            off=R.normal(0,[.57,1.80],2)
            p=np.array([x,z])+off
            if screened(*p,.04):continue
            hh2,gx,gz=b.floor(*p)
            if hh2<-.34:continue
            hue=R.random(); c=(133,140,132) if hue<.52 else (153,145,122) if hue<.88 else (172,161,135)
            if hh2<.72: c=np.asarray(c)*[.79,.86,.83]
            ceiling=.075 if math.hypot(gx,gz)>.85 else .038
            plate(*p,sz,sz*R.uniform(.28,.59),angle+R.normal(0,.09),min(ceiling,sz*.061),c)
            count['grounded_plate_fragments']+=1
    # Actual shelter-dependent colonies, first seeded over the entire cove.
    # Group-level spacing and intra-group holes prevent dotted contour lines.
    zz=R.uniform(-103,214,650);dd=R.uniform(1.1,45,len(zz));xx=b.waterx(zz)+dd
    _,yy,grade,_,_,_,ex,turf,wet,coverage=fields(b,xx,zz)
    for i,(x,z) in enumerate(zip(xx,zz)):
        if coverage[i]<.18 or grade[i]>.85 or yy[i]<.35:continue
        if R.random()>.36*(.48+.52*turf[i])*(1-.70*ex[i]):continue
        kind='fescue' if dd[i]>19 else ('thrift' if ex[i]>.14 or R.random()<.35 else 'glass')
        patchscale=R.uniform(.46,1.58); nplant=R.integers(8,25)
        for j in range(nplant):
            px,pz=np.array([x,z])+R.normal(0,[1.22,2.5],2)*patchscale
            h,gx,gz=b.floor(px,pz)
            if h<.32 or math.hypot(gx,gz)>.88 or screened(px,pz):continue
            size=R.uniform(.42,1.02)*(1.2 if kind=='fescue' else .87)
            k=kind if R.random()<.80 else ('fescue' if kind=='glass' else 'glass')
            cushion(plants[f"Living margins / {int((pz+140)//27):02d}"],np.array([px,pz]),size,k);count[k]+=1
    # Wind-clipped saltbrush living above the wet rock: a distinct, broad
    # tier between low strand herbs and the old alders on the backing hill.
    # Native position/raised shoulders of the original 1785 and 1619/1472
    # deposits, plus the shoulder of the actual northern head, define nuclei.
    nuclei=[(-89,19,12),(-58,22,14),(-26,21,9),(16,16,9),(32,12.6,11),
            (57,17,9),(96,19,9),(121,15.7,13),(151,17,6),(183,18,8),(202,19,5)]
    for centerz,depth,groups in nuclei:
        for j in range(groups):
            z=centerz+R.normal(0,5.7); x=b.waterx(z)+depth+R.normal(0,3.2)
            h,gx,gz=b.floor(x,z)
            if h<1.35 or math.hypot(gx,gz)>.92 or screened(x,z,.39): continue
            g=plants[f"Salt-pruned brush / {int((z+140)//27):02d}"]
            size=R.uniform(.62,1.26); base=np.array([x,z])
            tint=R.uniform(.91,1.06)
            for arm in range(R.integers(6,12)):
                theta=arm*2.39996+R.uniform(-.34,.34)
                aim=np.array([math.cos(theta)+.36,math.sin(theta)+.10]);aim/=np.linalg.norm(aim)
                lat=np.array([-aim[1],aim[0]])
                reach=size*R.uniform(.59,1.24)
                start=base+.095*size*aim
                a=start+aim*.48*reach; end=start+aim*reach
                yy=np.maximum(b.floor(*np.column_stack((start,a,end)))[0],h)+[0,.25*size,.29*size]
                stemw=.018*size
                coords=np.array([start-lat*stemw,start+lat*stemw,a-lat*stemw,a+lat*stemw,end])
                heights=yy[[0,0,1,1,2]].copy();heights[:2]-=.013
                cane=np.column_stack((coords[:,0],heights,coords[:,1]))
                g.draw(cane,np.array([(90,87,68),(103,100,76),(126,120,90),(129,130,98),(142,145,111)]),((0,1,2),(1,3,2),(2,3,4)))
                for k in range(7):
                    t=.29+.100*k; p0=start+t*reach*aim
                    height=np.interp(t,[0,.48,1],yy)
                    for side in [-1,1]:
                        length=size*R.uniform(.13,.26)*(1-.27*t)
                        w0=length*R.uniform(.20,.31); axis=(aim*.54+side*lat*.84)
                        n=np.array([-axis[1],axis[0]])
                        xy=np.array([p0,p0+axis*length*.56-n*w0,p0+axis*length*.56+n*w0,p0+axis*length*.64,p0+axis*length])
                        heights=height+np.array([0,.043,.048,.071,.088])*size
                        heights=np.maximum(heights,b.floor(xy[:,0],xy[:,1])[0]+.011)
                        petal=np.column_stack((xy[:,0],heights,xy[:,1]))
                        leaf=np.array([138,151,121])*(1+.09*t)*tint
                        g.draw(petal,np.array([leaf*.72,leaf*.96,leaf*1.045,leaf*1.1,leaf*1.05]),((0,1,3),(0,3,2),(1,4,3),(3,4,2)))
            count['saltbrush_stools']+=1
    # Drift is broken into short local hooks in genuine low-water pockets.
    for z in np.r_[np.arange(-8,71,6.7),np.arange(86,180,10.9),np.arange(193,215,6)]:
        z+=R.uniform(-2,2)
        for j in range(R.integers(3,12)):
            pz=z+R.normal(0,1.3);px=b.waterx(pz)+R.uniform(.4,1.9)
            h=b.floor(px,pz)[0]
            if not .07<h<.98:continue
            cushion(mineral[f"Stranded ribbons / {int((pz+140)//27):02d}"],np.array([px,pz]),R.uniform(.29,.81),'wrack');count['wrack']+=1
    return dict(stone_and_wrack=export(mineral,folder/'broken-beds.glb'),
                ground_colonies=export(plants,folder/'living-margins.glb'),
                counts=dict(count),max_relief_m_walkable=.038,max_relief_m_steep=.075)
