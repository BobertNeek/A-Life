from pathlib import Path
import json,re
O=Path('/workspace/shared/m2-recovery-motion-coordination-v1')
names=re.findall(r'#\[test\]\nfn (\w+)\(', (O/'source/crates/alife_training/src/cpu/motor_mediation/confidence/policy/optional_vocal/tests.rs').read_text())
assert len(names)==10,names
prefix='cpu::motor_mediation::confidence::policy::optional_vocal::'
tests=[prefix+'tests::'+x for x in names]+[prefix+'recovery::tests::fixed_recovery_motion_escape_truth_table']
(O/'EXACT-TESTS.json').write_text(json.dumps(tests,indent=2)+'\n')
(O/'run_exact_tests.py').write_text('''from pathlib import Path
import json,subprocess,sys
O=Path(__file__).parent
names=json.loads((O/'EXACT-TESTS.json').read_text())
results=[]
for name in names:
 p=subprocess.run([str(O/'candidate-tests-v1'),name,'--exact','--test-threads=1','--nocapture'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 print(p.stdout,flush=True)
 passed=p.returncode==0 and '1 passed; 0 failed;' in p.stdout and 'running 1 test' in p.stdout
 results.append({'test':name,'exit':p.returncode,'exact_one_passed':passed})
 (O/'EXACT-TEST-RESULTS.json').write_text(json.dumps(results,indent=2)+'\\n')
 if not passed:sys.exit(1)
assert len(results)==11
''')
p=json.loads((O/'compile-tests-v1.command.json').read_text());p.update(name='synthetic-tests-v1',command=['python',str(O/'run_exact_tests.py')],timeout_seconds=120,forecast_bytes=2*1024**2,cap_bytes=8*1024**2,no_native_fixtures=True)
(O/'synthetic-tests-v1.command.json').write_text(json.dumps(p,indent=2)+'\n')
