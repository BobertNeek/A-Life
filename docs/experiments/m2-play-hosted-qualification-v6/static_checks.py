#!/usr/bin/env python3
"""Synthetic/source checks only. Denies every child process and native invocation."""
import ast,copy,gzip,hashlib,importlib.util,io,json,pathlib,re,tarfile
ROOT=pathlib.Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('play_guard',ROOT/'run_hosted.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
m.subprocess.Popen=lambda *a,**k:(_ for _ in ()).throw(AssertionError('subprocess forbidden'))
results=[]
def ok(name,f):f();results.append(name)
def reject(code,f):
 try:f()
 except m.Stop as e:assert str(e)==code
 else:raise AssertionError('accepted invalid synthetic input')
def admission():
 before='1'*40;head='2'*40;seal='3'*64;src='4'*64
 env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':m.BRANCH,'GITHUB_RUN_ATTEMPT':'1'}
 event={'before':before,'after':head,'ref':m.BRANCH,'created':False,'deleted':False,'forced':False}
 marker={'schema':'m2-play-qualification-activation-v1','plan_id':m.PLAN,'main':m.MAIN,'research_base':m.BASE,'prepared_head':before,'seal_sha256':seal,'source_manifest_sha256':src,'attempt':1,'fixed_disposable_qualification_only':True}
 def call(e=env,v=event,p=[head,before],d='A\t'+m.MARKER+'\n',mk=marker):return m.validate_activation(e,v,head,p,d,mk,seal,src)
 assert call()==before
 for field in ['created','deleted','forced']:
  v=dict(event);v[field]=True;reject('ADMISSION',lambda v=v:call(v=v))
 for key,value in [('GITHUB_RUN_ATTEMPT','2'),('GITHUB_REF','refs/heads/main'),('GITHUB_EVENT_NAME','workflow_dispatch')]:
  e=dict(env);e[key]=value;reject('ADMISSION',lambda e=e:call(e=e))
 reject('ADMISSION',lambda:call(p=[head,before,'5'*40]));reject('ADMISSION',lambda:call(d='M\t'+m.MARKER+'\n'));reject('ADMISSION',lambda:call(d='A\t'+m.MARKER+'\nA\tother\n'))
 mk=dict(marker);mk['prepared_head']='PENDING';reject('ADMISSION',lambda:call(mk=mk))
def tree():
 old=['100644 blob '+'1'*40+'\tCargo.toml'];new=old+['100644 blob '+'2'*40+'\t'+m.PACKET+'/run_hosted.py']
 assert m.validate_tree_preservation(old,new)
 reject('ADMISSION',lambda:m.validate_tree_preservation(old,[]))
 reject('ADMISSION',lambda:m.validate_tree_preservation(old,[old[0].replace('1'*40,'3'*40)]+new[1:]))
 reject('ADMISSION',lambda:m.validate_tree_preservation(old,new+['100644 blob '+'2'*40+'\t'+m.MARKER]))
 reject('ADMISSION',lambda:m.validate_tree_preservation(old,new+['100644 blob '+'2'*40+'\t.github/workflows/evil.yml']))
def archive():
 data=b'{}\n';row={'path':'source.json','bytes':3,'sha256':m.sha(data)}
 def make(names):
  b=io.BytesIO()
  with tarfile.open(fileobj=b,mode='w:gz') as tf:
   for name,kind in names:
    t=tarfile.TarInfo(name);t.size=3;t.type=kind;tf.addfile(t,io.BytesIO(data) if kind==tarfile.REGTYPE else None)
  return b.getvalue()
 assert m.overlay_members(make([('source.json',tarfile.REGTYPE)]),{'source.json':row})=={'source.json':data}
 for rows in [[('../source.json',tarfile.REGTYPE)],[('/source.json',tarfile.REGTYPE)],[('source.json',tarfile.SYMTYPE)],[('extra',tarfile.REGTYPE)],[('source.json',tarfile.REGTYPE)]*2]:
  reject('SOURCE_CLOSURE',lambda rows=rows:m.overlay_members(make(rows),{'source.json':row}))
 reject('SOURCE_CLOSURE',lambda:m.overlay_members(make([]),{'source.json':row}))
def protocols():
 tests=json.loads((ROOT/'TEST-INVENTORY.json').read_text())['tests'];assert len(tests)==1;assert len({r['name'] for r in tests})==1
 assert tests[0]['id']=='new-05' and tests[0]['class']=='new'
 for t in tests:
  n=t['name'];m.test_listing((n+': test\n\n1 test, 0 benchmarks\n').encode(),n)
  m.test_outcome(('\nrunning 1 test\ntest '+n+' ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 99 filtered out; finished in 0.01s\n\n').encode(),n)
  reject('TEST_PROTOCOL',lambda n=n:m.test_listing((n+': test\n\n2 tests, 0 benchmarks\n').encode(),n))
  reject('TEST_PROTOCOL',lambda n=n:m.test_outcome(b'panic secret assertion details',n))
 lines=[{'schema':'m2-play-default-wire-v1','boundary':i,'bytes':42,'sha256':'a'*64} for i in range(3)]
 raw=lambda x:b'\n'.join(json.dumps(v).encode() for v in x)+b'\n'
 assert len(m.probe_rows(raw(lines)))==3
 for invalid in [lines[:2],lines+[lines[0]],list(reversed(lines)),[{**r,'secret':'never publish'} for r in lines]]:
  reject('PROBE_PROTOCOL',lambda invalid=invalid:m.probe_rows(raw(invalid)))
 reject('PROBE_PROTOCOL',lambda:m.probe_rows(b'\xff\n{}\n{}\n'))
def sanitation():
 secret='DO_NOT_EXPOSE_ASSERTION_TOKEN'
 bound={'crates/alife_training/src/cpu/mod.rs':{}}
 m.BOUND=bound
 obj={'message':{'level':'error','message':secret,'rendered':secret,'children':[secret],'code':{'code':'E0308'},'spans':[{'is_primary':True,'file_name':'crates/alife_training/src/cpu/mod.rs','line_start':7,'text':[secret]}]}}
 rows=m.diagnostic_rows(obj,'owned76-test-build',pathlib.Path('/private/source'),bound)
 assert rows==[{'code':'E0308','path':'crates/alife_training/src/cpu/mod.rs','line':7,'stage':'owned76-test-build'}]
 saved=copy.deepcopy(m.STATE);m.STATE['diagnostics']=rows
 assert secret not in m.summary('CHECK');m.STATE['raw']=secret;assert secret not in m.summary('CHECK')
 m.STATE['stage']=secret;assert secret not in m.summary('CHECK');m.STATE.clear();m.STATE.update(saved)
 for filename in ['../../secret','/private/secret','crates/alife_training/unknown.rs']:
  obj['message']['spans'][0]['file_name']=filename;assert m.diagnostic_rows(obj,'owned76-test-build',pathlib.Path('/private/source'),bound)==[]
def source_contract():
 tree=ast.parse((ROOT/'run_hosted.py').read_text());f={n.name:ast.get_source_segment((ROOT/'run_hosted.py').read_text(),n) for n in tree.body if isinstance(n,ast.FunctionDef)}
 assert "stdout=subprocess.PIPE" in f['run'] and "stderr=subprocess.PIPE" in f['run']
 assert "subprocess.Popen" in f['run'] and 'OUTPUT_LIMIT' in f['run'] and 'kill_session(p)' in f['run']
 assert 'consumer(line)' in f['run'] and 'len(line)>4*1024**2' in f['run']
 assert "('PATH','HOME')" in f['main'] and 'GITHUB_TOKEN' not in f['main']
 assert "'--exact'" in f['run_tests'] and "'--no-run'" in f['build']
 assert m.LIMITS['free_bytes']==4*1024**3 and m.LIMITS['memavailable_bytes']==2*1024**3
 assert 'unlink(' not in (ROOT/'run_hosted.py').read_text()
 assert "if not __debug__: fail('SOURCE_CLOSURE')" in f['restore_sources']
 assert 'upload-artifact' not in (ROOT/'workflow.yml').read_text() if (ROOT/'workflow.yml').exists() else True
def dependency_paths():
    m.SOURCE=pathlib.Path('/guard/source');m.WORK=pathlib.Path('/guard/work');m.TARGET=m.WORK/'target';m.SYSROOT=pathlib.Path('/guard/sysroot')
    m.BOUND={'build.rs':{},'crates/alife_training/src/lib.rs':{}};m.LOCKED_REGISTRY={'serde-1.0.228'};m.DERIVED={m.TARGET/'debug/build/serde-123/out'}
    assert m.local_dep_path('build.rs')=='build.rs'
    assert m.local_dep_path('/guard/source/crates/alife_training/src/lib.rs')=='crates/alife_training/src/lib.rs'
    assert m.local_dep_path('/guard/work/cargo/registry/src/index.crates.io-abc123/serde-1.0.228/src/lib.rs') is None
    assert m.local_dep_path('/guard/sysroot/lib/rustlib/lib.rs') is None
    assert m.local_dep_path('/guard/work/target/debug/build/serde-123/out/generated.rs') is None
    for path in ['/unknown/private.rs','../private.rs','unbound.rs','/guard/work/cargo/git/checkouts/lib.rs','/guard/work/cargo/registry/src/index.crates.io-abc123/evil-1.0/src/lib.rs','/guard/work/target/unknown.rs']:
        reject('SOURCE_CLOSURE',lambda path=path:m.local_dep_path(path))
def selected_attributes():
    tests=json.loads((ROOT/'TEST-INVENTORY.json').read_text())['tests'];manifest=json.loads((ROOT/'source-manifest.json').read_text());bound={r['path']:r for r in manifest['owned76_files']}
    for t in tests:
        assert t['attributes']==['test'] and t['ignored'] is False and t['should_panic'] is False
        assert bound[t['source']]['sha256']==t['source_sha256']
        assert t['attribute_start_line']<t['function_start_line']
    workflow=ROOT/'workflow.yml'
    if workflow.exists():
        text=workflow.read_text();assert 'fetch-depth: 3' in text and "github.event.created == false" in text and "github.run_attempt == '1'" in text
        assert 'upload-artifact' not in text and 'workflow_dispatch' not in text and 'pull_request' not in text
def depinfo_grammar():
    m.SOURCE=pathlib.Path('/guard/source');m.WORK=pathlib.Path('/guard/work');m.TARGET=m.WORK/'target';m.SYSROOT=pathlib.Path('/guard/sysroot')
    m.BOUND={'Cargo.toml':{},'crates/x/Cargo.toml':{},'crates/x/src/lib.rs':{}};m.DERIVED={m.TARGET/'debug/build/x123/out'};m.ENV={};m.BUILD_ENV={}
    manifests={'/guard/source/Cargo.toml':'[workspace.package]\nversion="0.1.0"\n','/guard/source/crates/x/Cargo.toml':'[package]\nname="x"\nversion.workspace=true\n'}
    oldread=pathlib.Path.read_text;oldisfile=pathlib.Path.is_file
    pathlib.Path.read_text=lambda p,*a,**k:manifests[str(p)] if str(p) in manifests else oldread(p,*a,**k)
    pathlib.Path.is_file=lambda p:str(p) in manifests or oldisfile(p)
    try:
        source='/guard/source/crates/x/src/lib.rs'
        text='/guard/work/target/debug/deps/x.rlib: '+source+'\n\n'+source+':\n# env-dep:CARGO_MANIFEST_DIR=/guard/source/crates/x\n# env-dep:CARGO_PKG_VERSION=0.1.0\n# env-dep:CARGO_PKG_VERSION_PATCH=0\n# env-dep:ALIFE_CONSUMER_GIT_HEAD\n# env-dep:ALIFE_CONSUMER_GIT_TREE\n# env-dep:OUT_DIR=/guard/work/target/debug/build/x123/out\n'
        assert m.parse_depinfo(text)==[source]
        m.DERIVED.add(pathlib.Path('/guard/work/other-target/debug/build/x123/out'))
        reject('SOURCE_CLOSURE',lambda:m.parse_depinfo(text+'# env-dep:OUT_DIR=/guard/work/other-target/debug/build/x123/out\n'))
        for extra in ['# env-dep:UNKNOWN\n','# env-dep:CARGO_PKG_VERSION=99.0.0\n','# env-dep:ALIFE_CONSUMER_GIT_HEAD=secret\n','# env-dep:OUT_DIR=/unknown/out\n','/guard/work/target/standalone-without-colon\n','# arbitrary unbound comment\n']:
            reject('SOURCE_CLOSURE',lambda extra=extra:m.parse_depinfo(text+extra))
        continuation='/guard/work/target/x.rlib: '+source+' \\\n '+source+'\n'
        assert m.parse_depinfo(continuation)==[source,source]
    finally:pathlib.Path.read_text=oldread;pathlib.Path.is_file=oldisfile
def reconstruction_protocol():
    spec=importlib.util.spec_from_file_location('reconstruct_check',ROOT/'reconstruct_source.py');r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
    old={'Cargo.toml':b'old\n'};delta=b'diff --git a/Cargo.toml b/Cargo.toml\n--- a/Cargo.toml\n+++ b/Cargo.toml\n@@ -1 +1 @@\n-old\n+new\n'
    rec=lambda n,b:{'path':n,'bytes':len(b),'sha256':m.sha(b)}
    config={'checkout_files':[rec('Cargo.toml',b'old\n')],'steps':[{'parts':[rec('delta.patch',delta)],'patch_bytes':len(delta),'patch_sha256':m.sha(delta),'outputs':[rec('Cargo.toml',b'new\n')]}]}
    assert r.reconstruct_base(old,config,lambda name:delta,[rec('Cargo.toml',b'new\n')])=={'Cargo.toml':b'new\n'}
    def rejected(f):
        try:f()
        except (AssertionError,KeyError,UnicodeError,IndexError):return
        raise AssertionError('invalid reconstruction accepted')
    rejected(lambda:r.reconstruct_base({'Cargo.toml':b'wrong\n'},config,lambda name:delta,[rec('Cargo.toml',b'new\n')]))
    rejected(lambda:r.reconstruct_base(old,config,lambda name:delta+b'extra',[rec('Cargo.toml',b'new\n')]))
    rejected(lambda:r.reconstruct_base(old,config,lambda name:delta,[rec('Cargo.toml',b'wrong\n')]))
    rejected(lambda:r.apply(delta+delta,old.get))
    rejected(lambda:r.apply(delta.replace(b'b/Cargo.toml',b'b/../Cargo.toml'),old.get))
    rejected(lambda:r.apply(delta.replace(b'-old',b'-wrong'),old.get))
    manifest=json.loads((ROOT/'source-manifest.json').read_text());config=json.loads((ROOT/'RECONSTRUCTION.json').read_text())
    assert (len(manifest['base_files']),len(manifest['original60_files']),len(manifest['owned76_files']))==(799,813,818)
    assert config['checkout_commit']==m.MAIN and config['logical_local_base']==m.BASE and config['logical_ids_are_not_fetch_refs'] is True
    assert len(config['steps'])==3 and [x['id'] for x in config['steps']]==['main-to-fbf-library-closure','fbf-nominal-source-supplement','logical-base-source-only-delta']
    diff=json.loads((ROOT/'FUNCTION-DIFF.json').read_text());assert diff['changed_runner_functions']==['restore_sources','choose_role','run_tests','summary','main']
    for row in diff['identical_files']:assert m.sha((ROOT/row['path']).read_bytes())==row['sha256']
    text=(ROOT/'workflow.yml').read_text() if (ROOT/'workflow.yml').exists() else ''
    if text:
        assert 'ref: '+m.MAIN in text and 'ref: '+m.BASE not in text and 'ref: fbf2a517' not in text
def transport_protocol():
    d=json.loads((ROOT/'FUNCTION-DIFF.json').read_text());text=(ROOT/'run_hosted.py').read_text()
    f={n.name:ast.get_source_segment(text,n) for n in ast.parse(text).body if isinstance(n,(ast.FunctionDef,ast.ClassDef))}
    hashes={n:m.sha(v.encode()) for n,v in f.items()}
    assert hashes==d['v6_function_sha256'] and m.sha(text.encode())==d['v6_runner_sha256']
    assert [n for n in d['v5_function_sha256'] if hashes[n]!=d['v5_function_sha256'][n]]==['choose_role','main','restore_sources','run_tests','summary']
    assert set(hashes)-set(d['v5_function_sha256'])=={'cold_phase_rows','owned_patch'}
    for name in ['admit','run','build','setup_toolchain','parse_depinfo','verify_depinfo']:assert hashes[name]==d['v5_function_sha256'][name]
    assert "tests,test_hash=build('owned76',tests=True)" in f['main']
    assert "build('original60')" not in f['main'] and "run_probe(" not in f['main'] and "build('owned76');" not in f['main']
    assert "'--nocapture'" in f['run_tests'] and "len(tests)!=1" in f['run_tests']
    freeze=json.loads((ROOT/'TRANSPORT-FUNCTION-FREEZE.json').read_text());v=(ROOT/'verify_remote_inventory.py').read_text();assert m.sha(v.encode())==freeze['sha256']
    assert [{'name':n.name,'sha256':m.sha(ast.get_source_segment(v,n).encode())} for n in ast.parse(v).body if isinstance(n,ast.FunctionDef)]==freeze['functions']
def source_fix_protocol():
    fix=json.loads((ROOT/'SOURCE-FIX.json').read_text());manifest=json.loads((ROOT/'source-manifest.json').read_text())
    assert fix['runtime_production_files_changed']==0 and fix['original813_files_unchanged'] and fix['owned817_files_unchanged']
    assert next(r for r in manifest['owned76_files'] if r['path']==fix['source'])==fix['after']
    assert fix['assertions_removed']==0 and fix['existing_rejection_assertions_preserved']
    patch=m.owned_patch();assert len(patch)==fix['new_patch']['bytes'] and m.sha(patch)==fix['new_patch']['sha256']
    assert patch.count(b'*debit = serde_json::Value::from(0.0_f32);')==1
    assert patch.count(b'assert!(debit.as_f64().unwrap() > 0.0);')==1
    assert fix['native_verification'].startswith('NOT_RUN')
def phase_protocol():
    name=json.loads((ROOT/'TEST-INVENTORY.json').read_text())['tests'][0]['name'];expected=[a+f'{i:02}' for a in ('C','S') for i in range(23)]
    markers=''.join('M2_PLAY_COLD_V1:'+x[0]+':'+x[1:]+'\n' for x in expected)
    tail='ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 99 filtered out; finished in 0.01s\n\n'
    raw=('\nrunning 1 test\ntest '+name+' ... '+markers+tail).encode()
    clean,codes=m.cold_phase_rows(raw,True);assert codes==expected;m.test_outcome(clean,name)
    prefix=('\nrunning 1 test\ntest '+name+' ... '+markers.split('M2_PLAY_COLD_V1:C:20')[0]+'FAILED\nprivate assertion payload').encode()
    _,codes=m.cold_phase_rows(prefix,False);assert codes[-1]=='C19'
    saved=copy.deepcopy(m.STATE);m.STATE['cold_phases']=codes;out=m.summary('TEST');assert 'private assertion payload' not in out and 'C19' in out;m.STATE.clear();m.STATE.update(saved)
    for bad in [raw.replace(b'C:01',b'C:99'),raw.replace(b'C:01',b'C:00'),raw.replace(b'C:01',b'S:01'),raw.replace(b'M2_PLAY_COLD_V1:C:01\n',b''),raw+b'M2_PLAY_COLD_V1:Z:99\n']:
        reject('TEST_PROTOCOL',lambda bad=bad:m.cold_phase_rows(bad,True))
    reject('TEST_PROTOCOL',lambda:m.cold_phase_rows(prefix,True))
    saved=copy.deepcopy(m.STATE);m.STATE['cold_phases']=['SECRET'];assert 'SECRET' not in m.summary('TEST');m.STATE.clear();m.STATE.update(saved)
def owned_patch_transport_protocol():
    t=json.loads((ROOT/'OWNED-PATCH-TRANSPORT.json').read_text());patch=m.owned_patch()
    assert m.sha(patch)==t['effective']['sha256'] and len(patch)==t['effective']['bytes']
    assert t['source_byte_changes_from_original_v6']==0
    old=m.read_json
    try:
        for role in ('base','delta','effective'):
            for key,value in [('path','../wrong'),('bytes',0),('sha256','0'*64)]:
                bad=copy.deepcopy(t);bad[role][key]=value
                m.read_json=lambda p,bad=bad:bad
                reject('BINDING',m.owned_patch)
    finally:m.read_json=old
for name,fn in [('owned-patch-exact-transport',owned_patch_transport_protocol),('activation-rejects-creation-rerun-forge',admission),('tree-additions-only',tree),('tar-exact-inventory',archive),('exact-new05-and-strict-protocols',protocols),('public-diagnostic-projection',sanitation),('bounded-pipes-source-contract',source_contract),('outside-paths-fail-closed',dependency_paths),('selected-test-attributes',selected_attributes),('depinfo-env-and-make-grammar',depinfo_grammar),('reachable-main-exact-reconstruction',reconstruction_protocol),('owned-lib-single-test-continuation',transport_protocol),('typed-floating-debit-mutation',source_fix_protocol),('allowlisted-cold-phase-protocol',phase_protocol)]:ok(name,fn)
print(json.dumps({'schema':'m2-play-static-v1','passed':True,'checks':results,'native_invocations':0,'compiler_invocations':0,'github_writes':0,'synthetic_only':True},sort_keys=True))
