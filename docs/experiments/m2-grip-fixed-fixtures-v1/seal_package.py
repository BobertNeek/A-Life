#!/usr/bin/env python3
"""Freeze source-only local bytes; no GitHub action and no native execution."""
import sys
sys.dont_write_bytecode=True
import ast,gzip,hashlib,io,json,pathlib,shutil,tarfile
from packet_inputs import ROOT,V5,read,verify_preservation
PLAN='m2-grip-fixed-fixtures-v1';DEST='docs/experiments/'+PLAN
ARCHIVE='source-only-m2-grip-fixed-fixtures-v1-update.tar.gz'
EXCLUDED={'seal.json','workflow.yml','ACTIVATE.template.json','ARCHIVE-MANIFEST.json',ARCHIVE}
def sha(b):return hashlib.sha256(b).hexdigest()
def save(n,x):(ROOT/n).write_text(json.dumps(x,indent=2,sort_keys=True)+'\n')
def rec(p):
 b=p.read_bytes();b.decode('utf-8','strict');assert b'\0' not in b
 return {'path':p.relative_to(ROOT).as_posix(),'bytes':len(b),'sha256':sha(b)}
assert shutil.disk_usage(ROOT).free>=4*1024**3+1024**2
assert json.loads((ROOT/'STATIC-CHECK.json').read_bytes())['passed'] is True
assert json.loads((ROOT/'FIXTURES.json').read_bytes())['ready'] is True
assert not (ROOT/'ACTIVATE.json').exists()
verify_preservation()
reuse=json.loads((ROOT/'REUSE-MANIFEST.json').read_bytes())
local=[rec(p) for p in sorted(ROOT.rglob('*')) if p.is_file() and p.name not in EXCLUDED]
assert not any(p.is_symlink() or '__pycache__' in p.parts for p in ROOT.rglob('*'))
assert not {x['path'] for x in local}&{x['path'] for x in reuse['files']}
references=[]
for row in reuse['files']:
 b=read(row['path']);b.decode('utf-8','strict');assert b'\0' not in b
 assert len(b)==row['bytes'] and sha(b)==row['sha256']
 references.append({k:row[k] for k in ['path','bytes','sha256']})
save('seal.json',{'schema':'m2-grip-fixed-fixture-seal-v1','plan_id':PLAN,'source_ready':True,'files':sorted(local+references,key=lambda x:x['path'])})
seal=sha((ROOT/'seal.json').read_bytes());source=sha((ROOT/'source-manifest.json').read_bytes())
save('ACTIVATE.template.json',{'schema':'m2-grip-fixed-fixture-activation-v1','plan_id':PLAN,'main':'c3aec16df33861f1316d127736f5f60eea327ede','research_base':'fbf2a517f93098b5d936878a3d26bbb8d0391c38','prepared_head':'PENDING_VERIFIED_IMMUTABLE_GITHUB_COMMIT','seal_sha256':seal,'source_manifest_sha256':source,'attempt':1,'fixed_fixtures_only':True})
p=ROOT/'workflow.yml';text=p.read_text();import re
text,count=re.subn(r'EXPECTED_SEAL_SHA256: [A-Za-z0-9_]+','EXPECTED_SEAL_SHA256: '+seal,text);assert count==1;p.write_text(text)
rows=[]
with (ROOT/ARCHIVE).open('wb') as f:
 with gzip.GzipFile(filename='',fileobj=f,mode='wb',mtime=0) as z:
  with tarfile.open(mode='w',fileobj=z,format=tarfile.PAX_FORMAT) as tar:
   for p in sorted(ROOT.rglob('*')):
    if not p.is_file() or p.name in {ARCHIVE,'ARCHIVE-MANIFEST.json'}:continue
    row=rec(p);name='.github/workflows/'+PLAN+'.yml' if row['path']=='workflow.yml' else DEST+'/'+row['path']
    info=tarfile.TarInfo(name);info.size=row['bytes'];info.mode=0o644;info.mtime=0;info.uid=info.gid=0;info.uname=info.gname=''
    tar.addfile(info,io.BytesIO(p.read_bytes()));rows.append({'archive_path':name,**row})
raw=(ROOT/ARCHIVE).read_bytes()
save('ARCHIVE-MANIFEST.json',{'schema':'m2-grip-fixed-fixture-archive-v1','archive':ARCHIVE,'bytes':len(raw),'sha256':sha(raw),'member_count':len(rows),'members':rows,'seal_sha256':seal,'source_manifest_sha256':source,'source_only':True,'activation_present':False,'reused_blob_count':len(reuse['files']),'reuse_source_commit':reuse['verified_source_commit'],'transport':'update-only; combine all archive members and all exact REUSE-MANIFEST source blobs at their listed destinations; retain all inherited main files unchanged'})
verify_preservation()
assert sum(p.stat().st_size for p in ROOT.rglob('*') if p.is_file())<=2*1024**2
assert shutil.disk_usage(ROOT).free>=4*1024**3
print(json.dumps({'archive_bytes':len(raw),'archive_sha256':sha(raw),'members':len(rows),'reused_blobs':len(reuse['files']),'seal_sha256':seal,'source_manifest_sha256':source,'remote_writes':0,'native_invocations':0}))
