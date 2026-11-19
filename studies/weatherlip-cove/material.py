"""Paint the original surface; import only the registered cosmetic pieces."""
import json
from pathlib import Path
import bpy

def shader(meshes,folder):
    # The pre-existing bench builder adds its pigments to the current material, not
    # to a second ground sheet. Here there is deliberately no new sand mesh.
    import sys
    from importlib.util import spec_from_file_location,module_from_spec
    src=Path(__file__).resolve().parents[1]/'leeward-passage'/'material.py'
    spec=spec_from_file_location('shellback_chart',src);m=module_from_spec(spec);spec.loader.exec_module(m)
    m._bind_layer(meshes,folder,(),'site.json','weatherlip-ground.png',19.,.008,.33)
    # Mineral joints also change the normal; do not leave these as painted
    # charcoal marks. The same physical chart can be edited in the neutral desk.
    for mat in {s.material for o in meshes for s in o.material_slots if s.material}:
        nt=mat.node_tree
        if not nt:continue
        tex=next((q for q in nt.nodes if q.type=='TEX_IMAGE' and q.image and q.image.name.startswith('weatherlip-ground')),None)
        if not tex:continue
        bs=next(q for q in nt.nodes if q.type=='BSDF_PRINCIPLED')
        orig=bs.inputs['Normal']
        col=tex.outputs['Color']; a=tex.outputs['Alpha']
        bump=nt.nodes.new('ShaderNodeBump');bump.name='Shallow mineral joint relief'
        bump.inputs['Distance'].default_value=.052
        scale=nt.nodes.new('ShaderNodeMath');scale.operation='MULTIPLY'
        nt.links.new(a,scale.inputs[0]);scale.inputs[1].default_value=.48
        nt.links.new(col,bump.inputs['Height']);nt.links.new(scale.outputs[0],bump.inputs['Strength'])
        if orig.is_linked:nt.links.new(orig.links[0].from_socket,bump.inputs['Normal'])
        nt.links.new(bump.outputs[0],orig)

def apply(meshes,folder,tag,loc):
    shader(meshes,folder)
    bpy.ops.import_scene.gltf(filepath=str((folder/'living-margins.glb').resolve()))
    if tag!='bight-high':
        before=set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str((folder/'broken-beds.glb').resolve()))
        # Inspect the true near layers; protect the distant LOD from centimetre
        # stones gridding through a coarser hill. Identical limits at both eyes.
        for ob in set(bpy.data.objects)-before:
            if ob.type=='MESH' and abs(-ob.location.y-loc[2])>55:ob.hide_render=True
