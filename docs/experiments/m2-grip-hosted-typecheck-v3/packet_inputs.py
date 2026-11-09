"""Preparation-only immutable resolver; never imported by the hosted runner.

The locally update-only packet deliberately omits large unchanged inputs.
Published v3 must materialize every REUSE-MANIFEST entry at its v3 path.
"""
import hashlib
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent


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
    path = ROOT.parent / directory / name
    assert path.is_file() and not path.is_symlink()
    return verify(path.read_bytes(), row)


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
    assert reuse['verified_source_commit'] == 'f5ce4d6de7ddf2e04473befefc715cc48bb28f26'
    data = prior(2, name)
    verify(data, rows[name])
    assert hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest() == rows[name]['git_blob_sha1']
    return ROOT.parent / 'hosted-typecheck-v2' / name


def read(name):
    return input_path(name).read_bytes()


def verify_preservation():
    evidence = json.loads((ROOT / 'PRIOR-PRESERVATION.json').read_bytes())
    for version in evidence['versions']:
        for row in version['files']:
            verify((ROOT.parent / version['directory'] / row['path']).read_bytes(), row)
    for row in evidence['source_files']:
        verify((ROOT.parent / row['path']).read_bytes(), row)
