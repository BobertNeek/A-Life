"""Four fictional dry-slack species. Solid, folded silhouettes with real branching."""
import math
import numpy as np
from finished_island_glb import save
from island_erratic_glb import streams
TAU=math.tau

class Model:
    def __init__(self): self.p=[];self.c=[];self.t=[]
    def add(self,pts,colors,faces):
        k=len(self.p);self.p.extend(pts);self.c.extend(np.broadcast_to(colors,(len(pts),3)))
        self.t.extend(tuple(k+i for i in f) for f in faces)
    def blade(self,a,b,width,base,*,bow=.06,roll=.25):
        a,b=np.asarray(a),np.asarray(b);v=b-a;n=np.array([v[1],-v[0],0.]);n/=max(np.linalg.norm(n),1e-7)
        height=np.array([0,0,bow]); m=a+.48*v+height
        w=n*width; tip=a+.81*v+height*.57
        pts=[a,m-w,m+height*.30,m+w,tip-w*.55,tip+height*.1,tip+w*.55,b]
        c=np.array(base)*np.array([.55,.87,1.04,1+roll,.98,1.12,1+roll,1.05])[:,None]
        self.add(pts,c,[(0,2,1),(0,3,2),(1,2,4),(2,5,4),(2,3,6),(2,6,5),(4,5,7),(5,6,7)])
    def rod(self,pts,widths,base,sides=4):
        ring=[]; color=[]; fs=[]; p=np.array(pts)
        for j,(cen,width) in enumerate(zip(p,widths)):
            v=p[min(j+1,len(p)-1)]-p[max(j-1,0)];v/=max(np.linalg.norm(v),1e-9)
            n=np.cross(v,[1,0,.13]);n/=max(np.linalg.norm(n),1e-9); w=np.cross(v,n)
            for i in range(sides):
                th=TAU*i/sides;ring.append(cen+width*(n*math.cos(th)+w*math.sin(th)))
                color.append(np.array(base)*(.74+.20*j/max(len(p)-1,1)+.10*math.cos(th)))
            if j:
                for i in range(sides):
                    k=(j-1)*sides+i;o=(j-1)*sides+(i+1)%sides
                    fs.extend([(k,o,k+sides),(o,o+sides,k+sides)])
        self.add(ring,color,fs)
    def spindle(self,cen,r,h,th,cream):
        p=[];c=[];f=[];n=6
        for j,(rr,zz) in enumerate([(.24,-.51),(1.,-.19),(.73,.2),(.05,.56)]):
            for i in range(n):
                t=th+i*TAU/n
                p.append(cen+[math.cos(t)*rr*r+.14*r*zz,math.sin(t)*rr*r,zz*h])
                c.append(np.array(cream)*([.68,.90,1.04,1.14][j])*(.95+.06*math.cos(t*3)))
                if j:
                    a=(j-1)*n+i;b=(j-1)*n+(i+1)%n;f.extend([(a,b,b+n),(a,b+n,a+n)])
        self.add(p,c,f)
    def crown(self,center,angle,size,R):
        # Three recurved pearl bracts wrap the amber seed. They grow apart
        # at the tip: this is not a billboarded flower or spherical lamp.
        self.spindle(center+[0,0,.021*size],.025*size,.092*size,angle,(193,140,62))
        for i in range(3):
            a=angle+i*TAU/3;v=np.array([math.cos(a),math.sin(a),0]);n=np.array([-v[1],v[0],0])
            H=.083*size;W=.036*size;t=R.uniform(.87,1.12)
            p=[center-[0,0,.022*size],center+v*.036*size-n*W+[0,0,.027*size],center+v*.049*size+[0,0,.023*size],center+v*.036*size+n*W+[0,0,.027*size],center+v*.047*size-n*W*.61+[0,0,H*.74],center+v*.056*size+[0,0,H*.73],center+v*.047*size+n*W*.61+[0,0,H*.74],center+v*.026*size+[0,0,H*t]]
            colors=np.array([[116,132,102],[168,163,139],[205,199,162],[177,163,143],[219,212,182],[234,224,181],[199,184,158],[241,227,186]])
            self.add(p,colors,[(0,1,2),(0,2,3),(1,4,2),(4,5,2),(2,5,6),(2,6,3),(4,7,5),(5,7,6)])

def plant(kind,v,detail):
    R=np.random.default_rng(2231+31*v+851*['milkspindle','copper-rush','silver-mat','windwillow'].index(kind))
    g=Model();base=np.array([0.,0.,-.039]);wind=np.array([.55,-.25,0.])
    def axis(th):return np.array([math.cos(th),math.sin(th),0.])
    if kind=='windwillow':
        for j in range(7 if detail else 5):
            th=j*TAU/7+.5*v+R.uniform(-.29,.29);aim=axis(th);span=R.uniform(.44,.94);hi=R.uniform(.39,.77)
            root=base+aim*.043;elbow=.31*span*aim+.1*wind+[0,0,.29*hi];end=aim*span+wind*.11+[0,0,hi]
            g.rod([root,elbow,end],[.018,.012,.005],[143,117,81])
            for i in range(5 if detail else 3):
                f=.24+i*(.145 if detail else .27);pivot=elbow+(end-elbow)*f
                for side in [-1,1]:
                    w=axis(th+side*R.uniform(.57,1.14)); length=R.uniform(.15,.295)*(1-.22*f);tip=pivot+w*length+[0,0,R.uniform(.035,.14)]
                    if detail:g.rod([pivot,tip],[.0061,.0024],[124,129,92],3)
                    for l in range(3 if detail else 2):
                        start=pivot+(tip-pivot)*(.27*l);vec=axis(th+side*(.65+.41*l));t=R.uniform(.085,.159)*(1 if detail else 1.30)
                        g.blade(start,start+vec*t+[0,0,.063],t*.25,np.array([143,159,126])*(.87+.16*f),bow=.024,roll=.11)
            if detail and j%2==0:g.spindle(end+[0,0,.01],.019,.079,th,[176,148,93])
    elif kind=='silver-mat':
        for j in range(16 if detail else 9):
            th=j*2.39996+R.uniform(-.21,.21);aim=axis(th);reach=R.uniform(.17,.42);tip=aim*reach+[0,0,R.uniform(.035,.17)]
            g.blade(base+aim*.017,tip,reach*R.uniform(.11,.17),np.array([148,167,144])*R.uniform(.91,1.10),bow=R.uniform(.036,.085),roll=.15)
            if detail and j%5==0:g.blade(base+aim*reach*.38+[0,0,.060],tip+axis(th+.7)*.061,reach*.042,[165,179,150],bow=.019)
    else:
        milk=kind=='milkspindle'
        for j in range((14 if detail else 8) if milk else (22 if detail else 13)):
            th=j*2.39996+R.uniform(-.30,.30); vec=axis(th)
            radius=R.uniform(.16,.45);height=R.uniform(.09,.40) if milk else R.uniform(.13,.46)
            tip=vec*radius+wind*.066+[0,0,height]
            col=np.array([107,144,115] if milk else [154,135,81])*R.uniform(.80,1.17)
            g.blade(base+vec*.019,tip,R.uniform(.022,.038) if milk else R.uniform(.009,.017),col,bow=R.uniform(.03,.09),roll=.18)
        for j in range((4 if detail else 3) if milk else (8 if detail else 5)):
            th=j*2.39996+v*.74+R.uniform(-.21,.21);aim=axis(th)
            h=R.uniform(.37,.70) if milk else R.uniform(.40,.82);reach=R.uniform(.08,.31)
            root=base+aim*.009;mid=aim*reach*.31+[0,0,h*.64];end=aim*reach+wind*.103+[0,0,h]
            g.rod([root,mid,end],[.008,.006,.0035] if milk else [.005,.004,.0021],[132,152,109] if milk else [163,130,79],3)
            if milk:
                g.crown(end,th,R.uniform(.77,1.17),R)
                if j==1 and detail:
                    a=mid+(end-mid)*.61;b=a+axis(th+.66)*.087+[0,0,.052];g.rod([a,b],[.004,.0028],[142,148,106],3);g.crown(b,th+.7,.65,R)
            else:
                for k in range(3 if detail else 2):
                    s=.25+k*.25;o=mid+(end-mid)*s;vec=axis(th+(.72 if k%2 else -.76));tip=o+vec*(.093-.014*k)+[0,0,.026]
                    if detail:g.rod([o,tip],[.003,.0017],[171,134,81],3)
                    g.spindle(tip,.012 if detail else .017,.057,th,[185,135,82] if k%2 else [207,162,94])
    return g

def distant(kind,v):
    """Whole plant outline expressed in folded opaque clusters, never alpha quads."""
    r=np.random.default_rng(9123+v*7)
    g=Model();B=np.array([0,0,-.039])
    if kind=='windwillow':
        for i in range(7):
            a=2.39996*i+v*.3;d=np.array([math.cos(a),math.sin(a),0]);N=np.array([d[1],-d[0],0])
            h=r.uniform(.4,.70); span=r.uniform(.5,.90); tip=d*span+[.06,0,h];m=(tip+B)*.52+[0,0,.10]
            leaf=np.array([145,158,120])*r.uniform(.83,1.05)
            g.add([B,m-N*.115,m+[0,0,.08],m+N*.13,tip-N*.084,tip+N*.09,tip+d*.08],leaf*np.array([.48,.77,1.05,1.10,.91,1.06,1.12])[:,None],[(0,2,1),(0,3,2),(1,2,4),(2,5,4),(3,5,2),(4,5,6)])
    else:
        milk=kind=='milkspindle';cu=kind=='copper-rush'
        for i in range(6 if cu else 5):
            a=i*2.4+v*.47;d=np.array([math.cos(a),math.sin(a),0]);N=np.array([d[1],-d[0],0])
            span=r.uniform(.22,.41);h=r.uniform(.10,.36) if (milk or cu) else r.uniform(.03,.13)
            tip=d*span+[0,0,h];m=d*(span*.54)+[0,0,h*.74+.035]
            w=.029 if cu else .048 if milk else .060
            col=np.array([146,130,79] if cu else [122,147,115] if milk else [153,170,140])
            g.add([B,m-N*w,m+N*w,m+[0,0,.03],tip],col*np.array([.6,.88,1.10,1.04,1.08])[:,None],[(0,1,3),(0,3,2),(1,4,3),(3,4,2)])
            if i<3 and (cu or milk):
                tip=d*r.uniform(.09,.23)+[.055,0,r.uniform(.44,.69)];w=.010 if cu else .016
                g.add([B-N*.011,B+N*.011,tip+N*.008,tip-N*.008],[145,140,91],[(0,1,2),(0,2,3)])
                col=[205,154,93] if cu else [218,209,170];o=.025 if cu else .047
                g.add([tip+[0,0,-.035],tip-N*o,tip+d*o,tip+N*o,tip+[0,0,.066]],np.array(col)*np.array([.65,.9,1.07,.95,1.09])[:,None],[(0,1,2),(0,2,3),(1,4,2),(2,4,3)])
    return g

def write(out,rows,detail):
    doc=dict(asset=dict(version='2.0',generator='Sunspindle / dry eastern slack'),scene=0,scenes=[dict(nodes=[])],nodes=[],meshes=[],bufferViews=[],accessors=[],materials=[dict(name='Dry slack / cuticle, awn and pearl',doubleSided=True,pbrMetallicRoughness=dict(metallicFactor=0.,roughnessFactor=.77))],extensionsUsed=['EXT_mesh_gpu_instancing'],extensionsRequired=['EXT_mesh_gpu_instancing'])
    buf=bytearray();emit=streams(doc,buf);total=0;visible_tri=0
    for name in sorted({r[0] for r in rows}):
        base,level=name.split(' / ') if ' / ' in name else (name,detail);kind,v=base.rsplit('-',1);g=plant(kind,int(v),True) if level=='near' else distant(kind,int(v));p=np.asarray(g.p,dtype='<f4')[:,[0,2,1]].copy()
        # x,z,height author coordinates have opposite handedness to native x,y,z.
        t=np.asarray(g.t,dtype='<u2')[:,[0,2,1]].copy();c=np.clip(np.asarray(g.c)/255,0,1)
        nn=np.zeros_like(p);fac=np.cross(p[t[:,1]]-p[t[:,0]],p[t[:,2]]-p[t[:,0]])
        for k in range(3):np.add.at(nn,t[:,k],fac)
        nn/=np.maximum(1e-10,np.linalg.norm(nn,axis=1,keepdims=True))
        lin=np.where(c<=.04045,c/12.92,((c+.055)/1.055)**2.4)
        rgba=np.column_stack((np.uint8(np.rint(255*lin)),np.full(len(p),255,'B')))
        att=dict(POSITION=emit(p,5126,'VEC3',bounds=True),NORMAL=emit(nn,5126,'VEC3'),COLOR_0=emit(rgba,5121,'VEC4',normalized=True))
        m=len(doc['meshes']);doc['meshes'].append(dict(name=name,primitives=[dict(attributes=att,indices=emit(t.ravel(),5123,'SCALAR',indices=True),material=0)]))
        rws=np.asarray([r[1:] for r in rows if r[0]==name],dtype='<f4');scl=rws[:,3];rot=rws[:,4]
        attrs=dict(TRANSLATION=emit(rws[:,:3],5126,'VEC3',bounds=True),SCALE=emit(np.column_stack((scl,scl,scl)),5126,'VEC3'),ROTATION=emit(np.column_stack((rot*0,np.sin(rot/2),rot*0,np.cos(rot/2))).astype('<f4'),5126,'VEC4'))
        doc['nodes'].append(dict(name=name,mesh=m,extensions={'EXT_mesh_gpu_instancing':{'attributes':attrs}}));doc['scenes'][0]['nodes'].append(m)
        total+=len(t);visible_tri+=len(rws)*len(t)
    doc['buffers']=[dict(byteLength=len(buf))];save(doc,buf,out)
    return dict(specimens=len(doc['meshes']),unique_triangles=total,triangles_over_all_instances=visible_tri)
