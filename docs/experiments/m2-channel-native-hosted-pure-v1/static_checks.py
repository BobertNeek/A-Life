"""Reproducible source/protocol checks. Child processes are prohibited."""
import json,pathlib,importlib.util,subprocess,hashlib,copy,ast
ROOT=pathlib.Path(__file__).resolve().parent
subprocess.Popen=lambda *a,**k:(_ for _ in()).throw(AssertionError('No subprocess allowed'))
spec=importlib.util.spec_from_file_location('runner',ROOT/'run_hosted.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
passed=[]
def eq(a,b):assert a==b
def check(n,f):f();passed.append(n)
def reject(f):
 try:f()
 except m.Stop:return
 raise AssertionError('Expected rejection')
def listing(names):return ('\n'.join(n+': test'for n in sorted(names))+'\n\n'+str(len(names))+' tests, 0 benchmarks\n').encode()
check('eight-exact',lambda:m.test_listing(listing(m.TEST_NAMES),m.TEST_NAMES))
check('thirteen-exact',lambda:m.test_listing(listing(m.ALL_TEST_NAMES),m.ALL_TEST_NAMES))
check('extra-rejected',lambda:reject(lambda:m.test_listing(listing(m.TEST_NAMES+['extra']),m.TEST_NAMES)))
check('missing-rejected',lambda:reject(lambda:m.test_listing(listing(m.TEST_NAMES[:-1]),m.TEST_NAMES)))
check('duplicate-rejected',lambda:reject(lambda:m.test_listing(listing(m.TEST_NAMES+[m.TEST_NAMES[0]]),m.TEST_NAMES)))
raw=('\nrunning 1 test\ntest '+m.TEST_NAMES[0]+' ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 1.00s\n\n').encode()
check('success-exact',lambda:m.test_report(raw,m.TEST_NAMES[0]))
check('ignored-rejected',lambda:reject(lambda:m.test_report(raw.replace(b'0 ignored',b'1 ignored'),m.TEST_NAMES[0])))
check('private-tail-rejected',lambda:reject(lambda:m.test_report(raw+b'private assertion',m.TEST_NAMES[0])))
check('wrong-name-rejected',lambda:reject(lambda:m.test_report(raw,m.TEST_NAMES[1])))
check('pipe-cap-rejected',lambda:reject(lambda:m.test_report(b'x'*65537,m.TEST_NAMES[0])))
manifest=json.loads((ROOT/'source-manifest.json').read_text())
source=m.capsule_source((ROOT/'v4-source-capsule.tar.gz').read_bytes(),manifest)
overlay=m.overlay_members((ROOT/'sampler-source-overlay.tar.gz').read_bytes(),{x['path']:x for x in manifest['sampler_overlay_files']})
check('capsule103',lambda:eq(len(source),103))
check('overlay10',lambda:eq(len(overlay),10))
check('companion-byte-identity',lambda:eq(all(source[x['original']]==overlay[x['companion']]for x in manifest['companion_byte_identity']),True))
check('noignored-source',lambda:eq(any('#[ignore'in b.decode()for n,b in overlay.items()if n.endswith('.rs')),False))
old={x['path']:x for x in manifest['base_files']};old.update({x['path']:x for x in manifest['qualified_overlay_files']});old.update({x['path']:x for x in manifest['capsule_source_files']});old.update({x['path']:x for x in manifest['sampler_overlay_files']})
check('final822exact',lambda:eq(sorted(old.values(),key=lambda x:x['path']),manifest['sampler_files']))
head='a'*40;before='b'*40;env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':m.BRANCH,'GITHUB_RUN_ATTEMPT':'1'}
event={'before':before,'after':head,'ref':m.BRANCH,'created':False,'deleted':False,'forced':False}
mark={'schema':'m2-channel-native-pure-activation-v1','plan_id':m.PLAN,'main':m.MAIN,'research_base':m.BASE,'prepared_head':before,'seal_sha256':'s','source_manifest_sha256':'m','attempt':1,'fixed_disposable_qualification_only':True}
a=(env,event,head,[head,before],'A\t'+m.MARKER+'\n',mark,'s','m')
check('sole-marker-valid',lambda:m.validate_activation(*a))
check('branch-create-rejected',lambda:reject(lambda:m.validate_activation(env,{**event,'created':True},*a[2:])))
check('attempt2-rejected',lambda:reject(lambda:m.validate_activation({**env,'GITHUB_RUN_ATTEMPT':'2'},*a[1:])))
check('forced-rejected',lambda:reject(lambda:m.validate_activation(env,{**event,'forced':True},*a[2:])))
check('extra-marker-path-rejected',lambda:reject(lambda:m.validate_activation(*a[:4],a[4]+'A\tother\n',*a[5:])))
bound={'crates/alife_channel_native_qualification/src/lib.rs':{}}
obj={'message':{'level':'error','message':'PRIVATE','code':{'code':'E0583'},'spans':[{'is_primary':True,'file_name':next(iter(bound)),'line_start':1}]}}
check('error-projection-exact',lambda:eq(m.diagnostic_rows(obj,'sampler-test-build',pathlib.Path('/fixture'),bound),[{'code':'E0583','path':next(iter(bound)),'line':1,'stage':'sampler-test-build'}]))
check('outside-error-removed',lambda:eq(m.diagnostic_rows(obj,'sampler-test-build',pathlib.Path('/fixture'),{}),[]))
fixtures=json.loads((ROOT/'INHERITED-WORKFLOW-FIXTURES.json').read_text())['files'];wf=json.loads((ROOT/'WORKFLOW-PRESERVATION.json').read_text())['files']
check('fifteen-inherited-byte-mode-exact',lambda:eq([{k:x[k]for k in ('path','bytes','sha256','git_blob_sha1','mode')}for x in fixtures],wf))
for x in fixtures:
 b=x['content_utf8'].encode();assert len(b)==x['bytes'] and hashlib.sha256(b).hexdigest()==x['sha256'];assert hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()==x['git_blob_sha1']
oldrows=['100644 blob '+x['git_blob_sha1']+'\t'+x['path']for x in fixtures]
newrows=oldrows+['100644 blob '+'c'*40+'\t'+m.PACKET+'/README.md']
check('additions-only-valid',lambda:m.validate_tree_preservation(oldrows,newrows))
check('inherited-mutation-rejected',lambda:reject(lambda:m.validate_tree_preservation(oldrows,['100644 blob '+'d'*40+'\t'+fixtures[0]['path']]+newrows[1:])))
check('prepared-marker-rejected',lambda:reject(lambda:m.validate_tree_preservation(oldrows,newrows+['100644 blob '+'e'*40+'\t'+m.MARKER])))
text=(ROOT/'run_hosted.py').read_text();assert "'-p','alife_training'"not in text and "'-p','alife_channel_native_qualification'"in text and "cmd+=['--no-run']"in text
check('cargo-companion-only',lambda:eq(True,True))
print(json.dumps({'schema':'source-protocol-static-v2','count':len(passed),'passed':passed,'subprocesses':0,'native_invocations':0,'world_calls':0,'cargo_compatibility':'not executed'},sort_keys=True))
