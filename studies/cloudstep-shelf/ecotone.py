"""Cloudstep approach: one visual transition across two real steep faces, the
bare perched shelf between them, an open mineral shoulder and actual pine roots.

This does NOT infer a walking route or carry a wash across a real rise. No
plants relocated; no new scattered woodland or raised ground. Native optics
from the accepted perched and lower studies are deliberately reconciled, not
blindly multiplied one over another.
"""
from pathlib import Path
from collections import deque
import csv,hashlib,io,json,math,shutil,sys
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter,distance_transform_edt
HERE=Path(__file__).resolve().parent
sys.path[:0]=[str(HERE),str(HERE.parents[1]/'scripts'),str(HERE.parent/'lanternback-rim')]
from island_inlet_ground import Inlet,ease
from asset_scene import Source
from strata import nz
BOX=(39.,102.,-423.,-340.)
GRAIN=.5
AXIS=[
 ('original upper shoulder',57.0812,-350.471),
 ('descending main joint',64.481,-358.611),
 ('first lower sill',69.8884,-364.561),
 ('bare perched shelf',70.,-372.),
 ('second steep face',74.,-382.),
 ('weathered rollover',77.,-391.),
 ('open approach',79.,-400.),
 ('held eye edge',81.5,-408.7),
 ('fine mineral clearing',80.4,-410.2),
 ('rooted original pine 991',78.6012,-415.607)
]

def plan(grid):
    a,b,c,d=BOX
    return np.meshgrid(np.arange(a+grid*.5,b,grid),np.arange(c+grid*.5,d,grid))
def lookup(im,area,x,z,interpolate=True):
    a,b,c,d=area;h,w=im.shape[:2];u=(x-a)/(b-a)*w-.5;v=(z-c)/(d-c)*h-.5
    i=np.clip(np.floor(u).astype('i'),0,w-1);j=np.clip(np.floor(v).astype('i'),0,h-1)
    if not interpolate:return im[j,i]
    i1=np.minimum(w-1,i+1);j1=np.minimum(h-1,j+1)
    fu=(u-np.floor(u))[...,None];fv=(v-np.floor(v))[...,None]
    vals=im[...,None] if im.ndim==2 else im
    ans=(1-fv)*((1-fu)*vals[j,i]+fu*vals[j,i1])+fv*((1-fu)*vals[j1,i]+fu*vals[j1,i1])
    return ans[...,0] if im.ndim==2 else ans
def connected(allowed,seed):
    a,b,c,d=BOX;T=allowed.shape[1];i=int((seed[0]-a)/GRAIN);j=int((seed[1]-c)/GRAIN)
    if not allowed[j,i]:raise ValueError('Original measured island seat is missing')
    found=np.zeros_like(allowed);found[j,i]=True;fifo=deque([(j,i)])
    while fifo:
        j,i=fifo.popleft()
        for p,q in ((j-1,i),(j+1,i),(j,i-1),(j,i+1)):
            if 0<=p<allowed.shape[0] and 0<=q<T and allowed[p,q] and not found[p,q]:
                found[p,q]=True;fifo.append((p,q))
    return found
def soft(mask,radius):
    inside=distance_transform_edt(mask)*GRAIN;outside=distance_transform_edt(~mask)*GRAIN
    return ease(-radius,radius,inside-outside)

def contact_field(native,copse,land):
    a,b,c,d=BOX;spacing=.20;h=int(round((d-c)/spacing));w=int(round((b-a)/spacing))
    deposits=np.zeros((h,w),'f');stock=Source(copse);accepted=0;tiles=[]
    for node in stock.doc['nodes']:
        shift=np.asarray(node.get('translation',[0,0,0]),'f');pr=stock.doc['meshes'][node['mesh']]['primitives'][0];att=pr['attributes']
        prop=stock.doc['accessors'][att['POSITION']];lo=np.array(prop['min'])+shift;hi=np.array(prop['max'])+shift
        if hi[0]<a or lo[0]>b or hi[2]<c or lo[2]>d:continue
        P=stock.read(att['POSITION'])+shift;U=stock.read(att['TEXCOORD_0'])
        T=stock.read(pr['indices']).reshape(-1,3);cent=P[T].mean(1)
        keep=(cent[:,0]>a-.5)&(cent[:,0]<b+.5)&(cent[:,2]>c-.5)&(cent[:,2]<d+.5);T=T[keep]
        if not len(T):continue
        mat=stock.doc['materials'][pr['material']]
        atlas=stock.image(mat['pbrMetallicRoughness']['baseColorTexture'])[0];hh,ww=atlas.shape[:2]
        start=accepted
        for aa,bb in ((.22,.22),(.54,.23),(.22,.54),(.08,.43),(.80,.10),(.10,.80)):
            W=np.array([1-aa-bb,aa,bb],'f');point=np.einsum('j,ijk->ik',W,P[T]);uv=np.einsum('j,ijk->ik',W,U[T])
            X,Y,Z=point.T;g,*_=land.floor(X,Z)
            cols=atlas[np.minimum(hh-1,(np.clip(uv[:,1],0,1)*hh).astype('i')),np.minimum(ww-1,(np.clip(uv[:,0],0,1)*ww).astype('i'))]
            # A leaf cropped transparent above the dirt is not a rooted area.
            # Attenuation only beneath source texels actually touching the land.
            ok=(X>a)&(X<b)&(Z>c)&(Z<d)&(Y-g>-.16)&(Y-g<.105)&(cols[:,3]>=255*mat.get('alphaCutoff',.42))
            xx,zz=X[ok],Z[ok]
            np.add.at(deposits,(((zz-c)/(d-c)*h).astype('i'),((xx-a)/(b-a)*w).astype('i')),1)
            accepted+=len(xx)
        if accepted>start:tiles.append(node['name'])
    stock.close()
    core=np.clip(gaussian_filter(np.minimum(deposits,3),.64),0,1)
    margin=np.clip(gaussian_filter(np.minimum(deposits,2),1.78)*1.45,0,1)
    return np.maximum(core*.57,margin),dict(original_alpha_tested_ground_contact_samples=accepted,held_low_copse_tiles=tiles,contact_clearance_m=[-.16,.105],max_visual_spread_m=.9)

def chart_meta(folder,file,key='native_plan_xz'):
    desc=json.loads((folder/(file+'.json')).read_text());return desc[key]
def rgb(folder,file):
    return np.asarray(Image.open(folder/(file+'.png')).convert('RGB'),'f')
def clamp_field(im,box,x,z,neutral):
    a,b,c,d=box;inside=(x>=a)&(x<=b)&(z>=c)&(z<=d)
    out=lookup(im,box,x,z)/neutral
    return np.where(inside[...,None],out,1.)
FAN=(62.,96.,-403.,-375.)
def fan(native,land,protected,rooted):
    from mountainskin import plates
    a,b,c,d=FAN;step=.115;W=int(np.ceil((b-a)/step));H=int(np.ceil((d-c)/step))
    X,Z=np.meshgrid(a+(np.arange(W)+.5)*(b-a)/W,c+(np.arange(H)+.5)*(d-c)/H)
    Y,gx,gz=land.floor(X,Z);body=np.zeros_like(X);progress=np.zeros_like(X);tracks=[]
    # Source is the SECOND broken face, not #1695 above the 94m sill,
    # and not BoulderC #1330 which lies downhill from the held floor eye.
    for seed,phase in (((70.,-385.),.11),((74.,-382.),.53),((79.,-382.),.77)):
        x,z=seed;used=[]
        for i in range(58):
            y,xx,zz=land.floor(x,z);s=math.hypot(xx,zz)
            used.append([float(x),float(y),float(z),float(s)])
            if s<.14:break
            # Re-evaluate the triangle gradient at every 45cm step and STOP
            # at a rise. This is art placement, not a claim about hydrology.
            nx=x-.45*xx/s;nz0=z-.45*zz/s
            if land.floor(nx,nz0)[0]>=y:break
            x,z=nx,nz0
        tr=np.asarray(used);full=.45*(len(tr)-1);distance=np.full_like(X,1e6);along=np.zeros_like(X)
        for j,(qx,qy,qz,gr) in enumerate(tr):
            delta=(X-qx)**2+(Z-qz)**2;m=delta<distance;along[m]=.45*j;distance=np.minimum(distance,delta)
        norm=along/max(.01,full);width=1.12+2.70*norm**.83+.42*(nz(X,Z,3.91,442)-.5)
        spread=(1-ease(.35*width,1.61*width,np.sqrt(distance)))*ease(-.3,2.2,along)*(.47+.53*ease(0,.64,norm))
        # The far terminal shoulders fade where the genuine fall stops,
        # rather than continuing toward whichever tree the camera happens to see.
        spread*=1-ease(.73,1.08,norm)
        lead=spread>body;progress=np.where(lead,norm,progress);body=np.maximum(body,spread)
        tracks.append(dict(source_xz=seed,path_xyz_grade=tr.round(4).tolist(),line_length_m=round(full,3),height_drop_m=round(tr[0,1]-tr[-1,1],3)))
    original_root=lookup(rooted,BOX,X,Z);a0,b0,c0,d0=protected[1]
    needles=np.where((X>=a0)&(X<=b0)&(Z>=c0)&(Z<=d0),np.clip((128-lookup(protected[0],protected[1],X,Z)[...,2])/36,0,1),0.)
    body*=1-.83*np.maximum(original_root,needles)
    body*=ease(0,1.5,np.minimum(X-a,b-X))*ease(0,1.5,np.minimum(Z-c,d-Z))
    wave=nz(X,Z,3.0,419)-.5;fine=nz(X,Z,.62,948)-.5
    # Lenses of weathered angular stone become finer toward the stall.
    # Only selected facets reach the surface, leaving original material
    # visible in long gaps, rather than drawing three artificial paths.
    U=(.67*X-.73*Z+.18*Y+1.31*wave)/.77;V=(.73*X+.67*Z-.28*Y+.38*fine)/.36
    joint,cell=plates(U,V)
    field=body*ease(.23,.73,.45+.65*wave+.54*fine+.28*(cell-.5))
    coarse=field*(1-.53*progress)
    cleavage=coarse*(1-ease(.035,.17,joint))
    sand=body*progress*(.29+.44*ease(.05,.65,.43+wave))
    gain=np.stack((1+.30*sand+(.59+.45*(cell-.5))*coarse-.34*cleavage,
                   1+.20*sand+(.39+.37*(cell-.5))*coarse-.28*cleavage,
                   1+.095*sand+(.23+.29*(cell-.5))*coarse-.22*cleavage),-1)
    im=Image.fromarray(np.uint8(np.clip(np.rint(gain*128),0,255)));o=io.BytesIO();im.save(o,format='PNG',optimize=True)
    return o.getvalue(),body,dict(native_plan_xz=FAN,neutral=128,trace_sources=tracks,traces_are_not_paths_or_physics=True,material_surface_only=True)
def build(native,copse,folder,limit=295000):
    def original(path,target,git=False):
        h=hashlib.sha1() if git else hashlib.sha256()
        if git:h.update(('blob '+str(path.stat().st_size)+'\0').encode())
        with path.open('rb') as f:
            while part:=f.read(131072):h.update(part)
        if h.hexdigest()!=target:raise ValueError('Need the registered original source: '+path.name)
    original(native/'island-v1.bin','8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38')
    original(native/'props.json','03dc20686110ad3dfb30c243ccd547cfc3595127',True)
    original(copse,'7dc9cf345efbec44ec5cd1f0c24dadb7ed32a706',True)
    original(copse.parent/'copse'/'duff-bark.png','e8e4988b93d6e01371cd0ad80c84bbe726c931f7',True)
    land=Inlet(native/'island-v1.bin')
    props=json.loads((native/'props.json').read_text());X,Z=plan(GRAIN);Y,GX,GZ=land.floor(X,Z);grade=np.hypot(GX,GZ)
    # 94.07 at the first sill, then +1.27 m into its backside: the upper
    # apron really terminates at a basin. A blanket downhill debris ribbon
    # extending into the 68m grove would climb this original V1 surface.
    upper=connected((np.abs(Y-94.07)<2.5)&(grade<.42),(69.8884,-364.561))
    lower=connected((Y>=57.)&(Y<=73.)&(grade<.44),(82.,-406.))
    perch=soft(upper,1.2);shelf=soft(lower,1.4)
    rooted,contact_note=contact_field(native,copse,land)
    held=chart_meta(folder,'held-copse','box_xz')
    needle_map=rgb(folder,'forest-shed-gain')
    woody=np.clip((128-lookup(needle_map,held,X,Z)[...,2])/36,0,1)
    inside=(X>=held[0])&(X<=held[1])&(Z>=held[2])&(Z<=held[3])
    woody=np.where(inside,woody,0)
    root=lookup(rooted,BOX,X,Z)
    bc=chart_meta(folder,'broken-granite');gran=clamp_field(rgb(folder,'broken-granite'),bc,X,Z,90)
    jc=chart_meta(folder,'joined-true-beds');joined=clamp_field(rgb(folder,'joined-true-beds'),jc,X,Z,98)
    # Root shelters require independent original evidence. Local gentle soil
    # without bough or original low geometry is not upgraded into a forest.
    shelter=np.maximum(woody,root)*shelf;mineral=shelf*(1-ease(.08,.49,shelter))
    # Two held geological faces flank the perch. Guard the accepted local
    # dipping contact so the 1695 bench does not become another detached fin.
    protected_joint=ease(.018,.20,np.max(np.abs(joined-1.),axis=-1))
    steep=ease(.49,1.10,grade);keep_bed=np.maximum(steep,protected_joint)
    old_broad=np.maximum(gran,.0001)
    # Restrained adjudication of the previous material. The broad grain in
    # the held granite should fade on a true, flat silty perch, not abut a
    # foreign moss carpet or re-tint the whole existing mountain.
    grain=nz(X,Z,2.9,119)-.5
    band=np.clip(1-np.abs((X-62.)*.83+(Z+379.)*.35)/24.,0,1)
    bench=perch*(.71+.29*band)*(1-protected_joint)
    local_edge=ease(0,3.,np.minimum(X-BOX[0],BOX[1]-X))*ease(0,3.,np.minimum(Z-BOX[2],BOX[3]-Z))
    calm=np.clip(.80*bench+.68*shelter+.38*mineral,0,.88)*(1-.77*keep_bed)
    correction=np.power(old_broad,-calm[...,None])
    # High shelf: warm ochre-silver dust in the quiet broken stone, not
    # green at altitude. Low shelter: cocoa/blue-black organic pockets
    # seated into the *actual* root contact; skip empty nearest-root circles.
    light=bench*(.44+.52*ease(.04,.68,.41+grain))*local_edge
    fine=mineral*(.24+.45*ease(.09,.63,.34+grain))
    # Do not repeat the close chart's character or add tiny distant clutter.
    pa,pb,pc,pd=(73.5,86.5,-418.5,-404.0)
    is_local=ease(0,1.1,np.minimum(X-pa,pb-X))*ease(0,1.1,np.minimum(Z-pc,pd-Z))
    fine*=1-.86*is_local
    extra=np.stack((1+.24*light+.09*fine+.13*shelter,
                     1+.15*light+.045*fine-.12*shelter,
                     1+.05*light+.01*fine-.18*shelter),-1)
    correction=1+local_edge[...,None]*(correction*extra-1)
    # Half-metre chart; the source response, geometry and roots remain
    # separate at their honest resolution. No new relief or global props.
    image=Image.fromarray(np.uint8(np.clip(np.rint(correction*128),0,255)))
    stream=io.BytesIO();image.save(stream,format='PNG',optimize=True);raw=stream.getvalue()
    records=[]
    for name,x,z in AXIS:
        y,gx,gz=land.floor(x,z)
        closest=min((math.hypot(p['position'][0]-x,p['position'][2]-z),i) for i,p in enumerate(props) if p['kind'] in ('Conifer','Sapling','Broadleaf'))
        at=lambda im:round(float(lookup(im,BOX,np.array(x),np.array(z))),4)
        records.append(dict(landform=name,x=x,z=z,v1_y=round(float(y),4),grade=round(math.hypot(gx,gz),4),nearest_original_tree=closest[1],nearest_tree_metres=round(closest[0],3),perched_weight=at(perch),lower_shelf_weight=at(shelf),real_shelter_weight=at(shelter),original_root_contact_weight=at(root)))
    r=dict(native_plan_xz=BOX,neutral=128,register=records,held_higher_shelf_m2=int(upper.sum())*.25,
        held_lower_connected_shelf_m2=int(lower.sum())*.25,
        actual_original_trees_on_upper_half_metre_cells=sum(1 for p in props if p['kind'] in ('Conifer','Sapling','Broadleaf') and BOX[0]<=p['position'][0]<BOX[1] and BOX[2]<=p['position'][2]<BOX[3] and upper[int((p['position'][2]-BOX[2])/GRAIN),int((p['position'][0]-BOX[0])/GRAIN)]),
        source_contact=contact_note,arbitrates_existing_granite=True,not_a_route_or_hydrology_or_collision_change=True,
        new_vegetation_instances=0)
    fan_rgb,fan_weight,fan_note=fan(native,land,(needle_map,held),rooted)
    for k in ('summit-window','sheltered-boles','shelf-section'):
        with (folder/(k+'-footprint.csv')).open() as f:points=list(csv.DictReader(f))
        xx=np.array([float(v['x']) for v in points]);zz=np.array([float(v['z']) for v in points])
        fa,fb,fc,fd=FAN;valid=(xx>fa)&(xx<fb)&(zz>fc)&(zz<fd)
        frac=np.zeros_like(xx);frac[valid]=lookup(fan_weight,FAN,xx[valid],zz[valid])
        fan_note.setdefault('native_ground_only_camera_samples',{})[k]=dict(hits=len(points),inside_patch_over_point2=int((frac>.2).sum()))
    js=(json.dumps(r,indent=2)+'\n').encode();fs=(json.dumps(fan_note,indent=2)+'\n').encode()
    n=len(raw)+len(js)+len(fan_rgb)+len(fs)
    if n>limit or shutil.disk_usage(folder).free<4509715661+20971520+n+65536: return dict(staged=False,required_bytes=n,**r)
    (folder/'approach-transition.png').write_bytes(raw);(folder/'approach-transition.json').write_bytes(js)
    (folder/'second-face-fan.png').write_bytes(fan_rgb);(folder/'second-face-fan.json').write_bytes(fs)
    return dict(staged=True,required_bytes=n,**r)
if __name__=='__main__':print(build(Path(sys.argv[1]),Path(sys.argv[2]),Path(sys.argv[3]),int(sys.argv[4]) if len(sys.argv)>4 else 295000))
