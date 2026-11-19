"""Wind flags, ember samphire, sea oats, shell valves and till lenses.

A small set of editable metre-addressed massing groups. Tall growth never
occupies the clear centre; all stones are low flakes, not new obstacles.
"""
import math
from collections import defaultdict,Counter
import numpy as np
from ground import width
from finished_island_glb import save
from island_erratic_glb import streams

class Mesh:
    def __init__(self):self.xyz=[];self.col=[];self.tri=[]
    def pt(self,x,y,z,c):
        i=len(self.xyz);self.xyz.append((x,y,z));self.col.append(c);return i
    def face(self,*t):self.tri.append(t)
def approach(reach,out):
    R=np.random.default_rng(59818); tiles=defaultdict(Mesh);count=Counter()
    v=lambda a: np.array((math.cos(a),math.sin(a)))
    rot=lambda u: np.array((-u[1],u[0]))
    def y(p):return float(reach.floor(*p)[0])
    def pt(p,h,c):return g.pt(p[0],h,p[1],c)
    def tri(a,b,c):
        A,B,C=(g.xyz[i] for i in (a,b,c))
        if (B[2]-A[2])*(C[0]-A[0])-(B[0]-A[0])*(C[2]-A[2])<0:b,c=c,b
        g.face(a,b,c)
    def blade(p,angle,l,h,w,c,curve,base=None):
        u=v(angle);n=rot(u);y0=y(p)-.012 if base is None else base
        a=pt(p-w*n*.34,y0,c*.66);b=pt(p+w*n*.34,y0,c*.72)
        q=p+l*u*.22;lh=max(y(q)+.01,y0+h*.67);e=pt(q-n*w,lh,c*.91);f=pt(q+n*w,lh,c*.90);m=pt(q,lh+w*.52,c*1.025)
        tri(a,b,m);tri(a,m,e);tri(b,f,m)
        q2=p+l*(.74*u+n*curve*.40);yh=max(y(q2)+.019,y0+h);h1=pt(q2-n*w*.54,yh,c*.95);h2=pt(q2+n*w*.54,yh,c*1.02);m2=pt(q2,yh+w*.22,c*1.06)
        for k in ((e,m,m2),(e,m2,h1),(m,f,h2),(m,h2,m2)):tri(*k)
        q3=p+l*(u+n*curve);tip=pt(q3,max(y(q3)+.016,y0+h*.79),c*1.05);tri(h1,m2,tip);tri(m2,h2,tip)
    def sedge(p,size,family):
        az=R.uniform(0,6.28);ink=np.array({'reed':(157,166,112),'oat':(189,170,113),'shore':(137,159,131)}[family])*R.uniform(.83,1.08)
        for i in range(R.integers(5,10)):
            s=R.uniform(.60,1.24)*size;angle=az+i*2.4
            u=v(angle)*.63+np.array([.8,.49])*.52 # prevailing sea air; splay, not orbiting forks
            blade(p+R.uniform(-.050,.050,2)*s,math.atan2(u[1],u[0]),s*R.uniform(.53,.94),s*R.uniform(.58,.96),s*R.uniform(.032,.070),ink*R.uniform(.91,1.06),R.uniform(-.38,.38))
        if family=='oat' and R.random()<.48:
            stem(p,size,ink)
    def stem(p,size,c):
        y0=y(p)-.009;n=R.uniform(-.35,.35);dr=v(.71+n);right=rot(dr);last=p;ly=y0
        # A bent culm with drooping, paired spikelets, never grains on a rod.
        for j in range(4):
            t=(j+1)/4; q=p+dr*size*(.09*t+.39*t*t); yy=y0+size*(1.50*t-.38*t*t)
            a=pt(last-right*.005,ly,c*.86);b=pt(last+right*.005,ly,c*.89);e=pt(q-right*.0035,yy,c*1.11);f=pt(q+right*.0035,yy,c*1.09);tri(a,b,e);tri(b,e,f)
            if j>0:
                sign=1 if j%2 else -1
                for off in (0,.065):
                    bp=q-dr*size*off;bh=yy-size*off
                    axis=dr*.70+right*sign*.59;mid=bp+axis*size*.09;end=bp+axis*size*.20
                    ia=pt(bp,bh,c);ib=pt(mid-right*size*.024,bh-size*.015,c*.94);ic=pt(mid+right*size*.024,bh-size*.012,c*1.06);im=pt(mid,bh+size*.005,c*1.12);it=pt(end,bh-size*.065,c*1.12)
                    for t3 in ((ia,ib,im),(ia,im,ic),(ib,it,im),(im,it,ic)):tri(*t3)
            last,ly=q,yy
    def glasswort(p,size):
        c=np.array(R.choice([(158,119,92),(148,145,103),(168,132,107),(133,155,121)]))*R.uniform(.9,1.05)
        h=y(p)-.009;a0=R.uniform(0,6.28)
        for j in range(R.integers(5,9)):
            u=v(a0+j*2.4);n=rot(u);l=size*R.uniform(.6,1.2);q=p+u*l*.74;yy=max(y(q)+.012,h+l*.17)
            # Strong flattened three-lobed pads, some blushed from sea salt.
            for k in (-1,0,1):
                ax=u*.82+n*.67*k; e=q+ax*.32*l; q1=q-ax*.18*l
                a=pt(q1,yy,c*.81);b=pt(q+(e-q)*.42-n*l*.091,yy+l*.23,c*.9);m=pt(q+(e-q)*.45,yy+l*.32,c*1.05);f=pt(q+(e-q)*.42+n*l*.091,yy+l*.23,c*.99);t=pt(e,yy+l*.29,c*1.05)
                for face in ((a,b,m),(a,m,f),(b,t,m),(m,t,f)):tri(*face)
            blade(p,math.atan2(u[1],u[0]),l*.78,l*.21,l*.088,c,.08)
    def shard(p,size,az,shell):
        u,n=v(az),rot(v(az));c=np.array(R.choice([(208,193,166),(180,165,150),(198,178,151),(218,204,177)] if shell else [(150,151,137),(155,142,127),(167,156,140),(127,133,124)]))*R.uniform(.88,1.06)
        # Half-buried, firm triangles sampled independently at every rim
        # vertex. Valve has a fan hinge and six radial folds, not a pebble.
        floor=y(p);L=size;W=size*R.uniform(.38,.69)
        if shell:
            apex=p-u*L*.55;h=pt(apex,y(apex)+.004,c*.79);prev=None
            for j in range(11):
                a=-1.19+j*.238;q=apex+(u*math.cos(a)*L+n*math.sin(a)*W)*(1-.045*(j%2));k=pt(q,y(q)+.006,c*(.88+.08*(j%2)));m=apex+(q-apex)*.61;mg=pt(m,y(m)+L*(.12 if j%2 else .20),c*(.89 if j%2 else 1.07))
                if prev:
                    e,f=prev;tri(h,f,mg);tri(f,e,k);tri(f,k,mg)
                prev=k,mg
        else:
            nside=int(R.integers(5,8));points=[]
            for j in range(nside):
                t=j*math.tau/nside+R.uniform(-.13,.13);q=p+u*(math.cos(t)*L)*R.uniform(.77,1.12)+n*math.sin(t)*W;points.append(pt(q,y(q)-.009,c*.81))
            high=pt(p+u*.17*L,floor+min(.065,.27*L),c*1.03)
            for a,b in zip(points,points[1:]+points[:1]):tri(a,b,high)
    # Stretch each clone nucleus with wind and underlying meadow opening. The
    # opening is not outlined by two planted lines: a random ellipse may lie
    # on either shoulder, sometimes crosses the trail in low creeping growth.
    for s0 in np.sort(R.uniform(-1,reach.m[-1]+1,222)):
        near=1.3+width(s0);d0=R.uniform(near,15.0)*R.choice([-1,1],p=[.55,.45]);shelf=s0>181
        fam=R.choice(['shore','samphire','oat'],p=[.48,.37,.15]) if shelf else R.choice(['reed','oat','samphire'],p=[.48,.47,.05])
        span=R.uniform(1.15,4.2);wide=R.uniform(.42,1.65);scale=R.uniform(.26,.62) if fam!='samphire' else R.uniform(.10,.24)
        for _ in range(R.integers(4,12)):
            s=s0+np.clip(R.normal(),-2,2)*span;d=d0+R.normal()*wide
            if s<0 or s>reach.m[-1]+.8 or abs(d)<width(s)*.94 or abs(d)>19:continue
            p=np.array(reach.station(s,d));h,gx,gz=reach.floor(*p)
            if h<.75 or math.hypot(gx,gz)>.78:continue
            g=tiles[(int(max(0,s)//28),'leaves')];size=scale*R.uniform(.64,1.19)
            if fam=='samphire':glasswort(p,size)
            else:sedge(p,size,fam)
            count[fam]+=1
    # Glacial chert + later storm deposits appear in overlapping small pans,
    # not evenly spaced stones, and most of the long passage is open turf.
    for s0 in np.sort(np.r_[R.uniform(32,184,72),R.uniform(189,219,52)]):
        shore=s0>189;d0=R.normal(.0,2.1) if shore else R.normal(-2.8,2.1)
        for _ in range(R.integers(10,27)):
            s=s0+R.normal(0,1.2 if shore else 1.72);d=d0+R.normal(0,.64)
            if not 6<s<reach.m[-1]+1:continue
            p=np.array(reach.station(s,d));h,gx,gz=reach.floor(*p)
            if h<.18 or math.hypot(gx,gz)>.66:continue
            g=tiles[(int(s//28),'flakes')];is_shell=bool(shore and h<7.1 and R.random()<.36)
            shard(p,R.uniform(.045,.114) if is_shell else R.uniform(.045,.22),R.normal(.73,.65) if not is_shell else R.uniform(0,6.28),is_shell)
            count['valves' if is_shell else 'shingle']+=1
    return save_tiles(tiles,out,reach),dict(count)
def save_tiles(tiles,out,reach):
    d=dict(asset={'version':'2.0','generator':'Qualia Shellback Reach, editable source-only'},scene=0,scenes=[dict(nodes=[])],nodes=[],meshes=[],materials=[dict(name='Sun-sided rushes and worn facets',doubleSided=True,pbrMetallicRoughness=dict(metallicFactor=0,roughnessFactor=.87))],bufferViews=[],accessors=[]);buf=bytearray();attr=streams(d,buf);tot=0
    for (zone,kind),g in sorted(tiles.items()):
        if not g.tri:continue
        p=np.asarray(g.xyz,dtype='<f4');orig=(p.min(0)+p.max(0))/2;p-=orig;t=np.asarray(g.tri,dtype='<u4');n=np.zeros_like(p)
        fn=np.cross(p[t[:,1]]-p[t[:,0]],p[t[:,2]]-p[t[:,0]])
        for k in range(3):np.add.at(n,t[:,k],fn)
        n/=np.maximum(1e-9,np.linalg.norm(n,axis=1,keepdims=True))
        sr=np.clip(np.asarray(g.col)/255,0,1);lin=np.where(sr<=.04045,sr/12.92,((sr+.055)/1.055)**2.4);col=np.column_stack((np.uint8(np.rint(lin*255)),np.full(len(p),255,'B')))
        at=dict(POSITION=attr(p,5126,'VEC3',bounds=True),NORMAL=attr(n,5126,'VEC3'),COLOR_0=attr(col,5121,'VEC4',normalized=True))
        ii=5123 if len(p)<65536 else 5125;idx=attr(t.ravel().astype('<u2' if ii==5123 else '<u4'),ii,'SCALAR',indices=True)
        name=f'Reach_{zone:02d}_{kind}';l=len(d['nodes']);d['scenes'][0]['nodes'].append(l);d['nodes'].append(dict(mesh=l,name=name,translation=orig.tolist()));d['meshes'].append(dict(name=name,primitives=[dict(attributes=at,indices=idx,material=0)]));tot+=len(t)
    d['buffers']=[dict(byteLength=len(buf))];save(d,buf,out/'reach-detail.glb');return tot
