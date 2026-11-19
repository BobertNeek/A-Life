"""The native triangles keep their normals/maps. Paint at fixed world scale."""
import bpy,json
from pathlib import Path
def bind(meshes,folder):
    x0,x1,z0,z1=json.loads((folder/'route.json').read_text())['bounds']
    image=bpy.data.images.load(str((folder/'reach-wash.png').resolve()),check_existing=True)
    image.alpha_mode='CHANNEL_PACKED'
    mats={slot.material for o in meshes for slot in o.material_slots if slot.material}
    for m in mats:
        if not m.use_nodes:continue
        tree=m.node_tree;n=tree.nodes.new;link=tree.links.new
        sh=next(q for q in tree.nodes if q.type=='BSDF_PRINCIPLED')
        geo=n('ShaderNodeNewGeometry');position=geo.outputs['Position']
        a=n('ShaderNodeVectorMath');a.operation='ADD';link(position,a.inputs[0]);a.inputs[1].default_value=(-x0,z1,0)
        s=n('ShaderNodeVectorMath');s.operation='MULTIPLY';link(a.outputs[0],s.inputs[0]);s.inputs[1].default_value=(1/(x1-x0),1/(z1-z0),0)
        t=n('ShaderNodeTexImage');t.image=image;t.extension='CLIP';link(s.outputs[0],t.inputs['Vector'])
        mix=n('ShaderNodeMixRGB');mix.label='Windward humus, till and shell';main=sh.inputs['Base Color']
        if main.is_linked:link(main.links[0].from_socket,mix.inputs[1])
        else:mix.inputs[1].default_value=main.default_value[:]
        link(t.outputs['Alpha'],mix.inputs[0]);link(t.outputs['Color'],mix.inputs[2]);link(mix.outputs[0],main)
        # Small, masked surface tooth. A centimetre of wrinkled lamination,
        # not vector displacement or relief beneath a solid prop.
        noise=n('ShaderNodeTexNoise');noise.noise_dimensions='3D';noise.inputs['Scale'].default_value=23.
        noise.inputs['Detail'].default_value=2.;link(position,noise.inputs['Vector'])
        bump=n('ShaderNodeBump');bump.inputs['Distance'].default_value=.007;gain=n('ShaderNodeMath');gain.operation='MULTIPLY';link(t.outputs['Alpha'],gain.inputs[0]);gain.inputs[1].default_value=.31;link(gain.outputs[0],bump.inputs['Strength'])
        link(noise.outputs['Fac'],bump.inputs['Height']);norm=sh.inputs['Normal']
        if norm.is_linked:link(norm.links[0].from_socket,bump.inputs['Normal'])
        link(bump.outputs[0],norm)
def growth(folder,guardian):
    before=set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str((folder/'reach-detail.glb').resolve()))
    for ob in set(bpy.data.objects)-before:
        if ob.type=='MESH' and guardian and ob.name.endswith('flakes'):
            # Loose centimetre-scale valves and clasts aren't visible at this
            # scale. The physical island and the larger plant crowns stay put.
            ob.hide_render=True
