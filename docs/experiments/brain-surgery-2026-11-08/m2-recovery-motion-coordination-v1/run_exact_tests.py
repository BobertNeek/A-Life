from pathlib import Path
import json,subprocess,sys
O=Path(__file__).parent
names=json.loads((O/'EXACT-TESTS.json').read_text())
results=[]
for name in names:
 p=subprocess.run([str(O/'candidate-tests-v1'),name,'--exact','--test-threads=1','--nocapture'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 print(p.stdout,flush=True)
 passed=p.returncode==0 and '1 passed; 0 failed;' in p.stdout and 'running 1 test' in p.stdout
 results.append({'test':name,'exit':p.returncode,'exact_one_passed':passed})
 (O/'EXACT-TEST-RESULTS.json').write_text(json.dumps(results,indent=2)+'\n')
 if not passed:sys.exit(1)
assert len(results)==11
