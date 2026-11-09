"""Local preparation-only resolver for immutable V5 source inputs."""
import hashlib, importlib.util, json, pathlib, sys
sys.dont_write_bytecode = True
ROOT = pathlib.Path(__file__).resolve().parent
V5 = pathlib.Path('/workspace/shared/m2-grip-hosted-typecheck-v5')
def sha(b): return hashlib.sha256(b).hexdigest()
def read(name):
    path = pathlib.PurePosixPath(name)
    assert str(path)==name and not path.is_absolute() and '..' not in path.parts
    rows={x['path']:x for x in json.loads((ROOT/'REUSE-MANIFEST.json').read_bytes())['files']}
    own=ROOT/name
    if own.is_file(): data=own.read_bytes()
    else:
        row=rows[name]
        src=V5/row.get('local_v5_path',name)
        if src.is_file(): data=src.read_bytes()
        else:
            spec=importlib.util.spec_from_file_location('old_inputs',V5/'packet_inputs.py')
            old=importlib.util.module_from_spec(spec);spec.loader.exec_module(old)
            data=old.read(name)
    if name in rows:
        row=rows[name]
        assert len(data)==row['bytes'] and sha(data)==row['sha256']
        assert hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()==row['git_blob_sha1']
    return data
def verify_prior_inputs(evidence):
    base = pathlib.Path('/workspace/shared')
    prior = base / 'm2-grip-fixed-fixtures-v1'
    v5 = base / 'm2-grip-hosted-typecheck-v5'
    history = base / 'm2-grip-options-v1'
    expected_cache = {
        'directory': 'hosted-typecheck-v2',
        'path': '__pycache__/source_patch.cpython-312.pyc',
        'bytes': 3763,
        'sha256': '3560c0b9e0469dd7cc8c4c51710056044e1e7e420561dc742506cb855de1e0fd'}
    assert evidence['non_source_cache_exception'] == expected_cache

    def verify_one(root, row):
        rel = pathlib.PurePosixPath(row['path'])
        assert str(rel) == row['path'] and not rel.is_absolute() and '..' not in rel.parts
        p = root.joinpath(*rel.parts)
        assert not root.is_symlink() and p.is_file() and not p.is_symlink()
        for parent in p.parents:
            if parent == root:
                break
            assert not parent.is_symlink()
        assert p.resolve().is_relative_to(root.resolve())
        b = p.read_bytes()
        assert len(b) == row['bytes'] and sha(b) == row['sha256']

    def verify_directory(root, rows, extras=()):
        assert root.is_dir() and not root.is_symlink()
        actual = set()
        for p in root.rglob('*'):
            assert not p.is_symlink()
            if p.is_file():
                actual.add(p.relative_to(root).as_posix())
        names = [row['path'] for row in rows] + [row['path'] for row in extras]
        assert len(names) == len(set(names)) and actual == set(names)
        for row in list(rows) + list(extras):
            verify_one(root, row)

    verify_directory(prior, evidence['files'])
    v5_rows = json.loads((prior/'PRIOR-V5-PRESERVATION.json').read_bytes())['files']
    verify_directory(v5, v5_rows)
    old = json.loads((v5/'PRIOR-PRESERVATION.json').read_bytes())
    assert [v['directory'] for v in old['versions']] == [
        'hosted-typecheck-v1','hosted-typecheck-v2',
        'hosted-typecheck-v3','hosted-typecheck-v4']
    assert sum(len(v['files']) for v in old['versions']) == 113
    for version in old['versions']:
        extra = [expected_cache] if version['directory'] == expected_cache['directory'] else []
        verify_directory(history/version['directory'], version['files'], extra)
    assert len(old['source_files']) == 11
    for row in old['source_files']:
        verify_one(history, row)

def verify_preservation():
    evidence = json.loads((ROOT/'PRIOR-V1-PRESERVATION.json').read_bytes())
    verify_prior_inputs(evidence)
    prior = pathlib.Path('/workspace/shared/m2-grip-fixed-fixtures-v2')
    rows = json.loads((ROOT/'PRIOR-V2-PRESERVATION.json').read_bytes())['files']
    actual = set()
    for p in prior.rglob('*'):
        assert not p.is_symlink()
        if p.is_file():
            actual.add(p.relative_to(prior).as_posix())
    assert actual == {row['path'] for row in rows}
    for row in rows:
        p = prior/row['path']
        assert p.is_file() and not p.is_symlink()
        b = p.read_bytes()
        assert len(b) == row['bytes'] and sha(b) == row['sha256']

_verify_through_v2 = verify_preservation
def verify_preservation():
    _verify_through_v2()
    prior = pathlib.Path('/tmp/m2-grip-fixed-fixtures-v3')
    rows = json.loads((ROOT/'PRIOR-V3-PRESERVATION.json').read_bytes())['files']
    actual = set()
    for p in prior.rglob('*'):
        assert not p.is_symlink()
        if p.is_file():
            actual.add(p.relative_to(prior).as_posix())
    assert actual == {row['path'] for row in rows}
    for row in rows:
        p = prior/row['path'];b = p.read_bytes()
        assert len(b) == row['bytes'] and sha(b) == row['sha256']

_verify_through_v3 = verify_preservation
def verify_preservation():
    _verify_through_v3()
    prior = pathlib.Path('/tmp/m2-grip-fixed-fixtures-v4')
    rows = json.loads((ROOT/'PRIOR-V4-PRESERVATION.json').read_bytes())['files']
    actual = set()
    for p in prior.rglob('*'):
        assert not p.is_symlink()
        if p.is_file(): actual.add(p.relative_to(prior).as_posix())
    assert actual == {row['path'] for row in rows}
    for row in rows:
        b=(prior/row['path']).read_bytes()
        assert len(b)==row['bytes'] and sha(b)==row['sha256']
