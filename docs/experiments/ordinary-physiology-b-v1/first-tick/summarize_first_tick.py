import json,sys,hashlib,collections
from pathlib import Path
p=Path(sys.argv[1]);j=json.loads(p.read_text());assert len(j['trials'])==80
rows=[];choices=collections.Counter();contacts=collections.Counter()
for t in j['trials']:
 assert len(t['rows'])==1 and t['early_stop'] is None and t['founder_optimizer_unchanged']
 r=t['rows'][0];assert len(r['decisions'])==1;d=r['decisions'][0]
 selected=[]
 for channel,index in enumerate(d['action']['motor_candidates']):
  if index is None:continue
  c=d['candidates'][index];selected.append(dict(channel_index=channel,index=index,family=c['family'],kind=c['kind']))
  choices[(c['family'],c['kind'])]+=1
 for x in d['outcome'].get('joint',{}).get('channel_outcomes',[]):contacts[x['physical']['contact']]+=1
 rep=d['candidates'][d['action']['representative']]
 path={str(x['id']):{k:v for k,v in x.items() if k!='id'} for x in d['path']}
 rows.append(dict(arm=t['recipe']['arm'],case=t['recipe']['case']['name'],first_input_signature=t['first_input_signature'],microsteps=d['microsteps'],receptor_effects=d['receptor_effects'],gate_offset=d['region_gate_offset'],atp_available=d['available_atp_q16'],atp_debit=d['debit_atp_q16'],brain_work=d['brain_work'],drive_before=r['biology_before']['homeostasis']['drives'],drive_after=r['biology_after']['homeostasis']['drives'],energy_before=r['biology_before']['body']['energy'],energy_after=r['biology_after']['body']['energy'],health_after=r['biology_after']['body']['health'],position_before=r['position_before'],position_after=r['position_after'],sleeping=r['sleeping_organisms'],resident_before=r['resident_before'],resident_after=r['resident_after'],representative=dict(family=rep['family'],kind=rep['kind']),choices=selected,selected_primitives=[dict(channel=c['channel'],primitive=c['primitive'])for c in d['selected_bundle']['channels']],channel_outcomes=d['outcome'].get('joint',{}).get('channel_outcomes',[]),candidates=[dict({k:c[k] for k in ['index','family','kind','innate_bias','neural_action_readout','total_logit']},has_target=c['candidate']['target']['entity'] is not None)for c in d['candidates']],path=path,tonic_effective_weights=d['tonic_effective_weights'],production_replay_exact=d['production_replay_exact']))
key=lambda r:(r['arm']['bounded'],r['arm']['dedicated_b'],r['arm']['tonic_bridges'],r['case'])
by={key(r):r for r in rows}
contrasts=[]
for bounded in [False,True]:
 for b in [False,True]:
  for tonic in [False,True]:
   for changed,base,family in [('neutral_fatigued','neutral_comfortable','Rest'),('food_fatigued','food_comfortable','Approach'),('bitter_fatigued','bitter_comfortable','Approach'),('food_hungry','food_comfortable','Ingest'),('bitter_hungry','bitter_comfortable','Ingest'),('object_curious','object_comfortable','Inspect')]:
    a=by[(bounded,b,tonic,changed)];z=by[(bounded,b,tonic,base)]
    ca=[c for c in a['candidates'] if c['family']==family and (family=='Rest' or c['has_target'])];cz=[c for c in z['candidates'] if c['family']==family and (family=='Rest' or c['has_target'])]
    assert len(ca)==len(cz)==1
    contrasts.append(dict(bounded=bounded,dedicated_b=b,tonic_bridges=tonic,contrast=changed+'-'+base,family=family,neural_delta=ca[0]['neural_action_readout']-cz[0]['neural_action_readout'],total_delta=ca[0]['total_logit']-cz[0]['total_logit'],innate_delta=ca[0]['innate_bias']-cz[0]['innate_bias']))
factor_contrasts=[]
for bounded in [False,True]:
 for tonic in [False,True]:
  for name in sorted({r['case']for r in rows}):
   a=by[(bounded,True,tonic,name)];z=by[(bounded,False,tonic,name)]
   assert len(a['candidates'])==len(z['candidates'])
   for x,y in zip(a['candidates'],z['candidates']):
    assert (x['index'],x['family'],x['kind'],x['has_target'])==(y['index'],y['family'],y['kind'],y['has_target'])
    factor_contrasts.append(dict(bounded=bounded,tonic_bridges=tonic,case=name,index=x['index'],family=x['family'],kind=x['kind'],has_target=x['has_target'],dedicated_minus_baseline_neural=x['neural_action_readout']-y['neural_action_readout'],dedicated_minus_baseline_total=x['total_logit']-y['total_logit']))
out=dict(schema='ordinary_physiology_b_first_tick_summary_v1',scope='production integration only; first tick cannot establish delayed drive competence',source='6dee4ea0e93cb8e7315e94611d9ac0c86fb97ff4',raw_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),world_ticks=80,new_seeds_used=0,seed=7,all_replays_exact=all(r['production_replay_exact']for r in rows),first_inputs_exact_across_arms=j['first_inputs_exact_across_arms'],microstep_counts=sorted(set(r['microsteps']for r in rows)),gate_offsets=sorted(set(r['gate_offset']for r in rows)),total_sleeping=sum(r['sleeping']for r in rows),min_health=min(r['health_after']for r in rows),choices=[dict(family=f,kind=k,count=n)for(f,k),n in sorted(choices.items())],physical_contacts=dict(contacts),actual_zero_displacement_cases=sum(r['position_after']==r['position_before'] for r in rows),neural_contrast_max_abs=max(abs(c['neural_delta'])for c in contrasts),contrasts=contrasts,factor_contrasts=factor_contrasts,rows=rows)
print(json.dumps(out,indent=2))
