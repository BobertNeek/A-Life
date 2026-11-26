#!/usr/bin/env python3
"""Execute the pinned native saved-life evaluator, one arm and one shared lease."""
from datetime import datetime, timezone
import fcntl, hashlib, json, os, resource, shutil, signal, stat, subprocess, sys, time
from pathlib import Path

LAB=Path(__file__).resolve().parents[1]
ROOT=LAB.parent
ELF=LAB/'bin/evaluate_saved_life_retention'
DRIVER=ROOT/'qualia-public-runtime-reset-20261126/isolated-driver-work/vetted-driver/usr'
LIB=DRIVER/'lib/x86_64-linux-gnu'
ICD=DRIVER/'share/vulkan/icd.d/lvp_icd.json'
LOCK=ROOT/'.qualia-stock-U-A-exclusive.lock'
CARD_PIN='8544f30571929b83198c206dd6a94ac97feac7db055784647f1014fae59e3129'
CARD=LAB/'active-disclosed29-v1.json'
CAP=1_000_000_000
MIN_B=9_900_000_000
MIN_RAM=5_242_880
ELF_PIN='588b60942ded9959eef5860e96bf546b2acd05a71fe6a6d77d9296304ff22bf6'

def digest(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f,'sha256').hexdigest()

def read(p): return json.loads(p.read_bytes())

def utc(): return datetime.now(timezone.utc).isoformat()

def emit(p,v):
    with p.open('x') as f:
        json.dump(v,f,indent=2,sort_keys=True,allow_nan=False);f.write('\n')

def mem():
    return int(next(r.split()[1] for r in Path('/proc/meminfo').read_text().splitlines() if r.startswith('MemAvailable:')))

def free(path): return shutil.disk_usage(path).free

def owned_bytes(path):
    return sum(f.lstat().st_blocks*512 for f in path.rglob('*') if f.is_file())

def guard_identity(fd):
    a=os.fstat(fd); b=LOCK.lstat()
    assert (a.st_dev,a.st_ino)==(b.st_dev,b.st_ino)
    assert stat.S_ISREG(a.st_mode) and not LOCK.is_symlink() and stat.S_IMODE(a.st_mode)==0o600
    assert a.st_size==0 and a.st_uid==os.getuid() and a.st_nlink==1

def group_live(pid):
    for x in Path('/proc').glob('[0-9]*/stat'):
        try:
            q=x.read_text().rsplit(')',1)[1].split()
            if len(q)>2 and q[0] not in ('Z','X') and int(q[2])==pid:return True
        except (OSError,ValueError,IndexError):pass
    return False

def child_limits():
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    resource.setrlimit(resource.RLIMIT_FSIZE,(268435456,268435456))

def canonical(x):
    return json.dumps(x,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()

def case_contract(case):
    assert digest(CARD)==CARD_PIN
    card=read(CARD)
    row=card['cases'][case]; s=row['scenario']
    assert hashlib.sha256(canonical(s)).hexdigest()==row['scenario_sha256']
    return card,row,s


def check_runtime():
    assert digest(ELF)==ELF_PIN
    assert digest(LIB/'libvulkan_lvp.so')=='0004262f4dc95585492d55925face67a79a7c869ad8760b42ae01bd0bdb807a2'
    assert digest(ICD)=='9a6a0d616f3f2efc2d227f89653047e5330ff634b0519ffbf1b2c60dc1c0e0e7'


def admitted_live(card):
    with (LAB/'SURGERY_STATE.json.lock').open('rb') as gate:
        fcntl.flock(gate,fcntl.LOCK_SH)
        state=read(LAB/'SURGERY_STATE.json')
    controls=state['saved_life_controls']; active=controls.get('active_card') or {}
    assert controls['round_paused'] is False and state['known_good_eval_bin_sha256']==card['executable_sha256']==ELF_PIN
    assert active.get('sha256')==CARD_PIN and active.get('id')==card['scorecard_id']
    source=LAB/state['active_base_dir']/'Current.alife-checkpoint'
    sd=source.lstat()
    assert stat.S_ISREG(sd.st_mode) and sd.st_uid==os.getuid() and stat.S_IMODE(sd.st_mode) in (0o400,0o600)
    msha=digest(source)
    assert msha==controls['full_saved_life_current_sha256']
    return state, source, msha


def make_batch(name):
    batch=LAB/'private'/name
    batch.mkdir(exist_ok=True,mode=0o700)
    # These availability checks apply to the filesystem holding all scratch, including TMPDIR.
    assert not batch.is_symlink() and free(batch)>=MIN_B and free(batch)>=4_000_000_000 and mem()>=MIN_RAM
    return batch


def run_arm(lease, work, case, row, s, source, msha, req, author):
    work.mkdir(mode=0o700)
    shutil.copyfile(source,work/'Current.alife-checkpoint'); copy=work/'Current.alife-checkpoint';copy.chmod(0o400)
    assert digest(copy)==msha and copy.stat().st_ino!=source.stat().st_ino
    for n in ('home','cache','tmp','runtime'):(work/n).mkdir(mode=0o700)
    exe=[str(ELF),'--experiment-manifest',str(work),str(copy),str(work/'native'),str(work/'manifest.json')]
    now=int(time.time())
    req=dict(req,deadline_unix_seconds=now+420,finalization_deadline_unix_seconds=now+510)
    emit(work/'manifest.json',req)
    env={'PATH':'/usr/bin:/bin','LANG':'C.UTF-8','HOME':str(work/'home'),'TMPDIR':str(work/'tmp'),
        'XDG_CACHE_HOME':str(work/'cache'),'XDG_RUNTIME_DIR':str(work/'runtime'),
        'ALIFE_EXPLICIT_FIXED_HOST_PRESSURE_NS':'0','ALIFE_EXPLICIT_SOFTWARE_RETENTION_TEST':'1',
        'ALIFE_SLM_PRIOR':'off','ALIFE_SLM_PRIOR_BACKEND':'off','LD_LIBRARY_PATH':str(LIB),'VK_DRIVER_FILES':str(ICD)}
    ar={**author,'case':case,'source_current_sha256':msha,'rubric_sha256':CARD_PIN,'scenario_sha256':row['scenario_sha256'],
        'evaluator_sha256':ELF_PIN,'native_manifest_sha256':digest(work/'manifest.json'),'pressure_ns':0,'prior':'off'}
    emit(work/'authority.json',ar)
    started=utc()
    stopped=None; p=None
    with (work/'stdout').open('xb') as so,(work/'stderr').open('xb') as se:
        try:
            p=subprocess.Popen(exe,cwd=work,env=env,stdin=subprocess.DEVNULL,stdout=so,stderr=se,start_new_session=True,preexec_fn=child_limits)
            emit(work/'started.json',dict(started_at_utc=started,parent_pid=os.getpid(),child_pid=p.pid))
            print(json.dumps(dict(case=case,started_at_utc=started,pid=p.pid,native_manifest_sha256=ar['native_manifest_sha256'])),flush=True)
            while p.poll() is None:
                guard_identity(lease.fileno())
                if free(work)<MIN_B or free(work)<3_000_000_000 or mem()<MIN_RAM:stopped='live resource floor';break
                if owned_bytes(work)>CAP or so.tell()+se.tell()>16*1024*1024:stopped='artifact cap';break
                if time.time()>now+539:stopped='external deadline';break
                time.sleep(0.2)
        finally:
            if p is not None:
                for sig in (signal.SIGTERM,signal.SIGKILL):
                    if p.poll() is not None and not group_live(p.pid):break
                    try:os.killpg(p.pid,sig)
                    except ProcessLookupError:pass
                    try:p.wait(5)
                    except subprocess.TimeoutExpired:pass
    rr=work/'native/retention.json'
    terminal=dict(**ar,started_at_utc=started,finished_at_utc=utc(),child_pid=p.pid,parent_pid=os.getpid(),
                 exit_code=p.returncode,group_reaped=not group_live(p.pid),stop=stopped,
                 native_receipt_sha256=digest(rr) if rr.exists() else None)
    emit(work/'process.json',terminal)
    assert p.returncode==0 and terminal['group_reaped'] and not stopped and digest(source)==digest(copy)==msha
    assert digest(ELF)==ELF_PIN
    native=read(rr)
    assert native['source_sha256']==msha and native['source_unchanged'] is True
    assert native['published_case']==case and native['published_case_registry']==s['registry']
    assert native['captured_decisions']==row['decisions']
    assert native['measurement_valid'] is True and native['gpu_restored'] is True
    assert native['prior_remained_off'] is True
    assert canonical(native['request']['direct_weights'])==canonical(req['intervention'].get('edit'))
    assert native['rows_sha256']==digest(work/'native/rows.jsonl')
    print(json.dumps(terminal),flush=True)


def watchdog(check_mode):
    q=subprocess.run([sys.executable,'scripts/surgery_loop.py','watchdog','--state','SURGERY_STATE.json','--fail-on-stall'],cwd=LAB,capture_output=True,text=True)
    cfg=json.loads(q.stdout)
    if cfg.get('verified_state_failure') is not None or cfg['status']=='ROLLBACK_REQUIRED':
        raise RuntimeError('real state failure, no native launch: '+q.stdout)
    if check_mode=='baseline':
        assert q.returncode==0 and cfg['status']=='BASELINE_REQUIRED'
    else:
        assert q.returncode==0 and cfg['status'] in ('BASELINE_REQUIRED','SCORE_QUEUE_BACKLOG','HEALTHY')
    return cfg


def run(case):
    from prepare_native_round_manifest import prepare_baseline
    os.umask(0o077)
    check_runtime()
    card,row,s=case_contract(case)
    with LOCK.open('rb') as lease:
        guard_identity(lease.fileno()); fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB)
        if (LAB/'private/active29-fresh-r41-v1/short-pair-before-next-case').exists():
            raise RuntimeError('queued matched short-pair boundary reached')
        watchdog('baseline')
        state,source,msha=admitted_live(card)
        assert state['active_base_id']=='R41'
        auth=prepare_baseline(state,CARD.read_bytes(),case,int(time.time())+420)
        batch=make_batch('active29-fresh-r41-v1')
        run_arm(lease,batch/case,case,row,s,source,msha,auth['A'],{'scorecard_id':card['scorecard_id']})


def pair(case, patch_path):
    from prepare_native_round_manifest import prepare
    os.umask(0o077)
    check_runtime()
    card,row,s=case_contract(case)
    with LOCK.open('rb') as lease:
        guard_identity(lease.fileno()); fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB)
        watchdog('pair')
        state,source,msha=admitted_live(card)
        card_bytes=CARD.read_bytes()
        auth=prepare(state,patch_path,(LAB/'original-29-disclosed.json').read_bytes(),case,s['evaluation_nonce'],int(time.time())+420,card_bytes)
        patch=read(patch_path)
        batch=make_batch('active29-matched-'+auth['patch_sha256'][:16])
        path=batch/case; path.mkdir(mode=0o700)
        author={'scorecard_id':card['scorecard_id'],'patch_id':patch['patch_id'],
                'patch_sha256':auth['patch_sha256'],'native_request_canonical_sha256':auth['native_request_canonical_sha256']}
        for arm in ('A','B'):
            guard_identity(lease.fileno())
            run_arm(lease,path/arm,case,row,s,source,msha,auth[arm],author)
        print(json.dumps({'case':case,'pair_directory':str(path),'patch_sha256':auth['patch_sha256']}),flush=True)


def run_remaining():
    order=['r2-word-approach-food','r2-word-silence','r2-word-mirror-silence',
           'r14-word-root-silence','original-autonomous-axis','original-autonomous-plus4']
    card=read(CARD)
    cases=order+[n for n in card['cases'] if n not in order]
    batch=LAB/'private/active29-fresh-r41-v1'
    for case in cases:
        path=batch/case
        if path.exists():
            p=read(path/'process.json')
            assert p['exit_code']==0 and p['group_reaped'] is True
            continue
        if (batch/'short-pair-before-next-case').exists():
            print(json.dumps({'status':'SAFE_BASELINE_BOUNDARY','next_case':case}),flush=True)
            return
        run(case)


def pair_remaining(patch_path):
    card=read(CARD)
    pin=digest(patch_path)
    batch=LAB/'private'/('active29-matched-'+pin[:16])
    for case in card['cases']:
        path=batch/case
        if path.exists():
            for arm in ('A','B'):
                closure=read(path/arm/'process.json')
                assert closure['patch_sha256']==pin and closure['exit_code']==0 and closure['group_reaped'] is True
            continue
        pair(case,patch_path)


if __name__=='__main__':
    if sys.argv[1]=='--remaining':run_remaining()
    elif sys.argv[1]=='--paired':pair(sys.argv[2],Path(sys.argv[3]).resolve())
    elif sys.argv[1]=='--paired-remaining':pair_remaining(Path(sys.argv[2]).resolve())
    else:run(sys.argv[1])
