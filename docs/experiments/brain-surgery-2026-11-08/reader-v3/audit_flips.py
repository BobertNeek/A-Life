#!/usr/bin/env python3
import json,pathlib
r=pathlib.Path(__file__).resolve().parent
ids={1:'IDLE',3:'REST',101:'APPROACH',102:'FLEE',103:'NO_LOCOMOTION',104:'NO_POSTURE',210:'EAT',212:'NO_MANIPULATION',213:'PLAY',2201:'CLOSE',2202:'RELEASE'}
rows=[]
for line in (r/'public/events.ndjson').read_bytes().splitlines():
 event=json.loads(line)
 if event['kind']!='step':continue
 for f in event['decisions']:
  b,v=f['variants'][:2]
  if b['motor_candidates']==v['motor_candidates']:continue
  selected={i for i in b['motor_candidates']+v['motor_candidates'] if i is not None}
  candidates=[]
  for c,d in zip(b['candidates'],v['candidates']):
   if c['index'] not in selected:continue
   candidates.append({'index':c['index'],'action':c['action'],'name':ids.get(c['action'],str(c['action'])),'target':c['target'],'held':c['held'],'grasp_reachable':c['grasp_reachable'],'input53':c['input53'],'off_logit':c['logit'],'shadow_logit':d['logit'],'off_representative_probability':c['representative_probability'],'shadow_representative_probability':d['representative_probability']})
  rows.append({'seed':event['seed'],'primary_step':event['primary_step'],'off_representative':b['representative'],'shadow_representative':v['representative'],'off_motor_indices':b['motor_candidates'],'shadow_motor_indices':v['motor_candidates'],'selected_candidates':candidates,'actual_off_physical':f['actual_physical'],'safe_eat_candidates':[{'index':c['index'],'off_logit':c['logit'],'shadow_logit':d['logit'],'off_probability':c['representative_probability'],'shadow_probability':d['representative_probability']} for c,d in zip(b['candidates'],v['candidates']) if c['target']=='s' and c['action']==210]})
assert len(rows)==9
(r/'nine-flip-audit.json').write_text(json.dumps(rows,indent=2,allow_nan=False)+'\n')
for x in rows: print(x['seed'],x['primary_step'],[(c['name'],c['target'],round(c['off_logit'],6),round(c['shadow_logit'],6),c['held']) for c in x['selected_candidates']],[(p['action'],p['target'],p['succeeded'],p['transfers']) for p in x['actual_off_physical']])
