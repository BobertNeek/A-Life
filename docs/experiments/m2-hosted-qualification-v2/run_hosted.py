#!/usr/bin/env python3
"""Draft GitHub-only runner. No invocation occurred during source preparation.

Never prints native output, exception text, unapproved paths or assertion values.
Only pinned names/hashes, static enums, bounded scalar results and compiler codes
with primary locations in the exact bound source corpus can escape.
"""
import hashlib, json, os, pathlib, re, resource, shutil, signal, subprocess, sys, tempfile, time
ROOT=pathlib.Path(__file__).resolve().parent
MAIN='c3aec16df33861f1316d127736f5f60eea327ede'
LIMIT=12*1024**3
START=time.monotonic()
SAFE_CONTEXT={}
COMPLETED=[]
FIXTURE_NAMES=set()
BOUND_RECORDS={}
BRANCH='refs/heads/qualification/m2-fixed-fixtures-v3'
MARKER_PATH='docs/experiments/m2-hosted-qualification-v3/ACTIVATE.json'
PLAN_ID='m2-fixed-fixtures-v3'
TOOLCHAIN='1.99.1'
RUST_COMMIT='58cf8f9c35b355199727627d07ae34a4e2846d67'
RUST_DATE='2026-10-13'
RUST_HOST='x86_64-unknown-linux-gnu'
RUST_VERSION=f'rustc {TOOLCHAIN} ({RUST_COMMIT[:9]} {RUST_DATE})'
ACTION_COMMIT='b725eda525362e7cf34af9a7243cac1e3f01a7ec'
ACTION_SHA256='0b52547c145ae18d8631c9ab711e4d917e9e489cea800368f338c25a56e0aee2'
TOOLCHAIN_ROOT=None
PLATFORM_HOME=None
TOOLCHAIN_BYTES=0
TOOLCHAIN_LIMIT=3*1024**3  # Selected sysroot, monitored; platform caches are separate.
SETUP_STAGES={'rustup-discovery','rustup-home','official-recipe','install',
              'sysroot','rustc-version','cargo-version','binary-hashes','resource-accounting','ready'}
TOOLCHAIN_FAILURES={'TOOLCHAIN_RUSTUP','TOOLCHAIN_HOME','TOOLCHAIN_PROVENANCE','TOOLCHAIN_INSTALL',
                    'TOOLCHAIN_SYSROOT','TOOLCHAIN_VERSION','TOOLCHAIN_CARGO','TOOLCHAIN_HASH'}
MAX_OUTPUT=32*1024**2  # Monitored threshold, not a hard filesystem quota.
MAX_ERRORS=12
MAX_PRIMARY_LOCATIONS=3
ALLOWED={'boredom':6,'reflex':22}  # Meal and GDS qualified locally; both stay disabled.
IGNORED={
 'alife_world':{'headless::contact_chemistry_tests::contact_endpoint_cost_against_previous_hazard_scan',
                'headless::task_4_3a2_tests::mating_reach::mating_query_cpu_cost'},
 'alife_core':{'topology::lifecycle_tests::lifecycle_matched_timing_probe'},
 'alife_training':{
  'cpu::hotpath_bench::synthetic_cpu_hotpath_ledger',
  'cpu::replay::cadence_assay::paired_full_replay_measurement',
  'trainer::replay_packing_assay::single_call_packing_stages',
  'trainer::replay_packing_assay::tests::single_call_packing_validated_prefix',
  'trainer::replay_packing_assay::memory::single_call_packing_memory'}}
CODES={'OK','ADMISSION','BINDING','PATCH','TOOLCHAIN','BUILD','BUILD_PROTOCOL','LIST_PROTOCOL',
       'IGNORED_SET','TEST_FAILED','TEST_PROTOCOL','COVERAGE_PROTOCOL','TIME_LIMIT','OUTPUT_LIMIT',
       'DISK_LIMIT','SOURCE_CLOSURE','INTERNAL'} | TOOLCHAIN_FAILURES
class Stop(Exception): pass
def fail(code):
    assert code in CODES
    raise Stop(code)
def sha(b): return hashlib.sha256(b).hexdigest()
def load(p): return json.loads((ROOT/p).read_text())
def safe_relative_path(path):
    if not isinstance(path,str) or not re.fullmatch(r'[A-Za-z0-9_.\-/]{1,240}',path): return False
    p=pathlib.PurePosixPath(path)
    return not p.is_absolute() and str(p)==path and not any(x in {'.','..'} for x in p.parts)
def verify_bytes(data,row):
    if not isinstance(row,dict) or type(row.get('bytes')) is not int or row['bytes']<0 or not isinstance(row.get('sha256'),str) or not re.fullmatch('[0-9a-f]{64}',row['sha256']): fail('BINDING')
    if len(data)!=row['bytes'] or sha(data)!=row['sha256']: fail('BINDING')
def verify_record(root,row):
    if not isinstance(row,dict) or not safe_relative_path(row.get('path')): fail('BINDING')
    p=root/row['path']
    if not p.resolve().is_relative_to(root.resolve()) or not p.is_file() or p.is_symlink(): fail('BINDING')
    verify_bytes(p.read_bytes(),row)
def verify_seal(root,expected):
    data=(root/'seal.json').read_bytes()
    if not isinstance(expected,str) or not re.fullmatch('[0-9a-f]{64}',expected) or sha(data)!=expected: fail('BINDING')
    try: seal=json.loads(data)
    except (ValueError,UnicodeError): fail('BINDING')
    if not isinstance(seal,dict) or not isinstance(seal.get('files'),list) or not seal['files']: fail('BINDING')
    paths=[]
    for row in seal['files']:
        verify_record(root,row); paths.append(row['path'])
    if len(paths)!=len(set(paths)): fail('BINDING')
    return seal
def assemble_closure(root,meta):
    if not isinstance(meta,dict) or meta.get('transport')!='utf8-line-parts-v1' or not isinstance(meta.get('parts'),list) or len(meta['parts'])!=3: fail('BINDING')
    chunks=[]
    for i,row in enumerate(meta['parts'],1):
        if not isinstance(row,dict) or row.get('path')!=f'patches/main-to-fbf-library-closure.part-{i:02d}.patch': fail('BINDING')
        verify_record(root,row); data=(root/row['path']).read_bytes()
        if not data or len(data)>550*1024 or not data.endswith(b'\n') or b'\x00' in data: fail('BINDING')
        try: data.decode('utf-8','strict')
        except UnicodeError: fail('BINDING')
        chunks.append(data)
    data=b''.join(chunks); verify_bytes(data,meta)
    return data
def primary_source_path(name,source,allowed):
    if not isinstance(name,str): return None
    prefix=str(source).rstrip('/')+'/'
    if name.startswith(prefix): name=name[len(prefix):]
    if not safe_relative_path(name) or name not in allowed: return None
    return name
def project_cargo_errors(raw,source,allowed):
    """Pure allowlist projection; never returns diagnostic strings or snippets."""
    if not isinstance(raw,bytes) or len(raw)>MAX_OUTPUT: fail('BUILD_PROTOCOL')
    errors=[]; seen=set(); truncated=False; unclassified=False
    for line in raw.splitlines():
        try: obj=json.loads(line)
        except (ValueError,UnicodeError): fail('BUILD_PROTOCOL')
        if not isinstance(obj,dict) or obj.get('reason') not in {'compiler-artifact','compiler-message','build-script-executed','build-finished'}: fail('BUILD_PROTOCOL')
        if obj['reason']!='compiler-message': continue
        message=obj.get('message')
        if not isinstance(message,dict) or not isinstance(message.get('level'),str): fail('BUILD_PROTOCOL')
        if message['level']!='error': continue
        code=message.get('code')
        if code is None:
            unclassified=True; continue
        if not isinstance(code,dict) or not isinstance(code.get('code'),str) or not re.fullmatch(r'E\d{4}',code['code']): fail('BUILD_PROTOCOL')
        spans=message.get('spans')
        if not isinstance(spans,list): fail('BUILD_PROTOCOL')
        locations=[]
        for span in spans:
            if not isinstance(span,dict) or type(span.get('is_primary')) is not bool: fail('BUILD_PROTOCOL')
            if not span['is_primary']: continue
            path=primary_source_path(span.get('file_name'),source,allowed)
            line_no=span.get('line_start'); column=span.get('column_start')
            if path is None: continue
            if type(line_no) is not int or type(column) is not int or not 1<=line_no<=10_000_000 or not 1<=column<=1_000_000: fail('BUILD_PROTOCOL')
            location={'path':path,'line':line_no,'column':column}
            if location not in locations:
                if len(locations)<MAX_PRIMARY_LOCATIONS: locations.append(location)
                else: truncated=True
        row={'code':code['code'],'locations':locations}
        identity=json.dumps(row,sort_keys=True)
        if identity in seen: continue
        seen.add(identity)
        if len(errors)<MAX_ERRORS: errors.append(row)
        else: truncated=True
    return {'compiler_errors':errors,'compiler_errors_truncated':truncated,'unclassified_error_present':unclassified}
def validate_activation(env,event,head,parents,changed,marker,seal_digest,fixture_digest):
    expected_env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':BRANCH,'GITHUB_RUN_ATTEMPT':'1'}
    if any(env.get(k)!=v for k,v in expected_env.items()): fail('ADMISSION')
    if not isinstance(event,dict) or not isinstance(head,str) or not re.fullmatch('[0-9a-f]{40}',head): fail('ADMISSION')
    before=event.get('before','')
    if not isinstance(before,str) or not re.fullmatch('[0-9a-f]{40}',before) or before=='0'*40 or event.get('after')!=head or event.get('ref')!=BRANCH or any(event.get(x) is not False for x in ['created','deleted','forced']): fail('ADMISSION')
    if parents!=[head,before] or changed!='A\t'+MARKER_PATH+'\n': fail('ADMISSION')
    if not isinstance(seal_digest,str) or not re.fullmatch('[0-9a-f]{64}',seal_digest) or not isinstance(fixture_digest,str) or not re.fullmatch('[0-9a-f]{64}',fixture_digest): fail('BINDING')
    expected={'schema':'m2-hosted-activation-v1','plan_id':PLAN_ID,'prepared_head':before,'main':MAIN,
              'seal_sha256':seal_digest,'fixture_list_sha256':fixture_digest,'variants':['boredom','reflex']}
    if marker!=expected: fail('ADMISSION')
    return before
def failure_summary(code,context,completed,allowed_names,source_records):
    """Only pinned names, static enums and prevalidated code/location rows leave."""
    fallback={'code':'INTERNAL','completed_count':0,'completed_tests':[]}
    if not isinstance(code,str) or code not in CODES or code=='OK' or not isinstance(context,dict): return fallback
    keys={'variant','stage','package','test','compiler_errors','compiler_errors_truncated','unclassified_error_present','setup_stage','toolchain'}
    if not set(context)<=keys or not isinstance(completed,list) or len(completed)>28: return fallback
    if any(not isinstance(n,str) or n not in allowed_names for n in completed) or len(completed)!=len(set(completed)): return fallback
    out={'code':code,'completed_count':len(completed),'completed_tests':list(completed)}
    choices={'variant':set(ALLOWED),'stage':{'binding','toolchain','compile','listing','fixture'},'package':{'alife_core','alife_training','alife_world'},'setup_stage':SETUP_STAGES}
    for key,values in choices.items():
        if key in context:
            if not isinstance(context[key],str) or context[key] not in values: return fallback
            out[key]=context[key]
    if 'test' in context:
        if not isinstance(context['test'],str) or context['test'] not in allowed_names: return fallback
        out['test']=context['test']
    if 'compiler_errors' in context:
        if code not in {'BUILD','BUILD_PROTOCOL'} or context.get('stage')!='compile': return fallback
        errors=context['compiler_errors']
        if not isinstance(errors,list) or len(errors)>MAX_ERRORS: return fallback
        for row in errors:
            if not isinstance(row,dict) or set(row)!={'code','locations'} or not isinstance(row['code'],str) or not re.fullmatch(r'E\d{4}',row['code']) or not isinstance(row['locations'],list) or len(row['locations'])>MAX_PRIMARY_LOCATIONS: return fallback
            for loc in row['locations']:
                if not isinstance(loc,dict) or set(loc)!={'path','line','column'} or not safe_relative_path(loc['path']) or loc['path'] not in source_records or type(loc['line']) is not int or type(loc['column']) is not int or not 1<=loc['line']<=10_000_000 or not 1<=loc['column']<=1_000_000: return fallback
        out['compiler_errors']=errors
    for key in ['compiler_errors_truncated','unclassified_error_present']:
        if key in context:
            if 'compiler_errors' not in out or type(context[key]) is not bool: return fallback
            out[key]=context[key]
    if 'toolchain' in context:
        if not valid_toolchain_facts(context['toolchain']): return fallback
        out['toolchain']=dict(context['toolchain'])
    return out
def serialize_summary(result):
    try: text=json.dumps(result,sort_keys=True,separators=(',',':'),allow_nan=False)
    except (TypeError,ValueError): return '{"code":"INTERNAL"}'
    return text if len(text.encode('utf-8'))<=32768 else '{"code":"OUTPUT_LIMIT"}'
def bytes_used(path):
    return sum(p.stat().st_size for p in path.rglob('*') if p.is_file() and not p.is_symlink())
def limits():
    if time.monotonic()-START>50*60: fail('TIME_LIMIT')
    if shutil.disk_usage(WORK).free<2*1024**3 or bytes_used(WORK)+bytes_used(CHECKOUTS)>LIMIT: fail('DISK_LIMIT')
    if PLATFORM_HOME is not None and shutil.disk_usage(PLATFORM_HOME).free<2*1024**3: fail('DISK_LIMIT')
    if TOOLCHAIN_ROOT is not None and TOOLCHAIN_ROOT.exists() and bytes_used(TOOLCHAIN_ROOT)>TOOLCHAIN_LIMIT: fail('DISK_LIMIT')
def preexec():
    os.setsid()
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
def run(args,code,seconds=120,cwd=None,allow_nonzero=False):
    limits()
    # Files never become workflow artifacts. No tee, cat, tail or failure upload.
    with tempfile.TemporaryFile(dir=PRIVATE) as out, tempfile.TemporaryFile(dir=PRIVATE) as err:
        try: p=subprocess.Popen(args,cwd=cwd or SOURCE,env=ENV,stdout=out,stderr=err,preexec_fn=preexec)
        except OSError: fail(code)
        until=time.monotonic()+seconds
        try:
            while p.poll() is None:
                if time.monotonic()>until: fail('TIME_LIMIT')
                if out.tell()+err.tell()>MAX_OUTPUT: fail('OUTPUT_LIMIT')
                limits(); time.sleep(2)
            if out.tell()+err.tell()>MAX_OUTPUT: fail('OUTPUT_LIMIT')
            if p.returncode and not allow_nonzero: fail(code)
            out.seek(0); b=out.read(MAX_OUTPUT+1)
            if len(b)>MAX_OUTPUT: fail('OUTPUT_LIMIT')
            return (b,p.returncode) if allow_nonzero else b
        finally:
            if p.poll() is None:
                os.killpg(p.pid,signal.SIGKILL); p.wait()
def list_names(b):
    try: s=b.decode('utf-8','strict').strip().splitlines()
    except UnicodeError: fail('LIST_PROTOCOL')
    if not s: fail('LIST_PROTOCOL')
    names=[]; summaries=0
    for row in s:
        if not row: continue
        m=re.fullmatch(r'([A-Za-z0-9_:]+): test',row)
        if m: names.append(m[1]); continue
        m=re.fullmatch(r'(\d+) tests?, 0 benchmarks?',row)
        if not m or int(m[1])!=len(names) or row!=s[-1]: fail('LIST_PROTOCOL')
        summaries+=1
    if len(set(names))!=len(names) or summaries!=1: fail('LIST_PROTOCOL')
    return names
def test_result(b,name):
    try: s=b.decode('utf-8','strict')
    except UnicodeError: fail('TEST_PROTOCOL')
    if len(re.findall(r'^running 1 test$',s,re.M))!=1 or 'test '+name+' ... ' not in s: fail('TEST_PROTOCOL')
    rows=re.findall(r'^test result: (.*)$',s,re.M)
    if len(rows)!=1 or not re.fullmatch(r'ok\. 1 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out; finished in \d+(?:\.\d+)?s',rows[0]): fail('TEST_PROTOCOL')
    if s.strip().splitlines()[-1]!='test result: '+rows[0]: fail('TEST_PROTOCOL')
    return s
def patch(name,meta):
    if name=='closure':
        data=assemble_closure(ROOT,meta)
    else:
        verify_record(ROOT,meta); data=(ROOT/meta['path']).read_bytes()
    p=PRIVATE/'apply.patch'; p.write_bytes(data)
    run(['git','apply','--check',str(p)],'PATCH')
    run(['git','apply',str(p)],'PATCH'); p.unlink()
def compiled_local_inputs(exe,allowed):
    # Cargo's rustc dep-info gives the actual roots/includes used by this binary.
    # It must be a subset of the bound source corpus, never docs/**/*.rs scanning.
    d=pathlib.Path(str(exe)+'.d')
    if not d.is_file(): fail('SOURCE_CLOSURE')
    content=d.read_text()
    if '\\\n' in content or '\\ ' in content: fail('SOURCE_CLOSURE')
    found=0
    for token in content.replace(': ', '\n').split():
        token=token.rstrip(':')
        if token.startswith(str(SOURCE)+'/'):
            resolved=pathlib.Path(token).resolve()
        elif token.startswith(('crates/','docs/','assets/')):
            resolved=(SOURCE/token).resolve()
        else: continue
        if not resolved.is_relative_to(SOURCE): fail('SOURCE_CLOSURE')
        q=resolved.relative_to(SOURCE).as_posix()
        if q not in allowed: fail('SOURCE_CLOSURE')
        found+=1
    if not found: fail('SOURCE_CLOSURE')
def build(package):
    cmd=toolchain_command('cargo','test','--locked','--lib','-p',package,'--no-run','--message-format=json')
    if package=='alife_training': cmd+=['--features','cpu-backend']
    raw,status=run(cmd,'BUILD',seconds=30*60,allow_nonzero=True)
    if status:
        projection=project_cargo_errors(raw,SOURCE,BOUND_RECORDS)
        SAFE_CONTEXT.update(projection)
        fail('BUILD')
    candidates=[]
    for line in raw.splitlines():
        try: obj=json.loads(line)
        except Exception: fail('BUILD_PROTOCOL')
        if obj.get('reason') not in {'compiler-artifact','compiler-message','build-script-executed','build-finished'}: fail('BUILD_PROTOCOL')
        if obj.get('reason')=='compiler-artifact' and obj.get('target',{}).get('name')==package and obj.get('profile',{}).get('test') and obj.get('executable'):
            if obj['target'].get('kind')!=['lib']: fail('BUILD_PROTOCOL')
            expected=['cpu-backend','default','receipt-lineage'] if package=='alife_training' else []
            if sorted(obj['features'])!=expected: fail('BUILD_PROTOCOL')
            candidates.append(pathlib.Path(obj['executable']))
    if len(candidates)!=1: fail('BUILD_PROTOCOL')
    exe=candidates[0].resolve()
    if not exe.is_relative_to(WORK/'target') or not exe.is_file(): fail('BUILD_PROTOCOL')
    return exe
def normal_toolchain_env(original,work,private):
    # Keep the hosted platform HOME/rustup context only while locating/installing
    # the official compiler. Cargo storage, temp and targets are private already.
    if not original.get('PATH') or not original.get('HOME'): fail('TOOLCHAIN_HOME')
    env={k:original[k] for k in ['PATH','HOME','RUSTUP_HOME'] if k in original}
    env.update({'CARGO_HOME':str(work/'cargo'),'CARGO_TARGET_DIR':str(work/'target'),
        'CARGO_INCREMENTAL':'0','CARGO_BUILD_JOBS':'2','CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0',
        'RUST_TEST_THREADS':'1','RUST_BACKTRACE':'0','LANG':'C.UTF-8','LC_ALL':'C.UTF-8',
        'TMPDIR':str(private),'TMP':str(private),'TEMP':str(private),
        'RUSTUP_DIST_SERVER':'https://static.rust-lang.org',
        'RUSTUP_UPDATE_ROOT':'https://static.rust-lang.org/rustup',
        'CARGO_TERM_COLOR':'never','RUSTUP_NO_UPDATE_CHECK':'1','RUSTUP_PERMIT_COPY_RENAME':'1'})
    return env

def private_toolchain_env(env,work,home,sysroot):
    out=dict(env)
    out.update({'HOME':str(work/'home'),'RUSTUP_HOME':str(home),
                'RUSTC':str(sysroot/'bin'/'rustc'),'RUSTDOC':str(sysroot/'bin'/'rustdoc')})
    return out

def toolchain_command(program,*args):
    if program not in {'rustc','cargo'}: fail('INTERNAL')
    return [RUSTUP,'run',TOOLCHAIN,program,*args]

def install_command():
    # Exact minimal-install recipe from the verified official action, no retry,
    # bootstrap, default mutation, extra component, or self-update.
    return [RUSTUP,'toolchain','install',TOOLCHAIN,'--profile','minimal','--no-self-update','--force-non-host']

def single_path(raw,code):
    if not isinstance(raw,bytes) or len(raw)>4096: fail(code)
    try: text=raw.decode('utf-8','strict').rstrip('\n')
    except UnicodeError: fail(code)
    if not text or any(ord(c)<32 or ord(c)==127 for c in text) or not pathlib.Path(text).is_absolute(): fail(code)
    if str(pathlib.PurePosixPath(text))!=text or any(part in {'.','..'} for part in pathlib.PurePosixPath(text).parts): fail(code)
    return pathlib.Path(text)

def validate_layout(home,sysroot):
    if sysroot!=home/'toolchains'/(TOOLCHAIN+'-'+RUST_HOST): fail('TOOLCHAIN_SYSROOT')
    return sysroot

def parse_rustc_version(raw):
    if not isinstance(raw,bytes) or len(raw)>2048: fail('TOOLCHAIN_VERSION')
    try: lines=raw.decode('ascii','strict').splitlines()
    except UnicodeError: fail('TOOLCHAIN_VERSION')
    if len(lines)!=7 or lines[0]!=RUST_VERSION: fail('TOOLCHAIN_VERSION')
    fields={}
    for line in lines[1:]:
        if ': ' not in line: fail('TOOLCHAIN_VERSION')
        key,value=line.split(': ',1)
        if key in fields: fail('TOOLCHAIN_VERSION')
        fields[key]=value
    expected={'binary':'rustc','commit-hash':RUST_COMMIT,'commit-date':RUST_DATE,'host':RUST_HOST,'release':TOOLCHAIN}
    if any(fields.get(k)!=v for k,v in expected.items()) or set(fields)!=set(expected)|{'LLVM version'}: fail('TOOLCHAIN_VERSION')
    if not re.fullmatch(r'[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}',fields['LLVM version']): fail('TOOLCHAIN_VERSION')
    return {'rustc_version':RUST_VERSION,'rustc_commit':RUST_COMMIT,'rustc_host':RUST_HOST,'llvm_version':fields['LLVM version']}

def parse_cargo_version(raw):
    if not isinstance(raw,bytes) or len(raw)>256: fail('TOOLCHAIN_CARGO')
    try: value=raw.decode('ascii','strict').strip()
    except UnicodeError: fail('TOOLCHAIN_CARGO')
    if not re.fullmatch(r'cargo 1\.99\.1 \([0-9a-f]{9,40} \d{4}-\d{2}-\d{2}\)',value): fail('TOOLCHAIN_CARGO')
    return value

def file_sha(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''): h.update(block)
    return h.hexdigest()

def valid_toolchain_facts(facts):
    expected={'rustc_version','rustc_commit','rustc_host','llvm_version','cargo_version',
              'rustc_binary_sha256','cargo_binary_sha256','selected_sysroot_bytes',
              'platform_toolchain_storage','action_commit'}
    if not isinstance(facts,dict) or set(facts)!=expected: return False
    if facts['rustc_version']!=RUST_VERSION or facts['rustc_commit']!=RUST_COMMIT or facts['rustc_host']!=RUST_HOST or facts['action_commit']!=ACTION_COMMIT: return False
    if facts['platform_toolchain_storage']!='separate-normal-hosted-home': return False
    if not isinstance(facts['llvm_version'],str) or not re.fullmatch(r'[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}',facts['llvm_version']): return False
    if not isinstance(facts['cargo_version'],str) or not re.fullmatch(r'cargo 1\.99\.1 \([0-9a-f]{9,40} \d{4}-\d{2}-\d{2}\)',facts['cargo_version']): return False
    if any(not isinstance(facts[k],str) or not re.fullmatch('[0-9a-f]{64}',facts[k]) for k in ['rustc_binary_sha256','cargo_binary_sha256']): return False
    return type(facts['selected_sysroot_bytes']) is int and 0<facts['selected_sysroot_bytes']<=TOOLCHAIN_LIMIT

def stage(name):
    if name not in SETUP_STAGES: fail('INTERNAL')
    SAFE_CONTEXT.update(stage='toolchain',setup_stage=name)

def discover_toolchain(original,install=False):
    global ENV,RUSTUP,TOOLCHAIN_ROOT,TOOLCHAIN_BYTES,PLATFORM_HOME
    stage('rustup-discovery')
    executable=shutil.which('rustup',path=original.get('PATH',''))
    if not executable: fail('TOOLCHAIN_RUSTUP')
    try: RUSTUP=str(pathlib.Path(executable).resolve(strict=True))
    except OSError: fail('TOOLCHAIN_RUSTUP')
    if not pathlib.Path(RUSTUP).is_file(): fail('TOOLCHAIN_RUSTUP')
    ENV=normal_toolchain_env(original,WORK,PRIVATE)
    stage('rustup-home')
    try: home=single_path(run([RUSTUP,'show','home'],'TOOLCHAIN_HOME'),'TOOLCHAIN_HOME').resolve(strict=True)
    except OSError: fail('TOOLCHAIN_HOME')
    if not home.is_dir() or home.is_relative_to(WORK) or home.is_relative_to(CHECKOUTS): fail('TOOLCHAIN_HOME')
    ENV['RUSTUP_HOME']=str(home)
    PLATFORM_HOME=home
    TOOLCHAIN_ROOT=home/'toolchains'/(TOOLCHAIN+'-'+RUST_HOST)
    limits()
    stage('official-recipe')
    try: action_bytes=(ROOT/'upstream-toolchain-action.yml').read_bytes()
    except OSError: fail('TOOLCHAIN_PROVENANCE')
    if sha(action_bytes)!=ACTION_SHA256: fail('TOOLCHAIN_PROVENANCE')
    if install:
        stage('install')
        run(install_command(),'TOOLCHAIN_INSTALL',seconds=600)
    stage('sysroot')
    try: sysroot=single_path(run(toolchain_command('rustc','--print','sysroot'),'TOOLCHAIN_SYSROOT'),'TOOLCHAIN_SYSROOT').resolve(strict=True)
    except OSError: fail('TOOLCHAIN_SYSROOT')
    validate_layout(home,sysroot)
    for binary in ['rustc','cargo','rustdoc']:
        p=sysroot/'bin'/binary
        if not p.is_file() or p.is_symlink() or not os.access(p,os.X_OK): fail('TOOLCHAIN_SYSROOT')
    TOOLCHAIN_ROOT=sysroot
    ENV=private_toolchain_env(ENV,WORK,home,sysroot)
    stage('rustc-version')
    facts=parse_rustc_version(run(toolchain_command('rustc','--version','--verbose'),'TOOLCHAIN_VERSION'))
    stage('cargo-version')
    facts['cargo_version']=parse_cargo_version(run(toolchain_command('cargo','--version'),'TOOLCHAIN_CARGO'))
    stage('binary-hashes')
    try:
        facts.update(rustc_binary_sha256=file_sha(sysroot/'bin'/'rustc'),cargo_binary_sha256=file_sha(sysroot/'bin'/'cargo'))
    except OSError: fail('TOOLCHAIN_HASH')
    stage('resource-accounting')
    TOOLCHAIN_BYTES=bytes_used(sysroot);limits()
    facts.update(selected_sysroot_bytes=TOOLCHAIN_BYTES,platform_toolchain_storage='separate-normal-hosted-home',action_commit=ACTION_COMMIT)
    if not valid_toolchain_facts(facts): fail('TOOLCHAIN_VERSION')
    SAFE_CONTEXT['toolchain']=facts
    stage('ready')
    return facts

def initialize(variant,source):
    global WORK,PRIVATE,SOURCE,ENV,CHECKOUTS
    if variant not in ALLOWED or os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_RUN_ATTEMPT')!='1': fail('ADMISSION')
    SAFE_CONTEXT.update(variant=variant,stage='binding')
    SOURCE=pathlib.Path(source).resolve()
    CHECKOUTS=pathlib.Path(os.environ['GITHUB_WORKSPACE']).resolve()
    WORK=pathlib.Path(os.environ['RUNNER_TEMP'])/('m2-fixed-fixtures-v3-'+variant)
    if WORK.is_symlink(): fail('BINDING')
    WORK.mkdir(mode=0o700,exist_ok=True)
    PRIVATE=WORK/'private'
    for p in [PRIVATE,WORK/'home',WORK/'cargo']:
        if p.is_symlink(): fail('BINDING')
        p.mkdir(mode=0o700,exist_ok=True)
    ENV=normal_toolchain_env(os.environ,WORK,PRIVATE)
    ENV['HOME']=str(WORK/'home')

def admit():
    head=os.environ.get('GITHUB_SHA','')
    if not re.fullmatch('[0-9a-f]{40}',head): fail('ADMISSION')
    verify_seal(ROOT,os.environ.get('EXPECTED_SEAL_SHA256'))
    event=json.loads(pathlib.Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    before=event.get('before','')
    if not isinstance(before,str) or not re.fullmatch('[0-9a-f]{40}',before): fail('ADMISSION')
    parents=run(['git','rev-list','--parents','-n','1','HEAD'],'ADMISSION',cwd=ROOT).decode().strip().split()
    changed=run(['git','diff','--name-status',before,head],'ADMISSION',cwd=ROOT).decode()
    marker_path=CHECKOUTS/'bundle'/MARKER_PATH
    marker=json.loads(marker_path.read_bytes())
    before=validate_activation(os.environ,event,head,parents,changed,marker,
        sha((ROOT/'seal.json').read_bytes()),sha((ROOT/'fixtures.json').read_bytes()))
    return head,before,sha(marker_path.read_bytes())

def setup():
    # Separate workflow step, before the private native runner. No Rust build,
    # test, source fixture or autonomous executable is invoked by this entry.
    variant=sys.argv[2] if len(sys.argv)>2 else ''
    initialize(variant,sys.argv[3])
    head,before,marker_hash=admit()
    facts=discover_toolchain(os.environ,install=True)
    return {'code':'OK','stage':'toolchain','setup_stage':'ready','variant':variant,
            'plan_id':PLAN_ID,'qualification_head':head,'prepared_head':before,
            'activation_marker_sha256':marker_hash,'toolchain':facts,'native_builds':0,'fixture_cases':0}

def main():
    global FIXTURE_NAMES,BOUND_RECORDS
    variant=sys.argv[1] if len(sys.argv)>1 else ''
    initialize(variant,sys.argv[2])
    head,before,marker_hash=admit()
    if run(['git','rev-parse','HEAD'],'BINDING').decode().strip()!=MAIN or run(['git','status','--porcelain'],'BINDING').strip(): fail('BINDING')
    closure=load('closure.json'); patch('closure',closure['patch'])
    for row in closure['files']: verify_record(SOURCE,row)
    patches=load('patches.json')
    for p in (['meal-core','meal-host'] if variant=='meal' else [variant]): patch(p,patches[p])
    records={r['path']:r for r in load('source-base.json')['bound_source_superset']}
    binding=load('source-'+variant+'.json')
    if binding['base_manifest_sha256']!=sha((ROOT/'source-base.json').read_bytes()): fail('BINDING')
    records.update({r['path']:r for r in binding['overrides']})
    for row in records.values(): verify_record(SOURCE,row)
    BOUND_RECORDS=records
    fixtures=[f for f in load('fixtures.json')['fixtures'] if f['variant']==variant]
    if len(fixtures)!=ALLOWED[variant] or any(f['ignored'] for f in fixtures): fail('BINDING')
    FIXTURE_NAMES={f['name'] for f in fixtures}
    if len(FIXTURE_NAMES)!=len(fixtures): fail('BINDING')
    facts=discover_toolchain(os.environ,install=False)
    SAFE_CONTEXT.pop('setup_stage',None)
    completed=COMPLETED; coverage=None; reported_ignored=[]; harnesses={}
    for package in sorted({f['package'] for f in fixtures}):
        SAFE_CONTEXT.pop('test',None)
        SAFE_CONTEXT.update(stage='compile',package=package)
        exe=build(package); compiled_local_inputs(exe,records)
        harnesses[package]=sha(exe.read_bytes())
        SAFE_CONTEXT['stage']='listing'
        ignored=set(list_names(run([str(exe),'--list','--ignored','--format','terse'],'LIST_PROTOCOL')))
        if ignored!=IGNORED[package]: fail('IGNORED_SET')
        reported_ignored+=sorted(ignored)
        for f in (x for x in fixtures if x['package']==package):
            name=f['name']
            SAFE_CONTEXT.update(stage='fixture',test=name)
            if list_names(run([str(exe),'--list','--format','terse','--exact',name],'LIST_PROTOCOL'))!=[name]: fail('LIST_PROTOCOL')
            raw=run([str(exe),'--exact',name,'--test-threads=1','--nocapture'],'TEST_FAILED',seconds=300)
            s=test_result(raw,name)
            if name.endswith('meal_actual_sampled_fixed_scene_reports_physical_and_association_coverage_separately'):
                rows=re.findall(r'actual-sampled fixed-source decisions=(\d+) physical-consumed=(\d+) supplemental-proven=(\d+) supplemental-retained=(\d+); branch-covered=(true|false)',s)
                if len(rows)!=1: fail('COVERAGE_PROTOCOL')
                values=[int(x) for x in rows[0][:4]]
                if not 0<=values[3]<=values[2]<=values[1]<=values[0]<=8 or (rows[0][4]=='true')!=(values[2]>0): fail('COVERAGE_PROTOCOL')
                coverage=dict(zip(['decisions','physical_consumed','supplemental_proven','supplemental_retained'],values))
                coverage['branch_covered']=rows[0][4]=='true'
            completed.append(name)
    if len(completed)!=ALLOWED[variant]: fail('TEST_PROTOCOL')
    return {'code':'OK','variant':variant,'qualification_head':head,'main':MAIN,
        'source_binding_sha256':sha((ROOT/('source-'+variant+'.json')).read_bytes()),
        'fixture_list_sha256':sha((ROOT/'fixtures.json').read_bytes()),'passed':completed,
        'seal_sha256':sha((ROOT/'seal.json').read_bytes()),'toolchain':facts,'harness_sha256':harnesses,
        'prepared_head':before,'activation_marker_sha256':marker_hash,
        'reported_not_run_ignored':reported_ignored,'sampled_meal_coverage':coverage,
        'gpu_hardware_claim':False,'scientific_study':False}
if __name__=='__main__':
    try: result=setup() if len(sys.argv)>1 and sys.argv[1]=='--setup' else main()
    except Stop as e: result=failure_summary(str(e),SAFE_CONTEXT,COMPLETED,FIXTURE_NAMES,BOUND_RECORDS)
    except BaseException: result=failure_summary('INTERNAL',SAFE_CONTEXT,COMPLETED,FIXTURE_NAMES,BOUND_RECORDS)
    text=serialize_summary(result)
    print(text)
    summary=os.environ.get('GITHUB_STEP_SUMMARY')
    if summary:
        with open(summary,'a') as f: f.write(text+'\n')
    sys.exit(0 if result.get('code')=='OK' else 1)
