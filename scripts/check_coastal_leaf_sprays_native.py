"""Blender source-bound retention and attachment audit; CPU, no source writes.

Run with Blender 4.3.2 --background --factory-startup --python-exit-code 1.
This optional local audit is separate from the standard-library CI gate.
"""
import argparse
import json
import sys
import tempfile
from pathlib import Path

import bpy

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_coastal_leaf_sprays import (
    ASSETS, SOURCE, HISTORICAL_SOURCE, HISTORICAL_SHA256,
    digest, export_kind, metrics,
)


def inspect_source(path):
    bpy.ops.wm.open_mainfile(filepath=str(path))
    result = {}
    for kind in ('CoastalShrub', 'BerryBush'):
        result[kind] = [metrics(bpy.data.objects[f'{kind}_L{i}']) for i in range(3)]
    return result


def check():
    assert bpy.app.version == (4, 3, 2), 'Use Blender 4.3.2'
    assert digest(HISTORICAL_SOURCE) == HISTORICAL_SHA256
    assert digest(SOURCE) == '65a83ed2922ad989fe9e4a67b1d6adcac7cc373bc72477e4ef584a4d64a6198d'
    before_hashes = {p: digest(p) for p in (SOURCE, HISTORICAL_SOURCE,
                     ASSETS / 'BerryBush.glb', ASSETS / 'props.json')}
    old = inspect_source(HISTORICAL_SOURCE)
    new = inspect_source(SOURCE)
    assert old['BerryBush'] == new['BerryBush'], 'Native BerryBush changed'
    for level, (a, b) in enumerate(zip(old['CoastalShrub'], new['CoastalShrub'])):
        assert a['bounds_xyz'] == b['bounds_xyz'], ('Native bounds changed', level)
        assert a['wood_signature'] == b['wood_signature'], ('Native stem corner position/color changed', level)
        assert b['triangles'] <= a['triangles'] and b['vertices'] <= a['vertices']
        assert b['zero_area_faces'] == 0
        ob = bpy.data.objects[f'CoastalShrub_L{level}']
        assert {i for p in ob.data.polygons for i in p.vertices} == set(range(len(ob.data.vertices)))
        assert tuple(ob.location) == (0., 0., 0.)
        assert tuple(ob.rotation_euler) == (0., 0., 0.) and tuple(ob.scale) == (1., 1., 1.)
        if level < 2:
            assert b['folded_blade_roots_checked'] == (60, 48)[level]
            assert b['leaf_root_attachment_max_error_m'] == 0
        else:
            assert a['mesh_signature'] == b['mesh_signature'], 'Native L2 changed'
    with tempfile.TemporaryDirectory(prefix='coastal-native-check-') as directory:
        output = Path(directory)
        export_kind('CoastalShrub', output / 'CoastalShrub.glb')
        export_kind('BerryBush', output / 'BerryBush.glb')
        assert (output / 'CoastalShrub.glb').read_bytes() == (ASSETS / 'CoastalShrub.glb').read_bytes()
        assert (output / 'BerryBush.glb').read_bytes() == (ASSETS / 'BerryBush.glb').read_bytes()
    assert before_hashes == {p: digest(p) for p in before_hashes}, 'Read-only audit changed a source'
    return {'result': 'PASS', 'blender': bpy.app.version_string,
            'native_sha256': digest(SOURCE),
            'runtime_sha256': digest(ASSETS / 'CoastalShrub.glb'),
            'native_lods': new['CoastalShrub'],
            'stems_corner_positions_and_colors_retained': True,
            'all_108_blade_roots_at_original_attachments': True,
            'blades_topologically_welded_to_stems': False,
            'native_and_runtime_L2_retained': True,
            'BerryBush_native_and_runtime_retained': True,
            'source_reload_export_byte_identical': True,
            'source_files_unchanged': True, 'live_game': False, 'physical_gpu': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    result = check()
    payload = json.dumps(result, indent=2) + '\n'
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(payload)
    print('COASTAL_NATIVE_CHECK', payload, flush=True)
