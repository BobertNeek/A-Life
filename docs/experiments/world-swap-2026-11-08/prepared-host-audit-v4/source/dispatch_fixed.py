"""No-study fixed-test dispatcher. Binds source/binary, caps and discards native output."""
from pathlib import Path
import hashlib,json,os,re,resource,selectors,signal,subprocess,sys,time
W=Path(__file__).resolve().parent.parent;D=Path(__file__).resolve().parent;P=W/'world-swap-diagnostics-prepared-v4'
sha=lambda raw:hashlib.sha256(raw).hexdigest()
variant,mode=sys.argv[1:];assert variant in ('old','new') and mode in ('zero_life','prepared')
resource.setrlimit(resource.RLIMIT_CORE,(0,0));os.umask(0o077)
rpath=D/(variant+'-compile.binding.json');r=json.loads(rpath.read_bytes());root=W/('world-swap-diagnostics-source-'+variant+'-v4')
git=lambda *a:subprocess.check_output(['git','-C',str(root),*a],timeout=60)
assert r['test_binary'] and r['fresh_artifact'];assert r['variant']==variant
assert sha((P/'PACKAGE-MANIFEST.json').read_bytes())==r['package_manifest_sha256']
manifest=json.loads((P/'PACKAGE-MANIFEST.json').read_bytes());assert all(sha((P/n).read_bytes())==h for n,h in manifest.items())
assert sha((P/'SOURCE-BINDING.json').read_bytes())==r['source_proof_sha256']
proof=json.loads((P/'SOURCE-BINDING.json').read_bytes());assert r['source_tree']==proof[variant+'_reconstructed_tree_oid_unwritten']==git('rev-parse','HEAD^{tree}').decode().strip()
assert not git('status','--porcelain').strip()
for row in git('ls-files','--stage','-z').split(b'\0'):
 if not row:continue
 meta,name=row.split(b'\t');m,oid,stage=meta.decode().split();f=root/name.decode();assert stage=='0'
 if not f.exists() and not f.is_symlink():assert name.startswith((b'assets/',b'crates/alife_game_app/assets/',b'crates/alife_world/assets/'));continue
 raw=os.readlink(f).encode() if m=='120000' else f.read_bytes();assert hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==oid
exe=Path(r['executable']);assert sha(exe.read_bytes())==r['executable_sha256']
log=D/(variant+'-compile.jsonl');assert sha(log.read_bytes())==r['cargo_compile_json_sha256']
import types
def source_module(name):
 path=P/(name+'.py');raw=path.read_bytes();assert sha(raw)==manifest[name+'.py']
 module=types.ModuleType(name);module.__file__=str(path);sys.modules[name]=module
 exec(compile(raw,str(path),'exec'),module.__dict__);return module
c=source_module('report_contract')
f=source_module('prepared_fixture_contract')
def validate_durable_progress(progress):
 c.validate(progress,f.PROGRESS)
 for name in f.CAPS:
  count=progress[name];c.need(count['returned']<=count['requested']<=count['returned']+1,'accounting')
 p=progress;n=p['rows_collected']
 c.need(n<=p['projection_states_unchanged']<=p['projection']['returned']<=p['projection']['requested']<=p['paired_states_equal']<=p['audited_host']['returned']<=p['audited_host']['requested']<=p['ordinary_host']['returned']<=p['ordinary_host']['requested']<=min(4,n+1),'accounting')
 c.need(p['replay']['requested']<=p['audited_host']['returned'],'accounting')
 if p['fresh_construction']['requested']:c.need(p['asset_construction']['returned']==1,'accounting')
 if p['fresh_construction']['returned']:c.need(p['preparation']['requested']==p['preparation']['returned']==600,'accounting')
 if p['restore']['requested']:c.need(p['fresh_construction']['returned']==1 and p['preparation']['returned']==600,'accounting')
 if p['preparation']['requested']:c.need(p['fresh_construction']['requested']==1 and p['asset_construction']['returned']==1,'accounting')
 if p['ordinary_host']['requested']:c.need(p['restore']['returned']==p['fresh_construction']['returned']==p['asset_construction']['returned']==1 and p['preparation']['returned']==p['preparation']['requested']==600,'accounting')
 if p['stage']=='finished':c.need(n==4,'accounting')
 return progress
command=[str(exe),'--test-threads=1'];expected=15
if mode=='prepared':
 command+=['native_qualification::prepared_audit_does_not_change_fixed_host_transitions','--exact','--ignored'];expected=1
source=dict(source_tree=r['source_tree'],source_closure_sha256=proof[variant+'_tree_manifest_sha256'],executable_sha256=r['executable_sha256'],plan_sha256=sha((P/'QUALIFICATION-PLAN.json').read_bytes()),common_patch_sha256=sha((P/'common-audit-kernel.patch').read_bytes()),world_patch_sha256=sha((P/'world-factor.patch').read_bytes()))
fixture=D/('prepared-'+variant)
assert not fixture.exists()
def save(path,v):
 with path.open('xb') as handle:handle.write(c.canonical(v));handle.flush();os.fsync(handle.fileno())
attempt=dict(schema_version=1,scope='bounded_fixed_qualification_not_study',world=variant+'_world',mode=mode,source=source,compile_binding_sha256=sha(rpath.read_bytes()),command=command,fixture_seed=f.SEED if mode=='prepared' else None,preparation_ticks_max=600 if mode=='prepared' else 0,host_steps_max=8 if mode=='prepared' else 0,replays_max=4 if mode=='prepared' else 0,core_limit=[0,0],private_umask='077',output_limit=65536,timeout_seconds=300)
save(D/(variant+'-'+mode+'.attempted.json'),attempt)
env=os.environ.copy();env['RUST_BACKTRACE']='0'
if mode=='prepared':
 fixture.mkdir(mode=0o700);save(fixture/'binding.json',dict(world=variant+'_world',source=source));env['WORLD_SWAP_PREPARED_FIXTURE_ROOT']=str(fixture)
p=None;data=bytearray();gate='launcher';code=None;admitted=None;progress=None
try:
 p=subprocess.Popen(command,cwd=root,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
 save(D/(variant+'-'+mode+'.started.json'),dict(native_started=True))
 selector=selectors.DefaultSelector()
 for stream in (p.stdout,p.stderr):os.set_blocking(stream.fileno(),False);selector.register(stream,selectors.EVENT_READ)
 deadline=time.monotonic()+300;gate='running'
 while selector.get_map():
  if time.monotonic()>deadline:gate='timeout';break
  for key,_ in selector.select(.05):
   chunk=os.read(key.fileobj.fileno(),min(65537-len(data),4096))
   if not chunk:selector.unregister(key.fileobj);key.fileobj.close();continue
   data.extend(chunk)
   if len(data)>65536:gate='output_bound';break
  if gate=='output_bound':break
 else:
  code=p.wait(timeout=max(.01,deadline-time.monotonic()))
  exact=bool(re.search(rb'test result: ok\. '+str(expected).encode()+rb' passed; 0 failed;',data))
  gate='passed' if code==0 and exact else 'test_failed' if code else 'test_count'
except Exception:
 gate='launcher_error'
finally:
 if p is not None:
  try:os.killpg(p.pid,signal.SIGKILL)
  except ProcessLookupError:pass
  try:code=p.wait(timeout=5)
  except subprocess.TimeoutExpired:gate='cleanup_uncertain'
 if mode=='prepared':
  try:
   path=fixture/'receipt.json'
   if path.exists():
    c.need(path.is_file() and not path.is_symlink() and path.stat().st_size<=256*1024,'output_bound')
    admitted=f.validate_prepared_fixture(path.read_bytes(),variant+'_world',source)
    if admitted['outcome']!='fixture_passed':gate='fixture_rejected'
    progress=admitted['progress']
   else:
    if gate=='passed':gate='missing_receipt'
    path=fixture/'progress.json'
    if path.exists():
     c.need(path.is_file() and not path.is_symlink() and path.stat().st_size<=16*1024,'output_bound')
     partial=c.parse(path.read_bytes(),c.obj(progress=f.PROGRESS,evidence=f.EVIDENCE))
     progress=validate_durable_progress(partial['progress'])
  except c.Reject as error:gate='scalar_rejected';admitted=None;progress=None
  except Exception:gate='scalar_unknown';admitted=None;progress=None
 result=dict(schema_version=1,scope='fixed_qualification_not_study',world=variant+'_world',mode=mode,gate=gate,exit_code=code,output_bytes=min(len(data),65537),raw_output_retained=False,tests_passed=expected if gate=='passed' else None,source=source,progress=progress,progress_origin='validated_native_receipt' if admitted else 'durable_partial_snapshot' if progress else None,native_receipt_admitted=admitted is not None,fixture_outcome=admitted['outcome'] if admitted else None,finite_code=admitted['code'] if admitted else None,evidence=admitted['evidence'] if admitted else None,manipulation_coverage=admitted['manipulation_coverage'] if admitted else None,completed_study_cases=0)
 save(D/(variant+'-'+mode+'.returned.json'),result);print(json.dumps(result,sort_keys=True));data.clear()
raise SystemExit(0 if gate=='passed' else 2)
