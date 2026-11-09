#!/usr/bin/env python3
"""Inert hosted exact-fixture runner. Import performs no native work.

After sealed push admission, compile only selected test targets without running
then list exact names and run the frozen fixed fixtures once each. No autonomous
study, example, GPU device or existing-life access. All native output is private.
"""
import hashlib
import json
import os
import pathlib
import re
import resource
import selectors
import shutil
import signal
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parent
PLAN = 'm2-grip-fixed-fixtures-v1'
BRANCH = 'refs/heads/qualification/m2-grip-fixed-fixtures-v1'
PACKET = 'docs/experiments/m2-grip-fixed-fixtures-v1'
MARKER = PACKET + '/ACTIVATE.json'
MAIN = 'c3aec16df33861f1316d127736f5f60eea327ede'
FBF = 'fbf2a517f93098b5d936878a3d26bbb8d0391c38'
TOOLCHAIN = '1.99.1'
RUST_COMMIT = '58cf8f9c35b355199727627d07ae34a4e2846d67'
RUST_DATE = '2026-10-13'
HOST = 'x86_64-unknown-linux-gnu'
RUST_VERSION = f'rustc {TOOLCHAIN} ({RUST_COMMIT[:9]} {RUST_DATE})'
CARGO_VERSION = 'cargo 1.99.1 (5f94df478 2026-08-27)'
ACTION_SHA = '0b52547c145ae18d8631c9ab711e4d917e9e489cea800368f338c25a56e0aee2'
STAGES = (
    ('core', 'alife_core', ('--all-targets',), ()),
    ('world', 'alife_world', ('--all-targets',), ()),
    ('gpu', 'alife_gpu_backend', ('--all-targets',), ('training-rollout',)),
    ('runtime', 'alife_runtime', ('--all-targets',), ('training-rollout',)),
    ('training', 'alife_training', ('--lib', '--test', 'grip_options_host'), ('cpu-backend',)),
    ('gameapp', 'alife_game_app', ('--all-targets',), ('cpu-training',)),
)
EXPECTED_FEATURES = {
    'alife_core': [], 'alife_world': [],
    'alife_gpu_backend': ['default', 'training-rollout'],
    'alife_runtime': ['default', 'training-rollout'],
    'alife_training': ['cpu-backend', 'default', 'receipt-lineage'],
    'alife_game_app': ['cpu-training', 'default', 'foundation-training', 'gpu-runtime'],
}
LIMITS = {'wall_seconds': 5400, 'rss_bytes': 6 * 1024**3,
          'workspace_bytes': 12 * 1024**3, 'free_bytes': 2 * 1024**3,
          'output_bytes': 32 * 1024**2, 'summary_bytes': 32768,
          'toolchain_bytes': 3 * 1024**3, 'poll_seconds': 0.25}
CODES = {'OK', 'ADMISSION', 'BINDING', 'PATCH', 'SOURCE_CLOSURE',
         'TOOLCHAIN', 'CHECK', 'CHECK_PROTOCOL', 'FEATURES', 'TIME_LIMIT',
         'RSS_LIMIT', 'DISK_LIMIT', 'OUTPUT_LIMIT', 'INTERNAL',
         'FIXTURE_INVENTORY', 'TEST_LIST', 'TEST_EXECUTION', 'TEST_PROTOCOL'}
START = time.monotonic()
STATE = {'stage': 'admission', 'completed': [], 'diagnostics': [],
         'peak_rss_bytes': 0, 'peak_workspace_bytes': 0, 'output_bytes': 0,
         'confirmed_passed_test_bodies': 0, 'launched_fixture_processes': 0,
         'listed_fixture_ids': [], 'attempted_fixture_ids': [], 'passed_fixture_ids': [],
         'compiled_target_ids': [], 'binary_identities': [],
         'source_audited_attempt_dose': {}, 'source_audited_passed_dose': {}}
WORKSPACE = WORK = SOURCE = SYSROOT = None
ENV = {}
BOUND = {}
RUSTUP = None

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

def check_limits(sid=None):
    if time.monotonic() - START >= LIMITS['wall_seconds']:
        fail('TIME_LIMIT')
    if sid is not None:
        rss = session_rss(sid)
        STATE['peak_rss_bytes'] = max(STATE['peak_rss_bytes'], rss)
        if rss > LIMITS['rss_bytes']:
            fail('RSS_LIMIT')
    used = size(WORKSPACE) + size(WORK) + size(SYSROOT)
    if SYSROOT is not None:
        home = SYSROOT.parent.parent
        used += size(home / 'downloads') + size(home / 'tmp')
    STATE['peak_workspace_bytes'] = max(STATE['peak_workspace_bytes'], used)
    if used > LIMITS['workspace_bytes']:
        fail('DISK_LIMIT')
    for p in (WORKSPACE, WORK, SYSROOT):
        if p is not None and p.exists() and shutil.disk_usage(p).free < LIMITS['free_bytes']:
            fail('DISK_LIMIT')
    if SYSROOT is not None and size(SYSROOT) > LIMITS['toolchain_bytes']:
        fail('DISK_LIMIT')

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

def run(args, code, *, cwd=None, seconds=5400, capture=False, consumer=None, allow_nonzero=False):
    """Pipes are bounded in memory; stderr is counted then discarded, never logged."""
    check_limits()
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

def validate_activation(env, event, head, parents, changed, marker, seal, source_sha):
    required = {'GITHUB_ACTIONS': 'true', 'GITHUB_EVENT_NAME': 'push',
                'GITHUB_REPOSITORY': 'BobertNeek/A-Life', 'GITHUB_REF': BRANCH,
                'GITHUB_RUN_ATTEMPT': '1'}
    if any(env.get(k) != v for k, v in required.items()):
        fail('ADMISSION')
    before = event.get('before', '')
    if not re.fullmatch('[0-9a-f]{40}', before) or before == '0' * 40:
        fail('ADMISSION')
    if not re.fullmatch('[0-9a-f]{40}', head) or event.get('after') != head:
        fail('ADMISSION')
    if event.get('ref') != BRANCH or any(event.get(k) is not False for k in ('created', 'deleted', 'forced')):
        fail('ADMISSION')
    if parents != [head, before] or changed != 'A\t' + MARKER + '\n':
        fail('ADMISSION')
    expected = {'schema': 'm2-grip-fixed-fixture-activation-v1', 'plan_id': PLAN,
                'main': MAIN, 'research_base': FBF, 'prepared_head': before,
                'seal_sha256': seal, 'source_manifest_sha256': source_sha,
                'attempt': 1, 'fixed_fixtures_only': True}
    if marker != expected:
        fail('ADMISSION')
    return before

def verify_workflow_preservation(bundle, eligible):
    """Retain exact pinned-main workflow blobs, modes and bytes; no removals."""
    contract = read_json(ROOT / 'WORKFLOW-PRESERVATION.json')
    rows = contract.get('files', [])
    if contract.get('main_commit') != MAIN or len(rows) != 15:
        fail('ADMISSION')
    expected = {}
    for row in rows:
        path = row.get('path', '')
        if (not path.startswith('.github/workflows/') or not safe_path(path)
                or row.get('mode') != '100644'
                or not re.fullmatch('[0-9a-f]{40}', row.get('git_blob_sha1', ''))
                or path in expected):
            fail('ADMISSION')
        expected[path] = '100644 blob ' + row['git_blob_sha1'] + '\t' + path
        verify_record(bundle, row)
    own = '.github/workflows/' + PLAN + '.yml'
    inherited = []
    own_count = 0
    for line in eligible:
        if re.fullmatch('100644 blob [0-9a-f]{40}\t' + re.escape(own), line):
            own_count += 1
        else:
            inherited.append(line)
    if own_count != 1 or inherited != sorted(expected.values(), key=lambda x: x.split('\t')[1]):
        fail('ADMISSION')

def admit():
    seal_bytes = (ROOT / 'seal.json').read_bytes()
    seal_sha = sha(seal_bytes)
    if os.environ.get('EXPECTED_SEAL_SHA256') != seal_sha:
        fail('BINDING')
    seal = read_json(ROOT / 'seal.json')
    if seal.get('plan_id') != PLAN or seal.get('source_ready') is not True:
        fail('BINDING')
    names = []
    for row in seal['files']:
        verify_record(ROOT, row); names.append(row['path'])
    if len(names) != len(set(names)):
        fail('BINDING')
    head = os.environ.get('GITHUB_SHA', '')
    if not re.fullmatch('[0-9a-f]{40}', head):
        fail('ADMISSION')
    event = read_json(pathlib.Path(os.environ['GITHUB_EVENT_PATH']))
    before = event.get('before', '')
    if not re.fullmatch('[0-9a-f]{40}', before):
        fail('ADMISSION')
    bundle = WORKSPACE / 'bundle'
    if ROOT != bundle / PACKET:
        fail('ADMISSION')
    if private_git('rev-parse', 'HEAD', cwd=bundle).decode().strip() != head:
        fail('ADMISSION')
    parents = private_git('rev-list', '--parents', '-n', '1', 'HEAD', cwd=bundle).decode().split()
    changed = private_git('diff', '--name-status', before, head, cwd=bundle).decode()
    eligible = private_git('ls-tree', '-r', head, '.github/workflows', cwd=bundle).decode().splitlines()
    verify_workflow_preservation(bundle, eligible)
    source_sha = digest_file(ROOT / 'source-manifest.json')
    marker_path = bundle / MARKER
    prepared = validate_activation(os.environ, event, head, parents, changed,
                                   read_json(marker_path), seal_sha, source_sha)
    # Even though workflow path filtering is one marker, reject all other edits
    # and any non-first addition in the only accepted one-parent commit.
    if private_git('status', '--porcelain', cwd=bundle).strip():
        fail('BINDING')
    STATE.update(activation_head=head, prepared_head=prepared, seal_sha256=seal_sha,
                 source_manifest_sha256=source_sha, activation_marker_sha256=digest_file(marker_path))

def restore_sources():
    global BOUND
    STATE['stage'] = 'recovery'
    if private_git('rev-parse', 'HEAD').decode().strip() != MAIN or private_git('status', '--porcelain').strip():
        fail('BINDING')
    recovery = read_json(ROOT / 'recovery.json')
    for step in recovery['steps']:
        chunks = []
        for row in step['parts']:
            verify_record(ROOT, row)
            chunks.append((ROOT / row['path']).read_bytes())
        data = b''.join(chunks)
        if sha(data) != step['sha256'] or len(data) != step['bytes']:
            fail('BINDING')
        data.decode('utf-8', 'strict')
        if b'\0' in data or b'GIT binary patch' in data:
            fail('PATCH')
        patch = WORK / 'apply.patch'
        patch.write_bytes(data)
        run(['git', 'apply', '--check', str(patch)], 'PATCH', seconds=120)
        run(['git', 'apply', str(patch)], 'PATCH', seconds=120)
        patch.unlink()
        if step['name'] == 'fbf-library':
            for row in read_json(ROOT / 'closure.json')['files']:
                verify_record(SOURCE, row)
    restore_test_fixture()
    manifest = read_json(ROOT / 'source-manifest.json')
    if manifest.get('unresolved_includes'):
        fail('SOURCE_CLOSURE')
    BOUND = {row['path']: row for row in manifest['files']}
    if len(BOUND) != len(manifest['files']):
        fail('BINDING')
    for row in BOUND.values():
        verify_record(SOURCE, row)
    STATE.update(main=MAIN, research_base=FBF, grip_patch_sha256=recovery['steps'][-3]['sha256'],
                 correction_patch_sha256=recovery['steps'][-2]['sha256'],
                 dependency_correction_patch_sha256=recovery['steps'][-1]['sha256'],
                 cargo_lock_sha256=BOUND['Cargo.lock']['sha256'], bound_files=len(BOUND))

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
    if not found or digest_file(ROOT / 'upstream-toolchain-action.yml') != ACTION_SHA:
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

def local_dep_path(token):
    p = pathlib.Path(token)
    if not p.is_absolute():
        if not token.startswith(('crates/', 'assets/', 'docs/', 'scripts/')):
            return None
        p = SOURCE / p
    p = p.resolve()
    if not p.is_relative_to(SOURCE):
        return None
    return p.relative_to(SOURCE).as_posix()

def verify_depinfo(artifacts):
    """Bind actual local roots/includes from the entirely fresh target tree."""
    roots = set()
    for obj in artifacts:
        q = local_dep_path(obj['target']['src_path'])
        if q is not None:
            if q not in BOUND:
                fail('SOURCE_CLOSURE')
            roots.add(q)
        for name in obj.get('filenames', []):
            if not pathlib.Path(name).resolve().is_relative_to(WORK / 'target'):
                fail('CHECK_PROTOCOL')
    observed = set()
    read_bytes = 0
    for dep in (WORK / 'target').rglob('*.d'):
        length = dep.stat().st_size
        read_bytes += length
        if length > 8 * 1024**2 or read_bytes > 128 * 1024**2:
            fail('SOURCE_CLOSURE')
        text = dep.read_text().replace('\\\n', '')
        if '\\ ' in text:
            fail('SOURCE_CLOSURE')
        for token in text.split():
            if token.endswith(':') or token.startswith('#'):
                continue
            q = local_dep_path(token)
            if q is None:
                continue
            if q not in BOUND:
                fail('SOURCE_CLOSURE')
            observed.add(q)
    if not roots or not roots <= observed:
        fail('SOURCE_CLOSURE')
    for q in observed:
        verify_record(SOURCE, BOUND[q])


def check_stage(stage, package, targets, features):
    STATE['stage'] = stage
    artifacts = []
    finished = []
    def consume(line):
        try:
            obj = json.loads(line)
        except (ValueError, UnicodeError):
            fail('CHECK_PROTOCOL')
        if not isinstance(obj, dict):
            fail('CHECK_PROTOCOL')
        reason = obj.get('reason')
        if reason == 'compiler-message':
            for row in diagnostic_rows(obj, stage, SOURCE, BOUND):
                if row not in STATE['diagnostics'] and len(STATE['diagnostics']) < 24:
                    STATE['diagnostics'].append(row)
        elif reason == 'compiler-artifact':
            if len(artifacts) >= 4096:
                fail('CHECK_PROTOCOL')
            # Keep only the source/target metadata necessary for closure checks.
            artifacts.append({k: obj.get(k) for k in ('target', 'features', 'filenames', 'package_id')})
        elif reason == 'build-finished':
            finished.append(obj.get('success'))
        elif reason != 'build-script-executed':
            fail('CHECK_PROTOCOL')
    cmd = [RUSTUP, 'run', TOOLCHAIN, 'cargo', 'check', '--locked', '-j1', '-p', package,
           *targets, '--message-format=json']
    if features:
        cmd += ['--features', ','.join(features)]
    _, status = run(cmd, 'CHECK', consumer=consume, allow_nonzero=True)
    if status:
        fail('CHECK')
    if finished != [True]:
        fail('CHECK_PROTOCOL')
    selected = [x for x in artifacts if x['target'].get('src_path', '').startswith(str(SOURCE / 'crates' / package) + '/')
                and x['target'].get('kind') != ['custom-build']]
    if not selected or any(sorted(x['features']) != EXPECTED_FEATURES[package] for x in selected):
        fail('FEATURES')
    verify_depinfo(artifacts)
    for row in BOUND.values():
        verify_record(SOURCE, row)
    STATE['completed'].append(stage)

def restore_test_fixture():
    delta = read_json(ROOT / 'TEST-ONLY-DELTA.json')
    row = delta['source_record']
    if row.get('path') != 'crates/alife_gpu_backend/tests/grip_profile_upload_gate.rs':
        fail('BINDING')
    if (SOURCE / row['path']).exists():
        fail('BINDING')
    verify_record(ROOT, delta['patch_record'])
    data = (ROOT / delta['patch_record']['path']).read_bytes()
    if b'\0' in data or b'GIT binary patch' in data:
        fail('PATCH')
    data.decode('utf-8', 'strict')
    patch = WORK / 'fixture.patch'
    patch.write_bytes(data)
    run(['git', 'apply', '--check', str(patch)], 'PATCH', seconds=120)
    run(['git', 'apply', str(patch)], 'PATCH', seconds=120)
    patch.unlink()
    verify_record(SOURCE, row)
    old = read_json(ROOT / 'source-manifest-v5.json')
    new = read_json(ROOT / 'source-manifest.json')
    expected = sorted(old['files'] + [row], key=lambda x: x['path'])
    if len(old['files']) != 865 or new['files'] != expected:
        fail('BINDING')
    for prior in old['files']:
        verify_record(SOURCE, prior)


def fixture_inventory():
    obj = read_json(ROOT / 'FIXTURES.json')
    rows = obj.get('fixtures', [])
    targets = obj.get('targets', [])
    if obj.get('ready') is not True or not 1 <= len(rows) <= 80 or not 1 <= len(targets) <= 12:
        fail('FIXTURE_INVENTORY')
    ids = set()
    target_ids = set()
    names = set()
    for target in targets:
        if (set(target) != {'id', 'package', 'kind', 'name', 'features'}
                or not re.fullmatch('[a-z][a-z0-9_]{1,50}', target['id'])
                or target['id'] in target_ids
                or target['package'] not in {'alife_core', 'alife_world', 'alife_training', 'alife_gpu_backend'}
                or target['kind'] not in {'lib', 'test'}
                or not re.fullmatch('[a-z][a-z0-9_]{1,80}', target['name'])
                or target['features'] != {'alife_core': [], 'alife_world': [], 'alife_training': ['cpu-backend'], 'alife_gpu_backend': ['training-rollout']}[target['package']]):
            fail('FIXTURE_INVENTORY')
        target_ids.add(target['id'])
    for row in rows:
        key = (row.get('target_id'), row.get('name'))
        if (not re.fullmatch('f[0-9]{3}', row.get('id', '')) or row['id'] in ids
                or key in names or row.get('target_id') not in target_ids
                or not re.fullmatch('[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*', row.get('name', ''))
                or len(row['name']) > 240 or row.get('ignored') is not False
                or row.get('source_path') not in BOUND
                or row.get('source_sha256') != BOUND[row['source_path']]['sha256']
                or not row.get('dose')
                or any(not re.fullmatch('[a-z_]{1,70}', k) or type(v) is not int or not 0 <= v <= 10000000 for k, v in row['dose'].items())):
            fail('FIXTURE_INVENTORY')
        ids.add(row['id']); names.add(key)
    if {row['target_id'] for row in rows} != target_ids:
        fail('FIXTURE_INVENTORY')
    return targets, rows


def build_fixture_target(spec):
    STATE['stage'] = 'build_' + spec['id']
    artifacts = []
    finished = []
    def consume(line):
        try:
            obj = json.loads(line)
        except (ValueError, UnicodeError):
            fail('CHECK_PROTOCOL')
        if not isinstance(obj, dict):
            fail('CHECK_PROTOCOL')
        reason = obj.get('reason')
        if reason == 'compiler-message':
            for row in diagnostic_rows(obj, STATE['stage'], SOURCE, BOUND):
                if row not in STATE['diagnostics'] and len(STATE['diagnostics']) < 24:
                    STATE['diagnostics'].append(row)
        elif reason == 'compiler-artifact':
            if len(artifacts) >= 4096:
                fail('CHECK_PROTOCOL')
            artifacts.append({k: obj.get(k) for k in ('target', 'features', 'filenames', 'package_id', 'profile', 'executable')})
        elif reason == 'build-finished':
            finished.append(obj.get('success'))
        elif reason != 'build-script-executed':
            fail('CHECK_PROTOCOL')
    target_args = ['--lib'] if spec['kind'] == 'lib' else ['--test', spec['name']]
    cmd = [RUSTUP, 'run', TOOLCHAIN, 'cargo', 'test', '--no-run', '--locked', '-j1', '-p', spec['package'],
           *target_args, '--message-format=json']
    if spec['features']:
        cmd += ['--features', ','.join(spec['features'])]
    _, status = run(cmd, 'CHECK', consumer=consume, allow_nonzero=True)
    if status:
        fail('CHECK')
    if finished != [True]:
        fail('CHECK_PROTOCOL')
    selected = [x for x in artifacts if x['target'].get('src_path', '').startswith(str(SOURCE / 'crates' / spec['package']) + '/')
                and x['target'].get('kind') != ['custom-build']]
    if not selected or any(sorted(x['features']) != EXPECTED_FEATURES[spec['package']] for x in selected):
        fail('FEATURES')
    candidates = [x for x in selected if x.get('profile', {}).get('test') is True
                  and x['target'].get('name') == spec['name'] and x.get('executable')]
    if len(candidates) != 1:
        fail('CHECK_PROTOCOL')
    executable = pathlib.Path(candidates[0]['executable'])
    if (not executable.is_file() or executable.is_symlink()
            or not executable.resolve().is_relative_to(WORK / 'target')
            or not os.access(executable, os.X_OK)):
        fail('CHECK_PROTOCOL')
    verify_depinfo(artifacts)
    for row in BOUND.values():
        verify_record(SOURCE, row)
    identity = digest_file(executable)
    STATE['compiled_target_ids'].append(spec['id'])
    STATE['binary_identities'].append({'target_id': spec['id'], 'sha256': identity})
    return executable, identity


def validate_exact_listing(raw, name):
    try:
        lines = [line for line in raw.decode('ascii', 'strict').splitlines() if line]
    except (ValueError, UnicodeError):
        fail('TEST_LIST')
    expected = name + ': test'
    if lines not in ([expected], [expected, '1 test, 0 benchmarks']):
        fail('TEST_LIST')


def validate_exact_result(raw):
    try:
        lines = [line for line in raw.decode('utf-8', 'strict').splitlines() if line]
    except (ValueError, UnicodeError):
        fail('TEST_PROTOCOL')
    if lines.count('running 1 test') != 1 or not lines:
        fail('TEST_PROTOCOL')
    if not re.fullmatch(r'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out; finished in [0-9]+(?:\.[0-9]+)?s', lines[-1]):
        fail('TEST_PROTOCOL')


def add_dose(field, row):
    for key, value in row['dose'].items():
        STATE[field][key] = STATE[field].get(key, 0) + value


def list_exact_fixture(row, executable, identity):
    STATE['stage'] = 'list_' + row['id']
    if digest_file(executable) != identity or row['id'] in STATE['listed_fixture_ids']:
        fail('BINDING')
    raw, status = run([str(executable), '--exact', row['name'], '--list', '--format', 'terse'],
                      'TEST_LIST', capture=True, allow_nonzero=True, seconds=120, cwd=WORK / 'temp')
    if status:
        fail('TEST_LIST')
    validate_exact_listing(raw, row['name'])
    STATE['listed_fixture_ids'].append(row['id'])


def execute_exact_fixture(row, executable, identity):
    STATE['stage'] = 'run_' + row['id']
    if (digest_file(executable) != identity or row['id'] not in STATE['listed_fixture_ids']
            or row['id'] in STATE['attempted_fixture_ids']):
        fail('BINDING')
    directory = WORK / 'fixtures' / row['id']
    if directory.exists():
        fail('BINDING')
    directory.mkdir(mode=0o700, parents=True)
    STATE['attempted_fixture_ids'].append(row['id'])
    STATE['launched_fixture_processes'] += 1
    add_dose('source_audited_attempt_dose', row)
    raw, status = run([str(executable), '--exact', row['name'], '--test-threads=1', '--format', 'terse'],
                      'TEST_EXECUTION', capture=True, allow_nonzero=True, cwd=directory)
    if status:
        fail('TEST_EXECUTION')
    validate_exact_result(raw)
    STATE['confirmed_passed_test_bodies'] += 1
    STATE['passed_fixture_ids'].append(row['id'])
    add_dose('source_audited_passed_dose', row)
    if digest_file(executable) != identity:
        fail('BINDING')


def summary(code):
    # STATE is assigned only from static stages, validated public identities,
    # sanitized diagnostic_rows and bounded counters; no exception/native text.
    result = {'schema': 'm2-grip-fixed-fixture-summary-v1', 'plan_id': PLAN,
              'code': code, **STATE}
    result['elapsed_seconds'] = min(5401, int(time.monotonic() - START))
    raw = json.dumps(result, sort_keys=True, separators=(',', ':'), allow_nan=False)
    return raw if len(raw.encode()) <= LIMITS['summary_bytes'] else '{"code":"OUTPUT_LIMIT"}'

def main():
    global WORKSPACE, WORK, SOURCE, ENV
    original = dict(os.environ)
    if original.get('GITHUB_ACTIONS') != 'true' or original.get('GITHUB_RUN_ATTEMPT') != '1':
        fail('ADMISSION')
    WORKSPACE = pathlib.Path(original['GITHUB_WORKSPACE']).resolve(strict=True)
    SOURCE = WORKSPACE / 'source'
    if SOURCE.is_symlink() or not SOURCE.is_dir():
        fail('BINDING')
    WORK = pathlib.Path(original['RUNNER_TEMP']).resolve(strict=True) / PLAN
    # Never accept a prior target/cache/session, including a previous failed run.
    if WORK.exists() or WORK.is_symlink() or WORK.is_relative_to(WORKSPACE):
        fail('BINDING')
    WORK.mkdir(mode=0o700)
    for name in ('home', 'cargo', 'target', 'temp'):
        (WORK / name).mkdir(mode=0o700)
    ENV = {k: original[k] for k in ('PATH', 'HOME') if k in original}
    ENV.update({'CARGO_HOME': str(WORK / 'cargo'), 'CARGO_TARGET_DIR': str(WORK / 'target'),
                'CARGO_INCREMENTAL': '0', 'CARGO_BUILD_JOBS': '1',
                'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_PROFILE_TEST_DEBUG': '0',
                'CARGO_PROFILE_DEV_INCREMENTAL': 'false', 'CARGO_PROFILE_TEST_INCREMENTAL': 'false',
                'CARGO_TERM_COLOR': 'never', 'CARGO_NET_RETRY': '0',
                'RUST_BACKTRACE': '0', 'RUST_TEST_THREADS': '1',
                'LANG': 'C.UTF-8', 'LC_ALL': 'C.UTF-8',
                'TMPDIR': str(WORK / 'temp'), 'TMP': str(WORK / 'temp'), 'TEMP': str(WORK / 'temp'),
                'RUSTUP_DIST_SERVER': 'https://static.rust-lang.org',
                'RUSTUP_UPDATE_ROOT': 'https://static.rust-lang.org/rustup',
                'RUSTUP_NO_UPDATE_CHECK': '1', 'RUSTUP_PERMIT_COPY_RENAME': '1',
                'GIT_CONFIG_NOSYSTEM': '1', 'GIT_TERMINAL_PROMPT': '0'})
    admit()
    restore_sources()
    targets, fixtures = fixture_inventory()
    setup_toolchain(original)
    binaries = {spec['id']: build_fixture_target(spec) for spec in targets}
    # Admit the entire exact inventory before executing any fixture body.
    for row in fixtures:
        list_exact_fixture(row, *binaries[row['target_id']])
    for row in fixtures:
        execute_exact_fixture(row, *binaries[row['target_id']])
    for row in BOUND.values():
        verify_record(SOURCE, row)
    STATE['stage'] = 'complete'
    check_limits()

if __name__ == '__main__':
    code = 'OK'
    try:
        main()
    except Stop as exc:
        code = str(exc) if str(exc) in CODES else 'INTERNAL'
    except BaseException:
        code = 'INTERNAL'
    print(summary(code), flush=True)
    sys.exit(0 if code == 'OK' else 1)
