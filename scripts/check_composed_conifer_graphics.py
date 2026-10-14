"""Check the fixed PR24 + reconstructed Conifer art composition (CPU only).

This is a separate profile. The exact historical PR31 graphics checker and
standalone shrub retention contract remain unchanged. Pinned Git history is
required; no candidate-derived allow-list or missing-history fallback is used.
"""
if not __debug__:
    raise RuntimeError('Composed Conifer checker requires normal Python mode; -O/PYTHONOPTIMIZE is unsupported')

import argparse
import hashlib
import json
import os
from pathlib import Path

# Fixed object identities must never follow a repository's replacement refs.
os.environ['GIT_NO_REPLACE_OBJECTS'] = '1'

ROOT = Path(__file__).resolve().parents[1]
PARENT = '8634e01bd8ae28c34de94cd2f173d3e71633f035'
HISTORICAL_CHECKER_REVISION = '9363be21bc787145cff4969dd2b9672c23f8d4f1'
HERE = Path('crates/alife_game_app/assets/landscape/island')
MANIFEST = Path('crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json')
CONIFER = str(HERE / 'Conifer.glb')
# (parent blob, fixed new blob); all reviewed overlays are non-executable.
ART_OVERLAY = {
    CONIFER: ('9b8871c0b789c2ab4ed23edb3cfd3c4c0c601b42', '6947ec7702724f90ecb504441d38408f541cf653'),
    str(HERE / 'README.md'): ('0767beaa0a8aeca7eb298a88a52efaee24192981', 'df325cc9cff99c1d9f4ea720ad24ec293702977f'),
    str(HERE / 'conifer-leader-repair.blend'): (None, '35e66ff85a2cc1dabc731863757ef78cdc51194c'),
    str(HERE / 'conifer-leader/Conifer.baseline.glb'): (None, '9b8871c0b789c2ab4ed23edb3cfd3c4c0c601b42'),
    str(MANIFEST): ('5b10075c450fd85e484ab088cbb33e28994d1116', '922a50215f043607a8f9a7f74f6f04b9d658b092'),
}
CONIFER_SCRIPTS = {
    'scripts/build_conifer_leader.py': '03ad81e244cc056b9c1801b0773ada79c02207dc',
    'scripts/check_conifer_leader.py': '1430f3ca4b7e182d268de60332ec313912611922',
    'scripts/conifer-leader-patch.json': '313d0314655c4da81f009f0389348608185989b6',
    'scripts/conifer_glb.py': 'da3afbd6fbd70077a2f7f3f38123e97eaa7df353',
    'scripts/derive_conifer_leader.py': 'fba440179892a710e1714084cdd48bb6c0497759',
    'scripts/repair_conifer_leader.py': 'b2b2f99ad0139947ced78a4f04a11adab1effe09',
}
# Protect the reused assertion-based helpers before importing or invoking them.
# PR31 changed only the composition driver and shrub adapter in this closure.
HISTORICAL_HELPERS = {
    'scripts/check_composed_island_graphics.py': '03f3d141901b57543b4b5dab060731f99babb770',
    'scripts/check_island_biological_shrubs.py': '592587808b263b331d391fc12af567faab5eeb97',
    'scripts/check_fern_readability.py': '024a6a579fe79b1fd4803f1624f26f6e6e71dcd8',
    'scripts/check_sapling_color.py': '3efcc230d702350eb535c9abf479b2db8ee6101f',
    'scripts/check_island_art.py': '08010224c92957dd32d21176d647ccb928d72a1c',
    'scripts/retain_fern_geometry.py': 'c470fe3a1bcc383e9b94492a7644ce65185ee613',
    'scripts/retain_sapling_geometry.py': '68f2e31362ea48bac82f975bb15b890e57326521',
}


def blob_id(payload):
    return hashlib.sha1(b'blob ' + str(len(payload)).encode() + b'\0' + payload).hexdigest()


def reject_symlink_path(root, relative):
    path = root
    for part in Path(relative).parts:
        path = path / part
        assert not path.is_symlink(), ('Unexpected symlink', path)


def validate_script_identities(root):
    expected = {**HISTORICAL_HELPERS, **CONIFER_SCRIPTS}
    for name, digest in expected.items():
        reject_symlink_path(root, name)
        path = root / name
        assert path.is_file(), ('Missing checker/source file', name)
        assert blob_id(path.read_bytes()) == digest, ('Checker/source blob changed', name)
        assert not path.stat().st_mode & 0o111, ('Checker/source mode changed', name)
    allowed = set(CONIFER_SCRIPTS) | {
        'scripts/check_composed_conifer_graphics.py', 'scripts/test_composed_conifer_graphics.py'}
    actual = {p.relative_to(root).as_posix() for p in (root / 'scripts').glob('*conifer*')}
    assert actual <= allowed, ('Unexpected Conifer source paths', sorted(actual - allowed))
    return len(CONIFER_SCRIPTS)


# Check the executing source copy as well as the later --root candidate.
validate_script_identities(ROOT)
import check_composed_island_graphics as historical
import check_conifer_leader as conifer


def expected_art_tree(parent):
    assert len(parent) == 113, 'Pinned PR24 art record count changed'
    expected = dict(parent)
    for name, (old, new) in ART_OVERLAY.items():
        if old is None:
            assert name not in parent, ('Reviewed source addition exists in parent', name)
        else:
            assert parent.get(name) == ('100644', old), ('Pinned PR24 overlay preimage changed', name)
        expected[name] = ('100644', new)
    assert len(expected) == 115
    return expected


def validate_manifest(root, parent_raw):
    assert hashlib.sha256(parent_raw).hexdigest() == conifer.BASE_MANIFEST_SHA256, 'Not the exact pinned PR24 manifest'
    parent = json.loads(parent_raw)
    actual = json.loads((root / MANIFEST).read_text())
    historical.entries_by_path(parent)
    entries = historical.entries_by_path(actual)
    conifer.validate_registration(actual, parent, (root / CONIFER).read_bytes())
    for name, entry in entries.items():
        reject_symlink_path(root, name)
        payload = (root / name).read_bytes()
        assert len(payload) == entry['size_bytes'], ('Size mismatch', name)
        assert historical.digest_payload(name, payload) == entry['digest'], ('Digest mismatch', name)
    return {'manifest_entries_valid': 48, 'complete_registrations_unchanged_from_pr24': 47,
            'authorized_runtime_paths_from_pr24': [CONIFER],
            'reviewed_runtime_refreshes_from_pr20': 5, 'unchanged_registrations_from_pr20': 43}


def check(root=ROOT):
    root = Path(root).resolve()
    scripts = validate_script_identities(root)
    baseline = historical.git_art_tree(root, historical.BASE)
    parent = historical.git_art_tree(root, PARENT)
    historical.validate_reviewed_trees(baseline, parent)
    expected = expected_art_tree(parent)
    for name in historical.ART_ROOTS:
        reject_symlink_path(root, name)
    blobs = historical.validate_candidate_tree(root, expected)
    report = validate_manifest(root, historical.git_bytes(root, PARENT, MANIFEST))
    # Do not catch failures or emit PASS until every structural validator returns.
    shrubs = historical.check_shrub_structures(root, lambda p: historical.git_bytes(root, historical.BASE, p))
    fern = historical.check_fern(root / HERE / 'fern-readability/Fern.baseline.glb', root / HERE / 'Fern.glb')
    sapling = historical.check_sapling(root / HERE / 'sapling-color/Sapling.baseline.glb', root / HERE / 'Sapling.glb')
    leader = conifer.check(root)
    report.update({
        'result': 'PASS', 'profile': 'pr24-plus-reconstructed-conifer-v1',
        'pinned_pr24_parent': PARENT, 'historical_checker_revision': HISTORICAL_CHECKER_REVISION,
        'historical_revisions': historical.REVISIONS,
        'reviewed_art_blobs_valid': blobs, 'retained_pr24_art_blobs_valid': 110,
        'reviewed_conifer_script_blobs_valid': scripts,
        'shrubs': shrubs, 'fern': fern, 'sapling': sapling, 'conifer': leader,
        'game_rendering_proof': False, 'physical_gpu_proof': False,
        'full_workspace_qualification': False,
    })
    return report


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
