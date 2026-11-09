import pathlib,json,importlib.util,subprocess,copy,hashlib,ast
ROOT=pathlib.Path(__file__).resolve().parent
subprocess.Popen=lambda *a,**k:(_ for _ in()).throw(AssertionError('No subprocess allowed'))
spec=importlib.util.spec_from_file_location('runner',ROOT/'run_hosted.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
passed=[]
def check(n,f):f();passed.append(n)
def eq(a,b):assert a==b
def reject(f):
 try:f()
 except m.Stop:return
 raise AssertionError('expected rejection')
def listing(ns):return ('\n'.join(n+': test'for n in sorted(ns))+'\n\n'+str(len(ns))+(' test'if len(ns)==1 else' tests')+', 0 benchmarks\n').encode()
check('one-exact',lambda:m.test_listing(listing(m.TEST_NAMES),m.TEST_NAMES))
for label,ns in [('extra',m.TEST_NAMES+['extra']),('missing',[]),('duplicate',m.TEST_NAMES*2)]:check(label+'-reject',lambda ns=ns:reject(lambda:m.test_listing(listing(ns),m.TEST_NAMES)))
rows=[]
for i,label in enumerate(['prefix','prefix','prefix','play','cold_play','no_manip']):
 tick=min(i,3);rows.append({'label':label,'ordinal':i,'tick_before':tick,'tick_after':tick+1,'generation_before':tick,'generation_after':tick+1,'play_selected':i!=5,'phase':'Preparation'if i!=5 else'Completed','posture_phase':'Attempt','stimulation':0.0,'investigation':0.0,'perceived_novelty':0.0,'awake_dose':0.0,'sleep_dose':0.0,'energy_before':0.8,'energy_after':0.79,'boredom_before':0.0,'boredom_after':0.0,'source_executions':1,'biology_intervals':1,'optimizer_updates':0})
endpoint={'intervals':6,'prefix_intervals':3,'treatment_intervals':3,'source_executions':6,'biology_intervals':6,'optimizer_updates':0,'cold_equal':True,'play_energy':0.79,'quiet_energy':0.79,'play_awake_dose':0.0,'quiet_awake_dose':0.0,'play_stimulation':0.0,'quiet_stimulation':0.0}
def report(rs=rows,ep=endpoint):return ('\nrunning 1 test\ntest '+m.TEST_NAMES[0]+' ... \n'+''.join('CONTROL_PHYSICS_V1 '+json.dumps(r)+'\n'for r in rs)+'CONTROL_ENDPOINT_V1 '+json.dumps(ep)+'\nCONTROL_PHASE_V1 '+json.dumps({'phase':'complete','ordinal':5,'attempted':6,'returned':6,'verified':6,'restored':2})+'\nok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s\n\n').encode()
check('zero-stimulation-and-recovery-valid',lambda:m.test_report(report(),m.TEST_NAMES[0]))
check('preparation-and-uncompleted-posture-valid',lambda:eq(m.STATE['controlled_observations'][0]['posture_phase'],'Attempt'))
check('partial-observation-preserved',lambda:eq(len(m.parse_control_rows(('PRIVATE\nCONTROL_PHYSICS_V1 '+json.dumps(rows[0])+'\nPRIVATE').encode(),False)[0]),1))
check('arbitrary-tail-reject',lambda:reject(lambda:m.test_report(report()+b'PRIVATE',m.TEST_NAMES[0])))
check('extra-field-reject',lambda:reject(lambda:m.test_report(report([{**rows[0],'private':'secret'}]+rows[1:]),m.TEST_NAMES[0])))
check('invalid-phase-reject',lambda:reject(lambda:m.test_report(report([{**rows[0],'phase':'PRIVATE'}]+rows[1:]),m.TEST_NAMES[0])))
check('numeric-phase-reject',lambda:reject(lambda:m.test_report(report([{**rows[0],'phase':1}]+rows[1:]),m.TEST_NAMES[0])))
check('nonfinite-reject',lambda:reject(lambda:m.test_report(report([{**rows[0],'stimulation':float('nan')}]+rows[1:]),m.TEST_NAMES[0])))
check('wrong-dose-reject',lambda:reject(lambda:m.test_report(report(ep={**endpoint,'intervals':7}),m.TEST_NAMES[0])))
check('endpoint-mismatch-reject',lambda:reject(lambda:m.test_report(report(ep={**endpoint,'quiet_energy':0.5}),m.TEST_NAMES[0])))
check('ignored-reject',lambda:reject(lambda:m.test_report(report().replace(b'0 ignored',b'1 ignored'),m.TEST_NAMES[0])))
check('pipe-cap-reject',lambda:reject(lambda:m.test_report(b'x'*65537,m.TEST_NAMES[0])))
manifest=json.loads((ROOT/'source-manifest.json').read_text());source=m.capsule_source((ROOT/'host-source-capsule.tar.gz').read_bytes(),manifest)
check('capsule107',lambda:eq(len(source),107))
owned=json.loads((ROOT/'FIXED-INVENTORY.json').read_text())[0]['source'];check('one-owned-test',lambda:eq(source[owned].count(b'#[test]'),1));check('no-ignored',lambda:eq(b'#[ignore'in source[owned],False))
cn=source['crates/alife_training/src/cpu/motor_mediation/confidence/policy/optional_vocal/channel_native.rs']
check('ordinary-draw-control-absent',lambda:eq(b'#[cfg(test)] controlled_physics:None'in cn,True))
check('controlled-gradient-rejected',lambda:eq(b'if a.controlled_physics.is_some() {return Err(invalid());}'in cn,True))
check('actual-host-step-only',lambda:eq(b'grouped_policy_step_observed::<Controlled<PLAY>>'in source[owned]and b'apply_registered_command'not in source[owned],True))
check('fixed-prefix-no-adaptation',lambda:eq(b'for i in 0..3'in source[owned],True))
check('proof-not-rng',lambda:eq(b'channel-native-controlled-physics-proof-v1;no-rng-draw'in cn,True))
literal='#[test] fn old() {}\nconst S:&str=r##"\n#[test]\n"##;\n'
check('projection-preserves-strings',lambda:eq(m.exclude_old_test_registration(literal),(literal.replace('#[test]','#[cfg(any())]',1),1)))
head='a'*40;before='b'*40;env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':m.BRANCH,'GITHUB_RUN_ATTEMPT':'1'};event={'before':before,'after':head,'ref':m.BRANCH,'created':False,'deleted':False,'forced':False};mark={'schema':'m2-channel-native-controlled-listing-activation-v5','plan_id':m.PLAN,'main':m.MAIN,'research_base':m.BASE,'prepared_head':before,'seal_sha256':'s','source_manifest_sha256':'m','attempt':1,'fixed_disposable_qualification_only':True};args=(env,event,head,[head,before],'A\t'+m.MARKER+'\n',mark,'s','m')
check('sole-marker-valid',lambda:m.validate_activation(*args));check('attempt2-reject',lambda:reject(lambda:m.validate_activation({**env,'GITHUB_RUN_ATTEMPT':'2'},*args[1:])));check('extra-marker-reject',lambda:reject(lambda:m.validate_activation(*args[:4],args[4]+'A\tother\n',*args[5:])))
fixtures=json.loads((ROOT/'INHERITED-WORKFLOW-FIXTURES.json').read_text())['files'];wf=json.loads((ROOT/'WORKFLOW-PRESERVATION.json').read_text())['files'];check('fifteen-workflows-preserved',lambda:eq([{k:x[k]for k in('path','bytes','sha256','git_blob_sha1','mode')}for x in fixtures],wf))
obj={'message':{'level':'error','message':'PRIVATE','code':{'code':'E0308'},'spans':[{'is_primary':True,'file_name':owned,'line_start':1}]}};check('error-projection',lambda:eq(m.diagnostic_rows(obj,'sampler-test-build',pathlib.Path('/fixture'),{owned:{}}),[{'code':'E0308','path':owned,'line':1,'stage':'sampler-test-build'}]))
check('outside-error-removed',lambda:eq(m.diagnostic_rows(obj,'sampler-test-build',pathlib.Path('/fixture'),{}),[]))

def phase_wire(row):return 'CONTROL_PHASE_V1 '+json.dumps(row)+'\n'
phase0={'phase':'construction_started','ordinal':0,'attempted':0,'returned':0,'verified':0,'restored':0}
phase1={**phase0,'phase':'dispatch_started','attempted':1}
check('partial-failure-phase-retained',lambda:eq(m.parse_control_rows(('PRIVATE\n'+phase_wire(phase0)+phase_wire(phase1)+'PRIVATE').encode(),False)[0],[]))
check('last-safe-phase-exposed',lambda:eq(m.STATE['phase_snapshots'][-1],phase1))
check('unknown-phase-rejected',lambda:reject(lambda:m.parse_control_rows(phase_wire({**phase0,'phase':'PRIVATE'}).encode(),False)))
check('unsafe-phase-field-rejected',lambda:reject(lambda:m.parse_control_rows(phase_wire({**phase0,'secret':'PRIVATE'}).encode(),False)))
check('phase-counter-overflow-rejected',lambda:reject(lambda:m.parse_control_rows(phase_wire({**phase1,'attempted':7}).encode(),False)))
check('phase-counter-regression-rejected',lambda:reject(lambda:m.parse_control_rows((phase_wire(phase1)+phase_wire(phase0)).encode(),False)))
check('phase-float-counter-rejected',lambda:reject(lambda:m.parse_control_rows(phase_wire({**phase0,'ordinal':0.0}).encode(),False)))
check('complete-requires-terminal-counters',lambda:reject(lambda:m.parse_control_rows(phase_wire(phase0).encode(),True)))


# Independent one-test wire fixtures are transcribed from pinned official libtest,
# not rendered by the parser or the local listing/report helpers.
pinned_list=(ROOT/'PINNED-LIBTEST-LIST.txt').read_bytes()
check('pinned-libtest-singular-list',lambda:m.test_listing(pinned_list,m.TEST_NAMES))
check('plural-one-footer-rejected',lambda:reject(lambda:m.test_listing(pinned_list.replace(b'1 test,',b'1 tests,'),m.TEST_NAMES)))
check('wrong-listed-name-rejected',lambda:reject(lambda:m.test_listing(pinned_list.replace(b'::controlled::',b'::wrong::'),m.TEST_NAMES)))
check('missing-list-row-rejected',lambda:reject(lambda:m.test_listing(b'0 tests, 0 benchmarks\n',m.TEST_NAMES)))
check('benchmarks-rejected',lambda:reject(lambda:m.test_listing(pinned_list.replace(b'0 benchmarks',b'1 benchmark'),m.TEST_NAMES)))
check('listing-arbitrary-tail-rejected',lambda:reject(lambda:m.test_listing(pinned_list+b'PRIVATE\n',m.TEST_NAMES)))
check('listing-invalid-utf8-rejected',lambda:reject(lambda:m.test_listing(pinned_list+b'\xff',m.TEST_NAMES)))
# Only allowlisted intervention readouts are removed before comparing the exact
# libtest execution envelope. This exercises run start/name/footer jointly.
pinned_envelope=(ROOT/'PINNED-LIBTEST-RUN.txt').read_bytes()
control_wire=''.join('CONTROL_PHYSICS_V1 '+json.dumps(r)+'\n'for r in rows)+'CONTROL_ENDPOINT_V1 '+json.dumps(endpoint)+'\nCONTROL_PHASE_V1 '+json.dumps({'phase':'complete','ordinal':5,'attempted':6,'returned':6,'verified':6,'restored':2})+'\n'
wire=pinned_envelope.replace(b' ... ok\n',b' ... \n'+control_wire.encode()+b'ok\n')
check('pinned-one-test-result-zero-filtered',lambda:m.test_report(wire,m.TEST_NAMES[0]))
for label,bad in [('stale-twelve-filtered',wire.replace(b'0 filtered out',b'12 filtered out')),('one-filtered',wire.replace(b'0 filtered out',b'1 filtered out')),('two-passed',wire.replace(b'1 passed',b'2 passed')),('running-plural',wire.replace(b'running 1 test\n',b'running 1 tests\n')),('wrong-executed-name',wire.replace(b'::controlled::',b'::wrong::')),('failed-result',wire.replace(b'test result: ok.',b'test result: FAILED.')),('arbitrary-result-tail',wire+b'PRIVATE')]:
 check(label+'-reject',lambda bad=bad:reject(lambda:m.test_report(bad,m.TEST_NAMES[0])))
proof=json.loads((ROOT/'LIBTEST-DISCOVERY-PROOF.json').read_text())
ns={'fail':m.fail};exec(proof['v4_test_listing_source'],ns)
check('frozen-v4-rejects-official-singular',lambda:reject(lambda:ns['test_listing'](pinned_list,m.TEST_NAMES)))
check('fixture-wire-exact-name',lambda:eq(pinned_list,(m.TEST_NAMES[0]+': test\n\n1 test, 0 benchmarks\n').encode()))
check('source-capsule-unchanged-from-v4',lambda:eq(hashlib.sha256((ROOT/'host-source-capsule.tar.gz').read_bytes()).hexdigest(),proof['v4_capsule_sha256']))


freeze=json.loads((ROOT/'FUNCTION-FREEZE.json').read_text());runner_text=(ROOT/'run_hosted.py').read_text();runner_tree=ast.parse(runner_text)
fn_sources={x.name:ast.get_source_segment(runner_text,x)for x in runner_tree.body if isinstance(x,ast.FunctionDef)}
check('all-36-frozen-runner-functions',lambda:eq({k:hashlib.sha256(fn_sources[k].encode()).hexdigest()for k in freeze['unchanged_runner_functions']},freeze['unchanged_runner_functions']))
check('only-three-version-or-listing-functions-change',lambda:eq(sorted(freeze['changed_runner_functions']),['summary','test_listing','validate_activation']))
prior=json.loads((ROOT/'V4-PRESERVATION.json').read_text());prior_files={r['path']:r for r in prior['files']}
for n in ['CONTROLLED-FIXTURE.patch','DIAGNOSTIC-ONLY.patch','DOSE-INVENTORY.json','FIXED-INVENTORY.json','OUTPUT-POLICY.json','REUSE-INPUTS.json','WORKFLOW-PRESERVATION.json','INHERITED-WORKFLOW-FIXTURES.json','host-source-capsule.tar.gz']:
 check('v4-unchanged-'+n,lambda n=n:eq(hashlib.sha256((ROOT/n).read_bytes()).hexdigest(),prior_files[n]['sha256']))
source_proof=json.loads((ROOT/'SOURCE-PROOF.json').read_text())
check('all-source-manifest-fields-frozen',lambda:eq({k:hashlib.sha256(json.dumps(manifest[k],sort_keys=True,separators=(',',':')).encode()).hexdigest()for k in source_proof['source_manifest_unchanged_field_sha256']},source_proof['source_manifest_unchanged_field_sha256']))
check('resource-floors-unchanged',lambda:eq(m.LIMITS,{'wall_seconds':5400,'rss_bytes':6*1024**3,'workspace_bytes':16*1024**3,'free_bytes':4*1024**3,'memavailable_bytes':2*1024**3,'initial_free_bytes':20*1024**3,'output_bytes':32*1024**2,'summary_bytes':32768,'toolchain_bytes':3*1024**3,'poll_seconds':0.25,'compile_seconds':2700,'fixture_seconds':300}))
closure=json.loads((ROOT/'SOURCE-CLOSURE-STATIC.json').read_text())
check('static-closure-exact-inventory',lambda:eq(closure['expected_executable_inventory'],m.TEST_NAMES))
check('static-closure-exact-v4-projection',lambda:eq((closure['excluded_old_test_count'],closure['test_projection_sha256']),(396,'17e1310f96fe4bdb4dc7c2fd7ade4df0aecc95f4ab59b28f50f0f5175a57f22a')))
check('accepted-main-1313-workflows-15',lambda:eq((closure['accepted_main_leaves'],closure['inherited_workflows']),(1313,15)))
# Exercise the real runner protocol control flow with inert wire data, never a
# native executable. Confirm listing rejection cannot enter the fixture.
old_run,old_digest,old_bound=m.run,m.digest_file,m.BOUND
calls=[]
def fake_run(cmd,*args,**kwargs):
 calls.append(cmd)
 return (pinned_list if len(calls)==1 else wire,0)
m.run=fake_run;m.digest_file=lambda path:'fixed';m.BOUND={};m.STATE['test_processes_attempted']=0;m.STATE['test_results']=[]
m.run_tests(pathlib.Path('/inert'), 'fixed')
check('one-exact-synthetic-runner-invocation',lambda:eq(calls,[['/inert','--list'],['/inert','--exact',m.TEST_NAMES[0],'--test-threads=1','--nocapture']]))
check('synthetic-runner-single-result',lambda:eq((m.STATE['test_processes_attempted'],m.STATE['test_results']),(1,[{'name':m.TEST_NAMES[0],'passed':1,'failed':0,'ignored':0}])))
calls.clear();m.STATE['test_processes_attempted']=0
m.run=lambda cmd,*args,**kwargs:(calls.append(cmd)or(pinned_list.replace(b'1 test,',b'1 tests,'),0))
check('synthetic-list-failure-blocks-fixture',lambda:reject(lambda:m.run_tests(pathlib.Path('/inert'),'fixed')))
check('synthetic-list-failure-zero-tests',lambda:eq((calls,m.STATE['test_processes_attempted']),([['/inert','--list']],0)))
m.run,m.digest_file,m.BOUND=old_run,old_digest,old_bound


fixture=source[owned].decode()
check('fixture-single-actual-host-callsite',lambda:eq(fixture.count('grouped_policy_step_observed::<Controlled<PLAY>>'),1))
check('fixture-three-prefix-plus-three-treatment',lambda:eq(('for i in 0..3 'in fixture,fixture.count('interval::<true>'),fixture.count('interval::<false>')),(True,3,1)))
check('fixture-no-extra-invalid-forward-probe',lambda:eq(any(x in fixture for x in ['forward_canonical','dispatch_foundation','NaN','NAN','optimize(','waking_synaptic_plasticity(']),False))

ast.parse((ROOT/'run_hosted.py').read_text());check('native-blocked',lambda:eq(True,True))
print(json.dumps({'count':len(passed),'passed':passed,'native_invocations':0,'world_calls':0,'subprocesses':0},sort_keys=True))
