"""Bind a local Cargo compile receipt to exact installed source and executable bytes.
No executable is run by this file. This is private qualification orchestration.
"""
from pathlib import Path
import hashlib,json,os,subprocess,sys
variant,log_name=sys.argv[1:];workspace=Path(__file__).resolve().parent.parent
root=workspace/('world-swap-diagnostics-source-'+variant+'-v4');package=Path(os.environ.get('WORLD_SWAP_CONTROLS_PACKAGE',str(workspace/'world-swap-diagnostics-prepared-v4')));log=Path(log_name).resolve()
sha=lambda raw:hashlib.sha256(raw).hexdigest()
git=lambda *args:subprocess.check_output(['git','-C',str(root),*args],timeout=60)
proof_raw=(package/'SOURCE-BINDING.json').read_bytes();proof=json.loads(proof_raw);manifest_raw=(package/'PACKAGE-MANIFEST.json').read_bytes();manifest=json.loads(manifest_raw)
assert all(sha((package/name).read_bytes())==digest for name,digest in manifest.items())
tree=git('write-tree').decode().strip();assert tree==proof[variant+'_reconstructed_tree_oid_unwritten']
assert not git('diff','--name-only').strip();assert not git('ls-files','--others','--exclude-standard').strip()
verified=0;absent=0
for row in git('ls-files','--stage','-z').split(b'\0'):
 if not row:continue
 info,name=row.split(b'\t');mode,oid,stage=info.decode().split();path=root/name.decode();assert stage=='0'
 if not path.exists() and not path.is_symlink():
  assert name.startswith((b'assets/',b'crates/alife_game_app/assets/',b'crates/alife_world/assets/'));absent+=1;continue
 raw=os.readlink(path).encode() if mode=='120000' else path.read_bytes()
 assert hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==oid,(name,'changed source')
 verified+=1
messages=[json.loads(row) for row in log.read_text().splitlines() if row.startswith('{')];assert messages[-1]['reason']=='build-finished' and messages[-1]['success']is True
artifacts=[v for v in messages if v.get('reason')=='compiler-artifact' and v.get('executable') and v['target']['name']=='probe_world_swap'];assert len(artifacts)==1
artifact=artifacts[0];assert artifact['manifest_path']==str(root/'crates/alife_training/Cargo.toml') and artifact['target']['src_path']==str(root/'crates/alife_training/examples/probe_world_swap.rs')
exe=Path(artifact['executable']);assert exe.is_file() and not exe.is_symlink()
result=dict(schema_version=1,variant=variant,source_tree=tree,source_proof_sha256=sha(proof_raw),package_manifest_sha256=sha(manifest_raw),cargo_compile_json_sha256=sha(log.read_bytes()),cargo_manifest_path=artifact['manifest_path'],executable=str(exe),executable_sha256=sha(exe.read_bytes()),test_binary=artifact['profile']['test'],materialized_tracked_files_verified=verified,absent_sparse_assets=absent,compiler_profile=artifact['profile'],toolchain='rustc 1.99.1 (58cf8f9c3 2026-10-13)',fresh_artifact=not artifact['fresh'])
receipt=log.with_suffix('.binding.json');raw=(json.dumps(result,sort_keys=True,separators=(',',':'))+'\n').encode()
with receipt.open('xb') as f:f.write(raw);f.flush();os.fsync(f.fileno())
print(json.dumps(result,sort_keys=True))
