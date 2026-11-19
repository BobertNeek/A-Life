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
        fade=distance_weight(nt,12.,28.);inverse=nt.nodes.new('ShaderNodeMath');inverse.operation='SUBTRACT';inverse.inputs[0].default_value=1.
        nt.links.new(fade,inverse.inputs[1]);strength=nt.nodes.new('ShaderNodeMath');strength.operation='MULTIPLY'
        nt.links.new(channels.outputs[2],strength.inputs[0]);nt.links.new(inverse.outputs[0],strength.inputs[1])
        nt.links.new(strength.outputs[0],bump.inputs['Strength']);nt.links.new(channels.outputs[1],bump.inputs['Height'])
        base=bs.inputs['Normal']
        if base.is_linked:nt.links.new(base.links[0].from_socket,bump.inputs['Normal'])
        nt.links.new(bump.outputs['Normal'],base)


def distance_weight(nt,lo,hi):
    camera=nt.nodes.new('ShaderNodeCameraData')
    weight=nt.nodes.new('ShaderNodeMapRange');weight.clamp=True;weight.interpolation_type='SMOOTHSTEP'
    nt.links.new(camera.outputs['View Distance'],weight.inputs['Value'])
    weight.inputs['From Min'].default_value=lo;weight.inputs['From Max'].default_value=hi
    return weight.outputs['Result']

def pigment(ground,surf,source,meta):
    """Continuous real-camera distance blend, rather than removing a floor at LOD."""
    a,b,c,d=meta['native_plan_xz'];lo,hi=meta['native_camera_micro_to_macro_blend_m']
    scenes={}
    for key in ('pineglass-mineral','pineglass-mineral-far'):
        im=bpy.data.images.load(str((source/(key+'.png')).resolve()),check_existing=True);im.colorspace_settings.name='Non-Color';scenes[key]=im
    for material in {s.material for obj in ground for s in obj.material_slots if s.material and s.material.use_nodes}:
        nt=material.node_tree
        uv=surf.coords(nt,(0,1/(b-a),-a/(b-a)),(1,1/(d-c),d/(d-c)))
        mix=nt.nodes.new('ShaderNodeMixRGB');mix.blend_type='MIX';nt.links.new(distance_weight(nt,lo,hi),mix.inputs[0])
        for slot,(key,im) in enumerate(scenes.items(),1):
            pic=nt.nodes.new('ShaderNodeTexImage');pic.label=key;pic.extension='EXTEND';pic.image=im;nt.links.new(uv,pic.inputs['Vector']);nt.links.new(pic.outputs['Color'],mix.inputs[slot])
        convert=nt.nodes.new('ShaderNodeVectorMath');convert.operation='SCALE';convert.inputs['Scale'].default_value=255/meta['neutral']
        nt.links.new(mix.outputs[0],convert.inputs[0])
        inp=surf.bsdf(material).inputs['Base Color'];mul=nt.nodes.new('ShaderNodeVectorMath');mul.operation='MULTIPLY'
        if inp.is_linked:nt.links.new(inp.links[0].from_socket,mul.inputs[0])
        else:mul.inputs[0].default_value=inp.default_value[:3]
        nt.links.new(convert.outputs[0],mul.inputs[1]);nt.links.new(mul.outputs[0],inp)
