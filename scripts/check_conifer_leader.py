"""Exact Conifer geometry, editable-source identity and PR24 registration check.

CPU only. This narrow check is independent of the historical four-asset
composition gate. It does not validate all other runtime payloads or rendering.
"""
if not __debug__:
    raise RuntimeError('Conifer checker requires normal Python mode; -O/PYTHONOPTIMIZE is unsupported')

import argparse
import json
import subprocess
from pathlib import Path

import numpy as np

from derive_conifer_leader import components
from conifer_glb import Glb
from repair_conifer_leader import BASE_SHA256, FIXED_SHA256, ROOT, repair_bytes, sha


def winding(point, vertices, faces):
    t = vertices[faces]-point
    a, b, c = t[:, 0], t[:, 1], t[:, 2]
    lengths = np.linalg.norm(t, axis=2)
    numerator = np.einsum('ij,ij->i', a, np.cross(b, c))
    denominator = (np.prod(lengths, axis=1)+np.einsum('ij,ij->i', a, b)*lengths[:, 2]
                   +np.einsum('ij,ij->i', b, c)*lengths[:, 0]
                   +np.einsum('ij,ij->i', c, a)*lengths[:, 1])
    return float(np.sum(2*np.arctan2(numerator, denominator))/(4*np.pi))


def check_asset(baseline, candidate):
    old, new = Glb(baseline), Glb(candidate)
    assert sha(old.raw) == BASE_SHA256 and sha(new.raw) == FIXED_SHA256
    assert repair_bytes(old.raw) == new.raw
    assert old.doc == new.doc and len(old.raw) == len(new.raw) == 112240
    assert [m['name'] for m in old.doc['meshes']] == ['Conifer_L0', 'Conifer_L1', 'Conifer_L2']
    assert not any(any(k in n for k in ('matrix', 'translation', 'rotation', 'scale')) for n in new.doc['nodes'])
    allowed = np.zeros(len(old.raw), dtype=bool)
    result = []
    for level, mesh in enumerate(old.doc['meshes']):
        assert len(mesh['primitives']) == 1
        p = mesh['primitives'][0]
        verts = np.array(old.rows(p['attributes']['POSITION']))
        fixed = np.array(new.rows(p['attributes']['POSITION']))
        normals = np.array(new.rows(p['attributes']['NORMAL']))
        old_normals = np.array(old.rows(p['attributes']['NORMAL']))
        faces = np.array(old.rows(p['indices']), dtype=int).reshape(-1, 3)
        assert len(verts) == [2092, 744, 153][level]
        assert len(faces) == [3456, 668, 172][level]
        assert faces.min() >= 0 and faces.max() < len(verts)
        assert np.all(np.isfinite(fixed)) and np.all(np.isfinite(normals))
        assert np.max(np.abs(np.linalg.norm(normals, axis=1)-1)) < 1e-6
        assert np.all(np.linalg.norm(np.cross(fixed[faces[:, 1]]-fixed[faces[:, 0]], fixed[faces[:, 2]]-fixed[faces[:, 0]]), axis=1) > 1e-10)
        assert np.array_equal(verts.min(axis=0), fixed.min(axis=0))
        assert np.array_equal(verts.max(axis=0), fixed.max(axis=0))
        assert np.array_equal(verts[:, [0, 2]], fixed[:, [0, 2]])
        moved = np.any(verts != fixed, axis=1)
        changed_normals = np.any(old_normals != normals, axis=1)
        assert int(moved.sum()) == [10, 1, 11][level]
        assert int(changed_normals.sum()) == [20, 4, 27][level]
        groups = components(verts, faces)
        tip = max(groups, key=lambda ids: verts[ids, 1].max())
        assert np.array_equal(verts[tip], fixed[tip])
        tip_mask = np.zeros(len(verts), dtype=bool)
        tip_mask[tip] = True
        tip_faces = faces[np.all(tip_mask[faces], axis=1)]
        rest = np.setdiff1d(np.arange(len(verts)), tip)
        assert np.all(verts[moved, 1] > 9.4) and np.all(fixed[moved, 1] > verts[moved, 1])
        cap = fixed[moved]
        samples = [winding(point, verts, tip_faces) for point in [cap.mean(axis=0), *cap]]
        assert all(abs(abs(w)-1) < 1e-4 for w in samples)
        for attr, acc in p['attributes'].items():
            if attr not in ('POSITION', 'NORMAL'):
                assert old.rows(acc) == new.rows(acc)
        assert old.rows(p['indices']) == new.rows(p['indices'])
        for attr, mask in [('POSITION', moved), ('NORMAL', changed_normals)]:
            acc = p['attributes'][attr]
            for vertex in np.where(mask)[0]:
                off = old.offset(acc, int(vertex))
                allowed[off:off+12] = True
        result.append({'mesh': mesh['name'], 'vertices': len(verts), 'triangles': len(faces),
                       'position_vertices_changed': int(moved.sum()), 'normal_vertices_changed': int(changed_normals.sum()),
                       'original_tip_gap_m': float(verts[tip, 1].min()-verts[rest, 1].max()),
                       'candidate_cap_height_m': float(cap[:, 1].max()),
                       'candidate_cap_centroid_tip_winding': samples[0],
                       'all_cap_vertices_inside_original_tip_shell': True,
                       'bounds_exact': True, 'indices_colors_other_attributes_exact': True,
                       'all_foliage_positions_exact': True, 'finite_unit_normals_and_nondegenerate_triangles': True,
                       'geometric_overlap_not_topological_welding': True})
    differences = np.frombuffer(old.raw, dtype=np.uint8) != np.frombuffer(new.raw, dtype=np.uint8)
    assert np.all(allowed[differences])
    return {'result': 'PASS', 'baseline_sha256': BASE_SHA256, 'candidate_sha256': FIXED_SHA256,
            'file_size_bytes': len(new.raw), 'changed_byte_count': int(differences.sum()),
            'unchanged_entire_json_chunk': True, 'byte_differences_confined_to_audited_fields': True,
            'exact_stdlib_recipe_reproduction': True, 'lods': result,
            'limits': ['Narrow CPU asset/registration check, not whole-app or all-other-payload validation.',
                       'Editable source is a reconstruction; original authoring source is preserved.',
                       'Geometric trunk/tip overlap, not a topologically welded single component.',
                       'No renderer, GPU, training, frame timing, gameplay or runtime LOD-transition acceptance.']}



BASE_REVISION = '8634e01bd8ae28c34de94cd2f173d3e71633f035'
BASE_MANIFEST_SHA256 = '9dabbb65d493be47d4ffeae563cd6fde8d749d4c5d863e28714c74b4423a2dd8'
SOURCE_SHA256 = '40b36e82d0555cffd0a1c0973603ae71cccb110ad99876fe6ae34bc27f53d2ef'
HERE = Path('crates/alife_game_app/assets/landscape/island')
MANIFEST = Path('crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json')
GENERATOR = {'config_path': 'scripts/build_conifer_leader.py', 'date': '2026-10-12',
             'seed': 'conifer-central-leader-v1/from-8634e01', 'tool': 'Blender-4.3.2'}


def digest_bytes(payload):
    digest = 0xcbf29ce484222325
    for byte in payload:
        digest = ((digest ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    return f'fnv1a64:{digest:016x}'


def validate_registration(actual, baseline, candidate):
    import copy
    expected = copy.deepcopy(baseline)
    entries = expected['entries']
    assert len(entries) == 48
    assert len({e['asset_id'] for e in entries}) == 48
    assert len({e['local_path'] for e in entries}) == 48
    path = str(HERE / 'Conifer.glb')
    entry = next(e for e in entries if e['local_path'] == path)
    assert entry['asset_id'] == 'approved-island-conifer'
    assert sha(candidate) == FIXED_SHA256 and len(candidate) == 112240
    entry.update(digest=digest_bytes(candidate), size_bytes=len(candidate), generator=GENERATOR)
    assert actual == expected, 'Registration differs from the sole expected PR24 Conifer update'
    return {'entries': 48, 'complete_other_registrations_unchanged': 47,
            'conifer_asset_id': entry['asset_id'], 'conifer_digest': entry['digest'],
            'all_other_runtime_payloads_checked': False}


def check(root=ROOT, baseline_manifest=None):
    root = Path(root).resolve()
    if baseline_manifest is None:
        baseline_raw = subprocess.check_output(
            ['git', 'show', BASE_REVISION + ':' + str(MANIFEST)], cwd=root)
    else:
        baseline_raw = Path(baseline_manifest).read_bytes()
    assert sha(baseline_raw) == BASE_MANIFEST_SHA256, 'Not the exact pinned PR24 manifest'
    baseline = root / HERE / 'conifer-leader/Conifer.baseline.glb'
    candidate = root / HERE / 'Conifer.glb'
    report = check_asset(baseline, candidate)
    assert sha((root / HERE / 'conifer-leader-repair.blend').read_bytes()) == SOURCE_SHA256, 'Editable source changed; review required'
    report['source_identity_exact_to_recovered_package'] = True
    report['registration'] = validate_registration(
        json.loads((root / MANIFEST).read_text()), json.loads(baseline_raw), candidate.read_bytes())
    report['pinned_pr24_revision'] = BASE_REVISION
    report['fresh_blender_reload_or_bevy_loader'] = False
    return report


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--root', type=Path, default=ROOT)
    p.add_argument('--baseline-manifest', type=Path,
                   help='Optional exact PR24 manifest copy; its full SHA256 is required')
    p.add_argument('--output', type=Path,
                   help='Optional CPU receipt path; no output by default')
    a = p.parse_args()
    receipt = check(a.root, a.baseline_manifest)
    text = json.dumps(receipt, indent=2) + '\n'
    if a.output:
        a.output.parent.mkdir(parents=True, exist_ok=True)
        a.output.write_text(text)
    print(text, end='')


if __name__ == '__main__':
    main()
