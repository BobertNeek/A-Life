"""Synthetic scalar and bounded-pipe tests only. No native process or existing life is read."""
import ast,copy,json,os,unittest
from pathlib import Path
from unittest.mock import patch
import report_contract as c
import bounded_runner as r
S='0'*64
SOURCE={'source_tree':'0'*40,**{key:S for key in ('source_closure_sha256','executable_sha256','plan_sha256','common_patch_sha256','world_patch_sha256')}}
BINDING={'old_world':SOURCE.copy(),'new_world':SOURCE.copy()}
def owners(h='nobody',s='nobody'):
 return [dict(target=role,owner=owner,active=owner not in ('missing','consumed'),mode='activity' if owner=='self' else 'none') for role,owner in [('h',h),('s',s)]]
def initial(seed=2026112301):
 return dict(asset_sha256=c.ASSET,**{key:S for key in ('founder_optimizer_sha256','full_initial_checkpoint_sha256','initial_body_biology_sha256','initial_scene_sha256')},sampling_seed=seed,sampling_counter=0,actual_world_tick=600)
def blank_case(cid='s1_old_world'):
 seed,world=c.CASES[cid];rows=[dict(primary_tick=i,world_tick_before=599+i,world_tick_after=600+i,sampling_counter_after=0,sleeping=True,sleep_consolidations=0,
  boundary_owners=owners(),alive=True,energy=.4,health=1.,injury=0.,active_memory_count=0,staged_memory_count=0,decisions=[]) for i in range(1,257)]
 return dict(schema_version=1,case_id=cid,seed=seed,world=world,initial=initial(seed),source=BINDING[world].copy(),rows=rows,stop='tick_budget',founder_optimizer_unchanged=True,checkpoint_roundtrip_exact=True,primary_next_checkpoint_tested=False)
def encoded(x):
 raw=c.canonical(x);return raw,c.canonical(dict(case_id=x['case_id'],original_summary_sha256=c.sha(raw),original_summary_byte_count=len(raw)))
def validate(x):return c.validate_case(*encoded(x),BINDING)
def candidate(target='s'):
 return dict(index=0,primitive=210,family=0,target=target,active=True,owner='nobody',mode='none',contact=True,contact_reachable=True,grasp_reachable=True,held_count=1,
  actor_alive=True,effector=True,target_sources=0,family_sources=0,target_confidence=0.,family_confidence=0.,family_valence=0.,source_sequence=None,source_exact_ingest=False)
def transfer(target,kind):return dict(target=target,kind=kind,previous_owner='nobody' if kind=='acquired' else 'self',new_owner='self' if kind=='acquired' else 'nobody')
def channel():return dict(slot=2,candidate=0,primitive=210,target='s',phase='preparation',executed=True,failure='none',contact='touch',physical_target='s',event='atomic_swap',transfers=[transfer('h','placed'),transfer('s','acquired')])
def decision(ch=None):return dict(sequence=1,sampling_counter=0,actual_replay_equal=True,pain=0.,pre_energy=.4,aversive_harm=0.,experienced_valence=0.,owners_before=owners('self','nobody'),owners_after=owners('nobody','self'),candidates=[candidate()],channels=[ch or channel()],physical_meals=[False,False],ingest_episode_active=[False,False],ingest_episode_staged=[False,False])
def case_with_decision(d=None):
 d=copy.deepcopy(d or decision());d.update(sequence=2,sampling_counter=1);held=next(x['target'] for x in d['owners_before'] if x['owner']=='self')
 ch=channel();ch.update(target=held,physical_target=held,event='acquired',transfers=[transfer(held,'acquired')]);setup=decision(ch);setup.update(owners_before=owners(),owners_after=d['owners_before'],candidates=[candidate(held)])
 x=blank_case();x['rows'][0].update(decisions=[setup],sleeping=False,sampling_counter_after=1,boundary_owners=setup['owners_after'])
 x['rows'][1].update(decisions=[d],sleeping=False,sampling_counter_after=2,boundary_owners=d['owners_after'])
 for row in x['rows'][2:]:row.update(sampling_counter_after=2,boundary_owners=d['owners_after'])
 return x
class ContractTests(unittest.TestCase):
 def bad(self,x,gate=None):
  with self.assertRaises(c.Reject) as e:validate(x)
  if gate:self.assertEqual(e.exception.gate,gate)
 def test_no_contact_not_avoidance(self):self.assertFalse(c.metrics(validate(blank_case()))['h_held_s_eat_opportunity_observed'])
 def test_swap_not_meal(self):
  m=c.metrics(validate(case_with_decision()));self.assertEqual((m['h_to_s_atomic_swaps'],m['safe_meals']),(1,0))
 def test_placed_only_and_released_remain_distinct(self):
  ch=channel();ch.update(event='placed_only',physical_target='h',transfers=[transfer('h','placed')]);self.assertEqual(c.actual_event(ch),'placed_only')
  ch.update(event='released',target='h',transfers=[transfer('h','released')]);self.assertEqual(c.actual_event(ch),'released')
 def test_reversed_swap_rejected(self):
  x=case_with_decision();x['rows'][1]['decisions'][0]['channels'][0]['transfers'].reverse();self.bad(x,'transfer_order')
 def test_failed_pickup_must_keep_hold(self):
  ch=channel();ch.update(event='failed',executed=False,failure='missing_affordance',contact='none',transfers=[]);d=decision(ch);d['owners_after']=owners('self','nobody');validate(case_with_decision(d))
  x=case_with_decision();x['rows'][1]['decisions'][0]['channels'][0]['failure']='missing_affordance';self.bad(x,'failed_transfer')
 def test_physical_meal_without_episode_accepted_separately(self):
  ch=channel();ch.update(event='consumed',phase='completed',contact='consumed',transfers=[transfer('s','consumed')]);d=decision(ch);d.update(owners_before=owners('nobody','self'),owners_after=owners('nobody','consumed'),physical_meals=[False,True])
  m=c.metrics(validate(case_with_decision(d)));self.assertEqual(m['safe_meals'],1);self.assertEqual(m['physical_episode_coverage_active'],[0,0])
 def test_fabricated_episode_rejected(self):
  x=case_with_decision();x['rows'][1]['decisions'][0]['ingest_episode_active'][1]=True;self.bad(x,'coverage')
 def test_unknown_recall_source_rejected(self):
  x=case_with_decision();x['rows'][1]['decisions'][0]['candidates'][0].update(source_sequence=20,source_exact_ingest=True,family_sources=1);self.bad(x,'recall_provenance')
 def test_rng_and_clock_are_real(self):
  x=case_with_decision();x['rows'][1]['decisions'][0]['sampling_counter']=0;self.bad(x,'sampling')
  x=blank_case();x['rows'][1]['world_tick_after']+=1;self.bad(x,'clock')
 def test_unknown_sensitive_fields_rejected(self):
  x=case_with_decision();x['rows'][1]['decisions'][0]['candidates'][0]['weights']=[];self.bad(x,'unknown_or_missing_field')
 def test_duplicate_nonfinite_json_rejected(self):
  for raw,schema in [(b'{"a":1,"a":2}',c.obj(a=c.integer(0,3))),(b'NaN',c.UNIT)]:
   with self.assertRaises(c.Reject):c.parse(raw,schema)
 def test_original_completion_bytes_bound(self):
  raw,completion=encoded(blank_case())
  with self.assertRaises(c.Reject) as e:c.validate_case(raw+b' ',completion,BINDING)
  self.assertEqual(e.exception.gate,'completion_hash')
 def test_safeguard_stops_without_extension(self):
  x=blank_case();x['rows'][1]['health']=.69;self.bad(x,'continued_after_stop');x['rows']=x['rows'][:2];x['stop']='health';validate(x)
 def test_f32_threshold_matches_native(self):
  x=blank_case();x['rows'][0]['health']=c.f32(.70);validate(x)
 def test_horizon_cannot_extend(self):
  x=blank_case();x['rows'].append(copy.deepcopy(x['rows'][-1]));self.bad(x,'bound')
 def test_source_identity_bound(self):
  x=blank_case();x['source']['executable_sha256']='1'*64;self.bad(x,'source_binding')
class TightenedContractTests(unittest.TestCase):
 def test_transfers_only_on_manipulation(self):
  for slot in (0,1,3,4):
   x=case_with_decision();x['rows'][1]['decisions'][0]['channels'][0]['slot']=slot
   with self.assertRaises(c.Reject) as e:validate(x)
   self.assertEqual(e.exception.gate,'transfer_channel')
 def test_unreceipted_boundary_acquisition_rejected(self):
  x=blank_case();x['rows'][0]['boundary_owners']=owners('self','nobody')
  with self.assertRaises(c.Reject) as e:validate(x)
  self.assertEqual(e.exception.gate,'unreceipted_boundary_transfer')
 def test_unreceipted_after_decision_boundary_rejected(self):
  x=case_with_decision();x['rows'][1]['boundary_owners']=owners('self','nobody')
  with self.assertRaises(c.Reject) as e:validate(x)
  self.assertEqual(e.exception.gate,'unreceipted_boundary_transfer')
 def test_transfer_requires_execution_and_contact(self):
  for key,value in [('executed',False),('executed',None),('contact','none')]:
   x=case_with_decision();x['rows'][1]['decisions'][0]['channels'][0][key]=value
   with self.assertRaises(c.Reject) as e:validate(x)
   self.assertEqual(e.exception.gate,'schema' if key=='executed' and value is None else 'transfer_execution')
 def test_terminal_detachment_allowed_but_living_release_rejected(self):
  x=case_with_decision();x['rows']=x['rows'][:2];x['rows'][1]['boundary_owners']=owners();x['rows'][1]['alive']=False;x['stop']='death';validate(x)
  x['rows'][1]['alive']=True
  with self.assertRaises(c.Reject) as e:validate(x)
  self.assertEqual(e.exception.gate,'unreceipted_boundary_transfer')
 def test_ecology_regeneration_is_only_unreceipted_change(self):
  ch=channel();ch.update(event='consumed',phase='completed',contact='consumed',transfers=[transfer('s','consumed')]);d=decision(ch);d.update(owners_before=owners('nobody','self'),owners_after=owners('nobody','consumed'),physical_meals=[False,True])
  x=case_with_decision(d)
  for row in x['rows'][2:]:row['boundary_owners']=owners()
  validate(x)

class LedgerAndPipeTests(unittest.TestCase):
 def prepared(self,l,cid):
  l.dispatch(cid);seed,world=c.CASES[cid];l.prepared_case(cid,dict(case_id=cid,initial=initial(seed),source=BINDING[world]),BINDING)
 def accept(self,l,cid):l.started.add(cid);l.note_returned(cid);l.accept(cid,*encoded(blank_case(cid)),BINDING)
 def test_pair_mismatch_blocks_both(self):
  l=r.Ledger();self.prepared(l,'s1_old_world');self.prepared(l,'s1_new_world');l.prepared['s1_new_world']['initial']['initial_scene_sha256']='1'*64
  with self.assertRaises(c.Reject):l.start('s1_old_world','s1_new_world')
 def test_start_request_is_not_observed_primary_start(self):
  l=r.Ledger();self.prepared(l,'s1_old_world');self.prepared(l,'s1_new_world');l.start('s1_old_world','s1_new_world');s=l.sets();self.assertEqual(s['start_requested'],['s1_old_world']);self.assertEqual(s['primary_started'],[])
 def test_no_two_primary_or_retry(self):
  l=r.Ledger();self.prepared(l,'s1_old_world');self.prepared(l,'s1_new_world');l.start('s1_old_world','s1_new_world')
  with self.assertRaises(c.Reject):l.start('s1_new_world','s1_old_world')
  with self.assertRaises(c.Reject):l.dispatch('s1_old_world')
 def test_all_four_accounted(self):
  l=r.Ledger()
  for a,b in (c.ORDER[:2],c.ORDER[2:]):
   self.prepared(l,a);self.prepared(l,b)
   for cid,peer in ((a,b),(b,a)):l.start(cid,peer);self.accept(l,cid)
  self.assertEqual(l.sets()['validated'],list(c.ORDER));self.assertEqual(l.sets()['unknown'],[])
 def test_rejected_and_unknown_are_not_invented_results(self):
  l=r.Ledger();self.prepared(l,'s1_old_world');self.prepared(l,'s1_new_world');l.start('s1_old_world','s1_new_world');l.started.add('s1_old_world');l.note_returned('s1_old_world')
  x=blank_case();x['rows'][0]['private_state']=[]
  with self.assertRaises(c.Reject):l.accept('s1_old_world',*encoded(x),BINDING)
  s=l.sets();self.assertEqual(s['native_returned'],['s1_old_world']);self.assertEqual(s['rejected'],['s1_old_world']);self.assertEqual(s['unknown'],['s1_new_world']);self.assertEqual(s['validated'],[])
 def test_exact_hard_pipe_budget(self):
  a,b=os.pipe();budget=r.ByteBudget(4)
  try:
   os.write(b,b'abcdXYZ');first=os.read(a,budget.read_size());budget.accept(first);self.assertEqual(first,b'abcd');self.assertEqual(budget.used,4);self.assertEqual(budget.read_size(),1)
   sentinel=os.read(a,budget.read_size())
   with self.assertRaises(c.Reject):budget.accept(sentinel)
   self.assertEqual(budget.used,4)
  finally:os.close(a);os.close(b)
 def test_aggregate_budget_shared_across_streams(self):
  budget=r.ByteBudget(4);budget.accept(b'ab');budget.accept(b'cd')
  with self.assertRaises(c.Reject):budget.accept(b'X')
 def test_no_raw_stderr_file_exists_in_source(self):
  source=Path(r.__file__).read_text();ast.parse(source);self.assertIn('stderr=subprocess.PIPE',source);self.assertNotIn('.stderr.private',source);self.assertIn('budget.accept(chunk)',source)
 def test_reserved_original_plan_hash(self):self.assertEqual(c.sha(Path(__file__).with_name('PLAN.json').read_bytes()),r.PLAN_SHA)
 def test_native_start_without_marker_is_conservative_attempt(self):
  l=r.Ledger();l.dispatch('s1_old_world');l.native_process_started('s1_old_world');s=l.sets()
  self.assertEqual(s['attempted'],['s1_old_world']);self.assertEqual(s['construction_started'],[]);self.assertEqual(s['unknown'],['s1_old_world'])
 def test_public_cap_fallback_preserves_every_validated_summary(self):
  class FakePath:
   def __init__(self,s='job'):self.s=s
   def __truediv__(self,x):return FakePath(self.s+'/'+x)
   def mkdir(self,**kw):pass
   def __str__(self):return self.s
  l=r.Ledger()
  for a,b in (c.ORDER[:2],c.ORDER[2:]):
   self.prepared(l,a);self.prepared(l,b)
   for cid,peer in ((a,b),(b,a)):l.start(cid,peer);self.accept(l,cid)
  saved={}
  with patch.object(r,'PUBLIC_CAP',20000),patch.object(r,'create',side_effect=lambda path,raw:saved.__setitem__(str(path).split('/')[-1],raw)):
   r.finalize(FakePath(),l,BINDING,'complete',b'source',b'world',{},0)
  self.assertEqual(len(json.loads(saved['summary.json'])['cases']),4)
  self.assertEqual(json.loads(saved['rows.json'])['omitted_validated_cases'],list(c.ORDER))
  self.assertLessEqual(sum(map(len,saved.values())),20000)
 def test_native_output_policy_lowers_core_and_sets_private_umask(self):
  with patch.object(r.resource,'setrlimit') as limit,patch.object(r.os,'umask') as mask:
   r.restrict_process_outputs();limit.assert_called_once_with(r.resource.RLIMIT_CORE,(0,0));mask.assert_called_once_with(0o077)
 def test_unknown_case_not_exported(self):
  class FakePath:
   def __init__(self,s='job'):self.s=s
   def __truediv__(self,x):return FakePath(self.s+'/'+x)
   def mkdir(self,**kw):pass
   def __str__(self):return self.s
  l=r.Ledger();l.dispatch('s1_old_world');saved={}
  with patch.object(r,'create',side_effect=lambda path,raw:saved.__setitem__(str(path).split('/')[-1],raw)):r.finalize(FakePath(),l,BINDING,'runtime_unknown',b'source',b'world',{},4)
  self.assertEqual(set(saved),r.PUBLIC_FILES);self.assertEqual(json.loads(saved['rows.json'])['cases'],[]);self.assertFalse(json.loads(saved['checks.json'])['raw_stderr_persisted'])
if __name__=='__main__':unittest.main()
