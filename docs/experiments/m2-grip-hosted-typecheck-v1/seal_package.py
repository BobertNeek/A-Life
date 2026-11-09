#!/usr/bin/env python3
"""Seal prepared source bytes and build deterministic source-only archive locally."""
import gzip
import hashlib
import io
import json
import pathlib
import tarfile

ROOT = pathlib.Path(__file__).resolve().parent
PLAN = 'm2-grip-hosted-typecheck-v1'
DEST = 'docs/experiments/' + PLAN
ARCHIVE = 'source-only-m2-grip-typecheck-v1.tar.gz'
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
save('seal.json', {'schema': 'm2-grip-hosted-seal-v1', 'plan_id': PLAN, 'source_ready': True,
                  'files': [rec(p) for p in files]})
seal = sha((ROOT / 'seal.json').read_bytes())
source = sha((ROOT / 'source-manifest.json').read_bytes())
save('ACTIVATE.template.json', {'schema': 'm2-grip-typecheck-activation-v1', 'plan_id': PLAN,
    'main': 'c3aec16df33861f1316d127736f5f60eea327ede',
    'research_base': 'fbf2a517f93098b5d936878a3d26bbb8d0391c38',
    'prepared_head': 'PENDING_VERIFIED_IMMUTABLE_GITHUB_COMMIT',
    'seal_sha256': seal, 'source_manifest_sha256': source,
    'attempt': 1, 'typecheck_only': True})
workflow = '''name: M2 grip source typecheck v1
on:
  push:
    branches: [qualification/m2-grip-hosted-typecheck-v1]
    paths: [docs/experiments/m2-grip-hosted-typecheck-v1/ACTIVATE.json]
permissions:
  contents: read
concurrency:
  group: m2-grip-hosted-typecheck-v1
  cancel-in-progress: false
jobs:
  source-typecheck:
    if: >-
      github.repository == 'BobertNeek/A-Life' &&
      github.ref == 'refs/heads/qualification/m2-grip-hosted-typecheck-v1' &&
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
        run: python3 bundle/docs/experiments/m2-grip-hosted-typecheck-v1/run_hosted.py
'''.replace('__DIGEST__', seal)
(ROOT / 'workflow.yml').write_text(workflow)
# Tar uses repository destinations; workflow is deployed only to .github/workflows.
# The sidecar hashes every member, including seal/template/workflow, without a
# recursive self-hash. No mutable activation record is ever an archive member.
rows = []
with (ROOT / ARCHIVE).open('wb') as target:
    with gzip.GzipFile(filename='', mode='wb', fileobj=target, mtime=0) as zipped:
        with tarfile.open(mode='w', fileobj=zipped, format=tarfile.PAX_FORMAT) as archive:
            for p in sorted(p for p in ROOT.rglob('*') if p.is_file() and p.name not in {ARCHIVE, 'ARCHIVE-MANIFEST.json'}):
                row = rec(p)
                name = '.github/workflows/' + PLAN + '.yml' if row['path'] == 'workflow.yml' else DEST + '/' + row['path']
                info = tarfile.TarInfo(name)
                info.size = row['bytes']; info.mtime = 0; info.mode = 0o644
                info.uid = info.gid = 0; info.uname = info.gname = ''
                archive.addfile(info, io.BytesIO(p.read_bytes()))
                rows.append({'archive_path': name, **row})
raw = (ROOT / ARCHIVE).read_bytes()
save('ARCHIVE-MANIFEST.json', {'schema': 'm2-grip-source-archive-v1', 'archive': ARCHIVE,
    'bytes': len(raw), 'sha256': sha(raw), 'member_count': len(rows), 'members': rows,
    'seal_sha256': seal, 'source_manifest_sha256': source,
    'source_only': True, 'activation_present': False,
    'source_bytes': sum(x['bytes'] for x in rows)})
assert sum(p.stat().st_size for p in ROOT.rglob('*') if p.is_file()) <= 5 * 1024**2
print(json.dumps({'archive_bytes': len(raw), 'archive_sha256': sha(raw), 'members': len(rows),
                  'seal_sha256': seal, 'source_manifest_sha256': source}))
