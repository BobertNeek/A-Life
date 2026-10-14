"""Create/reload a Conifer-only editable reconstruction, then guarded export.

The source is reconstructed directly from the pinned runtime vertices/triangles,
not extracted from or substituted into the original multi-asset .blend. Exact
shipping export uses source vertex coordinates and independently recalculated
incident normals in the unchanged baseline binary layout. A normal Blender
native export is diagnostic only because its serializer can reorder vertices,
quantize custom normals and expand attributes. The audited adapter supports only
this fixed repair; other source edits deliberately fail its exact-output guard.
"""
import argparse
import json
import sys
from pathlib import Path

import bpy
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from derive_conifer_leader import derive_bytes
from conifer_glb import Glb
from repair_conifer_leader import BASE_SHA256, FIXED_SHA256, ROOT, repair_bytes, sha


def author(baseline, source):
    fixed = Glb(repair_bytes(baseline.read_bytes()))
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    for level, spec in enumerate(fixed.doc['meshes']):
        p = spec['primitives'][0]
        positions = fixed.rows(p['attributes']['POSITION'])
        normals = fixed.rows(p['attributes']['NORMAL'])
        indices = [r[0] for r in fixed.rows(p['indices'])]
        faces = [tuple(indices[i:i+3]) for i in range(0, len(indices), 3)]
        mesh = bpy.data.meshes.new(spec['name'])
        mesh.from_pydata([(x, -z, y) for x, y, z in positions], [], faces)
        mesh.update()
        ob = bpy.data.objects.new(spec['name'], mesh)
        bpy.context.scene.collection.objects.link(ob)
        for poly in mesh.polygons:
            poly.use_smooth = True
        mesh.normals_split_custom_set_from_vertices([(x, -z, y) for x, y, z in normals])
        colors = fixed.rows(p['attributes']['COLOR_0'])
        a = fixed.doc['accessors'][p['attributes']['COLOR_0']]
        if a['componentType'] != 5121 or not a.get('normalized'):
            raise ValueError('Expected original normalized-byte colors')
        attr = mesh.color_attributes.new(name='ArtColor', type='FLOAT_COLOR', domain='POINT')
        for entry, color in zip(attr.data, colors):
            entry.color = (*[c/255 for c in color[:3]], color[3]/255 if len(color) == 4 else 1)
        mesh.color_attributes.active_color = attr
        mat = bpy.data.materials.get('Island shared painted surface')
        if mat is None:
            mat = bpy.data.materials.new('Island shared painted surface')
            mat.use_nodes = True
            bsdf = next(n for n in mat.node_tree.nodes if n.type == 'BSDF_PRINCIPLED')
            original = fixed.doc['materials'][0]['pbrMetallicRoughness']
            bsdf.inputs['Base Color'].default_value = original.get('baseColorFactor', [1, 1, 1, 1])
            bsdf.inputs['Metallic'].default_value = original.get('metallicFactor', 1)
            bsdf.inputs['Roughness'].default_value = original.get('roughnessFactor', 1)
            color_node = mat.node_tree.nodes.new('ShaderNodeVertexColor')
            color_node.layer_name = 'ArtColor'
            mat.node_tree.links.new(color_node.outputs['Color'], bsdf.inputs['Base Color'])
            mat.use_backface_culling = not fixed.doc['materials'][0].get('doubleSided', False)
        mesh.materials.append(mat)
        ob['source_provenance'] = 'Reconstructed from exact runtime GLB; original island-assets.blend unchanged'
        ob['baseline_sha256'] = BASE_SHA256
        ob['candidate_sha256'] = FIXED_SHA256
        ob['vertex_identity'] = 'Mesh vertex index equals pinned GLB POSITION index'
        ob['coordinate_contract'] = 'metres, source Z-up; runtime Y-up; origin ground-centred'
        ob.hide_set(level != 0)
        ob.hide_render = level != 0
    scene = bpy.context.scene
    scene.unit_settings.system = 'METRIC'
    scene.unit_settings.scale_length = 1
    scene['source_provenance'] = 'Conifer-only runtime reconstruction, not original monolithic authoring source'
    bpy.context.preferences.filepaths.save_version = 0
    source.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(source))


def export_source(source, baseline, output, native, receipt):
    base = Glb(baseline)
    expected = Glb(repair_bytes(base.raw))
    bpy.ops.wm.open_mainfile(filepath=str(source))
    if {o.name for o in bpy.data.objects if o.type == 'MESH'} != {'Conifer_L0', 'Conifer_L1', 'Conifer_L2'}:
        raise ValueError('Unexpected source mesh set')
    bpy.ops.object.select_all(action='DESELECT')
    source_positions, rows = {}, []
    for spec in base.doc['meshes']:
        p = spec['primitives'][0]
        ob = bpy.data.objects[spec['name']]
        if tuple(ob.location) != (0., 0., 0.) or tuple(ob.scale) != (1., 1., 1.) or any(ob.rotation_euler):
            raise ValueError('Source object transform changed')
        mesh = ob.data
        indices = [r[0] for r in base.rows(p['indices'])]
        actual_indices = [i for poly in mesh.polygons for i in poly.vertices]
        if actual_indices != indices or len(mesh.vertices) != base.doc['accessors'][p['attributes']['POSITION']]['count']:
            raise ValueError('Source topology changed')
        positions = [(v.co.x, v.co.z, -v.co.y) for v in mesh.vertices]
        source_positions[spec['name']] = positions
        original_colors = base.rows(p['attributes']['COLOR_0'])
        attr = mesh.color_attributes.get('ArtColor')
        if attr is None or attr.domain != 'POINT' or len(mesh.materials) != 1:
            raise ValueError('Source material/color schema changed')
        colors = [tuple(round(c*255) for c in entry.color[:3]) for entry in attr.data]
        if colors != [r[:3] for r in original_colors]:
            raise ValueError('Source colors changed')
        expected_normals = expected.rows(p['attributes']['NORMAL'])
        error = 0.
        for loop, normal in zip(mesh.loops, mesh.corner_normals):
            x, y, z = normal.vector
            error = max(error, max(abs(a-b) for a, b in zip((x, z, -y), expected_normals[loop.vertex_index])))
        # Blender stores custom normals in a quantized native representation.
        # This diagnostic tolerance is never applied to the shipping bytes,
        # whose normals are independently recalculated and hash-guarded.
        if error > .0005:
            raise ValueError('Unexpected editable-source normal discrepancy')
        rows.append({'mesh': spec['name'], 'vertices': len(mesh.vertices), 'triangles': len(mesh.polygons),
                     'positions_read_from_reloaded_source': True, 'triangle_indices_exact': True,
                     'colors_recover_original_bytes': True, 'custom_normal_quantization_max_abs_error': error})
        ob.hide_set(False)
        ob.hide_render = False
        ob.select_set(True)
    # Actual source coordinates, not stored output GLB bytes, drive the adapter.
    data, derivation = derive_bytes(base.raw, source_positions=source_positions)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(data)
    native.parent.mkdir(parents=True, exist_ok=True)
    receipt.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(filepath=str(native), export_format='GLB', use_selection=True,
                              export_animations=False, export_vertex_color='ACTIVE')
    result = {'result': 'PASS', 'blender': bpy.app.version_string, 'numpy': np.__version__,
              'source_provenance': 'Conifer-only editable reconstruction from pinned runtime GLB',
              'shipping_export_adapter': 'Read current source mesh positions; validate exact repair; recalculate incident normals; retain all other baseline bytes',
              'candidate_sha256': sha(data), 'source_sha256': sha(source.read_bytes()),
              'native_export_sha256': sha(native.read_bytes()), 'lods': rows, 'derivation': derivation,
              'limits': ['Source supports this audited repair only; new edits require deliberate recipe/check revisions.',
                         'Native Blender export is diagnostic and is not the shipping candidate.',
                         'Original island-assets.blend and multi-asset generator were not rebuilt or modified.']}
    receipt.write_text(json.dumps(result, indent=2)+'\n')
    print('CONIFER_SOURCE_EXPORT', json.dumps(result), flush=True)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source', type=Path, default=ROOT/'crates/alife_game_app/assets/landscape/island/conifer-leader-repair.blend')
    p.add_argument('--baseline', type=Path, default=ROOT/'crates/alife_game_app/assets/landscape/island/conifer-leader/Conifer.baseline.glb')
    p.add_argument('--output', type=Path, default=ROOT/'crates/alife_game_app/assets/landscape/island/Conifer.glb')
    p.add_argument('--native', type=Path, default=ROOT/'target/artifacts/conifer-leader/Conifer.native-export.glb')
    p.add_argument('--receipt', type=Path, default=ROOT/'target/artifacts/conifer-leader/source-reload-reexport.json')
    p.add_argument('--author-from-baseline', action='store_true')
    args = sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
    a = p.parse_args(args)
    if a.author_from_baseline:
        author(a.baseline, a.source)
    export_source(a.source, a.baseline, a.output, a.native, a.receipt)


if __name__ == '__main__':
    main()
