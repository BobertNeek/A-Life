from pathlib import Path
import os,sys,json,time,hashlib,subprocess,shutil,signal,resource
O=Path('/workspace/shared/m2-recovery-motion-coordination-v1');TOTAL=240*1024**2;PROTECTED=4*1024**3

def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def allocation():
 seen=set();total=0
 for p in [O,*O.rglob('*')]:
  st=p.lstat();key=(st.st_dev,st.st_ino)
  if key in seen:continue
  seen.add(key);total+=st.st_blocks*512
 return total

def available():
 return int(next(l for l in Path('/proc/meminfo').read_text().splitlines() if l.startswith('MemAvailable:')).split()[1])*1024

def rss_session(sid):
 total=0
 for p in Path('/proc').iterdir():
  if not p.name.isdigit():continue
  try:
   fields=(p/'stat').read_text().rsplit(')',1)[1].split()
   if int(fields[3])!=sid:continue
   total+=int(fields[21])*os.sysconf('SC_PAGE_SIZE')
  except (FileNotFoundError,ProcessLookupError,PermissionError,ValueError):pass
 return total
plan_path=Path(sys.argv[1]);p=json.loads(plan_path.read_text());name=p['name'];log=O/(name+'.log');launch=O/(name+'.launch.json');result=O/(name+'.result.json')
assert not any(x.exists() for x in [log,launch,result]),'Existing launch/output: no uncertain retry'
assert TOTAL == 240*1024**2
freeze=json.loads((O/'SOURCE-FREEZE.json').read_text())
for f in freeze['files']:assert sha(O/'source'/f['path'])==f['sha256'],f['path']
for d in json.loads((O/'DEPENDENCIES.json').read_text())['read_only_dependencies']:assert sha(d['path'])==d['sha256']
start=allocation();free=shutil.disk_usage(O).free;forecast=p['forecast_bytes'];assert start+forecast<=TOTAL;assert free-PROTECTED>=max(forecast,TOTAL-start);assert available()>3*1024**3
(O/'tmp').mkdir(exist_ok=True)
record={'name':name,'plan_sha256':sha(plan_path),'source_freeze_sha256':sha(O/'SOURCE-FREEZE.json'),'starting_allocation':start,'free_bytes':free,'protected_bytes':PROTECTED,'total_cap':TOTAL,'start_unix':time.time(),'stage_forecast':forecast,'stage_cap':p['cap_bytes'],'no_native_calls_in_compile':p.get('no_native_fixtures',False)}
with launch.open('x') as f:json.dump(record,f,indent=2);f.flush();os.fsync(f.fileno())
env=os.environ.copy();env.update(p['env']);t=time.monotonic();peak=start;rss=0;minmem=available();reason=None
with log.open('xb') as lf:
 proc=subprocess.Popen(p['command'],cwd=p['cwd'],env=env,stdout=lf,stderr=subprocess.STDOUT,start_new_session=True)
 print(json.dumps({'name':name,'pid':proc.pid,'start_allocation':start}),flush=True)
 while proc.poll() is None:
  amount=allocation();peak=max(peak,amount);rss=max(rss,rss_session(proc.pid));minmem=min(minmem,available())
  if amount>TOTAL:reason='total allocation ceiling'
  elif amount-start>p['cap_bytes']:reason='stage allocation ceiling'
  elif time.monotonic()-t>p['timeout_seconds']:reason='fixed timeout'
  elif available()<2*1024**3:reason='available-memory floor'
  elif shutil.disk_usage(O).free<PROTECTED:reason='protected disk floor'
  elif log.stat().st_size>16*1024**2:reason='log ceiling'
  if reason:
   os.killpg(proc.pid,signal.SIGTERM)
   try:proc.wait(timeout=15)
   except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
   break
  time.sleep(.5)
 rc=proc.wait()
final=allocation();peak=max(peak,final)
for f in freeze['files']:assert sha(O/'source'/f['path'])==f['sha256'],f['path']
for d in json.loads((O/'DEPENDENCIES.json').read_text())['read_only_dependencies']:assert sha(d['path'])==d['sha256']
record.update({'pid':proc.pid,'exit_code':rc,'stopped_reason':reason,'elapsed_seconds':time.monotonic()-t,'final_allocation':final,'peak_allocation_sampled':peak,'peak_growth_sampled':peak-start,'peak_session_rss_sampled':rss,'child_maxrss_kib':resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss,'minimum_available_memory':minmem,'free_after':shutil.disk_usage(O).free,'log_sha256':sha(log),'monitored_not_quota':True})
with result.open('x') as f:json.dump(record,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
print(json.dumps(record),flush=True)
sys.exit(rc or int(reason is not None))
