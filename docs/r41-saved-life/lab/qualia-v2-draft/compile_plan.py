#!/usr/bin/env python3
"""Deterministic draft plan export: no active config writes, launches, or fixture authorization."""
from pathlib import Path
import itertools as X
from hashlib import sha256
import json
D=Path(__file__).resolve().parent
def read(n):return json.loads((D/n).read_bytes())
def compact(d):return json.dumps(d,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
V=read('qualia-v2.json'); F=read('fixture-recipes.json'); W=read('woodland-height-recipes.json')
runs=[]
def add(task,input,part,expected):
    manifest={
       'schema':'qualia.v2.fixture.preflight-draft.v1','case_id':task['case_id'],'condition':part,
       'admission_state':'not-materialized-do-not-run','authority':F['authentic_openings'],'expected_world_only':input,
       'expected_observer_only':expected,
       'max_decisions':task['max_accepted_decisions'],'max_world_ticks': min(8192,task['max_accepted_decisions']*8),
       'allowed_interventions':['control','digest-bound_direct_data_patch'],
       'pair_verification':'currents identical before native control or named immutable direct edits; separately recorded public setup/ordinary masks and same native sampler stream; compare only paid execution',
       'admission_gate':task['admission_gate']
    }
    runs.append({'trial_sha256':sha256(compact(manifest)).hexdigest(),**manifest})
for t in F['tasks']:
 k=t['case_id']; b=t['conditions']; L=F['layouts'][t['layout']]
 if k.startswith('v2-l'):
  choices=[('positive',a)for a in b['command_tokens']]+[('foil',a)for a in b['foils']]
  for m,o,(p,c) in X.product(b['mirror_left_sign'],b['insertion_order'],choices):
   add(t,{**L,'one_native_player_emit':c if c else None,'observation_idx_window_after_emit':[0,1],
          'mirror_left_sign':m,'object_insertion_order':o},p,
         {'reference_tokens':c,'max_initial_decisions':16 if k.startswith('v2-l1') else 64,
          'must_bind_paid_target_to_observed_referent':True,'count_matching_autonomous_foil_rate':p=='foil'})
 elif k.startswith('v2-d'):
  for subtype in b['subconditions']:
   add(t,{**L,'interoceptive_stratum':subtype,'thresholds_H_and_F':[.65,.25],
          'off_drive_ceiling':.35,'spontaneous_payload_only':k=='v2-d1-drive-names','one_native_player_emit':None},
        'low-or-relieved-foil' if 'low'in subtype or 'after'in subtype else 'positive',
        {'advance_ordinary_native_preconditioning_only':True,'critical_cue_is_real_hunger_or_exertion_fatigue_not_wakefulness':True,
         'record_rest_and_sleep_lifecycle_separately':True})
 elif k.startswith('v2-s1'):
  for m,o,e in X.product(b['mirror_left_sign'],b['insertion_order'],b['conditions']):
   raw={'ordinary acute contact':[.16,.84],'matched harmless surface':[0.,.02],
        'warm edible foil':[0.,.21],'cold-but-painful negative-control':[.16,-.84]}[e]
   add(t,{**L,'mirror_left_sign':m,'object_insertion_order':o,'variant_effect':e,
          'after_ordinary_surface_type_substitution':{'native_hazard_pain':raw[0],'surface_temperature':raw[1],
            'trial_surface_kind':{'warm edible foil':'Food','matched harmless surface':'Obstacle'}.get(e,'Hazard')}},
       e,{'protective_exit_primitives_permitted':[102],'displacement_min_m':.08,
          'exit_deadline_decisions':8,'consecutive_no_contact':3,'after_exit_injury_cap':.05,
          'food_must_be_different_from_hazard':True,'record_precaution_before_first_harm':True})
 elif k.startswith('v2-s2'):
  for m,o,e in X.product(b['mirror_left_sign'],b['insertion_order'],b['conditions']):
   add(t,{**L,'mirror_left_sign':m,'object_insertion_order':o,'variant_effect':e},e,
       {'requires_real_source_actor_and_paid_effect_join':True,'refrain_in_correctly_occluded_or_neutral_foils':True})
 else:
  for wm,o,typ in X.product(b['layouts'],b['object_insertion_order'],b['seasonal_asset_variants']):
   assert wm in W['terrain']; geom=W['terrain'][wm]
   add(t,{**L,'woodland_recipe':wm,'woodland_recipe_canonical_sha256':sha256(compact(geom)).hexdigest(),
          'object_insertion_order':o,'production_trunk_variant':typ},wm,
       {'gate_order_minimum':2,'gate_direction':'positive-forward','fixed_goal_distance_m_max':.75,
        'uneven_vertical_span_m_min':.0 if 'flat'in wm else .18,'checked_real_same_target_positive_meal':True})
assert len(runs)==len({m['trial_sha256']for m in runs}) and all(r['admission_state']=='not-materialized-do-not-run' for r in runs)
data={'schema':'qualia-v2-detached-execution-plan-draft.1','card_sha256':sha256((D/'qualia-v2.json').read_bytes()).hexdigest(),
      'native_run_permitted':False,'published_base_promotions_unchanged':True,
      'same_runner_required':True,'materialized_trials':0,'catalog':runs}
b=(json.dumps(data,sort_keys=True,indent=2,allow_nan=False)+'\n').encode()
with (D/'draft-execution-plan.json').open('xb') as f:f.write(b)
print(len(runs),sha256(b).hexdigest())
