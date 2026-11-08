import json,hashlib,pathlib
root=pathlib.Path('/workspace/shared'); src=root/'motor-production-frame-report-v5.json';d=json.loads(src.read_text());frames={x['case']:x['candidates'] for x in d['enumerated_frames']}; n=0;rows=[]
noop={103,104,212}
for row in d['rows']:
 candidates=frames[row['case']]
 for arm in ['unchanged','candidate','zero_competition']:
  for interval in row[arm]['intervals']:
   for step in interval['microsteps']:
    sel=step['selection']; counts=[0]*len(candidates);rest=inactive=rest_active=different=0
    for sample in sel['sampled_bundles']:
     n+=1;representative=sample['representative'];counts[representative]+=1
     selected=[candidates[i] if i is not None else None for i in sample['motor_candidates']]
     for c,saved in zip(selected,sample['selected_primitives']):
      assert saved == (None if c is None else {k:c[k] for k in ['action_id','kind','family','target']})
     active=any(c and c['action_id'] not in noop and c['kind'] not in ['Idle','Rest'] for c in selected)
     isrest=candidates[representative]['family'] in ['Idle','Rest']
     inactive+=not active;rest+=isrest and not active;rest_active+=isrest and active
     l,m=selected[0],selected[2]
     different+=bool(l and m and l['family']=='Approach' and l['target']['entity'] is not None and m['family']=='Ingest' and l['target']['entity']!=m['target']['entity'])
    assert sel['representative_counts']==counts
    assert (sel['valid_rest_bundle'],sel['wholly_inactive_bundle'],sel['rest_representative_with_other_active_slots'],sel['approach_eat_target_mismatch'])==(rest,inactive,rest_active,different)
  last=row[arm]['intervals'][-1]['microsteps'][-1];s=last['selection']
  rows.append({'case':row['case'],'birth_dendrites':row['birth_dendrites'],'microsteps_per_interval':row['microsteps_per_interval'],'arm':arm,'first_recurrent_difference_microstep':row['first_candidate_vs_unchanged_activation_difference_microstep'],'representatives':[{'primitive':candidates[i]['action_id'],'target':candidates[i]['target']['entity'],'family':candidates[i]['family'],'count':count} for i,count in enumerate(s['representative_counts']) if count], 'quiet_rest_bundles':s['valid_rest_bundle'],'wholly_inactive_bundles':s['wholly_inactive_bundle'],'rest_plus_activity':s['rest_representative_with_other_active_slots'],'approach_eat_different_targets':s['approach_eat_target_mismatch'],'family_above_095':[a['abs_above_095'] for a in last['family_activity']]})
assert n==124416
out={'schema':'motor-native-frame-audit-v5','source_commit':'28a08813a88403ba78f79072b22c0c753bbc604b','source_tree':'9a2b6f1147631fc3a11b899f165393b62f71d9dc','report_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),'audited_sampled_bundles':n,'all_counts_and_native_primitive_targets_recomputed':True,'head_count':d['motor_head_count'],'head_width':d['motor_head_width'],'final_interval_rows':rows,'interpretation':'Different selected targets are not an execution/binding failure: food A entity2 is already reachable at distance0.3 with no possession or solid blocker; Approach entity3 plus Eat entity2 can be legitimate. No world interval was executed. Quiet full opportunity result is limited to these fixed inputs and weights; no universal rest failure claim.'}
p=root/'motor-production-frame-audit-v5.json'
with p.open('x') as f:json.dump(out,f,indent=2)
print('PASS',n,'complete sampled bundles audited; output',p)
