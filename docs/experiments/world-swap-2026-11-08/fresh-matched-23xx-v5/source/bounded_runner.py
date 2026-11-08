"""NEW reconstructed, DISABLED orchestrator. Bounded pipes replace polled raw stderr files.

No __main__ dispatch route. Native main must remain disabled until separately
reviewed activation, source reproof and executable binding. No retry after dispatch.
"""
import json,os,resource,selectors,shutil,signal,subprocess,time
from pathlib import Path
import report_contract as c
import diagnostic_contract as diagnostic
import scalar_prefix_contract as prefix_contract
import prefix_runner_seam
FLOOR=2415919104;SECONDS=1320;PRIVATE_CAP=67108864;PUBLIC_CAP=16777216;STDERR_CAP=4194304;STDOUT_CAP=4096
PLAN_SHA='8bd2f5fd6cd53bd8757633eed2660b69a3820ba2509d3ccc7dea4e00e834995a'
WORLD_SHA='5cc4ea55ff8505bff852291ad0037bf05b7d1b1010a8db59cd264584332b4b2c'
WORLD_HEAD='30efe5ee3bb5a5cdf270288eb603b227d6c5cdfd'
ACTIVATION_MAIN_SHA='29e0b8af26d0a2c14d144ff8b4a2c8a6e5b90064d8e97a0cd75bc6498d5ea4fd'
PREPARATION_GUARD_SHA='24db6afa9d13b55b25ed8e3533d2d58e81678fe30cf83f48eaf5cf0ebbcee320'
PUBLIC_FILES={'report.md','summary.json','rows.json','checks.json','source-bindings.json','common-audit-kernel.patch','world-factor.patch','SHA256SUMS'}
def create(path,raw):
 with Path(path).open('xb') as f:f.write(raw);f.flush();os.fsync(f.fileno())
def bounded(path,limit=8*1024**2):
 p=Path(path);c.need(p.is_file() and not p.is_symlink() and 0<p.stat().st_size<=limit,'file_bound')
 with p.open('rb') as f:raw=f.read(limit+1)
 c.need(len(raw)<=limit,'file_bound');return raw
def git(repo,*args):
 r=subprocess.run(['git','-C',str(repo),*args],capture_output=True,timeout=20,check=False)
 c.need(r.returncode==0,'source_binding');return r.stdout
def source_admission(repo,executable,s):
 c.validate(s,c.SOURCE);c.need(git(repo,'rev-parse','HEAD^{tree}').decode().strip()==s['source_tree'],'source_binding')
 c.need(not git(repo,'status','--porcelain','--untracked-files=all').strip(),'source_binding');manifest={}
 for row in git(repo,'ls-tree','-rz','--full-tree','HEAD').split(b'\0'):
  if row:
   fields,path=row.split(b'\t',1);manifest[path.decode()]=fields.decode().split()
 c.need(c.sha(c.canonical(manifest))==s['source_closure_sha256'],'source_binding')
 c.need(c.sha(bounded(executable,512*1024**2))==s['executable_sha256'],'executable')
def check_binding(binding,permit,plan,common,world,proof_raw):
 c.need(type(binding)is dict and set(binding)=={'old_world','new_world'},'binding')
 for s in binding.values():c.validate(s,c.SOURCE)
 c.need(c.sha(plan)==PLAN_SHA and all(s['plan_sha256']==PLAN_SHA for s in binding.values()),'plan_binding')
 c.need(c.sha(world)==WORLD_SHA and all(s['common_patch_sha256']==c.sha(common) and s['world_patch_sha256']==WORLD_SHA for s in binding.values()),'patch_binding')
 c.need(not any(t in common+world for t in (b'GIT binary patch',b'Binary files')),'patch_binding')
 keys={'approved','binding_sha256','reporter_sha256','freshness_rechecked','native_qualification_passed','independent_source_review_passed','no_prior_dispatch','job_root_sha256','source_proof_sha256'}
 c.need(type(permit)is dict and set(permit)==keys,'permit')
 c.need(all(permit[k]is True for k in keys-{'binding_sha256','reporter_sha256','job_root_sha256','source_proof_sha256'}),'permit')
 for key in ('binding_sha256','reporter_sha256','job_root_sha256','source_proof_sha256'):c.validate(permit[key],c.SHA)
 c.need(c.sha(proof_raw)==permit['source_proof_sha256'],'source_proof')
 try:p=json.loads(proof_raw)
 except Exception:raise c.Reject('source_proof') from None
 c.need(p.get('base_commit')=='20bcb45104e5bec688b01abefa96cdf95bdcee60' and p.get('world_head_commit')==WORLD_HEAD
  and p.get('patch_commutation_exact')is True and p.get('candidate_constructor_body_exact')is True
  and p.get('candidate_constructor_body_sha256')=='04ccb232110305dfb4e116c652db01ebc7bb3dbbc63c1c1e71558f3936cd5a2e','source_proof')
 c.need(p.get('activation_main_exact')is True and p.get('activation_main_sha256')==ACTIVATION_MAIN_SHA and p.get('preparation_guard_exact')is True and p.get('preparation_guard_sha256')==PREPARATION_GUARD_SHA,'activation_source')
 c.need(p.get('world_patch_sha256')==WORLD_SHA and p.get('common_patch_sha256')==c.sha(common) and p.get('plan_sha256')==PLAN_SHA,'source_proof')
 for variant,prefix in (('old_world','old'),('new_world','new')):
  c.need(binding[variant]['source_tree']==p.get(prefix+'_reconstructed_tree_oid_unwritten') and binding[variant]['source_closure_sha256']==p.get(prefix+'_tree_manifest_sha256'),'source_proof')
 c.need(permit['binding_sha256']==c.sha(c.canonical(binding)),'permit')
 reporter={n:c.sha(bounded(Path(__file__).with_name(n),1024**2)) for n in ('report_contract.py','bounded_runner.py','diagnostic_contract.py','scalar_prefix_contract.py','prefix_runner_seam.py')}
 c.need(permit['reporter_sha256']==c.sha(c.canonical(reporter)),'permit')
class ByteBudget:
 """Exact retained/accepted byte ceiling. One discarded sentinel detects overflow."""
 def __init__(self,limit):self.limit=limit;self.used=0
 def read_size(self):return max(1,min(65536,self.limit-self.used))
 def accept(self,chunk):c.need(len(chunk)<=self.limit-self.used,'stream_bound');self.used+=len(chunk)
class Ledger:
 def __init__(self):
  self.dispatched=set();self.native_started=set();self.construction_started=set();self.attempted=set();self.prepared={};self.start_requested=set();self.started=set();self.returned=set();self.validated={};self.rejected=set();self.completions={};self.prefixes={};self.prefix_progress={};self.diagnostics={};self.marker_errors={};self.rejection_notified=set();self.finished_summary_receipts={}
 def dispatch(self,cid):c.need(cid in c.CASES and cid not in self.dispatched,'no_retry');self.dispatched.add(cid)
 def native_process_started(self,cid):
  c.need(cid in self.dispatched and cid not in self.native_started,'accounting');self.native_started.add(cid);self.attempted.add(cid)
 def prepared_case(self,cid,value,binding):
  c.validate(value,c.obj(case_id=c.enum(*c.CASES),initial=c.PAIR,source=c.SOURCE));c.need(cid in self.dispatched and cid not in self.prepared and value['case_id']==cid,'accounting')
  seed,world=c.CASES[cid];c.need(value['source']==binding[world] and value['initial']['asset_sha256']==c.ASSET and value['initial']['sampling_seed']==seed,'initial_binding')
  self.attempted.add(cid);self.construction_started.add(cid);self.prepared[cid]=value
 def pair_ready(self,a,b):c.need(a in self.prepared and b in self.prepared,'pair_not_prepared');c.need(self.prepared[a]['initial']==self.prepared[b]['initial'],'initial_pair_mismatch')
 def start(self,cid,peer):
  self.pair_ready(cid,peer);c.need(cid not in self.start_requested and not(self.start_requested-set(self.validated)),'one_primary_at_a_time');self.start_requested.add(cid)
 def note_returned(self,cid):c.need(cid in self.start_requested and cid not in self.returned,'accounting');self.returned.add(cid)
 def accept(self,cid,raw,completion,binding):
  c.need(cid in self.returned and cid in self.started and cid not in self.validated,'accounting')
  try:
   receipt=c.parse(completion,c.COMPLETION);c.need(receipt['case_id']==cid and receipt['original_summary_sha256']==c.sha(raw) and receipt['original_summary_byte_count']==len(raw),'completion_hash');self.finished_summary_receipts[cid]=receipt
   x=c.validate_case(raw,completion,binding);c.need(x['case_id']==cid and x['initial']==self.prepared[cid]['initial'],'initial_binding')
  except c.Reject:self.rejected.add(cid);raise
  self.validated[cid]=x;self.completions[cid]=c.parse(completion,c.COMPLETION)
 def sets(self):
  c.need(set(self.validated)<=self.returned<=self.dispatched and not(set(self.validated)&self.rejected),'accounting')
  known=self.dispatched|self.attempted;ordered=lambda values:[cid for cid in c.ORDER if cid in values]
  return dict(dispatched=ordered(self.dispatched),native_process_started=ordered(self.native_started),construction_started=ordered(self.construction_started),attempted=ordered(self.attempted),prepared=ordered(self.prepared),start_requested=ordered(self.start_requested),
   primary_started=ordered(self.started),native_returned=ordered(self.returned),native_rejection_returned=ordered(self.rejection_notified),native_summary_bytes_verified=ordered(self.finished_summary_receipts),validated=ordered(self.validated),rejected=ordered(self.rejected),
   unknown=ordered(known-set(self.validated)-self.rejected),unstarted=ordered(set(c.CASES)-known))
def resources(root,deadline):
 c.need(time.monotonic()<deadline,'deadline');c.need(shutil.disk_usage(root).free>=FLOOR,'resource');total=0
 for p in root.rglob('*'):
  c.need(not p.is_symlink(),'resource')
  if p.is_file():total+=p.stat().st_size
 c.need(total<=PRIVATE_CAP,'resource')
def marker(case,cid,kind):
 schemas={
  'attempted':c.obj(case_id=c.enum(*c.CASES),seed=c.enum(*sorted({v[0] for v in c.CASES.values()})),world=c.enum('old_world','new_world'),stage=c.enum('before_construction')),
  'started':c.obj(case_id=c.enum(*c.CASES),stage=c.enum('before_first_primary_tick'))}
 x=c.parse(bounded(case/(kind+'.json'),4096),schemas[kind]);c.need(x['case_id']==cid,'attempt_marker')
 if kind=='attempted':c.need((x['seed'],x['world'])==c.CASES[cid],'attempt_marker')

def finalize(root,ledger,binding,gate,common,world,exits,stderr_bytes):
 sets=ledger.sets();cases=[ledger.validated[cid] for cid in c.ORDER if cid in ledger.validated]
 summaries=[dict(case_id=x['case_id'],stop=x['stop'],metrics=c.metrics(x)) for x in cases];pairs=[]
 for i in (1,2):
  a,b=f's{i}_old_world',f's{i}_new_world'
  if a in ledger.validated and b in ledger.validated:
   ma,mb=c.metrics(ledger.validated[a]),c.metrics(ledger.validated[b]);pairs.append(dict(seed=c.CASES[a][0],safe_meal_delta_new_minus_old=mb['safe_meals']-ma['safe_meals'],harmful_meal_delta_new_minus_old=mb['harmful_meals']-ma['harmful_meals'],old_h_held_s_eat_opportunity_observed=ma['h_held_s_eat_opportunity_observed'],new_h_held_s_eat_opportunity_observed=mb['h_held_s_eat_opportunity_observed']))
 exposure={cid:dict(preparation_minimum_ticks=600 if cid in ledger.prepared else 0,
  preparation_maximum_ticks=600 if cid in ledger.dispatched else 0,
  preparation_exact=cid in ledger.prepared or cid not in ledger.dispatched,
  primary_minimum_ticks=len(ledger.validated[cid]['rows']) if cid in ledger.validated else 0,
  primary_maximum_ticks=len(ledger.validated[cid]['rows']) if cid in ledger.validated else 256 if cid in ledger.start_requested or cid in ledger.started else 0,
  primary_exact=cid in ledger.validated or (cid not in ledger.start_requested and cid not in ledger.started)) for cid in c.ORDER}
 for cid,prefix in ledger.prefixes.items():
  progress=ledger.prefix_progress[cid];exposure[cid]['primary_minimum_ticks']=progress['completed_host_steps_minimum'];exposure[cid]['primary_maximum_ticks']=progress['completed_host_steps_maximum'];exposure[cid]['primary_exact']=progress['completed_host_steps_minimum']==progress['completed_host_steps_maximum']
 for cid,value in ledger.diagnostics.items():
  p=value['progress'];exposure[cid]['preparation_minimum_ticks']=p['preparation_returned'];exposure[cid]['preparation_maximum_ticks']=p['preparation_requested'];exposure[cid]['preparation_exact']=p['preparation_returned']==p['preparation_requested']
  if cid not in ledger.prefixes and cid not in ledger.validated:
   exposure[cid]['primary_minimum_ticks']=p['host_returned'];exposure[cid]['primary_maximum_ticks']=p['host_requested'];exposure[cid]['primary_exact']=p['host_returned']==p['host_requested']
 report=['# World-only hand-swap comparison','','Native CPU development evidence only. Two paired seeds are not generalization.',
  'One unchanged v12 foundation, ordinary sleep only. Atomic replacement and item grounding are one bundled world factor.',
  'Physical meals, exact Ingest episode coverage and exact-key recall are separate. Historical forced-sleep runs are not causal controls.','']
 for s in summaries:
  m=s['metrics'];report.append(f"- {s['case_id']}: {m['primary_ticks']} ticks, H/S meals {m['harmful_meals']}/{m['safe_meals']}, stop {s['stop']}.")
 for cid in c.ORDER:
  if cid in ledger.prefixes:
   prefix=ledger.prefixes[cid]['summary'];report.append(f"- {cid}: INCOMPLETE strictly validated prefix of {prefix['admitted_rows']} rows; lower-bound evidence only, not a completed case or comparison.")
 report+=['',f"Validated {len(sets['validated'])}; rejected {len(sets['rejected'])}; unknown {len(sets['unknown'])}; unstarted {len(sets['unstarted'])}.",
  'If no natural held-H/selected-S opportunity occurs, the proposed obstruction remains untested. An opportunity or generic failure does not establish hold-caused obstruction. No contact is not proof of avoidance. A swap is not proof of learned aversion.']
 outputs={'report.md':('\n'.join(report)+'\n').encode(),'summary.json':c.canonical(dict(cases=summaries,complete_pairs=pairs)),'rows.json':c.canonical(dict(cases=cases)),
  'checks.json':c.canonical(dict(case_sets=sets,exposure_bounds=exposure,terminal_gate=gate,original_byte_completions=ledger.completions,finished_summary_byte_receipts=ledger.finished_summary_receipts,finite_native_rejections=ledger.diagnostics,scalar_marker_errors=ledger.marker_errors,prefix_progress=ledger.prefix_progress,process_exits=exits,discarded_stderr_bytes=stderr_bytes,
    attempted_definition='Native process creation succeeded, or construction marker observed; conservative even if preparation fails',construction_started_definition='Native create-new marker observed before frozen-asset construction; not proof of completed preparation',native_returned_definition='CASE_READY notification observed; not validation or process exit',primary_started_definition='native create-new started marker observed before first primary advance',raw_stderr_persisted=False)),
  'source-bindings.json':c.canonical(binding),'common-audit-kernel.patch':common,'world-factor.patch':world}
 outputs=prefix_runner_seam.add_prefixes_and_bound(outputs,ledger.prefixes,public_cap=PUBLIC_CAP)
 c.need(set(outputs)==PUBLIC_FILES and sum(map(len,outputs.values()))<=PUBLIC_CAP,'public_bound')
 public=root/'public';public.mkdir(exist_ok=False)
 for name,raw in outputs.items():create(public/name,raw)
 return sets

def restrict_process_outputs():
 # Applied before any study marker/directory/process; inherited by every child.
 resource.setrlimit(resource.RLIMIT_CORE,(0,0));os.umask(0o077)
def run_reviewed(repos,executables,binding,permit,plan_path,common_path,world_path,proof_path,root):
 """Uncalled integration seam. Requires a separately reviewed exact activation receipt."""
 root=Path(root);plan=bounded(plan_path,1024**2);common=bounded(common_path,1024**2);world=bounded(world_path,1024**2)
 check_binding(binding,permit,plan,common,world,bounded(proof_path,1024**2))
 for v in ('old_world','new_world'):source_admission(repos[v],executables[v],binding[v])
 c.need(permit['job_root_sha256']==c.sha(str(root.resolve()).encode()),'job_root_binding');c.need(not root.exists() and shutil.disk_usage(root.parent).free>=FLOOR,'fresh_root')
 restrict_process_outputs()
 create(root.parent/'world-swap-2026112301-2026112302.dispatch.json',c.canonical(dict(job_root_sha256=permit['job_root_sha256'],binding_sha256=permit['binding_sha256'])))
 root.mkdir();deadline=time.monotonic()+SECONDS
 create(root/'binding.json',c.canonical(dict(protocol='world_only_swap_v1',deadline_unix_seconds=int(time.time())+SECONDS,**binding)))
 create(root/'activation-receipt.json',c.canonical(permit));ledger=Ledger();processes={};buffers={};stdout_budgets={};stderr_budget=ByteBudget(STDERR_CAP);exits={};gate='complete';selector=selectors.DefaultSelector();quiescent=set()
 def send(cid,kind):p=processes[cid];p.stdin.write(f'{kind} {cid}\n'.encode());p.stdin.flush()
 def poll_until(predicate):
  while not predicate():
   resources(root,deadline)
   for key,_ in selector.select(.1):
    cid,stream=key.data;budget=stderr_budget if stream=='stderr' else stdout_budgets[cid]
    chunk=os.read(key.fileobj.fileno(),budget.read_size())
    if not chunk:
     if stream=='stdout':c.need(not buffers[cid],'stdout_contract')
     selector.unregister(key.fileobj);continue
    budget.accept(chunk)
    if stream=='stderr':continue  # Discard raw bytes. No stderr file exists.
    buffers[cid]+=chunk;c.need(len(buffers[cid])<=256,'stdout_contract')
    while b'\n' in buffers[cid]:
     line,buffers[cid]=buffers[cid].split(b'\n',1)
     try:parts=line.decode('ascii').split(' ')
     except Exception:raise c.Reject('stdout_contract') from None
     c.need(len(parts)==2 and parts[1]==cid,'stdout_contract');kind=parts[0];case=root/cid
     if kind=='PREPARED':
      marker(case,cid,'attempted');value=c.parse(bounded(case/'prepared.json'),c.obj(case_id=c.enum(*c.CASES),initial=c.PAIR,source=c.SOURCE));ledger.prepared_case(cid,value,binding)
     elif kind=='CASE_READY':
      ledger.note_returned(cid)
      try:
       marker(case,cid,'attempted');marker(case,cid,'started');ledger.started.add(cid)
       ledger.accept(cid,bounded(case/'summary.json'),bounded(case/'completion.json'),binding)
      except c.Reject:ledger.rejected.add(cid);raise
      create(case/'validated.json',c.canonical(dict(case_id=cid,original_summary_sha256=ledger.completions[cid]['original_summary_sha256'])));send(cid,'VALIDATED')
     elif kind=='REJECTED':c.need(cid not in ledger.validated,'accounting');ledger.rejection_notified.add(cid);ledger.rejected.add(cid);raise c.Reject('native_rejected')
     else:raise c.Reject('stdout_contract')
   for cid,p in processes.items():
    code=p.poll()
    if code is not None:
     exits[cid]='zero' if code==0 else 'signal' if code<0 else 'nonzero'
     if code!=0:raise c.Reject('native_exit')
     if cid not in ledger.validated and not any(k.data[0]==cid and k.data[1]=='stdout' for k in selector.get_map().values()):raise c.Reject('missing_return')
 try:
  for pair in (c.ORDER[:2],c.ORDER[2:]):
   for cid in pair:
    v=c.CASES[cid][1];case=root/cid;case.mkdir();ledger.dispatch(cid);create(root/(cid+'.dispatched.json'),c.canonical(dict(case_id=cid)))
    p=subprocess.Popen([str(executables[v]),'run-reviewed',str(case),str(root/'binding.json'),str(c.CASES[cid][0]-2026112301),v],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
    ledger.native_process_started(cid);processes[cid]=p;buffers[cid]=b'';stdout_budgets[cid]=ByteBudget(STDOUT_CAP);selector.register(p.stdout,selectors.EVENT_READ,(cid,'stdout'));selector.register(p.stderr,selectors.EVENT_READ,(cid,'stderr'))
   poll_until(lambda:all(cid in ledger.prepared for cid in pair));ledger.pair_ready(*pair)
   for cid,peer in (pair,pair[::-1]):
    ledger.start(cid,peer);send(cid,'START');poll_until(lambda:cid in ledger.validated and processes[cid].poll()==0 and not any(k.data[0]==cid for k in selector.get_map().values()))
 except c.Reject as e:gate=e.gate
 except Exception:gate='runtime_unknown'
 finally:
  for cid,p in processes.items():
   # Kill the complete process group even after its leader exited.
   try:os.killpg(p.pid,signal.SIGKILL)
   except ProcessLookupError:pass
   try:p.wait(timeout=5)
   except subprocess.TimeoutExpired:gate='kill_unknown'
   if p.returncode is not None:quiescent.add(cid)
   exits[cid]='zero' if p.returncode==0 else 'signal' if p.returncode is not None and p.returncode<0 else 'nonzero' if p.returncode is not None else 'unknown'
   for stream in (p.stdin,p.stdout,p.stderr):
    if stream is not None:stream.close()
  selector.close()
  for cid in ledger.dispatched:
   if cid not in quiescent:
    ledger.marker_errors[cid]='termination_unconfirmed';continue
   case=root/cid
   for kind in ('attempted','started'):
    if not (case/(kind+'.json')).exists():continue
    try:
     marker(case,cid,kind)
     if kind=='attempted':ledger.attempted.add(cid);ledger.construction_started.add(cid)
     else:c.need(cid in ledger.start_requested,'unexpected_started_marker');ledger.started.add(cid)
    except (c.Reject,OSError):ledger.marker_errors[cid]=kind+'_marker_invalid'
   if cid not in ledger.prepared and (case/'prepared.json').exists():
    try:
     value=c.parse(bounded(case/'prepared.json'),c.obj(case_id=c.enum(*c.CASES),initial=c.PAIR,source=c.SOURCE));ledger.prepared_case(cid,value,binding)
    except (c.Reject,OSError):ledger.marker_errors[cid]='prepared_marker_invalid'
   rejection_raw=None
   if (case/'rejection.json').exists() and cid not in ledger.validated:
    ledger.rejected.add(cid)
    try:
     rejection_raw=bounded(case/'rejection.json',16384);value=diagnostic.parse_rejection(rejection_raw,cid);p=value['progress']
     c.need(p['host_requested']<=p['collected_rows']+1,'diagnostic_counts')
     if cid in ledger.prepared:c.need(p['preparation_returned']==p['preparation_requested']==600,'diagnostic_counts')
     if p['host_requested']:c.need(cid in ledger.started and cid in ledger.prepared,'diagnostic_counts')
     ledger.diagnostics[cid]=value
    except (c.Reject,OSError):ledger.marker_errors[cid]='rejection_metadata_invalid';rejection_raw=None
   path=case/'scalar-prefix.jsonl'
   if cid not in ledger.validated and cid in ledger.prepared and cid in ledger.started and path.exists():
    try:
     c.need(path.is_file() and not path.is_symlink(),'prefix_file')
     with path.open('rb') as stream:
      admitted=prefix_contract.admit_stream(stream,cid,ledger.prepared[cid]['initial'],binding,start_verified=cid in ledger.start_requested)
     ledger.prefixes[cid]=admitted;ledger.prefix_progress[cid]=prefix_contract.assess_progress(rejection_raw,admitted)
     if rejection_raw is not None and ledger.prefix_progress[cid]['status']=='unavailable_or_invalid':
      ledger.diagnostics.pop(cid,None);ledger.marker_errors[cid]='rejection_metadata_inconsistent_with_prefix'
    except (c.Reject,OSError):ledger.marker_errors[cid]='prefix_header_or_file_invalid'
 return finalize(root,ledger,binding,gate,common,world,exits,stderr_budget.used)
if __name__=='__main__':raise SystemExit('disabled reconstructed source-only runner: no dispatch route')
