"""Near-only mineral floor optics; original terrain points remain untouched."""
import bpy
def optics(ground,surf,source,box):
    a,b,c,d=box
    img=bpy.data.images.load(str((source/'pineglass-response.png').resolve()),check_existing=True)
    img.colorspace_settings.name='Non-Color'
    for material in {s.material for obj in ground for s in obj.material_slots if s.material and s.material.use_nodes}:
        nt=material.node_tree;bs=surf.bsdf(material)
        uv=surf.coords(nt,(0,1/(b-a),-a/(b-a)),(1,1/(d-c),d/(d-c)))
        tex=nt.nodes.new('ShaderNodeTexImage');tex.image=img;tex.extension='EXTEND';nt.links.new(uv,tex.inputs['Vector'])
        channels=nt.nodes.new('ShaderNodeSeparateColor');nt.links.new(tex.outputs['Color'],channels.inputs[0])
        m=nt.nodes.new('ShaderNodeMixRGB');m.blend_type='MIX';nt.links.new(channels.outputs[2],m.inputs[0])
        rough=bs.inputs['Roughness']
        if rough.is_linked:nt.links.new(rough.links[0].from_socket,m.inputs[1])
        else:m.inputs[1].default_value=(*([rough.default_value]*3),1)
        nt.links.new(channels.outputs[0],m.inputs[2]);nt.links.new(m.outputs[0],rough)
        bump=nt.nodes.new('ShaderNodeBump');bump.inputs['Distance'].default_value=.0038
        nt.links.new(channels.outputs[2],bump.inputs['Strength']);nt.links.new(channels.outputs[1],bump.inputs['Height'])
        base=bs.inputs['Normal']
        if base.is_linked:nt.links.new(base.links[0].from_socket,bump.inputs['Normal'])
        nt.links.new(bump.outputs['Normal'],base)
