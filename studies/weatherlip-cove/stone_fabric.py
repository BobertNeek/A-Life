"""World-metre mineral fabric and small seams in an actual grounded pavement."""
import numpy as np
from scipy.ndimage import gaussian_filter
from scipy.spatial import cKDTree
from PIL import Image

def fabric(folder):
    # Tileable physical reference field: quartz blebs, shell pinholes, planes
    # of alternating broken grit. Periodicity is suppressed at use by two
    # differently rotated world scales and the actual nonperiodic site mask.
    R=np.random.default_rng(153728);N=512
    y,x=np.mgrid[0:N,0:N]/N
    def n(sig):
        q=gaussian_filter(R.normal(size=(N,N)),sig,mode='wrap');return np.clip((q-q.mean())/q.std(),-2.5,2.5)
    broad=n(18);pen=n(2.8);spots=n(.55);m=n(5.3)
    theta=2*np.pi*(2*x+y)+.46*broad+.20*pen
    lam=np.clip((np.cos(theta)-.53)*2.2,0,1)*np.clip(m*.52+.34,0,.9)
    seeds=R.uniform(0,1,(95,2));tree=cKDTree(seeds,boxsize=1.0)
    # A bent crack lattice with long gaps. Fewer than a third of boundaries
    # survive the mineral fill; do not make a uniformly crazed imitation.
    points=np.stack(((x+.006*broad+.003*pen)%1,(y+.009*m)%1),axis=-1)
    dist,ix=tree.query(points,k=2)
    near=dist[:,:,1]-dist[:,:,0]
    fade=np.clip((m*.4+broad*.31+.38),0,1)
    split=(1-np.clip((near-.0013)/.0065,0,1))*fade
    rim=np.exp(-((near-.0085)/.0036)**2)*fade*.52
    quartz=np.clip((n(1.35)-1.51)*1.3,0,1)
    pits=np.clip((-n(.95)-1.55)*1.35,0,1)
    v=np.clip(.53+.047*broad+.032*pen+.033*spots+.08*lam-.37*split+.15*rim+.15*quartz-.10*pits,.16,.92)
    warmth=np.clip(.55+.17*broad+.15*pen,0,1);h=np.clip(.49+.047*pen+.040*spots+.036*lam-.35*split+.09*rim+.08*quartz-.13*pits,.08,.9)
    p=np.stack((v+.027*warmth,v+.013*warmth,v-.019*warmth),axis=-1)
    Image.fromarray(np.uint8(p.clip(0,1)*255)).save(folder/'rind-pigment.png')
    Image.fromarray(np.uint8(h*255)).save(folder/'rind-relief.png')

def clip(poly,n,bound):
    v=[]
    for a,c in zip(poly,np.roll(poly,-1,axis=0)):
        ha=np.dot(a,n)-bound;hc=np.dot(c,n)-bound
        if ha<=0:v.append(a)
        if (ha<=0)!=(hc<=0):v.append(a+(c-a)*(ha/(ha-hc)))
    return np.asarray(v)

def pavement(b,groups,counts):
    """An actual single cropped rock face cracked into adjacent, dipping slabs.
    All sub-blocks share the parent's irregular border and split planes."""
    from shore import fields
    R=np.random.default_rng(852351)
    z=R.uniform(-102,213,540);d=R.uniform(.4,20,540);x=b.waterx(z)+d
    _,h,g,_,_,_,rock,_,_,a=fields(b,x,z)
    for i in range(len(x)):
        if rock[i]*a[i]<.62 or h[i]<.12 or R.random()>.37:continue
        X,Z=x[i],z[i];S=R.uniform(.58,1.32)
        depth=R.uniform(.58,1.04)*S;length=R.uniform(1.5,3.4)*S
        ang=R.normal(1.16 if Z<60 else 1.56,.16)
        plane=np.array([[np.cos(ang),np.sin(ang)],[-np.sin(ang),np.cos(ang)]])
        border=np.array([[-.88,-.27],[-.64,-.71],[-.10,-1],[.56,-.73],[.96,-.15],[.77,.32],[.22,.81],[-.28,.90],[-.70,.62],[-.96,.13]]) * R.uniform(.83,1.10,(10,1))
        seeds=np.array([[-.58,.14],[-.36,-.51],[.02,.46],[.20,-.37],[.58,.17]])+R.uniform(-.14,.14,(5,2))
        mat=groups[f"Fissured pavement / {int((Z+140)//27):02d}"]
        palette=np.array([131,135,124])+R.uniform(-8,8)
        for j,seed in enumerate(seeds):
            poly=border.copy()
            for k,other in enumerate(seeds):
                if k==j:continue
                n=other-seed;poly=clip(poly,n,np.dot(n,(seed+other)*.5))
                if len(poly)<3:break
            if len(poly)<3:continue
            cen=np.mean(poly,axis=0);poly=cen+(poly-cen)*R.uniform(.92,.974)
            P=np.array([X,Z])+((poly * [length*.49,depth*.67])@plane)
            count=len(P);ip=cen+(poly-cen)*.92
            Q=np.array([X,Z])+((ip * [length*.49,depth*.67])@plane)
            rise=R.uniform(.014,.033) if g[i]<=.85 else R.uniform(.03,.063)
            up=np.clip(.48+.32*ip[:,1]-.18*ip[:,0]+.10*R.random(count),.12,1)*rise
            roof=np.column_stack((Q[:,0],b.floor(Q[:,0],Q[:,1])[0]+up,Q[:,1]))
            low=np.column_stack((P[:,0],b.floor(P[:,0],P[:,1])[0]-.014,P[:,1]))
            # Separate perimeter normals from top plane, keeping the joint edge
            # readable even when the entire header stands just 2–3 cm proud.
            mat.draw(np.row_stack((low,roof)),np.row_stack(([palette*.56]*count,[palette*.86]*count)),[(p,q,r) for t in range(count) for p,q,r in ((t,(t+1)%count,(t+1)%count+count),(t,(t+1)%count+count,t+count))])
            C=Q.mean(0); ch=float(b.floor(*C)[0]+up.mean()+rise*.08)
            mat.draw(np.row_stack((roof,[C[0],ch,C[1]])),np.row_stack(([palette*R.uniform(.94,1.06)]*count,palette*1.02)),[(t,(t+1)%count,count) for t in range(count)])
            counts['interlocking_bed_faces']+=1
