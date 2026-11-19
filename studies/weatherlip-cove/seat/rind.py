"""A closed mineral skin for two real erratics, using Weatherlip's own grain."""
from pathlib import Path
import numpy as np
from PIL import Image
from scipy.ndimage import map_coordinates
from island_erratic_rind import paint as granite, SIDES,AXES,MARGIN,unit

def paint(folder,coast,native_minerals):
    folder.mkdir(exist_ok=True,parents=True); tile=256
    # The existing closed granite chart supplies irregular quartz and tiny
    # crustose lichen across seams. Evaluate the cove's selected quartz
    # fabric in three projections instead of copying a sand decal to a rock.
    stock=folder.parent/'donor-atlas';granite(stock,tile)
    read=lambda p:np.asarray(Image.open(p).convert('RGB')).astype(float)/255.
    al=read(stock/'granite.png'); normal=read(stock/'granite-normal.png')*2-1; orm=read(stock/'granite-orm.png')
    cove=read(coast/'rind-pigment.png').mean(-1)-.51
    original=read(native_minerals/'fracture.png').mean(-1)
    original-=np.median(original)
    relief=read(coast/'rind-relief.png')[:,:,0]-.50
    yy,xx=(np.mgrid[:tile,:tile]+.5)/tile
    for i,(axis,sgn) in enumerate(SIDES):
        a,b=AXES[i];q=np.zeros((tile,tile,3));q[:,:,axis]=sgn;q[:,:,a]=(xx-.5)*2*MARGIN;q[:,:,b]=(.5-yy)*2*MARGIN;q=unit(q)
        w=np.abs(q)**3;w/=w.sum(2)[:,:,None]
        def material(src,scale=2.17):
            layers=[]
            for ia,(i0,i1) in enumerate(((2,1),(0,2),(0,1))):
                v=scale*q[:,:,i1]+.18*scale*q[:,:,i0]+.27*ia;u=scale*q[:,:,i0]-.18*scale*q[:,:,i1]-.39*ia
                layers.append(map_coordinates(src,np.array([v*src.shape[0],u*src.shape[1]]),order=1,mode='grid-wrap'))
            return (np.stack(layers,-1)*w).sum(-1)
        f=material(cove);h=material(relief)
        old=material(original,.76)
        # Keep a side of the stone with substantial, angled original fissures.
        # A uniformly skin-smoothed Q atlas rounds the whole mass into putty.
        exposed=np.clip(.62+.43*q[:,:,1]-.20*q[:,:,2],.23,.96)
        large=np.clip(1+2.25*old*exposed,.60,1.21)
        y,x=divmod(i,3);reg=np.s_[y*tile:(y+1)*tile,x*tile:(x+1)*tile]
        al[reg]=np.clip(al[reg]*large[:,:,None]*(1+f[:,:,None]*.76)+np.maximum(f,0)[:,:,None]*[.034,.045,.03],0,1)
        hz,hx=np.gradient(h);n=normal[reg].copy();n[:,:,0]-=hx*.78;n[:,:,1]+=hz*.78;normal[reg]=unit(n)
        # Full white occlusion: only the real supported foot may cast a
        # contact shadow. The opaque material has no darkened shadow sticker.
        orm[reg][:,:,0]=1.;orm[reg][:,:,1]=np.clip(orm[reg][:,:,1]-f*.09,.44,.96);orm[reg][:,:,2]=0.
    for n,x in [('fracture',al),('fracture-normal',normal*.5+.5),('fracture-orm',orm)]:
        Image.fromarray(np.uint8(np.rint(np.clip(x,0,1)*255))).save(folder/(n+'.png'))
