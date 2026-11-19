"""Read-only survey of a real west-facing open bight, in V1 metres."""
from pathlib import Path
import csv, hashlib, json
import numpy as np
from island_inlet_ground import Inlet, ease

HASH="8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38"
EXTENT=(-414.,-291.,-120.,234.)
TAGS=("bight-high","bight-tooth","bight-beach")

class Bight(Inlet):
    def __init__(self, path):
        if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=HASH:
            raise ValueError("The dated bight studies are bound to the original V1.")
        super().__init__(path)
        self.zz=np.arange(-125,240.01,.25)
        a=np.full_like(self.zz,-435); b=np.full_like(self.zz,-302)
        for _ in range(27):
            mid=(a+b)*.5
            dry=self.floor(mid,self.zz)[0]>=0
            a=np.where(dry,a,mid); b=np.where(dry,mid,b)
        self.wet=(a+b)*.5

    def waterx(self,z):
        return np.interp(z,self.zz,self.wet)

    def camera(self,tag):
        return frame(tag,lambda x,z:float(self.floor(x,z)[0]))

    def survey(self,props,out):
        z=np.arange(-103.,225.01,2.)
        x=self.waterx(z)
        dist=np.r_[0,np.cumsum(np.hypot(np.diff(x),np.diff(z)))]
        objects=json.loads(Path(props).read_text())
        with (out/'native-sections.csv').open('w',newline='') as f:
            w=csv.writer(f);w.writerow(['shore_length_m','native_west_shore_x','native_z','bed_y_minus_2m','bed_y_plus_3m','bed_y_plus_15m','bed_y_plus_38m'])
            for i in range(len(z)):
                w.writerow([round(v,6) for v in (dist[i],x[i],z[i],*(float(self.floor(x[i]+k,z[i])[0]) for k in (-2,3,15,38)))])
        return dict(bounds=EXTENT,terrain_sha256=HASH,wet_dry_zero_contour_m=round(dist[-1],3),
                    surveyed_z=[float(z[0]),float(z[-1])],cross_section_meaning="Eastward horizontal metre offsets from native triangle sea-level intercept at each sample; not walking clearance.",
                    landmarks={str(i):objects[i] for i in (1115,1578,1785,821,612,1619,1472,982,1705)},
                    stations={t:self.camera(t) for t in TAGS}, native_installation=False)

from stations import frame

def _h(x,z,s):
    a=np.sin(x*127.1+z*311.7+s*17.3)*43758.5453
    return a-np.floor(a)

def noise(x,z,metres,seed=0):
    X,Z=x/metres,z/metres
    i,j=np.floor(X),np.floor(Z)
    a,b=X-i,Z-j
    a=a*a*(3-2*a); b=b*b*(3-2*b)
    return (1-b)*((1-a)*_h(i,j,seed)+a*_h(i+1,j,seed))+b*((1-a)*_h(i,j+1,seed)+a*_h(i+1,j+1,seed))

def cut(z,d,z0,d0,along,across,dip=0):
    return np.sqrt(((z-z0)/along)**2+((d-d0-(z-z0)*dip)/across)**2)

def fields(b,x,z):
    d=x-b.waterx(z); h,gx,gz=b.floor(x,z); sl=np.hypot(gx,gz)
    big=noise(x,z,13.7,1)-.5;mid=noise(x,z,3.45,4)-.5; l=noise(x,z,.92,0)-.5
    # Four contacts split by actual beach coves, not another lengthwise road.
    # The high northern head exposes a wide face; the shelter picks up just
    # two separated tails of it. Their dip changes at the broken central nose.
    lenses=[(-77,9.2,47,19,-.13),(-33,13.5,36,16,-.20),(-8,1.5,19,7.4,.12),
            (21,6.0,32,8.7,-.14),(36,12,16,8.0,.23),(61,3.4,9.8,5.8,-.28),
            (91,11.8,18,10,.23),(119,9.4,16.8,11.8,.31),(140,2.6,10,5,.15),
            (160,8.0,13,7,-.20),(199,4.5,16,6.6,.18)]
    # Weathering is directional. Warp both shore-relative axes before
    # crossing the contact, let fault branches interrupt the landward edge.
    # This prevents each isolated tail reading like a grey round stone decal.
    dw=d+8.2*big+2.2*mid+1.2*np.sin(.204*z+.26*d)
    zw=z+6.8*(noise(x-2,z,8.3,19)-.5)+.17*d
    hill=np.minimum.reduce([cut(zw,dw,*p) for p in lenses])
    finger=noise(d*.91,z*.33,3.47,32)-.5
    edge=hill+.28*big+.26*mid+.105*l+.36*finger
    rock=1-ease(.77,1.03,edge)
    # Pale sediment floats back over sheets. Draw only the shoulder and high
    # fringe. The chart does not create skerry silhouettes out in open water.
    upper=15+9*noise(x,z,24,3)+7*ease(.78,1.25,sl)
    turf=ease(upper-5,upper+3,d+8*big+2.8*mid)
    toe=ease(-1.5,.6,h)*(1-ease(.49,1.95,h+mid*.45))
    coverage=ease(-3.,-.3,h)*(1-ease(32+13*big,51+8*big,d))
    coverage*=ease(-117,-99,z)*(1-ease(209,231,z))
    return d,h,sl,big,mid,l,rock,turf,toe,coverage

def colour(b,x,z):
    d,h,g,broad,mid,fine,rock,turf,wet,alpha=fields(b,x,z)
    s=noise(x,z,.27,4)-.5;mottle=noise(x,z,.08,11)-.5
    def shade(t,amount=1):
        return np.asarray(t)[None,None,:]+amount[...,None] if not np.isscalar(amount) else np.asarray(t)
    def mix(a,v,w):
        return a+(v-a)*np.clip(w,0,1)[...,None]
    sed=shade((193,175,137), broad*19+mid*12+fine*5+s*3+mottle*4)
    damp=shade((118,127,112),broad*14+fine*6)
    sed=mix(sed,damp,wet*.75)
    tur=shade((137,146,96),21*broad+16*mid+7*fine+9*s)
    # A single tooth's grey-green faces, weathered buff flecks and real joint
    # gouges. Along-strike offset changes at fractured rafts, not contour lines.
    dip=np.where(z<61,-.21,.27)
    u=d-(z*.17+2.4*np.sin(z/17))*dip+1.7*broad+.27*fine
    v=z+.74*d+3.2*broad+.7*mid
    dur=noise(u*.87,v*1.61,3.25,7)
    key=noise(u*2.0,v*.43,3.67,5)
    iron=ease(.54,.83,dur+.20*key)*(.28+.72*noise(x,z,12.2,16))
    grey=shade((119,128,119), broad*19+mid*13+fine*9+s*7+mottle*6)
    grey=mix(grey,shade((170,149,114),mid*15+fine*8),iron*.43)
    # Broken, tapering mineral cleavage rather than a tiling black grid.
    a=.177*u+.16*np.sin(u*.66)+.24*np.sin(v*.13+u*.16)+.52*noise(u,v,7.7,8)+.20*noise(u,v,1.63,36)
    joint=np.abs(a-np.rint(a))*4.4
    intensity=(1-ease(.035,.23,joint))*(.06+.53*ease(.43,.78,key))*ease(.18,.59,noise(u,v,6.0,15))
    pale=(1-ease(.08,.32,abs(joint-.42)))*ease(.55,.87,dur)
    grey=mix(grey,shade((69,79,76)),intensity)
    grey=mix(grey,shade((181,172,143)),pale*.23)
    grey=mix(grey,shade((72,98,89),9*mid),wet*(.25+.35*noise(x,z,2.5,14)))
    soil=mix(sed,tur,turf)
    # Earth shadows between honey-grey rock toes and sheltered living soil.
    shore=rock*(1-.56*turf)
    soil=mix(soil,shade((131,124,95),9*broad),ease(.02,.6,shore)*(1-ease(.68,.92,shore))*.55)
    c=mix(soil,grey,shore)
    # A separate real geology/sand/turf map; the close rock shader must not
    # put its quartz fractures into dunes or into the vegetation shoulder.
    mask=np.dstack((shore,(1-shore)*(1-turf),turf*(1-rock)))
    return (np.uint8(np.clip(np.concatenate((c,(alpha*.84*255)[...,None]),axis=2),0,255)),
            np.uint8(np.clip(mask*255,0,255)))
