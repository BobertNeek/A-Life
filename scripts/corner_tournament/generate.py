"""ONE fixed semantic group, four antithetic source-sensitive directions, not gain-grid."""
import json,sys,struct,hashlib
from pathlib import Path
import torch
if not torch.__version__.startswith("2.15."):
 raise RuntimeError("This reproduction expects Torch 2.15.x; requalify ordered native parity for another version")
torch.set_num_threads(1)
def f32(x):return struct.unpack('<f',struct.pack('<f',x))[0]
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
p=Path(sys.argv[1]);out=Path(sys.argv[2]);data=json.loads(p.read_text());g=data['graph'];T=lambda x:torch.tensor(x,dtype=torch.float32)
# Ordered native f32 accumulator parity for original common-base replay.
family={x['family']:x['bias']for x in data['families']};reconstructed=[]
for c in data['candidates']:
 log=T(family[c['family']])+T(c['innate_bias']);m=T(0.);cg=T(0.)
 for e in g:
  if e['family']!=c['family']or e['head']=='SpeechPayload':continue
  v=T(c['decoder_inputs'][e['lane']])*(T(e['policy'])+T(e['offset']))
  if e['head']=='ActionCandidate':log=log+T(e['activation'])*v
  elif e['head']=='MemoryContext':m=m+v
  elif e['head']=='CognitiveContext':cg=cg+v
 log=log+torch.clamp(m,-data['memory_bound'],data['memory_bound']);log=log+torch.clamp(cg,-data['cognitive_bound'],data['cognitive_bound'])
 reconstructed.append(bits(log.item()))
assert reconstructed==data['logit_bits'],('native_ordered_torch_parity',reconstructed,data['logit_bits'])
variants=[{'id':'baseline','basis':'none','sign':0,'edits':[],'weight_edits_bits':[]}];directions=[]
# Derivative of Approach-Avoid score contrast with this semantic input set to1.
# All other lane inputs zero only for numerical direction analysis, NEVER actorinput.
# One fixed unit contrast amplitude. No candidate kind/target or outcome feedback.
for name,lane in [('distance',2),('chemical_1',16),('forward_cos',1),('signed_bearing_sin',0)]:
 es=[e for e in g if e['head']=='ActionCandidate'and e['family']in ('Approach','Avoid')and e['lane']==lane]
 assert es and set(e['family']for e in es)=={'Approach','Avoid'}
 w=torch.tensor([e['policy'] for e in es],dtype=torch.float32,requires_grad=True)
 acts=T([e['activation']for e in es]);signs=T([1 if e['family']=='Approach'else -1 for e in es])
 contrast=torch.sum(w*acts*signs);contrast.backward();jac=w.grad.detach();norm2=torch.dot(jac,jac)
 assert norm2.item()>1e-8,('degeneratebasis',name)
 delta=jac/norm2 # exactly ONE declared logitcontrast unit at analyzed sourceactivity
 assert torch.max(torch.abs(delta)).item()<64,('unboundedproposal',name)
 directions.append({'basis':name,'lane':lane,'coordinates':[e['index']for e in es],'source_activations':[e['activation']for e in es],'jacobian':jac.tolist(),'norm_squared':norm2.item(),'delta':delta.tolist(),'unit_input_contrast_effect':torch.dot(jac,delta).item()})
 for sign,label in [(1,'plus'),(-1,'minus')]:
  edits=[]
  for e,d in zip(es,delta.tolist()):
   value=f32(f32(e['policy'])+f32(sign*d))
   if bits(value)!=e['policy_bits']:edits.append({'index':e['index'],'old_bits':e['policy_bits'],'new_bits':bits(value),'new_value':value,'source':e['source'],'head':e['head'],'family':e['family'],'lane':lane})
  assert len(edits)>=2
  variants.append({'id':name+'-'+label,'basis':name,'lane':lane,'sign':sign,'edits':edits,'weight_edits_bits':[[e['index'],e['new_bits']]for e in edits]})
assert len(variants)==9 and len({json.dumps(v['weight_edits_bits'])for v in variants})==9
result={'status':'FROZEN_BEFORE_BODY_RANKING','common_base':'externally supplied retained tick626 g0 + Inspect valence0.4 common base for all9','original_context_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'torch_version':torch.__version__,'device':'cpu','native_ordered_f32_logits_exact':True,'amplitude':'one unit Approach-minus-Avoid contrast at actual common-base sourceactivity, per unit semantic input; fixed before outcomes, no sweep','sensitivity_only_not_trajectory_prediction':True,'physical_semantics':{'0':'bearing_sin','1':'bearing_cos','2':'distance','16':'chemical[1] (GroundedObjectSlots); not contact, which is lane18'},'directions':directions,'variants':variants}
with out.open('x')as f:json.dump(result,f,indent=2);f.write('\n')
print(json.dumps({'variants':len(variants),'edit_counts':[len(v['edits'])for v in variants],'parity':True}))
