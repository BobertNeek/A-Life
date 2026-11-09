#!/usr/bin/env python3
"""Prepare one source-only overlay atop immutable hosted v1; never run native code."""
import copy
import functools
import hashlib
import json
import pathlib
import subprocess
import sys
from source_patch import apply

ROOT = pathlib.Path(__file__).resolve().parent
V1 = ROOT.parent / 'hosted-typecheck-v1'
MAIN = 'c3aec16df33861f1316d127736f5f60eea327ede'
FBF = 'fbf2a517f93098b5d936878a3d26bbb8d0391c38'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def record(name, data):
    return {'path': name, 'bytes': len(data), 'sha256': sha(data)}


def save(name, value):
    (ROOT / name).write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')


def verify_v1():
    evidence = json.loads((ROOT / 'V1-PRESERVATION.json').read_bytes())
    for row in evidence['files']:
        data = (V1 / row['path']).read_bytes()
        assert record(row['path'], data) == row
    assert sha((ROOT.parent / 'fresh-grip-options-from-fbf2a517-v4.patch').read_bytes()) == evidence['v4_patch_sha256']


def main():
    repo, correction, expected, correction_metadata = map(str, sys.argv[1:])
    repo = pathlib.Path(repo).resolve()
    correction = pathlib.Path(correction).resolve()
    correction_metadata = pathlib.Path(correction_metadata).resolve()
    verify_v1()
    data = correction.read_bytes()
    assert sha(data) == expected
    data.decode('utf-8', 'strict')
    assert b'\0' not in data and b'GIT binary patch' not in data
    name = 'patches/grip-v4-to-v5.patch'
    (ROOT / name).write_bytes(data)

    @functools.lru_cache(maxsize=None)
    def blob(path):
        p = subprocess.run(['git', '-C', str(repo), 'show', MAIN + ':' + path],
                           stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        return None if p.returncode else p.stdout

    virtual = {}
    recovery = json.loads((V1 / 'recovery.json').read_bytes())
    for step in recovery['steps']:
        patch = b''.join((ROOT / row['path']).read_bytes() for row in step['parts'])
        assert sha(patch) == step['sha256'] and len(patch) == step['bytes']
        virtual.update(apply(patch, lambda path: virtual.get(path, blob(path))))
    old_manifest = json.loads((V1 / 'source-manifest.json').read_bytes())
    for row in old_manifest['files']:
        assert record(row['path'], virtual.get(row['path'], blob(row['path']))) == row
    metadata = json.loads(correction_metadata.read_bytes())
    assert metadata['patch_sha256'] == expected and metadata['patch_bytes'] == len(data)
    assert metadata['parent_patch_sha256'] == old_manifest['grip_patch_sha256']
    fixes = apply(data, lambda path: virtual.get(path, blob(path)))
    assert set(fixes) == {row['path'] for row in metadata['files']}
    assert len(fixes) == 3
    for row in metadata['files']:
        assert sha(virtual.get(row['path'], blob(row['path']))) == row['before_sha256']
        assert sha(fixes[row['path']]) == row['sha256'] and len(fixes[row['path']]) == row['bytes']
    virtual.update(fixes)
    prior_registered_path = ROOT.parent / 'REGISTERED-SOURCE-v4.json'
    prior_registered = json.loads(prior_registered_path.read_bytes())
    mapped = {row['path']: {k: row[k] for k in ('path', 'bytes', 'sha256')} for row in prior_registered['files']}
    mapped.update({path: record(path, value) for path, value in fixes.items()})
    registered = {'schema': 'grip-source-registration-v5-overlay', 'base_commit': FBF,
        'v4_registered_map_sha256': sha(prior_registered_path.read_bytes()),
        'correction_metadata_sha256': sha(correction_metadata.read_bytes()),
        'status': 'source-registered-hosted-v2-not-run', 'files': [mapped[path] for path in sorted(mapped)]}
    save('registered-source-v5.json', registered)
    save('CORRECTION.json', metadata)
    registered_path = ROOT / 'registered-source-v5.json'
    parent_map_path = ROOT.parent / 'REGISTERED-SOURCE-v5.json'
    parent_map = json.loads(parent_map_path.read_bytes())
    assert [{k: row[k] for k in ('path', 'bytes', 'sha256')} for row in parent_map['files']] == registered['files']
    freeze_path = ROOT.parent / 'SOURCE-FREEZE-v5.json'
    freeze = json.loads(freeze_path.read_bytes())
    assert freeze['mapped_paths'] == len(registered['files']) == 147
    assert freeze['correction_patch_sha256'] == expected
    (ROOT / 'SOURCE-FREEZE-v5.json').write_bytes(freeze_path.read_bytes())
    assert registered['base_commit'] == FBF
    for row in registered['files']:
        got = virtual.get(row['path'], blob(row['path']))
        assert got is not None and sha(got) == row['sha256'] and len(got) == row['bytes']
    old_paths = {row['path'] for row in old_manifest['files']}
    assert set(fixes) <= old_paths
    manifest = copy.deepcopy(old_manifest)
    manifest.update(schema='m2-grip-complete-nominal-source-v2',
                    scope='Exact fbf nominal source superset plus immutable grip V4 and the three-file V5 exhaustive-match corrections. Scope and feature selection unchanged from hosted v1.',
                    correction_patch_sha256=expected,
                    registered_map_sha256=sha(parent_map_path.read_bytes()),
                    packet_registration_sha256=sha(registered_path.read_bytes()),
                    registered_source_closure=freeze['source_closure'],
                    registered_source_closure_method='Parent SOURCE-FREEZE-v5 identity; all 147 path/bytes/SHA256 rows independently checked against recovery',
                    prior_source_manifest_sha256=sha((V1 / 'source-manifest.json').read_bytes()))
    manifest['files'] = [record(row['path'], virtual.get(row['path'], blob(row['path']))) for row in old_manifest['files']]
    assert sum(a != b for a, b in zip(old_manifest['files'], manifest['files'])) == 3
    save('source-manifest.json', manifest)
    recovery['schema'] = 'm2-grip-recovery-chain-v2'
    recovery['steps'].append({'name': 'grip-v5-correction', **record(name, data), 'parts': [record(name, data)]})
    save('recovery.json', recovery)
    save('SOURCE-DELTA.json', {'schema': 'm2-grip-v5-hosted-source-delta-v1',
        'v1_prepared_head': '4a3a39be5f8a4a95f662ed2e7c3a67ed50b62da1',
        'v1_activation_head': '9f68e9c278f037c845e38354243570db223e5e43',
        'prior_full_grip_v4_sha256': old_manifest['grip_patch_sha256'],
        'correction_patch': record(name, data),
        'registered_map_sha256': sha(registered_path.read_bytes()),
        'changed_files': [{'before': before, 'after': after} for before, after in zip(old_manifest['files'], manifest['files']) if before != after],
        'expected_prior_failure': {'code': 'E0004', 'path': 'crates/alife_world/tests/gpu_memory_grounding_persistence.rs', 'line': 201, 'stage': 'world'},
        'candidate_semantics': 'Add raw6 to the world fixture loop and exhaustive match; name raw5/raw6 explicitly in game-app evidence labels; explicitly reject both CPU research profiles in GPU evidence soak. No GPU admission, stage/feature changes, or source/body execution.',
        'v1_preserved': True, 'v4_preserved': True})
    publication = copy.deepcopy(json.loads((V1 / 'publication.json').read_bytes()))
    publication = json.loads(json.dumps(publication).replace('m2-grip-hosted-typecheck-v1', 'm2-grip-hosted-typecheck-v2'))
    publication.update(schema='m2-grip-source-publication-v2',
        reuse_manifest='REUSE-MANIFEST.json',
        source_reference_commit='4a3a39be5f8a4a95f662ed2e7c3a67ed50b62da1',
        source_reference_role='Whitelist immutable existing blob reuse only. Not the prepared branch parent and never an activation reference.',
        archive_mode='update-only; parent must combine listed immutable reused blobs with all archive members at exact v2 destinations before creating the prepared commit',
        old_studies='V1, V4 and all prior study workflows/markers are ineligible. The new v2 prepared branch is parented at accepted main and contains only its new v2 workflow.')
    save('publication.json', publication)
    prior = json.loads((V1 / 'PREPARATION.json').read_bytes())
    prior.update(schema='m2-grip-source-preparation-v2', source_ready=True,
        registered_grip_paths_verified=len(registered['files']),
        correction_bytes=len(data), correction_sha256=expected,
        correction_files=len(fixes), v1_preserved=True, v4_preserved=True,
        source_reconstruction='main -> exact inherited fbf library closure -> nominal supplement -> immutable grip V4 -> three-file V5 correction',
        toolchain_status='Same official Rust 1.99.1 setup succeeded on hosted v1. New v2 not executed.',
        hosted_v2_native_invocations=0, network_calls=0)
    save('PREPARATION.json', prior)
    verify_v1()
    if (ROOT / 'PENDING.json').exists():
        (ROOT / 'PENDING.json').unlink()
    print(json.dumps({'source_ready': True, 'bound_files': len(manifest['files']),
                      'registered_paths_verified': len(registered['files']),
                      'correction_bytes': len(data), 'correction_sha256': expected,
                      'v1_preserved': True, 'native_invocations': 0}))


if __name__ == '__main__':
    main()
