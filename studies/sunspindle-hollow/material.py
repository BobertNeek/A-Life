"""Registered pigment and local ground response on the existing terrain material."""
import bpy, json
from pathlib import Path
from importlib.util import spec_from_file_location,module_from_spec

def coat(meshes,field):
    src=Path(__file__).resolve().parents[1]/'leeward-passage'/'material.py'
    sp=spec_from_file_location('held_ground_chart',src);old=module_from_spec(sp);sp.loader.exec_module(old)
    old._bind_layer(meshes,field,(),'site.json','hollow-ground.png',26.,.007,.27)
    for mat in {s.material for o in meshes for s in o.material_slots if s.material}:
        nt=mat.node_tree
        if not nt:continue
        tx=next((n for n in nt.nodes if n.type=='TEX_IMAGE' and n.image and n.image.name.startswith('hollow-ground')),None)
        if not tx:continue
        bs=next(n for n in nt.nodes if n.type=='BSDF_PRINCIPLED')
        rough=bs.inputs['Roughness'];mix=nt.nodes.new('ShaderNodeMixRGB');mix.label='Dry mica and fibrous bed'
        if rough.is_linked:nt.links.new(rough.links[0].from_socket,mix.inputs[1])
        else:mix.inputs[1].default_value=(rough.default_value,)*3+(1,)
        # Coarse mineral and shoot litter cross-grain, without any displacement.
        pos=nt.nodes.new('ShaderNodeNewGeometry').outputs['Position']
        stretch=nt.nodes.new('ShaderNodeVectorMath');stretch.operation='MULTIPLY';stretch.inputs[1].default_value=(3.0,.83,1.6)
        nt.links.new(pos,stretch.inputs[0])
        wave=nt.nodes.new('ShaderNodeTexNoise');wave.inputs['Scale'].default_value=7.2; wave.inputs['Detail'].default_value=2.4;wave.inputs['Roughness'].default_value=.53;nt.links.new(stretch.outputs[0],wave.inputs['Vector'])
        ramp=nt.nodes.new('ShaderNodeMapRange');ramp.inputs['To Min'].default_value=.68;ramp.inputs['To Max'].default_value=.94;nt.links.new(wave.outputs['Fac'],ramp.inputs['Value'])
        nt.links.new(tx.outputs['Alpha'],mix.inputs[0]);nt.links.new(ramp.outputs[0],mix.inputs[2]);nt.links.new(mix.outputs[0],rough)
        ht=bs.inputs['Normal'];b=nt.nodes.new('ShaderNodeBump');b.inputs['Distance'].default_value=.004;b.inputs['Strength'].default_value=.23
        nt.links.new(wave.outputs['Fac'],b.inputs['Height'])
        if ht.is_linked:nt.links.new(ht.links[0].from_socket,b.inputs['Normal'])
        # Critical: mask the bump as well as the pigment outside the art site.
        k=nt.nodes.new('ShaderNodeMath');k.operation='MULTIPLY';k.inputs[1].default_value=.23;nt.links.new(tx.outputs['Alpha'],k.inputs[0]);nt.links.new(k.outputs[0],b.inputs['Strength'])
        nt.links.new(b.outputs[0],ht)

def plant(path):
    bpy.ops.import_scene.gltf(filepath=str(path.resolve()))
