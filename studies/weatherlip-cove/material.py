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
        kind=nt.nodes.new('ShaderNodeTexImage')
        kind.image=bpy.data.images.load(str((folder/'rock-sand-turf.png').resolve()),check_existing=True)
        kind.image.colorspace_settings.name='Non-Color';kind.extension='CLIP'
        nt.links.new(tex.inputs['Vector'].links[0].from_socket,kind.inputs['Vector'])
        rgb=nt.nodes.new('ShaderNodeSeparateColor');nt.links.new(kind.outputs['Color'],rgb.inputs[0])
        def channel(key):
            gate=nt.nodes.new('ShaderNodeMath');gate.operation='MULTIPLY'
            nt.links.new(a,gate.inputs[0]);nt.links.new(rgb.outputs[key],gate.inputs[1])
            return gate.outputs[0]
        grains(mat,folder,channel('Red'))
        grains(mat,folder,channel('Green'),sand=True)

def apply(meshes,folder,tag,loc):
    shader(meshes,folder)
    bpy.ops.import_scene.gltf(filepath=str((folder/'living-margins.glb').resolve()))
    if tag!='bight-high':
        before=set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str((folder/'broken-beds.glb').resolve()))
        # Inspect the true near layers; protect the distant LOD from centimetre
        # stones gridding through a coarser hill. Identical limits at both eyes.
        mineral_skin(set(bpy.data.objects)-before,folder)
        for ob in set(bpy.data.objects)-before:
            if ob.type=='MESH' and abs(-ob.location.y-loc[2])>55:ob.hide_render=True

def grains(mat,folder,site_mask=None,stone=False,sand=False):
    """Chipped metre-scale rock joints and centimetre quartz on real close faces.

    Use stable world metres and two unrelated rotations, not Generated UVs:
    otherwise every loose flake has the same stretch and colour patch.
    """
    nt=mat.node_tree
    if not nt:return
    nodes=nt.nodes;link=nt.links.new
    bs=next(n for n in nodes if n.type=='BSDF_PRINCIPLED')
    geo=nodes.new('ShaderNodeNewGeometry')
    dye=bpy.data.images.load(str((folder/('strand-grit.png' if sand else 'rind-pigment.png')).resolve()),check_existing=True)
    ht=bpy.data.images.load(str((folder/('strand-grit.png' if sand else 'rind-relief.png')).resolve()),check_existing=True)
    dye.colorspace_settings.name='Non-Color';ht.colorspace_settings.name='Non-Color'
    # Position is transformed into the two axes of local strike: relief can be
    # seen on the hillside as well as on a nearly horizontal flat.
    sep=nodes.new('ShaderNodeSeparateXYZ');link(geo.outputs['Position'],sep.inputs[0])
    xz=nodes.new('ShaderNodeMath');xz.operation='MULTIPLY_ADD';link(sep.outputs['Z'],xz.inputs[0]);xz.inputs[1].default_value=.57;link(sep.outputs['X'],xz.inputs[2])
    planar=nodes.new('ShaderNodeCombineXYZ');link(xz.outputs[0],planar.inputs[0]);link(sep.outputs['Y'],planar.inputs[1])
    settings = ((1.55,.4,.62,.0038),(.23,-.16,.32,.0015)) if sand else ((1.38,.37,.95,.027),(.281,-.68,.57,.008))
    for metres,angle,power,depth in settings:
        rot=nodes.new('ShaderNodeVectorRotate');rot.rotation_type='Z_AXIS';link(planar.outputs[0],rot.inputs['Vector']);rot.inputs['Angle'].default_value=angle
        size=nodes.new('ShaderNodeVectorMath');size.operation='SCALE';link(rot.outputs['Vector'],size.inputs[0]);size.inputs['Scale'].default_value=1/metres
        colour=nodes.new('ShaderNodeTexImage');colour.image=dye;colour.label='Mineral flecks, physical metres';link(size.outputs['Vector'],colour.inputs['Vector'])
        height=nodes.new('ShaderNodeTexImage');height.image=ht;link(size.outputs['Vector'],height.inputs['Vector'])
        gain=nodes.new('ShaderNodeMath');gain.operation='MULTIPLY';gain.inputs[0].default_value=1.
        gain.inputs[1].default_value=power if stone else power*.85
        if site_mask:link(site_mask,gain.inputs[0])
        main=bs.inputs['Base Color'];base=main.links[0].from_socket if main.is_linked else None
        mix=nodes.new('ShaderNodeMixRGB');mix.blend_type='OVERLAY';link(gain.outputs[0],mix.inputs[0]);link(colour.outputs['Color'],mix.inputs[2])
        if base:link(base,mix.inputs[1])
        else:mix.inputs[1].default_value=main.default_value
        link(mix.outputs[0],main)
        normal=bs.inputs['Normal'];bump=nodes.new('ShaderNodeBump');bump.inputs['Distance'].default_value=depth if stone else depth*.56
        link(gain.outputs[0],bump.inputs['Strength']);link(height.outputs['Color'],bump.inputs['Height'])
        if normal.is_linked:link(normal.links[0].from_socket,bump.inputs['Normal'])
        link(bump.outputs[0],normal)
    if stone:
        dry=nodes.new('ShaderNodeMapRange');dry.clamp=True;link(sep.outputs['Z'],dry.inputs[0])
        for key,val in [('From Min',.16),('From Max',1.48),('To Min',.43),('To Max',.86)]:dry.inputs[key].default_value=val
        link(dry.outputs['Result'],bs.inputs['Roughness'])

def mineral_skin(objects,folder):
    # The source GLB shares one colour material. Split only the near stone
    # shader; wrack keeps its own waxier terrestrial/algal vertex colours.
    donors={}
    for ob in objects:
        if ob.type!='MESH' or not (ob.name.startswith('Broken beds') or ob.name.startswith('Fissured pavement')):continue
        for s in ob.material_slots:
            m=s.material
            if not m:continue
            if m not in donors:
                clone=m.copy();clone.name='Interlocking beds and quartz rind';grains(clone,folder,stone=True)
                donors[m]=clone
            s.material=donors[m]
