from pathlib import Path
import json
O=Path('/workspace/shared/m2-recovery-motion-coordination-v1'); B=Path('/workspace/shared/m2-action-target-coordination-v1')
p=json.loads((B/'compile-tests-v2.command.json').read_text())
def adapt(x):return x.replace(str(B),str(O)).replace('candidate-tests-v2','candidate-tests-v1')
p['command']=[adapt(x) for x in p['command']];p['env']={k:adapt(v) for k,v in p['env'].items()};p['cwd']=adapt(p['cwd']);p['name']='compile-tests-v1';p['timeout_seconds']=180;p['cap_bytes']=200*1024**2;p['forecast_bytes']=160*1024**2
(O/'compile-tests-v1.command.json').write_text(json.dumps(p,indent=2)+'\n')
(O/'DEPENDENCIES.json').write_bytes((B/'DEPENDENCIES.json').read_bytes())
s=(B/'run_stage_v6.py').read_text().replace(str(B),str(O)).replace('PROTECTED=4*1024**3+705*1024**2','PROTECTED=4*1024**3').replace('SOURCE-FREEZE-v6.json','SOURCE-FREEZE.json')
s=s.replace("final=allocation();peak=max(peak,final)","""final=allocation();peak=max(peak,final)
for f in freeze['files']:assert sha(O/'source'/f['path'])==f['sha256'],f['path']
for d in json.loads((O/'DEPENDENCIES.json').read_text())['read_only_dependencies']:assert sha(d['path'])==d['sha256']""")
(O/'run_stage.py').write_text(s)
