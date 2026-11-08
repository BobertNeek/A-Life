#!/usr/bin/env python3
"""Analyze only the prospectively allowlisted public scalar stream, including partial prefixes."""
import collections,json,math,pathlib
root=pathlib.Path(__file__).resolve().parent
path=root/'public/events.ndjson'
raw=path.read_bytes(); assert len(raw)<=64*1024*1024
trailing=not raw.endswith(b'\n')
lines=raw.splitlines();complete_lines=lines[:-1] if trailing else lines
records=[]
for line in complete_lines:
 assert len(line)+1<=512*1024
 value=json.loads(line,parse_constant=lambda s:(_ for _ in ()).throw(ValueError(s)))
 def finite(v):
  if isinstance(v,float):assert math.isfinite(v)
  elif isinstance(v,list):
   for x in v:finite(x)
  elif isinstance(v,dict):
   for x in v.values():finite(x)
 finite(value); records.append(value)
seeds=[2026112401,2026112402];gains=[0.,.0625,.25,1.,4.]
result={'kind':'passive_shadow_analysis','complete_records':len(records),'public_bytes':len(raw),'trailing_incomplete_record':trailing,'invocation_complete':any(r['kind']=='invocation_complete' for r in records),'failure_records':[r for r in records if r['kind']=='invocation_failed'],'seeds':[]}
for seed in seeds:
 rr=[r for r in records if r.get('seed')==seed];steps=[r for r in rr if r['kind']=='step'];summary=next((r for r in rr if r['kind']=='life_complete'),None)
 stats={g:{'candidate_scores':0,'representative_changes':0,'motor_bundle_changes':0,'max_abs_logit_delta':0.,'max_abs_probability_delta':0.,'memory_cap_candidates':0} for g in gains}
 ops=collections.defaultdict(lambda:{'candidates':0,'nonzero_port':0,'exact_ingest_bound':0,'exact_ingest_bound_nonzero':0,'port_min':None,'port_max':None,'effective_coeff53_min':None,'effective_coeff53_max':None})
 physical=collections.Counter();decisions=0
 def limits(d,key,value):
  d[key+'_min']=value if d[key+'_min'] is None else min(d[key+'_min'],value)
  d[key+'_max']=value if d[key+'_max'] is None else max(d[key+'_max'],value)
 for step in steps:
  for frame in step['decisions']:
   decisions+=1;assert frame['baseline_replay_matches_actual'] and frame['preservation'];vs=frame['variants'];assert [v['gain'] for v in vs]==gains;base=vs[0]
   for c in base['candidates']:
    if c['action'] not in [2201,2202]:continue
    o=ops[f"{c['target']}/{c['action']}"];o['candidates']+=1;o['nonzero_port']+=int(c['input53']!=0)
    bound=c['source_ingest_family'] and c['source_exact_consumed'] and c['source_same_tracked_target'];o['exact_ingest_bound']+=int(bound);o['exact_ingest_bound_nonzero']+=int(bound and c['input53']!=0)
    limits(o,'port',c['input53']);limits(o,'effective_coeff53',c['effective_coeff25_26_53'][2])
   for v in vs:
    st=stats[v['gain']];st['representative_changes']+=int(v['representative']!=base['representative']);st['motor_bundle_changes']+=int(v['motor_candidates']!=base['motor_candidates'])
    for c,b in zip(v['candidates'],base['candidates']):
     assert c['index']==b['index'] and c['action']==b['action'] and c['target']==b['target'];st['candidate_scores']+=1
     assert abs(c['input53'])<=1.
     assert abs(c['logit']-b['logit'])<=8*v['gain']*abs(c['input53'])+0.0001
     st['max_abs_logit_delta']=max(st['max_abs_logit_delta'],abs(c['logit']-b['logit']))
     st['max_abs_probability_delta']=max(st['max_abs_probability_delta'],abs(c['representative_probability']-b['representative_probability']))
     st['memory_cap_candidates']+=int(c['memory_raw']!=c['memory_clamped'])
   for event in frame['actual_physical']:
    if event['succeeded']:physical[f"{event['target']}/{event['action']}/success"]+=1
    if event['contact_consumed']:physical[f"{event['target']}/physical_meal"]+=1
    for transfer in event['transfers']:physical[f"{transfer['target']}/{transfer['kind']}"]+=1
 state={'seed':seed,'life_attempted':any(r['kind']=='life_attempt' for r in rr),'preparation_attempted':max([r['attempted'] for r in rr if r['kind']=='preparation_progress']+[0]),'preparation_returned':max([r['returned'] for r in rr if r['kind']=='preparation_progress']+[0]),'primary_attempted':sum(r['kind']=='step_attempt' for r in rr),'primary_returned':sum(r['kind']=='step_returned' for r in rr),'primary_fully_projected':len(steps),'decisions_projected':decisions,'summary':summary,'last_projected_meals_h_s':steps[-1]['meals_h_s'] if steps else None,'min_projected_health':min([r['health'] for r in steps],default=None),'max_projected_injury':max([r['injury'] for r in steps],default=None),'last_projected_energy':steps[-1]['energy'] if steps else None,'operation_engagement':dict(ops),'shadow_gains':stats,'actual_physical_counts':dict(physical)}
 result['seeds'].append(state)
(root/'analysis.json').write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
print(json.dumps(result,indent=2,allow_nan=False))
