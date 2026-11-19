"""Recast the soles of two actual cove solids. No new ground or separate darkening.

The outer plan is held: every convex footprint corner of each old body still
exists. Part of the downhill outline really is over air below the *saved box*
and cannot be filled in. Occupy the original volume only up to that hard
altitude and key the rest into the uphill triangles instead.
"""
from pathlib import Path
from copy import deepcopy
import json, math
import numpy as np
from scipy.spatial import ConvexHull,cKDTree
from finished_island_glb import parse,save,values
from island_boulder_bedjoint import binding
from island_erratic_blocks import unit
from island_erratic_glb import streams
from island_erratic_rind import project
from island_talus_contact import members

TARGETS={1785:('BoulderB',1664),1619:('BoulderA',1547)}
def ease(a,b,x):
    u=np.clip((x-a)/(b-a),0,1); return u*u*(3-2*u)

def underside(p,xz):
    eq=ConvexHull(p).equations
    low=eq[eq[:,1]<-1e-7]
    return np.max(-(xz@low[:,[0,2]].T+low[:,3])/low[:,1],axis=1)

def seats(q,ground,level,body,min_box,max_box,scale):
    xz=q[:,[0,2]];rim=ConvexHull(xz); ring=xz[rim.vertices]
    c=ring.mean(0); lo=max(min_box,float(q[:,1].min())); hi=min(max_box,float(q[:,1].max()));height=hi-lo
    # Source shell extends over a sloping bed, but the opaque collider starts
    # at a horizontal altitude. A bracketed cut through its plan is the real
    # maximum contact area. Never draw a false floor outside that cut.
    a,b=xz.min(0),xz.max(0)
    X,Z=np.meshgrid(np.linspace(a[0],b[0],67),np.linspace(a[1],b[1],67))
    grid=np.column_stack([X.ravel(),Z.ravel()])
    grid=grid[(grid@rim.equations[:,:2].T+rim.equations[:,2]<-.0005).all(1)]
    bed=ground(grid); original_gap=(underside(q,grid)-bed)*scale
    original_contact=bed>=lo
    if original_contact.mean()<.12:
        raise ValueError('Source cannot support a truthful mounted heel.')
    # The downhill bed is below the *physical* bottom. Widening a horizontal
    # apron down to that level would produce a bigger black lip, not a true
    # socket. Put the sole only on the uphill bearing area. Cut the unsupported
    # underface on a steep raking bevel instead.
    support=grid[bed>lo+.015/scale]
    kh=ConvexHull(support); pol=support[kh.vertices]; hub=pol.mean(0)
    dirs=np.linspace(0,2*math.pi,18 if level==0 else 11 if level==1 else 7,endpoint=False)
    pol=pol[np.unique(np.argmax((pol-hub)@np.stack([np.cos(dirs),np.sin(dirs)]),axis=0))]
    # Restore ordered polygon after directional vertex subsampling.
    pol=pol[ConvexHull(pol).vertices];pol=hub+(pol-hub)*.992
    base=np.clip(ground(pol)-.034/scale,lo,lo+height*.19)
    foot=np.c_[pol[:,0],base,pol[:,1]]
    ca=pol;cb=np.roll(ca,-1,axis=0); ab=cb-ca
    def nearest(x):
        tau=np.clip(((x[:,None,:]-ca)*ab).sum(-1)/np.maximum(1e-10,np.sum(ab*ab,axis=1)),0,1)
        cand=ca+tau[:,:,None]*ab
        near=cand[np.arange(len(x)),np.linalg.norm(cand-x[:,None,:],axis=2).argmin(1)]
        blocked=(x@kh.equations[:,:2].T+kh.equations[:,2]<=0).all(1)
        near[blocked]=x[blocked]
        return near
    peg=nearest(ring);dist=np.linalg.norm(ring-peg,axis=1)
    exposed=ease(.015,.40*height,dist)
    ang=np.arctan2(ring[:,1]-c[1],ring[:,0]-c[0])
    ridge=.5+.5*np.cos(ang*2.1+body*1.3)
    # Keep every exposed plan extremum, but only on a shoulder where rock
    # actually projects; preserve negative space between its three bodies.
    by=np.maximum(lo+height*(.20+.10*ridge+.22*exposed),ground(ring)+.08*height)
    by=np.clip(by,lo+.17*height,lo+.66*height)
    belt=np.c_[ring[:,0],by,ring[:,1]]
    # Break the near-facing fracture into two planes rather than a flat cut.
    q1=peg+(ring-peg)*(.46+.11*ridge)[:,None]
    q2=peg+(ring-peg)*(.82+.035*ridge)[:,None]
    base0=np.clip(ground(peg)-.027/scale,lo,lo+height*.19)
    ky1=base0+(by-base0)*(.28+.11*ridge)
    ky2=base0+(by-base0)*(.63+.12*ridge)
    flank1=np.c_[q1[:,0],ky1,q1[:,1]][dist>.035]
    flank2=np.c_[q2[:,0],ky2,q2[:,1]][dist>.065]
    # Retain original broad high faces. Let the downhill shoulder tip inward
    # with bedding; one real old summit remains the native max. No hip is
    # swelled outward just to cast a larger shadow onto the hillside.
    retained=q[q[:,1]>=lo+height*.29].copy()
    rr=retained[:,[0,2]]
    ledge=1-ease(-.23,.13,(ground(rr)-lo)/height)
    fac=ease(.23,.72,(retained[:,1]-lo)/height)
    summit=int(np.argmax(retained[:,1]));held=float(retained[summit,1])
    retained[:,1]=np.clip(retained[:,1]-.15*height*ledge*fac,lo,hi)
    retained[summit,1]=min(held,hi)
    # Keep high and low tiers shaped by the same rising substrate.
    arr=np.vstack((retained,belt,foot,*( [flank1,flank2] if level==0 else [flank1[::2]] if level==1 else [] )))
    arr=np.unique(np.round(arr,7),axis=0)
    hull=ConvexHull(arr);v=arr[hull.vertices]
    ix=np.full(len(arr),-1);ix[hull.vertices]=np.arange(len(v));tri=ix[hull.simplices]
    fn=np.cross(v[tri[:,1]]-v[tri[:,0]],v[tri[:,2]]-v[tri[:,0]])
    rev=np.einsum('ij,ij->i',fn,v[tri].mean(1)-v.mean(0))<0;tri[rev]=tri[rev][:,[0,2,1]]
    new_gap=(underside(v,grid)-bed)*scale
    # Affine x/z supports of both projections agree, not just their AABBs.
    rr=ConvexHull(v[:,[0,2]])
    supports=np.column_stack((rim.equations[:,:2].T,rr.equations[:,:2].T))
    span=np.max(np.abs((xz@supports).max(0)-(v[:,[0,2]]@supports).max(0)))*scale
    report=dict(old_contact_fraction=float(np.mean(original_gap<=.015)),new_contact_fraction=float(np.mean(new_gap<=.015)),
                projected_area_m2=float(rim.volume*scale**2),physically_reachable_fraction=float(np.mean((lo-bed)*scale<=.015)),
                plan_support_max_delta_m=float(span),lowest_local_height_m=float(v[:,1].min()*scale),
                before_positive_gap_q50_m=float(np.percentile(np.maximum(original_gap,0),50)),
                after_positive_gap_q50_m=float(np.percentile(np.maximum(new_gap,0),50)),
                uphill_support_samples=int(original_contact.sum()),source_samples=len(grid))
    return v,tri,report

def write(src,surface,prop,index,dest):
    kind,expected=TARGETS[index]
    if prop['kind']!=kind or Path(src).name!=kind+'.glb':raise ValueError('Wrong original rock.')
    original,raw=parse(src);doc=deepcopy(original);doc.update(meshes=[],accessors=[],bufferViews=[])
    first=original['meshes'][0]['primitives'][0]
    floor,obstacle,i=binding(surface,prop,np.array(values(original,raw,first['attributes']['POSITION'])))
    if i!=expected:raise ValueError('The original solid or native map no longer matches this study.')
    sc=prop['scale']; parent_y=prop['position'][1]
    by=(obstacle[4:6].astype(float)-parent_y)/sc
    # Keep the original material model; the separate coastal texture copy
    # incorporates the same cove mineral grain without changing the native donor.
    for img in doc['images']:img['uri']='coastal-erratic/'+Path(img['uri']).name
    buf=bytearray();emit=streams(doc,buf); levels={}
    for level,m in enumerate(original['meshes']):
        p=m['primitives'][0]
        p0,n0,c0=(np.array(values(original,raw,p['attributes'][k])) for k in ['POSITION','NORMAL','COLOR_0'])
        tri0=np.array(values(original,raw,p['indices'])).reshape(-1,3)
        groups=sorted(members(p0,tri0),key=lambda sub:-np.prod(np.ptp(p0[sub],axis=0)))
        if len(groups)!=3:raise ValueError('Original compound changed.')
        chunks=[];rows=[]
        for j,ids in enumerate(groups):
            q,original_ids=np.unique(np.round(p0[ids],7),axis=0,return_index=True)
            v,t,r=seats(q,floor,level,j,by[0],by[1],sc)
            # Stone relief is lit by the same sun and casts the true new shadow.
            # Site colour is material, not an alpha darkener on the soil.
            near=cKDTree(q);_,nn=near.query(v)
            # Each face keeps its related cleavage normals.
            fn=unit(np.cross(v[t[:,1]]-v[t[:,0]],v[t[:,2]]-v[t[:,0]]))
            N=np.empty((len(t),3,3))
            incident=[[] for _ in v]
            for ti,face in enumerate(t):
                for vi in face:incident[vi].append(ti)
            for ti,face in enumerate(t):
                for ci,vi in enumerate(face):
                    side=fn[incident[vi]];w=ease(.54,.89,side@fn[ti])
                    soft=unit((side*w[:,None]).sum(0))
                    N[ti,ci]=unit(.40*fn[ti]+.60*soft)
            c=c0[ids][original_ids][nn].copy()
            net=(v[:,1]-floor(v[:,[0,2]]))*sc
            damp=1-ease(-.01,.27,net)
            salt=ease(.025,.20,net)*(1-ease(.22,.67,net))
            rough=.5+.5*np.sin(v[:,0]*12.2+v[:,2]*8.4+j*1.8)
            c[:,:3]=np.clip(c[:,:3]*(1-.080*damp[:,None]) + salt[:,None]*(.35+.55*rough[:,None])*np.array([.022,.028,.013]),0,1)
            # Preserve cool iron-shale B versus ochre glacial A. Dampen only
            # where the actual local V1 passes this stone, not a world Y stripe.
            uv=project(v,t,.91*(ord(kind[-1])-64)+2.14*j)
            chunks.append(np.column_stack((v[t].reshape(-1,3),N.reshape(-1,3),uv,c[t].reshape(-1,4))))
            rows.append(r)
        allv,ii=np.unique(np.vstack(chunks).astype('<f4'),axis=0,return_inverse=True)
        pp,nn,uv,colour=np.split(allv,[3,6,8],axis=1)
        at=dict(POSITION=emit(pp,5126,'VEC3',bounds=True),NORMAL=emit(nn,5126,'VEC3'),
                TEXCOORD_0=emit(uv,5126,'VEC2'),COLOR_0=emit(np.uint8(np.rint(np.clip(colour,0,1)*255)),5121,'VEC4',normalized=True))
        name=f'{kind}Seat{index}_L{level}'
        doc['meshes'].append(dict(name=name,primitives=[dict(attributes=at,indices=emit(ii.ravel().astype('<u2'),5123,'SCALAR',indices=True),material=p['material'])]))
        for node in doc['nodes']:
            if node.get('mesh')==level:node['name']=name
        levels[f'L{level}']=dict(triangles=len(ii)//3,original_triangles=len(tri0),bodies=rows)
    doc['buffers']=[dict(byteLength=len(buf))]
    save(doc,buf,dest)
    return dict(kind=kind,anchor=index,native_parent=prop,unchanged_solid_index=i,
                native_box_xmin_zmin_xmax_zmax_ymin_ymax=obstacle.tolist(),levels=levels)
