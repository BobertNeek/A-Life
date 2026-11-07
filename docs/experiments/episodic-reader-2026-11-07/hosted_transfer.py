"""Bounded hosted-only orchestration. Never uploads private native outputs."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
HOSTED = HERE / 'hosted'
GIB = 1 << 30
EXECUTION_SECONDS = 22 * 60
NEW_LIFE_MINIMUM_SECONDS = 120
STUDY_ID = 'episodic-reader-transfer-2026-11-07'
STUDY_REPOSITORY = 'BobertNeek/A-Life'
STUDY_BRANCH = 'experiment/episodic-reader-transfer-20261107'
STUDY_PLAN_SHA256 = 'a7b6fd8952cb40d415f0de24de1a65c1c7b2b974daa665b73c0aa0b78e6698f3'
PUBLIC_FILES = ('report.md', 'summary.json', 'causal-rows.json', 'checks.json',
                'build-receipt.json', 'source-only.patch', 'SHA256SUMS')
CHECK_KEYS = {'schema','status','integration_qualification','source_bound','native_build_completed',
              'exposure_started','attempted_cases','observed_cases','completed_cases','failed_cases','failure_stage','failure_code'}
FAILURE_CODES = {'','integration_qualification','initial_execution_guard','disk_budget','source_verification','frozen_core_tests','frozen_cpu_example_tests','frozen_build','prepare_binding','native_case','sanitizer_schema','orchestration_contract','wall_clock_timeout'}
class HostedFailure(RuntimeError):
    def __init__(self,code):
        assert code in FAILURE_CODES
        self.code=code
        super().__init__(code)

RECEIPT_KEYS = {'schema','integration_head','frozen_commit','base_commit','reconstruction_sha256',
                'episode_patch_sha256','source_manifest_sha256','plan_sha256','bound_input_count',
                'effective_inputs_sha256','rustc','executable_sha256','asset_sha256','build_profile',
                'build_flags','source_only_patch_role'}

def sha(data): return hashlib.sha256(data).hexdigest()
def file_sha(path): return sha(Path(path).read_bytes())
def read(path): return json.loads(Path(path).read_text())
def write(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')
    temporary.replace(path)

def remaining_time(deadline, minimum=1):
    remaining=deadline-time.monotonic()
    if remaining<minimum: raise HostedFailure('wall_clock_timeout')
    return remaining

def require_initial_study_event():
    expected = {'GITHUB_EVENT_NAME': 'pull_request', 'GITHUB_RUN_ATTEMPT': '1',
                'TRANSFER_EVENT_ACTION': 'opened', 'GITHUB_REPOSITORY': STUDY_REPOSITORY,
                'TRANSFER_HEAD_REPOSITORY': STUDY_REPOSITORY, 'GITHUB_HEAD_REF': STUDY_BRANCH,
                'TRANSFER_STUDY_ID': STUDY_ID}
    if any(os.environ.get(key) != value for key, value in expected.items()):
        raise HostedFailure('initial_execution_guard')
    if file_sha(HOSTED/'transfer-plan.json') != STUDY_PLAN_SHA256:
        raise HostedFailure('initial_execution_guard')

def require_space(path, minimum):
    if shutil.disk_usage(path).free < minimum:
        raise RuntimeError('insufficient_disk_budget')

def patch_paths(data):
    return [line[6:] for line in data.decode().splitlines() if line.startswith('+++ b/')]

def verify_source(study, manifest, episode=False):
    expected = dict(manifest['frozen_inputs'])
    if episode: expected.update(manifest['episode_inputs'])
    for name, digest in expected.items():
        path = study / name
        if path.is_symlink() or not path.is_file() or file_sha(path) != digest:
            raise RuntimeError('source_hash_mismatch:' + name)
    # Explicitly reject additional build inputs in the relevant source closure.
    closure = ('alife_storage','alife_core','alife_world','alife_gpu_backend','alife_runtime','alife_training')
    actual = set()
    for crate in closure:
        actual.update(str(p.relative_to(study)) for p in (study/'crates'/crate).rglob('*') if p.is_file())
    if actual != {name for name in expected if any(name.startswith('crates/'+c+'/') for c in closure)}:
        raise RuntimeError('unexpected_or_missing_build_input')
    return sha(json.dumps(expected, sort_keys=True, separators=(',', ':')).encode())

def validate_public(public):
    public = Path(public)
    if {p.name for p in public.iterdir()} != set(PUBLIC_FILES):
        raise RuntimeError('public_file_allowlist_mismatch')
    for name in PUBLIC_FILES:
        p = public/name
        if p.is_symlink() or not p.is_file() or p.stat().st_size > 2_000_000:
            raise RuntimeError('public_file_type_or_size')
    checks = read(public/'checks.json'); receipt = read(public/'build-receipt.json')
    if set(checks) != CHECK_KEYS or set(receipt) != RECEIPT_KEYS:
        raise RuntimeError('public_metadata_schema')
    if checks['schema'] != 'hosted-transfer-checks-v1' or receipt['schema'] != 'hosted-transfer-build-v1':
        raise RuntimeError('public_metadata_version')
    if checks['status'] not in ('not_started','partial','completed','infrastructure_failure'):
        raise RuntimeError('public_status')
    for key in ('source_bound','native_build_completed','exposure_started'):
        if type(checks[key]) is not bool: raise RuntimeError('public_boolean')
    if checks['integration_qualification'] not in ('success','failure','cancelled','skipped','unknown'):
        raise RuntimeError('qualification_status')
    allowed = {(2026111201+i,arm) for i in range(4) for arm in ('family-value-off','family-value-on')}
    for key in ('attempted_cases','observed_cases','completed_cases','failed_cases'):
        cases=checks[key]
        if not isinstance(cases,list) or len(cases)>8: raise RuntimeError('case_count')
        pairs=[]
        for case in cases:
            if set(case)!= {'seed','arm'} or (case['seed'],case['arm']) not in allowed: raise RuntimeError('case_schema')
            pairs.append((case['seed'],case['arm']))
        if len(set(pairs)) != len(pairs): raise RuntimeError('duplicate_case')
    if checks['failure_stage'] not in ('','qualification','source','build','prepare','life','report','disk','attempt_guard','time_budget'):
        raise RuntimeError('failure_stage')
    if checks['failure_code'] not in FAILURE_CODES: raise RuntimeError('failure_code')
    # Receipts contain only bounded strings, fixed plans and hashes, never raw state.
    for key in RECEIPT_KEYS - {'bound_input_count','asset_sha256','build_flags'}:
        if not isinstance(receipt[key],str) or len(receipt[key])>4000: raise RuntimeError('receipt_string')
    for key in ('reconstruction_sha256','episode_patch_sha256','source_manifest_sha256','plan_sha256','effective_inputs_sha256','executable_sha256'):
        value=receipt[key]
        if value and (len(value)!=64 or any(c not in '0123456789abcdef' for c in value)): raise RuntimeError('receipt_digest')
    if type(receipt['bound_input_count']) is not int or not 0<=receipt['bound_input_count']<=5000: raise RuntimeError('input_count')
    plan=read(HOSTED/'transfer-plan.json')
    if receipt['asset_sha256'] != plan['asset_sha256'] or receipt['build_flags'] != ['jobs=1','incremental=0','dev-debug=0','test-debug=0','cpu-backend']:
        raise RuntimeError('receipt_plan')
    declared=read(HOSTED/'source-manifest.json')
    fixed={'frozen_commit':declared['frozen_commit'],'base_commit':declared['base_commit'],
           'reconstruction_sha256':declared['reconstruction_sha256'],'episode_patch_sha256':declared['episode_patch_sha256'],
           'source_manifest_sha256':file_sha(HOSTED/'source-manifest.json'),'plan_sha256':file_sha(HOSTED/'transfer-plan.json')}
    if any(receipt[key]!=value for key,value in fixed.items()): raise RuntimeError('receipt_source_binding')
    expected_head=os.environ.get('EXPECTED_SOURCE_REVISION')
    if expected_head and receipt['integration_head']!=expected_head: raise RuntimeError('receipt_head_binding')
    if receipt['build_profile']!='dev': raise RuntimeError('receipt_profile')
    effective=dict(declared['frozen_inputs']);effective.update(declared['episode_inputs'])
    effective_digest=sha(json.dumps(effective,sort_keys=True,separators=(',', ':')).encode())
    if checks['source_bound']:
        if receipt['bound_input_count']!=len(effective) or receipt['effective_inputs_sha256']!=effective_digest: raise RuntimeError('receipt_input_binding')
    elif receipt['bound_input_count']!=0 or receipt['effective_inputs_sha256']: raise RuntimeError('unbound_input_claim')
    if checks['native_build_completed'] and (not checks['source_bound'] or not receipt['executable_sha256'] or not receipt['rustc'].startswith('rustc 1.99.1 ')): raise RuntimeError('build_completion_binding')
    if (checks['exposure_started'] or checks['observed_cases'] or checks['completed_cases']) and not (checks['source_bound'] and checks['native_build_completed']): raise RuntimeError('exposure_without_build_binding')
    module = reporting()
    summary,_=module.validate_public_outputs(public)
    if summary['source_commit'] is not None:
        if receipt['integration_head']!=summary['source_commit'] or receipt['executable_sha256']!=summary['executable_sha256'] or summary['retrieval_source_sha256']!=declared['retrieval_source_sha256']: raise RuntimeError('summary_receipt_binding')
    elif checks['exposure_started']: raise RuntimeError('exposure_without_manifest')
    validated={(life['seed'],life['arm']) for life in summary['lives']}
    completed={(case['seed'],case['arm']) for case in checks['completed_cases']}
    observed={(case['seed'],case['arm']) for case in checks['observed_cases']}
    failed={(case['seed'],case['arm']) for case in checks['failed_cases']}
    runtime_failed={(life['seed'],life['arm']) for life in summary['lives'] if life['stop_reason']=='host_error'}
    attempted={(case['seed'],case['arm']) for case in checks['attempted_cases']}
    if observed != validated or not observed <= attempted or failed != runtime_failed or completed != observed-failed: raise RuntimeError('public_completed_case_mismatch')
    if checks['status']=='completed' and (len(completed)!=8 or summary['status']!='complete'): raise RuntimeError('false_completed_status')
    lines=(public/'SHA256SUMS').read_text().splitlines()
    expected=[file_sha(public/name)+'  '+name for name in PUBLIC_FILES if name!='SHA256SUMS']
    if lines!=expected: raise RuntimeError('public_hashes')
    if (public/'source-only.patch').read_bytes()!=(HOSTED/'episode-only.patch').read_bytes():
        raise RuntimeError('public_source_patch')

def reporting():
    spec=importlib.util.spec_from_file_location('transfer_reporting',HERE/'transfer_reporting.py')
    module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module

def report(private, public):
    result=subprocess.run([sys.executable,str(HERE/'transfer_reporting.py'),str(private),str(public),'--allow-not-started'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=30)
    if result.returncode not in (0,2): raise RuntimeError('sanitized_report_failed')
    reporting().validate_public_outputs(public)

def initial_metadata(public, head, qualification):
    manifest=read(HOSTED/'source-manifest.json'); plan=read(HOSTED/'transfer-plan.json')
    checks={'schema':'hosted-transfer-checks-v1','status':'not_started','integration_qualification':qualification,
            'source_bound':False,'native_build_completed':False,'exposure_started':False,
            'attempted_cases':[],'observed_cases':[],'completed_cases':[],'failed_cases':[],'failure_stage':'','failure_code':''}
    receipt={'schema':'hosted-transfer-build-v1','integration_head':head,'frozen_commit':manifest['frozen_commit'],
             'base_commit':manifest['base_commit'],'reconstruction_sha256':manifest['reconstruction_sha256'],
             'episode_patch_sha256':manifest['episode_patch_sha256'],'source_manifest_sha256':file_sha(HOSTED/'source-manifest.json'),
             'plan_sha256':file_sha(HOSTED/'transfer-plan.json'),'bound_input_count':0,'effective_inputs_sha256':'','rustc':'',
             'executable_sha256':'','asset_sha256':plan['asset_sha256'],'build_profile':'dev',
             'build_flags':['jobs=1','incremental=0','dev-debug=0','test-debug=0','cpu-backend'],
             'source_only_patch_role':'Declared episode-only delta against the hash-verified frozen 612c study closure.'}
    public.mkdir(parents=True,exist_ok=True)
    write(public/'checks.json',checks);write(public/'build-receipt.json',receipt)
    shutil.copyfile(HOSTED/'episode-only.patch',public/'source-only.patch')
    return checks,receipt

def execute(root, workspace, head, qualification):
    private=root/'private'; public=root/'public'; logs=root/'private-logs'; logs.mkdir(parents=True,exist_ok=True)
    checks,receipt=initial_metadata(public,head,qualification)
    stage='qualification';deadline=time.monotonic()+EXECUTION_SECONDS
    def save():
        write(public/'checks.json',checks);write(public/'build-receipt.json',receipt)
    def command(name,args,cwd=None):
        with (logs/(name+'.log')).open('wb') as log:
            try:
                proc=subprocess.run(args,cwd=cwd,stdout=log,stderr=subprocess.STDOUT,timeout=max(1,remaining_time(deadline)-30))
            except subprocess.TimeoutExpired as error:
                raise HostedFailure('wall_clock_timeout') from error
        if proc.returncode:
            code={'frozen-core':'frozen_core_tests','frozen-cpu':'frozen_cpu_example_tests','frozen-build':'frozen_build','prepare':'prepare_binding'}.get(name,'native_case' if name.startswith('life-') else 'source_verification')
            raise HostedFailure(code)
    try:
        report(private,public)
        if qualification!='success': raise RuntimeError('qualification_failed')
        stage='attempt_guard'
        require_initial_study_event()
        stage='disk';require_space(root,12*GIB)
        stage='source'
        if subprocess.check_output(['git','rev-parse','HEAD'],cwd=workspace,text=True,timeout=min(30,remaining_time(deadline))).strip()!=head: raise RuntimeError('integration_head_mismatch')
        manifest=read(HOSTED/'source-manifest.json')
        if file_sha(HOSTED/'frozen-study.patch')!=manifest['reconstruction_sha256'] or file_sha(HOSTED/'episode-only.patch')!=manifest['episode_patch_sha256']:
            raise RuntimeError('patch_hash_mismatch')
        if set(patch_paths((HOSTED/'frozen-study.patch').read_bytes()))!=set(manifest['reconstruction_paths']) or patch_paths((HOSTED/'episode-only.patch').read_bytes())!=manifest['episode_paths']:
            raise RuntimeError('patch_scope_mismatch')
        # No current-branch files are replaced: all historical source is isolated.
        command('fetch-base',['git','fetch','--no-tags','origin',manifest['base_commit']],workspace)
        study=root/'frozen-study'
        command('worktree',['git','worktree','add','--detach',str(study),manifest['base_commit']],workspace)
        command('reconstruct',['git','apply','--check',str(HOSTED/'frozen-study.patch')],study)
        command('reconstruct-apply',['git','apply',str(HOSTED/'frozen-study.patch')],study)
        verify_source(study,manifest)
        command('episode-check',['git','apply','--check',str(HOSTED/'episode-only.patch')],study)
        command('episode-apply',['git','apply',str(HOSTED/'episode-only.patch')],study)
        receipt['effective_inputs_sha256']=verify_source(study,manifest,True)
        receipt['bound_input_count']=len(manifest['frozen_inputs'])
        # The integration assay must be precisely this declared extension too.
        for name,digest in manifest['episode_inputs'].items():
            if file_sha(workspace/name)!=digest: raise RuntimeError('integration_episode_diff_mismatch')
        checks['source_bound']=True;save()
        stage='build';os.environ['CARGO_TARGET_DIR']=str(root/'frozen-target')
        receipt['rustc']=subprocess.check_output(['rustc','-Vv'],text=True,timeout=min(30,remaining_time(deadline))).strip()
        if not receipt['rustc'].startswith('rustc 1.99.1 '): raise RuntimeError('toolchain_version')
        command('frozen-core',['cargo','test','--locked','--profile','dev','-p','alife_core','--test','candidate_memory_retrieval','--test','n2048_memory_gain_candidate'],study)
        command('frozen-cpu',['cargo','test','--locked','--profile','dev','-p','alife_training','--features','cpu-backend','--test','cpu_memory_gain_candidate','--example','probe_native_learning'],study)
        command('frozen-build',['cargo','build','--locked','--profile','dev','-p','alife_training','--features','cpu-backend','--example','probe_native_learning'],study)
        binary=root/'frozen-target/debug/examples/probe_native_learning'
        receipt['executable_sha256']=file_sha(binary);checks['native_build_completed']=True;save()
        stage='prepare';command('prepare',[str(binary),'prepare-transfer',str(private),head],study)
        m=read(private/'manifest.json');plan=read(HOSTED/'transfer-plan.json')
        if m['plan']['episodes']!=plan['episodes'] or m['plan']['asset_sha256']!=plan['asset_sha256'] or m['retrieval_source_sha256']!=manifest['retrieval_source_sha256']:
            raise RuntimeError('runtime_plan_binding')
        report(private,public)
        for episode in plan['episodes']:
            for arm in ('family-value-off','family-value-on'):
                stage='time_budget';remaining_time(deadline,NEW_LIFE_MINIMUM_SECONDS)
                stage='disk';require_space(root,2*GIB+(256<<20))
                case={'seed':episode['seed'],'arm':arm}
                checks['exposure_started']=True;checks['attempted_cases'].append(case);checks['status']='partial';save()
                stage='life'
                command(f"life-{episode['seed']}-{arm}",[str(binary),'transfer-one',str(private),str(episode['seed']),arm],study)
                stage='report';report(private,public)
                checks['observed_cases'].append(case);checks['completed_cases'].append(case);save()
        if len(checks['completed_cases'])!=8: raise RuntimeError('coverage')
        checks['status']='completed';save()
    except Exception as error:
        codes={'qualification':'integration_qualification','attempt_guard':'initial_execution_guard','disk':'disk_budget','source':'source_verification','build':'orchestration_contract','prepare':'prepare_binding','life':'native_case','report':'sanitizer_schema','time_budget':'wall_clock_timeout'}
        checks['status']='infrastructure_failure';checks['failure_stage']=stage
        checks['failure_code']=error.code if isinstance(error,HostedFailure) else ('wall_clock_timeout' if isinstance(error,subprocess.TimeoutExpired) else codes.get(stage,'orchestration_contract'));save()
        # Raw error text and native logs stay private. The stage is enough for the public blocker.
        return False
    return True

def finalize(root,head,qualification):
    public=root/'public';private=root/'private'
    if not (public/'checks.json').exists(): initial_metadata(public,head,qualification)
    try:
        report(private,public)
    except Exception:
        # Preserve the most recent independently validated partial report.
        for name in ('report.md','summary.json','causal-rows.json'):
            if not (public/name).is_file(): raise RuntimeError('no_valid_sanitized_report')
        checks=read(public/'checks.json');checks['status']='infrastructure_failure';checks['failure_stage']=checks['failure_stage'] or 'report';checks['failure_code']=checks['failure_code'] or 'sanitizer_schema';write(public/'checks.json',checks)
    checks=read(public/'checks.json')
    summary,_=reporting().validate_public_outputs(public)
    validated=[{'seed':life['seed'],'arm':life['arm']} for life in summary['lives']]
    known={(c['seed'],c['arm']) for c in checks['completed_cases']}
    observed={(c['seed'],c['arm']) for c in validated}
    attempted={(c['seed'],c['arm']) for c in checks['attempted_cases']}
    if not known <= observed or not observed <= attempted: raise RuntimeError('finalization_coverage_conflict')
    checks['observed_cases']=validated
    failed={(life['seed'],life['arm']) for life in summary['lives'] if life['stop_reason']=='host_error'}
    checks['failed_cases']=[case for case in validated if (case['seed'],case['arm']) in failed]
    checks['completed_cases']=[case for case in validated if (case['seed'],case['arm']) not in failed]
    if checks['status']=='partial':
        checks['status']='infrastructure_failure';checks['failure_code']=checks['failure_code'] or 'orchestration_contract'
    write(public/'checks.json',checks)
    (public/'SHA256SUMS').write_text(''.join(file_sha(public/name)+'  '+name+'\n' for name in PUBLIC_FILES if name!='SHA256SUMS'))
    validate_public(public)
    # Print only the exact validated allowlisted payloads, never raw logs.
    for name in ('summary.json','checks.json','build-receipt.json','causal-rows.json','SHA256SUMS'):
        content=(public/name).read_text()
        if len(content.encode())<=131072:
            print('BEGIN_SANITIZED_TRANSFER_'+name)
            print(content,end='')
            print('END_SANITIZED_TRANSFER_'+name)
        else:
            print('SANITIZED_TRANSFER_OMITTED_FROM_LOG_SIZE_LIMIT '+name+' sha256='+file_sha(public/name))

def main():
    parser=argparse.ArgumentParser();parser.add_argument('mode',choices=['execute','finalize','validate']);parser.add_argument('root',type=Path)
    parser.add_argument('--workspace',type=Path,default=Path.cwd());parser.add_argument('--head',default=os.environ.get('EXPECTED_SOURCE_REVISION',''))
    parser.add_argument('--qualification',default=os.environ.get('QUALIFICATION_RESULT','unknown'));args=parser.parse_args()
    args.root.mkdir(parents=True,exist_ok=True)
    if args.mode=='execute':return 0 if execute(args.root,args.workspace,args.head,args.qualification) else 1
    if args.mode=='finalize':finalize(args.root,args.head,args.qualification)
    else:validate_public(args.root/'public')
    return 0
if __name__=='__main__':sys.exit(main())
