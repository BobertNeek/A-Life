"""Check the six exact finished runtime assets and both reconciled baselines.

These checks are independent of historical composed profiles and can be called
by the final Conifer-inclusive art audit. They do not establish game appearance.
"""
if not __debug__:
    raise RuntimeError('Finished-asset checks require normal Python mode')

import argparse
import hashlib
import json
import math
from pathlib import Path

from finished_island_glb import layout, parse, payload, values

ROOT = Path(__file__).resolve().parents[1]
HERE = Path('crates/alife_game_app/assets/landscape/island')
SOURCE = HERE / 'finished-island-refinements'
NAMES = {'Flowers', 'Grass', 'BoulderA', 'BoulderB', 'BoulderC', 'Cliff'}


def hash_bytes(raw):
    return hashlib.sha256(raw).hexdigest()


def digest(raw):
    value = 0xcbf29ce484222325
    for byte in raw:
        value = ((value ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    return f'fnv1a64:{value:016x}'


def check_asset(root, asset):
    source = root / SOURCE
    name = asset['name']
    current_path = source / asset['preintegration']['file']
    baseline_path = source / asset['source_pack_baseline']['file']
    runtime_path = root / asset['runtime_path']
    for path, record in ((current_path, asset['preintegration']),
                         (baseline_path, asset['source_pack_baseline']),
                         (runtime_path, asset['runtime'])):
        raw = path.read_bytes()
        assert len(raw) == record['bytes'] and hash_bytes(raw) == record['sha256'], ('Exact asset bytes differ', path)
    assert hash_bytes((source / asset['editable_source']['file']).read_bytes()) == asset['editable_source']['sha256'], ('Editable source changed', name)
    current, current_binary = parse(current_path)
    baseline, baseline_binary = parse(baseline_path)
    runtime, binary = parse(runtime_path)
    assert runtime == baseline, ('Saved source-pack layout changed', name)
    assert digest(runtime_path.read_bytes()) == asset['runtime']['fnv1a64']
    assert runtime['nodes'] == current['nodes'] == [{'mesh': i, 'name': name + '_L' + str(i)} for i in range(3)]
    assert runtime['materials'] == current['materials'] and len(runtime['materials']) == 1
    assert len(runtime['meshes']) == 3 and len(runtime['buffers']) == 1
    assert 'uri' not in runtime['buffers'][0]
    assert all(not runtime.get(key) for key in ('textures', 'images', 'extensionsUsed', 'extensionsRequired', 'skins', 'animations'))
    allowed = set()
    rows = []
    for level, (mesh, previous_mesh, installed_mesh, expected) in enumerate(zip(
            runtime['meshes'], baseline['meshes'], current['meshes'], asset['lods'])):
        assert mesh['name'] == expected['node_and_mesh_name'] == name + '_L' + str(level)
        assert len(mesh['primitives']) == 1
        primitive, previous, installed = [m['primitives'][0] for m in (mesh, previous_mesh, installed_mesh)]
        assert primitive['material'] == 0 and primitive.get('mode', 4) == 4
        assert set(primitive['attributes']) == {'POSITION', 'NORMAL', 'COLOR_0'}
        indices = [row[0] for row in values(runtime, binary, primitive['indices'])]
        positions = values(runtime, binary, primitive['attributes']['POSITION'])
        normals = values(runtime, binary, primitive['attributes']['NORMAL'])
        assert len(indices) // 3 == expected['triangles'] and len(indices) % 3 == 0
        assert len(positions) == expected['vertices'] and all(0 <= i < len(positions) for i in indices)
        assert all(math.isfinite(x) for row in positions + normals for x in row)
        bounds = [[fn(row[k] for row in positions) for k in range(3)] for fn in (min, max)]
        installed_positions = values(current, current_binary, installed['attributes']['POSITION'])
        installed_bounds = [[fn(row[k] for row in installed_positions) for k in range(3)] for fn in (min, max)]
        assert bounds == installed_bounds == [expected['runtime_aabb_min'], expected['runtime_aabb_max']], ('Reconciled AABB changed', name, level)
        assert payload(runtime, binary, primitive['indices']) == payload(baseline, baseline_binary, previous['indices'])
        color = runtime['accessors'][primitive['attributes']['COLOR_0']]
        assert color['componentType'] == 5121 and color['type'] == 'VEC3' and color.get('normalized')
        assert color['count'] == len(positions) and len(normals) == len(positions)
        semantics = ('COLOR_0',) if name == 'Cliff' else ('POSITION', 'NORMAL', 'COLOR_0')
        for semantic in semantics:
            accessor, _, offset, stride, size = layout(runtime, primitive['attributes'][semantic])
            for i in range(accessor['count']):
                allowed.update(range(offset + i * stride, offset + i * stride + size))
        rows.append({'lod': level, 'triangles': len(indices) // 3, 'vertices': len(positions),
                     'exact_installed_bounds': True, 'source_pack_indices_retained': True})
    assert len(binary) == len(baseline_binary)
    assert all(a == b for i, (a, b) in enumerate(zip(binary, baseline_binary)) if i not in allowed), ('Undeclared source-pack byte changed', name)
    return {'name': name, 'runtime_sha256': asset['runtime']['sha256'],
            'installed_baseline_reconciled': True, 'source_pack_baseline_pinned': True,
            'editable_source_pinned': True, 'color_only_vs_source_pack': name == 'Cliff', 'lods': rows}


def check(root=ROOT):
    root = Path(root).resolve()
    lineage = json.loads((root / SOURCE / 'lineage.json').read_text())
    assert len(lineage['assets']) == 6 and {a['name'] for a in lineage['assets']} == NAMES
    assert len({a['asset_id'] for a in lineage['assets']}) == 6
    assert all(a['runtime_path'] == str(HERE / (a['name'] + '.glb')) for a in lineage['assets'])
    return {'result': 'PASS', 'assets': [check_asset(root, a) for a in lineage['assets']],
            'runtime_assets': 6, 'lod_meshes': 18, 'game_rendering_proof': False, 'physical_gpu_proof': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    text = json.dumps(check(args.root), indent=2) + '\n'
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text)
    print(text, end='')
