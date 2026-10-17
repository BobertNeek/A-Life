"""Fixed CoastalShrub leaf-spray successor; standard-library CPU audit only.

The independent Broadleaf/consolidated snapshots and every unrelated art/world
file remain fixed. This establishes source, mesh and registration contracts,
not in-game rendering, physical-GPU behavior or visual acceptance.
"""
if not __debug__:
    raise RuntimeError('CoastalShrub art checks require normal Python mode')

import argparse
import copy
import hashlib
import json
import math
import struct
from pathlib import Path

import check_broadleaf_canopy as predecessor
import check_island_art as island_art
import check_island_canopies as island_canopies

ROOT = Path(__file__).resolve().parents[1]
HERE = Path('crates/alife_game_app/assets/landscape/island')
RUNTIME = (HERE / 'CoastalShrub.glb').as_posix()
BASELINE = HERE / 'coastal-leaf-sprays/CoastalShrub.baseline.glb'
SOURCE = HERE / 'CoastalShrub.leaf-sprays.blend'
SPECIFICATION = HERE / 'asset-specification.json'
PLACEMENTS = HERE / 'props.json'
PROFILE = Path('scripts/coastal-leaf-sprays-profile.json')
PROFILE_SHA256 = 'e0ce66630f6de0e2247c0238652baa0f195e1ecb78c7dfea38f99f462e6ad74c'
PREDECESSOR = Path('scripts/broadleaf-canopy-profile.json')
PREDECESSOR_SHA256 = '391b9ea622a6a05b44556ded119abdb0e8a08fc64cad1eeb2765a9d54cbc40da'
CONSOLIDATED = Path('scripts/consolidated-island-art-profile.json')
CONSOLIDATED_SHA256 = '2c8e26550d5a49a5bf4c962c005e2451b92a8687e84aea75cfb829421c993500'
BASELINE_SHA256 = 'da06fbb85f454978c12459bf9b4fffa8ecbea031724b83406db225589ab657af'
RUNTIME_SHA256 = '7a2636250308b67ba4794914b8edc0b1738646233a249c78a5f42573944f2a18'
SOURCE_SHA256 = '65a83ed2922ad989fe9e4a67b1d6adcac7cc373bc72477e4ef584a4d64a6198d'
BASELINE_SPECIFICATION_SHA256 = 'f41b7595f13365f9551751908fe51487d0bc786b05623e4b6d74526089d944e8'
TRIANGLES = [672, 204, 218]
VERTICES = [600, 276, 163]
RUNTIME_BYTES = 38552
MANIFEST = predecessor.MANIFEST
ART_ROOTS = predecessor.ART_ROOTS
ALLOWED_CHANGES = {RUNTIME, (HERE / 'README.md').as_posix(),
                   SPECIFICATION.as_posix(), MANIFEST.as_posix()}
ADDED_ART = {SOURCE.as_posix(), BASELINE.as_posix()}
IMPORTED_HELPERS = {
    'scripts/check_broadleaf_canopy.py': 'e6a113cafd864e3e85f4914eac81705280ebf1af7c990f25f03dff8762280041',
    'scripts/check_island_art.py': '4616794226f95829886d8a809100d793c6b016dec9dc5525e65f24804d9f924e',
    'scripts/check_island_canopies.py': '38fadffe32510d17ca1610fd5f8e9b1285bd50a3e0ca076e426305ac1062052c',
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical_sha(value):
    return sha(json.dumps(value, sort_keys=True, separators=(',', ':')).encode())


def safe_path(root, name, label):
    relative = Path(name)
    assert not relative.is_absolute() and relative.parts and '..' not in relative.parts, (label, name)
    path = Path(root)
    for part in relative.parts:
        path /= part
        assert not path.is_symlink(), (label + ' symlink forbidden', name)
    return path


def validate_source(root, name, digest):
    path = safe_path(root, name, 'Source')
    assert sha(path.read_bytes()) == digest, ('Source changed', name)


def validate_source_trees(root, sources, helper_roots=None):
    # --root selects data; executable helpers are still imported from this
    # process. Audit both, including the actual modules' directories.
    if helper_roots is None:
        helper_roots = [ROOT] + [Path(module.__file__).resolve().parents[1]
                                 for module in (predecessor, island_art, island_canopies)]
    for source_root in {Path(root).resolve(), *(Path(p).resolve() for p in helper_roots)}:
        for name, digest in sources.items():
            validate_source(source_root, name, digest)


def load_profiles(root):
    raw = safe_path(root, PROFILE, 'Profile').read_bytes()
    assert sha(raw) == PROFILE_SHA256, 'Fixed CoastalShrub profile changed'
    profile = json.loads(raw)
    before_raw = safe_path(root, PREDECESSOR, 'Profile').read_bytes()
    assert sha(before_raw) == PREDECESSOR_SHA256 == profile['predecessor_profile_sha256'], \
        'Historical Broadleaf profile changed'
    consolidated_raw = safe_path(root, CONSOLIDATED, 'Profile').read_bytes()
    assert sha(consolidated_raw) == CONSOLIDATED_SHA256, 'Historical consolidated profile changed'
    return profile, json.loads(before_raw), json.loads(consolidated_raw)


def validate_profile_sources(profile, before, consolidated):
    retained = dict(consolidated['checker_and_recipe_sources'])
    retained.update(before['sources'])
    retained.update(IMPORTED_HELPERS)
    assert profile['retained_sources'] == retained, 'Historical source inventory changed'
    assert set(profile['sources']) == {
        'scripts/build_coastal_leaf_sprays.py', 'scripts/check_coastal_leaf_sprays_native.py',
        'scripts/test_coastal_leaf_sprays.py', 'scripts/render_coastal_leaf_sprays.py'}, 'Successor source inventory changed'
    assert not set(retained) & set(profile['sources']), 'Source inventories overlap'
    assert PROFILE.as_posix() not in profile['sources']
    assert 'scripts/check_coastal_leaf_sprays.py' not in profile['sources']
    assert profile['triangles'] == TRIANGLES and profile['vertices'] == VERTICES
    return dict(retained, **profile['sources'])


def inventory(records):
    result = {}
    for record in records:
        assert set(record) == {'path', 'bytes', 'sha256', 'executable'}, 'Invalid art inventory record'
        name = record['path']
        relative = Path(name)
        assert not relative.is_absolute() and relative.parts and '..' not in relative.parts
        assert name not in result, ('Duplicate art path', name)
        assert type(record['bytes']) is int and record['bytes'] >= 0
        assert type(record['executable']) is bool
        assert len(record['sha256']) == 64 and all(c in '0123456789abcdef' for c in record['sha256'])
        result[name] = record
    return result


def validate_retention(profile, before):
    old, new = inventory(before['expected_art_files']), inventory(profile['expected_art_files'])
    assert set(new) - set(old) == ADDED_ART and not set(old) - set(new), 'Art path retention changed'
    for name in set(old) - ALLOWED_CHANGES:
        assert new[name] == old[name], ('Unrelated art profile changed', name)
    assert old[RUNTIME]['sha256'] == BASELINE_SHA256
    assert new[BASELINE.as_posix()] == dict(old[RUNTIME], path=BASELINE.as_posix()), 'Original CoastalShrub not retained'
    assert new[RUNTIME]['sha256'] == RUNTIME_SHA256 and new[RUNTIME]['bytes'] == RUNTIME_BYTES
    assert new[SOURCE.as_posix()]['sha256'] == SOURCE_SHA256, 'Approved native source changed'
    assert not new[RUNTIME]['executable'] and not new[SOURCE.as_posix()]['executable']
    return new


def validate_files(root, records):
    expected = inventory(records)
    actual = set()
    for directory in ART_ROOTS:
        base = safe_path(root, directory, 'Art')
        for path in base.rglob('*'):
            assert not path.is_symlink(), ('Art symlink forbidden', path)
            if path.is_file():
                actual.add(path.relative_to(root).as_posix())
    assert actual == set(expected), 'CoastalShrub art path set changed'
    for name, record in expected.items():
        path = safe_path(root, name, 'Art')
        raw = path.read_bytes()
        assert len(raw) == record['bytes'] and sha(raw) == record['sha256'], ('Art bytes changed', name)
        assert bool(path.stat().st_mode & 0o111) == record['executable'], ('Art mode changed', name)


def validate_specification(root, profile):
    baseline = profile['baseline_specification']
    assert canonical_sha(baseline) == BASELINE_SPECIFICATION_SHA256, 'Historical specification changed'
    shrub = profile['coastal_specification']
    assert shrub['width_depth_height_m'] == baseline['assets']['CoastalShrub']['width_depth_height_m']
    assert [shrub[k] for k in ('near_triangles', 'far_triangles', 'overview_triangles')] == TRIANGLES
    assert shrub['editable_override'] == SOURCE.name, 'Native source registration changed'
    expected = copy.deepcopy(baseline)
    expected['assets']['CoastalShrub'] = shrub
    actual = json.loads(safe_path(root, SPECIFICATION, 'Specification').read_text())
    assert actual == expected, 'Unrelated specification or CoastalShrub registration changed'


def validate_manifest_records(root, actual, expected):
    for manifest in (expected, actual):
        names = [entry['local_path'] for entry in manifest['entries']]
        assert len(names) == len(set(names)) == 48, 'Manifest must retain 48 unique registrations'
    assert actual == expected, 'Unreviewed manifest changes'
    for entry in actual['entries']:
        raw = safe_path(root, entry['local_path'], 'Manifest asset').read_bytes()
        assert len(raw) == entry['size_bytes'], ('Manifest size mismatch', entry['local_path'])
        assert predecessor.digest_payload(entry['local_path'], raw) == entry['digest'], \
            ('Manifest digest mismatch', entry['local_path'])


def validate_manifest(root, profile, before, consolidated):
    expected = predecessor.expected_manifest(before, consolidated, root)
    before_entries = {entry['local_path']: copy.deepcopy(entry) for entry in expected['entries']}
    assert set(profile['manifest_override']) == {'digest', 'size_bytes', 'generator'}
    assert profile['manifest_override']['size_bytes'] == RUNTIME_BYTES
    for entry in expected['entries']:
        if entry['local_path'] == RUNTIME:
            entry.update(profile['manifest_override'])
    assert sum(entry != before_entries[entry['local_path']] for entry in expected['entries']) == 1
    actual = json.loads(safe_path(root, MANIFEST, 'Manifest').read_text())
    validate_manifest_records(root, actual, expected)
    return len(actual['entries'])


def validate_placements(root):
    props = json.loads(safe_path(root, PLACEMENTS, 'Placements').read_text())
    count = sum(p['kind'] == 'CoastalShrub' for p in props)
    assert count == 170, ('CoastalShrub placement count changed', count)
    return count


def load_glb(path):
    raw = Path(path).read_bytes()
    assert len(raw) >= 28 and struct.unpack_from('<III', raw) == (0x46546c67, 2, len(raw)), 'Invalid GLB header'
    size, kind = struct.unpack_from('<II', raw, 12)
    assert kind == 0x4e4f534a and size % 4 == 0 and 28 + size <= len(raw), 'Invalid GLB JSON chunk'
    binary_size, kind = struct.unpack_from('<II', raw, 20 + size)
    assert kind == 0x004e4942 and binary_size % 4 == 0 and 28 + size + binary_size == len(raw), 'Invalid GLB binary chunk'
    glb = island_art.Glb(Path(path))
    assert len(glb.doc['buffers']) == 1 and set(glb.doc['buffers'][0]) == {'byteLength'}
    assert 0 <= binary_size - glb.doc['buffers'][0]['byteLength'] <= 3
    return glb


def accessor_payload(glb, index):
    """Logical accessor metadata and bit-exact values, independent of packing."""
    accessor = glb.doc['accessors'][index]
    view = glb.doc['bufferViews'][accessor['bufferView']]
    assert not accessor.get('sparse') and view.get('buffer', 0) == 0
    width = {5126: 4, 5125: 4, 5123: 2, 5121: 1}[accessor['componentType']] * \
        {'SCALAR': 1, 'VEC3': 3, 'VEC4': 4}[accessor['type']]
    stride = view.get('byteStride', width)
    start = accessor.get('byteOffset', 0)
    assert stride >= width and accessor['count'] > 0
    assert start + (accessor['count'] - 1) * stride + width <= view['byteLength']
    assert view.get('byteOffset', 0) + view['byteLength'] <= glb.doc['buffers'][0]['byteLength']
    offset = glb.binary + view.get('byteOffset', 0) + start
    payload = b''.join(glb.raw[offset + i * stride:offset + i * stride + width]
                       for i in range(accessor['count']))
    metadata = {key: value for key, value in accessor.items() if key not in ('bufferView', 'byteOffset')}
    return metadata, payload


def bounds(vertices):
    return [[operation(row[k] for row in vertices) for k in range(3)] for operation in (min, max)]


def geometry(path, baseline=None):
    old = load_glb(Path(baseline) if baseline is not None else ROOT / BASELINE)
    new = load_glb(path)
    assert sha(old.raw) == BASELINE_SHA256, 'CoastalShrub baseline changed'
    for field in ('nodes', 'scenes', 'scene', 'materials'):
        assert old.doc.get(field) == new.doc.get(field), ('Binding changed', field)
    assert all(set(node) == {'mesh', 'name'} for node in new.doc['nodes']), 'Nonidentity node'
    assert len(new.doc['nodes']) == len(new.doc['meshes']) == 3
    assert not any(new.doc.get(field) for field in
                   ('textures', 'images', 'animations', 'skins', 'extensionsRequired', 'extensionsUsed'))
    records = []
    for level, (previous, current) in enumerate(zip(old.doc['meshes'], new.doc['meshes'])):
        assert set(current) == {'name', 'primitives'}
        assert previous['name'] == current['name'] == f'CoastalShrub_L{level}', 'LOD binding changed'
        assert len(previous['primitives']) == len(current['primitives']) == 1, 'One primitive required per LOD'
        p, q = previous['primitives'][0], current['primitives'][0]
        assert set(q) <= {'attributes', 'indices', 'material', 'mode'}
        assert p['material'] == q['material'] == 0 and q.get('mode', 4) == 4
        assert set(p['attributes']) == set(q['attributes']) == {'POSITION', 'NORMAL', 'COLOR_0'}
        for index in [*q['attributes'].values(), q['indices']]:
            accessor_payload(new, index)
        pos = new.accessor(q['attributes']['POSITION'])
        normals = new.accessor(q['attributes']['NORMAL'])
        colors = new.accessor(q['attributes']['COLOR_0'])
        for semantic in ('POSITION', 'NORMAL'):
            accessor = new.doc['accessors'][q['attributes'][semantic]]
            assert accessor['componentType'] == 5126 and accessor['type'] == 'VEC3'
            assert not accessor.get('normalized')
        color = new.doc['accessors'][q['attributes']['COLOR_0']]
        assert color['componentType'] == 5121 and color.get('normalized') is True and color['type'] == 'VEC3', \
            'Normalized U8 RGB required'
        assert len(pos) == len(normals) == len(colors) == VERTICES[level], ('Vertex count changed', level)
        assert all(math.isfinite(x) for row in pos + normals for x in row), 'Nonfinite geometry'
        assert all(abs(sum(x * x for x in row) - 1) < 1e-5 for row in normals), 'Nonunit normal'
        old_pos = old.accessor(p['attributes']['POSITION'])
        aabb = bounds(pos)
        assert aabb == bounds(old_pos), ('LOD bounds changed', level)
        position_accessor = new.doc['accessors'][q['attributes']['POSITION']]
        assert position_accessor['min'] == aabb[0] and position_accessor['max'] == aabb[1], 'Accessor bounds changed'
        index_accessor = new.doc['accessors'][q['indices']]
        assert index_accessor['type'] == 'SCALAR' and index_accessor['componentType'] in (5121, 5123, 5125)
        assert not index_accessor.get('normalized')
        indices = [row[0] for row in new.accessor(q['indices'])]
        assert len(indices) == TRIANGLES[level] * 3, ('Triangle count changed', level)
        assert set(indices) == set(range(len(pos))), 'Unused or invalid exported vertices'
        for offset in range(0, len(indices), 3):
            a, b, c = [pos[i] for i in indices[offset:offset + 3]]
            u, v = [b[k] - a[k] for k in range(3)], [c[k] - a[k] for k in range(3)]
            cross = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2],
                     u[0] * v[1] - u[1] * v[0])
            assert sum(x * x for x in cross) > 1e-20, ('Degenerate triangle', level, offset // 3)
        if level == 2:
            for semantic in ('POSITION', 'NORMAL', 'COLOR_0'):
                assert accessor_payload(old, p['attributes'][semantic]) == \
                    accessor_payload(new, q['attributes'][semantic]), ('L2 attribute changed', semantic)
            assert accessor_payload(old, p['indices']) == accessor_payload(new, q['indices']), 'L2 indices changed'
        records.append({'lod': level, 'baseline_triangles': len(old.accessor(p['indices'])) // 3,
                        'triangles': len(indices) // 3, 'baseline_vertices': len(old_pos),
                        'vertices': len(pos), 'bounds_min_max_xyz_m': aabb, 'bound_change_m': 0})
    return records


def check(root=ROOT):
    root = Path(root).resolve()
    profile, before, consolidated = load_profiles(root)
    sources = validate_profile_sources(profile, before, consolidated)
    validate_source_trees(root, dict(sources, **{PREDECESSOR.as_posix(): PREDECESSOR_SHA256,
                                               CONSOLIDATED.as_posix(): CONSOLIDATED_SHA256}))
    files = validate_retention(profile, before)
    validate_files(root, profile['expected_art_files'])
    validate_specification(root, profile)
    manifest_count = validate_manifest(root, profile, before, consolidated)
    placement_count = validate_placements(root)
    lods = geometry(root / RUNTIME, root / BASELINE)
    broadleaf = predecessor.geometry(root / HERE / 'Broadleaf.glb')
    return {'result': 'PASS', 'profile': profile['profile'], 'lods': lods,
            'runtime_bytes': (root / RUNTIME).stat().st_size, 'art_blobs_audited': len(files),
            'manifest_entries_valid': manifest_count, 'other_manifest_records_retained': 47,
            'coastal_shrub_placements': placement_count, 'l2_attributes_and_indices_retained': True,
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
