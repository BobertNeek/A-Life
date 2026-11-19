"""One physically located sedimentary connection at native Cliff #1695.
Preserve the old footprint/top. Build three stepped *connected* rock benches
inside it; continue their common geological dip optically onto untouched V1
triangles. The optical material is not additional walkable/physical relief.
"""
from pathlib import Path
import json,sys,math
import numpy as np
from PIL import Image
H=Path(__file__).resolve().parent;sys.path[:0]=[str(H.parents[1]/"scripts"),str(H.parent/"lanternback-rim")]
from island_inlet_ground import Inlet,ease
from strata import nz
ID=1695
BOX=(39.7,80.7,-371.8,-337.8)
def axes(land,q,X,Z):
    x,y,z=q['position'];_,dx,dz=land.floor(x,z)
    D=np.array([-float(dx),-float(dz)])/math.hypot(dx,dz);C=np.array([D[1],-D[0]])
    return (X-x)*D[0]+(Z-z)*D[1],(X-x)*C[0]+(Z-z)*C[1]
def dip(D,C):
    # Common oblique structure: drops 1.06m for each actual downslope metre
    # and cambers laterally. The ground falls an actual ~2m/m for a while.
    return -1.06*D+.32*C + .39*(nz(D,C,4.8,752)-.5)+.12*(nz(D,C,1.09,215)-.5)
def seats(land,q,X,Z,low,edge):
    D,C=axes(land,q,X,Z);s=q['scale'];base=q['position'][1]
    h=dip(D,C)
    # From uphill to downhill the cap, long middle shoulder and low toe all
    # use the same dip. Their terminations are NOT parallel: the front of
    # the second bench forks around the original oblique rectangle.
    broken=.41*(nz(D,C,1.73,402)-.5)
    outlines=(
        (D+.26*C-(-.58+.61*np.tanh(C*1.19))+broken,6.55,.85),
        (D-.27*C-(1.84-.45*np.tanh((C+.66)*1.92))+.57*broken,2.05,.62),
        (D-.15*C-(3.92+.31*np.tanh((C-.31)*1.3))-.62*broken,-1.57,.79),
    )
    result=low.copy();influence=np.zeros_like(D)
    for i,(across,rise,front) in enumerate(outlines):
        wid=[3.42,4.2,4.75][i]
        lateral=(1-ease(wid*.64,wid+.52,abs(C+[.34,-.19,.51][i])))
        launch=(1-ease(-front*.44,front,across))*lateral*edge
        absolute=(h+rise+.36*(nz(D,C,2.29,510+i)-.5))/s
        result=np.maximum(result,low+np.maximum(0,absolute-low)*launch)
        influence=np.maximum(influence,launch)
    # The true top is still 12 local metres. Distribute the last correction
    # across top-cap interior so nobody manufactures a single point spike.
    peaks=(1-ease(-.59,.37,outlines[0][0]))*edge
    valid=peaks>.01
    lift=np.min((12.-result)[valid]/peaks[valid])
    return result+lift*peaks
def lithology(land,q,X,Y,Z):
    D,C=axes(land,q,X,Z);base=q['position'][1];h=Y-base-dip(D,C)
    # Three discontinuous mineral bed contacts that cross BOTH true surfaces.
    # The pale lip at h=-1.6 crosses the actual native ground ~3m downhill;
    # deeper contacts are exposed where the original incline rolls at 7-12m.
    broad=nz(D,C,4.80,620)-.5;mid=nz(D,C,1.09,644)-.5
    focus=(1-ease(6.2,14.4,abs(C+1.1+.23*D)))*(1-ease(13.,22.,abs(D-4.2)))
    rock=np.zeros_like(D);pale=np.zeros_like(D);cut=rock.copy();rust=rock.copy()
    for j,(centre,width,peak,left,right) in enumerate(((-1.62,1.65,-.34,-11.,7.4),(-4.82,1.00,.59,-6.9,10.2),(-8.08,1.29,-.73,-9.8,5.3))):
        dist=h-centre+.24*mid
        taper=ease(left,left+2.4,C)*(1-ease(right-3.05,right,C))
        surface=(1-ease(.12*width,1.65*width,abs(dist-.32*width)))*taper
        discontinuous=.43+.57*ease(.08,.65,.54+.97*broad+.54*mid+.22*np.tanh(peak+C*.24))
        rock=np.maximum(rock,(1-ease(.4*width,2.3*width,abs(dist)))*taper)
        pale=np.maximum(pale,surface*discontinuous)
        cut=np.maximum(cut,(1-ease(.085,.41,abs(dist+.68*width)))*taper*(.38+.62*discontinuous))
        rust=np.maximum(rust,(1-ease(.17,.60,abs(dist+1.08*width)))*taper*ease(.1,.66,.4+mid))
    rock*=focus;pale*=focus;cut*=focus;rust*=focus
    # A mineral colour family, not painted light. Leave the low drainage
    # bowl and wide native slope outside this local connection untouched.
    gain=np.stack((1+.60*rock+.56*pale+.19*rust-.47*cut+.10*rock*mid,
                   1+.41*rock+.40*pale+.045*rust-.42*cut+.085*rock*mid,
                   1+.24*rock+.28*pale-.10*rust-.37*cut+.065*rock*mid),-1)
    return np.clip(gain,.60,2.52),rock,pale,cut
def sculpt(land,q,folder,lod):
    from shoulders import shoulder,local
    from mountainskin import field
    from island_erratic_glb import streams
    from finished_island_glb import save
    # Reuse exact XZ lattice, original pivot and texture coordinates; do not
    # create a second coincident surface below this new shoulder.
    old,_,colour,tri,_=shoulder(land,q,ID,lod==0)
    x,z=old[:,0],old[:,2];X,Z=local(x,z,q)
    fy,_,_=land.floor(X,Z);s=q['scale'];base=q['position'][1]
    edge=ease(0,.51,np.minimum(4-abs(x),2-abs(z)));low=(fy-base-.12)/s
    top=seats(land,q,X,Z,low,edge);v=np.column_stack((x,top,z)).astype('<f4')
    fc=np.cross(v[tri[:,1]]-v[tri[:,0]],v[tri[:,2]]-v[tri[:,0]])
    n=np.zeros_like(v)
    for j in range(3):np.add.at(n,tri[:,j],fc)
    n/=np.maximum(1.e-8,np.linalg.norm(n,axis=1,keepdims=True))
    wx_y=base+s*top;mineral,cover,pale,cut=lithology(land,q,X,wx_y,Z)
    broad=field(land,X,Z)[0]
    D,C=axes(land,q,X,Z);geog=wx_y-base-dip(D,C)
    # These two upper exposures live primarily ON the block; the lower
    # -1.62/-4.82/-8.08m contacts recur on the untouched sloping terrain.
    highveil=(1-ease(.14,.86,abs(geog-2.05)))*(.4+.45*nz(D,C,1.62,540))+(1-ease(.22,1.2,abs(geog-6.55)))*.52
    tone=np.stack([.27+.09*highveil,.265+.073*highveil,.249+.055*highveil],-1)
    stone=(.55*colour+.45*tone)*np.array([1.46,1.40,1.35])*mineral*(1+.31*(broad-1))
    stone=np.asarray(np.clip(stone,.03,.96),'<f4')
    face=fc/np.maximum(1.e-8,np.linalg.norm(fc,axis=1,keepdims=True))
    axis=np.argmax(abs(face),axis=1);ids,inv=np.unique(np.column_stack((tri.ravel(),axis.repeat(3))),axis=0,return_inverse=True)
    old,ax=ids.T;vv=v[old];uu=np.where(ax==0,vv[:,2],vv[:,0]);vvv=np.where(ax==1,vv[:,2],-vv[:,1])
    uv=np.column_stack(((uu+.32*vvv)/8.9,(vvv-.10*uu)/8.9)).astype('<f4');tri=inv.reshape(-1,3).astype('<u2')
    yaw=q['yaw']
    doc=dict(asset=dict(version='2.0',generator='Cloudstep / connected original cliff beds'),scene=0,scenes=[dict(nodes=[0])],
        nodes=[dict(name='Joined_1695',mesh=0,translation=q['position'],rotation=[0,math.sin(yaw/2),0,math.cos(yaw/2)],scale=[s]*3)],
        meshes=[],accessors=[],bufferViews=[],materials=[dict(name='The same dipping gneiss as V1 surface',doubleSided=True,
        pbrMetallicRoughness=dict(metallicFactor=0.,roughnessFactor=.86,baseColorTexture=dict(index=0)))],
        images=[dict(uri='jointed-rock-face.png')],textures=[dict(source=0,sampler=0)],samplers=[dict(magFilter=9729,minFilter=9987,wrapS=10497,wrapT=10497)])
    buf=bytearray();emit=streams(doc,buf);attr=dict(POSITION=emit(vv,5126,'VEC3',bounds=True),NORMAL=emit(n[old],5126,'VEC3'),COLOR_0=emit(stone[old],5126,'VEC3'),TEXCOORD_0=emit(uv,5126,'VEC2'))
    doc['meshes']=[dict(primitives=[dict(attributes=attr,indices=emit(tri.ravel(),5123,'SCALAR',indices=True),material=0)])]
    doc['buffers']=[dict(byteLength=len(buf))];save(doc,buf,folder/('joined-1695-L'+str(lod)+'.glb'))
    trim=edge<.00001
    return dict(triangles=len(tri),world_top=round(float(wx_y.max()),4),min_seat_to_v1=round(float((wx_y-fy)[trim].min()),4),
                max_seat_to_v1=round(float((wx_y-fy)[trim].max()),4))

def make(native,folder):
    land=Inlet(native/'island-v1.bin')
    q=json.loads((native/'props.json').read_text())[ID]
    a,b,c,d=BOX;W,H=438,364
    X,Z=np.meshgrid(a+(np.arange(W)+.5)*(b-a)/W,c+(np.arange(H)+.5)*(d-c)/H)
    Y=land.floor(X,Z)[0];g,cover,pale,cut=lithology(land,q,X,Y,Z)
    Image.fromarray(np.uint8(np.clip(np.rint(g*98),0,255))).save(folder/'joined-true-beds.png',optimize=True)
    D,C=axes(land,q,X,Z)
    # Honest points on the native contact, not reconstructed heights on the prop.
    witness=[]
    for test in [0,3,7,11,15,19]:
        k=np.argmin((D-test)**2+C*C);i,j=np.unravel_index(k,D.shape)
        witness.append(dict(down_m=test,native_xyz=np.asarray([X[i,j],Y[i,j],Z[i,j]]).round(4).tolist(),
                         relative_to_common_plane=round(float(Y[i,j]-q['position'][1]-dip(D[i,j],C[i,j])),4)))
    meta=dict(native_plan_xz=BOX,neutral=98,original_prop=ID,lods={str(l):sculpt(land,q,folder,l) for l in (0,1)},earthen_geometry_changes=0,upper_obstacle_top_m=q['position'][1]+12*q['scale'],
              true_downhill_cross_section=witness,percent_patch_cover_over_point2=round(100*float((cover>.2).mean()),3))
    (folder/'joined-true-beds.json').write_text(json.dumps(meta,indent=2)+'\n')
    return meta
if __name__=='__main__':print(make(*(Path(p) for p in sys.argv[1:])))
