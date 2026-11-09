#!/usr/bin/env python3
"""Read/reconstruct source bytes only. Never run Cargo, rustc or engine code."""
import copy
import functools
import importlib.util
import json
import pathlib
import subprocess
import sys
import tomllib
from packet_inputs import ROOT, input_path, prior, read, sha, verify_preservation

spec = importlib.util.spec_from_file_location('source_patch', input_path('source_patch.py'))
p = importlib.util.module_from_spec(spec); spec.loader.exec_module(p)
MAIN = 'c3aec16df33861f1316d127736f5f60eea327ede'
FBF = 'fbf2a517f93098b5d936878a3d26bbb8d0391c38'


def record(name, data):
    return {'path': name, 'bytes': len(data), 'sha256': sha(data)}


def save(name, data):
    (ROOT / name).write_text(json.dumps(data, indent=2, sort_keys=True) + '\n')


def main():
    repo, patch_path, patch_sha, metadata_path = sys.argv[1:]
    verify_preservation()
    data = pathlib.Path(patch_path).read_bytes()
    assert sha(data) == patch_sha and b'\0' not in data and b'GIT binary patch' not in data
    data.decode('utf-8', 'strict')
    metadata = json.loads(pathlib.Path(metadata_path).read_bytes())
    assert metadata['patch_sha256'] == patch_sha and metadata['patch_bytes'] == len(data)
    name = 'patches/grip-v5-to-v6.patch'
    (ROOT / name).write_bytes(data)
    save('CORRECTION.json', metadata)

    @functools.lru_cache(maxsize=None)
    def blob(path):
        result = subprocess.run(['git', '-C', repo, 'show', MAIN + ':' + path],
                                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        return None if result.returncode else result.stdout

    recovery = json.loads(prior(2, 'recovery.json'))
    virtual = {}
    for step in recovery['steps']:
        patch = b''.join(read(row['path']) for row in step['parts'])
        assert sha(patch) == step['sha256'] and len(patch) == step['bytes']
        virtual.update(p.apply(patch, lambda path: virtual.get(path, blob(path))))
    old = json.loads(prior(2, 'source-manifest.json'))
    for row in old['files']:
        assert record(row['path'], virtual.get(row['path'], blob(row['path']))) == row
    fixes = p.apply(data, lambda path: virtual.get(path, blob(path)))
    assert set(fixes) == {'Cargo.lock', 'crates/alife_world/Cargo.toml'}
    for row in metadata['files']:
        assert sha(virtual.get(row['path'], blob(row['path']))) == row['before_sha256']
        assert record(row['path'], fixes[row['path']]) == {k: row[k] for k in ('path', 'bytes', 'sha256')}
    assert {row['path'] for row in metadata['files']} == set(fixes)

    # Verify this is only an existing workspace dependency edge, not a version,
    # registry/source, default-feature, target, runtime or production dep change.
    before_toml = tomllib.loads(virtual.get('crates/alife_world/Cargo.toml', blob('crates/alife_world/Cargo.toml')).decode())
    after_toml = tomllib.loads(fixes['crates/alife_world/Cargo.toml'].decode())
    expected_toml = copy.deepcopy(before_toml)
    assert expected_toml['dev-dependencies'] == {}
    expected_toml['dev-dependencies']['bincode'] = {'workspace': True}
    assert after_toml == expected_toml
    before_lock = tomllib.loads(virtual.get('Cargo.lock', blob('Cargo.lock')).decode())
    after_lock = tomllib.loads(fixes['Cargo.lock'].decode())
    expected_lock = copy.deepcopy(before_lock)
    world = next(row for row in expected_lock['package'] if row['name'] == 'alife_world')
    assert 'bincode' not in world['dependencies']
    world['dependencies'].append('bincode'); world['dependencies'].sort()
    assert after_lock == expected_lock
    assert len([x for x in before_lock['package'] if x['name'] == 'bincode']) == 1
    virtual.update(fixes)

    parent_map_path = ROOT.parent / 'REGISTERED-SOURCE-v6.json'
    parent_map = json.loads(parent_map_path.read_bytes())
    assert parent_map['base_commit'] == FBF
    for row in parent_map['files']:
        assert record(row['path'], virtual.get(row['path'], blob(row['path']))) == {k: row[k] for k in ('path', 'bytes', 'sha256')}
    freeze_path = ROOT.parent / 'SOURCE-FREEZE-v6.json'
    freeze = json.loads(freeze_path.read_bytes())
    assert freeze['mapped_paths'] == len(parent_map['files'])
    assert freeze['correction_patch_sha256'] == patch_sha
    assert freeze['cargo_lock_sha256'] == sha(fixes['Cargo.lock'])
    (ROOT / 'SOURCE-FREEZE-v6.json').write_bytes(freeze_path.read_bytes())
    # Delta registration avoids another complete duplicated source-map file.
    save('REGISTRATION-DELTA-v6.json', {'schema': 'grip-registration-delta-v6',
        'base_registration': 'registered-source-v5.json',
        'base_registration_sha256': sha(read('registered-source-v5.json')),
        'base_source_commit': 'f5ce4d6de7ddf2e04473befefc715cc48bb28f26',
        'parent_v6_registered_map_sha256': sha(parent_map_path.read_bytes()),
        'source_closure': freeze['source_closure'],
        'mapped_paths': len(parent_map['files']),
        'overrides': [record(path, fixes[path]) for path in sorted(fixes)]})
    manifest = copy.deepcopy(old)
    manifest.update(schema='m2-grip-complete-nominal-source-v3',
        scope='Exact fbf nominal source plus grip V4, V5 matches and V6 world test dependency edge. Targets/features/guard semantics unchanged.',
        dependency_correction_patch_sha256=patch_sha,
        prior_source_manifest_sha256=sha(prior(2, 'source-manifest.json')),
        registered_map_sha256=sha(parent_map_path.read_bytes()),
        registered_source_closure=freeze['source_closure'],
        registered_source_closure_method='Parent SOURCE-FREEZE-v6 identity; every registered path/bytes/SHA256 independently checked against recovery',
        packet_registration_sha256=sha((ROOT / 'REGISTRATION-DELTA-v6.json').read_bytes()))
    manifest['files'] = [record(row['path'], virtual.get(row['path'], blob(row['path']))) for row in old['files']]
    assert sum(a != b for a, b in zip(old['files'], manifest['files'])) == 2
    save('source-manifest.json', manifest)
    recovery['schema'] = 'm2-grip-recovery-chain-v3'
    recovery['steps'].append({'name': 'grip-v6-world-devdep', **record(name, data), 'parts': [record(name, data)]})
    save('recovery.json', recovery)
    deps = json.loads(prior(2, 'dependencies.json'))
    deps.update(schema='m2-grip-dependencies-v3', cargo_lock_sha256=sha(fixes['Cargo.lock']),
        source_change='Only world dev-dependency bincode.workspace=true and its existing Cargo.lock dependency edge.',
        package_versions_changed=False, registry_packages_added=False,
        accepted_main_cargo_lock_sha256=sha(blob('Cargo.lock')),
        immediate_prior_cargo_lock_sha256=old['files'][next(i for i, x in enumerate(old['files']) if x['path'] == 'Cargo.lock')]['sha256'])
    save('dependencies.json', deps)
    save('SOURCE-DELTA.json', {'schema': 'm2-grip-v6-hosted-source-delta-v1',
        'correction_patch': record(name, data),
        'changed_files': [{'before': a, 'after': b} for a, b in zip(old['files'], manifest['files']) if a != b],
        'semantic_checks': {'only_world_dev_dependency_added': True, 'only_existing_bincode_lock_edge_added': True,
                            'package_versions_unchanged': True, 'features_unchanged': True, 'source_code_unchanged': True},
        'expected_prior_failure': {'code': 'E0433', 'path': 'crates/alife_world/src/headless/grip_options.rs', 'lines': [212, 213, 219, 220], 'stage': 'world'}})
    publication = json.loads(prior(2, 'publication.json'))
    publication = json.loads(json.dumps(publication).replace('m2-grip-hosted-typecheck-v2', 'm2-grip-hosted-typecheck-v3'))
    publication.update(schema='m2-grip-source-publication-v3',
        source_reference_commit='f5ce4d6de7ddf2e04473befefc715cc48bb28f26',
        old_studies='V1/V2/V4/V5 and all prior workflows/markers are ineligible. Prepared v3 is parented at accepted main and contains only its new v3 workflow.')
    save('publication.json', publication)
    prep = json.loads(prior(2, 'PREPARATION.json'))
    prep.pop('hosted_v2_native_invocations', None)
    prep.update(schema='m2-grip-source-preparation-v3', source_ready=True,
        registered_grip_paths_verified=len(parent_map['files']), correction_bytes=len(data), correction_sha256=patch_sha,
        correction_files=2, v1_preserved=True, v2_preserved=True, v4_preserved=True, v5_preserved=True,
        source_reconstruction='main -> fbf library closure -> nominal supplement -> V4 -> V5 matches -> V6 dev-dependency edge',
        toolchain_status='Same official toolchain succeeded on V1/V2. New V3 not run.', hosted_v3_native_invocations=0,
        native_invocations=0, network_calls=0)
    save('PREPARATION.json', prep)
    verify_preservation()
    print(json.dumps({'source_ready': True, 'bound_files': len(manifest['files']),
                      'registered_paths': len(parent_map['files']), 'correction_bytes': len(data),
                      'new_lock_sha256': sha(fixes['Cargo.lock']), 'native_invocations': 0}))


if __name__ == '__main__':
    main()
