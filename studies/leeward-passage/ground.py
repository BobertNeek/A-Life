"""Meadow, feldspar neck, alder shelter and storm strand on real V1 triangles."""
import math
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter
from reach import BOUNDS

def ramp(a,b,x):
    p=np.clip((x-a)/(b-a),0,1);return p*p*(3-2*p)
def locality(s,m,w):
    return np.exp(-.5*((s-m)/w)**2)
def width(s):
    return 1.4+.42*np.sin(.047*s+.13)+.20*np.cos(.137*s)+.62*locality(s,104,14)+1.30*locality(s,213,13)
def paint(reach,out):
    x0,x1,z0,z1=BOUNDS;w,h=844,1636
    z,x=np.mgrid[:h,:w];x=x0+(x+.5)*(x1-x0)/w;z=z0+(z+.5)*(z1-z0)/h
    s,d=reach.address(x,z);floor,gx,gz=reach.floor(x,z);rng=np.random.default_rng(30593482)
    def n(sc):
        f=gaussian_filter(rng.normal(size=x.shape).astype('f'),sc)
        return np.clip((f-f.mean())/max(f.std(),.00001),-3,3)
    broad=n(19);mid=n(5);fine=n(.8);grain=n(0)
    # Worn humus is not a road. Sections close over; windswept pans and old
    # ice-deposit flutes split/coalesce asymmetrically across the open swale.
    end=ramp(-3,9,s)*(1-ramp(reach.m[-1]-1,reach.m[-1]+6,s))
    zone=(1-ramp(9,25,np.abs(d+.24*broad)))*end*ramp(-.02,.55,floor)
    spine=width(s);dev=d+mid*.18+broad*.42
    tread=1-ramp(.50*spine,1.65*spine,np.abs(dev))
    fallback=(.42+.31*locality(s,102,20)+.23*locality(s,203,18)+.24*locality(s,30,12)) 
    # Broad flanks widen where soil catches on the low, dry native saddle.
    west=2.8+2.8*np.sin(.067*s)+1.3*np.sin(.114*s+3)
    seam=np.abs(d+west+.8*broad+.31*mid)
    pan=np.maximum(.0,1-ramp(.75,4.8,seam))*(.94*locality(s,107,23)+.46*locality(s,167,13))
    low=locality(s,93,27);heath=locality(s,145,27);shore=ramp(184,216,s)*ramp(.1,4,9.2-floor)
    # Instead of encircling trunks in circles, wash collects downwind of
    # three original alders on the physical bluff. Offset each wake.
    shelter=np.zeros_like(x)
    for px,pz,span in [(130.4,419.4,6.9),(132.7,424.1,8.1),(133.,444.8,8.)]:
        shelter=np.maximum(shelter,np.exp(-.5*(((x-px+2.0)/span)**2+((z-pz-1.1)/(span*.63))**2)))
    base=np.empty((*x.shape,3),'f');base[:]=[117,136,83]
    def glaze(t,ink): 
        nonlocal base
        t=np.clip(t,0,1)[...,None];base=base*(1-t)+np.array(ink,'f')*t
    glaze(.62*low,[151,152,99]);glaze(.75*heath,[132,146,100]);glaze(.82*shelter,[101,107,69])
    glaze(.92*pan,[161,154,123]);glaze(.83*shore,[161,157,127])
    swept=(.5+.5*np.sin(.81*d+.10*s+.68*broad+.16*mid))**4
    glaze((.28*low+.22*shore)*swept,[179,169,133]);glaze((.30+.42*shore)*tread,[173,149,111])
    glaze((.17+.53*shore)*tread+.61*shore*(1-ramp(.45,2.15,np.abs(dev-2.5+.6*broad))),[203,183,145])
    glaze(.35*(1-ramp(.1,.68,np.abs(np.sin(.62*s+.9*dev)+.40*mid)))*pan,[177,172,150])
    strata=.46*swept*pan+.43*shore
    base+=((2.3*broad+2.9*mid+(2.2+2*strata)*fine+(1.3+2*strata)*grain)[...,None])*[1.1,1,.71]
    a=zone*np.clip(.32+.22*low+.18*heath+.29*shore+.22*shelter+.33*pan+fallback*tread+np.clip(.058*broad,-.10,.14),.09,.96)
    rgba=np.uint8(np.clip(np.dstack((base,a*255)),0,255));Image.fromarray(rgba).save(out/'reach-wash.png')
    return {'bounds':BOUNDS,'wash_size':[w,h],'surface_method':'Pigment on existing original triangles. No second ground or displacement.'}
