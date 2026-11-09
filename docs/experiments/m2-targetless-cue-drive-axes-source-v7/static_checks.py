"""Pure source/projection controls. Never invokes Rust, neural code, or world paths."""
import ast,copy,hashlib,io,json,pathlib,re,struct,tarfile
ROOT=pathlib.Path(__file__).resolve().parent
if 'PACKET_BYTES' not in globals():
    PACKET_BYTES={p.relative_to(ROOT).as_posix():p.read_bytes()for p in ROOT.rglob('*')if p.is_file()}
p=PACKET_BYTES
source=p['run_hosted.py'].decode();tree=ast.parse(source)
selected={'Stop','fail','sha','strict_number','nullable_bool','project_reason','project_meal_v2','project_sources_v3','project_current_v3','_test_report_v4','project_full_support_v5','_test_report_v6','project_drive_axes_v7','test_report','test_listing','validate_activation','validate_tree_preservation','overlay_members','safe_path'}
nodes=[n for n in tree.body if isinstance(n,(ast.FunctionDef,ast.ClassDef))and n.name in selected]
assert {n.name for n in nodes}==selected
ns={'json':json,'re':re,'hashlib':hashlib,'io':io,'tarfile':tarfile,'pathlib':pathlib,'CODES':{'TEST_PROTOCOL','OUTPUT_LIMIT','INTERNAL','BINDING','ADMISSION','SOURCE_CLOSURE'}}
for node in tree.body:
    if isinstance(node,ast.Assign)and len(node.targets)==1 and isinstance(node.targets[0],ast.Name)and node.targets[0].id in {'PLAN','MAIN','BASE','TEST_NAME'}:
        ns[node.targets[0].id]=ast.literal_eval(node.value)
ns.update(BRANCH='refs/heads/qualification/'+ns['PLAN'],PACKET='docs/experiments/'+ns['PLAN']);ns['MARKER']=ns['PACKET']+'/ACTIVATE.json'
exec(compile(ast.Module(body=nodes,type_ignores=[]),'<pure-runner-projections>','exec'),ns)
passed=[]
def check(name,fn):fn();passed.append(name)
def eq(a,b):assert a==b,(a,b)
def rejected(fn):
    try:fn()
    except ns['Stop']:return
    raise AssertionError('Expected strict protocol rejection')
# Synthetic protocol fixtures, not copied creature/runtime data.
def context():
    a={'schema':'canonical-targetless-source-readouts-v3','query_capture_success':True,'sources':[dict(sequence=i+1,source_consumed=i==1,signed_joint_valence=.2,canonical_source=True,v3_weight=None)for i in range(2)],'v3_joint_value':None,'cold_equal':True,'cross_load_rejected':True,'future_grip':'Unknown','future_consumption':'Unknown'}
    rows=[dict(reader=name,support_count=1330,selected_index=0,selected_primitives=[[4,104]],selected_eat=False,same_bundle_as_on=True,bundle_probability=.5,selected_joint_log_probability=-.7,selected_potential=0.,second_root_eat_probability=0.,probability_sum=1.)for name in ['v3_on','off','strict_v1','graded_v1','meal_v2']]
    return dict(sampler=dict(schema='canonical-targetless-current-opportunity-v3',comparison='same-real-source-frame-rng-readers-one-committed-path',source_dispatch_generation=3,memory_last_sequence=2,decision_sequence=3,frame_tick=242,seed=123,counter=2,scripted=False,second_root_candidate_present=False,readouts=rows,actual_source_association=a,future_grip='Unknown',future_consumption='Unknown'),third_source_consumed=False,third_joint_valence=-.1,committed_paths=1,registered_intervals=3,source_forwards=3,counter_deltas=[3]*5,final_memory_cold_equal=True)
def support():
    c=context();s=c['sampler'];n=1330
    rows=[dict(sequence=i+1,source_consumed=i==1,signed_joint_valence=.2,matched_rows=0,negative_rows=0,zero_rows=0,positive_rows=0,second_root_eat_matched_rows=0,first_rejections=[dict(role=None,reason='Drives',count=n)])for i in range(2)]
    return dict(schema='canonical-targetless-full-support-first-rejection-v5',support_kind='production-rows-including-physical-bundle-aliases',support_count=n,unique_bundle_count=800,original_source_count=2,query_capture_success_count=n,value_counts=dict(none=n,negative=0,zero=0,positive=0),second_root_eat_rows=0,second_root_eat_matching_rows=0,second_root_eat_nonzero_rows=0,sources=rows,probabilities=dict(max_abs_row_on_off=0.,max_abs_bundle_on_off=0.,nominal_kl_on_off=-1e-12,kl_roundoff_tolerance=1e-9,sum_on=1.,sum_off=1.,on_probability_underflow_rows=0,off_probability_underflow_rows=0),administrative_dose=dict(additional_support_enumerations=2,bundle_queries=n,paired_source_admission_queries=2*n,mirror_weight_equality_checks=2*n,diagnostic_production_weight_evaluations=4*n,production_value_equality_checks=n,row_potential_equality_checks=n,extra_native_forwards=0,extra_world_intervals=0,alternate_committed_paths=0),source_dispatch_generation=3,memory_last_sequence=2,decision_sequence=3,frame_tick=242,seed=s['seed'],counter=s['counter'],source_archive_unchanged=True,future_grip='Unknown',future_consumption='Unknown')
names=('hunger','fatigue','fear','pain','loneliness','curiosity','brain_atp','temperature_stress','reproductive_drive','extension_0','extension_1')
def f32(bits):return struct.unpack('<f',struct.pack('<I',bits))[0]
def axes(bits=0x3e19999b,index=7):
    rows=[dict(sequence=i+1,source_tick=240+i,source_consumed=i==1,reject_mask=1<<index,reject_count=1,first_failing_axis_index=index,first_failing_axis_name=names[index],violating_axes=[dict(axis_index=index,axis_name=names[index],source_minus_query_f32=f32(bits),delta_f32_bits=bits)])for i in range(2)]
    return dict(schema='canonical-targetless-drive-axis-rejections-v7',axis_schema='DriveSnapshot.to_array/v1/11',threshold_f32_bits=0x3e19999a,frame_tick=242,decision_sequence=3,memory_last_sequence=2,source_dispatch_generation=3,original_source_count=2,sources=rows,administrative_dose=dict(frozen_query_drive_array_reads=1,source_context_checks=2,explicit_axis_checks=22,reference_predicate_checks=2,reference_predicate_axis_checks_upper_bound=22,additional_support_enumerations=0,additional_query_captures=0,additional_memory_restores=0,extra_native_forwards=0,extra_world_intervals=0,alternate_committed_paths=0))
def project(a,s=None,c=None):return ns['project_drive_axes_v7'](a,context()if c is None else c,support()if s is None else s)
check('named-axis-positive-next-f32-valid',lambda:project(axes()))
check('named-axis-negative-next-f32-valid',lambda:project(axes(0xbe19999b)))
for i in range(11):check('fixed-axis-'+str(i),lambda i=i:project(axes(index=i)))
for bits in [0x3e19999a,0x3e199999,0xbe19999a,0xbe199999,0x7fc00000,0x7f800000,0x3f800001]:
    check('nonviolating-or-invalid-bits-'+hex(bits),lambda bits=bits:rejected(lambda:project(axes(bits))))
zero=axes();sz=support()
for row in zero['sources']:row.update(reject_mask=0,reject_count=0,first_failing_axis_index=None,first_failing_axis_name=None,violating_axes=[])
for row in sz['sources']:row['first_rejections']=[dict(role=None,reason='RoleCount',count=1330)]
check('zero-mask-valid-with-earlier-rejection',lambda:project(zero,sz))
check('zero-mask-drives-histogram-rejected',lambda:rejected(lambda:project(zero)))
for reason in ['QueryCaptureRejected','SourceNotCanonical','MissingSourceObservation','Owner','RoleCount','Coordination']:
    early=support()
    for row in early['sources']:row['first_rejections']=[dict(role=None,reason=reason,count=1330)]
    check('all-rows-earlier-'+reason,lambda s=early:project(axes(),s))
mixed=support()
for row in mixed['sources']:row['first_rejections']=[dict(role=None,reason='Drives',count=600),dict(role=None,reason='RoleCount',count=730)]
check('partial-earlier-gate-accounting-valid',lambda:project(axes(),mixed))
for name,path,value in [
 ('unknown-key',['private'],1),('schema',['schema'],'x'),('axis-schema',['axis_schema'],'x'),('threshold-f64-bits',['threshold_f32_bits'],0x3e199999),('threshold-bool',['threshold_f32_bits'],True),('stale-query',['frame_tick'],243),('stale-decision',['decision_sequence'],2),('stale-memory',['memory_last_sequence'],3),('stale-generation',['source_dispatch_generation'],2),('source-count',['original_source_count'],True),('sequence',['sources',0,'sequence'],2),('source-tick',['sources',1,'source_tick'],240),('consumed',['sources',0,'source_consumed'],True),('mask-bool',['sources',0,'reject_mask'],True),('mask-high',['sources',0,'reject_mask'],2048),('count-bool',['sources',0,'reject_count'],True),('bad-popcount',['sources',0,'reject_count'],2),('bad-first-index',['sources',0,'first_failing_axis_index'],8),('bad-first-name',['sources',0,'first_failing_axis_name'],'hunger'),('axis-bool',['sources',0,'violating_axes',0,'axis_index'],True),('axis-name',['sources',0,'violating_axes',0,'axis_name'],'fatigue'),('wrong-delta',['sources',0,'violating_axes',0,'source_minus_query_f32'],.2),('nonfinite-delta',['sources',0,'violating_axes',0,'source_minus_query_f32'],float('nan')),('bits-bool',['sources',0,'violating_axes',0,'delta_f32_bits'],True),('hidden-absolute',['sources',0,'violating_axes',0,'source_value'],.9),('extra-forward',['administrative_dose','extra_native_forwards'],1),('extra-restore',['administrative_dose','additional_memory_restores'],1),('extra-enumeration',['administrative_dose','additional_support_enumerations'],1),('bool-dose',['administrative_dose','explicit_axis_checks'],True)]:
    a=axes();obj=a
    for k in path[:-1]:obj=obj[k]
    obj[path[-1]]=value
    check('reject-'+name,lambda a=a:rejected(lambda:project(a)))
for name,fn in [('duplicate-axis',lambda a:a['sources'][0]['violating_axes'].append(copy.deepcopy(a['sources'][0]['violating_axes'][0]))),('missing-axis',lambda a:a['sources'][0]['violating_axes'].clear()),('extra-source',lambda a:a['sources'].append(copy.deepcopy(a['sources'][0]))),('missing-source',lambda a:a['sources'].pop())]:
    a=axes();fn(a);check(name,lambda a=a:rejected(lambda:project(a)))
# Exercise the actual new wrapper calling the literal legacy parser, not a stub.
pre='\nrunning 1 test\ntest '+ns['TEST_NAME']+' ... '
tail='ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s\n\n'
h=dict(candidate_present=False,meal_matched=False,reason='second root not in actual observed support')
def raw_report(a=None):
    diagnostic=support()
    if a is not None:diagnostic['drive_axis_rejection_v7']=a
    return(pre+'CUE_HISTORICAL_SECOND_TARGET '+json.dumps(h)+'\nCURRENT_OPPORTUNITY_V3 '+json.dumps(context())+'\nCURRENT_SUPPORT_FIRST_REJECTION_V5 '+json.dumps(diagnostic)+'\n'+tail).encode()
valid=raw_report(axes());result=ns['test_report'](valid);legacy=ns['_test_report_v6'](raw_report());extension=result.pop('drive_axis_rejection_v7')
check('actual-wrapper-legacy-result-literal',lambda:eq(result,legacy))
check('signed-kl-roundoff-retained',lambda:eq(result['full_support_first_rejection_v5']['probabilities']['nominal_kl_on_off'],-1e-12))
check('wrapper-missing-extension-rejected',lambda:rejected(lambda:ns['test_report'](raw_report())))
check('wrapper-duplicate-keys-rejected',lambda:rejected(lambda:ns['test_report'](valid.replace(b'"threshold_f32_bits":',b'"threshold_f32_bits": 1, "threshold_f32_bits":',1))))
check('wrapper-nonfinite-json-rejected',lambda:rejected(lambda:ns['test_report'](valid.replace(b'"threshold_f32_bits": 1041865114',b'"threshold_f32_bits": NaN',1))))
check('wrapper-overflow-json-rejected',lambda:rejected(lambda:ns['test_report'](valid.replace(b'\"sum_on\": 1.0',b'\"sum_on\": 1e999',1))))
check('wrapper-private-tail-rejected',lambda:rejected(lambda:ns['test_report'](valid+b'PRIVATE')))
oversize=axes();oversize['sources'][0]['violating_axes'][0]['axis_name']='x'*17000
check('combined16k-report-cap-rejected',lambda:rejected(lambda:ns['test_report'](raw_report(oversize))))
check('wrapper-pipe-cap-rejected',lambda:rejected(lambda:ns['test_report'](b'x'*65537)))
# Every enforcement helper and previous projection except explicitly named wrappers stays exact.
frozen=json.loads(p['FUNCTION-FREEZE.json']);functions={n.name:ast.get_source_segment(source,n)for n in tree.body if isinstance(n,(ast.FunctionDef,ast.ClassDef))}
for row in frozen['functions']:check('function-freeze-'+row['name'],lambda r=row:eq(hashlib.sha256(functions[r['name']].encode()).hexdigest(),r['sha256']))
check('only-three-runner-functions-changed',lambda:eq(frozen['changed_functions'],['summary','test_report','validate_activation']))
with tarfile.open(fileobj=io.BytesIO(p['canonical-targetless-source-overlay.tar.gz']),mode='r:gz')as tf:
    members={a.name:tf.extractfile(a).read()for a in tf if a.isfile()}
m=json.loads(p['source-manifest.json']);check('all829-final-source-rows',lambda:eq(len(m['graded_files']),829));check('exact-eight-overlay-members',lambda:eq(len(members),8))
for row in m['canonical_v3_overlay_files']:
    b=members[row['path']];check('overlay-hash-'+row['path'],lambda b=b,r=row:eq((len(b),hashlib.sha256(b).hexdigest()),(r['bytes'],r['sha256'])))
check('no-new-source-files',lambda:eq(m['drive_axes_v7']['new_source_files'],[]))
canon=members['crates/alife_training/src/cpu/canonical_cue_v5_diagnostic.rs'].decode();joint=members['crates/alife_training/src/cpu/motor_mediation/confidence/policy/optional_vocal/joint_episode_v5_diagnostic.rs'].decode()
for fragment in ['let threshold = f32::from_bits(0x3e19999a);','let delta: f32 = source.drives[axis] - query_drives[axis];','"source_minus_query_f32":f64::from(delta)','let reference_reject = source.drives.iter().zip(query_drives).any(|(x,y)| (x-y).abs() > 0.15);','> 8*1024','source_frame.tick() != event.source_tick','a.to_bits() != b.to_bits()']:
    check('source-bound-fragment-'+str(len(passed)),lambda f=fragment:eq(f in canon,True))
check('one-named-readout-call',lambda:eq(joint.count('memory.diagnostic_drive_axes_v7('),1))
check('rust-finite-delta-guard',lambda:eq('if !delta.is_finite() { return Err(invalid()); }'in canon,True))
gate=json.loads(p['MATCHER-GATE-ORDER.json']);check('mirror-entire-file-hash-current',lambda:eq(gate['mirror_sha256'],hashlib.sha256(canon.encode()).hexdigest()))
check('old16k-cap-retained',lambda:eq('> 16*1024'in joint,True))
check('one-existing-memory-restore',lambda:eq(joint.count('JointEpisodeMemoryV1::restore('),1))
fixture=members['crates/alife_training/src/cpu/joint_episode_host_tests.rs'].decode()
check('three-literal-source-step-calls',lambda:eq(fixture.count('grouped_policy_step_observed::<ScriptedJointMechanismPolicy>'),3))
check('fixed240-preparation',lambda:eq('240'in fixture,True))
check('main-leaves1313',lambda:eq(len(json.loads(p['ACCEPTED-MAIN-TREE.json'])['files']),1313))
check('main-workflows15',lambda:eq(sum(r['path'].startswith('.github/workflows/')for r in json.loads(p['ACCEPTED-MAIN-TREE.json'])['files']),15))
if 'OLD_PACKET' in globals():
    oldsource=OLD_PACKET['run_hosted.py'].decode();oldfunc={n.name:ast.get_source_segment(oldsource,n)for n in ast.parse(oldsource).body if isinstance(n,(ast.FunctionDef,ast.ClassDef))}
    for name,body in oldfunc.items():
        if name not in {'summary','test_report','validate_activation'}:check('literal-v6-'+name,lambda n=name,b=body:eq(functions[n],b))
    check('literal-v6-wrapper-body',lambda:eq(functions['_test_report_v6'].replace('def _test_report_v6(','def test_report(',1),oldfunc['test_report']))
if 'OLD_OVERLAY' in globals():
    changes=sorted(k for k in members if members[k]!=OLD_OVERLAY[k]);check('only-three-test-source-files-changed',lambda:eq(changes,sorted(m['drive_axes_v7']['changed_source_files'])))
    path='crates/alife_training/src/cpu/joint_episode_host_tests.rs';check('fixture-only-registration-renamed',lambda:eq(members[path].replace(b'canonical_targetless_three_interval_drive_axis_diagnostic_v7',b'canonical_targetless_three_interval_first_rejection_diagnostic_v5'),OLD_OVERLAY[path]))
STATIC_RESULT={'schema':'named-axis-static-results-v7','passed':passed,'count':len(passed),'native_invocations':0,'world_intervals':0,'source_forwards':0,'source_only':True}
