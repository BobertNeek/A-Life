import json,hashlib,statistics,itertools
from pathlib import Path
src=Path('/workspace/scratch/ee5e08a3ca9b/nano-direct-native-v4.json')
p=json.loads(src.read_bytes()); out={'source_commit':'fb918e29a612be0cef6d2a0e5885f04d7f0b7d1f','source_tree':'2854ee759e9d0fe6a018738db72e6e8a8b359018','raw_local_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),'executed_world_intervals':p['executed_world_intervals'],'contexts':[], 'discrete_choice_comparisons':[]}
def stats(x):
 return {'count':len(x),'min':min(x) if x else None,'max':max(x) if x else None,'mean':statistics.mean(x) if x else None}
for r in p['results']:
 rows=r['rows']; mem=[v for row in rows for _,v in row['memory_inputs']]; contrib=[row['new_port_contribution_abs_max'] for row in rows]; gaps=[row['global_candidate_top_gap_not_conditional_sampler_gap'] for row in rows]; ratios=[c/g for c,g in zip(contrib,gaps) if g is not None and g>0]
 q={k:r[k] for k in ['factor','peer_body_fixture','requested_loneliness','phenotype_hash','inputs_digest','matched_initial_state_verified','new_head_effective_offset_abs_max','new_head_effective_weight_abs_max','new_head_initial_genetic_bits']}
 q.update(decision_count=len(rows),microsteps=sorted(set(row['microsteps'] for row in rows)),initial_loneliness=r['initial_biology']['homeostasis']['drives']['loneliness'],final_loneliness=r['final_biology']['homeostasis']['drives']['loneliness'],recall_counts={'positive':sum(v>0 for v in mem),'negative':sum(v<0 for v in mem),'zero':sum(v==0 for v in mem)},recall_range=stats(mem),nonzero_port_rows=sum(v>0 for v in contrib),port_abs_max=stats(contrib),global_top_gap_not_conditional_sampler_gap=stats([x for x in gaps if x is not None]),port_over_global_positive_gap=stats(ratios),port_over_temperature=stats([row['new_port_abs_max_over_temperature'] for row in rows]),counterfactuals=[])
 cf=r['detached_counterfactuals']; index={(c['pressure'],c['clear_lane32'],c['zero_effective_new_head'],c['frames']):c for c in cf}
 for (pressure,clear,zero,frames),c in index.items():
  if zero: continue
  z=index[(pressure,clear,True,frames)]
  delta=[a-b for a,b in zip(c['final_logits'],z['final_logits'])]
  q['counterfactuals'].append({'pressure':pressure,'clear_lane32':clear,'frames':frames,'microsteps':c['microsteps'],'source':c['final_pressure_source'],'reference':c['final_reference_source'],'new_port_signed_min':min(delta),'new_port_signed_max':max(delta),'new_port_abs_max':max(map(abs,delta))})
 q['conditional_channels']=[]
 for slot,label in enumerate(['Locomotion','Orientation','Manipulation','Vocal','Posture','SpeciesSpecific']):
  measurements=[]
  for row in rows:
   ids=[i for i in range(row['action']['candidate_count']) if row['action']['motor_masks'][slot] & (1<<i)]
   if len(ids)<2:continue
   logits=row['logits'][0]; d=row['new_port_contribution']; ordered=sorted(ids,key=lambda i:logits[i],reverse=True); a,b=ordered[:2]; gap=logits[a]-logits[b]
   forced=row['action']['forced_slots'][row['action']['representative']]==slot
   measurements.append({'top_gap':gap,'new_head_top_pair_margin_delta':d[a]-d[b],'new_head_max_pairwise_delta':max(d[i] for i in ids)-min(d[i] for i in ids),'forced_by_representative':forced})
  if measurements:
   q['conditional_channels'].append({'slot':slot,'label':label,'eligible_rows':len(measurements),'independently_sampled_rows':sum(not m['forced_by_representative'] for m in measurements),'top_gap':stats([m['top_gap'] for m in measurements]),'head_top_pair_margin_delta':stats([m['new_head_top_pair_margin_delta'] for m in measurements]),'head_max_pairwise_delta':stats([m['new_head_max_pairwise_delta'] for m in measurements]),'absolute_top_pair_delta_over_positive_gap':stats([abs(m['new_head_top_pair_margin_delta'])/m['top_gap'] for m in measurements if m['top_gap']>0])})
 out['contexts'].append(q)
for a,b in itertools.combinations(p['results'],2):
 if (a['peer_body_fixture'],a['requested_loneliness'])!=(b['peer_body_fixture'],b['requested_loneliness']): continue
 def choice(row):return (row['action']['representative'],row['action']['motor_candidates'])
 out['discrete_choice_comparisons'].append({'factor_a':a['factor'],'factor_b':b['factor'],'peer':a['peer_body_fixture'],'need':a['requested_loneliness'],'compared_rows':min(len(a['rows']),len(b['rows'])),'different_rows':sum(choice(x)!=choice(y) for x,y in zip(a['rows'],b['rows']))})
path=Path('/workspace/scratch/ee5e08a3ca9b/nano-direct-native-v4-summary.json');path.write_text(json.dumps(out,indent=2)+'\n')
print('intervals',out['executed_world_intervals'],'hash',out['raw_local_sha256'])
for c in out['contexts']: print(c['factor'],c['peer_body_fixture'],c['requested_loneliness'],'n',c['decision_count'],'recall',c['recall_counts'],'port',c['port_abs_max'],'offset',c['new_head_effective_offset_abs_max'],'finalL',c['final_loneliness'])
print('choice differences',[(x['factor_a'],x['factor_b'],x['peer'],x['need'],x['different_rows']) for x in out['discrete_choice_comparisons']])
