"""Export six editable Blender 4.3.2 overrides through their saved layout guards.

This loads current native edits; it never reapplies the original refinement.
Run after the base island build. Intermediate native GLBs are not runtime assets.
"""
import hashlib
import json
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from finished_island_glb import transplant

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/landscape/island'
SOURCE = HERE / 'finished-island-refinements'


def main():
    assert bpy.app.version == (4, 3, 2), 'Use the verified Blender 4.3.2 exporter'
    profile = json.loads((SOURCE / 'lineage.json').read_text())
    receipts = ROOT / 'target/artifacts/finished-island-refinements/export'
    receipts.mkdir(parents=True, exist_ok=True)
    records = []
    for asset in profile['assets']:
        name = asset['name']
        baseline = SOURCE / asset['source_pack_baseline']['file']
        assert hashlib.sha256(baseline.read_bytes()).hexdigest() == asset['source_pack_baseline']['sha256'], name
        bpy.ops.wm.open_mainfile(filepath=str(SOURCE / asset['editable_source']['file']))
        objects = [bpy.data.objects[name + '_L' + str(i)] for i in range(3)]
        bpy.ops.object.select_all(action='DESELECT')
        for obj in objects:
            assert obj.type == 'MESH'
            assert tuple(obj.location) == (0., 0., 0.)
            assert tuple(obj.rotation_euler) == (0., 0., 0.)
            assert tuple(obj.scale) == (1., 1., 1.)
            obj.hide_set(False)
            obj.hide_render = False
            obj.select_set(True)
        bpy.context.view_layer.objects.active = objects[0]
        native = receipts / (name + '.native-export.glb')
        bpy.ops.export_scene.gltf(filepath=str(native), export_format='GLB',
            use_selection=True, export_animations=False, export_cameras=False,
            export_lights=False, export_materials='EXPORT', export_yup=True)
        output = HERE / (name + '.glb')
        result = transplant(baseline, native, output, color_only=name == 'Cliff')
        result.update(name=name, bytes=output.stat().st_size,
            sha256=hashlib.sha256(output.read_bytes()).hexdigest())
        records.append(result)
    text = json.dumps({'result': 'PASS', 'blender': bpy.app.version_string,
                       'assets': records}, indent=2) + '\n'
    (receipts / 'source-export-check.json').write_text(text)
    print(text, end='')


if __name__ == '__main__':
    main()
