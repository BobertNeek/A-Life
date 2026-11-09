"""Pure Python source/protocol controls. No imports of the runner or child process calls."""
import ast,copy,hashlib,io,json,pathlib,re,tarfile
ROOT=pathlib.Path(__file__).resolve().parent
source=(ROOT/'run_hosted.py').read_text();tree=ast.parse(source)
selected={'Stop','fail','sha','safe_path','overlay_members','strict_number','nullable_bool','project_reason','project_meal_v2','project_sources_v3','project_current_v3','test_report','_test_report_v4','project_full_support_v5','test_listing','validate_activation','validate_tree_preservation'}
nodes=[n for n in tree.body if isinstance(n,(ast.FunctionDef,ast.ClassDef))and n.name in selected]
assert {n.name for n in nodes}==selected
assign={}
for n in tree.body:
 if isinstance(n,ast.Assign)and len(n.targets)==1 and isinstance(n.targets[0],ast.Name)and n.targets[0].id in ['PLAN','MAIN','BASE','TEST_NAME']:
  assign[n.targets[0].id]=ast.literal_eval(n.value)
ns=dict(assign,json=json,re=re,hashlib=hashlib,tarfile=tarfile,io=io,pathlib=pathlib,CODES={'BINDING','SOURCE_CLOSURE','TEST_PROTOCOL','OUTPUT_LIMIT','ADMISSION','INTERNAL'})
ns['BRANCH']='refs/heads/qualification/'+ns['PLAN'];ns['PACKET']='docs/experiments/'+ns['PLAN'];ns['MARKER']=ns['PACKET']+'/ACTIVATE.json'
exec(compile(ast.Module(body=nodes,type_ignores=[]),'<pure-selected-runner-functions>','exec'),ns)
ns['v5_test_report']=ns['test_report'];ns['test_report']=ns['_test_report_v4']
passed=[]
def check(name,fn):fn();passed.append(name)
def eq(a,b):assert a==b,(a,b)
def reject(fn):
 try:fn()
 except ns['Stop']:return
 raise AssertionError('Expected schema/admission rejection')
def sources(value=-.4,weight=.5):
 return dict(schema='canonical-targetless-source-readouts-v3',query_capture_success=True,sources=[dict(sequence=1,source_consumed=False,signed_joint_valence=.2,canonical_source=True,v3_weight=None),dict(sequence=2,source_consumed=True,signed_joint_valence=value,canonical_source=True,v3_weight=weight)],v3_joint_value=None if weight is None else value*weight,cold_equal=True,cross_load_rejected=True,future_grip='Unknown',future_consumption='Unknown')
def current(value=-.4,weight=.5):
 sr=sources(value,weight);rows=[]
 for name in ['v3_on','off','strict_v1','graded_v1','meal_v2']:
  rows.append(dict(reader=name,support_count=2,selected_index=0,selected_primitives=[[4,104]],selected_eat=False,same_bundle_as_on=True,bundle_probability=.5,selected_joint_log_probability=-.6931471805599453,selected_potential=(sr['v3_joint_value']or 0.)if name=='v3_on'else 0.,second_root_eat_probability=0.,probability_sum=1.))
 return dict(sampler=dict(schema='canonical-targetless-current-opportunity-v3',comparison='same-real-source-frame-rng-readers-one-committed-path',source_dispatch_generation=3,memory_last_sequence=2,decision_sequence=3,frame_tick=243,seed=123,counter=2,scripted=False,second_root_candidate_present=False,readouts=rows,actual_source_association=sr,future_grip='Unknown',future_consumption='Unknown'),third_source_consumed=False,third_joint_valence=-.1,committed_paths=1,registered_intervals=3,source_forwards=3,counter_deltas=[3]*5,final_memory_cold_equal=True)
for name,val,weight in [('negative',-.4,.5),('positive',.4,.5),('zero',0.,.5),('no-match',.4,None)]:
 check('current-'+name+'-not-eating-valid',lambda v=val,w=weight:ns['project_current_v3'](current(v,w)))
for name,path,value in [('stale-source',['sampler','source_dispatch_generation'],2),('stale-sequence',['sampler','decision_sequence'],2),('stale-memory',['sampler','memory_last_sequence'],3),('scripted-third',['sampler','scripted'],True),('extra-world-path',['committed_paths'],2),('extra-forward',['source_forwards'],4),('missing-biology',['counter_deltas'],[3,3,3,3,2]),('future-invented',['sampler','future_consumption'],'Yes'),('extra-key',['secret'],1)]:
 bad=current();p=bad
 for k in path[:-1]:p=p[k]
 p[path[-1]]=value;check(name+'-rejected',lambda b=bad:reject(lambda:ns['project_current_v3'](b)))
for name,key,value in [('probability-sum','probability_sum',.8),('nonfinite','bundle_probability',float('nan')),('out-of-support','selected_index',2),('wrong-reader','reader','injected'),('invented-eat','selected_eat',True),('effect-mismatch','selected_potential',.2)]:
 bad=current();bad['sampler']['readouts'][0][key]=value;check(name+'-rejected',lambda b=bad:reject(lambda:ns['project_current_v3'](b)))
bad=sources();bad['sources'][1]['canonical_source']=False;check('uncanonical-source-weight-rejected',lambda:reject(lambda:ns['project_sources_v3'](bad)))
bad2=sources();bad2['v3_joint_value']=.5;check('signed-weight-recomputed',lambda:reject(lambda:ns['project_sources_v3'](bad2)))
check('exact-one-list',lambda:ns['test_listing']((ns['TEST_NAME']+': test\n\n1 test, 0 benchmarks\n').encode(),ns['TEST_NAME']))
check('extra-test-rejected',lambda:reject(lambda:ns['test_listing']((ns['TEST_NAME']+': test\nextra: test\n\n2 tests, 0 benchmarks\n').encode(),ns['TEST_NAME'])))
pre='\nrunning 1 test\ntest '+ns['TEST_NAME']+' ... ';tail='ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s\n\n'
h={'candidate_present':False,'meal_matched':False,'reason':'second root not in actual observed support'}
raw=(pre+'CUE_HISTORICAL_SECOND_TARGET '+json.dumps(h)+'\nCURRENT_OPPORTUNITY_V3 '+json.dumps(current())+'\n'+tail).encode()
check('absent-historical-current-report-valid',lambda:ns['test_report'](raw))
check('private-tail-rejected',lambda:reject(lambda:ns['test_report'](raw+b'private assertion payload')))
check('output-hard-cap',lambda:reject(lambda:ns['test_report'](b'x'*65537)))
reason=dict(query_capture_success=True,query_capture_error=None,records=[dict(sequence=i,source_frame_bytes=19000,frame_budget_bytes=32768,witness_retained=True,stored_capture_success=True,rebuild_capture_success=True,rebuild_capture_error=None,original_self_match=True,pair_match=False,pair_rejections=['Posture:observation_presence'],signed_joint_valence=.2 if i==1 else -.4,source_consumed=i==2)for i in(1,2)],future_grip='Unknown',future_consumption='Unknown',scope='historical-frame diagnostic, no policy dispatch')
g={'reason_report':reason,'graded_joint_value':None,'future_consumption':'Unknown'}
v2=dict(schema='meal-kinematic-matched-readouts-v2',comparison='matched-engineering-regression-not-held-out',sources=[dict(sequence=r['sequence'],source_consumed=r['source_consumed'],signed_joint_valence=r['signed_joint_valence'],v1_graded_weight=None,v2_meal_weight=None)for r in reason['records']],v1_graded_joint_value=None,v2_meal_joint_value=None,v1_v2_cross_load_rejected=True,cold_readouts_equal=True,future_grip='Unknown',future_consumption='Unknown')
h=dict(candidate_present=True,cue_error=None,cue_sequences=None,cue_valence=None,exact_error=None,exact_sequences=None,future_consumption='Unknown',meal_matched=False)
full=(pre+'\n'.join(tag+' '+json.dumps(v)for tag,v in [('GRADED_CUE_DIAGNOSTIC',g),('MEAL_CUE_V2_DIAGNOSTIC',v2),('TARGETLESS_HISTORICAL_V3',sources()),('CUE_HISTORICAL_SECOND_TARGET',h),('CURRENT_OPPORTUNITY_V3',current())])+'\n'+tail).encode()
check('all-historical-and-current-readers-valid',lambda:ns['test_report'](full))
check('historical-only-is-not-completion',lambda:reject(lambda:ns['test_report'](full[:full.index(b'CURRENT_OPPORTUNITY_V3')]+tail.encode())))
reason2=copy.deepcopy(reason);reason2['records'][0]['rebuild_capture_error']='PRIVATE';check('legacy-private-debug-redacted',lambda:eq('PRIVATE'in json.dumps(ns['project_reason'](reason2)),False))
head='a'*40;before='b'*40;seal='c'*64;src='d'*64
env=dict(GITHUB_ACTIONS='true',GITHUB_EVENT_NAME='push',GITHUB_REPOSITORY='BobertNeek/A-Life',GITHUB_REF=ns['BRANCH'],GITHUB_RUN_ATTEMPT='1')
event=dict(before=before,after=head,ref=ns['BRANCH'],created=False,deleted=False,forced=False)
marker=dict(schema='m2-targetless-cue-first-rejection-source-activation-v6',plan_id=ns['PLAN'],main=ns['MAIN'],research_base=ns['BASE'],prepared_head=before,seal_sha256=seal,source_manifest_sha256=src,attempt=1,fixed_disposable_qualification_only=True)
args=(event,head,[head,before],'A\t'+ns['MARKER']+'\n',marker,seal,src)
check('exact-activation-admitted',lambda:eq(ns['validate_activation'](env,*args),before))
for k,v in [('GITHUB_RUN_ATTEMPT','2'),('GITHUB_EVENT_NAME','workflow_dispatch'),('GITHUB_REF','refs/heads/main')]:
 bad=dict(env);bad[k]=v;check(k+'-rejected',lambda b=bad:reject(lambda:ns['validate_activation'](b,*args)))
manifest=json.loads((ROOT/'source-manifest.json').read_bytes());reuse=json.loads((ROOT/'REUSE-MANIFEST.json').read_bytes());lookup={r['path']:pathlib.Path(r['source_artifact'])for r in reuse['files']}
def get(name):
 p=ROOT/name
 return(p if p.is_file()else lookup[name]).read_bytes()
for name,key in [('canonical-targetless-source-overlay.tar.gz','canonical_v3_overlay_files'),('meal-kinematic-source-overlay.tar.gz','meal_v2_overlay_files'),('graded-source-overlay.tar.gz','graded_overlay_files'),('qualified-original-overlay.tar.gz','qualified_overlay_files'),('nominal-frozen-input-residual.tar.gz','residual_files')]:
 check(name+'-sealed-closure',lambda n=name,k=key:ns['overlay_members'](get(n),{r['path']:r for r in manifest[k]}))
check('source-counts',lambda:eq([len(manifest[k])for k in ['base_files','graded_files','canonical_v3_overlay_files']],[799,829,8]))
frozen=json.loads((ROOT/'FUNCTION-FREEZE.json').read_bytes())
for row in frozen['functions']:
 node=next(n for n in tree.body if isinstance(n,(ast.FunctionDef,ast.ClassDef))and n.name==row['name']);assert hashlib.sha256(ast.get_source_segment(source,node).encode()).hexdigest()==row['sha256']
check('inherited-enforcement-function-freeze',lambda:eq(frozen['changed_functions'],['validate_activation']))


# Compile-only V4 regression controls use sealed source bytes, never Rust execution.
with tarfile.open(fileobj=io.BytesIO(get('canonical-targetless-source-overlay.tar.gz')),mode='r:gz') as tf:
 overlay={a.name:tf.extractfile(a).read()for a in tf if a.isfile()}
repair=json.loads((ROOT/'SOURCE-REPAIR.json').read_bytes());fixpath=repair['path'];fixture=overlay[fixpath].decode()
check('sized-bound-fixed-at-original-helper',lambda:eq(fixture.count('fn bytes<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {'),1))
check('both-original-slice-operands-retained',lambda:eq('assert_eq!(bytes(&final_memory.sleep_joint_sources()[..2]),bytes(memory.sleep_joint_sources()));'in fixture,True))
check('original-fixed-test-name-retained',lambda:eq('fn '+ns['TEST_NAME'].split('::')[-1]+'()'in fixture.replace(' ()','()'),True))
check('three-existing-source-interval-calls-only',lambda:eq(fixture.count('grouped_policy_step_observed::<ScriptedJointMechanismPolicy>'),3))
check('one-active-test-registration',lambda:eq(fixture.replace('#[cfg(any())]\n#[test]','').count('#[test]'),1))
check('native-v3-owner-retained',lambda:eq('fresh-canonical-targetless-joint-v3/profile5/exact-role-registry/no-migration'in overlay['crates/alife_training/src/cpu/canonical_cue_v3.rs'].decode(),True))
check('owned-restored-memory-across-third-step',lambda:eq('fn memory(&self)->Result<JointEpisodeMemoryV1,CpuHostError>{Ok(JointEpisodeMemoryV1::restore(&self.bytes,&self.anchor)?)}'in overlay['crates/alife_training/src/cpu/joint_episode_host.rs'].decode(),True))
for text in ['assert_eq!(frozen(&host),before);','assert_eq!(final_counters,(counters.0+3,counters.1+3,counters.2+3,counters.3+3,counters.4+3));','assert_eq!(cold.sleep_joint_sources(),final_memory.sleep_joint_sources());']:
 check('retained-invariant-'+str(len(passed)),lambda t=text:eq(t in fixture,True))
check('minimal-prior-compile-provenance-reference',lambda:eq(set(json.loads((ROOT/'PRIOR-V3-FAILURE.json').read_bytes())),{'run_url','activation_commit','content_sha256'}))
check('historical-v4-compile-repair-only-fixture',lambda:eq(manifest['compile_repair_v4']['changed_source_files'],[fixpath]))
# Reject changing, removing, renaming or mode-changing ANY inherited main entry.
main=json.loads((ROOT/'ACCEPTED-MAIN-TREE.json').read_bytes());mainrows=[r['mode']+' blob '+r['git_blob_sha1']+'\t'+r['path']for r in main['files']];prepared=mainrows+['100644 blob '+'e'*40+'\t'+ns['PACKET']+'/seal.json','100644 blob '+'f'*40+'\t.github/workflows/'+ns['PLAN']+'.yml']
check('all1313-main-files-preserved',lambda:eq(len(mainrows),1313))
check('all15-inherited-workflows-preserved',lambda:eq(len([r for r in main['files']if r['path'].startswith('.github/workflows/')]),15))
check('additions-only-full-tree-admitted',lambda:ns['validate_tree_preservation'](mainrows,prepared))
for name,bad in [('blob-change',[prepared[0].replace(main['files'][0]['git_blob_sha1'],'0'*40)]+prepared[1:]),('deletion',prepared[1:]),('mode-change',[prepared[0].replace('100644','100755',1)]+prepared[1:]),('unexpected-path',prepared+['100644 blob '+'0'*40+'\tunexpected.txt']),('premature-activation',prepared+['100644 blob '+'0'*40+'\t'+ns['MARKER']])]:
 check('full-tree-'+name+'-rejected',lambda b=bad:reject(lambda:ns['validate_tree_preservation'](mainrows,b)))
for name,changed in [('extra-activation-file','A\t'+ns['MARKER']+'\nA\textra.txt\n'),('modified-marker','M\t'+ns['MARKER']+'\n')]:
 badargs=(event,head,[head,before],changed,marker,seal,src);check(name+'-rejected',lambda a=badargs:reject(lambda:ns['validate_activation'](env,*a)))
oldmarker=dict(marker);oldmarker['schema']='m2-targetless-cue-activation-v3';check('old-v3-marker-rejected',lambda:reject(lambda:ns['validate_activation'](env,event,head,[head,before],'A\t'+ns['MARKER']+'\n',oldmarker,seal,src)))
limit_node=next(n for n in tree.body if isinstance(n,ast.Assign)and any(isinstance(t,ast.Name)and t.id=='LIMITS'for t in n.targets))
limit_ns={'__builtins__':{}};exec(compile(ast.Module(body=[limit_node],type_ignores=[]),'<pure-numeric-limit-assignment>','exec'),limit_ns);limits=limit_ns['LIMITS']
check('floors-and-resource-bounds-retained',lambda:eq([limits[k]for k in ['initial_free_bytes','free_bytes','memavailable_bytes','rss_bytes','wall_seconds','compile_seconds','fixture_seconds','summary_bytes']],[20*1024**3,4*1024**3,2*1024**3,6*1024**3,5400,2700,300,32768]))
check('historical-v4-nine-byte-trait-bound-only',lambda:eq(repair['after']['bytes']-repair['before']['bytes'],9))


# V5 diagnostics retain the prior report schema and add one strict bounded projection.
def full_current_v5():
 obj=current(value=-.4,weight=None)
 for row in obj['sampler']['readouts']:row['support_count']=1330
 return obj

def support_v5():
 c=full_current_v5();sampler=c['sampler'];n=1330
 sources_rows=[]
 for s in sampler['actual_source_association']['sources']:
  sources_rows.append(dict(sequence=s['sequence'],source_consumed=s['source_consumed'],signed_joint_valence=s['signed_joint_valence'],matched_rows=0,negative_rows=0,zero_rows=0,positive_rows=0,second_root_eat_matched_rows=0,first_rejections=[dict(role=None,reason='RoleCount',count=n)]))
 return dict(schema='canonical-targetless-full-support-first-rejection-v5',support_kind='production-rows-including-physical-bundle-aliases',support_count=n,unique_bundle_count=800,original_source_count=2,query_capture_success_count=n,value_counts=dict(none=n,negative=0,zero=0,positive=0),second_root_eat_rows=0,second_root_eat_matching_rows=0,second_root_eat_nonzero_rows=0,sources=sources_rows,probabilities=dict(max_abs_row_on_off=0.,max_abs_bundle_on_off=0.,nominal_kl_on_off=0.,kl_roundoff_tolerance=1e-9,sum_on=1.,sum_off=1.,on_probability_underflow_rows=0,off_probability_underflow_rows=0),administrative_dose=dict(additional_support_enumerations=2,bundle_queries=n,paired_source_admission_queries=2*n,mirror_weight_equality_checks=2*n,diagnostic_production_weight_evaluations=4*n,production_value_equality_checks=n,row_potential_equality_checks=n,extra_native_forwards=0,extra_world_intervals=0,alternate_committed_paths=0),source_dispatch_generation=3,memory_last_sequence=2,decision_sequence=3,frame_tick=sampler['frame_tick'],seed=sampler['seed'],counter=sampler['counter'],source_archive_unchanged=True,future_grip='Unknown',future_consumption='Unknown')

check('v5-full-support-null-valid',lambda:ns['project_full_support_v5'](support_v5(),full_current_v5()))
small_negative=support_v5();small_negative['probabilities']['nominal_kl_on_off']=-1e-12
check('v5-negative-kl-roundoff-retained',lambda:eq(ns['project_full_support_v5'](small_negative,full_current_v5())['probabilities']['nominal_kl_on_off'],-1e-12))
for name,path,value in [('unrecognized-key',['private'],1),('wrong-schema',['schema'],'future'),('wrong-support-count',['support_count'],1331),('bool-count',['support_count'],True),('wrong-source-count',['original_source_count'],3),('zero-unique-bundles',['unique_bundle_count'],0),('stale-source',['source_dispatch_generation'],2),('stale-frame',['frame_tick'],0),('wrong-seed',['seed'],1),('future-consumption',['future_consumption'],'Yes'),('source-mutated',['source_archive_unchanged'],False),('value-count-sum',['value_counts','none'],1329),('invalid-eat-count',['second_root_eat_rows'],1331),('invented-eat-match',['second_root_eat_matching_rows'],1),('unknown-reason',['sources',0,'first_rejections',0,'reason'],'PRIVATE'),('untyped-reason',['sources',0,'first_rejections',0,'reason'],[]),('role-on-global',['sources',0,'first_rejections',0,'role'],0),('missing-global-role',['sources',0,'first_rejections',0,'reason'],'Chemistry'),('hist-count-sum',['sources',0,'first_rejections',0,'count'],1329),('hist-bool-count',['sources',0,'first_rejections',0,'count'],True),('fabricated-match',['sources',0,'matched_rows'],1),('changed-sequence',['sources',0,'sequence'],3),('changed-consumed',['sources',1,'source_consumed'],False),('changed-valence',['sources',0,'signed_joint_valence'],.9),('nonfinite-delta',['probabilities','max_abs_row_on_off'],float('nan')),('negative-abs-delta',['probabilities','max_abs_bundle_on_off'],-.1),('too-negative-kl',['probabilities','nominal_kl_on_off'],-1e-6),('non-normalized',['probabilities','sum_on'],.9),('extra-enumeration',['administrative_dose','additional_support_enumerations'],3),('extra-forward',['administrative_dose','extra_native_forwards'],1),('extra-world',['administrative_dose','extra_world_intervals'],1),('alternate-path',['administrative_dose','alternate_committed_paths'],1),('wrong-mirror-count',['administrative_dose','mirror_weight_equality_checks'],2659),('wrong-production-count',['administrative_dose','diagnostic_production_weight_evaluations'],2660)]:
 bad=support_v5();p=bad
 for k in path[:-1]:p=p[k]
 p[path[-1]]=value
 check('v5-'+name+'-rejected',lambda b=bad:reject(lambda:ns['project_full_support_v5'](b,full_current_v5())))
# Some(0) remains distinct from None; a matched zero-weight/contribution is valid.
zero=support_v5();zero['value_counts']={'none':0,'negative':0,'zero':1330,'positive':0}
for row in zero['sources']:
 row.update(matched_rows=1330,zero_rows=1330,first_rejections=[dict(role=None,reason='Matched',count=1330)])
zero_current=full_current_v5()
for row in zero_current['sampler']['actual_source_association']['sources']:row['v3_weight']=0.
zero_current['sampler']['actual_source_association']['v3_joint_value']=0.
check('v5-accepted-zero-not-null-valid',lambda:ns['project_full_support_v5'](zero,zero_current))
positive=support_v5();positive['value_counts']={'none':0,'negative':0,'zero':0,'positive':1330};positive['sources'][0].update(matched_rows=1330,positive_rows=1330,first_rejections=[dict(role=None,reason='Matched',count=1330)])
positive['probabilities']['max_abs_row_on_off']=.01;positive['probabilities']['max_abs_bundle_on_off']=.03;positive['probabilities']['nominal_kl_on_off']=.002
positive_current=full_current_v5();positive_current['sampler']['actual_source_association']['sources'][0]['v3_weight']=.5;positive_current['sampler']['actual_source_association']['v3_joint_value']=.1;positive_current['sampler']['readouts'][0]['selected_potential']=.1
check('v5-positive-association-and-effect-valid',lambda:ns['project_full_support_v5'](positive,positive_current))
negative=support_v5();negative['value_counts']={'none':0,'negative':1330,'zero':0,'positive':0};negative['sources'][1].update(matched_rows=1330,negative_rows=1330,first_rejections=[dict(role=None,reason='Matched',count=1330)])
negative_current=full_current_v5();negative_current['sampler']['actual_source_association']['sources'][1]['v3_weight']=.5;negative_current['sampler']['actual_source_association']['v3_joint_value']=-.2;negative_current['sampler']['readouts'][0]['selected_potential']=-.2
check('v5-negative-association-valid',lambda:ns['project_full_support_v5'](negative,negative_current))
# Capture failures reduce eligible kernel comparisons but not source admission pairs.
capture=support_v5();capture['query_capture_success_count']=0
for row in capture['sources']:row['first_rejections']=[dict(role=None,reason='QueryCaptureRejected',count=1330)]
capture['administrative_dose']['mirror_weight_equality_checks']=0;capture['administrative_dose']['diagnostic_production_weight_evaluations']=0
capture_current=full_current_v5();capture_current['sampler']['actual_source_association']['query_capture_success']=False
check('v5-capture-failures-have-zero-mirror-kernel-calls',lambda:ns['project_full_support_v5'](capture,capture_current))
absent=dict(candidate_present=False,meal_matched=False,reason='second root not in actual observed support')
newraw=(pre+'CUE_HISTORICAL_SECOND_TARGET '+json.dumps(absent)+'\nCURRENT_OPPORTUNITY_V3 '+json.dumps(full_current_v5())+'\nCURRENT_SUPPORT_FIRST_REJECTION_V5 '+json.dumps(support_v5())+'\n'+tail).encode()
check('v5-complete-report-valid',lambda:ns['v5_test_report'](newraw))
check('v5-missing-new-line-rejected',lambda:reject(lambda:ns['v5_test_report']((pre+'CUE_HISTORICAL_SECOND_TARGET '+json.dumps(absent)+'\nCURRENT_OPPORTUNITY_V3 '+json.dumps(full_current_v5())+'\n'+tail).encode())))
check('v5-duplicate-new-line-rejected',lambda:reject(lambda:ns['v5_test_report'](newraw.replace(b'CURRENT_SUPPORT_FIRST_REJECTION_V5 ',b'CURRENT_SUPPORT_FIRST_REJECTION_V5 {}\nCURRENT_SUPPORT_FIRST_REJECTION_V5 ',1))))
check('v5-private-tail-still-rejected',lambda:reject(lambda:ns['v5_test_report'](newraw+b'PRIVATE')))
check('v5-raw-hard-cap-still-rejected',lambda:reject(lambda:ns['v5_test_report'](b'x'*65537)))
check('v5-eight-source-overlay-members',lambda:eq(len(overlay),8))
check('v5-two-new-source-files',lambda:eq(len(manifest['graded_files']),829))
check('v5-diagnostic-source-test-only',lambda:eq(overlay['crates/alife_training/src/cpu/canonical_cue_v5_diagnostic.rs'].decode().count('#![cfg(test)]'),1))
check('v5-aggregate-is-appended-test-module',lambda:eq('#[cfg(test)]\n#[path="joint_episode_v5_diagnostic.rs"]\nmod diagnostic_v5;'in overlay['crates/alife_training/src/cpu/motor_mediation/confidence/policy/optional_vocal/joint_episode.rs'].decode(),True))
check('v5-no-extra-world-step-call',lambda:eq(fixture.count('grouped_policy_step_observed::<ScriptedJointMechanismPolicy>'),3))
check('v5-pure-diagnostic-in-fixture-once',lambda:eq(fixture.count('diagnose_full_support_v5'),1))
check('v5-full-original-row-identity-bound',lambda:eq(all(t in overlay['crates/alife_training/src/cpu/motor_mediation/confidence/policy/optional_vocal/joint_episode_v5_diagnostic.rs'].decode()for t in ['a.representative != b.representative','a.motors != b.motors','a.operation != b.operation','actual.binding != binding','actual.policy != JOINT_OPTIONAL_POLICY_V2','actual.bundle_log_probability_bits != selected_bundle_mass.ln().to_bits()']),True))
check('v5-legacy-report-body-literal',lambda:eq(json.loads((ROOT/'FUNCTION-FREEZE.json').read_bytes())['legacy_report_body_literal_v4'],True))
invented=support_v5();invented['probabilities']['max_abs_row_on_off']=.01
check('v5-all-null-invented-probability-effect-rejected',lambda:reject(lambda:ns['project_full_support_v5'](invented,full_current_v5())))
check('v5-selected-null-not-overwritten-by-full-zero-claim',lambda:reject(lambda:ns['project_full_support_v5'](zero,full_current_v5())))
partial=support_v5();partial['value_counts']={'none':1310,'negative':0,'zero':0,'positive':20};partial['second_root_eat_rows']=40;partial['second_root_eat_matching_rows']=30;partial['second_root_eat_nonzero_rows']=20
for i,row in enumerate(partial['sources']):
 row.update(matched_rows=20,positive_rows=20 if i==0 else 0,negative_rows=20 if i==1 else 0,second_root_eat_matched_rows=20,first_rejections=[dict(role=None,reason='Matched',count=20),dict(role=None,reason='RoleCount',count=1310)])
partial_current=full_current_v5();partial_current['sampler']['second_root_candidate_present']=True
check('v5-eat-matches-cannot-exceed-full-union',lambda:reject(lambda:ns['project_full_support_v5'](partial,partial_current)))
partial2=copy.deepcopy(partial);partial2['value_counts']={'none':1310,'negative':0,'zero':10,'positive':10};partial2['second_root_eat_matching_rows']=20;partial2['second_root_eat_nonzero_rows']=15
check('v5-eat-nonzero-cannot-exceed-full-nonzero',lambda:reject(lambda:ns['project_full_support_v5'](partial2,partial_current)))
freeze=json.loads((ROOT/'SOURCE-FREEZE.json').read_bytes())
for row in freeze['production_prefixes']:
 data=overlay[row['path']];prefix=data[:row['prefix_bytes']];appendix=data[row['prefix_bytes']:]
 check('v5-literal-production-prefix-'+row['path'].split('/')[-1],lambda b=prefix,r=row:eq(hashlib.sha256(b).hexdigest(),r['prefix_sha256']))
 check('v5-test-only-appendix-'+row['path'].split('/')[-1],lambda b=appendix,r=row:eq(hashlib.sha256(b).hexdigest(),r['appendix_sha256']))
 assert appendix.startswith(b'\n#[cfg(test)]\n#[path=')and appendix.endswith(b'mod diagnostic_v5;\n')
fx=freeze['fixture'];restored_fixture=overlay[fx['path']].replace(fx['administrative_appendix'].encode(),b'').replace(('fn '+fx['registration_after']+'()').encode(),('fn '+fx['registration_before']+'()').encode())
check('v5-entire-original-fixture-restores-exactly',lambda:eq(hashlib.sha256(restored_fixture).hexdigest(),fx['before']['sha256']))
changed=set(freeze['changed_existing_source_files']);added=set(freeze['new_source_files']);unchanged=[r for r in manifest['graded_files']if r['path']not in changed|added]
check('v5-all824-other-sources-literal-v4',lambda:eq(len(unchanged),824))
check('v5-all824-other-source-inventory-digest',lambda:eq(hashlib.sha256((json.dumps(unchanged,sort_keys=True,separators=(',',':'))+'\n').encode()).hexdigest(),freeze['unchanged_existing_source_rows_sha256']))
def rust_function_v5(data,start):
 text=data.decode();begin=text.index(start);opening=text.index('{',begin);depth=0
 for i in range(opening,len(text)):
  if text[i]=='{':depth+=1
  if text[i]=='}':
   depth-=1
   if depth==0:return text[begin:i+1]
 raise AssertionError('Unbalanced Rust function')
review=json.loads((ROOT/'MATCHER-GATE-ORDER.json').read_bytes());production=rust_function_v5(overlay[review['production_path']],'fn canonical_weight_v3(');mirror=rust_function_v5(overlay[review['mirror_path']],'fn first_rejection_v5(')
normalize=lambda s:re.sub(r'\s+','',s)
pn,mn=normalize(production),normalize(mirror);prevp=prevm=-1
for row in review['gates']:
 pf,mf=normalize(row['production_fragment']),normalize(row['mirror_fragment']);ip=pn.index(pf,prevp+1);im=mn.index(mf,prevm+1);assert ip>prevp and im>prevm;prevp=ip+len(pf)-1;prevm=im+len(mf)-1
check('v5-all27-first-gates-source-ordered',lambda:eq(len(review['gates']),27))
for fragment in review['additional_exact_shared_fragments_verified']:
 assert normalize(fragment)in pn and normalize(fragment)in mn
check('v5-distance-arithmetic-and-velocity-literal',lambda:eq(len(review['additional_exact_shared_fragments_verified']),5))
check('v5-reviewed-mirror-source-hash',lambda:eq(hashlib.sha256(overlay[review['mirror_path']]).hexdigest(),review['mirror_sha256']))
aggregate=overlay['crates/alife_training/src/cpu/motor_mediation/confidence/policy/optional_vocal/joint_episode_v5_diagnostic.rs'].decode()
check('v5-exactly-two-new-support-enumerations',lambda:eq([aggregate.count('self.rows(actual.temperature())?'),aggregate.count('self.rows_with_reader(actual.temperature(), CueReaderV3::Off)?'),aggregate.count('self.sample(')],[1,1,0]))
check('v5-no-new-registered-tests-in-diagnostics',lambda:eq(sum(b.count(b'#[test]')for path,b in overlay.items()if path.endswith('_v5_diagnostic.rs')),0))

check('v6-prior-result-payload-absent',lambda:eq((ROOT/'V4-QUALIFIED-RESULT.json').exists(),False))
check('v6-prior-provenance-minimal',lambda:eq(set(json.loads((ROOT/'V4-PROVENANCE.json').read_bytes())),{'run_url','activation_commit','content_sha256'}))
check('v6-scientific-projector-literal-v5',lambda:eq(json.loads((ROOT/'FUNCTION-FREEZE.json').read_bytes())['scientific_projection_literal_v5'],True))
check('v6-eight-source-overlay-exact',lambda:eq(hashlib.sha256(get('canonical-targetless-source-overlay.tar.gz')).hexdigest(),'d56c6626e2b019f213b7067b215a9f31b86d0499be7feddc18416d31109e1938'))
print(json.dumps({'schema':'canonical-targetless-source-only-static-results-v6','passed':passed,'count':len(passed),'native_invocations':0,'world_intervals':0,'source_forwards':0,'source_only':True},sort_keys=True))
