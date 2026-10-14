"""CPU-only, fixed-light review of the actual runtime GLBs."""
import argparse
import math
import sys
from pathlib import Path
import bpy
from mathutils import Vector

args = sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
p = argparse.ArgumentParser()
p.add_argument('--asset-root', required=True)
p.add_argument('--output', required=True)
p.add_argument('--lod', type=int, default=0)
p.add_argument('--title', default='Current main')
a = p.parse_args(args)
kinds = ['BerryBush', 'CoastalShrub']
bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)
scene = bpy.context.scene
scene.render.engine = 'CYCLES'
scene.cycles.device = 'CPU'
scene.cycles.samples = 32
scene.cycles.use_denoising = False
scene.render.resolution_x = 1400
scene.render.resolution_y = 800
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = 'PNG'
scene.view_settings.view_transform = 'AgX'
scene.world.use_nodes = True
bg = scene.world.node_tree.nodes.get('Background')
bg.inputs['Color'].default_value = (0.18, 0.24, 0.29, 1)
bg.inputs['Strength'].default_value = 0.65
for i, kind in enumerate(kinds):
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(Path(a.asset_root)/(kind+'.glb')))
    imported = set(bpy.data.objects)-old
    chosen = next(o for o in imported if o.type == 'MESH' and o.name == kind+'_L'+str(a.lod))
    for o in imported:
        if o is not chosen:
            bpy.data.objects.remove(o, do_unlink=True)
    chosen.location.x = (i-0.5)*2.85

def mat(name, color):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    n = m.node_tree.nodes.get('Principled BSDF')
    n.inputs['Base Color'].default_value = (*color, 1)
    n.inputs['Roughness'].default_value = 0.88
    return m

ground = mat('Review ground only', (0.055, 0.068, 0.075))
bpy.ops.mesh.primitive_plane_add(size=200, location=(0,0,-0.025))
bpy.context.object.data.materials.append(ground)
sun = bpy.data.lights.new('Review key', 'SUN')
sun.energy = 2.4
sun.angle = 0.12
o = bpy.data.objects.new('Review key', sun)
scene.collection.objects.link(o)
o.rotation_euler = (0.45,-0.50,-0.50)
light = bpy.data.lights.new('Review fill', 'AREA')
light.energy = 400
light.shape = 'DISK'
light.size = 9
o = bpy.data.objects.new('Review fill', light)
scene.collection.objects.link(o)
o.location = (0,-5,7)
o.rotation_euler = (Vector((0,0,0.5))-o.location).to_track_quat('-Z','Y').to_euler()
camera = bpy.data.cameras.new('Locked review camera')
camera.type = 'ORTHO'
camera.ortho_scale = 6.5
o = bpy.data.objects.new('Locked review camera', camera)
scene.collection.objects.link(o)
o.location = (0,-13,9)
o.rotation_euler = (Vector((0,0,0.65))-o.location).to_track_quat('-Z','Y').to_euler()
scene.camera = o
ink = mat('Review labels only', (0.72,0.80,0.84))
for i,kind in enumerate(kinds):
    t = bpy.data.curves.new(kind+' label', 'FONT')
    t.body = kind
    t.align_x = 'CENTER'
    t.size = 0.22
    o = bpy.data.objects.new(kind+' label',t)
    scene.collection.objects.link(o)
    o.location = ((i-0.5)*2.85,-1.55,0.10)
    o.rotation_euler = scene.camera.rotation_euler
    t.materials.append(ink)
t = bpy.data.curves.new('Review title', 'FONT')
t.body = a.title+'  |  LOD '+str(a.lod)+'  |  Runtime GLB / CPU Cycles'
t.align_x = 'CENTER'
t.size = 0.14
o = bpy.data.objects.new('Review title',t)
scene.collection.objects.link(o)
o.location = (0,1.3,1.8)
o.rotation_euler = scene.camera.rotation_euler
t.materials.append(ink)
out = Path(a.output)
out.parent.mkdir(parents=True, exist_ok=True)
scene.render.filepath = str(out)
bpy.ops.render.render(write_still=True)
print('REVIEW_RENDER',str(out),flush=True)
