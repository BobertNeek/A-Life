"""Inspect the actual reimported terrain GLB, preserving the saved studio cameras."""
import json
from pathlib import Path
import bpy
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'crates/alife_game_app/assets/landscape/highlands'
OUT=ROOT/'target/artifacts/terrain-v2'
bpy.ops.wm.open_mainfile(filepath=str(HERE/'mountain-valley.blend'))
for ob in list(bpy.context.scene.objects):
    if ob.type not in {'CAMERA','LIGHT'}: bpy.data.objects.remove(ob,do_unlink=True)
bpy.ops.import_scene.gltf(filepath=str(HERE/'Mountain_Valley.glb'))
scene=bpy.context.scene
ground=bpy.data.objects['Highlands_continuous_terrain']
bounds=[ground.matrix_world@Vector(p) for p in ground.bound_box]
extent=[max(p[i] for p in bounds)-min(p[i] for p in bounds) for i in range(3)]
assert abs(extent[0]-800)<.01 and abs(extent[1]-1050)<.01, extent
assert extent[2]>100, extent
assert ground.data.color_attributes, 'Missing terrain vertex colors'
scene.render.engine='CYCLES'
scene.cycles.samples=16
for name,camera in [('highlands-wide','Highlands_wide'),('highlands-ground','Highlands_ground')]:
    scene.camera=bpy.data.objects[camera]
    scene.render.filepath=str(OUT/(name+'.png'))
    bpy.ops.render.render(write_still=True)
# Identical camera/light geometry review separates silhouette from material detail.
clay=bpy.data.materials.new('Highlands review clay');clay.diffuse_color=(.5,.5,.5,1)
scene.view_layers[0].material_override=clay
scene.camera=bpy.data.objects['Highlands_wide']
scene.render.resolution_percentage=60
scene.render.filepath=str(OUT/'highlands-clay.png')
bpy.ops.render.render(write_still=True)
receipt={'blender':bpy.app.version_string,'source':'reimported Mountain_Valley.glb',
         'terrain_extent_m':extent,'views':['highlands-wide.png','highlands-ground.png','highlands-clay.png'],
         'runtime_integration_verified':False}
(OUT/'render-receipt.json').write_text(json.dumps(receipt,indent=2))
print('HIGHLANDS_GLB_REVIEW_COMPLETE',json.dumps(receipt),flush=True)
