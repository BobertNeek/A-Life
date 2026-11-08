"""Source/scalar-only upper envelope; no native process, world, host or private state."""
import hashlib,io,json,tempfile,types,sys
from pathlib import Path
from unittest.mock import patch
P=Path(__file__).resolve().parent.parent/'world-swap-matched-23xx-active-v5'
sha=lambda b:hashlib.sha256(b).hexdigest()
manifest=json.loads((P/'PACKAGE-MANIFEST.json').read_bytes())
def load(name):
 raw=(P/(name+'.py')).read_bytes();assert sha(raw)==manifest[name+'.py']
 m=types.ModuleType(name);m.__file__=str(P/(name+'.py'));sys.modules[name]=m;exec(compile(raw,m.__file__,'exec'),m.__dict__);return m
c=load('report_contract');d=load('diagnostic_contract');pc=load('scalar_prefix_contract');seam=load('prefix_runner_seam');r=load('bounded_runner');t=load('test_contract')
common=(P/'common-audit-kernel.patch').read_bytes();world=(P/'world-factor.patch').read_bytes()
# Every metadata leaf is a finite source enum/hash/constant, bounded count, bool,
# null or finite JSON number. 1,024 ASCII bytes per leaf overbounds their scalar
# serialization (counts <=2**53; binary64 numbers <=24 bytes; hashes <=64).
# Preserve actual key bytes and container overhead. Report has <=4 case lines,
# <=4 prefix lines and fixed prose, covered separately by 8 KiB.
def envelope(x):
 if isinstance(x,dict):return {k:envelope(v) for k,v in x.items()}
 if isinstance(x,list):
  assert len(x)<=4
  return [envelope(v) for v in x]+[envelope(x[0]) if x else 'X'*1024 for _ in range(4-len(x))]
 assert len(json.dumps(x,allow_nan=False).encode())<=1024
 return 'X'*1024
max_by_name={};max_actual=0;scenario_count=0
with tempfile.TemporaryDirectory(prefix='swap-public-bound-data-only-') as tmp:
 for mask in range(16):
  ledger=r.Ledger();prefixes={}
  for i,cid in enumerate(c.ORDER):
   value=t.blank_case(cid);ledger.dispatch(cid);ledger.native_process_started(cid)
   ledger.prepared_case(cid,dict(case_id=cid,initial=value['initial'],source=value['source']),t.BINDING)
   ledger.start_requested.add(cid);ledger.started.add(cid);ledger.returned.add(cid);ledger.rejection_notified.add(cid)
   raw,receipt=t.encoded(value)
   if mask & (1<<i):ledger.accept(cid,raw,receipt,t.BINDING)
   else:
    header=dict(schema_version=1,scope='study_scalar_prefix',case_id=cid,seed=value['seed'],world=value['world'],initial=value['initial'],source=value['source'])
    admitted=pc.admit_stream(io.BytesIO(c.canonical(header)+b''.join(c.canonical(row) for row in value['rows'])),cid,value['initial'],t.BINDING,start_verified=True)
    ledger.prefixes[cid]=admitted;ledger.prefix_progress[cid]=pc.assess_progress(None,admitted)
    ledger.rejected.add(cid)
   ledger.marker_errors[cid]='rejection_metadata_inconsistent_with_prefix'
   ledger.finished_summary_receipts[cid]=c.parse(receipt,c.COMPLETION)
   ledger.diagnostics[cid]=dict(case_id=cid,gate='native_rejected',code='output_bound',progress=dict(stage='checkpoint_roundtrip',preparation_requested=600,preparation_returned=600,host_requested=256,host_returned=256,replay_requested=256,replay_returned=256,projected_steps=256,collected_rows=256))
  original=seam.add_prefixes_and_bound
  def force_fallback(outputs,prefixes,public_cap=None):
   # Artificial scalar padding ONLY exercises the fallback seam; it is not a
   # valid/admitted study row and never enters exported evidence or summaries.
   outputs=dict(outputs);rows=json.loads(outputs['rows.json']);rows['synthetic_budget_padding']='X'*(16*1024**2);outputs['rows.json']=c.canonical(rows)
   return original(outputs,prefixes,public_cap=public_cap)
  root=Path(tmp)/str(mask);root.mkdir()
  with patch.object(seam,'add_prefixes_and_bound',force_fallback):r.finalize(root,ledger,t.BINDING,'termination_unconfirmed',common,world,dict.fromkeys(c.ORDER,'nonzero'),4194304)
  outputs={f.name:f.read_bytes() for f in (root/'public').iterdir()}
  assert set(outputs)==r.PUBLIC_FILES and json.loads(outputs['checks.json'])['row_export']=='summary_only_due_public_byte_cap'
  assert not json.loads(outputs['rows.json'])['cases'] and not json.loads(outputs['rows.json'])['incomplete_prefixes']
  for name,raw in outputs.items():
   bound=len(c.canonical(envelope(json.loads(raw)))) if name.endswith('.json') else 8192 if name=='report.md' else len(raw)
   max_by_name[name]=max(max_by_name.get(name,0),bound)
  max_actual=max(max_actual,sum(map(len,outputs.values())));scenario_count+=1
# Up to four cases may independently be absent/unstarted, prefix or full.
# Every metadata array is padded to four elements in the envelope, including
# unknown/unstarted lists absent from these saturated examples. The16 masks
# cover each full/prefix assignment; summing per-file maxima is conservative.
bound=sum(max_by_name.values());assert bound<16*1024**2
result=dict(schema_version=1,scope='source_scalar_only_not_study',package_manifest_sha256=sha((P/'PACKAGE-MANIFEST.json').read_bytes()),script_sha256=sha(Path(__file__).read_bytes()),case_assignment_patterns=16,scalar_leaf_serialized_byte_envelope=1026,report_byte_envelope=8192,source_patch_bytes=len(common)+len(world),max_actual_summary_only_bytes=max_actual,conservative_mandatory_bytes=bound,per_file_conservative_bytes=max_by_name,public_cap_bytes=16*1024**2,all_case_summaries_receipts_and_prefix_counts_preserved=True,host_steps=0,preparation_ticks=0,replays=0)
print(json.dumps(result,sort_keys=True,separators=(',',':')))
