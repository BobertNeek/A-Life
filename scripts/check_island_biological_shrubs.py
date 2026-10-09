"""Check the two overrides against their exact pre-refresh runtime contracts."""
import argparse
import hashlib
import json
import struct
import subprocess
from pathlib import Path

from check_island_art import Glb

ROOT = Path(__file__).resolve().parents[1]
HERE = Path('crates/alife_game_app/assets/landscape/island')
BASE = '9a4e3b3dea9ff1b8b63bf60fde5f2a8385e50c9b'
KINDS = ('BerryBush', 'CoastalShrub')


def baseline(path):
    return subprocess.check_output(['git', 'show', BASE + ':' + path.as_posix()], cwd=ROOT)


def glb(raw):
    result = Glb.__new__(Glb)
    result.raw = raw
    size = struct.unpack_from('<I', raw, 12)[0]
    result.doc = json.loads(raw[20:20 + size])
    result.binary = 28 + size
    return result


def check():
    rows, total_before, total_after, maximum_bound_error = [], 0, 0, 0
    for kind in KINDS:
        path = HERE / (kind + '.glb')
        old_raw, new_raw = baseline(path), (ROOT / path).read_bytes()
        old, new = glb(old_raw), glb(new_raw)
        assert {n['name'] for n in new.doc['nodes']} == {kind + '_L' + str(i) for i in range(3)}
        assert all(not any(k in n for k in ('matrix', 'translation', 'rotation', 'scale'))
                   for n in new.doc['nodes'])
        assert len(new.doc['materials']) == 1 and not new.doc.get('textures')
        assert new.doc['materials'] == old.doc['materials'], kind
        counts, errors = [], []
        old_meshes = {m['name']: m for m in old.doc['meshes']}
        for mesh in new.doc['meshes']:
            assert len(mesh['primitives']) == 1
            p = mesh['primitives'][0]
            previous = old_meshes[mesh['name']]['primitives'][0]
            assert p['material'] == 0 and 'COLOR_0' in p['attributes']
            color = new.doc['accessors'][p['attributes']['COLOR_0']]
            assert color['componentType'] == 5121 and color['normalized']
            vertices = new.accessor(p['attributes']['POSITION'])
            original = old.accessor(previous['attributes']['POSITION'])
            before_bounds = [f(v[k] for v in original) for f in (min, max) for k in range(3)]
            after_bounds = [f(v[k] for v in vertices) for f in (min, max) for k in range(3)]
            error = max(abs(a - b) for a, b in zip(before_bounds, after_bounds))
            assert error < 1e-6, (kind, mesh['name'], error)
            errors.append(error)
            count = new.doc['accessors'][p['indices']]['count'] // 3
            assert count == old.doc['accessors'][previous['indices']]['count'] // 3
            counts.append(count)
        total_before += len(old_raw)
        total_after += len(new_raw)
        maximum_bound_error = max(maximum_bound_error, *errors)
        rows.append({'kind': kind, 'lod_triangles': counts,
                     'before_bytes': len(old_raw), 'after_bytes': len(new_raw),
                     'sha256': hashlib.sha256(new_raw).hexdigest(),
                     'maximum_bounds_error_m': max(errors)})
    # Every other committed art file and world surface retains its exact bytes.
    protected = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', BASE,
        'crates/alife_game_app/assets', 'crates/alife_world/assets'], cwd=ROOT, text=True).splitlines()
    allowed = {str(HERE / (kind + '.glb')) for kind in KINDS}
    allowed.add('crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json')
    updated_documentation = {str(HERE / 'README.md')}
    verified = 0
    for name in protected:
        if name in allowed or name in updated_documentation:
            continue
        assert (ROOT / name).read_bytes() == baseline(Path(name)), name
        verified += 1
    manifest_path = Path('crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json')
    old_manifest = json.loads(baseline(manifest_path))
    manifest = json.loads((ROOT / manifest_path).read_text())
    old_entries = {e['local_path']: e for e in old_manifest['entries']}
    for entry in manifest['entries']:
        path = ROOT / entry['local_path']
        payload = path.read_bytes()
        data = payload.replace(b'\r\n', b'\n').replace(b'\r', b'\n') if path.suffix == '.json' else payload
        digest = 0xcbf29ce484222325
        for byte in data:
            digest = ((digest ^ byte) * 0x100000001b3) & 0xffffffffffffffff
        assert entry['size_bytes'] == len(payload) and entry['digest'] == f'fnv1a64:{digest:016x}'
        old_entry = old_entries[entry['local_path']]
        if entry['local_path'] not in allowed - {str(manifest_path)}:
            assert entry == old_entry
        else:
            assert {k: v for k, v in entry.items() if k not in ('digest', 'size_bytes', 'generator')} == {
                k: v for k, v in old_entry.items() if k not in ('digest', 'size_bytes', 'generator')}
    assert len(manifest['entries']) == len(old_manifest['entries'])
    return {'result': 'PASS', 'baseline': BASE, 'assets': rows,
            'target_bytes_before': total_before, 'target_bytes_after': total_after,
            'maximum_bounds_error_m': maximum_bound_error,
            'other_art_and_world_files_byte_retained': verified,
            'updated_asset_documentation': sorted(updated_documentation),
            'manifest_entries_valid': len(manifest['entries']),
            'game_rendering_proof': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    report = check()
    text = json.dumps(report, indent=2) + '\n'
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text)
    print(text, end='')
