"""Local preparation-only resolver for immutable V5 source inputs."""
import hashlib, importlib.util, json, pathlib, sys
sys.dont_write_bytecode = True
ROOT = pathlib.Path(__file__).resolve().parent
V5 = ROOT.parent / 'm2-grip-hosted-typecheck-v5'
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
def verify_preservation():
    before=json.loads((ROOT/'PRIOR-V5-PRESERVATION.json').read_bytes())
    assert {p.relative_to(V5).as_posix() for p in V5.rglob('*') if p.is_file()} == {x['path'] for x in before['files']}
    for row in before['files']:
        b=(V5/row['path']).read_bytes();assert len(b)==row['bytes'] and sha(b)==row['sha256']
    spec=importlib.util.spec_from_file_location('old_inputs',V5/'packet_inputs.py')
    old=importlib.util.module_from_spec(spec);spec.loader.exec_module(old);old.verify_preservation()
