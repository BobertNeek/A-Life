#!/usr/bin/env python3
"""Source-only sealing. Does not publish or invoke a compiler/test/native process."""
from pathlib import Path
import ast,gzip,hashlib,io,json,tarfile,shutil
R=Path(__file__).resolve().parent;PLAN=R.name;DEST='docs/experiments/'+PLAN
sha=lambda b:hashlib.sha256(b).hexdigest()
def save(n,j):(R/n).write_text(json.dumps(j,indent=2,sort_keys=True)+'\n')
def rec(name,data):return {'path':name,'bytes':len(data),'sha256':sha(data)}
prior=[]
for name in ['m2-play-runtime-integration-v1','m2-grip-hosted-typecheck-v5','m2-play-hosted-qualification-v1','m2-play-hosted-qualification-v2','m2-play-hosted-qualification-v3']:
 root=Path('/workspace/shared')/name
 assert root.is_dir() and any(root.iterdir())
 prior.append({'directory':name,'files':[rec(p.relative_to(root).as_posix(),p.read_bytes()) for p in sorted(root.rglob('*')) if p.is_file()]})
save('PRIOR-PRESERVATION.json',{'schema':'source-only-no-prior-mutation-v1','directories':prior})
assert shutil.disk_usage('/tmp').free>4*1024**3
reuse=json.loads((R/'REUSE-MANIFEST.json').read_text())
for r in reuse['files']:
 b=Path(r['source_artifact']).read_bytes();assert rec(r['path'],b)=={k:r[k] for k in ('path','bytes','sha256')}
source=sha((R/'source-manifest.json').read_bytes())
s=R.joinpath('run_hosted.py').read_text();tree=ast.parse(s)
freeze={'schema':'m2-play-runner-functions-v1','runner_sha256':sha(s.encode()),'functions':[{ 'name':n.name,'sha256':sha(ast.get_source_segment(s,n).encode())} for n in tree.body if isinstance(n,(ast.FunctionDef,ast.ClassDef))], 'commands':{
'build':['rustup','run','1.99.1','cargo','build','--locked','-j1','-p','alife_training','--features','cpu-backend','--lib','--message-format=json','--example','probe_play_need_default_wire_v1'],
'libtests_build':['rustup','run','1.99.1','cargo','test','--locked','-j1','-p','alife_training','--features','cpu-backend','--lib','--message-format=json','--no-run'],
'exact_list':['<sealed-owned-libtest>','--exact','<one-frozen-name>','--list'],'exact_run':['<sealed-owned-libtest>','--exact','<one-frozen-name>','--test-threads=1'],'probes':['<sealed-role-example>']},'resources_ast':next(ast.get_source_segment(s,n) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='LIMITS' for t in n.targets)), 'test_inventory_sha256':sha((R/'TEST-INVENTORY.json').read_bytes()),'dose_inventory_sha256':sha((R/'DOSE-INVENTORY.json').read_bytes())}
helper=(R/'reconstruct_source.py').read_text();freeze['reconstruction_helper_sha256']=sha(helper.encode());freeze['reconstruction_functions']=[{'name':n.name,'sha256':sha(ast.get_source_segment(helper,n).encode())} for n in ast.parse(helper).body if isinstance(n,ast.FunctionDef)];save('FUNCTION-FREEZE.json',freeze)
save('OUTPUT-POLICY.json',{'schema':'m2-play-public-projection-v1','artifact_upload_allowlist':[],'runner_stdout_allowlist':['one strict JSON summary <=32768bytes'],'never_publish':['compiler rendered text','compiler children','raw stderr/stdout','Rust panic or assertion strings','binaries','checkpoints','journals','private logs','target files','cache directories'],'raw_pipe_combined_bytes':33554432,'captured_stdout_bytes':2097152,'line_bytes':4194304,'unknown_native_protocol':'fixed code only, stop, no retry','actual_dose_claim':'planned bounds are never promoted to observed counts'})
save('publication.json',{'schema':'m2-play-publication-v1','status':'SOURCE_ONLY_NOT_PUBLISHED','repository':'BobertNeek/A-Life','branch':'qualification/'+PLAN,'prepared_parent':'c3aec16df33861f1316d127736f5f60eea327ede','research_base':'7f905465b79291fd52d1fd65568a9d10e716576a','source_checkout':'c3aec16df33861f1316d127736f5f60eea327ede','logical_ids_are_not_fetch_refs':True,'prepared_head':'PENDING_VERIFIED_IMMUTABLE_GITHUB_COMMIT','workflow_destination':'.github/workflows/'+PLAN+'.yml','packet_destination':DEST,'inherited_workflows_modified':0,'inherited_workflows_deleted':0,'branch_birth':'Create new branch directly at a verified additions-only prepared child of exact accepted main, marker absent; branch existence and main freshness must be checked before any write. Resolve uncertain outcomes before retry.','activation':'Separate authorization, sole ACTIVATE.json addition, direct child of exact verified remote prepared head. Attempt1 only; no dispatch, PR, merge, main update, branch reuse or retries.','reuse':'REUSE-MANIFEST distinguishes parent-verified immutable Grip tree blobs, parent-reported verified unreferenced V1 blobs, and new local bytes. Verify every exact destination before preparing a tree.','remote_actions_performed':[],'remote_branch_existence':'UNVERIFIED','archive_mode':'update-only, combine archive destinations and every reuse entry; preserve all old main files exactly','public_outputs':'one strict JSON runner summary; no uploaded artifacts'})
assert json.loads((R/'STATIC-CHECK.json').read_text())['passed']
excluded={'seal.json','workflow.yml','ACTIVATE.template.json','ARCHIVE-MANIFEST.json','source-only-m2-play-hosted-qualification-v4-update.tar.gz'}
files=[rec(p.name,p.read_bytes()) for p in sorted(R.iterdir()) if p.is_file() and p.name not in excluded]
assert not {x['path'] for x in files}&{x['path'] for x in reuse['files']}
files+=[{k:x[k] for k in ('path','bytes','sha256')} for x in reuse['files']]
save('seal.json',{'schema':'m2-play-hosted-seal-v1','plan_id':PLAN,'source_ready':True,'files':sorted(files,key=lambda x:x['path'])})
seal=sha((R/'seal.json').read_bytes())
save('ACTIVATE.template.json',{'schema':'m2-play-qualification-activation-v1','plan_id':PLAN,'main':'c3aec16df33861f1316d127736f5f60eea327ede','research_base':'7f905465b79291fd52d1fd65568a9d10e716576a','prepared_head':'PENDING_VERIFIED_IMMUTABLE_GITHUB_COMMIT','seal_sha256':seal,'source_manifest_sha256':source,'attempt':1,'fixed_disposable_qualification_only':True})
workflow=f'''name: M2 Play fixed hosted qualification v4
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
      - name: Admit and run frozen disposable qualification once
        shell: bash
        run: python3 bundle/{DEST}/run_hosted.py
'''
(R/'workflow.yml').write_text(workflow)
archive=R/'source-only-m2-play-hosted-qualification-v4-update.tar.gz';members=[]
for p in sorted(R.iterdir()):
 if p.is_file() and p.name not in ['ARCHIVE-MANIFEST.json',archive.name]:
  b=p.read_bytes();dest=('.github/workflows/'+PLAN+'.yml') if p.name=='workflow.yml' else DEST+'/'+p.name
  # Include workflow mirror inside packet as required by admission.
  members.append((dest,b));
  if p.name=='workflow.yml':members.append((DEST+'/workflow.yml',b))
with open(archive,'wb') as raw:
 with gzip.GzipFile(fileobj=raw,mode='wb',mtime=0,filename='') as gz:
  with tarfile.open(fileobj=gz,mode='w') as tar:
   for path,b in members:
    i=tarfile.TarInfo(path);i.mode=0o644;i.size=len(b);i.mtime=0;tar.addfile(i,io.BytesIO(b))
save('ARCHIVE-MANIFEST.json',{'schema':'m2-play-update-archive-v1','archive':archive.name,'archive_sha256':sha(archive.read_bytes()),'archive_bytes':archive.stat().st_size,'members':[rec(p,b) for p,b in members],'reuse_manifest_sha256':sha((R/'REUSE-MANIFEST.json').read_bytes()),'seal_sha256':seal,'source_manifest_sha256':source,'local_stage_limit_bytes':3145728,'no_remote_actions':True})
assert sum(p.stat().st_size for p in R.rglob('*') if p.is_file())<=3145728
print(json.dumps({'seal_sha256':seal,'source_manifest_sha256':source,'archive_sha256':sha(archive.read_bytes()),'archive_bytes':archive.stat().st_size,'stage_bytes':sum(p.stat().st_size for p in R.rglob('*') if p.is_file())}))
