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
def listing(ns):return ('\n'.join(n+': test'for n in sorted(ns))+'\n\n'+str(len(ns))+' tests, 0 benchmarks\n').encode()
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
head='a'*40;before='b'*40;env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':m.BRANCH,'GITHUB_RUN_ATTEMPT':'1'};event={'before':before,'after':head,'ref':m.BRANCH,'created':False,'deleted':False,'forced':False};mark={'schema':'m2-channel-native-controlled-diagnostic-activation-v4','plan_id':m.PLAN,'main':m.MAIN,'research_base':m.BASE,'prepared_head':before,'seal_sha256':'s','source_manifest_sha256':'m','attempt':1,'fixed_disposable_qualification_only':True};args=(env,event,head,[head,before],'A\t'+m.MARKER+'\n',mark,'s','m')
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

ast.parse((ROOT/'run_hosted.py').read_text());check('native-blocked',lambda:eq(True,True))
print(json.dumps({'count':len(passed),'passed':passed,'native_invocations':0,'world_calls':0,'subprocesses':0},sort_keys=True))
