"""Eastern dry slack. An unmodified V1 profile selects the colour and plants."""
import csv, hashlib, json, math, struct
from pathlib import Path
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter1d
from island_inlet_ground import Inlet, ease

SHA = '8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38'
BOUNDS = (290., 372., -111., 18.)

def noise(x,z,m,s):
    x,z=x/m,z/m
    i,j=np.floor(x),np.floor(z)
    a,b=x-i,z-j
    a,b=a*a*(3-2*a),b*b*(3-2*b)
    def h(i,j):
        q=np.sin(i*123.7+j*311.9+s*78.31)*43751.54
        return q-np.floor(q)
    return (1-b)*((1-a)*h(i,j)+a*h(i+1,j))+b*((1-a)*h(i,j+1)+a*h(i+1,j+1))

class Hollow(Inlet):
    def __init__(self,src):
        raw=Path(src).read_bytes()
        if hashlib.sha256(raw).hexdigest()!=SHA: raise ValueError('Only the held V1 surface.')
        super().__init__(src)
        self.sections=np.arange(-112.,19.,.5)
        probe=np.arange(305.,360.01,.125)
        x,z=np.meshgrid(probe,self.sections)
        hh=self.floor(x,z)[0]; mi=hh.argmin(axis=1)
        self.axis=probe[mi]; self.low=hh[np.arange(len(mi)),mi]
        # The lowest samples of the original triangulation stay available for
        # the receipt. Only the *botanical* response is gently interpolated.
        self.soft=gaussian_filter1d(self.axis,6.)
        w,d,_,_,_,n=struct.unpack_from('<IIfffI',raw,8)
        all_boxes=np.frombuffer(raw,'<f4',6*n,32+4*w*d).reshape(n,6)
        x0,x1,z0,z1=BOUNDS
        self.boxes=all_boxes[(all_boxes[:,0]<x1+2)&(all_boxes[:,2]>x0-2)&
                             (all_boxes[:,1]<z1+2)&(all_boxes[:,3]>z0-2)]
    def blocks(self,x,z,pad):
        b=self.boxes
        return np.any((x>b[:,0]-pad)&(x<b[:,2]+pad)&(z>b[:,1]-pad)&(z<b[:,3]+pad))
    def masks(self,x,z):
        h,gx,gz=self.floor(x,z); w=x-np.interp(z,self.sections,self.soft)
        rise=h-np.interp(z,self.sections,self.low)
        n=noise(x,z,11,4)-.5; n2=noise(x,z,3.7,7)-.5; dk=noise(x,z,1.45,13)-.5
        # Three joined but not concentric communities; none is an
        # isocontour stripe. A real dry eastern dune shelters the lowest sedge.
        rag=4.8*n+2.0*n2+.75*dk
        outline=np.sqrt(((w+3+.12*(z+40)+rag)/32)**2+((z+47+3*n2)/64)**2)
        cover=(1-ease(.81,1.05,outline))*ease(-.02,.25,h)
        ribbon=w+2.4+3.7*np.sin(z*.069)+2.15*np.sin(z*.126+.8)+rag
        slit=np.exp(-(.42*ribbon)**2)*(.3+.7*ease(.26,.65,noise(x,z,14.8,30)))
        # Sediment fans and airier intervals run across the slack, so it is
        # not lined with continuous equidistant carpets or a footpath.
        lilac=0.; russet=0.; chalk=0.
        def lens(z0,w0,along,across,tilt):
            return np.exp(-1.50*(((z-z0+2*n2)/along)**2+((w-w0-(z-z0)*tilt+rag)/across)**2))
        for v in [(-94.,-5,12,11,.34),(-67.,3,17,11,-.23),(-45.,-8,9,15,.32),(-21.,1,13.6,8,-.36),(-6,-6,9.8,11,.45)]:lilac+=lens(*v)
        for v in [(-89.,5,12,14,-.42),(-73.,-12,20,10,.24),(-52.,7,14,7.6,.25),(-35.,-7,14,12,-.45),(-11.,5,14,9.8,.41),(2.,-9,11,8.2,.22)]:russet+=lens(*v)
        for v in [(-88.,-18,10,8,-.25),(-56.,-12,9,14,-.42),(-37.,3,10,8,.41),(-20.,-11,14,7.8,-.23),(-2.,2,8,6,.38)]:chalk+=lens(*v)
        damp=(1-ease(.12,1.8,rise+.32*n2))*(.44+.55*noise(x,z,15.7,6))
        return dict(cover=cover,w=w,rise=rise,lilac=np.clip(lilac,0,1),russet=np.clip(russet,0,1),chalk=np.clip(chalk,0,1),slit=slit,damp=damp,n=n,n2=n2,dk=dk)
    def paint(self,out):
        x0,x1,z0,z1=BOUNDS
        W,H=820,1290; im=np.zeros((H,W,4),'B')
        for j in range(0,H,40):
            x,z=np.meshgrid(x0+(np.arange(W)+.5)*(x1-x0)/W,z0+(np.arange(j,min(j+40,H))+.5)*(z1-z0)/H)
            f=self.masks(x,z)
            def colour(c,v):return np.asarray(c)+v[...,None]
            def blend(c,a,t):return c+(a-c)*np.clip(t,0,1)[...,None]
            n,n2,dk=f['n'],f['n2'],f['dk']
            small=noise(x*.95,z*1.24,.38,18)-.5
            sand=colour([178,166,120],15*n+11*n2+6*small)
            turf=colour([139,151,96],21*n+15*n2+9*dk+7*small)
            c=blend(turf,sand,np.clip(.33+f['rise']*.19+n*.22,0,.77))
            rush=colour([107,125,94],16*n+10*n2+7*small)
            c=blend(c,rush,np.clip(f['damp']*.47+f['lilac']*.55,0,.84))
            c=blend(c,colour([160,133,86],13*n2+13*dk+5*small),f['russet']*.78)
            c=blend(c,colour([207,188,140],12*n2+7*small),f['chalk']*.83)
            c=blend(c,colour([109,110,85],7*small),f['slit']*.57)
            grain=noise(x*1.12,z*.73,.11,13)-.5
            # Tangled dry sheaths and loam aggregates, not an RGB noise overlay.
            ribs=np.abs(np.sin((z+.48*x)*6.8+3.2*dk+1.6*n2+.68*np.sin(x*3.92+z*.42)))
            litter=(1-ease(.05,.22,ribs))*ease(.35,.71,noise(x*.65,z*1.8,1.65,29))
            c+=((1.8+1.5*f['damp'])*grain-6*litter+3.6*small)[...,None]
            alpha=.92*f['cover']
            im[j:j+len(z)]=np.uint8(np.clip(np.dstack((c,alpha*255)),0,255))
        Image.fromarray(im).save(out/'hollow-ground.png',compress_level=7)
    def record(self,out):
        with (out/'sections.csv').open('w',newline='') as file:
            q=csv.writer(file);q.writerow(['z','lowest_x','lowest_y','west_18m_y','east_18m_y'])
            for k in range(0,len(self.sections),4):
                x,z=self.axis[k],self.sections[k]
                q.writerow([round(float(a),5) for a in [z,x,self.low[k],self.floor(x-18,z)[0],self.floor(x+18,z)[0]]])

def community(b,props):
    R=np.random.default_rng(70095);rows=[]
    pp=json.loads(Path(props).read_text())
    landmarks={str(i):pp[i] for i in (1284,1311,1400,1454,1648,1253,222)}
    # Broad established colonies use the actual low axis, not the camera.
    z=R.uniform(-108,13,2700);x=np.interp(z,b.sections,b.soft)+R.uniform(-37,28,len(z))
    f=b.masks(x,z)
    for i,(px,pz) in enumerate(zip(x,z)):
        if R.random()>f['cover'][i] or f['slit'][i]>.88:continue
        b0=f['lilac'][i]; b1=f['russet'][i];b2=f['chalk'][i]
        ph=noise(px,pz,5.3,35)
        choices=np.array([.125*b0**1.55+.03*f['damp'][i],.16*b1**1.60,.055*b2**1.6,.037*max(0,ph-.24)**1.3])
        k=R.random()
        if k>=choices.sum():continue
        typ=int(np.searchsorted(np.cumsum(choices),k))
        kind=('milkspindle','copper-rush','silver-mat','windwillow')[typ]
        family=R.integers([5,7,9,2][typ],[15,23,27,6][typ]);span=R.uniform(.7,2.25)
        for _ in range(family):
            xx,zz=np.array([px,pz])+R.normal(0,[1.1,1.5])*span
            yy,gx,gz=b.floor(xx,zz)
            size=R.uniform(.54,1.16)*(1. if typ!=3 else .98)
            # All sculpture is non-rigid plant tissue. Leave existing native
            # solid footprints and a full worst-case crown radius unplanted.
            if math.hypot(gx,gz)>.48 or yy<=.35 or b.blocks(xx,zz,size*[.49,.57,.61,1.11][typ]+.06):continue
            if b.masks(xx,zz)['cover']<.11:continue
            v=int(R.integers(3));ang=R.uniform(-math.pi,math.pi)
            rows.append([kind+'-'+str(v+1),round(xx,5),round(float(yy),6),round(zz,5),round(size,5),round(ang,5)])
    return rows,landmarks
