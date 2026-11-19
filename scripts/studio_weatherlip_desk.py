"""Pack an editable native-triangle shoreline cut, never a screenshot facade."""
import bpy, json, struct, sys, math
from pathlib import Path
from mathutils import Vector
surface,field,dest=map(Path,sys.argv[sys.argv.index('--')+1:])
here=Path(__file__).resolve().parents[1]/'studies/weatherlip-cove'
sys.path.insert(0,str(here))
from material import shader
site=json.loads((field/'site.json').read_text())
raw=surface.read_bytes();w,d,ox,oz,cell,_=struct.unpack_from('<IIfffI',raw,8)
bpy.ops.wm.read_factory_settings(use_empty=True)
scene=bpy.context.scene
scene.name='Weatherlip / geological and living shore'
verts=[]; faces=[]; colors=[]
# A true excerpt of the V1 source, not sampled artist displacement.
ix0,ix1=26,71; iz0,iz1=152,270
nx=ix1-ix0+1
for j in range(iz0,iz1+1):
    for i in range(ix0,ix1+1):
        x,z=ox+i*cell,oz+j*cell
        h=struct.unpack_from('<f',raw,32+4*(j*w+i))[0]
        verts.append((x,-z,h))
        rgb=(.088,.16,.14) if h<0 else (.48,.40,.24) if h<3.2 else (.22,.28,.14)
        colors.append((*rgb,1))
        if i<ix1 and j<iz1:
            n=(j-iz0)*nx+i-ix0; faces.extend(((n,n+nx+1,n+1),(n,n+nx,n+nx+1)))
g=bpy.data.meshes.new('V1 original diagonal pair / native 3.125 m');g.from_pydata(verts,[],faces);g.update()
a=g.color_attributes.new(name='Base soil',type='FLOAT_COLOR',domain='POINT')
for col,val in zip(a.data,colors): col.color=val
bed=bpy.data.objects.new('V1 read-only reference | original coast cut',g);scene.collection.objects.link(bed)
mat=bpy.data.materials.new('Editable soil / chart on existing surface');mat.use_nodes=True
base=mat.node_tree.nodes.new('ShaderNodeVertexColor');base.layer_name='Base soil'
s=mat.node_tree.nodes.get('Principled BSDF');mat.node_tree.links.new(base.outputs['Color'],s.inputs['Base Color'])
s.inputs['Roughness'].default_value=.94;bed.data.materials.append(mat); shader([bed],field)
def collection(label):
    coll=bpy.data.collections.new(label);scene.collection.children.link(coll);return coll
for fname,label in [('living-margins.glb','02 / Salt meadow and glaucous rosettes'),('broken-beds.glb','03 / Contact-fitted cuffs, talus and wrack')]:
    before=set(bpy.data.objects); bpy.ops.import_scene.gltf(filepath=str((field/fname).resolve()))
    coll=collection(label)
    for ob in set(bpy.data.objects)-before:
        for other in list(ob.users_collection):other.objects.unlink(ob)
        coll.objects.link(ob)
coll=collection('04 / Original landmarks, positional guides only'); coll.hide_render=True
for k,v in site['landmarks'].items():
    x,y,z=v['position']; o=bpy.data.objects.new('#'+k+' '+v['kind']+' | original location',None)
    o.location=(x,-z,y);o.empty_display_type='CIRCLE';o.empty_display_size=.68;coll.objects.link(o)
# No substitute cliffs, grove or ocean panorama in this editable source.
# Only the real paired photographs use the complete native habitat context.
cams=collection('05 / Identical photograph stations')
for tag,e in site['stations'].items():
    _,eye,aim,ortho,lens,_=e
    cv=lambda p:Vector((p[0],-p[2],p[1]))
    c=bpy.data.cameras.new(tag);ob=bpy.data.objects.new(tag,c);cams.objects.link(ob)
    ob.location=cv(eye);ob.rotation_euler=(cv(aim)-cv(eye)).to_track_quat('-Z','Y').to_euler()
    c.type='ORTHO' if ortho else 'PERSP';c.lens=lens;c.ortho_scale=max(.1,ortho);c.clip_end=1000
    if tag=='bight-high':scene.camera=ob
world=bpy.data.worlds.new('Neutral source workshop');world.color=(.43,.48,.47);scene.world=world
bpy.ops.object.light_add(type='SUN',rotation=(.53,-.42,.52));bpy.context.object.data.energy=2.0
note=bpy.data.texts.new('Read this / Weatherlip')
note.write('Editable sculpture and world-space paint chart anchored to exact V1 ground. Open the four numbered collections. The native trees/rocks, sky and water visible in the photographic folio are not baked into or moved by this editable patch. Landmark circles are reference positions only; no fake stones or walkability change. The guardian and two true 1.04 m cameras remain fixed.\n')
note.write(json.dumps(site,indent=2))
for sc in bpy.data.screens:
    for ar in sc.areas:
        if ar.type=='VIEW_3D':
            sp=ar.spaces.active;sp.region_3d.view_rotation=scene.camera.rotation_euler.to_quaternion();sp.region_3d.view_distance=205;sp.region_3d.view_location=(-356,-49,6);sp.region_3d.view_perspective='ORTHO';sp.overlay.show_overlays=False;sp.shading.type='MATERIAL'
bpy.ops.file.pack_all();bpy.ops.wm.save_as_mainfile(filepath=str(dest.resolve()),compress=True)
