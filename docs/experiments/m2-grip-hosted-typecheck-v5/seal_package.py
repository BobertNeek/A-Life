#!/usr/bin/env python3
"""Seal prepared source bytes and build deterministic source-only archive locally."""
import sys
sys.dont_write_bytecode = True
import gzip
import hashlib
import io
import json
import pathlib
import tarfile
from packet_inputs import PRIOR_ROOT, read, verify_preservation

ROOT = pathlib.Path(__file__).resolve().parent
PLAN = 'm2-grip-hosted-typecheck-v5'
DEST = 'docs/experiments/' + PLAN
ARCHIVE = 'source-only-m2-grip-typecheck-v5-update.tar.gz'
EXCLUDED = {'seal.json', 'workflow.yml', 'ACTIVATE.template.json', 'ARCHIVE-MANIFEST.json', ARCHIVE}

def sha(data):
    return hashlib.sha256(data).hexdigest()

def rec(path):
    b = path.read_bytes()
    assert b'\0' not in b
    b.decode('utf-8', 'strict')
    return {'path': path.relative_to(ROOT).as_posix(), 'bytes': len(b), 'sha256': sha(b)}

def save(path, data):
    (ROOT / path).write_text(json.dumps(data, indent=2, sort_keys=True) + '\n')

files = sorted(p for p in ROOT.rglob('*') if p.is_file() and p.name not in EXCLUDED)
assert not any(p.is_symlink() or '__pycache__' in p.parts for p in files)
preparation = json.loads((ROOT / 'PREPARATION.json').read_bytes())
assert preparation['source_ready'] is True
assert json.loads((ROOT / 'STATIC-CHECK.json').read_bytes())['passed'] is True
assert json.loads((ROOT / 'RUNNER-SEMANTICS.json').read_bytes())['admit_unchanged_except_workflow_inventory'] is True
verify_preservation()
reuse = json.loads((ROOT / 'REUSE-MANIFEST.json').read_bytes())
local_records = [rec(p) for p in files]
assert not {row['path'] for row in local_records} & {row['path'] for row in reuse['files']}
reference_records = []
for row in reuse['files']:
    data = read(row['path'])
    data.decode('utf-8', 'strict')
    assert b'\0' not in data and len(data) == row['bytes'] and sha(data) == row['sha256']
    reference_records.append({k: row[k] for k in ('path', 'bytes', 'sha256')})
save('seal.json', {'schema': 'm2-grip-hosted-seal-v5', 'plan_id': PLAN, 'source_ready': True,
                  'files': sorted(local_records + reference_records, key=lambda row: row['path'])})
seal = sha((ROOT / 'seal.json').read_bytes())
source = sha((ROOT / 'source-manifest.json').read_bytes())
save('ACTIVATE.template.json', {'schema': 'm2-grip-typecheck-activation-v5', 'plan_id': PLAN,
    'main': 'c3aec16df33861f1316d127736f5f60eea327ede',
    'research_base': 'fbf2a517f93098b5d936878a3d26bbb8d0391c38',
    'prepared_head': 'PENDING_VERIFIED_IMMUTABLE_GITHUB_COMMIT',
    'seal_sha256': seal, 'source_manifest_sha256': source,
    'attempt': 1, 'typecheck_only': True})
workflow = '''name: M2 grip source typecheck v5
on:
  push:
    branches: [qualification/m2-grip-hosted-typecheck-v5]
    paths: [docs/experiments/m2-grip-hosted-typecheck-v5/ACTIVATE.json]
permissions:
  contents: read
concurrency:
  group: m2-grip-hosted-typecheck-v5
  cancel-in-progress: false
jobs:
  source-typecheck:
    if: >-
      github.repository == 'BobertNeek/A-Life' &&
      github.ref == 'refs/heads/qualification/m2-grip-hosted-typecheck-v5' &&
      github.event.created == false && github.event.deleted == false &&
      github.event.forced == false && github.run_attempt == '1'
    runs-on: ubuntu-24.04
    timeout-minutes: 90
    env:
      EXPECTED_SEAL_SHA256: __DIGEST__
      PYTHONDONTWRITEBYTECODE: '1'
    steps:
      - name: Read immutable grip source packet
        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
        with:
          ref: ${{ github.sha }}
          path: bundle
          persist-credentials: false
          fetch-depth: 2
          lfs: false
          submodules: false
      - name: Read exact accepted main
        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
        with:
          ref: c3aec16df33861f1316d127736f5f60eea327ede
          path: source
          persist-credentials: false
          fetch-depth: 1
          lfs: false
          submodules: false
      - name: Admit, restore, install pinned official Rust, and check serially
        shell: bash
        run: python3 bundle/docs/experiments/m2-grip-hosted-typecheck-v5/run_hosted.py
'''.replace('__DIGEST__', seal)
(ROOT / 'workflow.yml').write_text(workflow)
# Tar uses repository destinations; workflow is deployed only to .github/workflows.
# The sidecar hashes every member, including seal/template/workflow, without a
# recursive self-hash. No mutable activation record is ever an archive member.
reuse = json.loads((ROOT / 'REUSE-MANIFEST.json').read_bytes())
reused_names = {row['path'] for row in reuse['files']}
for row in reuse['files']:
    data = read(row['path'])
    assert len(data) == row['bytes'] and sha(data) == row['sha256']
rows = []
with (ROOT / ARCHIVE).open('wb') as target:
    with gzip.GzipFile(filename='', mode='wb', fileobj=target, mtime=0) as zipped:
        with tarfile.open(mode='w', fileobj=zipped, format=tarfile.PAX_FORMAT) as archive:
            for p in sorted(p for p in ROOT.rglob('*') if p.is_file() and p.name not in {ARCHIVE, 'ARCHIVE-MANIFEST.json'} and p.relative_to(ROOT).as_posix() not in reused_names):
                row = rec(p)
                name = '.github/workflows/' + PLAN + '.yml' if row['path'] == 'workflow.yml' else DEST + '/' + row['path']
                info = tarfile.TarInfo(name)
                info.size = row['bytes']; info.mtime = 0; info.mode = 0o644
                info.uid = info.gid = 0; info.uname = info.gname = ''
                archive.addfile(info, io.BytesIO(p.read_bytes()))
                rows.append({'archive_path': name, **row})
raw = (ROOT / ARCHIVE).read_bytes()
save('ARCHIVE-MANIFEST.json', {'schema': 'm2-grip-source-archive-v5', 'archive': ARCHIVE,
    'bytes': len(raw), 'sha256': sha(raw), 'member_count': len(rows), 'members': rows,
    'seal_sha256': seal, 'source_manifest_sha256': source,
    'source_only': True, 'activation_present': False,
    'transport': 'update-only; combine every archive member with every REUSE-MANIFEST.json blob at its exact destination',
    'reused_blob_count': len(reuse['files']), 'reused_source_bytes': sum(row['bytes'] for row in reuse['files']),
    'reuse_source_commit': reuse['verified_source_commit'],
    'source_bytes': sum(x['bytes'] for x in rows)})
assert sum(p.stat().st_size for p in ROOT.rglob('*') if p.is_file()) <= 750000
verify_preservation()
assert sum(p.stat().st_size for p in PRIOR_ROOT.rglob('*') if p.is_file()) <= 16 * 1024**2
print(json.dumps({'archive_bytes': len(raw), 'archive_sha256': sha(raw), 'members': len(rows),
                  'seal_sha256': seal, 'source_manifest_sha256': source}))
