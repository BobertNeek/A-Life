"""Keep the archive intact. Start a separate editable coastal rock workbench."""
from pathlib import Path
import sys,json
import bpy
base,library,seats,dest=map(Path,sys.argv[sys.argv.index('--')+1:])
bpy.ops.wm.open_mainfile(filepath=str(base.resolve()))
scene=bpy.context.scene;scene.name='Weatherlip / original and fitted solids'
def coll(name):
    c=bpy.data.collections.new(name);scene.collection.children.link(c);return c
old=coll('06 / Former boulder shapes | hidden comparison')
fitted=coll('07 / Fitted boulders | editable')
bounds=coll('08 / Saved physical boxes | optional, never rendered')
old.hide_viewport=True;old.hide_render=True;bounds.hide_render=True;bounds.hide_viewport=True
reports=json.loads((seats/'contact.json').read_text())
native=lambda p:(p[0],-p[2],p[1])
def model(path,n,group,prop):
    before=set(bpy.data.objects);bpy.ops.import_scene.gltf(filepath=str(path.resolve()))
    fresh=set(bpy.data.objects)-before
    kept=next(o for o in fresh if o.name.startswith(n+'_L0') and o.type=='MESH')
    for o in list(fresh):
        if o!=kept:bpy.data.objects.remove(o,do_unlink=True)
    for c in list(kept.users_collection):c.objects.unlink(kept)
    group.objects.link(kept);kept.location=native(prop['position']);kept.rotation_euler=(0,0,prop['yaw']);kept.scale=[prop['scale']]*3
for key,r in reports.items():
    kind=r['kind'];prop=r['native_parent'];model(library/(kind+'.glb'),kind,old,prop);model(seats/(kind+'Seat'+key+'.glb'),kind+'Seat'+key,fitted,prop)
    x0,z0,x1,z1,y0,y1=r['native_box_xmin_zmin_xmax_zmax_ymin_ymax']
    o=bpy.data.objects.new('Original box '+str(r['unchanged_solid_index'])+' | carrier '+key,None)
    bounds.objects.link(o);o.empty_display_type='CUBE';o.empty_display_size=1
    o.location=((x0+x1)/2,-(z0+z1)/2,(y0+y1)/2)
    o.scale=((x1-x0)/2,(z1-z0)/2,(y1-y0)/2)
note=bpy.data.texts.new('Stone seats / physical relationship');note.write('Both editable replacement silhouettes are INSIDE their original plan. The faint remaining downhill air is real because the original bed lies below the bottom of the saved opaque volume. Only the rock can cast the new shadow. Disable 07 and enable 06 to compare the former shapes. The soil, original cove materials, lights, cameras and foliage are preserved from the preceding artist desk.\n\n'+json.dumps(reports,indent=2))
# Pack both generations of textures from the two real donor GLBs.
bpy.ops.file.pack_all();bpy.ops.wm.save_as_mainfile(filepath=str(dest.resolve()),compress=True)
