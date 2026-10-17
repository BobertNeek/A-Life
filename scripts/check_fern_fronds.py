"""Fixed connected-Fern successor: exact art/source retention, stdlib CPU only.

Historical color-only Fern and Broadleaf profiles retain their earlier snapshots.
This checker pins the new native source, all other art/world bytes and all 48
registrations. Offline source/mesh checks do not establish game/GPU acceptance.
"""
if not __debug__:
    raise RuntimeError('Fern frond checks require normal Python mode')

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path

import check_broadleaf_canopy as predecessor
from check_island_art import Glb

ROOT = Path(__file__).resolve().parents[1]
HERE = Path('crates/alife_game_app/assets/landscape/island')
RUNTIME = (HERE / 'Fern.glb').as_posix()
BASELINE = HERE / 'fern-connected-fronds/Fern.baseline.glb'
SOURCE = HERE / 'fern-connected-fronds.blend'
PROFILE = Path('scripts/fern-fronds-profile.json')
PROFILE_SHA256 = 'e998c16ef677eb28a97f0c2d17127e76be8739e47963f566372fa35527e2b3da'
PREDECESSOR = Path('scripts/broadleaf-canopy-profile.json')
PREDECESSOR_SHA256 = '391b9ea622a6a05b44556ded119abdb0e8a08fc64cad1eeb2765a9d54cbc40da'
CONSOLIDATED = Path('scripts/consolidated-island-art-profile.json')
CONSOLIDATED_SHA256 = '2c8e26550d5a49a5bf4c962c005e2451b92a8687e84aea75cfb829421c993500'
MANIFEST = predecessor.MANIFEST
ALLOWED_CHANGES = {RUNTIME, (HERE / 'README.md').as_posix(),
                   (HERE / 'asset-specification.json').as_posix(), MANIFEST.as_posix()}
ADDED_ART = {SOURCE.as_posix(), BASELINE.as_posix()}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def validate_source(root, name, digest):
    path = Path(root) / name
    assert not path.is_symlink(), ('Source symlink forbidden', name)
    assert sha(path.read_bytes()) == digest, ('Source changed', name)


def geometry(path, baseline=None):
    old = Glb(Path(baseline) if baseline else ROOT / BASELINE)
    new = Glb(Path(path))
    assert sha(old.raw) == '20d1ca87a71f8adc6ea9cd2b0ca8356cf4beec4a4ae145f2b8224b0b972dabb6'
    for field in ('nodes', 'scenes', 'scene', 'materials'):
        assert old.doc.get(field) == new.doc.get(field), ('Binding changed', field)
    assert len(old.doc['meshes']) == len(new.doc['meshes']) == 3
    assert not any(new.doc.get(field) for field in
                   ('textures', 'images', 'animations', 'skins', 'extensionsRequired'))
    records = []
    for level, (previous, current) in enumerate(zip(old.doc['meshes'], new.doc['meshes'])):
        assert previous['name'] == current['name'] == f'Fern_L{level}'
        assert len(previous['primitives']) == len(current['primitives']) == 1
        p, q = previous['primitives'][0], current['primitives'][0]
        assert p['material'] == q['material'] == 0 and q.get('mode', 4) == 4
        assert set(p['attributes']) == set(q['attributes']) == {'POSITION', 'NORMAL', 'COLOR_0'}
        old_pos = old.accessor(p['attributes']['POSITION'])
        pos = new.accessor(q['attributes']['POSITION'])
        normal = new.accessor(q['attributes']['NORMAL'])
        color = new.accessor(q['attributes']['COLOR_0'])
        old_indices = old.accessor(p['indices'])
        indices = [row[0] for row in new.accessor(q['indices'])]
        assert len(indices) % 3 == 0 and len(indices) <= len(old_indices)
        assert len(pos) == len(normal) == len(color) and len(pos) <= len(old_pos)
        assert set(indices) == set(range(len(pos))), 'Unused or invalid exported vertices'
        assert all(math.isfinite(value) for row in pos + normal for value in row)
        assert all(abs(sum(x * x for x in row) - 1) < 1e-5 for row in normal)
        color_accessor = new.doc['accessors'][q['attributes']['COLOR_0']]
        assert color_accessor['componentType'] == 5121 and color_accessor.get('normalized')
        assert color_accessor['type'] in ('VEC3', 'VEC4')
        assert all(len(row) == 3 or row[3] == 255 for row in color)
        bound_delta = max(abs(operation(row[k] for row in old_pos) -
                              operation(row[k] for row in pos))
                          for operation in (min, max) for k in range(3))
        assert bound_delta == 0, ('LOD bounds changed', level, bound_delta)
        for offset in range(0, len(indices), 3):
            a, b, c = [pos[i] for i in indices[offset:offset + 3]]
            u, v = [b[k] - a[k] for k in range(3)], [c[k] - a[k] for k in range(3)]
            cross = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2],
                     u[0] * v[1] - u[1] * v[0])
            assert sum(x * x for x in cross) > 1e-20, ('Degenerate triangle', level, offset // 3)
        records.append({'lod': level, 'baseline_triangles': len(old_indices) // 3,
                        'triangles': len(indices) // 3, 'baseline_vertices': len(old_pos),
                        'vertices': len(pos), 'bound_change_m': bound_delta})
    return records


def check(root=ROOT):
    root = Path(root).resolve()
    raw = (root / PROFILE).read_bytes()
    assert sha(raw) == PROFILE_SHA256, 'Fixed Fern profile changed'
    profile = json.loads(raw)
    before_raw = (root / PREDECESSOR).read_bytes()
    assert sha(before_raw) == PREDECESSOR_SHA256 == profile['predecessor_profile_sha256']
    before = json.loads(before_raw)
    consolidated_raw = (root / CONSOLIDATED).read_bytes()
    assert sha(consolidated_raw) == CONSOLIDATED_SHA256
    consolidated = json.loads(consolidated_raw)
    # Check both imported executable helpers and an explicitly selected source root.
    for source_root in {ROOT.resolve(), root}:
        for name, digest in profile['retained_sources'].items():
            validate_source(source_root, name, digest)
        for name, digest in profile['sources'].items():
            validate_source(source_root, name, digest)
    old_files = {row['path']: row for row in before['expected_art_files']}
    new_files = {row['path']: row for row in profile['expected_art_files']}
    assert len(new_files) == len(profile['expected_art_files'])
    assert set(new_files) - set(old_files) == ADDED_ART and not set(old_files) - set(new_files)
    for name in set(old_files) - ALLOWED_CHANGES:
        assert new_files[name] == old_files[name], ('Unrelated art profile changed', name)
    predecessor.validate_files(root, profile['expected_art_files'])
    assert sha((root / BASELINE).read_bytes()) == old_files[RUNTIME]['sha256']
    expected = predecessor.expected_manifest(before, consolidated, root)
    entries = {row['local_path']: row for row in expected['entries']}
    entries[RUNTIME].update(profile['manifest_override'])
    actual = json.loads((root / MANIFEST).read_text())
    assert actual == expected, 'Unreviewed manifest changes'
    for entry in actual['entries']:
        payload = (root / entry['local_path']).read_bytes()
        assert len(payload) == entry['size_bytes']
        assert predecessor.digest_payload(entry['local_path'], payload) == entry['digest']
    result = geometry(root / RUNTIME, root / BASELINE)
    assert [row['triangles'] for row in result] == [700, 294, 154]
    assert [row['vertices'] for row in result] == [952, 441, 231]
    broadleaf = predecessor.geometry(root / HERE / 'Broadleaf.glb')
    return {'result': 'PASS', 'profile': 'connected-fern-fronds-20261016', 'lods': result,
            'runtime_bytes': (root / RUNTIME).stat().st_size, 'art_blobs_audited': len(new_files),
            'manifest_entries_valid': len(actual['entries']), 'other_manifest_records_retained': 47,
            'all_other_art_and_world_bytes_retained': True, 'historical_profiles_retained': True,
            'retained_broadleaf_triangles': broadleaf['triangles'],
            'game_rendering_proof': False, 'physical_gpu_proof': False}


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
