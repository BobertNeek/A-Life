#!/usr/bin/env python3
"""Synthetic/static checks only. Never invoke Rust, Cargo, a fixture or GitHub."""
import sys
sys.dont_write_bytecode = True
import ast, copy, hashlib, importlib.util, json, pathlib, re, tempfile, unittest
from unittest.mock import patch
from packet_inputs import ROOT, V5, read, verify_preservation
spec=importlib.util.spec_from_file_location('runner',ROOT/'run_hosted.py')
r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
exec_namespace={}
exec(compile(read('source_patch.py'),'source_patch.py','exec'),exec_namespace)
apply_source_patch=exec_namespace['apply']
FIXTURES=json.loads((ROOT/'FIXTURES.json').read_bytes())
BOUND={x['path']:x for x in json.loads((ROOT/'source-manifest.json').read_bytes())['files']}

class AdmissionTests(unittest.TestCase):
    def setUp(self):
        self.before='a'*40;self.head='b'*40
        env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':r.BRANCH,'GITHUB_RUN_ATTEMPT':'1'}
        event={'before':self.before,'after':self.head,'ref':r.BRANCH,'created':False,'deleted':False,'forced':False}
        marker={'schema':'m2-grip-fixed-fixture-activation-v5','plan_id':r.PLAN,'main':r.MAIN,'research_base':r.FBF,'prepared_head':self.before,'seal_sha256':'c'*64,'source_manifest_sha256':'d'*64,'attempt':1,'fixed_fixtures_only':True}
        self.args=[env,event,self.head,[self.head,self.before],'A\t'+r.MARKER+'\n',marker,'c'*64,'d'*64]
    def test_exact_activation(self):self.assertEqual(r.validate_activation(*self.args),self.before)
    def test_all_wrong_environments(self):
        for key in self.args[0]:
            args=copy.deepcopy(self.args);args[0][key]='wrong'
            with self.assertRaises(r.Stop):r.validate_activation(*args)
    def test_created_deleted_forced_and_wrong_event(self):
        for key,value in [('created',True),('deleted',True),('forced',True),('before','0'*40),('after','e'*40),('ref','refs/heads/main')]:
            args=copy.deepcopy(self.args);args[1][key]=value
            with self.assertRaises(r.Stop):r.validate_activation(*args)
    def test_parent_and_marker_must_be_sole_new_child(self):
        for parents in [[],[self.head],[self.head,'e'*40],[self.head,self.before,'f'*40]]:
            args=copy.deepcopy(self.args);args[3]=parents
            with self.assertRaises(r.Stop):r.validate_activation(*args)
        for changed in ['', 'M\t'+r.MARKER+'\n','A\told/ACTIVATE.json\n','A\t'+r.MARKER+'\nM\tCargo.toml\n']:
            args=copy.deepcopy(self.args);args[4]=changed
            with self.assertRaises(r.Stop):r.validate_activation(*args)
    def test_every_marker_field_exact(self):
        for key in self.args[5]:
            args=copy.deepcopy(self.args);args[5][key]='invalid'
            with self.assertRaises(r.Stop):r.validate_activation(*args)
        args=copy.deepcopy(self.args);args[5]['typecheck_only']=True
        with self.assertRaises(r.Stop):r.validate_activation(*args)

class InventoryTests(unittest.TestCase):
    def check_inventory(self,obj):
        with patch.object(r,'read_json',return_value=obj),patch.object(r,'BOUND',BOUND):return r.fixture_inventory()
    def test_exact_f041_inventory_one_target(self):
        targets,rows=self.check_inventory(FIXTURES)
        self.assertEqual(len(targets),1);self.assertEqual(len(rows),1)
        self.assertTrue(any(x['name']=='grounded_scene_pair_scores_and_cold_continuation' for x in rows))
        self.assertFalse(any('sealed_source_descriptor' in x['name'] for x in rows))
        self.assertEqual(len(FIXTURES['excluded_fixtures']),1)
    def test_inventory_rejects_unready_and_duplicate(self):
        for modify in [lambda x:x.update(ready=False),lambda x:x['fixtures'].append(x['fixtures'][0]),lambda x:x['targets'].append(x['targets'][0])]:
            obj=copy.deepcopy(FIXTURES);modify(obj)
            with self.assertRaises(r.Stop):self.check_inventory(obj)
    def test_exact_name_and_source_and_dose_fail_closed(self):
        for key,value in [('name','grip*'),('name',''),('ignored',True),('source_path','/secret'),('source_sha256','0'*64),('dose',{}),('dose',{'unknown':None}),('dose',{'x':-1})]:
            obj=copy.deepcopy(FIXTURES);obj['fixtures'][0][key]=value
            with self.assertRaises(r.Stop):self.check_inventory(obj)
    def test_targets_reject_broad_or_hardware_features(self):
        for key,value in [('kind','all-targets'),('features',['gpu-tests']),('package','alife_game_app'),('name','*')]:
            obj=copy.deepcopy(FIXTURES);obj['targets'][0][key]=value
            with self.assertRaises(r.Stop):self.check_inventory(obj)
    def test_f041_dose_and_unknown_compute_disclosed(self):
        old=json.loads(pathlib.Path('/tmp/m2-grip-fixed-fixtures-v3/FIXTURES.json').read_bytes())
        prior=next(x for x in old['fixtures'] if x['id']=='f041')
        dose=dict(FIXTURES['fixtures'][0]['dose']);self.assertEqual(dose.pop('ordinary_identity_rejection_calls'),1)
        self.assertEqual(dose,prior['dose'])
        self.assertEqual(FIXTURES['source_audited_complete_dose']['source_forwards'],3)
        self.assertEqual(FIXTURES['source_audited_complete_dose']['committed_registered_biology_intervals'],3)
        self.assertEqual(FIXTURES['source_audited_complete_dose']['fresh_worlds'],3)
        self.assertFalse(FIXTURES['compiler_reconstruction_count_complete'])
        self.assertIn('not instrumented',FIXTURES['dose_semantics'])
    def test_selected_source_records_are_all_bound(self):
        for row in FIXTURES['fixtures']:
            self.assertEqual(row['source_sha256'],BOUND[row['source_path']]['sha256'])
            self.assertIn('::',row['name']) if row['source_path'].split('/')[2]=='src' else None

class ProtocolTests(unittest.TestCase):
    def test_listing_accepts_only_exact_single(self):
        name='some::exact_test'
        for raw in [b'some::exact_test: test\n',b'some::exact_test: test\n\n1 test, 0 benchmarks\n']:
            r.validate_exact_listing(raw,name)
    def test_listing_rejects_other_duplicate_broad_or_benchmark(self):
        for raw in [b'',b'other: test\n',b'some::exact_test: benchmark\n',b'some::exact_test: test\nother: test\n',b'some::exact_test: test\nsome::exact_test: test\n',b'some::exact_test: test\n2 tests, 0 benchmarks\n',b'\xff']:
            with self.assertRaises(r.Stop):r.validate_exact_listing(raw,'some::exact_test')
    def test_result_requires_exactly_one_pass(self):
        r.validate_exact_result(b'\nrunning 1 test\n.\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.01s\n')
    def test_result_rejects_ignored_empty_failed_multiple(self):
        base='running 1 test\n.\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.01s\n'
        for raw in [base.replace('1 passed','0 passed'),base.replace('0 ignored','1 ignored'),base.replace('0 failed','1 failed'),base.replace('running 1 test','running 2 tests'),base+'private assertion\n','',base.replace('ok.','FAILED.')]:
            with self.assertRaises(r.Stop):r.validate_exact_result(raw.encode())
    def test_list_uses_exact_flags_without_running_fixture(self):
        state=copy.deepcopy(r.STATE);row=FIXTURES['fixtures'][0];calls=[]
        def run(args,*a,**kw):calls.append(args);return (row['name']+': test\n').encode(),0
        with patch.object(r,'STATE',state),patch.object(r,'WORK',pathlib.Path('/fixture')),patch.object(r,'digest_file',return_value='a'*64),patch.object(r,'run',side_effect=run):
            r.list_exact_fixture(row,pathlib.Path('/fixture/target/test'),'a'*64)
        self.assertEqual(calls,[['/fixture/target/test','--exact',row['name'],'--list','--format','terse']])
        self.assertEqual(state['launched_fixture_processes'],0)
    def test_exact_run_once_and_no_retry_after_failure(self):
        row=FIXTURES['fixtures'][0];state=copy.deepcopy(r.STATE);state['listed_fixture_ids']=[row['id']]
        with tempfile.TemporaryDirectory(prefix='grip-static-') as d:
            with patch.object(r,'STATE',state),patch.object(r,'WORK',pathlib.Path(d)),patch.object(r,'digest_file',return_value='a'*64),patch.object(r,'run',return_value=(b'private panic/assertion text',101)) as run:
                with self.assertRaises(r.Stop):r.execute_exact_fixture(row,pathlib.Path(d)/'test','a'*64)
                with self.assertRaises(r.Stop):r.execute_exact_fixture(row,pathlib.Path(d)/'test','a'*64)
                self.assertEqual(run.call_count,1)
                self.assertEqual(run.call_args[0][0],[str(pathlib.Path(d)/'test'),'--exact',row['name'],'--test-threads=1','--format','terse'])
        self.assertEqual(state['launched_fixture_processes'],1);self.assertEqual(state['confirmed_passed_test_bodies'],0)
        self.assertNotIn('private',json.dumps(state))
    def test_run_success_records_only_static_identifiers(self):
        row=FIXTURES['fixtures'][0];state=copy.deepcopy(r.STATE);state['listed_fixture_ids']=[row['id']]
        raw=b'running 1 test\n.\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 999 filtered out; finished in 0.00s\n'
        with tempfile.TemporaryDirectory(prefix='grip-static-') as d:
            with patch.object(r,'STATE',state),patch.object(r,'WORK',pathlib.Path(d)),patch.object(r,'digest_file',return_value='a'*64),patch.object(r,'run',return_value=(raw,0)):
                r.execute_exact_fixture(row,pathlib.Path(d)/'test','a'*64)
        self.assertEqual(state['passed_fixture_ids'],[row['id']]);self.assertEqual(state['confirmed_passed_test_bodies'],1)
    def test_modified_binary_rejected_before_list_or_run(self):
        row=FIXTURES['fixtures'][0]
        with patch.object(r,'digest_file',return_value='b'*64),patch.object(r,'run') as run:
            with self.assertRaises(r.Stop):r.list_exact_fixture(row,pathlib.Path('/x'),'a'*64)
            with self.assertRaises(r.Stop):r.execute_exact_fixture(row,pathlib.Path('/x'),'a'*64)
            run.assert_not_called()

class CompilationTests(unittest.TestCase):
    def test_build_compiles_only_one_target_without_running(self):
        spec=FIXTURES['targets'][0]
        with tempfile.TemporaryDirectory(prefix='grip-static-') as d:
            work=pathlib.Path(d);(work/'target').mkdir();exe=work/'target/test';exe.write_text('inert fixture bytes');exe.chmod(0o700)
            source=work/'source';state=copy.deepcopy(r.STATE)
            artifact={'reason':'compiler-artifact','target':{'name':spec['name'],'src_path':str(source/'crates'/spec['package']/'src/lib.rs'),'kind':['test' if spec['kind']=='test' else 'lib']},'features':r.EXPECTED_FEATURES[spec['package']],'profile':{'test':True},'executable':str(exe),'filenames':[str(exe)],'package_id':spec['package']}
            def fake(args,*a,**kw):
                self.assertEqual(args[:9],['rustup','run','1.99.1','cargo','test','--no-run','--locked','-j1','-p'])
                self.assertNotIn('--all-targets',args);self.assertNotIn('--all-features',args)
                kw['consumer'](json.dumps(artifact).encode());kw['consumer'](b'{"reason":"build-finished","success":true}')
                return b'',0
            with patch.object(r,'WORK',work),patch.object(r,'SOURCE',source),patch.object(r,'STATE',state),patch.object(r,'RUSTUP','rustup'),patch.object(r,'BOUND',{}),patch.object(r,'verify_depinfo'),patch.object(r,'run',side_effect=fake):
                path,digest=r.build_fixture_target(spec)
                self.assertEqual(path,exe);self.assertEqual(len(digest),64)
            self.assertEqual(state['compiled_target_ids'],[spec['id']])
    def test_failure_stops_before_list_and_run(self):
        spec=FIXTURES['targets'][0]
        with patch.object(r,'run',return_value=(b'private compiler output',1)),patch.object(r,'list_exact_fixture') as listing,patch.object(r,'execute_exact_fixture') as execution:
            with self.assertRaises(r.Stop):r.build_fixture_target(spec)
            listing.assert_not_called();execution.assert_not_called()
    def test_all_builds_then_all_lists_then_each_exact_run(self):
        tree=ast.parse((ROOT/'run_hosted.py').read_text());main=next(x for x in tree.body if isinstance(x,ast.FunctionDef) and x.name=='main');source=ast.unparse(main)
        self.assertLess(source.index('fixture_inventory()'),source.index('setup_toolchain(original)'))
        self.assertLess(source.index('build_fixture_target(spec)'),source.index('list_exact_fixture(row'))
        self.assertLess(source.index('list_exact_fixture(row'),source.index('execute_exact_fixture(row'))
        self.assertLess(source.index('execute_exact_fixture(row'),source.rindex('verify_record(SOURCE, row)'))
        self.assertNotIn('check_stage(',source)

class PreservationTests(unittest.TestCase):
    def test_all_prior_packets_preserved(self):verify_preservation()
    def test_exact_three_v3_source_changes_and_other863_unchanged(self):
        old=json.loads(pathlib.Path('/tmp/m2-grip-fixed-fixtures-v3/source-manifest.json').read_bytes())
        new=json.loads((ROOT/'source-manifest.json').read_bytes())
        d=json.loads((ROOT/'F041-DIAGNOSTIC.json').read_bytes())
        records={r['path']:r for r in old['files']}
        for c in d['changes']:
            self.assertEqual(records[c['before_record']['path']],c['before_record'])
            records[c['source_record']['path']]=c['source_record']
        self.assertEqual(new['files'],sorted(records.values(),key=lambda x:x['path']))
        self.assertEqual(len(new['files']),866)
        oldmap={x['path']:x for x in old['files']}
        self.assertEqual(sum(oldmap.get(x['path'])==x for x in new['files']),863)
    def test_additive_patch_new_test_only_and_exact_bytes(self):
        delta=json.loads((ROOT/'TEST-ONLY-DELTA.json').read_bytes());raw=(ROOT/delta['patch_path']).read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(),delta['patch_sha256'])
        applied=apply_source_patch(raw,lambda _:None);self.assertEqual(list(applied),[delta['source_record']['path']])
        source=applied[delta['source_record']['path']]
        self.assertEqual(hashlib.sha256(source).hexdigest(),delta['source_record']['sha256'])
        self.assertEqual(len(re.findall(rb'#\[test\]',source)),1)
        for forbidden in [b'request_adapter',b'request_device',b'step_research',b'create_new_game',b'compile_n2048',b'gpu-tests']:
            self.assertNotIn(forbidden,source)
        self.assertLess(source.index(b'grip.validate().unwrap()'),source.index(b'GpuPerceptionUpload::try_from_frame(&grip'))
        self.assertIn(b'Err(GpuClosedLoopError::LayoutMismatch)',source)
        self.assertIn(b'upload.validate_against(&legacy, &slot).unwrap()',source)
    def test_unchanged_runtime_limits_and_guard_functions(self):
        old=ast.parse((V5/'run_hosted.py').read_text());new=ast.parse((ROOT/'run_hosted.py').read_text())
        funcs=lambda tree:{x.name:ast.dump(x,include_attributes=False) for x in tree.body if isinstance(x,ast.FunctionDef)}
        of,nf=funcs(old),funcs(new)
        for name in ['run','check_limits','session_rss','kill_session','child_setup','setup_toolchain','verify_depinfo','diagnostic_rows','verify_record','digest_file','verify_workflow_preservation']:
            self.assertEqual(of[name],nf[name],name)
        assign=lambda tree,name:next(ast.dump(x.value,include_attributes=False) for x in tree.body if isinstance(x,ast.Assign) and any(isinstance(t,ast.Name) and t.id==name for t in x.targets))
        for name in ['LIMITS','EXPECTED_FEATURES','TOOLCHAIN','RUST_COMMIT','RUST_DATE','RUST_VERSION','CARGO_VERSION','HOST','ACTION_SHA']:
            self.assertEqual(assign(old,name),assign(new,name),name)
    def test_actual_inherited_yaml_filters_exclude_both_pushes(self):
        import yaml,fnmatch
        fixtures=json.loads(read('INHERITED-WORKFLOW-FIXTURES.json'))['files'];contract=json.loads((ROOT/'WORKFLOW-PRESERVATION.json').read_bytes());records={x['path']:x for x in contract['files']}
        self.assertEqual(len(fixtures),15)
        for fixture in fixtures:
            data=fixture['content_utf8'].encode();row=records[fixture['path']]
            self.assertEqual(hashlib.sha256(data).hexdigest(),row['sha256'])
            self.assertEqual(hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest(),row['git_blob_sha1'])
            events=yaml.load(data,Loader=yaml.BaseLoader)['on']
            self.assertFalse(set(events)-{'push','pull_request','workflow_dispatch'})
            if 'push' in events:self.assertFalse(any(fnmatch.fnmatchcase(r.BRANCH.removeprefix('refs/heads/'),p) for p in events['push']['branches']))
    def test_actual_preserved_workflow_bytes_and_inventory(self):
        fixtures=json.loads(read('INHERITED-WORKFLOW-FIXTURES.json'))['files'];lines=['100644 blob '+x['git_blob_sha1']+'\t'+x['path'] for x in fixtures]
        lines+=['100644 blob '+'a'*40+'\t.github/workflows/'+r.PLAN+'.yml'];lines.sort(key=lambda x:x.split('\t')[1])
        with tempfile.TemporaryDirectory(prefix='grip-static-') as d:
            bundle=pathlib.Path(d)
            for row in fixtures:
                p=bundle/row['path'];p.parent.mkdir(parents=True,exist_ok=True);p.write_text(row['content_utf8'])
            r.verify_workflow_preservation(bundle,lines)
            with self.assertRaises(r.Stop):r.verify_workflow_preservation(bundle,lines[1:])
            with self.assertRaises(r.Stop):r.verify_workflow_preservation(bundle,lines+[lines[0]])
            (bundle/fixtures[0]['path']).write_text('tampered')
            with self.assertRaises(r.Stop):r.verify_workflow_preservation(bundle,lines)
    def test_single_marker_workflow_no_external_effects(self):
        import yaml
        text=(ROOT/'workflow.yml').read_text();doc=yaml.load(text,Loader=yaml.BaseLoader)
        self.assertEqual(doc['on'],{'push':{'branches':[r.BRANCH.removeprefix('refs/heads/')],'paths':[r.MARKER]}})
        self.assertEqual(doc['permissions'],{'contents':'read'})
        self.assertEqual(list(doc['jobs']),['source-typecheck'])
        gate=doc['jobs']['source-typecheck']['if']
        for condition in ["github.event.created == false","github.event.deleted == false","github.event.forced == false","github.run_attempt == '1'",r.BRANCH]:self.assertIn(condition,gate)
        for forbidden in ['upload-artifact','actions/cache','secrets.','workflow_dispatch:','pull_request:','contents: write']:self.assertNotIn(forbidden,text)
        self.assertEqual(text.count('persist-credentials: false'),2)
    def test_source_only_reuse_and_no_activation(self):
        for row in json.loads((ROOT/'REUSE-MANIFEST.json').read_bytes())['files']:
            b=read(row['path']);b.decode('utf-8','strict');self.assertNotIn(b'\0',b)
            self.assertEqual(hashlib.sha256(b).hexdigest(),row['sha256'])
            self.assertNotIn(row['path'],['run_hosted.py','workflow.yml','ACTIVATE.json'])
        self.assertFalse((ROOT/'ACTIVATE.json').exists())
    def test_local_staging_below_three_mib(self):
        self.assertLessEqual(sum(p.stat().st_size for p in ROOT.rglob('*') if p.is_file()),3*1024**2)
    def test_python_syntax(self):
        for p in ROOT.glob('*.py'):ast.parse(p.read_text(),filename=p.name)
    def test_summary_and_diagnostics_never_include_private_text(self):
        obj={'message':{'level':'error','code':{'code':'E0308','explanation':'private'},'message':'private','rendered':'private','spans':[{'is_primary':True,'file_name':'crates/alife_core/src/lib.rs','line_start':5,'text':['private']}]}}
        rows=r.diagnostic_rows(obj,'core',pathlib.Path('/source'),{'crates/alife_core/src/lib.rs':{}})
        self.assertEqual(rows,[{'code':'E0308','path':'crates/alife_core/src/lib.rs','line':5,'stage':'core'}])
        self.assertNotIn('private',r.summary('INTERNAL'));self.assertLessEqual(len(r.summary('INTERNAL').encode()),32768)



class DiagnosticV4Tests(unittest.TestCase):
    def metadata(self):
        return json.loads((ROOT/'F041-DIAGNOSTIC.json').read_bytes())
    def sources(self):
        base=pathlib.Path('/workspace/shared/m2-grip-options-v1/host-slice-v3')
        result=[]
        for c in self.metadata()['changes']:
            before=((base/c['before_record']['path']) if (base/c['before_record']['path']).is_file() else pathlib.Path('/workspace/shared/m2-grip-options-v1/core-slice-v3')/c['before_record']['path']).read_bytes()
            self.assertEqual(hashlib.sha256(before).hexdigest(),c['before_record']['sha256'])
            raw=(ROOT/c['patch_record']['path']).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(),c['patch_record']['sha256'])
            applied=apply_source_patch(raw,lambda p:before if p==c['before_record']['path'] else None)
            self.assertEqual(set(applied),{c['source_record']['path']})
            after=applied[c['source_record']['path']]
            self.assertEqual(len(after),c['source_record']['bytes'])
            self.assertEqual(hashlib.sha256(after).hexdigest(),c['source_record']['sha256'])
            result.append((before,after))
        return result
    def wire(self,events,failed=None):
        rows=[]
        for kind,ident in events:
            rows.append(('GRIP_F041_PHASE='+str(ident)) if kind=='phase' else
                        ('GRIP_F041_CHECK='+str(ident)+':'+('0' if failed==ident else '1')))
        return ('\n'.join(rows)+'\n').encode()

    def test_production_change_is_only_declared_grip_construction_normalization(self):
        (before,after),_,_=self.sources()
        insertion=(b"        // Reconstruct the frozen genetic source, not the newborn's evolving gates.\n"
                   b"        let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0)?);\n")
        self.assertEqual(after.replace(insertion,b'',1),before)
        self.assertEqual(after.count(insertion),1)
        branch=after[after.index(b'let (compiled, _) = if crate::cpu::grip_integration::is_grip(p)'):after.index(b'} else if p.foundation_abi().n2048_grounded_drive_basis().is_some()')]
        self.assertLess(branch.index(b'validate_n2048_grip_sleep_choice_research()?'),branch.index(insertion))
        self.assertIn(b'if compiled.phenotype_hash() != p.phenotype_hash()',after)
        self.assertIn(b'if inherited.encode_canonical()? != founder.source_foundation.encode_canonical()?',after)
        self.assertNotIn(b'record.',insertion)

    def test_original_integration_assertions_and_calls_are_reversible(self):
        _,(before,after),_=self.sources()
        text=after.decode()
        text=re.sub(r'// BEGIN_F041_DIAGNOSTIC_HELPERS\n.*?// END_F041_DIAGNOSTIC_HELPERS\n','',text,flags=re.S)
        text=re.sub(r'    // BEGIN_F041_ORDINARY_IDENTITY_REJECTION\n.*?    // END_F041_ORDINARY_IDENTITY_REJECTION\n','',text,flags=re.S)
        text=re.sub(r'^    report_f041_(?:phase|check)\([^\n]*\);\n','',text,flags=re.M)
        text=text.replace('    let decoded = CpuGripOptionsM2HostCheckpointV1::decode(&saved).unwrap();\n','',1)
        text=text.replace('        decoded,\n','        CpuGripOptionsM2HostCheckpointV1::decode(&saved).unwrap(),\n',1)
        self.assertEqual(text.encode(),before)
        self.assertEqual(after.count(b'step_research_observed('),3)
        self.assertEqual(re.findall(rb'#\[test\]\s*fn\s+(\w+)',before),re.findall(rb'#\[test\]\s*fn\s+(\w+)',after))
        self.assertNotIn(b'--nocapture',after)

    def test_phase_event_tables_match_source_and_metadata(self):
        d=self.metadata()
        self.assertEqual(list(r.F041_PHASES),d['phases'])
        self.assertEqual(list(r.F041_CHECKS),d['checks'])
        self.assertEqual([list(x) for x in r.F041_EVENTS],d['event_order'])
        self.assertEqual([i for k,i in r.F041_EVENTS if k=='phase'],list(range(1,len(r.F041_PHASES)+1)))
        self.assertEqual([i for k,i in r.F041_EVENTS if k=='check'],list(range(1,len(r.F041_CHECKS)+1)))
        self.assertLess(len(r.F041_EVENTS),100)

    def test_projection_accepts_every_exact_prefix_without_raw_text(self):
        for count in range(len(r.F041_EVENTS)+1):
            raw=b'private assertion and checkpoint text\n'+self.wire(r.F041_EVENTS[:count])+b'private tail\n'
            p=r.project_f041_failure(raw)
            self.assertNotIn('private',json.dumps(p))
            self.assertLessEqual(p['phase_count'],len(r.F041_PHASES))
            self.assertLess(len(json.dumps(p)),4096)

    def test_projection_rejects_skips_duplicates_unknowns_and_malformed(self):
        valid=self.wire(r.F041_EVENTS[:2])
        for raw in [b'GRIP_F041_PHASE=2\n',valid+b'GRIP_F041_PHASE=2\n',
                    b'GRIP_F041_PHASE=99\n',b'GRIP_F041_CHECK=1:2\n',
                    b'GRIP_F041_CHECK=1:1\n',b'GRIP_F041_PHASE=01\n',
                    b'GRIP_F041_SECRET=private\n',b'GRIP_F041_PHASE='+b'1'*81+b'\n']:
            with self.assertRaises(r.Stop):r.project_f041_failure(raw)

    def test_false_check_is_safe_and_terminal(self):
        index=next(i for i,e in enumerate(r.F041_EVENTS) if e[0]=='check')
        ident=r.F041_EVENTS[index][1]
        raw=self.wire(r.F041_EVENTS[:index+1],failed=ident)
        p=r.project_f041_failure(raw)
        self.assertFalse(p['checks'][-1]['passed'])
        with self.assertRaises(r.Stop):r.project_f041_failure(raw+self.wire(r.F041_EVENTS[index+1:index+2]))

    def test_diagnostic_only_added_on_failed_f041(self):
        row=FIXTURES['fixtures'][0];self.assertEqual(row['id'],'f041')
        state=copy.deepcopy(r.STATE);state['listed_fixture_ids']=['f041']
        raw=b'private panic\n'+self.wire(r.F041_EVENTS[:2])
        with tempfile.TemporaryDirectory(prefix='f041-static-') as d:
            with patch.object(r,'STATE',state),patch.object(r,'WORK',pathlib.Path(d)),patch.object(r,'digest_file',return_value='a'*64),patch.object(r,'run',return_value=(raw,101)):
                with self.assertRaises(r.Stop):r.execute_exact_fixture(row,pathlib.Path(d)/'test','a'*64)
        self.assertEqual(state['f041_failure']['last_observed_phase'],r.F041_PHASES[1])
        self.assertNotIn('private',json.dumps(state))

    def test_retained40_and_pending5_are_not_executed(self):
        c=json.loads((ROOT/'CONTINUATION.json').read_bytes())
        self.assertEqual(c['retained_fixture_ids'],['f%03d'%i for i in range(1,41)])
        self.assertEqual(c['execute_fixture_ids'],['f041'])
        self.assertEqual(c['pending_fixture_ids'],['f%03d'%i for i in range(42,47)])
        self.assertEqual([x['id'] for x in FIXTURES['fixtures']],['f041'])
        self.assertEqual(len(FIXTURES['targets']),1)
        self.assertEqual(FIXTURES['targets'][0]['name'],'grip_options_host')
        new={r['path']:r for r in json.loads((ROOT/'source-manifest.json').read_bytes())['files']}
        for row in c['retained_fixtures']:
            self.assertEqual(row['source_sha256'],new[row['source_path']]['sha256'])

    def test_v3_receipt_binds_exact_source_and_stop(self):
        c=json.loads((ROOT/'CONTINUATION.json').read_bytes());s=c['v3_retained_evidence']['sanitized_summary']
        self.assertEqual(s['passed_fixture_ids'],['f%03d'%i for i in range(19,41)])
        self.assertEqual(s['stage'],'run_f041')
        self.assertEqual(s['activation_head'],'65647c363ec99b0dc9b9f343d252f97fdd97413c')
        self.assertEqual(s['source_manifest_sha256'],'a0da59e3c8291883fc6f499500ee5824abf241d098b211a088dfda86719bf8ec')
        self.assertTrue(c['previous_attempt_dose_is_upper_bound'])

    def test_runner_changes_only_guarded_restore_from_v4(self):
        old=pathlib.Path('/tmp/m2-grip-fixed-fixtures-v4/run_hosted.py').read_text()
        old=old.replace('m2-grip-fixed-fixtures-v4','m2-grip-fixed-fixtures-v5')
        old=re.sub(r'(m2-grip-fixed-[a-z-]+)-v4\b',r'\1-v5',old)
        funcs=lambda s:{x.name:ast.dump(x,include_attributes=False) for x in ast.parse(s).body if isinstance(x,ast.FunctionDef)}
        a,b=funcs(old),funcs((ROOT/'run_hosted.py').read_text())
        self.assertEqual(set(b),set(a))
        self.assertEqual({n for n in a if a[n]!=b[n]},{'restore_test_fixture'})
        self.assertNotIn('--nocapture',(ROOT/'run_hosted.py').read_text())

    def test_remote_inventory_and_no_binary_cache_or_marker(self):
        rows=json.loads((ROOT/'REMOTE-FILES.json').read_bytes())['prepared_files']
        self.assertEqual(len(rows),len({x['path'] for x in rows}))
        self.assertEqual(len(rows),59)
        for row in rows:
            self.assertEqual(row['mode'],'100644')
            self.assertTrue(row['path'].startswith('docs/experiments/m2-grip-fixed-fixtures-v5/') or row['path']=='.github/workflows/m2-grip-fixed-fixtures-v5.yml')
            self.assertFalse(row['path'].endswith('/ACTIVATE.json'))
            self.assertNotIn('__pycache__',row['path'])
            self.assertFalse(row['path'].endswith(('.pyc','.tar.gz')))

class ResearchIdentityV5Tests(unittest.TestCase):
    def sources(self):
        root=pathlib.Path('/tmp/m2-grip-binding-investigation-v5/crates/alife_core/src')
        result={}
        for rel in ['evolutionary_genetics.rs','phenotype/capacity.rs','foundation.rs','grounded_drive_genetics.rs']:
            p='crates/alife_core/src/'+rel;b=(root/rel).read_bytes()
            row=BOUND[p]
            if rel=='grounded_drive_genetics.rs':
                d=json.loads((ROOT/'F041-DIAGNOSTIC.json').read_bytes())['changes'][2]
                self.assertEqual(hashlib.sha256(b).hexdigest(),d['before_record']['sha256'])
                b=apply_source_patch((ROOT/d['patch_record']['path']).read_bytes(),lambda _:b)[p]
            self.assertEqual(hashlib.sha256(b).hexdigest(),row['sha256'])
            result[rel]=b.decode()
        return result
    def test_only_checked_grip_identity_tail_changes(self):
        d=json.loads((ROOT/'F041-DIAGNOSTIC.json').read_bytes())['changes'][2]
        before=pathlib.Path('/tmp/m2-grip-binding-investigation-v5',d['before_record']['path']).read_text()
        after=self.sources()['grounded_drive_genetics.rs']
        prefix=before[:before.index('    FoundationGeneticIdentity::new(',before.index('fn grip_research_identity('))]
        self.assertTrue(after.startswith(prefix))
        self.assertEqual(after.count('Ok(FoundationGeneticIdentity {'),2)
        tail=after[len(prefix):]
        self.assertIn('u16::try_from(m.foundation_version().raw())',tail)
        self.assertNotIn('FoundationGeneticIdentity::new(',tail)
        self.assertNotIn('record.',tail)
    def test_ordinary_admission_rejects_research_classes_unchanged(self):
        x=self.sources();identity=x['evolutionary_genetics.rs'];capacity=x['phenotype/capacity.rs']
        self.assertIn('identity.validate_contract()?;',identity)
        self.assertIn('BrainCapacityClass::production_for_id(self.brain_class_id)?;',identity)
        block=capacity[capacity.index('pub fn production_for_id'):capacity.index('pub fn supported_for_id')]
        self.assertEqual(re.findall(r'Self::(N[0-9]+_ID) =>',block),['N512_ID','N1024_ID','N2048_ID'])
        self.assertIn('_ => Err(ScaffoldContractError::UnsupportedProductionBrainClass)',block)
        self.assertNotIn('N2048_GROUNDED_DRIVE_RESEARCH_ID =>',block)
    def test_wrong_class_profile_family_version_layout_or_rows_remain_closed(self):
        x=self.sources();f=x['foundation.rs'];helper=x['grounded_drive_genetics.rs'].split('fn grip_research_identity(',1)[1]
        self.assertLess(helper.index('asset.encode_canonical()?'),helper.index('Ok(FoundationGeneticIdentity {'))
        for guard in ['!= BrainCapacityClass::N2048_GROUNDED_DRIVE_RESEARCH_ID','!= SensorProfile::GroundedGripSleepChoiceV1','self.weights.len() != 34048','self.manifest.foundation_version != FoundationVersion::V1','self.manifest.compatibility_family_id != compatibility_family_id','layout_digest(&N2048FoundationLayoutV1::lobe_layout())']:
            self.assertIn(guard,f)
        self.assertIn('!asset.is_n2048_grounded_grip_sleep_choice()',helper)
    def test_promoted_invalid_stage_receipt_or_digest_remain_closed(self):
        x=self.sources();f=x['foundation.rs'];helper=x['grounded_drive_genetics.rs'].split('fn grip_research_identity(',1)[1]
        self.assertIn('asset.manifest().promotion_receipt().is_promoted()',helper)
        for guard in ['self.validate_self_contained()?','self.manifest.training_stage.validate()?','.validate(self.manifest.training_stage)?','digest != self.digest || digest != self.manifest.weight_asset.digest','self.weights.iter().any(|weight| !weight.is_finite())']:
            self.assertIn(guard,f)
    def test_source_mismatch_genome_and_phenotype_binding_retained(self):
        s=self.sources()['grounded_drive_genetics.rs']
        for guard in ['source.validate_n2048_grounded_drive_research()?','asset.frozen_grip_source_asset()?','!= *source','value.validate_n2048_grip_sleep_choice_research()?','if self.foundation != grip_research_identity(asset)?','retained.express_n2048_grounded_drive_research()?','asset.validate_against(&phenotype)?']:
            self.assertIn(guard,s)
    def test_manifest_patch_digests_bind_all_three_corrections(self):
        d=json.loads((ROOT/'F041-DIAGNOSTIC.json').read_bytes())
        manifest=json.loads((ROOT/'source-manifest.json').read_bytes())
        self.assertEqual(manifest['f041_patch_sha256'],[c['patch_record']['sha256'] for c in d['changes']])
        self.assertEqual(manifest['schema'],'m2-grip-fixed-fixture-source-v5')
        audit=json.loads((ROOT/'SOURCE-AUDIT.json').read_bytes())['full_v5_source_closure_audit']
        self.assertEqual(audit['source_inputs_verified'],866)
        self.assertEqual(audit['inherited_leaves'],1313)
        self.assertEqual(audit['inherited_workflows'],15)
    def test_v4_safe_failure_is_unchanged_and_not_a_pass(self):
        old=pathlib.Path('/tmp/m2-continuation-reconcile-20261109-2100/GRIP-V4-SUMMARY.json').read_bytes()
        self.assertEqual(old,(ROOT/'PRIOR-V4-SAFE-RESULT.json').read_bytes())
        summary=json.loads(old)
        self.assertEqual(summary['confirmed_passed_test_bodies'],0)
        self.assertEqual(summary['passed_fixture_ids'],[])
        self.assertEqual(summary['f041_failure'],{'last_observed_phase':'ASSET_GENOTYPE_BIND_BEGIN','phase_count':5,'checks':[]})

    def test_phase_tables_include_precise_asset_and_genotype_boundaries(self):
        for name in ['ASSET_CREATE_BEGIN','ASSET_CREATE_DONE','ORDINARY_IDENTITY_REJECT_BEGIN','ORDINARY_IDENTITY_REJECT_DONE','GENOTYPE_BIND_BEGIN','GENOTYPE_BIND_DONE']:
            self.assertIn(name,r.F041_PHASES)
        self.assertEqual(r.F041_PHASES[4:10],('ASSET_CREATE_BEGIN','ASSET_CREATE_DONE','ORDINARY_IDENTITY_REJECT_BEGIN','ORDINARY_IDENTITY_REJECT_DONE','GENOTYPE_BIND_BEGIN','GENOTYPE_BIND_DONE'))
        self.assertEqual(r.F041_CHECKS[0],'ORDINARY_IDENTITY_REJECTS_RESEARCH')

if __name__=='__main__':
    result=unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__]))
    report={'schema':'m2-grip-fixed-static-v5','passed':result.wasSuccessful(),'tests_run':result.testsRun,'failures':len(result.failures),'errors':len(result.errors),'native_invocations':0,'github_writes':0,'fixture_bodies_executed':0,'scope':'Pure Python synthetic/static checks only. No Rust, Cargo or fixture process.'}
    (ROOT/'STATIC-CHECK.json').write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
    raise SystemExit(0 if result.wasSuccessful() else 1)
