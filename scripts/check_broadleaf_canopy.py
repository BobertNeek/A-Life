"""Fixed Broadleaf successor profile; retain every other Island/art/world byte.

CPU geometry, registration and source-bound checks, not game or GPU acceptance.
Historical consolidated/conifer profiles intentionally remain unchanged.
"""
if not __debug__:
    raise RuntimeError('Broadleaf art checks require normal Python mode')

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path

import check_consolidated_island_art as predecessor
import check_composed_island_graphics as historical
from check_island_art import Glb
from check_island_canopies import points, silhouette

ROOT = Path(__file__).resolve().parents[1]
HERE = Path('crates/alife_game_app/assets/landscape/island')
RUNTIME = (HERE / 'Broadleaf.glb').as_posix()
PROFILE = Path('scripts/broadleaf-canopy-profile.json')
PROFILE_SHA256 = 'b445338758339243337b0933e0e96f553733390fd23ba91d71820cfcbdc4078e'
ALLOWED_PREDECESSOR_CHANGES = {
    RUNTIME, (HERE / 'README.md').as_posix(),
    (HERE / 'asset-specification.json').as_posix(), historical.MANIFEST.as_posix(),
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def expected_manifest(profile, old_profile, root):
    expected = copy.deepcopy(old_profile['base_manifest'])
    entries = historical.entries_by_path(expected)
    lineage = json.loads((root / predecessor.SOURCE / 'lineage.json').read_text())
    for asset in lineage['assets']:
        entries[asset['runtime_path']].update(
            digest=asset['runtime']['fnv1a64'], size_bytes=asset['runtime']['bytes'],
            generator={'config_path': 'scripts/build_finished_island_assets.py',
                       'date': '2026-10-09', 'seed': 'saved-finished-island-refinements/' + asset['name'],
                       'tool': 'Blender-4.3.2'})
    placement = old_profile['boulder_placement_correction']
    entries[placement['props_path']].update(digest=placement['digest'], size_bytes=placement['bytes'])
    entries[RUNTIME].update(profile['manifest_override'])
    return expected


def geometry(path):
    glb = Glb(path)
    doc = glb.doc
    assert len(doc['materials']) == 1 and not doc.get('textures') and not doc.get('images')
    assert doc['materials'] == [{
        'doubleSided': True, 'name': 'Island shared painted surface',
        'pbrMetallicRoughness': {'metallicFactor': 0, 'roughnessFactor': 0.8199999928474426}}]
    assert not doc.get('animations') and not doc.get('skins')
    assert len(doc['nodes']) == len(doc['meshes']) == 3
    assert all(set(n) == {'mesh', 'name'} for n in doc['nodes'])
    meshes = {m['name']: m for m in doc['meshes']}
    assert set(meshes) == {'Broadleaf_L0', 'Broadleaf_L1', 'Broadleaf_L2'}
    primitives, counts = [], []
    for level, ceiling in enumerate((4820, 1038, 88)):
        mesh = meshes['Broadleaf_L' + str(level)]
        assert len(mesh['primitives']) == 1
        p = mesh['primitives'][0]
        assert p.get('mode', 4) == 4 and p['material'] == 0
        assert set(p['attributes']) == {'POSITION', 'NORMAL', 'COLOR_0'}
        pos = glb.accessor(p['attributes']['POSITION'])
        normals = glb.accessor(p['attributes']['NORMAL'])
        colors = glb.accessor(p['attributes']['COLOR_0'])
        assert len(pos) == len(normals) == len(colors)
        assert all(math.isfinite(x) for row in pos + normals for x in row)
        assert all(abs(sum(n * n for n in row) - 1) < .002 for row in normals)
        color_accessor = doc['accessors'][p['attributes']['COLOR_0']]
        assert color_accessor['componentType'] == 5121 and color_accessor['normalized']
        assert color_accessor['type'] in ('VEC3', 'VEC4')
        assert all(len(c) == 3 or c[3] == 255 for c in colors)
        lo = [min(v[k] for v in pos) for k in range(3)]
        hi = [max(v[k] for v in pos) for k in range(3)]
        assert all(abs(a - b) < .00001 for a, b in zip(lo, (-3, 0, -2.75))), lo
        assert all(abs(a - b) < .00001 for a, b in zip(hi, (3, 8, 2.75))), hi
        indices = [i[0] for i in glb.accessor(p['indices'])]
        assert len(indices) % 3 == 0 and all(0 <= i < len(pos) for i in indices)
        for index in range(0, len(indices), 3):
            a, b, c = [pos[i] for i in indices[index:index + 3]]
            ab, ac = [b[k] - a[k] for k in range(3)], [c[k] - a[k] for k in range(3)]
            cross = (ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2],
                     ab[0] * ac[1] - ab[1] * ac[0])
            assert sum(x * x for x in cross) > 1e-16, ('Degenerate face', level, index // 3)
        counts.append(len(indices) // 3)
        assert counts[-1] <= ceiling, (level, counts[-1], ceiling)
        primitives.append(p)
    views = []
    for angle in (0, 45, 90, 135):
        yaw = math.radians(angle)
        near = points(glb, primitives[0], yaw)
        bounds = (min(x for x, y in near), min(y for x, y in near),
                  max(x for x, y in near), max(y for x, y in near))
        masks = [silhouette(glb, p, points(glb, p, yaw), bounds) for p in primitives]
        comparisons = []
        for level, other in enumerate(masks[1:], 1):
            intersection = sum(a and b for a, b in zip(masks[0], other))
            union = sum(a or b for a, b in zip(masks[0], other))
            coverage, iou = intersection / sum(masks[0]), intersection / union
            assert coverage >= (.85 if level == 1 else .80), (angle, level, coverage)
            assert iou >= (.75 if level == 1 else .70), (angle, level, iou)
            comparisons.append({'lod': level, 'near_silhouette_coverage': coverage, 'intersection_over_union': iou})
        views.append({'yaw_degrees': angle, 'comparisons': comparisons})
    return {'triangles': counts, 'materials': 1, 'primitives_per_lod': 1,
            'textures': 0, 'runtime_bounds_xyz_m': [6, 8, 5.5], 'silhouette_views': views}


def check(root=ROOT):
    root = Path(root).resolve()
    raw = (root / PROFILE).read_bytes()
    assert sha(raw) == PROFILE_SHA256, 'Fixed Broadleaf profile changed'
    profile = json.loads(raw)
    old_raw = (root / predecessor.PROFILE).read_bytes()
    assert sha(old_raw) == predecessor.PROFILE_SHA256 == profile['predecessor_profile_sha256']
    old_profile = json.loads(old_raw)
    for path, digest in old_profile['checker_and_recipe_sources'].items():
        assert sha((root / path).read_bytes()) == digest, ('Historical checker or recipe changed', path)
    old_files = {r['path']: r for r in old_profile['expected_art_files']}
    new_files = {r['path']: r for r in profile['expected_art_files']}
    assert len(new_files) == len(profile['expected_art_files'])
    for path in set(old_files) - ALLOWED_PREDECESSOR_CHANGES:
        assert new_files[path] == old_files[path], ('Unrelated art profile changed', path)
    for path in profile['sources']:
        assert sha((root / path).read_bytes()) == profile['sources'][path], ('Source changed', path)
    predecessor.validate_files(root, profile['expected_art_files'])
    baseline = root / HERE / 'broadleaf-canopy/Broadleaf.baseline.glb'
    assert sha(baseline.read_bytes()) == old_files[RUNTIME]['sha256'], 'Original Broadleaf not retained'
    expected = expected_manifest(profile, old_profile, root)
    actual = json.loads((root / historical.MANIFEST).read_text())
    assert actual == expected, 'Unreviewed manifest changes'
    for entry in actual['entries']:
        payload = (root / entry['local_path']).read_bytes()
        assert len(payload) == entry['size_bytes']
        assert historical.digest_payload(entry['local_path'], payload) == entry['digest']
    result = geometry(root / RUNTIME)
    assert result['triangles'] == profile['triangles']
    return dict(result, result='PASS', profile='island-broadleaf-canopy-20261016',
                art_blobs_audited=len(new_files), other_manifest_records_retained=47,
                all_other_art_and_world_bytes_retained=True, historical_profiles_retained=True,
                game_rendering_proof=False, physical_gpu_proof=False)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--root', type=Path, default=ROOT)
    p.add_argument('--output', type=Path)
    a = p.parse_args()
    result = check(a.root)
    text = json.dumps(result, indent=2) + '\n'
    if a.output:
        a.output.parent.mkdir(parents=True, exist_ok=True)
        a.output.write_text(text)
    print(text, end='')
