#!/usr/bin/env python3
"""Seal source-only additions. Never invokes a compiler or native workload."""
import ast,fnmatch,gzip,hashlib,io,json,pathlib,shutil,tarfile
R=pathlib.Path(__file__).resolve().parent;PLAN=R.name;DEST='docs/experiments/'+PLAN
sha=lambda b:hashlib.sha256(b).hexdigest()
def save(n,x):(R/n).write_text(json.dumps(x,indent=2,sort_keys=True)+'\n')
def rec(n,b):return {'path':n,'bytes':len(b),'sha256':sha(b)}
assert shutil.disk_usage('/tmp').free>4*1024**3
reuse=json.loads((R/'REUSE-MANIFEST.json').read_bytes())['files']
for row in reuse:
 b=pathlib.Path(row['source_artifact']).read_bytes();assert rec(row['path'],b)=={k:row[k] for k in ('path','bytes','sha256')};assert row['destination_path']==DEST+'/'+row['path']
s=(R/'run_hosted.py').read_text();h=(R/'reconstruct_source.py').read_text()
save('FUNCTION-FREEZE.json',{'schema':'m2-play-remaining-runner-functions-v7','runner_sha256':sha(s.encode()),'functions':[{'name':n.name,'sha256':sha(ast.get_source_segment(s,n).encode())} for n in ast.parse(s).body if isinstance(n,(ast.FunctionDef,ast.ClassDef))],'resources_ast':next(ast.get_source_segment(s,n) for n in ast.parse(s).body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='LIMITS' for t in n.targets)),'reconstruction_helper_sha256':sha(h.encode()),'reconstruction_functions':[{'name':n.name,'sha256':sha(ast.get_source_segment(h,n).encode())} for n in ast.parse(h).body if isinstance(n,ast.FunctionDef)],'test_inventory_sha256':sha((R/'TEST-INVENTORY.json').read_bytes()),'dose_inventory_sha256':sha((R/'DOSE-INVENTORY.json').read_bytes()),'scope':'Fixed remaining12 tests, two default-wire probes, no prior-test replay.'})
manifest=json.loads((R/'source-manifest.json').read_bytes());inventory=json.loads((R/'TEST-INVENTORY.json').read_bytes())
save('SOURCE-FREEZE.json',{'schema':'m2-play-v7-source-freeze-v1','raw_source_manifest_sha256':sha((R/'source-manifest.json').read_bytes()),'role_counts':{role:len(manifest[role+'_files']) for role in ['base','original60','owned76']},'scientific_source_changes_from_v6':0,'fixture_fragments_unchanged_from_v5':[{k:t[k] for k in ['id','source','source_sha256','function_fragment_sha256']} for t in inventory['tests']],'probe_source_sha256':sha((R/'probe_play_need_default_wire_v1.rs').read_bytes()),'effective_owned_patch_sha256':json.loads((R/'OWNED-PATCH-TRANSPORT.json').read_bytes())['effective']['sha256'],'runtime_readout_payloads_copied':0})
# Read-only trigger audit of all inherited workflow bytes.
import yaml
rows=json.loads((R/'INHERITED-WORKFLOW-FIXTURES.json').read_bytes())['files'];pres={x['path']:x for x in json.loads((R/'WORKFLOW-PRESERVATION.json').read_bytes())['files']};aud=[]
assert len(rows)==15
for x in rows:
 b=x['content_utf8'].encode();r=pres[x['path']];assert len(b)==r['bytes'] and sha(b)==r['sha256']
 y=yaml.load(b,Loader=yaml.BaseLoader);events=y.get('on',{});push=events.get('push') if isinstance(events,dict) else None
 eligible=False;branches=[]
 if push is not None:
  assert isinstance(push,dict);branches=push.get('branches',[]);assert branches and 'branches-ignore' not in push
  eligible=any(fnmatch.fnmatchcase('qualification/'+PLAN,p) for p in branches)
 assert not eligible
 aud.append({'path':x['path'],'sha256':sha(b),'events':list(events) if isinstance(events,(dict,list)) else [events],'push_branches':branches,'qualification_push_matches':eligible})
save('TRIGGER-AUDIT.json',{'schema':'m2-play-trigger-audit-v7','accepted_main':'c3aec16df33861f1316d127736f5f60eea327ede','inherited_count':15,'inherited_eligible_on_prepare':0,'inherited_eligible_on_activate':0,'inherited_files':aud,'own_activation':'Only exact V7 branch, sole marker path, event-created/deleted/forced false, attempt1.'})
assert json.loads((R/'STATIC-CHECK.json').read_bytes())['passed']
archive=R/('source-only-'+PLAN+'-update.tar.gz');excluded={'seal.json','workflow.yml','ACTIVATE.template.json','ARCHIVE-MANIFEST.json',archive.name}
files=[rec(p.name,p.read_bytes()) for p in sorted(R.iterdir()) if p.is_file() and p.name not in excluded]
assert not {x['path'] for x in files}&{x['path'] for x in reuse}
files += [{k:x[k] for k in ('path','bytes','sha256')} for x in reuse]
save('seal.json',{'schema':'m2-play-hosted-seal-v1','plan_id':PLAN,'source_ready':True,'files':sorted(files,key=lambda x:x['path'])})
seal=sha((R/'seal.json').read_bytes());source=sha((R/'source-manifest.json').read_bytes())
save('ACTIVATE.template.json',{'schema':'m2-play-qualification-activation-v1','plan_id':PLAN,'main':'c3aec16df33861f1316d127736f5f60eea327ede','research_base':'7f905465b79291fd52d1fd65568a9d10e716576a','prepared_head':'PENDING_VERIFIED_IMMUTABLE_GITHUB_COMMIT','seal_sha256':seal,'source_manifest_sha256':source,'attempt':1,'fixed_disposable_qualification_only':True})
workflow=f'''name: M2 Play fixed remaining qualification v7
on:
  push:
    branches: [qualification/{PLAN}]
    paths: [{DEST}/ACTIVATE.json]
permissions:
  contents: read
concurrency:
  group: {PLAN}
  cancel-in-progress: false
jobs:
  fixed-qualification:
    if: >-
      github.repository == 'BobertNeek/A-Life' &&
      github.ref == 'refs/heads/qualification/{PLAN}' &&
      github.event.created == false && github.event.deleted == false &&
      github.event.forced == false && github.run_attempt == '1'
    runs-on: ubuntu-24.04
    timeout-minutes: 90
    env:
      EXPECTED_SEAL_SHA256: {seal}
      PYTHONDONTWRITEBYTECODE: '1'
    steps:
      - name: Read exact activation and prepared parents
        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
        with:
          ref: ${{{{ github.sha }}}}
          path: bundle
          persist-credentials: false
          fetch-depth: 3
          lfs: false
          submodules: false
      - name: Read remotely reachable accepted main source
        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
        with:
          ref: c3aec16df33861f1316d127736f5f60eea327ede
          path: source
          persist-credentials: false
          fetch-depth: 1
          lfs: false
          submodules: false
      - name: Admit and run fixed remaining qualification once
        shell: bash
        run: python3 bundle/{DEST}/run_hosted.py
'''
(R/'workflow.yml').write_text(workflow)
members=[]
for p in sorted(R.iterdir()):
 if p.is_file() and p.name not in ['ARCHIVE-MANIFEST.json',archive.name]:
  b=p.read_bytes();dest='.github/workflows/'+PLAN+'.yml' if p.name=='workflow.yml' else DEST+'/'+p.name;members.append((dest,b))
  if p.name=='workflow.yml':members.append((DEST+'/workflow.yml',b))
with archive.open('wb') as stream:
 with gzip.GzipFile(fileobj=stream,mode='wb',mtime=0,filename='') as gz:
  with tarfile.open(fileobj=gz,mode='w') as t:
   for path,b in members:
    i=tarfile.TarInfo(path);i.mode=0o644;i.size=len(b);i.mtime=0;t.addfile(i,io.BytesIO(b))
save('ARCHIVE-MANIFEST.json',{'schema':'m2-play-update-archive-v2','archive':archive.name,'archive_bytes':archive.stat().st_size,'archive_sha256':sha(archive.read_bytes()),'members':[rec(n,b) for n,b in members],'seal_sha256':seal,'source_manifest_sha256':source,'local_stage_limit_bytes':3145728,'source_only':True,'runtime_readout_copies':0})
stage=sum(p.stat().st_size for p in R.rglob('*') if p.is_file());assert stage<=3145728
print(json.dumps({'seal_sha256':seal,'source_manifest_sha256':source,'archive_bytes':archive.stat().st_size,'stage_bytes':stage}))
