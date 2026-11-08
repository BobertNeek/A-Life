"""NEW reconstructed scalar-only contract. Original file is inaccessible; no byte-equivalence claim."""
import hashlib,json,math,re,struct
CASES={f's{i+1}_{w}':(2026112301+i,w) for i in range(2) for w in ('old_world','new_world')}
ORDER=('s1_old_world','s1_new_world','s2_new_world','s2_old_world')
ASSET='6edcd6dd1dc5bfedf8f7b73bf605bb480e36c0ba022a316840a6f0bd87629632'
class Reject(Exception):
    def __init__(self,gate):self.gate=gate;super().__init__(gate)
def need(x,gate):
    if not x:raise Reject(gate)
def obj(**x):return ('object',x)
def arr(x,n):return ('array',x,n)
def integer(a,b):return ('int',a,b)
def number(a,b):return ('num',a,b)
def enum(*x):return ('enum',x)
def maybe(x):return ('nullable',x)
B=('bool',);U=integer(0,256);UNIT=number(0,1);SIGNED=number(-1,1);SHA=('hex',64);GIT=('hex',40)
ROLE=enum('h','s','none','other');OWNER=enum('missing','consumed','nobody','self','other','player');MODE=enum('none','activity','deliberate')
OWN=obj(target=enum('h','s'),active=B,owner=OWNER,mode=MODE)
TRANSFER=obj(target=enum('h','s'),kind=enum('acquired','released','placed','consumed'),previous_owner=OWNER,new_owner=OWNER)
CAND=obj(index=integer(0,31),primitive=integer(1,65535),family=integer(0,7),target=ROLE,active=B,owner=OWNER,mode=MODE,
 contact=B,contact_reachable=maybe(B),grasp_reachable=maybe(B),held_count=integer(0,1),actor_alive=B,effector=B,
 target_sources=integer(0,4),family_sources=integer(0,4),target_confidence=UNIT,family_confidence=UNIT,family_valence=SIGNED,
 source_sequence=maybe(integer(1,2**32)),source_exact_ingest=B)
CHANNEL=obj(slot=integer(0,4),candidate=integer(0,31),primitive=integer(1,65535),target=ROLE,phase=enum('attempt','preparation','completed'),
 executed=B,failure=enum('none','missing_affordance','action_rejected','blocked','executor_invalid'),
 contact=enum('none','touch','collision','blocked','consumed','moved'),physical_target=ROLE,
 event=enum('none','failed','acquired','placed_only','released','consumed','atomic_swap'),transfers=arr(TRANSFER,2))
DECISION=obj(sequence=integer(1,2**32),sampling_counter=integer(0,2**32-1),actual_replay_equal=B,pain=UNIT,pre_energy=UNIT,
 aversive_harm=UNIT,experienced_valence=SIGNED,owners_before=arr(OWN,2),owners_after=arr(OWN,2),candidates=arr(CAND,32),channels=arr(CHANNEL,5),
 physical_meals=arr(B,2),ingest_episode_active=arr(B,2),ingest_episode_staged=arr(B,2))
ROW=obj(primary_tick=integer(1,256),world_tick_before=integer(0,2**53),world_tick_after=integer(0,2**53),sampling_counter_after=integer(0,2**32-1),
 sleeping=B,sleep_consolidations=integer(0,1),boundary_owners=arr(OWN,2),alive=B,energy=UNIT,health=UNIT,injury=UNIT,
 active_memory_count=integer(0,65535),staged_memory_count=integer(0,65535),decisions=arr(DECISION,1))
PAIR=obj(asset_sha256=SHA,founder_optimizer_sha256=SHA,full_initial_checkpoint_sha256=SHA,initial_body_biology_sha256=SHA,
 initial_scene_sha256=SHA,sampling_seed=integer(0,2**32-1),sampling_counter=enum(0),actual_world_tick=enum(600))
SOURCE=obj(source_tree=GIT,source_closure_sha256=SHA,executable_sha256=SHA,plan_sha256=SHA,common_patch_sha256=SHA,world_patch_sha256=SHA)
CASE=obj(schema_version=enum(1),case_id=enum(*CASES),seed=enum(2026112301,2026112302),world=enum('old_world','new_world'),initial=PAIR,source=SOURCE,
 rows=arr(ROW,256),stop=enum('tick_budget','third_harmful_meal','health','injury','death'),founder_optimizer_unchanged=B,checkpoint_roundtrip_exact=B,primary_next_checkpoint_tested=enum(False))
COMPLETION=obj(case_id=enum(*CASES),original_summary_sha256=SHA,original_summary_byte_count=integer(1,8*1024**2))
def validate(x,s):
 tag=s[0]
 if tag=='object':
  need(type(x)is dict,'schema');need(set(x)==set(s[1]),'unknown_or_missing_field')
  for k,v in s[1].items():validate(x[k],v)
 elif tag=='array':
  need(type(x)is list and len(x)<=s[2],'bound')
  for item in x:validate(item,s[1])
 elif tag=='bool':need(type(x)is bool,'schema')
 elif tag=='int':need(type(x)is int and s[1]<=x<=s[2],'bound')
 elif tag=='num':need(type(x)in(int,float) and math.isfinite(x) and s[1]<=x<=s[2],'bound')
 elif tag=='hex':need(type(x)is str and re.fullmatch('[0-9a-f]{'+str(s[1])+'}',x)is not None,'identity')
 elif tag=='enum':need(any(type(x)is type(y) and x==y for y in s[1]),'schema')
 elif tag=='nullable':
  if x is not None:validate(x,s[1])
 else:raise Reject('schema_definition')
def canonical(x):return (json.dumps(x,sort_keys=True,separators=(',',':'),allow_nan=False)+'\n').encode()
def sha(raw):return hashlib.sha256(raw).hexdigest()
def f32(x):return struct.unpack('!f',struct.pack('!f',x))[0]
def pairs(items):
 result={}
 for key,value in items:need(key not in result,'duplicate_key');result[key]=value
 return result
def parse(raw,s):
 need(type(raw)is bytes and 0<len(raw)<=8*1024**2,'bound')
 try:x=json.loads(raw,object_pairs_hook=pairs,parse_constant=lambda _:(_ for _ in ()).throw(Reject('nonfinite')))
 except Reject:raise
 except Exception:raise Reject('parse') from None
 validate(x,s);return x
def ownership(rows):
 need(len(rows)==2 and [r['target'] for r in rows]==['h','s'],'ownership')
 need(sum(r['owner']=='self' for r in rows)<=1,'ownership')
 for row in rows:
  need(row['active']==(row['owner'] not in ('missing','consumed')),'ownership')
  need((row['mode']=='none')==(row['owner'] not in ('self','other')),'ownership')
 return {r['target']:r['owner'] for r in rows}
def actual_event(c):
 t=c['transfers'];k=[v['kind'] for v in t]
 need(not t or c['slot']==2,'transfer_channel')
 if not t:event='failed' if c['failure']!='none' else 'none'
 elif k==['placed','acquired']:
  need(t[0]['target']!=t[1]['target'] and t[1]['target']==c['target'],'swap_target');event='atomic_swap'
  need(c['physical_target']==c['target'],'swap_target');need(c['primitive']!=210 or c['phase']=='preparation','swap_phase')
 elif k==['acquired']:event='acquired';need(t[0]['target']==c['target']==c['physical_target'],'acquire_target');need(c['primitive']!=210 or c['phase']=='preparation','acquire_phase')
 elif k==['placed']:event='placed_only';need(c['physical_target']==t[0]['target'],'place_target');need(c['primitive']!=210 or c['phase']=='preparation','place_phase')
 elif k==['released']:event='released';need(t[0]['target']==c['target'],'release_target')
 elif k==['consumed']:
  event='consumed';need(c['slot']==2 and c['primitive']==210 and c['phase']=='completed' and c['executed']is True
   and c['contact']=='consumed' and c['physical_target']==c['target']==t[0]['target'],'meal')
 else:raise Reject('transfer_order')
 if t:need(c['executed']is True and c['contact']==('consumed' if event=='consumed' else 'touch'),'transfer_execution')
 need(event==c['event'],'receipt_event');need(not t or c['failure']=='none','failed_transfer')
 need((c['failure']=='none')==c['executed'],'failure');need(c['contact']!='consumed' or event=='consumed','meal');return event

def validate_case(raw,completion_raw,binding):
 completion=parse(completion_raw,COMPLETION)
 need(completion['original_summary_sha256']==sha(raw) and completion['original_summary_byte_count']==len(raw),'completion_hash')
 x=parse(raw,CASE);need(x['case_id']==completion['case_id'],'case_id');need((x['seed'],x['world'])==CASES[x['case_id']],'case_id')
 need(x['initial']['asset_sha256']==ASSET and x['initial']['sampling_seed']==x['seed'],'asset')
 need(x['source']==binding[x['world']],'source_binding');need(x['founder_optimizer_unchanged'] and x['checkpoint_roundtrip_exact'],'preservation')
 need(len(x['rows'])>0,'empty_return')
 need(validate_rows(x['rows'])==x['stop'],'stop');return x

def validate_rows(rows):
 """Shared study semantics. Caller schema-validates ROWs; None means nonterminal."""
 counter=0;sequence=0;meals=[0,0];prior_physical=set();terminal=None;last_boundary={'h':'nobody','s':'nobody'}
 for index,row in enumerate(rows,1):
  need(terminal is None,'continued_after_stop');need(row['primary_tick']==index and row['world_tick_before']==599+index and row['world_tick_after']==600+index,'clock')
  expected_boundary=last_boundary.copy();ownership(row['boundary_owners']);need(not row['sleeping'] or not row['decisions'],'sleep_decision')
  for d in row['decisions']:
   need(d['actual_replay_equal'] and d['sampling_counter']==counter and d['sequence']>sequence,'sampling')
   counter+=1;sequence=d['sequence'];current=ownership(d['owners_before']);after=ownership(d['owners_after']);need(current==last_boundary,'boundary_continuity')
   need([c['index'] for c in d['candidates']]==list(range(len(d['candidates']))),'candidate_order')
   for c in d['candidates']:
    if c['source_exact_ingest']:
     need((c['source_sequence'],c['target']) in prior_physical and c['family_sources']>0,'recall_provenance')
   seen=set();observed=[False,False]
   for c in d['channels']:
    need(c['slot'] not in seen,'channel_duplicate');seen.add(c['slot']);need(c['candidate']<len(d['candidates']),'candidate_index')
    chosen=d['candidates'][c['candidate']];need((chosen['primitive'],chosen['target'])==(c['primitive'],c['target']),'selected_target')
    event=actual_event(c)
    for t in c['transfers']:
     need(current[t['target']]==t['previous_owner'],'owner_before')
     if t['kind']=='acquired':need(t['previous_owner']=='nobody' and t['new_owner']=='self','owner_transfer')
     else:need(t['previous_owner']=='self' and t['new_owner']=='nobody','owner_transfer')
     current[t['target']]='consumed' if t['kind']=='consumed' else t['new_owner']
    if event=='consumed':observed[0 if c['target']=='h' else 1]=True
   need(current==after,'owner_after');expected_boundary=after.copy();need(d['physical_meals']==observed,'meal_count')
   for key in ('physical_meals','ingest_episode_active','ingest_episode_staged'):need(len(d[key])==2,'coverage')
   for i in range(2):
    need(not(d['ingest_episode_active'][i] or d['ingest_episode_staged'][i]) or observed[i],'coverage')
    if observed[i]:meals[i]+=1;prior_physical.add((sequence,'h' if i==0 else 's'))
  last_boundary=ownership(row['boundary_owners'])
  for role in ('h','s'):
   need(last_boundary[role]==expected_boundary[role] or (expected_boundary[role],last_boundary[role])==('consumed','nobody') or (not row['alive'] and (expected_boundary[role],last_boundary[role])==('self','nobody')),'unreceipted_boundary_transfer')
  need(row['sampling_counter_after']==counter,'sampling')
  terminal=('death' if not row['alive'] else 'health' if f32(row['health'])<f32(.70) else 'injury' if f32(row['injury'])>f32(.25)
   else 'third_harmful_meal' if meals[0]>=3 else 'tick_budget' if index==256 else None)
 return terminal

def metrics(x):
 rows=x['rows'];decisions=[d for r in rows for d in r['decisions']];meals=[sum(d['physical_meals'][i] for d in decisions) for i in range(2)]
 traps=[d for d in decisions if ownership(d['owners_before'])['h']=='self' and any(c['slot']==2 and c['primitive']==210 and c['target']=='s' for c in d['channels'])]
 swaps=[d for d in traps if any(c['event']=='atomic_swap' and c['target']=='s' for c in d['channels'])]
 return dict(primary_ticks=len(rows),decisions=len(decisions),harmful_meals=meals[0],safe_meals=meals[1],safe_meals_per_100_ticks=100*meals[1]/len(rows),
  safe_meals_per_100_decisions=100*meals[1]/len(decisions) if decisions else None,h_held_s_eat_attempts=len(traps),h_to_s_atomic_swaps=len(swaps),
  h_held_s_eat_failures=sum(any(c['slot']==2 and c['primitive']==210 and c['target']=='s' and c['failure']!='none' for c in d['channels']) for d in traps),
  physical_episode_coverage_active=[sum(d['ingest_episode_active'][i] for d in decisions) for i in range(2)],physical_episode_coverage_staged=[sum(d['ingest_episode_staged'][i] for d in decisions) for i in range(2)],
  exact_ingest_recall_decisions=sum(any(c['source_exact_ingest'] for c in d['candidates']) for d in decisions),
  positive_confidence_ingest_recall_decisions=sum(any(c['source_exact_ingest'] and c['family_confidence']>0 for c in d['candidates']) for d in decisions),
  min_health=min(r['health'] for r in rows),max_injury=max(r['injury'] for r in rows),final_energy=rows[-1]['energy'],max_pain=max((d['pain'] for d in decisions),default=0),sleeping_ticks=sum(r['sleeping'] for r in rows),
  s_contact_while_h_held_decisions=sum(ownership(d['owners_before'])['h']=='self' and any(z['target']=='s' and z['contact'] for z in d['candidates']) for d in decisions),
  h_held_s_eat_opportunity_observed=bool(traps),inference='descriptive_two_seed_world_bundle_only')
