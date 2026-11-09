"""Source/protocol checks only. Never starts a child process."""
import importlib.util,pathlib,subprocess,json,copy,hashlib,tarfile
ROOT=pathlib.Path(__file__).resolve().parent
subprocess.Popen=lambda *a,**k: (_ for _ in ()).throw(AssertionError('No child process allowed'))
spec=importlib.util.spec_from_file_location('runner',ROOT/'run_hosted.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
passed=[]
def check(name,fn):fn();passed.append(name)
def reject(fn):
 try:fn()
 except m.Stop:return
 raise AssertionError('Expected rejection')
def eq(a,b):assert a==b
check('safe-paths',lambda:eq([m.safe_path(p) for p in ['a/b.rs','../x','/tmp/x','a//b']],[True,False,False,False]))
check('exact-one-inventory',lambda:m.test_listing((m.TEST_NAME+': test\n\n1 test, 0 benchmarks\n').encode(),m.TEST_NAME))
check('extra-test-rejected',lambda:reject(lambda:m.test_listing((m.TEST_NAME+': test\nextra: test\n\n2 tests, 0 benchmarks\n').encode(),m.TEST_NAME)))
pre='\nrunning 1 test\ntest '+m.TEST_NAME+' ... '
tail='ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s\n\n'
h={'candidate_present':False,'meal_matched':False,'reason':'second root not in actual observed support'}
raw=(pre+'CUE_HISTORICAL_SECOND_TARGET '+json.dumps(h)+'\n'+tail).encode()
check('valid-absent-support-miss',lambda:eq(m.test_report(raw)['historical_query']['candidate_present'],False))
check('assertion-text-rejected',lambda:reject(lambda:m.test_report(raw+b'private assertion payload')))
check('report-hard-cap',lambda:reject(lambda:m.test_report(b'x'*65537)))
rows=[]
for i in (1,2):rows.append(dict(sequence=i,source_frame_bytes=40000,frame_budget_bytes=32768,witness_retained=False,stored_capture_success=False,rebuild_capture_success=False,rebuild_capture_error='PRIVATE ERROR',original_self_match=None,pair_match=None,pair_rejections=None,signed_joint_valence=-.2,source_consumed=i==2))
r=dict(query_capture_success=True,query_capture_error=None,records=rows,future_grip='Unknown',future_consumption='Unknown',scope='historical-frame diagnostic, no policy dispatch')
check('error-text-projected-away',lambda:eq('PRIVATE ERROR' in json.dumps(m.project_reason(r)),False))
check('signed-valence-preserved',lambda:eq(m.project_reason(r)['records'][1]['signed_joint_valence'],-.2))
bad=copy.deepcopy(r);bad['records'][1]['pair_rejections']=['arbitrary private string']
check('nonallowlisted-reason-rejected',lambda:reject(lambda:m.project_reason(bad)))
bad2=copy.deepcopy(r);bad2['records'][0]['signed_joint_valence']=float('nan')
check('nonfinite-rejected',lambda:reject(lambda:m.project_reason(bad2)))
manifest=json.loads((ROOT/'source-manifest.json').read_bytes())
for filename,key in [('graded-source-overlay.tar.gz','graded_overlay_files'),('nominal-frozen-input-residual.tar.gz','residual_files')]:
 check(filename+'-closure',lambda f=filename,k=key:m.overlay_members((ROOT/f).read_bytes(),{r['path']:r for r in manifest[k]}))
check('source-counts',lambda:eq([len(manifest[k]) for k in ['base_files','graded_files','residual_files']],[799,823,5]))
check('source-only-residual',lambda:eq(all(r['path'].endswith(('.rs','.toml'))for r in manifest['residual_files']),True))
print(json.dumps({'schema':'source-only-static-checks-v1','passed':passed,'count':len(passed),'native_invocations':0,'world_intervals':0},sort_keys=True))
