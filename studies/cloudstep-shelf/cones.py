"""Three small seated cone husks and a reduced body-height authoring cut.
The origins came from true alpha-tested native pine boughs and V1 drop points,
not a random carpet. No collision, food, inventory or gameplay identity.
"""
from pathlib import Path
import csv,json,math,sys
import numpy as np
from scipy.spatial.transform import Rotation
H=Path(__file__).resolve().parent
sys.path.insert(0,str(H.parents[1]/'scripts'))
from finished_island_glb import save
from island_erratic_glb import streams

def model(form):
    rng=np.random.default_rng(54110+form);v=[];co=[];tri=[]
    def surface(pp,cc,ii):
        off=len(v);v.extend(pp);co.extend(cc);tri.extend([[off+j for j in t] for t in ii])
    # Normalised local model long axis X, Y-up. Imbricate shell scales
    # share an irregular longitudinal spine, not a pineapple cross-hatch.
    rings=[(-.48,.043),(-.32,.095),(-.10,.110),(.20,.083),(.45,.036)]
    side=6
    for k,(x,r) in enumerate(rings):
        for j in range(side):
            a=j*math.tau/side
            v.append([x,r*math.cos(a),r*math.sin(a)]);co.append([.071,.044,.023])
            if k:
                a=(k-1)*side+j;b=(k-1)*side+(j+1)%side
                tri.extend(((a,b,a+side),(b,b+side,a+side)))
    for band in range(7):
        t=(band+.65)/7.65
        x=-.45+.82*t
        radius=(.072+.101*math.sin(math.pi*t))*(1+.10*(form-1))
        n=(5 if band in (2,3,4) else 4)
        phase=band*2.29+form*.31
        for j in range(n):
            if (form==2 and band>=4 and j==2) or (form==1 and band==1 and j==0):continue
            a=phase+math.tau*j/n+rng.uniform(-.09,.09)
            radial=np.asarray([0.,math.cos(a),math.sin(a)])
            cross=np.asarray([0.,-math.sin(a),math.cos(a)])
            ex=np.asarray([1.,0.,0.]);base=ex*x + radial*radius*.55
            reach=.14+rng.uniform(-.02,.02);spread=(.094 if form==0 else .067)*(.75+.3*math.sin(math.pi*t))
            knuckle=base+ex*reach*.53+radial*spread*.73;end=base+ex*reach+radial*spread
            w=.054*(.72+.53*math.sin(math.pi*t))*rng.uniform(.9,1.12)
            colour=np.array([.165,.087,.038])*rng.uniform(.80,1.18)
            surface([base,knuckle-cross*w,knuckle+radial*.011,knuckle+cross*w*.87,end-cross*w*.35,end+radial*.009,end+cross*w*.38],
                    [colour*.50,colour*.81,colour*1.14,colour*.90,colour*1.18,colour*1.55,colour*.98],
                    [(0,1,2),(0,2,3),(1,4,2),(2,4,5),(2,5,3),(3,5,6)])
    p=np.array(v,'<f4');faces=np.array(tri,'<u2')
    # Retain the true bottom; only 2% of a 4.7–8.3cm cone is embedded.
    p[:,1]-=p[:,1].min()+.02
    area=np.cross(p[faces[:,1]]-p[faces[:,0]],p[faces[:,2]]-p[faces[:,0]])
    centre=p.mean(0);rel=p[faces].mean(1)-centre;rel[:,0]*=.20
    flip=(area*rel).sum(1)<0;faces[flip]=faces[flip][:,[0,2,1]];area[flip]*=-1
    N=np.zeros_like(p)
    for j in range(3):np.add.at(N,faces[:,j],area)
    N/=np.maximum(1e-9,np.linalg.norm(N,axis=1,keepdims=True))
    return p,N,np.array(co,'<f4'),faces

def export(folder,near):
    reg=list(csv.DictReader((folder/'spent-cones.csv').open()))
    anchors=json.loads((folder/'shelf-register.json').read_text())['registered_eye_xyz']
    doc=dict(asset=dict(version='2.0',generator='Cloudstep / actual-bough seedfall'),scene=0,scenes=[dict(nodes=[])],nodes=[],meshes=[],accessors=[],bufferViews=[],materials=[dict(name='Dry umber bracts, paler broke edges',doubleSided=True,pbrMetallicRoughness=dict(metallicFactor=0.,roughnessFactor=.83))],extensionsUsed=['EXT_mesh_gpu_instancing'],extensionsRequired=['EXT_mesh_gpu_instancing'])
    buf=bytearray();emit=streams(doc,buf);tot=0;root_count=0
    for f in range(3):
        rr=[r for i,r in enumerate(reg) if (i+int(r['source_pine']))%3==f and (not near or math.hypot(float(r['x'])-anchors[0],float(r['z'])-anchors[2])<10)]
        if not rr:continue
        p,n,c,ii=model(f);size=(p[:,0].max()-p[:,0].min())
        prim=dict(material=0,indices=emit(ii.ravel(),5123,'SCALAR',indices=True),attributes=dict(POSITION=emit(p,5126,'VEC3',bounds=True),NORMAL=emit(n,5126,'VEC3'),COLOR_0=emit(c,5126,'VEC3')))
        pos=[];quat=[];scale=[]
        for r in rr:
            normal=np.array([-float(r['dx']),1.,-float(r['dz'])]);normal/=np.linalg.norm(normal);axis=np.cross([0,1,0],normal)
            turn=Rotation.from_rotvec(axis*math.acos(normal[1])/max(1.e-8,np.linalg.norm(axis)))*Rotation.from_rotvec([0,float(r['yaw']),0])
            pos.append([float(r['x']),float(r['native_y']),float(r['z'])]);quat.append(turn.as_quat());scale.append([float(r['length_m'])/size]*3)
        attrs=dict(TRANSLATION=emit(np.array(pos,'<f4'),5126,'VEC3',bounds=True),ROTATION=emit(np.array(quat,'<f4'),5126,'VEC4'),SCALE=emit(np.array(scale,'<f4'),5126,'VEC3'))
        k=len(doc['nodes']);name=['Open spent cone','Short intact cone','Split sun-dried cone'][f]
        doc['nodes'].append(dict(name=name,mesh=k,extensions={'EXT_mesh_gpu_instancing':dict(attributes=attrs)}));doc['meshes'].append(dict(name=name,primitives=[prim]));doc['scenes'][0]['nodes'].append(k)
        root_count+=len(rr);tot+=len(ii)*len(rr)
    doc['buffers']=[dict(byteLength=len(buf))]
    out=folder/('cone-husks-eye.glb' if near else 'cone-husks-edit.glb');save(doc,buf,out)
    return dict(originally_rooted_sites=root_count,placed_triangles=tot,models=len(doc['meshes']))
def main(folder):
    note=dict(editor=export(folder,False),body=export(folder,True),guardian=dict(placed_triangles=0),visual_meshes_only=True)
    (folder/'cone-kit.json').write_text(json.dumps(note,indent=2)+'\n');print(note)
if __name__=='__main__':main(Path(sys.argv[1]))
