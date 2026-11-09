"""Pure Python source/protocol controls. No imports of the runner or child process calls."""
import ast,copy,hashlib,io,json,pathlib,re,tarfile
ROOT=pathlib.Path(__file__).resolve().parent
source=(ROOT/'run_hosted.py').read_text();tree=ast.parse(source)
selected={'Stop','fail','sha','safe_path','overlay_members','strict_number','nullable_bool','project_reason','project_meal_v2','project_sources_v3','project_current_v3','test_report','test_listing','validate_activation','validate_tree_preservation'}
nodes=[n for n in tree.body if isinstance(n,(ast.FunctionDef,ast.ClassDef))and n.name in selected]
assert {n.name for n in nodes}==selected
assign={}
for n in tree.body:
 if isinstance(n,ast.Assign)and len(n.targets)==1 and isinstance(n.targets[0],ast.Name)and n.targets[0].id in ['PLAN','MAIN','BASE','TEST_NAME']:
  assign[n.targets[0].id]=ast.literal_eval(n.value)
ns=dict(assign,json=json,re=re,hashlib=hashlib,tarfile=tarfile,io=io,pathlib=pathlib,CODES={'BINDING','SOURCE_CLOSURE','TEST_PROTOCOL','OUTPUT_LIMIT','ADMISSION','INTERNAL'})
ns['BRANCH']='refs/heads/qualification/'+ns['PLAN'];ns['PACKET']='docs/experiments/'+ns['PLAN'];ns['MARKER']=ns['PACKET']+'/ACTIVATE.json'
exec(compile(ast.Module(body=nodes,type_ignores=[]),'<pure-selected-runner-functions>','exec'),ns)
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
marker=dict(schema='m2-targetless-cue-activation-v3',plan_id=ns['PLAN'],main=ns['MAIN'],research_base=ns['BASE'],prepared_head=before,seal_sha256=seal,source_manifest_sha256=src,attempt=1,fixed_disposable_qualification_only=True)
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
check('source-counts',lambda:eq([len(manifest[k])for k in ['base_files','graded_files','canonical_v3_overlay_files']],[799,827,6]))
frozen=json.loads((ROOT/'FUNCTION-FREEZE.json').read_bytes())
for row in frozen['functions']:
 node=next(n for n in tree.body if isinstance(n,(ast.FunctionDef,ast.ClassDef))and n.name==row['name']);assert hashlib.sha256(ast.get_source_segment(source,node).encode()).hexdigest()==row['sha256']
check('inherited-enforcement-function-freeze',lambda:eq(frozen['changed_functions'],['restore_sources','summary','test_report','validate_activation']))
print(json.dumps({'schema':'canonical-targetless-pure-static-results-v3','passed':passed,'count':len(passed),'native_invocations':0,'world_intervals':0,'source_only':True},sort_keys=True))
