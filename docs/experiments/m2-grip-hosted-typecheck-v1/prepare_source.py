#!/usr/bin/env python3
"""Read Git source bytes and write only this packet. No native jobs/network."""
import functools
import hashlib
import importlib.util
import json
import pathlib
import posixpath
import re
import subprocess
import sys
import tomllib
from source_patch import apply

ROOT = pathlib.Path(__file__).resolve().parent
MAIN = 'c3aec16df33861f1316d127736f5f60eea327ede'
FBF = 'fbf2a517f93098b5d936878a3d26bbb8d0391c38'
GRIP = 'acf8cc50353ba76cb7adf5af1db5845ee6e6b2e2261d0ee823821428e2e4d054'
REPO = pathlib.Path(sys.argv[1]).resolve()
GRIP_ROOT = pathlib.Path(sys.argv[2]).resolve()

def git(*args):
    return subprocess.check_output(['git', '-C', str(REPO), *args], stderr=subprocess.DEVNULL)

@functools.lru_cache(maxsize=None)
def blob(rev, path):
    try:
        return git('show', rev + ':' + path)
    except subprocess.CalledProcessError:
        return None

def sha(b):
    return hashlib.sha256(b).hexdigest()

def record(path, data):
    return {'path': path, 'bytes': len(data), 'sha256': sha(data)}

def save(name, obj):
    (ROOT / name).write_text(json.dumps(obj, indent=2, sort_keys=True) + '\n')

assert git('rev-parse', MAIN).decode().strip() == MAIN
assert git('rev-parse', FBF).decode().strip() == FBF
files = git('ls-tree', '-r', '--name-only', FBF).decode().splitlines()
assert len(files) == len(set(files))
closure = json.loads((ROOT / 'closure.json').read_bytes())
closure_patch = b''.join((ROOT / row['path']).read_bytes() for row in closure['patch']['parts'])
assert sha(closure_patch) == closure['patch']['sha256']
reconstructed = apply(closure_patch, lambda p: blob(MAIN, p))
for row in closure['files']:
    assert reconstructed[row['path']] == blob(FBF, row['path'])
    assert record(row['path'], reconstructed[row['path']])['sha256'] == row['sha256']

# Exact source superset: every crate .rs/TOML/WGSL/SQL, plus workspace/lock/config.
# This deliberately binds transitive local crates and target/module source even
# when a declared feature disables it; build dep-info is checked afterward.
source_paths = {p for p in files if p.startswith('crates/') and p.endswith(('.rs', '.toml', '.wgsl', '.sql'))}
source_paths |= {'Cargo.toml', 'Cargo.lock', '.cargo/config.toml'}
metadata = ['docs/experiments/bounded-founder-v1/legacy-wire-capture.json',
            'docs/experiments/motor-lobe-native-m0-v1/mediation-m0-manifest.json']
source_paths.update(metadata)
extra_paths = sorted(source_paths - set(reconstructed))
supplement = git('diff', '--no-ext-diff', '--no-renames', '--no-color', MAIN, FBF, '--', *extra_paths)
assert b'GIT binary patch' not in supplement
supplement.decode('utf-8')
(ROOT / 'patches/fbf-nominal-source-supplement.patch').write_bytes(supplement)
supplement_rows = apply(supplement, lambda p: reconstructed.get(p, blob(MAIN, p)))
reconstructed.update(supplement_rows)
for p in sorted(source_paths):
    assert reconstructed.get(p, blob(MAIN, p)) == blob(FBF, p), p

patch = (ROOT / 'patches/fbf-to-grip-v4.patch').read_bytes()
assert sha(patch) == GRIP
candidate_rows = apply(patch, lambda p: reconstructed.get(p, blob(MAIN, p)))
reconstructed.update(candidate_rows)
registered = json.loads((GRIP_ROOT / 'REGISTERED-SOURCE-v4.json').read_bytes())
assert registered['base_commit'] == FBF and len(registered['files']) == 144
for row in registered['files']:
    data = reconstructed.get(row['path'], blob(MAIN, row['path']))
    assert record(row['path'], data)['sha256'] == row['sha256']
source_paths.update(row['path'] for row in registered['files'])
source_paths.update(reconstructed)

def candidate(path):
    return reconstructed.get(path, blob(MAIN, path))

# Public main-owned assets are verified but never transported in this packet.
# Includes not on selected targets are explicit, not silently substituted.
exclusions = {
    ('crates/alife_training/examples/probe_grouped_m2_durable_v1.rs',
     'docs/experiments/grouped-m2-durable-pair-v1/PROTOCOL.md'):
        'Unselected training example; no recipe or historical protocol admitted.',
    ('crates/alife_training/examples/probe_grouped_m2_durable_v1.rs',
     'crates/alife_training/examples/support/grouped_m2_qualification_v1.rs'):
        'Unselected training example; no recipe execution admitted.',
    ('crates/alife_training/examples/probe_object_rich_optional_vocal_v1.rs', 'PROTOCOL.md'):
        'Unselected training example; no historical protocol admitted.',
    ('crates/alife_training/examples/probe_object_rich_optional_vocal_v1.rs', 'RUNTIME-SOURCE.json'):
        'Unselected training example; no historical runtime data admitted.',
    ('crates/alife_training/tests/cpu_motor_host_legacy_wire.rs',
     'crates/alife_training/tests/fixtures/cpu-resident-pre-motor-v1.json.zst'):
        'Training checks only its lib and grip_options_host integration target; legacy binary-fixture test is not selected.',
    ('crates/alife_training/examples/probe_motor_mediation_native_v1.rs',
     'docs/experiments/motor-mediation-native-v1/archived-arithmetic-frames.json'):
        'Training examples are not selected; this archived-output include is not transported or admitted.'
}
pattern = r'include(?:_bytes|_str)?!\s*\(\s*(?:"([^"]+)"|concat!\s*\(\s*env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*"([^"]+)")'
includes = []
unresolved = []
for p in sorted(source_paths):
    if not p.endswith('.rs'):
        continue
    data = candidate(p)
    if data is None:
        unresolved.append({'path': p, 'reason': 'missing source'}); continue
    text = data.decode('utf-8')
    matches = list(re.finditer(pattern, text))
    assert len(matches) == len(re.findall(r'include(?:_bytes|_str)?!\s*\(', text)), p
    for match in matches:
        q = posixpath.normpath(posixpath.join(posixpath.dirname(p), match[1])) if match[1] else posixpath.normpath('/'.join(p.split('/')[:2]) + match[2])
        assert not q.startswith(('../', '/'))
        if (p, q) in exclusions:
            includes.append({'from': p, 'target': q, 'selected': False, 'reason': exclusions[p, q]})
            continue
        got = candidate(q)
        exact = blob(FBF, q)
        if q not in candidate_rows:
            assert got == exact, (p, q)
        if got is None:
            unresolved.append({'from': p, 'path': q, 'reason': 'missing include'})
            continue
        includes.append({'from': p, 'target': q, 'selected': True,
                         'provenance': 'recovered-source' if q in reconstructed else 'exact-main'})
source_paths.update(row['target'] for row in includes if row['selected'])
records = [record(p, candidate(p)) for p in sorted(source_paths)]
assert all(candidate(p) is not None for p in source_paths)

spec = importlib.util.spec_from_file_location('grip_runner', ROOT / 'run_hosted.py')
runner = importlib.util.module_from_spec(spec); spec.loader.exec_module(runner)
stages = []
for stage, package, targets, features in runner.STAGES:
    manifest = tomllib.loads(candidate(f'crates/{package}/Cargo.toml').decode())
    assert manifest['package']['name'] == package
    stages.append({'stage': stage, 'package': package, 'targets': list(targets),
                   'explicit_features': list(features), 'default_features_enabled': True,
                   'declared_default_features': manifest.get('features', {}).get('default', []),
                   'expected_root_features': runner.EXPECTED_FEATURES[package]})
# Confirm all explicitly required feature names really exist in pinned manifests.
for row in stages:
    m = tomllib.loads(candidate(f"crates/{row['package']}/Cargo.toml").decode())
    assert set(row['explicit_features']) <= set(m.get('features', {}))
assert 'crates/alife_training/tests/grip_options_host.rs' in source_paths

save('source-manifest.json', {
    'schema': 'm2-grip-complete-nominal-source-v1', 'main': MAIN, 'research_source': FBF,
    'scope': 'Exact fbf source superset plus immutable grip v4. Only selected target/feature closure is qualified; unused main files are not claimed to form full historical fbf tree.',
    'grip_patch_sha256': GRIP,
    'registered_map_sha256': sha((GRIP_ROOT / 'REGISTERED-SOURCE-v4.json').read_bytes()),
    'registered_source_closure': 'defbac1789bf87d347f2aaa7728e0ec92b6d1b69a998f8d10e0d490c8c2b02c1',
    'files': records, 'includes': includes, 'unresolved_includes': unresolved,
    'stages': stages, 'zero_execution': ['test bodies', 'recipes', 'source neural forwards', 'world commands', 'body advances'],
    'not_qualified': ['training integration targets other than grip_options_host', 'training examples', 'Bevy production frontend', 'runtime behavior', 'GPU execution']})
steps = [{'name': 'fbf-library', 'bytes': len(closure_patch), 'sha256': sha(closure_patch),
          'parts': closure['patch']['parts']},
         {'name': 'fbf-nominal-supplement', **record('patches/fbf-nominal-source-supplement.patch', supplement),
          'parts': [record('patches/fbf-nominal-source-supplement.patch', supplement)]},
         {'name': 'grip-v4', **record('patches/fbf-to-grip-v4.patch', patch),
          'parts': [record('patches/fbf-to-grip-v4.patch', patch)]}]
save('recovery.json', {'schema': 'm2-grip-recovery-chain-v1', 'main': MAIN, 'research_source': FBF,
                       'steps': steps, 'nominal_supplement_files': sorted(supplement_rows),
                       'transport': 'UTF-8 source patches only; no generated/binary/private state'})
old_workflows = git('ls-tree', '-r', '--name-only', MAIN, '.github/workflows').decode().splitlines()
assert len(old_workflows) == 15
save('publication.json', {'schema': 'm2-grip-source-publication-v1',
    'status': 'prepared locally, never published or activated',
    'branch': 'qualification/m2-grip-hosted-typecheck-v1', 'prepared_parent': MAIN,
    'packet_destination': 'docs/experiments/m2-grip-hosted-typecheck-v1',
    'workflow_destination': '.github/workflows/m2-grip-hosted-typecheck-v1.yml',
    'branch_local_deletions': [record(p, blob(MAIN, p)) for p in old_workflows],
    'required_prepared_workflows': ['.github/workflows/m2-grip-hosted-typecheck-v1.yml'],
    'prepared_head': 'PENDING_VERIFIED_IMMUTABLE_GITHUB_COMMIT',
    'remote_branch_existence': 'NOT_VERIFIED_LOCALLY',
    'publication_preflight': 'Before any write, parent must verify the dedicated remote branch does not exist and resolve any ambiguous prior create outcome. Never assume the name is unused or retry uncertain creation. Verify the exact prepared remote head and packet tree before activation.',
    'activation': 'Only after verifying published prepared head, add ACTIVATE.json as the sole changed path in one direct child commit. No rewrite/retry/marker reuse.',
    'branch_birth': 'Create the dedicated remote branch at the already-prepared commit tree with all 15 inherited workflows removed and activation absent. Never create or point it first at main, an old study, or a tree containing broad inherited push workflows.',
    'main_policy': 'No main changes; removals are branch-local. No PR or merge from this qualification branch.',
    'old_studies': 'No old activation markers, workflows, branches, fixtures, or attempts are eligible for this runner.',
    'remote_actions_performed': []})
lock = tomllib.loads(candidate('Cargo.lock').decode())
assert all(not x.get('source', '').startswith('git+') for x in lock['package'])
save('dependencies.json', {'schema': 'm2-grip-dependencies-v1',
    'cargo_lock_sha256': sha(candidate('Cargo.lock')), 'registry_sources': sorted({x['source'] for x in lock['package'] if 'source' in x}),
    'registry_packages': len([x for x in lock['package'] if 'source' in x]),
    'stages': stages, 'features_never_disabled': True, 'source_replacement': False,
    'fresh_target_and_cargo_home': True, 'engine_rlib_reuse': False,
    'binary_input_policy': 'Use only byte-identical existing main-owned assets for compiler includes; no binary assets are uploaded.',
    'metadata_additions': [dict(record(p, blob(FBF, p)), content=('Source contract hash/length receipts only; no captured bytes.' if 'legacy-wire' in p else 'Source-authored fixed M0 topology/configuration; status source_only_not_admitted_not_executed.')) for p in metadata]})
save('PREPARATION.json', {'schema': 'm2-grip-source-preparation-v1', 'source_ready': not unresolved,
    'main': MAIN, 'research_source': FBF, 'native_invocations': 0, 'network_calls': 0,
    'reconstruction_verified': True, 'inherited_closure_files_verified': len(closure['files']),
    'nominal_supplement_files': len(supplement_rows), 'nominal_supplement_bytes': len(supplement),
    'registered_grip_paths_verified': len(registered['files']), 'bound_files': len(records),
    'toolchain_status': 'Official recipe reused from independently successful V3 setup; this new grip job has not run.',
    'unresolved_includes': unresolved})
print(json.dumps({'source_ready': not unresolved, 'bound_files': len(records),
                  'supplement_bytes': len(supplement), 'supplement_files': len(supplement_rows)}))
