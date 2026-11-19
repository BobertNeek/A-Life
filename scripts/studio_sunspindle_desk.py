"""Portable edit: inscribed V1 triangle excerpt, linked plants, both matched cameras."""
import bpy,json,math,struct,sys
from mathutils import Vector
from pathlib import Path
src,field,dest=map(Path,sys.argv[sys.argv.index('--')+1:])
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'studies/sunspindle-hollow'))
from material import coat,plant
site=json.loads((field/'site.json').read_text()); raw=src.read_bytes()
w,d,x0,z0,cell,_=struct.unpack_from('<IIfffI',raw,8)
a,b,c,e=site['bounds'];i0,i1=math.floor((a-x0)/cell)-1,math.ceil((b-x0)/cell)+1
j0,j1=math.floor((c-z0)/cell)-1,math.ceil((e-z0)/cell)+1
verts=[];faces=[];nx=i1-i0+1
bpy.ops.wm.read_factory_settings(use_empty=True)
sc=bpy.context.scene
for j in range(j0,j1+1):
    for i in range(i0,i1+1):
        verts.append((x0+cell*i,-z0-cell*j,struct.unpack_from('<f',raw,32+4*(j*w+i))[0]))
        if i<i1 and j<j1:
            k=(j-j0)*nx+i-i0;faces.extend(((k,k+nx+1,k+1),(k,k+nx,k+nx+1)))
m=bpy.data.meshes.new('Original cells | axis and diagonal unmodified');m.from_pydata(verts,[],faces);m.update()
bed=bpy.data.objects.new('01   V1 ground | dry east hollow',m);sc.collection.objects.link(bed)
mat=bpy.data.materials.new('Existing ground plus site-specific wash');mat.use_nodes=True
p=mat.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=(.27,.30,.15,1);p.inputs['Roughness'].default_value=.89
bed.data.materials.append(mat);coat([bed],field)
plant(field/'registered-close.glb')
# The two registered levels are separate project exports with identical
# roots. Workbench uses the real close assembly rather than doubling them.
def group(name):
    col=bpy.data.collections.new(name);sc.collection.children.link(col);return col
refs=group('03   Original landmark coordinates | references only')
for i,r in site['landmarks'].items():
    x,y,z=r['position'];o=bpy.data.objects.new('#'+i+' '+r['kind'],None);o.location=(x,-z,y);o.empty_display_type='CIRCLE';o.empty_display_size=.55;refs.objects.link(o)
cams=group('04   Locked station lenses')
for name,q in site['stations'].items():
    _,eye,aim,span,lens,_=q
    def cv(p):return Vector((p[0],-p[2],p[1]))
    cam=bpy.data.cameras.new(name);o=bpy.data.objects.new(name,cam);cams.objects.link(o)
    o.location=cv(eye);o.rotation_euler=(cv(aim)-cv(eye)).to_track_quat('-Z','Y').to_euler()
    cam.type='ORTHO' if span else 'PERSP';cam.ortho_scale=max(.01,span);cam.lens=lens;cam.clip_end=1000
    if name=='slack-guardian':sc.camera=o
wld=bpy.data.worlds.new('Neutral editable desk');wld.color=(.36,.42,.41);sc.world=wld
bpy.ops.object.light_add(type='SUN',rotation=(.58,-.45,.64));bpy.context.object.data.energy=2.0
note=bpy.data.texts.new('Read first | Sunspindle Hollow');note.write('This editable desk contains the actual V1 cut, registered botanical assembly, world-space pigment and two locked cameras. The full native habitat, water, distant woodland and sky in the paired photographs are reference context, not counterfeit objects included here. No standing water or solid change.\n'+json.dumps(site,indent=2))
for s in bpy.data.screens:
 for ar in s.areas:
    if ar.type=='VIEW_3D':
        v=ar.spaces.active;v.region_3d.view_rotation=sc.camera.rotation_euler.to_quaternion();v.region_3d.view_distance=98;v.region_3d.view_location=(334,46,4);v.region_3d.view_perspective='ORTHO';v.shading.type='MATERIAL';v.overlay.show_overlays=False
bpy.ops.file.pack_all();bpy.ops.wm.save_as_mainfile(filepath=str(dest.resolve()),compress=True)
