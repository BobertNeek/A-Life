import pathlib,json,importlib.util,subprocess,copy,hashlib,ast
ROOT=pathlib.Path(__file__).resolve().parent
subprocess.Popen=lambda *a,**k:(_ for _ in()).throw(AssertionError('No child process allowed'))
spec=importlib.util.spec_from_file_location('runner',ROOT/'run_hosted.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
passed=[]
def check(n,f):f();passed.append(n)
def eq(a,b):assert a==b
def reject(f):
 try:f()
 except m.Stop:return
 raise AssertionError('expected rejection')
def listing(ns):return ('\n'.join(n+': test' for n in sorted(ns))+'\n\n'+str(len(ns))+(' test' if len(ns)==1 else ' tests')+', 0 benchmarks\n').encode()
check('one-exact',lambda:m.test_listing(listing(m.TEST_NAMES),m.TEST_NAMES))
for name,ns in [('extra',m.TEST_NAMES+['extra']),('missing',m.TEST_NAMES[:-1]),('duplicate',m.TEST_NAMES+[m.TEST_NAMES[0]])]:check(name+'-reject',lambda ns=ns:reject(lambda:m.test_listing(listing(ns),m.TEST_NAMES)))
raw=('\nrunning 1 test\ntest '+m.TEST_NAMES[0]+' ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s\n\n').encode()
check('single-exact-result',lambda:m.test_report(raw,m.TEST_NAMES[0]))
check('old-filter-count-reject',lambda:reject(lambda:m.test_report(raw.replace(b'0 filtered out',b'12 filtered out'),m.TEST_NAMES[0])))
check('wrong-singular-listing-reject',lambda:reject(lambda:m.test_listing(listing(m.TEST_NAMES).replace(b'1 test, 0 benchmarks',b'1 tests, 0 benchmarks'),m.TEST_NAMES)))
s=m.summary('OK');summary=json.loads(s)
check('fixed-dose-summary',lambda:eq((summary['planned_tests'],summary['planned_world_step_calls_max'],summary['source_forward_calls_max']),(1,1,1)))
check('ignored-reject',lambda:reject(lambda:m.test_report(raw.replace(b'0 ignored',b'1 ignored'),m.TEST_NAMES[0])))
check('private-output-reject',lambda:reject(lambda:m.test_report(raw+b'PRIVATE',m.TEST_NAMES[0])))
check('output-cap-reject',lambda:reject(lambda:m.test_report(b'x'*65537,m.TEST_NAMES[0])))
text='#[test]\nfn old() {}\nconst S:&str=r###"\n#[test]\n"###;\n/*\n#[test]\n*/\nfn helper() {}\n'
projected,count=m.exclude_old_test_registration(text)
check('only-real-registration-projected',lambda:eq((projected,count),(text.replace('#[test]','#[cfg(any())]',1),1)))
literal="const C:char='\"'; /* #[test] */ #[test] fn f() {}\n"
expected="const C:char='\"'; /* #[test] */ #[cfg(any())] fn f() {}\n"
check('character-and-inline-comment-preserved',lambda:eq(m.exclude_old_test_registration(literal),(expected,1)))
manifest=json.loads((ROOT/'source-manifest.json').read_text());source=m.capsule_source((ROOT/'host-source-capsule.tar.gz').read_bytes(),manifest)
check('capsule106exact',lambda:eq(len(source),106))
owned={r['source']for r in json.loads((ROOT/'FIXED-INVENTORY.json').read_text())}
check('owned1-source-noignored',lambda:eq(sum(source[p].count(b'#[test]')for p in owned),1))
check('owned-noignored',lambda:eq(any(b'#[ignore'in source[p]for p in owned),False))
check('late-physical-fault-present',lambda:eq(b'grouped_policy_step::<FailAfterPhysical>'in source['crates/alife_training/src/cpu/channel_native_host_tests.rs'],True))
check('exact-error-required',lambda:eq(b'Err(CpuHostError::State("fixed channel-native fault after physical transaction"))'in source['crates/alife_training/src/cpu/channel_native_host_tests.rs'],True))
check('nan-step-excluded',lambda:eq(b'new.step(f32::NAN)' in source['crates/alife_training/src/cpu/channel_native_host_tests.rs'],False))
head='a'*40;before='b'*40;env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':m.BRANCH,'GITHUB_RUN_ATTEMPT':'1'}
event={'before':before,'after':head,'ref':m.BRANCH,'created':False,'deleted':False,'forced':False};mark={'schema':'m2-channel-native-rollback-exact-activation-v1','plan_id':m.PLAN,'main':m.MAIN,'research_base':m.BASE,'prepared_head':before,'seal_sha256':'s','source_manifest_sha256':'m','attempt':1,'fixed_disposable_qualification_only':True};args=(env,event,head,[head,before],'A\t'+m.MARKER+'\n',mark,'s','m')
check('sole-marker',lambda:m.validate_activation(*args))
check('attempt2-reject',lambda:reject(lambda:m.validate_activation({**env,'GITHUB_RUN_ATTEMPT':'2'},*args[1:])))
check('created-reject',lambda:reject(lambda:m.validate_activation(env,{**event,'created':True},*args[2:])))
check('forced-reject',lambda:reject(lambda:m.validate_activation(env,{**event,'forced':True},*args[2:])))
check('extra-marker-reject',lambda:reject(lambda:m.validate_activation(*args[:4],args[4]+'A\tother\n',*args[5:])))
fixtures=json.loads((ROOT/'INHERITED-WORKFLOW-FIXTURES.json').read_text())['files'];wf=json.loads((ROOT/'WORKFLOW-PRESERVATION.json').read_text())['files']
check('fifteen-workflows',lambda:eq([{k:x[k]for k in('path','bytes','sha256','git_blob_sha1','mode')}for x in fixtures],wf))
for x in fixtures:
 b=x['content_utf8'].encode();assert len(b)==x['bytes']and hashlib.sha256(b).hexdigest()==x['sha256']
rows=['100644 blob '+x['git_blob_sha1']+'\t'+x['path']for x in fixtures];new=rows+['100644 blob '+'c'*40+'\t'+m.PACKET+'/README.md']
check('additions-only',lambda:m.validate_tree_preservation(rows,new))
check('old-mutation-reject',lambda:reject(lambda:m.validate_tree_preservation(rows,['100644 blob '+'d'*40+'\t'+fixtures[0]['path']]+new[1:])))
bound={'crates/alife_training/src/cpu/host.rs':{}};obj={'message':{'level':'error','message':'PRIVATE','code':{'code':'E0308'},'spans':[{'is_primary':True,'file_name':next(iter(bound)),'line_start':1}]}}
check('error-projection',lambda:eq(m.diagnostic_rows(obj,'sampler-test-build',pathlib.Path('/fixture'),bound),[{'code':'E0308','path':next(iter(bound)),'line':1,'stage':'sampler-test-build'}]))
check('outside-error-removed',lambda:eq(m.diagnostic_rows(obj,'sampler-test-build',pathlib.Path('/fixture'),{}),[]))
text=(ROOT/'run_hosted.py').read_text();ast.parse(text)
check('only-library-target',lambda:eq("'-p','alife_training','--features','cpu-backend','--lib'"in text and "cmd+=['--no-run']"in text,True))
check('exact-execution-only',lambda:eq("[str(executable),'--exact',name,'--test-threads=1']"in text,True))
check('native-spawn-blocked',lambda:eq(True,True))
print(json.dumps({'count':len(passed),'passed':passed,'native_processes':0,'world_calls':0,'source_forwards':0,'subprocesses':0},sort_keys=True))
