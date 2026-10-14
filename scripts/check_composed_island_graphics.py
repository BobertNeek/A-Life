"""Check the exact reviewed PR20 + PR23 + PR24 art composition (CPU only).

The standalone shrub checker keeps its original, narrower preservation gate.
This separate profile pins every art blob and all 48 registrations to reviewed
source, then runs the existing shrub, Fern and Sapling structural checks.
Complete Git history containing the pinned revisions is required.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

from check_fern_readability import check as check_fern
from check_island_biological_shrubs import BASE, check_shrub_structures
from check_sapling_color import check as check_sapling

ROOT = Path(__file__).resolve().parents[1]
HERE = Path('crates/alife_game_app/assets/landscape/island')
MANIFEST = Path('crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json')
ART_ROOTS = ('crates/alife_game_app/assets', 'crates/alife_world/assets')
REVISIONS = {
    'shrubs': 'ec5bb040e976db7ecb6f64132e3fbcd9794d2c29',
    'fern': '021949fd48971c20b9d5adf5b837861de19fb7c6',
    'sapling': '8634e01bd8ae28c34de94cd2f173d3e71633f035',
}
CHANGES = {
    str(HERE / 'BerryBush.glb'): REVISIONS['shrubs'],
    str(HERE / 'CoastalShrub.glb'): REVISIONS['shrubs'],
    str(HERE / 'Fern.glb'): REVISIONS['fern'],
    str(HERE / 'Sapling.glb'): REVISIONS['sapling'],
}
NEW_ARTIFACTS = {str(HERE / name) for name in (
    'island-biological-shrubs.blend', 'fern-readability.blend',
    'fern-readability/Fern.baseline.glb', 'sapling-color.blend',
    'sapling-color/Sapling.baseline.glb',
)}


def git_bytes(root, revision, path):
    return subprocess.check_output(
        ['git', 'show', revision + ':' + str(path)], cwd=root)


def git_art_tree(root, revision):
    raw = subprocess.check_output(
        ['git', 'ls-tree', '-rz', revision, '--', *ART_ROOTS], cwd=root)
    result = {}
    for row in raw.split(b'\0'):
        if not row:
            continue
        metadata, name = row.split(b'\t', 1)
        mode, kind, digest = metadata.decode().split()
        assert kind == 'blob' and mode in ('100644', '100755'), name
        result[name.decode()] = (mode, digest)
    return result


def entries_by_path(manifest):
    entries = manifest['entries']
    paths = [e['local_path'] for e in entries]
    ids = [e['asset_id'] for e in entries]
    assert len(entries) == len(set(paths)) == len(set(ids)) == 48, 'Registration set changed'
    assert all(not Path(p).is_absolute() and '..' not in Path(p).parts for p in paths)
    return {e['local_path']: e for e in entries}


def digest_payload(path, payload):
    data = payload.replace(b'\r\n', b'\n').replace(b'\r', b'\n') if Path(path).suffix == '.json' else payload
    value = 0xcbf29ce484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    return f'fnv1a64:{value:016x}'


def validate_manifest(root, baseline, reviewed, reviewed_payloads):
    old, expected = entries_by_path(baseline), entries_by_path(reviewed)
    assert list(old) == list(expected), 'Registration paths/order changed'
    assert {k: v for k, v in baseline.items() if k != 'entries'} == {
        k: v for k, v in reviewed.items() if k != 'entries'}, 'Manifest metadata changed'
    changed = {path for path in old if old[path] != expected[path]}
    assert changed == set(CHANGES), 'Reviewed manifest changes exceed or omit the four assets'
    assert set(reviewed_payloads) == set(CHANGES), 'Reviewed payload set changed'
    for path in changed:
        mutable = {'digest', 'size_bytes', 'generator'}
        assert {k: v for k, v in old[path].items() if k not in mutable} == {
            k: v for k, v in expected[path].items() if k not in mutable}, path
    actual = json.loads((root / MANIFEST).read_text())
    entries_by_path(actual)
    assert actual == reviewed, 'Candidate manifest differs from the exact reviewed registration set'
    for path, entry in expected.items():
        payload = (root / path).read_bytes()
        assert len(payload) == entry['size_bytes'], ('Size mismatch', path)
        assert digest_payload(path, payload) == entry['digest'], ('Digest mismatch', path)
        if path in CHANGES:
            assert payload == reviewed_payloads[path], ('Not the exact reviewed revision', path)
    return {'manifest_entries_valid': len(expected),
            'unchanged_registrations': len(expected) - len(changed),
            'authorized_runtime_paths': sorted(changed)}


def validate_reviewed_trees(baseline, reviewed):
    assert not set(baseline) - set(reviewed), 'Reviewed art deletes baseline files'
    assert set(reviewed) - set(baseline) == NEW_ARTIFACTS, 'Reviewed source artifacts changed'
    changed = {p for p in baseline if baseline[p] != reviewed[p]}
    expected = set(CHANGES) | {str(MANIFEST), str(HERE / 'README.md')}
    assert changed == expected, 'Reviewed art changes exceed or omit the explicit profile'


def validate_candidate_tree(root, reviewed):
    actual = set()
    for name in ART_ROOTS:
        for path in (root / name).rglob('*'):
            assert not path.is_symlink(), ('Unexpected art symlink', path)
            if path.is_file():
                actual.add(path.relative_to(root).as_posix())
    assert actual == set(reviewed), 'Candidate art paths differ from reviewed source'
    for name, (mode, digest) in reviewed.items():
        path = root / name
        payload = path.read_bytes()
        blob = b'blob ' + str(len(payload)).encode() + b'\0' + payload
        assert hashlib.sha1(blob).hexdigest() == digest, ('Art blob changed', name)
        assert bool(path.stat().st_mode & 0o111) == (mode == '100755'), ('Art mode changed', name)
    return len(actual)


def check(root=ROOT):
    root = Path(root).resolve()
    reviewed_ref = REVISIONS['sapling']
    baseline_tree = git_art_tree(root, BASE)
    reviewed_tree = git_art_tree(root, reviewed_ref)
    validate_reviewed_trees(baseline_tree, reviewed_tree)
    blobs = validate_candidate_tree(root, reviewed_tree)
    payloads = {p: git_bytes(root, ref, p) for p, ref in CHANGES.items()}
    report = validate_manifest(root,
        json.loads(git_bytes(root, BASE, MANIFEST)),
        json.loads(git_bytes(root, reviewed_ref, MANIFEST)), payloads)
    report.update({
        'result': 'PASS', 'baseline': BASE, 'reviewed_revisions': REVISIONS,
        'reviewed_art_blobs_valid': blobs,
        'shrubs': check_shrub_structures(root, lambda p: git_bytes(root, BASE, p)),
        'fern': check_fern(root / HERE / 'fern-readability/Fern.baseline.glb', root / HERE / 'Fern.glb'),
        'sapling': check_sapling(root / HERE / 'sapling-color/Sapling.baseline.glb', root / HERE / 'Sapling.glb'),
        'game_rendering_proof': False, 'physical_gpu_proof': False,
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
