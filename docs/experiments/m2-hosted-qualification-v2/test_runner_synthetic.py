#!/usr/bin/env python3
"""Pure synthetic runner tests. No subprocess, Cargo, Rust fixture or world runs."""
import ast, contextlib, copy, importlib.util, json, pathlib, unittest
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
        with patch.object(R,'RUSTUP','/synthetic/rustup',create=True),patch.object(R,'SOURCE',SOURCE,create=True),patch.object(R,'BOUND_RECORDS',{PATH:row(b'x')}),patch.object(R,'SAFE_CONTEXT',context),patch.object(R,'run',return_value=(raw(diagnostic()),101)) as fake:
            self.stop('BUILD',R.build,'alife_world')
            self.assertEqual(context['compiler_errors'][0]['code'],'E0308')
            self.assertTrue(fake.call_args.kwargs['allow_nonzero'])
            self.assertNotIn(SECRET,json.dumps(context))
    def test_bad_compile_protocol_drops_partial_projection_but_keeps_completion(self):
        context={'variant':'reflex','stage':'compile','package':'alife_world'}
        payload=raw(diagnostic())+b'\n'+SECRET.encode()
        with patch.object(R,'RUSTUP','/synthetic/rustup',create=True),patch.object(R,'SOURCE',SOURCE,create=True),patch.object(R,'BOUND_RECORDS',{PATH:row(b'x')}),patch.object(R,'SAFE_CONTEXT',context),patch.object(R,'run',return_value=(payload,101)):
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

    def rustc_verbose(self):
        return (R.RUST_VERSION+'\nbinary: rustc\ncommit-hash: '+R.RUST_COMMIT+
                '\ncommit-date: '+R.RUST_DATE+'\nhost: '+R.RUST_HOST+'\nrelease: '+R.TOOLCHAIN+'\nLLVM version: 23.1.1\n').encode()
    def toolchain_facts(self):
        return {**R.parse_rustc_version(self.rustc_verbose()),'cargo_version':'cargo 1.99.1 (5f94df478 2026-08-28)',
                'rustc_binary_sha256':'a'*64,'cargo_binary_sha256':'b'*64,'selected_sysroot_bytes':1234,
                'platform_toolchain_storage':'separate-normal-hosted-home','action_commit':R.ACTION_COMMIT}
    @contextlib.contextmanager
    def fake_discovery(self,fail_command=None,bad_home=False,bad_sysroot=False,bad_version=False,bad_cargo=False,bad_provenance=False,hash_failure=False):
        home=pathlib.Path('/platform/.rustup');sysroot=home/'toolchains'/('1.99.1-'+R.RUST_HOST)
        context={};calls=[];action=b'synthetic official action recipe'
        def fake_run(args,code,**kwargs):
            calls.append((args,dict(R.ENV)))
            if code==fail_command: R.fail(code)
            if args[1:]==['show','home']: return (b'../private\n' if bad_home else str(home).encode()+b'\n')
            if args[1:3]==['toolchain','install']: return SECRET.encode()
            if args[-2:]==['--print','sysroot']: return (b'/private/unapproved\n' if bad_sysroot else str(sysroot).encode()+b'\n')
            if args[-2:]==['--version','--verbose']: return (SECRET.encode() if bad_version else self.rustc_verbose())
            if args[-2:]==['cargo','--version']: return (SECRET.encode() if bad_cargo else b'cargo 1.99.1 (5f94df478 2026-08-28)\n')
            raise AssertionError('unexpected synthetic command')
        with contextlib.ExitStack() as stack:
            for name,value in [('WORK',pathlib.Path('/work')),('PRIVATE',pathlib.Path('/work/private')),('CHECKOUTS',pathlib.Path('/checkouts')),('RUSTUP','rustup'),('PLATFORM_HOME',None),('TOOLCHAIN_ROOT',None),('TOOLCHAIN_BYTES',0),('ENV',{}),('SAFE_CONTEXT',context),('ACTION_SHA256',R.sha(action) if not bad_provenance else '0'*64)]:
                stack.enter_context(patch.object(R,name,value,create=True))
            stack.enter_context(patch.object(R.shutil,'which',return_value='/platform/bin/rustup'))
            stack.enter_context(patch.object(pathlib.Path,'resolve',lambda p,**kw:p))
            stack.enter_context(patch.object(pathlib.Path,'is_file',return_value=True))
            stack.enter_context(patch.object(pathlib.Path,'is_dir',return_value=True))
            stack.enter_context(patch.object(pathlib.Path,'is_symlink',return_value=False))
            stack.enter_context(patch.object(pathlib.Path,'read_bytes',return_value=action))
            stack.enter_context(patch.object(R.os,'access',return_value=True))
            stack.enter_context(patch.object(R,'bytes_used',return_value=1234))
            stack.enter_context(patch.object(R,'limits'))
            stack.enter_context(patch.object(R,'file_sha',side_effect=OSError(SECRET) if hash_failure else None,return_value='a'*64))
            stack.enter_context(patch.object(R,'run',side_effect=fake_run))
            yield context,calls
    def test_normal_hosted_home_private_cargo_and_no_untrusted_environment(self):
        original={'HOME':'/platform/home','PATH':'/platform/bin:/usr/bin','RUSTUP_HOME':'/platform/.rustup',
                  'GITHUB_TOKEN':SECRET,'RUSTC_WRAPPER':SECRET,'CARGO_BUILD_RUSTC_WRAPPER':SECRET,
                  'RUSTUP_DIST_SERVER':'https://untrusted.invalid','CARGO_HOME':'/unapproved/cache'}
        env=R.normal_toolchain_env(original,pathlib.Path('/work'),pathlib.Path('/work/private'))
        self.assertEqual(env['HOME'],'/platform/home');self.assertEqual(env['RUSTUP_HOME'],'/platform/.rustup')
        for key,path in [('CARGO_HOME','/work/cargo'),('CARGO_TARGET_DIR','/work/target'),('TMPDIR','/work/private'),('TMP','/work/private'),('TEMP','/work/private')]:self.assertEqual(env[key],path)
        self.assertEqual(env['RUSTUP_DIST_SERVER'],'https://static.rust-lang.org')
        self.assertNotIn(SECRET,json.dumps(env));self.assertNotIn('GITHUB_TOKEN',env)
        del original['RUSTUP_HOME'];self.assertNotIn('RUSTUP_HOME',R.normal_toolchain_env(original,pathlib.Path('/work'),pathlib.Path('/work/private')))
    def test_private_environment_uses_discovered_home_and_real_compiler(self):
        env=R.private_toolchain_env({'PATH':'/platform/bin','HOME':'/platform/home'},pathlib.Path('/work'),pathlib.Path('/platform/.rustup'),pathlib.Path('/platform/toolchain'))
        self.assertEqual(env['HOME'],'/work/home');self.assertEqual(env['RUSTUP_HOME'],'/platform/.rustup')
        self.assertEqual(env['RUSTC'],'/platform/toolchain/bin/rustc');self.assertEqual(env['RUSTDOC'],'/platform/toolchain/bin/rustdoc')
    def test_commands_are_explicit_rustup_run_without_shim_plus_syntax(self):
        with patch.object(R,'RUSTUP','/platform/bin/rustup',create=True):
            self.assertEqual(R.toolchain_command('rustc','--version'),['/platform/bin/rustup','run','1.99.1','rustc','--version'])
            self.assertEqual(R.toolchain_command('cargo','test'),['/platform/bin/rustup','run','1.99.1','cargo','test'])
            self.assertEqual(R.install_command(),['/platform/bin/rustup','toolchain','install','1.99.1','--profile','minimal','--no-self-update','--force-non-host'])
            self.stop('INTERNAL',R.toolchain_command,'sh','-c',SECRET)
    def test_build_uses_discovered_explicit_rustup_command(self):
        with patch.object(R,'RUSTUP','/platform/bin/rustup',create=True),patch.object(R,'SOURCE',SOURCE,create=True),patch.object(R,'SAFE_CONTEXT',{}),patch.object(R,'run',return_value=(raw(diagnostic()),101)) as call:
            self.stop('BUILD',R.build,'alife_training')
            cmd=call.call_args.args[0]
            self.assertEqual(cmd[:4],['/platform/bin/rustup','run','1.99.1','cargo'])
            self.assertIn('--locked',cmd);self.assertEqual(cmd[-2:],['--features','cpu-backend']);self.assertNotIn('+1.99.1',cmd)
    def test_version_projection_exact_official_commit_and_no_raw_text(self):
        facts=R.parse_rustc_version(self.rustc_verbose());self.assertEqual(facts['rustc_commit'],R.RUST_COMMIT)
        for payload in [self.rustc_verbose()+SECRET.encode(),self.rustc_verbose().replace(R.RUST_COMMIT.encode(),b'0'*40),self.rustc_verbose().replace(b'1.99.1',b'1.99.0'),self.rustc_verbose().replace(b'23.1.1',SECRET.encode()),b'\xff']:
            self.stop('TOOLCHAIN_VERSION',R.parse_rustc_version,payload)
        for payload in [SECRET.encode(),b'cargo +1.99.1',b'cargo 1.98.0 (123456789 2026-01-01)',b'cargo 1.99.1 (123456789 2026-01-01) '+SECRET.encode(),b'\xff']:
            self.stop('TOOLCHAIN_CARGO',R.parse_cargo_version,payload)
    def test_sysroot_layout_and_single_path_fail_closed(self):
        home=pathlib.Path('/platform/.rustup');good=home/'toolchains'/('1.99.1-'+R.RUST_HOST)
        self.assertEqual(R.validate_layout(home,good),good)
        for path in ['/private/toolchain',str(home)+'/toolchains/stable-'+R.RUST_HOST,str(home)+'/toolchains/1.99.0-'+R.RUST_HOST]:
            self.stop('TOOLCHAIN_SYSROOT',R.validate_layout,home,pathlib.Path(path))
        for raw_path in [b'../home',b'/home\n'+SECRET.encode(),b'/home/../private',b'/home//rustup',b'/home/./rustup',b'/home\x00',b'\xff',b'/'+b'x'*4096]:
            self.stop('TOOLCHAIN_HOME',R.single_path,raw_path,'TOOLCHAIN_HOME')
    def test_setup_install_in_normal_home_then_private_validation(self):
        with self.fake_discovery() as (context,calls):
            facts=R.discover_toolchain({'HOME':'/platform/home','PATH':'/platform/bin'},install=True)
            self.assertTrue(R.valid_toolchain_facts(facts));self.assertEqual(context['setup_stage'],'ready')
            install=[(a,e) for a,e in calls if a[1:3]==['toolchain','install']];self.assertEqual(len(install),1)
            self.assertEqual(install[0][1]['HOME'],'/platform/home');self.assertEqual(install[0][1]['RUSTUP_HOME'],'/platform/.rustup')
            self.assertEqual(install[0][1]['CARGO_HOME'],'/work/cargo')
            self.assertTrue(all(e['HOME']=='/work/home' for a,e in calls if a[-1]=='--verbose' or a[-2:]==['cargo','--version']))
            self.assertNotIn(SECRET,json.dumps(context));self.assertNotIn('/platform',json.dumps(facts))
    def test_private_runner_discovery_never_installs(self):
        with self.fake_discovery() as (_,calls):
            R.discover_toolchain({'HOME':'/platform/home','PATH':'/platform/bin'},install=False)
            self.assertFalse(any(a[1:3]==['toolchain','install'] for a,e in calls))
    def test_setup_failure_stages_are_specific_and_private(self):
        cases=[('TOOLCHAIN_HOME','rustup-home',{'fail_command':'TOOLCHAIN_HOME'}),
               ('TOOLCHAIN_INSTALL','install',{'fail_command':'TOOLCHAIN_INSTALL'}),
               ('TOOLCHAIN_SYSROOT','sysroot',{'fail_command':'TOOLCHAIN_SYSROOT'}),
               ('TOOLCHAIN_VERSION','rustc-version',{'fail_command':'TOOLCHAIN_VERSION'}),
               ('TOOLCHAIN_CARGO','cargo-version',{'fail_command':'TOOLCHAIN_CARGO'}),
               ('TOOLCHAIN_HOME','rustup-home',{'bad_home':True}),('TOOLCHAIN_SYSROOT','sysroot',{'bad_sysroot':True}),
               ('TOOLCHAIN_VERSION','rustc-version',{'bad_version':True}),('TOOLCHAIN_CARGO','cargo-version',{'bad_cargo':True}),
               ('TOOLCHAIN_PROVENANCE','official-recipe',{'bad_provenance':True}),('TOOLCHAIN_HASH','binary-hashes',{'hash_failure':True})]
        for code,stage,kwargs in cases:
            with self.subTest(code=code,stage=stage),self.fake_discovery(**kwargs) as (context,calls):
                self.stop(code,R.discover_toolchain,{'HOME':'/platform/home','PATH':'/platform/bin'},True)
                out=R.failure_summary(code,context,[],set(),{});self.assertEqual(out['code'],code);self.assertEqual(out['setup_stage'],stage)
                text=R.serialize_summary(out);self.assertNotIn(SECRET,text);self.assertNotIn('/platform',text)
                self.assertEqual(out['completed_count'],0)
    def test_missing_rustup_reports_discovery_not_generic_toolchain(self):
        with patch.object(R.shutil,'which',return_value=None),patch.object(R,'SAFE_CONTEXT',{}) as context:
            self.stop('TOOLCHAIN_RUSTUP',R.discover_toolchain,{'PATH':'/platform/bin','HOME':'/platform/home'})
            self.assertEqual(context['setup_stage'],'rustup-discovery')
    def test_toolchain_failure_facts_are_strict_bounded_and_preserve_completion(self):
        good=self.toolchain_facts();self.assertTrue(R.valid_toolchain_facts(good))
        ctx={'variant':'reflex','stage':'fixture','test':'headless::tests::two','toolchain':good}
        out=R.failure_summary('TEST_FAILED',ctx,['headless::tests::one'],NAMES,{PATH})
        self.assertEqual(out['completed_count'],1);self.assertEqual(out['toolchain'],good)
        for key,value in [('private_path',SECRET),('rustc_version',SECRET),('cargo_binary_sha256',SECRET),('selected_sysroot_bytes',True),('selected_sysroot_bytes',R.TOOLCHAIN_LIMIT+1),('platform_toolchain_storage',SECRET),('llvm_version',[])]:
            bad={**good,key:value};self.assertFalse(R.valid_toolchain_facts(bad))
            out=R.failure_summary('TEST_FAILED',{**ctx,'toolchain':bad},[],NAMES,{PATH});self.assertEqual(out['code'],'INTERNAL');self.assertNotIn(SECRET,R.serialize_summary(out))
    def test_setup_entry_only_admits_and_installs_without_build_or_fixture(self):
        with patch.object(R.sys,'argv',['runner','--setup','boredom','/source']),patch.object(R,'initialize') as init,patch.object(R,'admit',return_value=('a'*40,'b'*40,'c'*64)),patch.object(R,'discover_toolchain',return_value=self.toolchain_facts()) as discovery,patch.object(R,'build') as build,patch.object(R,'run') as run:
            out=R.setup();init.assert_called_once_with('boredom','/source');self.assertTrue(discovery.call_args.kwargs['install'])
            build.assert_not_called();run.assert_not_called();self.assertEqual(out['native_builds'],0);self.assertEqual(out['fixture_cases'],0)
    def test_setup_admission_failure_precedes_install(self):
        with patch.object(R.sys,'argv',['runner','--setup','boredom','/source']),patch.object(R,'initialize'),patch.object(R,'admit',side_effect=R.Stop('ADMISSION')),patch.object(R,'discover_toolchain') as discovery:
            self.stop('ADMISSION',R.setup);discovery.assert_not_called()
    def test_v2_marker_branch_or_plan_cannot_activate_v3(self):
        for index,key,value in [(0,'GITHUB_REF','refs/heads/qualification/m2-fixed-fixtures-v2'),(1,'ref','refs/heads/qualification/m2-fixed-fixtures-v2'),(5,'plan_id','m2-fixed-fixtures-v2')]:
            args=self.activation();args[index][key]=value;self.stop('ADMISSION',R.validate_activation,*args)
        args=self.activation();args[4]='A\tdocs/experiments/m2-hosted-qualification-v2/ACTIVATE.json\n';self.stop('ADMISSION',R.validate_activation,*args)
    def test_resource_floor_covers_platform_home_and_selected_sysroot(self):
        class Usage:
            def __init__(self,free):self.free=free
        def exercise(free_work=3*1024**3,free_platform=3*1024**3,sysroot_bytes=1234):
            work=pathlib.Path('/work');platform=pathlib.Path('/platform');sysroot=platform/'toolchains/selected'
            with patch.object(R,'WORK',work,create=True),patch.object(R,'CHECKOUTS',pathlib.Path('/checkouts'),create=True),patch.object(R,'PLATFORM_HOME',platform),patch.object(R,'TOOLCHAIN_ROOT',sysroot),patch.object(pathlib.Path,'exists',return_value=True),patch.object(R.shutil,'disk_usage',side_effect=lambda p:Usage(free_work if p==work else free_platform)),patch.object(R,'bytes_used',side_effect=lambda p:sysroot_bytes if p==sysroot else 1024):
                R.limits()
        exercise()
        self.stop('DISK_LIMIT',exercise,1,3*1024**3,1234)
        self.stop('DISK_LIMIT',exercise,3*1024**3,1,1234)
        self.stop('DISK_LIMIT',exercise,3*1024**3,3*1024**3,R.TOOLCHAIN_LIMIT+1)

    def test_both_actual_listing_command_expressions_pin_pretty(self):
        module=ast.parse((ROOT/'run_hosted.py').read_text())
        main=next(n for n in module.body if isinstance(n,ast.FunctionDef) and n.name=='main')
        listings=[n for n in ast.walk(main) if isinstance(n,ast.Call) and isinstance(n.func,ast.Name) and n.func.id=='run' and n.args and isinstance(n.args[0],ast.List) and any(isinstance(x,ast.Constant) and x.value=='--list' for x in n.args[0].elts)]
        self.assertEqual(len(listings),2)
        calls=[];name='headless::tests::one'
        def fake_run(args,code):
            calls.append(args);self.assertEqual(code,'LIST_PROTOCOL')
            self.assertEqual(args[args.index('--format')+1],'pretty');self.assertNotIn('terse',args)
            if '--ignored' in args:
                names=sorted(R.IGNORED['alife_training'])
                return ('\n'.join(n+': test' for n in names)+'\n\n5 tests, 0 benchmarks\n').encode()
            self.assertEqual(args[-2:],['--exact',name])
            return (name+': test\n\n1 test, 0 benchmarks\n').encode()
        results=[R.list_names(eval(compile(ast.Expression(n),'<synthetic-command>','eval'),{'run':fake_run,'str':str,'exe':'synthetic-harness','name':name})) for n in listings]
        self.assertTrue(any('--ignored' in c for c in calls));self.assertTrue(any('--exact' in c for c in calls))
        self.assertIn(sorted(R.IGNORED['alife_training']),results);self.assertIn([name],results)
    def test_pretty_selection_footer_count_and_unique_names_stay_strict(self):
        self.assertEqual(R.list_names(b'a: test\n\n1 test, 0 benchmarks\n'),['a'])
        for payload in [b'a: test\n',b'a: test\nb: test\n',b'a: test\na: test\n\n2 tests, 0 benchmarks\n',b'a: test\n\n2 tests, 0 benchmarks\n',b'a: test\n\n0 tests, 0 benchmarks\n']:
            self.stop('LIST_PROTOCOL',R.list_names,payload)
        self.assertNotEqual(R.list_names(b'0 tests, 0 benchmarks\n'),['required_test'])
    def channel_mapping_inputs(self):
        spec=importlib.util.spec_from_file_location('mapping',ROOT/'reflex-key-v2/check_mapping.py')
        mapping=importlib.util.module_from_spec(spec);spec.loader.exec_module(mapping)
        core,source=mapping.load_inputs()
        return mapping,core,source
    def test_channel_mapping_exhausts_public_enum_and_preserves_multiset_keys(self):
        m,core,source=self.channel_mapping_inputs();result=m.witness(core,source)
        self.assertEqual(result['public_enum_values'],261);self.assertTrue(result['mapping_exact']);self.assertTrue(result['injective'])
        self.assertEqual(result['rust_invocations'],0);self.assertEqual(result['world_calls'],0)
    def test_channel_mapping_wrong_constant_cast_wildcard_or_guard_rejects(self):
        m,core,source=self.channel_mapping_inputs()
        for wrong in [source.replace('MotorChannel::Posture => 4,','MotorChannel::Posture => 3,'),source.replace('0x100 + u16::from(id)','0x101 + u16::from(id)'),source.replace('u16::from(id)','id as u16'),source.replace('MotorChannel::SpeciesSpecific(id) =>','_ =>'),source.replace('MotorChannel::SpeciesSpecific(id) =>','MotorChannel::SpeciesSpecific(id) if id > 0 =>')]:
            with self.assertRaises(ValueError):m.witness(core,wrong)
    def test_channel_mapping_future_enum_addition_or_changed_core_rejects(self):
        m,core,source=self.channel_mapping_inputs()
        for wrong in [core.replace('    Posture,','    Posture,\n    NewChannel,'),core.replace('SpeciesSpecific(u8)','SpeciesSpecific(u16)'),core.replace('Self::Vocal => 3,','Self::Vocal => 8,')]:
            with self.assertRaises(ValueError):m.witness(wrong,source)
    def test_channel_helper_remains_private_and_both_sites_are_required(self):
        m,core,source=self.channel_mapping_inputs()
        for wrong in [source.replace('fn recovery_reflex_channel_key','pub fn recovery_reflex_channel_key'),source.replace('recovery_reflex_channel_key(c.channel)','c.channel.canonical_key()',1)]:
            with self.assertRaises(ValueError):m.witness(core,wrong)

if __name__=='__main__':
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(SyntheticTests)
    result=unittest.TestResult();suite.run(result)
    # No synthetic payload or assertion text is printed, even if a self-test fails.
    report={'schema':'m2-runner-pure-synthetic-tests-v4','cases':result.testsRun,
            'passed':result.testsRun-len(result.errors)-len(result.failures),
            'failed_cases':[getattr(getattr(t,'test_case',t),'_testMethodName','synthetic_case') for t,_ in result.errors+result.failures],
            'subprocess_invocations':0,'native_builds':0,'native_fixtures':0,'world_ticks':0}
    (ROOT/'SYNTHETIC-TESTS.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report));raise SystemExit(0 if result.wasSuccessful() else 1)
