"""Exact PR24 + approved Conifer + six saved refinements profile, CPU only.

Historical four/five-asset profiles remain unchanged. This fixed successor
audits every art/world file, all 48 registrations and each structural recipe.
"""
if not __debug__:
    raise RuntimeError('Consolidated art checks require normal Python mode')

import argparse
import copy
import hashlib
import json
from pathlib import Path

import check_composed_island_graphics as historical
import check_conifer_leader as conifer
from check_finished_island_assets import SOURCE, check as check_finished

ROOT = Path(__file__).resolve().parents[1]
PROFILE = Path('scripts/consolidated-island-art-profile.json')
PROFILE_SHA256 = '3ab1ebac4a36d34a6efd9762469721b6c5991514fe0a9681d376abfd3370a38e'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def reject_symlinks(root, relative):
    current = root
    for part in Path(relative).parts:
        assert part not in ('..', '.') and not Path(relative).is_absolute()
        current /= part
        assert not current.is_symlink(), ('Art symlink forbidden', relative)


def validate_files(root, expected):
    actual = set()
    for directory in historical.ART_ROOTS:
        reject_symlinks(root, directory)
        for path in (root / directory).rglob('*'):
            assert not path.is_symlink(), ('Art symlink forbidden', path)
            if path.is_file():
                actual.add(path.relative_to(root).as_posix())
    records = {row['path']: row for row in expected}
    assert len(records) == len(expected)
    assert actual == set(records), 'Consolidated art path set changed'
    for name, record in records.items():
        reject_symlinks(root, name)
        path = root / name
        raw = path.read_bytes()
        assert len(raw) == record['bytes'] and sha(raw) == record['sha256'], ('Consolidated art bytes changed', name)
        assert bool(path.stat().st_mode & 0o111) == record['executable'], ('Consolidated art mode changed', name)
    return len(records)


def validate_manifest(root, profile, lineage):
    expected = copy.deepcopy(profile['base_manifest'])
    old_entries = historical.entries_by_path(expected)
    for asset in lineage['assets']:
        name = asset['runtime_path']
        assert old_entries[name]['asset_id'] == asset['asset_id']
        old_entries[name].update(digest=asset['runtime']['fnv1a64'],
            size_bytes=asset['runtime']['bytes'], generator={
                'config_path': 'scripts/build_finished_island_assets.py',
                'date': '2026-10-09', 'seed': 'saved-finished-island-refinements/' + asset['name'],
                'tool': 'Blender-4.3.2'})
    actual = json.loads((root / historical.MANIFEST).read_text())
    entries = historical.entries_by_path(actual)
    assert actual == expected, 'Manifest differs from exact Conifer base plus six reviewed records'
    for name, entry in entries.items():
        reject_symlinks(root, name)
        raw = (root / name).read_bytes()
        assert len(raw) == entry['size_bytes'], ('Manifest size mismatch', name)
        assert historical.digest_payload(name, raw) == entry['digest'], ('Manifest digest mismatch', name)
    return {'manifest_entries_valid': 48, 'six_refinement_records': 6,
            'other_complete_registrations_retained': 42, 'conifer_registration_retained': True}


def check(root=ROOT, history_root=None):
    root = Path(root).resolve()
    history_root = root if history_root is None else Path(history_root).resolve()
    raw = (root / PROFILE).read_bytes()
    assert sha(raw) == PROFILE_SHA256, 'Consolidated fixed profile changed'
    profile = json.loads(raw)
    # Imported validators execute beside this script, so validate that source
    # copy as well as an explicitly selected candidate root.
    for source_root in {ROOT.resolve(), root}:
        assert sha((source_root / PROFILE).read_bytes()) == PROFILE_SHA256, 'Executing profile changed'
        for name, expected_sha in profile['checker_and_recipe_sources'].items():
            reject_symlinks(source_root, name)
            assert sha((source_root / name).read_bytes()) == expected_sha, ('Checker/recipe source changed', name)
    blobs = validate_files(root, profile['expected_art_files'])
    lineage = json.loads((root / SOURCE / 'lineage.json').read_text())
    manifest = validate_manifest(root, profile, lineage)
    six = check_finished(root)
    shrubs = historical.check_shrub_structures(root,
        lambda path: historical.git_bytes(history_root, historical.BASE, path))
    fern = historical.check_fern(root / historical.HERE / 'fern-readability/Fern.baseline.glb', root / historical.HERE / 'Fern.glb')
    sapling = historical.check_sapling(root / historical.HERE / 'sapling-color/Sapling.baseline.glb', root / historical.HERE / 'Sapling.glb')
    leader = conifer.check_asset(root / historical.HERE / 'conifer-leader/Conifer.baseline.glb', root / historical.HERE / 'Conifer.glb')
    return dict(manifest, result='PASS', profile='pr24-conifer-six-finished-refinements-v1',
        reviewed_art_blobs_valid=blobs, six_finished_assets=six,
        shrubs=shrubs, fern=fern, sapling=sapling, conifer=leader,
        historical_profile_sources_retained=True, game_rendering_proof=False,
        physical_gpu_proof=False, full_workspace_qualification=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--history-root', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    text = json.dumps(check(args.root, args.history_root), indent=2) + '\n'
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text)
    print(text, end='')
