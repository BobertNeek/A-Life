"""Export the approved saved character with shared-material meshes consolidated.
The editable Blender source remains untouched. Run after export_hearthling.py.
"""
import bpy, json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'crates/alife_game_app/assets/creatures/hearthling'
bpy.ops.wm.open_mainfile(filepath=str(HERE/'hearthling.blend'))
normal=bpy.data.images['Hearthling_FurNormal']
normal.scale(1024,1024)
normal.pack()
rig=bpy.data.objects['Hearthling_Rig']
models=[o for o in bpy.context.scene.objects if o.type=='MESH' and o.name.startswith('Hearthling')]
print('BEFORE',json.dumps([{ 'name':o.name,'modifiers':[m.type for m in o.modifiers]} for o in models]),flush=True)
# Apply geometric modifiers individually before joining; keep the common armature.
for ob in models:
    bpy.ops.object.select_all(action='DESELECT');ob.select_set(True);bpy.context.view_layer.objects.active=ob
    for modifier in list(ob.modifiers):
        if modifier.type!='ARMATURE':bpy.ops.object.modifier_apply(modifier=modifier.name)
    # glTF/Bevy use COLOR_0. Merge the authored coat/eye channels into one
    # per-corner stream before joining, including white for constant materials.
    attr=ob.data.color_attributes.new(name='RuntimeColor',type='FLOAT_COLOR',domain='CORNER')
    for polygon in ob.data.polygons:
        mat=ob.data.materials[polygon.material_index]
        node=next((n for n in mat.node_tree.nodes if n.type=='VERTEX_COLOR'),None) if mat and mat.use_nodes else None
        source=ob.data.color_attributes.get(node.layer_name) if node else None
        for index in polygon.loop_indices:
            attr.data[index].color=source.data[index if source.domain=='CORNER' else ob.data.loops[index].vertex_index].color if source else (1,1,1,1)
    for old in list(ob.data.color_attributes):
        if old.name!='RuntimeColor':ob.data.color_attributes.remove(old)
    ob.data.color_attributes.active_color=attr
for mat in {mat for ob in models for mat in ob.data.materials if mat and mat.use_nodes}:
    for node in mat.node_tree.nodes:
        if node.type=='VERTEX_COLOR':node.layer_name='RuntimeColor'
bpy.ops.object.select_all(action='DESELECT')
for ob in models:ob.select_set(True)
bpy.context.view_layer.objects.active=models[0]
bpy.ops.object.join()
mesh=bpy.context.object;mesh.name='Hearthling_Runtime'
# Join deduplicates material slots; glTF emits one draw primitive per material.
assert len([m for m in mesh.modifiers if m.type=='ARMATURE'])==1
bpy.ops.object.select_all(action='DESELECT');mesh.select_set(True);rig.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(HERE/'hearthling.glb'),export_format='GLB',use_selection=True,
    export_animations=True,export_animation_mode='ACTIONS',export_force_sampling=True,export_skins=True,
    export_cameras=False,export_lights=False,export_vertex_color='ACTIVE')
print('OPTIMIZED_HEARTHLING_EXPORTED',flush=True)
