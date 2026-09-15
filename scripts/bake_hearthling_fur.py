"""Bake fine directional coat relief into a portable tangent normal atlas."""
import bpy
import math


def bake_fur():
    scene=bpy.context.scene
    rig=bpy.data.objects['Hearthling_Rig']; rig.data.pose_position='REST'
    coat=bpy.data.materials['Hearthling | vertex-colored coat']
    objects=[o for o in scene.objects if o.type=='MESH' and o.name.startswith('Hearthling')
             and coat in o.data.materials[:]]
    bpy.ops.object.select_all(action='DESELECT')
    for ob in objects: ob.select_set(True)
    bpy.context.view_layer.objects.active=objects[0]
    bpy.ops.object.mode_set(mode='EDIT'); bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.smart_project(angle_limit=math.radians(66),island_margin=.010)
    bpy.ops.object.mode_set(mode='OBJECT')
    nodes=coat.node_tree.nodes; links=coat.node_tree.links
    shader=nodes.get('Principled BSDF')
    coord=nodes.new('ShaderNodeTexCoord')
    scale=nodes.new('ShaderNodeVectorMath'); scale.operation='MULTIPLY'
    scale.inputs[1].default_value=(90,90,15)
    noise=nodes.new('ShaderNodeTexNoise'); noise.inputs['Scale'].default_value=1
    noise.inputs['Detail'].default_value=2; noise.inputs['Roughness'].default_value=.65
    bump=nodes.new('ShaderNodeBump'); bump.inputs['Strength'].default_value=.30
    bump.inputs['Distance'].default_value=.006
    links.new(coord.outputs['Generated'],scale.inputs[0])
    links.new(scale.outputs['Vector'],noise.inputs['Vector'])
    links.new(noise.outputs['Fac'],bump.inputs['Height'])
    links.new(bump.outputs['Normal'],shader.inputs['Normal'])
    atlas=bpy.data.images.new('Hearthling_FurNormal',width=2048,height=2048,alpha=False)
    atlas.generated_color=(.5,.5,1,1); atlas.colorspace_settings.name='Non-Color'
    texture=nodes.new('ShaderNodeTexImage'); texture.image=atlas; nodes.active=texture
    previous=scene.render.engine; scene.render.engine='CYCLES'
    scene.cycles.samples=1
    bpy.ops.object.bake(type='NORMAL',normal_space='TANGENT',use_clear=False,margin=8)
    for node in [coord,scale,noise,bump]: nodes.remove(node)
    normal=nodes.new('ShaderNodeNormalMap'); normal.inputs['Strength'].default_value=.70
    links.new(texture.outputs['Color'],normal.inputs['Color'])
    links.new(normal.outputs['Normal'],shader.inputs['Normal'])
    # Preserve the crown's named appearance-tint slot after joining its geometry.
    old_gold=bpy.data.materials['Hearthling | warm ochre']
    bpy.data.materials.remove(old_gold)
    gold=coat.copy(); gold.name='Hearthling | warm ochre'
    head=bpy.data.objects['Hearthling_Head']; head.data.materials.append(gold)
    for poly in head.data.polygons:
        if poly.center.z>1.91: poly.material_index=len(head.data.materials)-1
    atlas.pack(); scene.render.engine=previous; rig.data.pose_position='POSE'
    print('HEARTHLING_FUR_NORMAL_BAKED',flush=True)
