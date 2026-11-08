import json,sys,pathlib,gzip
out=[]
for file in sys.argv[1:]:
 r=json.load(gzip.open(file,'rt') if file.endswith('.gz') else open(file));arms={a['factor']:a for a in r['arms'] if a['motor_start']==0}
 def sample(factor,condition,context,frame=64):
  row=next(x for x in arms[factor]['rows'] if x['condition']==condition and x['context']==context)
  return next(x for x in row['samples'] if x['frame']==frame)
 def vec(factor,condition,context):return sample(factor,condition,context)['delta']
 records=[]
 for context in ['nutritious_contact','aversive_contact','distant','zero_B']:
  b=sample('Baseline','fatigue',context);d=sample('Dedicated','fatigue',context)
  row={'context':context,'baseline_balanced_logits':b['baseline_logits'],'dedicated_balanced_logits':d['baseline_logits'],'baseline_fatigue_delta':b['delta'],'dedicated_fatigue_delta':d['delta'],'added_fatigue_delta':[y-x for x,y in zip(b['delta'],d['delta'])], 'basis_B':d['basis_B'],'dedicated_neurons':d['path_neurons'],'hunger_baseline_delta':vec('Baseline','hunger',context),'hunger_dedicated_delta':vec('Dedicated','hunger',context)}
  for cut in ['SourceCut','RelayCut']:
   if cut not in arms: continue
   x=sample(cut+'Baseline','fatigue',context);y=sample(cut,'fatigue',context)
   row[cut+'_matched_added_delta']=[j-i for i,j in zip(x['delta'],y['delta'])]
  if 'ReadoutCut' in arms:
   x=sample('ReadoutCut','fatigue',context)
   row['readout_cut_indirect_delta']=[y-z for z,y in zip(b['delta'],x['delta'])]
   row['direct_B_fatigue_delta']=[y-z for z,y in zip(x['delta'],d['delta'])]
  row['washout_neural_delta']=sample('Dedicated','fatigue',context,96)['delta']
  records.append(row)
 math_errors=[]
 for condition in ({x['condition'] for x in arms['Dedicated']['rows']} if 'ReadoutCut' in arms else []):
  for context in ['nutritious_contact','aversive_contact','distant','zero_B']:
   for frame in [32,33,34,36,40,48,64,65,72,80,96]:
    a=sample('Dedicated',condition,context,frame);b=sample('ReadoutCut',condition,context,frame)
    e=abs(a['neural_logits'][3]-b['neural_logits'][3]-a['predicted_added_readout']);math_errors.append(e)
    assert all(abs(a['neural_logits'][i]-b['neural_logits'][i])<2e-6 for i in range(8) if i!=3)
 out.append({'file':pathlib.Path(file).name,'branch_profile':r['branch_plan']['profile'],'branch_digest':r['branch_plan']['branch_plan_sha256'],'reference_plan':r['reference_plan'],'records':records,'max_direct_readout_identity_error':max(math_errors) if math_errors else None,'world_steps':r['world_steps'],'learning_steps':r['learning_steps'],'readout_cut_traces_exact':arms.get('ReadoutCut',{}).get('readout_cut_traces_exact'),'optimizer_unchanged':all(a['optimizer_unchanged'] for a in r['arms']),'input_cuts_exact':all(a['input_cuts_exact'] for a in r['arms'])})
print(json.dumps(out,indent=2))
