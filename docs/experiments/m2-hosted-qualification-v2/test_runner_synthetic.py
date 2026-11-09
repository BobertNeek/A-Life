#!/usr/bin/env python3
"""Pure synthetic runner tests. No subprocess, Cargo, Rust fixture or world runs."""
import copy, importlib.util, json, pathlib, unittest
from unittest.mock import patch
ROOT=pathlib.Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('hosted_runner',ROOT/'run_hosted.py')
R=importlib.util.module_from_spec(spec); spec.loader.exec_module(R)
PATH='crates/alife_world/src/headless.rs'
SOURCE=pathlib.Path('/synthetic/source')
NAMES={'headless::tests::one','headless::tests::two'}
SECRET='SECRET_SENTINEL_do_not_publish_rendered_or_assertion_values'
def row(data,path=PATH): return {'path':path,'bytes':len(data),'sha256':R.sha(data)}
def diagnostic(code='E0308',filename=PATH,line=42,column=7):
    return {'reason':'compiler-message','message':{'level':'error',
        'message':SECRET,'rendered':SECRET,'children':[{'message':SECRET}],
        'code':{'code':code,'explanation':SECRET},'spans':[{
            'is_primary':True,'file_name':filename,'line_start':line,'column_start':column,
            'line_end':99,'column_end':99,'label':SECRET,'text':[{'text':SECRET}],
            'suggested_replacement':SECRET,'expansion':{'span':{'file_name':'/private/'+SECRET}}}]}}
def raw(*objects): return b'\n'.join(json.dumps(o).encode() for o in objects)
class SyntheticTests(unittest.TestCase):
    def stop(self,code,fn,*args):
        with self.assertRaises(R.Stop) as result: fn(*args)
        self.assertEqual(str(result.exception),code)
    def project(self,*objects): return R.project_cargo_errors(raw(*objects),SOURCE,{PATH})
    def test_error_code_and_primary_bound_location_only(self):
        out=self.project(diagnostic())
        self.assertEqual(out['compiler_errors'],[{'code':'E0308','locations':[{'path':PATH,'line':42,'column':7}]}])
        encoded=json.dumps(out)
        for forbidden in [SECRET,'rendered','message','children','label','suggested','expansion','line_end']:
            self.assertNotIn(forbidden,encoded)
    def test_absolute_bound_path_becomes_relative(self):
        self.assertEqual(self.project(diagnostic(filename=str(SOURCE/PATH)))['compiler_errors'][0]['locations'][0]['path'],PATH)
    def test_unsafe_or_unbound_paths_never_leave(self):
        unsafe=['/private/'+SECRET,'/home/runner/.cargo/registry/src/'+SECRET,'../'+PATH,
                str(SOURCE)+'/../private/'+SECRET,PATH.replace('/','\\'),
                './'+PATH,'crates//alife_world/src/headless.rs','file://'+PATH,
                PATH+'\n'+SECRET,PATH+'\x00',PATH+':'+SECRET,'unknown.rs']
        for path in unsafe:
            with self.subTest(path=path):
                out=self.project(diagnostic(filename=path))
                self.assertEqual(out['compiler_errors'],[{'code':'E0308','locations':[]}])
                self.assertNotIn(SECRET,json.dumps(out))
    def test_secondary_spans_never_leave(self):
        d=diagnostic();d['message']['spans'][0]['is_primary']=False
        self.assertEqual(self.project(d)['compiler_errors'][0]['locations'],[])
    def test_unclassified_error_is_static_boolean_only(self):
        d=diagnostic();d['message']['code']=None
        out=self.project(d);self.assertTrue(out['unclassified_error_present']);self.assertEqual(out['compiler_errors'],[])
        self.assertNotIn(SECRET,json.dumps(out))
    def test_warnings_are_not_compiler_errors(self):
        d=diagnostic();d['message']['level']='warning'
        self.assertEqual(self.project(d)['compiler_errors'],[])
    def test_code_and_location_deduplication(self):
        d=diagnostic();d['message']['spans']*=2
        out=self.project(d,d);self.assertEqual(len(out['compiler_errors']),1);self.assertEqual(len(out['compiler_errors'][0]['locations']),1)
    def test_bounded_codes_and_locations(self):
        objects=[diagnostic(code=f'E{i:04}',line=i+1) for i in range(20)]
        out=self.project(*objects);self.assertEqual(len(out['compiler_errors']),12);self.assertTrue(out['compiler_errors_truncated'])
        d=diagnostic();d['message']['spans']=[copy.deepcopy(d['message']['spans'][0]) for _ in range(8)]
        for i,s in enumerate(d['message']['spans']):s['line_start']=i+1
        out=self.project(d);self.assertEqual(len(out['compiler_errors'][0]['locations']),3);self.assertTrue(out['compiler_errors_truncated'])
    def test_compiler_parser_rejects_invalid_json_and_schema(self):
        for data in [b'not-json '+SECRET.encode(),b'\xff',b'[]',raw({'reason':'unknown'}),raw({'reason':'compiler-message','message':SECRET})]:
            with self.subTest(data=data):self.stop('BUILD_PROTOCOL',R.project_cargo_errors,data,SOURCE,{PATH})
    def test_error_code_cannot_transport_message_text(self):
        for code in [SECRET,'E123','E12345','E0308\n'+SECRET,7]:
            self.stop('BUILD_PROTOCOL',R.project_cargo_errors,raw(diagnostic(code=code)),SOURCE,{PATH})
    def test_primary_coordinate_schema_is_strict(self):
        for line,column in [(True,1),(1,False),(0,1),(1,-1),(10_000_001,1),(1,1_000_001),('42',7)]:
            self.stop('BUILD_PROTOCOL',R.project_cargo_errors,raw(diagnostic(line=line,column=column)),SOURCE,{PATH})
    def test_partial_completion_survives_runtime_failure(self):
        context={'variant':'reflex','stage':'fixture','package':'alife_world','test':'headless::tests::two'}
        out=R.failure_summary('TEST_FAILED',context,['headless::tests::one'],NAMES,{PATH})
        self.assertEqual(out['completed_count'],1);self.assertEqual(out['completed_tests'],['headless::tests::one'])
        self.assertEqual(out['test'],'headless::tests::two');self.assertEqual(out['code'],'TEST_FAILED')
    def test_partial_completion_survives_compile_failure(self):
        context={'variant':'reflex','stage':'compile','package':'alife_world',**self.project(diagnostic())}
        out=R.failure_summary('BUILD',context,['headless::tests::one'],NAMES,{PATH})
        self.assertEqual(out['completed_count'],1);self.assertEqual(out['compiler_errors'][0]['code'],'E0308')
        self.assertNotIn(SECRET,R.serialize_summary(out))
    def test_build_failure_projection_is_wired_without_subprocess(self):
        context={'variant':'reflex','stage':'compile','package':'alife_world'}
        with patch.object(R,'SOURCE',SOURCE,create=True),patch.object(R,'BOUND_RECORDS',{PATH:row(b'x')}),patch.object(R,'SAFE_CONTEXT',context),patch.object(R,'run',return_value=(raw(diagnostic()),101)) as fake:
            self.stop('BUILD',R.build,'alife_world')
            self.assertEqual(context['compiler_errors'][0]['code'],'E0308')
            self.assertTrue(fake.call_args.kwargs['allow_nonzero'])
            self.assertNotIn(SECRET,json.dumps(context))
    def test_bad_compile_protocol_drops_partial_projection_but_keeps_completion(self):
        context={'variant':'reflex','stage':'compile','package':'alife_world'}
        payload=raw(diagnostic())+b'\n'+SECRET.encode()
        with patch.object(R,'SOURCE',SOURCE,create=True),patch.object(R,'BOUND_RECORDS',{PATH:row(b'x')}),patch.object(R,'SAFE_CONTEXT',context),patch.object(R,'run',return_value=(payload,101)):
            self.stop('BUILD_PROTOCOL',R.build,'alife_world')
            self.assertNotIn('compiler_errors',context)
            out=R.failure_summary('BUILD_PROTOCOL',context,['headless::tests::one'],NAMES,{PATH})
            self.assertEqual(out['completed_count'],1);self.assertNotIn(SECRET,R.serialize_summary(out))
    def test_unpinned_names_and_unknown_context_fail_closed(self):
        cases=[({'variant':'reflex','message':SECRET},[]),({'test':SECRET},[]),({},[SECRET]),({},[{}]),
               ({},['headless::tests::one']*2),({'stage':SECRET},[]),({'variant':[]},[])]
        for ctx,done in cases:
            out=R.failure_summary('TEST_FAILED',ctx,done,NAMES,{PATH})
            self.assertEqual(out['code'],'INTERNAL');self.assertNotIn(SECRET,R.serialize_summary(out))
    def test_failure_renderer_rechecks_projected_rows(self):
        d={'code':'E0308','locations':[{'path':'/private/'+SECRET,'line':1,'column':1}]}
        out=R.failure_summary('BUILD',{'stage':'compile','compiler_errors':[d]},[],NAMES,{PATH})
        self.assertEqual(out['code'],'INTERNAL');self.assertNotIn(SECRET,R.serialize_summary(out))
    def test_final_serialization_is_utf8_bounded_and_nan_closed(self):
        self.assertEqual(R.serialize_summary({'code':'OK','data':'é'*32768}),'{"code":"OUTPUT_LIMIT"}')
        self.assertEqual(R.serialize_summary({'code':'OK','data':float('nan')}),'{"code":"INTERNAL"}')
    def test_list_parser_malformed_no_summary_or_duplicate_rejects(self):
        for data in [b'',b'\xff',b'a: test\n',b'a: test\n0 tests, 0 benchmarks\n',b'a: test\na: test\n2 tests, 0 benchmarks\n',b'private '+SECRET.encode()]:
            self.stop('LIST_PROTOCOL',R.list_names,data)
        self.assertEqual(R.list_names(b'a: test\n\n1 test, 0 benchmarks\n'),['a'])
    def test_test_parser_zero_ignored_failed_or_extra_output_rejects(self):
        name='headless::tests::one'
        good=f'\nrunning 1 test\ntest {name} ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.01s\n'.encode()
        R.test_result(good,name)
        for data in [b'\xff',good.replace(b'1 passed',b'0 passed'),good.replace(b'0 ignored',b'1 ignored'),good.replace(b'0 failed',b'1 failed'),good+SECRET.encode(),good.replace(b'0.01s',b'0..01s')]:
            self.stop('TEST_PROTOCOL',R.test_result,data,name)
    def test_source_bytes_hash_and_length_mismatches_reject(self):
        good=b'source-owned synthetic bytes\n';record=row(good)
        R.verify_bytes(good,record)
        self.stop('BINDING',R.verify_bytes,good+b'x',record)
        self.stop('BINDING',R.verify_bytes,good,{**record,'sha256':'0'*64})
        self.stop('BINDING',R.verify_bytes,good,{**record,'bytes':True})
    def test_source_record_path_traversal_is_rejected_before_io(self):
        for name in ['../secret','/private/secret','./file','a//b','a\\b']:
            self.stop('BINDING',R.verify_record,ROOT,row(b'x',name))
    def test_seal_mismatch_and_bound_file_mismatch_reject(self):
        content=b'source\n';record=row(content);seal=json.dumps({'files':[record]}).encode()
        with patch.object(pathlib.Path,'read_bytes',return_value=seal),patch.object(R,'verify_record') as verify:
            R.verify_seal(ROOT,R.sha(seal));verify.assert_called_once()
            self.stop('BINDING',R.verify_seal,ROOT,'0'*64)
        with patch.object(pathlib.Path,'read_bytes',return_value=seal),patch.object(R,'verify_record',side_effect=lambda root,row:R.verify_bytes(b'changed',row)):
            self.stop('BINDING',R.verify_seal,ROOT,R.sha(seal))
    def test_three_part_transport_order_hash_and_newline_checks(self):
        data=[b'one\n',b'two\n',b'three\n']; parts=[row(d,f'patches/main-to-fbf-library-closure.part-{i:02d}.patch') for i,d in enumerate(data,1)]
        meta={'transport':'utf8-line-parts-v1','parts':parts,**{k:v for k,v in row(b''.join(data)).items() if k!='path'}}
        lookup={p['path']:d for p,d in zip(parts,data)}
        def read(path):return lookup[str(path.relative_to(ROOT))]
        with patch.object(pathlib.Path,'read_bytes',read),patch.object(R,'verify_record',side_effect=lambda root,r:R.verify_bytes(lookup[r['path']],r)):
            self.assertEqual(R.assemble_closure(ROOT,meta),b''.join(data))
            self.stop('BINDING',R.assemble_closure,ROOT,{**meta,'sha256':'0'*64})
            swapped=copy.deepcopy(meta);swapped['parts'][0],swapped['parts'][1]=swapped['parts'][1],swapped['parts'][0]
            self.stop('BINDING',R.assemble_closure,ROOT,swapped)
            lookup[parts[0]['path']]=b'changed\n';self.stop('BINDING',R.assemble_closure,ROOT,meta)
        for bad in [b'one',b'bad\xff\n',b'bad\x00\n']:
            changed=copy.deepcopy(meta);changed['parts'][0]=row(bad,parts[0]['path']);lookup={p['path']:d for p,d in zip(parts,data)};lookup[parts[0]['path']]=bad
            with patch.object(pathlib.Path,'read_bytes',read),patch.object(R,'verify_record',side_effect=lambda root,r:R.verify_bytes(lookup[r['path']],r)):
                self.stop('BINDING',R.assemble_closure,ROOT,changed)
    def activation(self):
        before='1'*40;head='2'*40;seal='3'*64;fixtures='4'*64
        env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':R.BRANCH,'GITHUB_RUN_ATTEMPT':'1'}
        event={'before':before,'after':head,'ref':R.BRANCH,'created':False,'deleted':False,'forced':False}
        marker={'schema':'m2-hosted-activation-v1','plan_id':R.PLAN_ID,'prepared_head':before,'main':R.MAIN,'seal_sha256':seal,'fixture_list_sha256':fixtures,'variants':['boredom','reflex']}
        return [env,event,head,[head,before],'A\t'+R.MARKER_PATH+'\n',marker,seal,fixtures]
    def test_exact_marker_activation_admitted(self):
        args=self.activation();self.assertEqual(R.validate_activation(*args),'1'*40)
    def test_activation_environment_edge_cases(self):
        for key,value in [('GITHUB_ACTIONS','false'),('GITHUB_EVENT_NAME','workflow_dispatch'),('GITHUB_REPOSITORY','other/repo'),('GITHUB_REF','refs/heads/main'),('GITHUB_RUN_ATTEMPT','2')]:
            args=self.activation();args[0][key]=value;self.stop('ADMISSION',R.validate_activation,*args)
    def test_creation_forced_delete_and_nonboolean_flags_reject(self):
        for key in ['created','deleted','forced']:
            for value in [True,0,None,'false']:
                args=self.activation();args[1][key]=value;self.stop('ADMISSION',R.validate_activation,*args)
    def test_missing_flag_wrong_head_or_zero_parent_reject(self):
        for field,value in [('before','0'*40),('before','malformed'),('after','5'*40),('ref','refs/heads/main')]:
            args=self.activation();args[1][field]=value;self.stop('ADMISSION',R.validate_activation,*args)
        args=self.activation();del args[1]['forced'];self.stop('ADMISSION',R.validate_activation,*args)
    def test_parent_merge_marker_edit_delete_or_extra_path_reject(self):
        for parents in [[],['2'*40],['2'*40,'6'*40],['2'*40,'1'*40,'6'*40]]:
            args=self.activation();args[3]=parents;self.stop('ADMISSION',R.validate_activation,*args)
        for changed in ['','M\t'+R.MARKER_PATH+'\n','D\t'+R.MARKER_PATH+'\n','A\t'+R.MARKER_PATH+'\nM\tCargo.lock\n']:
            args=self.activation();args[4]=changed;self.stop('ADMISSION',R.validate_activation,*args)
    def test_marker_source_seal_plan_fixture_or_disabled_variant_mismatch(self):
        for key,value in [('prepared_head','5'*40),('main','5'*40),('seal_sha256','5'*64),('fixture_list_sha256','5'*64),('plan_id','old-plan'),('variants',['boredom','reflex','meal']),('variants',['gds']),('extra',SECRET)]:
            args=self.activation();args[5][key]=value;self.stop('ADMISSION',R.validate_activation,*args)
    def test_bad_seal_digest_encoding_rejects(self):
        args=self.activation();args[6]=SECRET;self.stop('BINDING',R.validate_activation,*args)

if __name__=='__main__':
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(SyntheticTests)
    result=unittest.TestResult();suite.run(result)
    # No synthetic payload or assertion text is printed, even if a self-test fails.
    report={'schema':'m2-runner-pure-synthetic-tests-v2','cases':result.testsRun,
            'passed':result.testsRun-len(result.errors)-len(result.failures),
            'failed_cases':[getattr(getattr(t,'test_case',t),'_testMethodName','synthetic_case') for t,_ in result.errors+result.failures],
            'subprocess_invocations':0,'native_builds':0,'native_fixtures':0,'world_ticks':0}
    (ROOT/'SYNTHETIC-TESTS.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report));raise SystemExit(0 if result.wasSuccessful() else 1)
