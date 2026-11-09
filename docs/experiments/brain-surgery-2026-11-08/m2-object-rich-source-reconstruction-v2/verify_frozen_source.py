#!/usr/bin/env python3
"""Read-only source verification. Does not construct, compile or execute A-Life."""
from pathlib import Path
import argparse,hashlib,json,subprocess
P=argparse.ArgumentParser()
g=P.add_mutually_exclusive_group(required=True)
g.add_argument('--source-root',type=Path,help='Fresh full repository root after applying the patch')
g.add_argument('--git-index',action='store_true',help='Read reconstructed blobs from the Git index selected by the caller environment')
a=P.parse_args()
D=Path(__file__).resolve().parent
m=(D/'SOURCE-FREEZE-EXPECTED.json').read_bytes()
assert hashlib.sha256(m).hexdigest()=='2402a15a31a1ccd6b4623c9d5fa0a5a9d5b80544790c80a09d805cd2e9c4bede','Expected manifest changed'
x=json.loads(m)
assert hashlib.sha256(json.dumps(x['files'],sort_keys=True,separators=(',',':')).encode()).hexdigest()==x['closure']=='0a444a493f97fdcea1bcd438fa50cdb7340e7067fe9ba418c8efcfd7ea41e0d1'
assert len(x['files'])==101
for f in x['files']:
 if a.git_index:
  b=subprocess.check_output(['git','show',':'+f['path']])
 else:
  root=a.source_root.resolve();p=root/f['path'];assert root in p.resolve().parents and not p.is_symlink();b=p.read_bytes()
 assert len(b)==f['bytes'] and hashlib.sha256(b).hexdigest()==f['sha256'],f['path']
print(json.dumps({'verified_source_files':101,'frozen_source_closure':x['closure'],'world_calls':0,'neural_calls':0,'compiled':False}))
