"""Preparation-only immutable resolver; never imported by the hosted runner.

The locally update-only packet deliberately omits large unchanged inputs.
Published v5 must materialize every REUSE-MANIFEST entry at its v5 path.
"""
import sys
sys.dont_write_bytecode = True
import hashlib
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent
PRIOR_ROOT = ROOT.parent / 'm2-grip-options-v1'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def verify(data, record):
    assert len(data) == record['bytes'] and sha(data) == record['sha256']
    return data


def safe(name):
    path = pathlib.PurePosixPath(name)
    assert not path.is_absolute() and '..' not in path.parts and str(path) == name
    return name


def prior(version, name):
    evidence = json.loads((ROOT / 'PRIOR-PRESERVATION.json').read_bytes())
    directory = f'hosted-typecheck-v{version}'
    rows = next(x['files'] for x in evidence['versions'] if x['directory'] == directory)
    row = next(x for x in rows if x['path'] == safe(name))
    path = PRIOR_ROOT / directory / name
    assert path.is_file() and not path.is_symlink()
    return verify(path.read_bytes(), row)


def resolve_prior_path(name):
    # V3 itself is update-only. Its persisted reuse contract binds missing
    # inputs back to V2, whose preserved files are materialized locally.
    v3 = PRIOR_ROOT / 'hosted-typecheck-v3'
    direct = v3 / safe(name)
    if direct.is_file():
        prior(3, name)
        return direct
    reuse = json.loads(prior(3, 'REUSE-MANIFEST.json'))
    row = next(row for row in reuse['files'] if row['path'] == name)
    data = prior(2, name)
    verify(data, row)
    return PRIOR_ROOT / 'hosted-typecheck-v2' / name


def resolve_prior_input(name):
    return resolve_prior_path(name).read_bytes()


def input_path(name):
    safe(name)
    local = ROOT / name
    reuse = json.loads((ROOT / 'REUSE-MANIFEST.json').read_bytes())
    rows = {row['path']: row for row in reuse['files']}
    if local.is_file():
        assert not local.is_symlink()
        if name in rows:
            verify(local.read_bytes(), rows[name])
        return local
    assert name in rows
    assert reuse['verified_source_commit'] == '90d63813e20554fabe5ffe95f92ffad3229b8f9a'
    data = resolve_prior_input(name)
    verify(data, rows[name])
    assert hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest() == rows[name]['git_blob_sha1']
    return resolve_prior_path(name)


def read(name):
    return input_path(name).read_bytes()


def verify_preservation():
    evidence = json.loads((ROOT / 'PRIOR-PRESERVATION.json').read_bytes())
    for version in evidence['versions']:
        directory = PRIOR_ROOT / version['directory']
        assert {p.relative_to(directory).as_posix() for p in directory.rglob('*') if p.is_file()} == {row['path'] for row in version['files']}
        for row in version['files']:
            verify((PRIOR_ROOT / version['directory'] / row['path']).read_bytes(), row)
    for row in evidence['source_files']:
        verify((PRIOR_ROOT / row['path']).read_bytes(), row)
