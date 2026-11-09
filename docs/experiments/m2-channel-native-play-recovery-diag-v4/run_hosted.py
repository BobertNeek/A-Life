#!/usr/bin/env python3
"""Frozen, admitted hosted-only native qualification. Import executes no work.
Only one fixed controlled physics fixture may execute, with exactly six intervals. Raw child
pipes stay bounded and private; only allowlisted public JSON is emitted.
"""
import math,hashlib,json,os,pathlib,re,resource,selectors,shutil,signal,subprocess,sys,time,tarfile,io,tomllib
ROOT=pathlib.Path(__file__).resolve().parent
PLAN="m2-channel-native-play-recovery-diag-v4"
BRANCH="refs/heads/qualification/"+PLAN
PACKET="docs/experiments/"+PLAN
MARKER=PACKET+"/ACTIVATE.json"
MAIN="c3aec16df33861f1316d127736f5f60eea327ede"
BASE="c3aec16df33861f1316d127736f5f60eea327ede"
TOOLCHAIN="1.99.1"
RUST_COMMIT="58cf8f9c35b355199727627d07ae34a4e2846d67"
RUST_DATE="2026-10-13"
HOST="x86_64-unknown-linux-gnu"
RUST_VERSION=f"rustc {TOOLCHAIN} ({RUST_COMMIT[:9]} {RUST_DATE})"
CARGO_VERSION="cargo 1.99.1 (5f94df478 2026-08-27)"
REUSE_COMMIT="80332c3a4b30a8a84f5bcb6a7ee0a20a47c7b19d"
REUSE_PACKET="docs/experiments/m2-channel-native-hosted-pure-v1"
REUSE=None
ACTION_SHA="0b52547c145ae18d8631c9ab711e4d917e9e489cea800368f338c25a56e0aee2"
LIMITS={"wall_seconds":5400,"rss_bytes":6*1024**3,"workspace_bytes":16*1024**3,"free_bytes":4*1024**3,"memavailable_bytes":2*1024**3,"initial_free_bytes":20*1024**3,"output_bytes":32*1024**2,"summary_bytes":32768,"toolchain_bytes":3*1024**3,"poll_seconds":0.25,"compile_seconds":2700,"fixture_seconds":300}
CODES={"OK","ADMISSION","BINDING","PATCH","SOURCE_CLOSURE","TOOLCHAIN","CHECK","CHECK_PROTOCOL","FEATURES","TIME_LIMIT","RSS_LIMIT","DISK_LIMIT","MEM_LIMIT","OUTPUT_LIMIT","TEST","TEST_PROTOCOL","PROBE_PROTOCOL","DEFAULT_PARITY","INTERNAL"}
START=time.monotonic()
STATE={"stage":"admission","completed":[],"diagnostics":[],"peak_rss_bytes":0,"peak_workspace_bytes":0,"output_bytes":0,"test_results":[],"probe_results":[],"test_processes_attempted":0,"probe_processes_attempted":0}
WORKSPACE=WORK=SOURCE=SYSROOT=TARGET=None
ENV={};BOUND={};RUSTUP=None;DERIVED=set();LOCKED_REGISTRY=set();BUILD_ENV={}


TEST_NAMES=['cpu::host::grouped_host_envelope::channel_native::tests::controlled::grounded_play_and_quiet_awake_rest_share_prefix_and_cold_source']
ALL_TEST_NAMES=sorted(TEST_NAMES)
class Stop(Exception):
    pass

def fail(code):
    raise Stop(code if code in CODES else 'INTERNAL')

def sha(data):
    return hashlib.sha256(data).hexdigest()

def digest_file(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()

def safe_path(path):
    if not isinstance(path, str) or not re.fullmatch(r'[A-Za-z0-9_.\-/]{1,240}', path):
        return False
    value = pathlib.PurePosixPath(path)
    return not value.is_absolute() and str(value) == path and '..' not in value.parts

def read_json(path):
    try:
        return json.loads(path.read_bytes())
    except (OSError, ValueError, UnicodeError):
        fail('BINDING')

def verify_record(root, row):
    if not isinstance(row, dict) or not safe_path(row.get('path')):
        fail('BINDING')
    p = root / row['path']
    if not p.is_file() or p.is_symlink() or not p.resolve().is_relative_to(root.resolve()):
        fail('SOURCE_CLOSURE')
    if p.stat().st_size != row.get('bytes') or digest_file(p) != row.get('sha256'):
        fail('BINDING')

def size(path):
    if path is None or not path.exists():
        return 0
    total = 0
    for root, dirs, files in os.walk(path, followlinks=False):
        for name in files:
            p = pathlib.Path(root) / name
            try:
                if not p.is_symlink():
                    total += p.stat().st_size
            except FileNotFoundError:
                pass
    return total

def session_rss(sid):
    """Sample all processes in the child session, including compiler descendants."""
    total = 0
    for proc in pathlib.Path('/proc').iterdir():
        if not proc.name.isdigit():
            continue
        try:
            fields = (proc / 'stat').read_text().rsplit(')', 1)[1].split()
            if int(fields[3]) == sid or int(proc.name) == os.getpid():
                total += int(fields[21]) * os.sysconf('SC_PAGE_SIZE')
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue
    return total

def kill_session(process):
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    for entry in pathlib.Path('/proc').iterdir():
        if not entry.name.isdigit():
            continue
        try:
            fields = (entry / 'stat').read_text().rsplit(')', 1)[1].split()
            if int(fields[3]) == process.pid:
                os.kill(int(entry.name), signal.SIGKILL)
        except (ProcessLookupError, FileNotFoundError, PermissionError):
            pass
    process.wait()

def child_setup():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_FSIZE, (2 * 1024**3, 2 * 1024**3))

def run(args, code, *, cwd=None, seconds=5400, capture=False, consumer=None, allow_nonzero=False, pipe_limit=None):
    """Pipes are bounded in memory; stderr is counted then discarded, never logged."""
    check_limits()
    call_output = 0
    output = bytearray()
    pending = bytearray()
    deadline = min(START + LIMITS['wall_seconds'], time.monotonic() + seconds)
    p = subprocess.Popen(args, cwd=cwd or SOURCE, env=ENV, stdout=subprocess.PIPE,
                         stderr=subprocess.PIPE, start_new_session=True, preexec_fn=child_setup)
    poller = selectors.DefaultSelector()
    for stream in (p.stdout, p.stderr):
        os.set_blocking(stream.fileno(), False)
        poller.register(stream, selectors.EVENT_READ)
    next_disk = 0.0
    try:
        while poller.get_map() or p.poll() is None:
            if time.monotonic() >= deadline:
                fail('TIME_LIMIT')
            rss = session_rss(p.pid)
            STATE['peak_rss_bytes'] = max(STATE['peak_rss_bytes'], rss)
            if rss > LIMITS['rss_bytes']:
                fail('RSS_LIMIT')
            if time.monotonic() >= next_disk:
                check_limits(p.pid); next_disk = time.monotonic() + 2
            for key, _ in poller.select(LIMITS['poll_seconds']):
                chunk = os.read(key.fileobj.fileno(), 65536)
                if not chunk:
                    poller.unregister(key.fileobj); continue
                call_output += len(chunk)
                if pipe_limit is not None and call_output > pipe_limit: fail('OUTPUT_LIMIT')
                STATE['output_bytes'] += len(chunk)
                if STATE['output_bytes'] > LIMITS['output_bytes']:
                    fail('OUTPUT_LIMIT')
                if key.fileobj is p.stdout:
                    if capture:
                        output.extend(chunk)
                        if len(output) > 2 * 1024**2:
                            fail('OUTPUT_LIMIT')
                    if consumer:
                        pending.extend(chunk)
                        while b'\n' in pending:
                            line, _, rest = pending.partition(b'\n'); pending = bytearray(rest)
                            
                            if len(line)>4*1024**2: fail('OUTPUT_LIMIT')
                            consumer(line)
                        if len(pending) > 4 * 1024**2:
                            fail('OUTPUT_LIMIT')
        if pending and consumer:
            consumer(bytes(pending))
        status = p.wait()
        check_limits()
        if status and not allow_nonzero:
            fail(code)
        return bytes(output), status
    finally:
        kill_session(p)
        poller.close()
        p.stdout.close(); p.stderr.close()

def private_git(*args, cwd=None):
    data, _ = run(['git', *args], 'BINDING', cwd=cwd, seconds=120, capture=True)
    return data

def exact_path(raw):
    value = raw.decode('utf-8', 'strict').rstrip('\n')
    path = pathlib.Path(value)
    if not path.is_absolute() or '\n' in value or any(ord(x) < 32 for x in value):
        fail('TOOLCHAIN')
    return path.resolve(strict=True)

def setup_toolchain(original):
    global ENV, SYSROOT, RUSTUP
    STATE['stage'] = 'toolchain'
    found = shutil.which('rustup', path=original.get('PATH', ''))
    if not found or digest_file(REUSE / 'upstream-toolchain-action.yml') != ACTION_SHA:
        fail('TOOLCHAIN')
    RUSTUP = str(pathlib.Path(found).resolve(strict=True))
    ENV['HOME'] = original['HOME']
    if 'RUSTUP_HOME' in original:
        ENV['RUSTUP_HOME'] = original['RUSTUP_HOME']
    home = exact_path(run([RUSTUP, 'show', 'home'], 'TOOLCHAIN', capture=True, seconds=60)[0])
    if home.is_relative_to(WORK) or home.is_relative_to(WORKSPACE):
        fail('TOOLCHAIN')
    ENV['RUSTUP_HOME'] = str(home)
    SYSROOT = home / 'toolchains' / (TOOLCHAIN + '-' + HOST)
    # Same verified official recipe as the prior successful setup; no bootstrap,
    # self-update, default mutation, retry, or reuse of any engine build products.
    run([RUSTUP, 'toolchain', 'install', TOOLCHAIN, '--profile', 'minimal',
         '--no-self-update', '--force-non-host'], 'TOOLCHAIN', seconds=600)
    sysroot = exact_path(run([RUSTUP, 'run', TOOLCHAIN, 'rustc', '--print', 'sysroot'],
                             'TOOLCHAIN', capture=True, seconds=60)[0])
    if sysroot != SYSROOT.resolve(strict=True):
        fail('TOOLCHAIN')
    for name in ('rustc', 'cargo', 'rustdoc'):
        p = sysroot / 'bin' / name
        if not p.is_file() or p.is_symlink() or not os.access(p, os.X_OK):
            fail('TOOLCHAIN')
    raw = run([RUSTUP, 'run', TOOLCHAIN, 'rustc', '--version', '--verbose'],
              'TOOLCHAIN', capture=True, seconds=60)[0].decode('ascii').strip().splitlines()
    fields = dict(line.split(': ', 1) for line in raw[1:])
    expected = {'binary': 'rustc', 'commit-hash': RUST_COMMIT, 'commit-date': RUST_DATE,
                'host': HOST, 'release': TOOLCHAIN, 'LLVM version': '23.1.1'}
    if raw[0] != RUST_VERSION or fields != expected:
        fail('TOOLCHAIN')
    cargo = run([RUSTUP, 'run', TOOLCHAIN, 'cargo', '--version'],
                'TOOLCHAIN', capture=True, seconds=60)[0].decode('ascii').strip()
    if cargo != CARGO_VERSION:
        fail('TOOLCHAIN')
    ENV.update(HOME=str(WORK / 'home'), RUSTC=str(sysroot / 'bin' / 'rustc'),
               RUSTDOC=str(sysroot / 'bin' / 'rustdoc'))
    STATE['toolchain'] = {'rustc': RUST_VERSION, 'rust_commit': RUST_COMMIT,
                          'cargo': CARGO_VERSION, 'host': HOST, 'llvm': '23.1.1',
                          'rustc_binary_sha256': digest_file(sysroot / 'bin' / 'rustc'),
                          'cargo_binary_sha256': digest_file(sysroot / 'bin' / 'cargo')}
    check_limits()

def diagnostic_rows(obj, stage, source, bound):
    """The sole compiler projection: code + normalized crate path/line + stage."""
    message = obj.get('message', {})
    if message.get('level') != 'error':
        return []
    code = (message.get('code') or {}).get('code')
    if not isinstance(code, str) or not re.fullmatch(r'E\d{4}', code):
        return []
    rows = []
    for span in message.get('spans', []):
        if span.get('is_primary') is not True:
            continue
        name = span.get('file_name')
        if not isinstance(name, str):
            continue
        prefix = str(source) + '/'
        if name.startswith(prefix):
            name = name[len(prefix):]
        line = span.get('line_start')
        if (safe_path(name) and re.fullmatch(r'crates/alife_[a-z0-9_]+/[A-Za-z0-9_.\-/]+', name)
                and name in bound and type(line) is int and 1 <= line <= 10000000):
            row = {'code': code, 'path': name, 'line': line, 'stage': stage}
            if row not in rows:
                rows.append(row)
            if len(rows) == 3:
                break
    return rows

def check_limits(sid=None):
    if time.monotonic()-START >= LIMITS['wall_seconds']: fail('TIME_LIMIT')
    if sid is not None:
        rss=session_rss(sid);STATE['peak_rss_bytes']=max(STATE['peak_rss_bytes'],rss)
        if rss>LIMITS['rss_bytes']: fail('RSS_LIMIT')
    mem=[line for line in pathlib.Path('/proc/meminfo').read_text().splitlines() if line.startswith('MemAvailable:')]
    if len(mem)!=1 or int(mem[0].split()[1])*1024<LIMITS['memavailable_bytes']: fail('MEM_LIMIT')
    used=size(WORKSPACE)+size(WORK)+size(SYSROOT)
    if SYSROOT is not None: used+=size(SYSROOT.parent.parent/'downloads')+size(SYSROOT.parent.parent/'tmp')
    STATE['peak_workspace_bytes']=max(STATE['peak_workspace_bytes'],used)
    if used>LIMITS['workspace_bytes']: fail('DISK_LIMIT')
    for path in (WORKSPACE,WORK,SYSROOT):
        if path is not None and path.exists() and shutil.disk_usage(path).free<LIMITS['free_bytes']: fail('DISK_LIMIT')
    if SYSROOT is not None and size(SYSROOT)>LIMITS['toolchain_bytes']: fail('DISK_LIMIT')

def validate_activation(env,event,head,parents,changed,marker,seal,source_sha):
    required={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':BRANCH,'GITHUB_RUN_ATTEMPT':'1'}
    if any(env.get(k)!=v for k,v in required.items()): fail('ADMISSION')
    before=event.get('before','')
    if not re.fullmatch('[0-9a-f]{40}',before) or before=='0'*40 or not re.fullmatch('[0-9a-f]{40}',head): fail('ADMISSION')
    if event.get('after')!=head or event.get('ref')!=BRANCH or any(event.get(k) is not False for k in ('created','deleted','forced')): fail('ADMISSION')
    if parents!=[head,before] or changed!='A\t'+MARKER+'\n': fail('ADMISSION')
    expected={'schema':'m2-channel-native-controlled-diagnostic-activation-v4','plan_id':PLAN,'main':MAIN,'research_base':BASE,'prepared_head':before,'seal_sha256':seal,'source_manifest_sha256':source_sha,'attempt':1,'fixed_disposable_qualification_only':True}
    if marker!=expected: fail('ADMISSION')
    return before

def validate_tree_preservation(main_rows,prepared_rows):
    def mapping(rows):
        out={}
        for row in rows:
            left,path=row.split('\t',1)
            if not safe_path(path) or path in out: fail('ADMISSION')
            out[path]=left
        return out
    old,new=mapping(main_rows),mapping(prepared_rows)
    if any(new.get(path)!=entry for path,entry in old.items()): fail('ADMISSION')
    added=set(new)-set(old)
    if not added or any(not (p.startswith(PACKET+'/') or p=='.github/workflows/'+PLAN+'.yml') for p in added) or MARKER in new: fail('ADMISSION')
    return added

def verify_workflows(bundle,eligible):
    rows=read_json(ROOT/'WORKFLOW-PRESERVATION.json')['files']
    if len(rows)!=15: fail('ADMISSION')
    expected={row['path']:'100644 blob '+row['git_blob_sha1']+'\t'+row['path'] for row in rows}
    for row in rows:
        if row.get('mode')!='100644': fail('ADMISSION')
        verify_record(bundle,row)
    own='.github/workflows/'+PLAN+'.yml'
    inherited=[x for x in eligible if not x.endswith('\t'+own)]
    if inherited!=sorted(expected.values(),key=lambda x:x.split('\t')[1]) or len(eligible)!=16: fail('ADMISSION')
    if (bundle/own).read_bytes()!=(ROOT/'workflow.yml').read_bytes(): fail('BINDING')

def admit():
    seal_sha=digest_file(ROOT/'seal.json')
    if os.environ.get('EXPECTED_SEAL_SHA256')!=seal_sha: fail('BINDING')
    seal=read_json(ROOT/'seal.json')
    if seal.get('plan_id')!=PLAN or seal.get('source_ready') is not True: fail('BINDING')
    names=[]
    for row in seal['files']: verify_record(ROOT,row);names.append(row['path'])
    if len(names)!=len(set(names)): fail('BINDING')
    bundle=WORKSPACE/'bundle'
    if ROOT!=bundle/PACKET: fail('ADMISSION')
    event=read_json(pathlib.Path(os.environ['GITHUB_EVENT_PATH']));head=os.environ.get('GITHUB_SHA','')
    before=event.get('before','')
    if not re.fullmatch('[0-9a-f]{40}',head) or not re.fullmatch('[0-9a-f]{40}',before): fail('ADMISSION')
    if private_git('rev-parse','HEAD',cwd=bundle).decode().strip()!=head: fail('BINDING')
    parents=private_git('rev-list','--parents','-n','1','HEAD',cwd=bundle).decode().split()
    changed=private_git('diff','--name-status',before,head,cwd=bundle).decode()
    source_sha=digest_file(ROOT/'source-manifest.json')
    prepared=validate_activation(os.environ,event,head,parents,changed,read_json(bundle/MARKER),seal_sha,source_sha)
    if private_git('rev-list','--parents','-n','1',prepared,cwd=bundle).decode().split()!=[prepared,MAIN]: fail('ADMISSION')
    old=private_git('ls-tree','-r',MAIN,cwd=bundle).decode().splitlines()
    new=private_git('ls-tree','-r',prepared,cwd=bundle).decode().splitlines()
    validate_tree_preservation(old,new)
    verify_workflows(bundle,private_git('ls-tree','-r',head,'.github/workflows',cwd=bundle).decode().splitlines())
    actual={p.relative_to(ROOT).as_posix() for p in ROOT.rglob('*') if p.is_file()}
    allowed=set(names)|{'seal.json','workflow.yml','ACTIVATE.template.json','ACTIVATE.json'}
    if actual!=allowed or private_git('status','--porcelain',cwd=bundle).strip(): fail('BINDING')
    STATE.update(activation_head=head,prepared_head=prepared,seal_sha256=seal_sha,source_manifest_sha256=source_sha)

def overlay_members(data,expected):
    if len(data)>1024**2: fail('BINDING')
    found={};total=0
    with tarfile.open(fileobj=io.BytesIO(data),mode='r:gz') as tf:
        for member in tf:
            name=member.name
            if not safe_path(name) or not member.isfile() or name not in expected or name in found: fail('SOURCE_CLOSURE')
            row=expected[name]
            total+=member.size
            if member.size!=row['bytes'] or total>4*1024**2: fail('SOURCE_CLOSURE')
            b=tf.extractfile(member).read(member.size+1)
            if len(b)!=row['bytes'] or sha(b)!=row['sha256']: fail('BINDING')
            found[name]=b
    if set(found)!=set(expected): fail('SOURCE_CLOSURE')
    return found

def choose_role(role):
    global SOURCE,TARGET,BOUND,LOCKED_REGISTRY
    if role not in ('sampler',): fail('INTERNAL')
    SOURCE=WORK/role;TARGET=WORK/('target-channel-native-controlled-v3')
    if not TARGET.exists():TARGET.mkdir(mode=0o700)
    ENV['CARGO_TARGET_DIR']=str(TARGET)
    if not BOUND: fail('SOURCE_CLOSURE')
    lock=tomllib.loads((SOURCE/'Cargo.lock').read_text())
    LOCKED_REGISTRY={r['name']+'-'+r['version'] for r in lock['package'] if r.get('source')=='registry+https://github.com/rust-lang/crates.io-index' and re.fullmatch('[0-9a-f]{64}',r.get('checksum',''))}
    if any(r.get('source') and r['source']!='registry+https://github.com/rust-lang/crates.io-index' for r in lock['package']): fail('SOURCE_CLOSURE')

def local_dep_path(token):
    """Classify every input, never silently exempt an unknown outside path."""
    if not isinstance(token,str) or any(ord(c)<32 for c in token):fail('SOURCE_CLOSURE')
    p=pathlib.Path(token)
    if not p.is_absolute():p=SOURCE/p
    p=p.resolve()
    if p.is_relative_to(SOURCE):
        q=p.relative_to(SOURCE).as_posix()
        if q not in BOUND:fail('SOURCE_CLOSURE')
        return q
    registry=WORK/'cargo'/'registry'/'src'
    if p.is_relative_to(registry):
        rel=p.relative_to(registry).parts
        if len(rel)>=3 and re.fullmatch(r'index\.crates\.io-[0-9a-f]+',rel[0]) and rel[1] in LOCKED_REGISTRY:return None
        fail('SOURCE_CLOSURE')
    if SYSROOT is not None and p.is_relative_to(SYSROOT):return None
    if TARGET is not None and p.is_relative_to(TARGET) and any(p.is_relative_to(d) for d in DERIVED):return None
    fail('SOURCE_CLOSURE')

def package_roots(paths):
    roots=set()
    for token in paths:
        p=pathlib.Path(token)
        if not p.is_absolute():p=SOURCE/p
        p=p.resolve()
        if p.is_relative_to(SOURCE):
            root=p.parent
            while root.is_relative_to(SOURCE):
                candidate=root/'Cargo.toml'
                q=candidate.relative_to(SOURCE).as_posix()
                if q in BOUND and candidate.is_file():
                    if 'package' in tomllib.loads(candidate.read_text()):roots.add(root)
                    break
                if root==SOURCE:break
                root=root.parent
        else:
            registry=WORK/'cargo/registry/src'
            if p.is_relative_to(registry):
                rel=p.relative_to(registry).parts
                if len(rel)>=3:roots.add(registry/rel[0]/rel[1])
    return roots

def validate_env_dep(text,roots):
    field=text[len('# env-dep:'):]
    key,sep,value=field.partition('=');value=value if sep else None
    if not re.fullmatch('[A-Za-z_][A-Za-z0-9_]*',key):fail('SOURCE_CLOSURE')
    if key in ENV:
        if value!=ENV[key]:fail('SOURCE_CLOSURE')
        return
    if key=='OUT_DIR':
        if value is None or pathlib.Path(value).resolve() not in DERIVED or not pathlib.Path(value).resolve().is_relative_to(TARGET):fail('SOURCE_CLOSURE')
        return
    if key in BUILD_ENV and value in BUILD_ENV[key]:return
    if key in {'ALIFE_CONSUMER_GIT_HEAD','ALIFE_CONSUMER_GIT_TREE','ALIFE_GROUPED_M2_DRIVER_SOURCE_COMMIT'}:
        if value is not None:fail('SOURCE_CLOSURE')
        return
    if key=='CARGO_MANIFEST_DIR':
        if value is None or pathlib.Path(value).resolve() not in roots:fail('SOURCE_CLOSURE')
        return
    fields={'CARGO_PKG_NAME':'name','CARGO_PKG_VERSION':'version','CARGO_PKG_AUTHORS':'authors','CARGO_PKG_DESCRIPTION':'description','CARGO_PKG_HOMEPAGE':'homepage','CARGO_PKG_REPOSITORY':'repository','CARGO_PKG_LICENSE':'license','CARGO_PKG_LICENSE_FILE':'license-file','CARGO_PKG_README':'readme','CARGO_PKG_RUST_VERSION':'rust-version'}
    if key not in fields and key not in {'CARGO_PKG_VERSION_MAJOR','CARGO_PKG_VERSION_MINOR','CARGO_PKG_VERSION_PATCH','CARGO_PKG_VERSION_PRE'}:fail('SOURCE_CLOSURE')
    allowed=set();workspace=tomllib.loads((SOURCE/'Cargo.toml').read_text()).get('workspace',{}).get('package',{})
    for root in roots:
        manifest=tomllib.loads((root/'Cargo.toml').read_text()).get('package',{})
        fieldname=fields.get(key,'version');item=manifest.get(fieldname,'')
        if isinstance(item,dict) and item.get('workspace') is True:item=workspace.get(fieldname,'')
        if key.startswith('CARGO_PKG_VERSION_'):
            if not isinstance(item,str) or not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:-[A-Za-z0-9.\-]+)?(?:\+[A-Za-z0-9.\-]+)?',item):fail('SOURCE_CLOSURE')
            parts=item.split('+',1)[0].split('-',1);numbers=parts[0].split('.')
            item={'CARGO_PKG_VERSION_MAJOR':numbers[0],'CARGO_PKG_VERSION_MINOR':numbers[1],'CARGO_PKG_VERSION_PATCH':numbers[2],'CARGO_PKG_VERSION_PRE':parts[1] if len(parts)>1 else ''}[key]
        if isinstance(item,list):item=':'.join(item)
        if not isinstance(item,str):fail('SOURCE_CLOSURE')
        allowed.add(item)
    if value not in allowed:fail('SOURCE_CLOSURE')

def parse_depinfo(text):
    """Linux rustc make dep-info: target rules, phony rules and env records."""
    paths=[];env=[]
    for line in text.replace('\\\n','').splitlines():
        if not line.strip():continue
        if line.startswith('# env-dep:'):env.append(line);continue
        if line.startswith('#'):fail('SOURCE_CLOSURE')
        if '\\ ' in line or line.count(':')!=1:fail('SOURCE_CLOSURE')
        lhs,rhs=line.split(':',1)
        targets=lhs.split()
        if not targets:fail('SOURCE_CLOSURE')
        for target in targets:
            q=pathlib.Path(target)
            if not q.is_absolute():q=SOURCE/q
            q=q.resolve()
            if not q.is_relative_to(TARGET):local_dep_path(target)
        for token in rhs.split():local_dep_path(token);paths.append(token)
    roots=package_roots(paths)
    for record in env:validate_env_dep(record,roots)
    return paths

def verify_depinfo(artifacts):
    roots=set();observed=set();total=0
    for obj in artifacts:
        q=local_dep_path(obj['target']['src_path'])
        if q is not None:
            if q not in BOUND:fail('SOURCE_CLOSURE')
            roots.add(q)
        for p in obj.get('filenames',[]):
            if not pathlib.Path(p).resolve().is_relative_to(TARGET):fail('CHECK_PROTOCOL')
    for dep in TARGET.rglob('*.d'):
        length=dep.stat().st_size;total+=length
        if length>8*1024**2 or total>128*1024**2:fail('SOURCE_CLOSURE')
        for token in parse_depinfo(dep.read_text()):
            q=local_dep_path(token)
            if q is not None:observed.add(q)
    if not roots or not roots<=observed:fail('SOURCE_CLOSURE')
    for q in observed:verify_record(SOURCE,BOUND[q])

def build(role='sampler',tests=True):
    choose_role(role);stage='sampler-test-build';STATE['stage']=stage
    artifacts=[];finished=[]
    def consume(line):
        try:obj=json.loads(line)
        except (ValueError,UnicodeError):fail('CHECK_PROTOCOL')
        if not isinstance(obj,dict):fail('CHECK_PROTOCOL')
        reason=obj.get('reason')
        if reason=='compiler-message':
            for row in diagnostic_rows(obj,stage,SOURCE,BOUND):
                if row not in STATE['diagnostics'] and len(STATE['diagnostics'])<24:STATE['diagnostics'].append(row)
        elif reason=='compiler-artifact':
            if len(artifacts)>=4096:fail('CHECK_PROTOCOL')
            artifacts.append({k:obj.get(k) for k in ('target','features','filenames','package_id','executable','profile')})
        elif reason=='build-finished':finished.append(obj.get('success'))
        elif reason=='build-script-executed':
            out=obj.get('out_dir')
            if not isinstance(out,str):fail('SOURCE_CLOSURE')
            path=pathlib.Path(out).resolve()
            if not path.is_relative_to(TARGET/'debug/build') or path.name!='out':fail('SOURCE_CLOSURE')
            DERIVED.add(path)
            env_pairs=obj.get('env',[])
            if not isinstance(env_pairs,list):fail('SOURCE_CLOSURE')
            for pair in env_pairs:
                if not isinstance(pair,list) or len(pair)!=2 or not all(isinstance(v,str) and len(v)<=4096 for v in pair) or not re.fullmatch('[A-Za-z_][A-Za-z0-9_]*',pair[0]):fail('SOURCE_CLOSURE')
                BUILD_ENV.setdefault(pair[0],set()).add(pair[1])
        else:fail('CHECK_PROTOCOL')
    cmd=[RUSTUP,'run',TOOLCHAIN,'cargo','test','--locked','-j1','-p','alife_training','--features','cpu-backend','--lib','--message-format=json']
    cmd+=['--no-run']
    _,status=run(cmd,'CHECK',seconds=LIMITS['compile_seconds'],consumer=consume,allow_nonzero=True)
    if status:fail('CHECK')
    if finished!=[True]:fail('CHECK_PROTOCOL')
    selected=[x for x in artifacts if x['target'].get('name')=='alife_training' and x.get('profile',{}).get('test') is True]
    if len(selected)!=1 or selected[0]['target'].get('kind')!=['lib'] or sorted(selected[0].get('features',[]))!=['cpu-backend','default','receipt-lineage']:fail('FEATURES')
    if any(x.get('profile',{}).get('test') is True and x not in selected for x in artifacts):fail('FEATURES')
    verify_depinfo(artifacts)
    for row in BOUND.values():verify_record(SOURCE,row)
    target='alife_training'
    matches=[x for x in selected if x['target'].get('name')==target and x.get('executable') and (not tests or x.get('profile',{}).get('test') is True)]
    if len(matches)!=1:fail('CHECK_PROTOCOL')
    path=pathlib.Path(matches[0]['executable'])
    if not path.is_file() or path.is_symlink() or not path.resolve().is_relative_to(TARGET):fail('CHECK_PROTOCOL')
    STATE['completed'].append(stage)
    STATE.setdefault('executable_sha256',{})[stage]=digest_file(path)
    return path,digest_file(path)


def test_listing(raw,expected):
    if len(raw)>65536:fail('TEST_PROTOCOL')
    try:text=raw.decode('utf-8','strict')
    except UnicodeError:fail('TEST_PROTOCOL')
    lines=text.splitlines();required=[n+': test' for n in sorted(expected)]+['',str(len(expected))+' tests, 0 benchmarks']
    if lines!=required:fail('TEST_PROTOCOL')


def capsule_source(data,manifest):
    found=overlay_members(data,{'source/'+r['path']:{**r,'path':'source/'+r['path']} for r in manifest['host_capsule_files']})
    return {name[7:]:b for name,b in found.items()}

def exclude_old_test_registration(text):
    # Mask comments and quoted literals without changing offsets. Never rewrite
    # a production string just because it contains a line spelled #[test].
    mask=list(text);i=0;n=len(text)
    def blank(a,b):
        for k in range(a,b):
            if mask[k] not in '\r\n':mask[k]=' '
    while i<n:
        if text.startswith('//',i):
            j=text.find('\n',i);j=n if j<0 else j;blank(i,j);i=j;continue
        if text.startswith('/*',i):
            j=i+2;depth=1
            while j<n and depth:
                if text.startswith('/*',j):depth+=1;j+=2
                elif text.startswith('*/',j):depth-=1;j+=2
                else:j+=1
            if depth:fail('SOURCE_CLOSURE')
            blank(i,j);i=j;continue
        char=re.match(r"'(?:\\(?:u\{[0-9A-Fa-f_]+\}|x[0-9A-Fa-f]{2}|.)|[^'\\\n])'",text[i:])
        if char:
            j=i+len(char.group(0));blank(i,j);i=j;continue
        raw=re.match(r'r(#+)?"',text[i:])
        if raw:
            close='"'+(raw.group(1)or'');j=text.find(close,i+len(raw.group(0)))
            if j<0:fail('SOURCE_CLOSURE')
            j+=len(close);blank(i,j);i=j;continue
        if text[i]=='"':
            j=i+1
            while j<n:
                if text[j]=='\\':j+=2
                elif text[j]=='"':j+=1;break
                else:j+=1
            blank(i,j);i=j;continue
        i+=1
    matches=list(re.finditer(r'#\s*\[\s*test\s*\]', ''.join(mask)))
    for match in reversed(matches):
        text=text[:match.start()]+'#[cfg(any())]'+text[match.end():]
    return text,len(matches)

def project_test_registration():
    # This isolated test assembly excludes old test BODIES, not production guards.
    # Preserve all production tokens and helper functions; require exact new inventory.
    global BOUND
    owned={r['source'] for r in read_json(ROOT/'FIXED-INVENTORY.json')}
    changed=[]
    for name,row in list(BOUND.items()):
        if not name.startswith('crates/alife_training/') or not name.endswith('.rs') or name in owned:continue
        path=SOURCE/name;old=path.read_bytes();text=old.decode('utf-8','strict')
        updated,count=exclude_old_test_registration(text)
        if count:
            data=updated.encode();path.write_bytes(data)
            BOUND[name]={'path':name,'bytes':len(data),'sha256':sha(data)}
            changed.append({'path':name,'preimage_sha256':sha(old),'sha256':sha(data),'excluded_tests':count})
    if not changed:fail('SOURCE_CLOSURE')
    STATE['excluded_old_test_count']=sum(r['excluded_tests'] for r in changed)
    STATE['test_projection_sha256']=sha(json.dumps(changed,sort_keys=True,separators=(',',':')).encode())

def restore_sources():
    global SOURCE,BOUND
    STATE['stage']='source-closure'
    base=WORKSPACE/'source'
    if private_git('rev-parse','HEAD',cwd=base).decode().strip()!=MAIN or private_git('status','--porcelain',cwd=base).strip():fail('BINDING')
    m=read_json(ROOT/'source-manifest.json')
    if m.get('base_commit')!=MAIN or m.get('unresolved_includes')!=[]:fail('BINDING')
    recovery=WORK/'logical-recovery';recovery.mkdir(mode=0o700)
    for row in m['reachable_main_files']:
        verify_record(base,row);dest=recovery/row['path'];dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(base/row['path'],dest)
    steps=read_json(REUSE/'RECONSTRUCTION-PATCHES.json')['steps']
    if [x['name']for x in steps]!=['main-to-fbf-library','fbf-nominal-supplement']:fail('BINDING')
    for index,step in enumerate(steps):
        chunks=[]
        for row in step['parts']:verify_record(REUSE,row);chunks.append((REUSE/row['path']).read_bytes())
        data=b''.join(chunks)
        if sha(data)!=step['sha256']or len(data)!=step['bytes']or b'GIT binary patch'in data or b'\0'in data:fail('BINDING')
        patch=WORK/('reconstruction-'+str(index)+'.patch');patch.write_bytes(data)
        run(['git','apply','--check',str(patch)],'PATCH',cwd=recovery,seconds=120)
        run(['git','apply',str(patch)],'PATCH',cwd=recovery,seconds=120)
    residual=overlay_members((REUSE/'nominal-frozen-input-residual.tar.gz').read_bytes(),{r['path']:r for r in m['residual_files']})
    for name,data in residual.items():
        dest=recovery/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
    SOURCE=WORK/'sampler';SOURCE.mkdir(mode=0o700)
    for row in m['base_files']:
        verify_record(recovery,row);dest=SOURCE/row['path'];dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(recovery/row['path'],dest)
    for archive,rows in [('qualified-original-overlay.tar.gz',m['qualified_overlay_files'])]:
        members=overlay_members((REUSE/archive).read_bytes(),{r['path']:r for r in rows})
        for name,data in members.items():
            dest=SOURCE/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
    for name,data in capsule_source((ROOT/'host-source-capsule.tar.gz').read_bytes(),m).items():
        dest=SOURCE/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
    BOUND={r['path']:r for r in m['sampler_files']}
    actual={p.relative_to(SOURCE).as_posix()for p in SOURCE.rglob('*')if p.is_file()}
    if actual!=set(BOUND):fail('SOURCE_CLOSURE')
    for row in BOUND.values():verify_record(SOURCE,row)
    project_test_registration()

INTERVAL_KEYS={'label','ordinal','tick_before','tick_after','generation_before','generation_after','play_selected','phase','posture_phase','stimulation','investigation','perceived_novelty','awake_dose','sleep_dose','energy_before','energy_after','boredom_before','boredom_after','source_executions','biology_intervals','optimizer_updates'}
ENDPOINT_KEYS={'intervals','prefix_intervals','treatment_intervals','source_executions','biology_intervals','optimizer_updates','cold_equal','play_energy','quiet_energy','play_awake_dose','quiet_awake_dose','play_stimulation','quiet_stimulation'}
PHASE_NAMES=['bank_comparison', 'body_checks', 'body_comparison', 'checkpoint_encode', 'checkpoint_ready', 'cold_comparison', 'cold_restore_started', 'cold_treatment_started', 'complete', 'construction_founder', 'construction_host', 'construction_ready', 'construction_recipe', 'construction_started', 'construction_toy', 'construction_world', 'controlled_ready', 'decision_preparation', 'dispatch_rejected', 'dispatch_returned', 'dispatch_started', 'interval_prepare', 'interval_verified', 'novelty_context_ready', 'outcome_close_rejected', 'outcome_close_returned', 'physical_outcome_available', 'play_treatment_started', 'quiet_treatment_started', 'restore_binding', 'restore_comparison', 'restore_decode', 'restore_ready', 'restore_resume', 'shared_prefix_complete', 'shared_prefix_started', 'source_bound', 'staging_rejected', 'staging_returned', 'staging_started']

def parse_phase_rows(text,complete):
    snapshots=[];previous=None
    STATE['phase_snapshots']=snapshots
    for match in re.finditer(r'(?m)^CONTROL_PHASE_V1 (.+)\n',text):
        if len(snapshots)>=128:fail('TEST_PROTOCOL')
        try:row=json.loads(match[1])
        except (ValueError,TypeError):fail('TEST_PROTOCOL')
        if not isinstance(row,dict)or set(row)!={'phase','ordinal','attempted','returned','verified','restored'}or row['phase']not in PHASE_NAMES:fail('TEST_PROTOCOL')
        if any(type(row[k])is not int for k in ('ordinal','attempted','returned','verified','restored')):fail('TEST_PROTOCOL')
        if not(0<=row['ordinal']<=5 and 0<=row['verified']<=row['returned']<=row['attempted']<=6 and 0<=row['restored']<=2):fail('TEST_PROTOCOL')
        if previous and any(row[k]<previous[k]for k in ('ordinal','attempted','returned','verified','restored')):fail('TEST_PROTOCOL')
        snapshots.append(row);previous=row
    if complete and(not snapshots or snapshots[-1]!={'phase':'complete','ordinal':5,'attempted':6,'returned':6,'verified':6,'restored':2}):fail('TEST_PROTOCOL')
    return re.sub(r'(?m)^CONTROL_PHASE_V1 .+\n','',text)

def parse_control_rows(raw,complete):
    if len(raw)>65536:fail('TEST_PROTOCOL')
    try:text=raw.decode('utf-8','strict')
    except UnicodeError:fail('TEST_PROTOCOL')
    text=parse_phase_rows(text,complete)
    observations=[];endpoint=None
    for match in re.finditer(r'(?m)^CONTROL_(PHYSICS|ENDPOINT)_V1 (.+)\n',text):
        try:row=json.loads(match[2],parse_constant=lambda _:fail('TEST_PROTOCOL'))
        except (ValueError,TypeError):fail('TEST_PROTOCOL')
        keys=INTERVAL_KEYS if match[1]=='PHYSICS' else ENDPOINT_KEYS
        if not isinstance(row,dict)or set(row)!=keys:fail('TEST_PROTOCOL')
        if match[1]=='PHYSICS':
            if type(row['play_selected'])is not bool or row['phase']not in ('Attempt','Preparation','Completed')or row['posture_phase']not in ('Attempt','Preparation','Completed'):fail('TEST_PROTOCOL')
            integer_keys={'ordinal','tick_before','tick_after','generation_before','generation_after','source_executions','biology_intervals','optimizer_updates'}
        else:
            if type(row['cold_equal'])is not bool:fail('TEST_PROTOCOL')
            integer_keys={'intervals','prefix_intervals','treatment_intervals','source_executions','biology_intervals','optimizer_updates'}
        if any(type(row[k])is not int for k in integer_keys):fail('TEST_PROTOCOL')
        for key,value in row.items():
            if isinstance(value,(int,float))and not isinstance(value,bool):
                if not math.isfinite(value)or abs(value)>1000000:fail('TEST_PROTOCOL')
            elif isinstance(value,bool):
                if key not in ('play_selected','cold_equal'):fail('TEST_PROTOCOL')
            elif isinstance(value,str):
                if key=='label'and value in ('prefix','play','cold_play','no_manip'):continue
                if key in ('phase','posture_phase')and value in ('Attempt','Preparation','Completed'):continue
                fail('TEST_PROTOCOL')
            else:fail('TEST_PROTOCOL')
        if match[1]=='PHYSICS':
            if len(observations)>=6 or endpoint is not None:fail('TEST_PROTOCOL')
            i=len(observations);label=['prefix','prefix','prefix','play','cold_play','no_manip'][i]
            if row['ordinal']!=i or row['label']!=label or row['play_selected']!=(i!=5)or row['tick_after']!=row['tick_before']+1 or row['generation_after']!=row['generation_before']+1 or row['source_executions']!=1 or row['biology_intervals']!=1 or row['optimizer_updates']!=0:fail('TEST_PROTOCOL')
            observations.append(row)
        else:
            if endpoint is not None or len(observations)!=6:fail('TEST_PROTOCOL')
            if any(row[k]!=v for k,v in {'intervals':6,'prefix_intervals':3,'treatment_intervals':3,'source_executions':6,'biology_intervals':6,'optimizer_updates':0,'cold_equal':True}.items()):fail('TEST_PROTOCOL')
            bindings={'play_energy':(3,'energy_after'),'quiet_energy':(5,'energy_after'),'play_awake_dose':(3,'awake_dose'),'quiet_awake_dose':(5,'awake_dose'),'play_stimulation':(3,'stimulation'),'quiet_stimulation':(5,'stimulation')}
            if any(row[k]!=observations[i][field]for k,(i,field)in bindings.items()):fail('TEST_PROTOCOL')
            endpoint=row
    if complete and(len(observations)!=6 or endpoint is None):fail('TEST_PROTOCOL')
    clean=re.sub(r'(?m)^CONTROL_(?:PHYSICS|ENDPOINT)_V1 .+\n','',text)
    return observations,endpoint,clean

def test_report(raw,name):
    observations,endpoint,text=parse_control_rows(raw,True)
    pattern=r'\nrunning 1 test\ntest '+re.escape(name)+r' \.\.\.\s+ok\n\ntest result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9]+\.[0-9]+s\n\n'
    if re.fullmatch(pattern,text)is None:fail('TEST_PROTOCOL')
    STATE['controlled_observations']=observations;STATE['controlled_endpoint']=endpoint
    return {'name':name,'passed':1,'failed':0,'ignored':0}

def run_tests(executable,digest):
    STATE['stage']='list-exact'
    raw,status=run([str(executable),'--list'],'TEST',seconds=30,capture=True,allow_nonzero=True,pipe_limit=65536)
    if status:fail('TEST')
    test_listing(raw,ALL_TEST_NAMES)
    for name in TEST_NAMES:
        if digest_file(executable)!=digest:fail('BINDING')
        STATE['stage']='test-fixed';STATE['test_processes_attempted']+=1
        raw,status=run([str(executable),'--exact',name,'--test-threads=1','--nocapture'],'TEST',seconds=300,capture=True,allow_nonzero=True,pipe_limit=65536)
        if status:
            observations,endpoint,_=parse_control_rows(raw,False)
            STATE['controlled_observations']=observations;STATE['controlled_endpoint']=endpoint
            fail('TEST')
        STATE['test_results'].append(test_report(raw,name))
    for row in BOUND.values():verify_record(SOURCE,row)

def summary(code):
    allowed={'admission','source-closure','toolchain','sampler-test-build','list-exact','test-fixed','complete'}
    if STATE['stage']not in allowed:return '{"code":"INTERNAL"}'
    diagnostics=[]
    for d in STATE['diagnostics']:
        if set(d)!={'code','path','line','stage'} or re.fullmatch(r'E\d{4}',d['code'])is None or d['path']not in BOUND or type(d['line'])is not int or d['stage']not in allowed:return '{"code":"INTERNAL"}'
        diagnostics.append(d)
    result={'schema':'channel-native-controlled-physics-summary-v4','plan_id':PLAN,'code':code if code in CODES else'INTERNAL','stage':STATE['stage'],'diagnostics':diagnostics,'test_processes_attempted':STATE['test_processes_attempted'],'results':STATE['test_results'],'planned_tests':1,'planned_world_step_calls_max':6,'source_forward_calls_max':6,'trained_checkpoint_file_reads':0,'controlled_physics_qualified':code=='OK','autonomous_sampler_qualified_by_this_run':False,'controlled_observations':STATE.get('controlled_observations',[]),'controlled_endpoint':STATE.get('controlled_endpoint'),'phase_snapshots':STATE.get('phase_snapshots',[]),'excluded_old_test_count':STATE.get('excluded_old_test_count',0),'test_projection_sha256':STATE.get('test_projection_sha256',''),'peak_rss_bytes':STATE['peak_rss_bytes'],'peak_workspace_bytes':STATE['peak_workspace_bytes'],'private_output_counted_bytes':STATE['output_bytes']}
    raw=json.dumps(result,separators=(',',':'),sort_keys=True,allow_nan=False)
    return raw if len(raw.encode())<=32768 else'{"code":"OUTPUT_LIMIT"}'


def main():
    global WORKSPACE,WORK,SOURCE,ENV,REUSE
    original=dict(os.environ)
    if original.get('GITHUB_ACTIONS')!='true'or original.get('GITHUB_RUN_ATTEMPT')!='1':fail('ADMISSION')
    WORKSPACE=pathlib.Path(original['GITHUB_WORKSPACE']).resolve(strict=True);SOURCE=WORKSPACE/'source';WORK=pathlib.Path(original['RUNNER_TEMP']).resolve(strict=True)/PLAN
    if WORK.exists()or WORK.is_symlink()or WORK.is_relative_to(WORKSPACE)or SOURCE.is_symlink()or not SOURCE.is_dir():fail('BINDING')
    if shutil.disk_usage(WORKSPACE).free<LIMITS['initial_free_bytes']:fail('DISK_LIMIT')
    WORK.mkdir(mode=0o700)
    for n in('home','cargo','temp'):(WORK/n).mkdir(mode=0o700)
    ENV={k:original[k]for k in('PATH','HOME')if k in original};ENV.update({'CARGO_HOME':str(WORK/'cargo'),'CARGO_INCREMENTAL':'0','CARGO_BUILD_JOBS':'1','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_PROFILE_TEST_STRIP':'symbols','CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_DEV_STRIP':'symbols','CARGO_PROFILE_TEST_INCREMENTAL':'false','CARGO_TERM_COLOR':'never','CARGO_NET_RETRY':'0','RUST_BACKTRACE':'0','RUST_TEST_THREADS':'1','LANG':'C.UTF-8','LC_ALL':'C.UTF-8','TMPDIR':str(WORK/'temp'),'TMP':str(WORK/'temp'),'TEMP':str(WORK/'temp'),'RUSTUP_DIST_SERVER':'https://static.rust-lang.org','RUSTUP_UPDATE_ROOT':'https://static.rust-lang.org/rustup','RUSTUP_NO_UPDATE_CHECK':'1','RUSTUP_PERMIT_COPY_RENAME':'1','GIT_CONFIG_NOSYSTEM':'1','GIT_TERMINAL_PROMPT':'0'})
    admit()
    reuse_checkout=WORKSPACE/'reuse';REUSE=reuse_checkout/REUSE_PACKET
    if private_git('rev-parse','HEAD',cwd=reuse_checkout).decode().strip()!=REUSE_COMMIT or private_git('status','--porcelain',cwd=reuse_checkout).strip():fail('BINDING')
    for row in read_json(ROOT/'REUSE-INPUTS.json')['files']:verify_record(REUSE,row)
    restore_sources();setup_toolchain(original);executable,digest=build();run_tests(executable,digest);STATE['stage']='complete';check_limits()

if __name__=='__main__':
    code='OK'
    try:main()
    except Stop as e:code=str(e)if str(e)in CODES else'INTERNAL'
    except BaseException:code='INTERNAL'
    try:out=summary(code)
    except BaseException:out='{"code":"INTERNAL"}'
    print(out,flush=True);sys.exit(0 if code=='OK' else 1)
