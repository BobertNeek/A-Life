#!/usr/bin/env python3
"""Verify and score native rows on the frozen disclosed-29 card. No process launch or ledger write."""
from __future__ import annotations
from collections import defaultdict
from fractions import Fraction
from hashlib import sha256
import argparse
from copy import deepcopy
import json
import math
from pathlib import Path
import struct
from datetime import datetime

from run_frozen_saved_life_v1 import LAB, CARD, CARD_PIN, ELF_PIN, digest, read, emit, canonical

R41_CURRENT='e785e79c581dbc29bb9e4b66ab2b3ae63d31a16db9c44b262753dfe319fbe841'
FIRST_CUT=3074
ZERO_TRACE=LAB/'private/verified-zero-retention.json'
ZERO_TRACE_SHA='9077aa77d9b2d66d5f47f9c602d641aadfb315b50a07cc37e332c19af94a10f8'
CASE_PROOF='case-measurement.json'
PRESSURE=0
NATIVE_FIELDS=('evaluation_nonce','max_decisions','max_world_ticks','max_output_bytes','deadline_unix_seconds','finalization_deadline_unix_seconds')

def require(x,msg):
    if not x:raise ValueError(msg)

def digest_value(x):return sha256(canonical(x)).hexdigest()

def range_change(row,p):
    def point(x):return [x[k] for k in ('x','y','z')]
    position=point(p)
    return math.dist(point(row['observer_pose_after'][0]),position)-math.dist(point(row['observer_pose_before'][0]),position)

def score_card():
    require(digest(CARD)==CARD_PIN,'frozen card byte mismatch')
    return read(CARD)

def cut_common(row):
    ob=row['observer_policy_diagnostics']; b=ob['behavior']; h=row['observer_host_pressure_control']['samples'][0][1]
    return {'pose_before':row['observer_pose_before'],'frame_digest':row['frame_digest'],
            'sealed_sensory':row['observer_sealed_sensory'],'health_needs':row['physiology']['before'],
            'heard':row['observed_native_heard_tokens'],'candidates':ob['candidates'],
            'policy_inputs_masks_and_sampler':{k:b[k] for k in ('decoder_inputs','decoder_input_stride','representative_mask','motor_masks','forced_motor_slots','sampling')},
            'pressure_execution':{k:h[k] for k in ('level','microsteps','enabled_route_ids')},
            'pressure_inputs':{k:h['pressure'][k] for k in ('completed_gpu_time_ns','queue_depth','logical_heap_pressure_q16','brain_atp_fraction_q16')}}

def parse_current(p,want):
    raw=p.read_bytes()
    require(sha256(raw).hexdigest()==want,'checkpoint digest mismatch')
    require(raw[:16]==b'ALIFE-CURRENT-1\n' and struct.unpack_from('<Q',raw,16)[0]==len(raw)-56 and sha256(raw[56:]).digest()==raw[24:56],
            'checkpoint Current closure mismatch')
    return json.loads(raw[56:])

def measure(work:Path, case:str, card:dict):
    entry=card['cases'][case]
    scen=entry['scenario']
    require(digest_value(scen)==entry['scenario_sha256'],'scenario hash')
    m=read(work/'manifest.json')
    a=read(work/'authority.json')
    p=read(work/'process.json')
    n=work/'native'
    native=n/'retention.json'; r=read(native)
    source_sha=m['source_current_sha256']
    patch=m['intervention']; direct=patch.get('edit')
    mode=patch['kind']
    require(mode in ('control','direct_weights'),'unsupported intervention')
    require(p['exit_code']==0 and p['group_reaped'] is True and not p['stop'] and p['parent_pid']!=p['child_pid'],'missing native process closure')
    require(datetime.fromisoformat(p['started_at_utc']).utcoffset() is not None and datetime.fromisoformat(p['finished_at_utc'])>=datetime.fromisoformat(p['started_at_utc']),'invalid process chronology')
    require(a['evaluator_sha256']==p['evaluator_sha256']==ELF_PIN and a['rubric_sha256']==CARD_PIN==p['rubric_sha256'],'wrong runner/rubric pin')
    require(a['scenario_sha256']==p['scenario_sha256']==entry['scenario_sha256'] and a['case']==p['case']==case,'wrong frozen scenario authority')
    require(a['native_manifest_sha256']==p['native_manifest_sha256']==digest(work/'manifest.json') and p['native_receipt_sha256']==digest(native),'manifest/receipt bytes drifted')
    require(source_sha==a['source_current_sha256']==p['source_current_sha256']==r['source_sha256']==digest(work/'Current.alife-checkpoint'),'wrong mature source bytes')
    require(r['source_world_tick']==FIRST_CUT and r['organism_id']==scen['starting_organism_id']==m['owner'],'wrong maturity or body')
    require(digest(ZERO_TRACE)==ZERO_TRACE_SHA,'authentic reference zero-cut was swapped')
    reference=read(ZERO_TRACE)
    if source_sha==R41_CURRENT:
        for k in ('policy_version','actor_optimizer_step','value_optimizer_step','sampling_before'):
            require(r[k]==reference[k],f'R41 checkpoint startup drifted: {k}')
    require(r['status']=='finished' and r['stop_reason']=='decision_limit' and r['error'] is None,'case did not close native full horizon')
    for k in ('measurement_valid','coverage_complete','gpu_restored','neural_execution','prior_remained_off','source_unchanged','saved_policy_base_unchanged','software_simulation'):
        require(r[k] is True,f'no native {k}')
    require(r['prior_enabled'] is False and r['qualified'] is False and r['hardware_gpu_qualification'] is False,'not intended prior-off/software-GPU run')
    require(r['offline_actor_updates']==r['offline_value_updates']==r['new_teacher_cues_emitted_by_evaluator']==0 and r['offline_optimizer_constructed'] is False and r['future_teacher_emitter_constructed'] is False,'unexpected training/emitter')
    require(r['existing_teacher_cues']['object_ids']==[] and r['existing_teacher_cues']['utterances']==0,'teacher present')
    require(r['published_case']==m['card_case']==case and r['published_case_registry']==scen['registry'],'registry mismatch')
    require(r['test_host_pressure_control']==scen['host_pressure'] and r['test_host_pressure_control']['applied_gpu_time_ns']==PRESSURE,'pressure mismatch')
    require(r['software_adapter']==reference['software_adapter'] and {k:v for k,v in r['hardware'].items() if k!='generation'}=={k:v for k,v in reference['hardware'].items() if k!='generation'},'software adapter mismatch')
    exp=r['retained_experiment']; ident=exp['identity']
    require(exp['evaluation_only'] is True and ident['intervention']==mode and ident['manifest_sha256']==digest(work/'manifest.json'),'native authority manifest mismatch')
    for k in ('owner','card_case','source_current_sha256','run_id','scenario','schema'):require(ident[k]==m[k],f'native field mismatch: {k}')
    for k in NATIVE_FIELDS:require(r['request'][k]==m[k],f'native request mismatch: {k}')
    nav=scen['navigation_task']; setup=scen['exact_released_visible_opening']
    require(r['request']['schema']==3 and r['request']['navigation_task']==nav and r['request']['direct_weights']==direct,'effective typed request mismatch')
    require(m['max_decisions']==entry['decisions']==scen['decision_limit'] and m['max_world_ticks']==scen['world_tick_limit'] and m['evaluation_nonce']==scen['evaluation_nonce'],'frozen horizon/nonce mismatch')
    nav_setup=r.get('navigation_task_setup')
    if nav is None:
        require(nav_setup is None and r.get('published_case_setup')==setup,'released scene setup changed')
    else:
        require(r.get('published_case_setup') is None and nav_setup and nav_setup['spec']==nav,'released navigation spec not used')
    said=r.get('published_case_player_utterance'); hearing=scen['authentic_player_tokens_per_cut']; spoken=bool(hearing)
    if spoken:
        require(said['source_kind']=='Player' and said['speaker_id'] is None and said['teacher_channel'] is None
                and said['addressee']==m['owner'] and said['emitted_tick']==FIRST_CUT and said['expires_after_tick']==FIRST_CUT+1 and said['tokens']==hearing[0],
                'missing exact addressed real Player cue')
        require(r['published_case_software_integrity'] is True and r['published_case_external_suggestion'] is True and r['cue_free'] is False and r['software_simulation_integrity_valid'] is False,'invalid Player expression mode')
    else:
        require(said is None and r['published_case_external_suggestion'] is False and r['cue_free'] is True and r['software_simulation_integrity_valid'] is True,'false Player cue in silence case')
    require(r['rows_file']=='rows.jsonl' and r['final_endpoint']['path']=='final.alife-checkpoint','unexpected native artifact paths')
    raw=n/'rows.jsonl'
    require(digest(raw)==r['rows_sha256'],'native row bytes changed')
    end=r['final_endpoint']
    require(end['world_tick']==FIRST_CUT+r['world_ticks'] and all(end[k] is True for k in ('same_individual','actor_value_Adam_unchanged','private_world_memory_neural_and_sampling_preserved')) and end['offline_trained_rows']==end['offline_updates']==0,'mature final endpoint changed authorities')
    require(digest(n/'final.alife-checkpoint')==end['sha256'],'final checkpoint digest mismatch')
    if mode=='control':
        require(not r.get('direct_weight_calibration') and end['source_current_sha256']==source_sha,'control received a patch')
        effective_source=source_sha
        candidate_actor_plane=None
    else:
        cal=r['direct_weight_calibration']; edited=n/'CALIBRATED-PARENT.alife-checkpoint'
        require(cal['source_current_sha256']==source_sha and cal['changed_weights']==cal['requested_coordinates']==len(direct['edits']) and cal['optimizer_updates']==cal['mutable_words_written']==0 and cal['world_tick']==FIRST_CUT,'wrong typed direct calibration')
        for k in ('Adam_moments_ages_retained','critic_retained','same_individual','sampler_retained','PPO_policy_counter_unchanged'):
            require(cal[k] is True,f'candidate lost mature {k}')
        effective_source=cal['calibrated_current_sha256']
        require(digest(edited)==effective_source==end['source_current_sha256'],'wrong calibrated mature endpoint')
        baseline=parse_current(work/'Current.alife-checkpoint',source_sha)
        candidate=parse_current(edited,effective_source)
        old_body,new_body=(x['same_individual']['progress'] for x in (baseline,candidate))
        require(old_body['world_tick']==new_body['world_tick']==FIRST_CUT and
                old_body['organism_id']==new_body['organism_id']==m['owner'],
                'calibration bytes are not the same mature individual at zero cut')
        require(baseline['same_individual']['sampling_state']==candidate['same_individual']['sampling_state'],
                'calibrated Current skipped or changed the mature sampler')
        a,b=(x['handoff'] for x in (baseline,candidate))
        require(a['value']==b['value'] and a['policy_version']==b['policy_version'],
                'calibrated Current changed the critic or PPO counter')
        for field in ('config','stage_mask','optimizer_step','first_moment','second_moment','update_ages'):
            require(a['actor'][field]==b['actor'][field],f'calibrated Current changed {field}')
        x,y=(h['compiler_inputs'] for h in (a,b))
        rebound={'foundation_abi_selection','n2048_candidate_asset','canonical_digest'}
        require(x.keys()==y.keys() and all(x[k]==y[k] for k in x if k not in rebound),
                'calibrated Current changed compiler or live-branch authority')
        old_abi,new_abi=(deepcopy(c['foundation_abi_selection']) for c in (x,y))
        actor_words=lambda layer:[struct.unpack('<I',struct.pack('<f',w))[0] for w in layer]
        normalized=[]
        for abi,h,compiler in ((old_abi,a,x),(new_abi,b,y)):
            digest_word=abi["contract"]["weight_asset"].pop("digest")
            selected=compiler["n2048_candidate_asset"]
            require(set(selected)=={"manifest","weights","digest"},'selected weight asset schema drift')
            spec=deepcopy(selected["manifest"])
            asset_digest=spec["weight_asset"].pop("digest")
            require(digest_word==h["actor"]["source_foundation_digest"]==selected["digest"]==asset_digest,
                    'ABI/actor/selected asset digests disagree')
            require(spec["weight_asset"]["weight_count"]==len(selected["weights"])==32768 and
                    actor_words(h["actor"]["weights"])==actor_words(selected["weights"]),
                    'selected asset words differ from admitted weights')
            normalized.append(spec)
        require(old_abi==new_abi,'calibrated Current changed more than the expected ABI weight asset digest')
        require(normalized[0]==normalized[1] and cal['new_foundation_digest']==b['actor']['source_foundation_digest'] and
                cal['new_phenotype_hash']==b['actor']['phenotype_hash'],'narrow ABI rebind or native calibration closure changed')
        weights=lambda checkpoint:checkpoint['handoff']['actor']['weights']
        w32=lambda w:struct.unpack('<I',struct.pack('<f',w))[0]
        delta=[[i,w32(x)] for i,(x,y) in enumerate(zip(weights(candidate),weights(baseline),strict=True)) if w32(x)!=w32(y)]
        require(delta==sorted(direct['edits']),'calibrated candidate changed a different weight set')
        candidate_actor_plane=sha256(struct.pack('<32768f',*weights(candidate))).hexdigest()
    rows=[json.loads(line) for line in raw.read_bytes().splitlines()]
    require(len(rows)==r['captured_decisions']==entry['decisions'],'native rows incomplete')
    food_id=(setup.get('food') or {}).get('id') if setup else None
    nav_result=r.get('navigation_task_result')
    if nav_result:food_id=nav_result['target']
    acquired=[];meals=[];vocal=[]; seen_utterances=set()
    for i,s in enumerate(rows):
        tick=FIRST_CUT+i
        require(s['row']==i and s['organism']==m['owner'] and s['decision_tick']==tick and s['outcome_tick']==tick+1 and s['teacher_heard'] is False,'noncanonical live biological row')
        pctrl=s['observer_host_pressure_control']; sample=pctrl['samples']
        require(pctrl['scope']=='controlled_host_pressure_not_fixed_neural_work' and len(sample)==1 and sample[0][1]['pressure']['completed_gpu_time_ns']==0 and sample[0][1]['tick']==tick,'physical row pressure changed')
        heard=[x for x in s['observed_native_heard_tokens'] if x and x['source_kind']=='Player']
        expected=hearing[i] if spoken else []
        require([q['token_id'] for q in heard]==expected,'actual Player input did not match tuple times')
        for j,word in enumerate(heard):
            require(word['utterance_id']==said['utterance_id'] and word['addressee']==m['owner'] and word['speaker_id'] is None and word['teacher_channel'] is None and word['sequence_position']==j and word['source_position']==said['source_position'] and 0<word['confidence']<=1,'native Player word provenance changed')
        after=[u for u in s['observed_world_utterances_after'] if u['source_kind']=='Player']
        require(after==([said] if spoken and i==0 else []),'unapproved Player event or expiry')
        if food_id is not None:
            food=next((q for q in s['observer_food_ownership']['entities'] if q['target']==food_id),None)
        else:food=None
        if food and food['before'] and food['after'] and food['before']['owner']!=m['owner'] and food['after']['owner']==m['owner']:acquired.append(tick+1)
        actions=s['joint_physical']['channel_outcomes']
        for z in actions:
            x=z['execution_evidence']; nutrition=x['body_inputs']['nutrition']
            if food_id is not None and x['primitive']==210 and x['command_target']==food_id and x['succeeded'] and nutrition>0:
                require(food and food['before']['owner']==m['owner'] and food['after']['consumed'],'paid native bite lacked held-and-consumed evidence')
                meals.append([tick+1,food_id,nutrition])
        offers={q['target']['entity']:q['target']['position'] for q in s['observer_policy_diagnostics']['candidates'] if q['action_id']==101 and q.get('target') and q['target']['entity'] is not None}
        for word in s['observed_world_utterances_after']:
            if word['emitted_tick']<FIRST_CUT or word['source_kind']=='Player':continue
            require(word['source_kind']=='Creature' and word['speaker_id']==m['owner'],'unknown native utterance source')
            if word['utterance_id'] in seen_utterances:continue
            seen_utterances.add(word['utterance_id'])
            require(word['emitted_tick'] in (s['decision_tick'],s['outcome_tick'])
                    and any(z['execution_evidence']['primitive']==400 and z['execution_evidence']['succeeded'] and z['physical']['energy_cost']>0 for z in actions),
                    'word was not actually paid and uttered')
            actual_go=any(z['channel']=='Locomotion' and z['execution_evidence']['primitive']==101 and z['execution_evidence']['command_target']==food_id for z in actions)
            grounded=word['tokens']==[1] and food and food['before'] and not food['before']['consumed'] and food_id in offers and offers[food_id] is not None and actual_go and range_change(s,offers[food_id])<0
            vocal.append({'tick':word['emitted_tick'],'grounded':bool(grounded),'tokens':word['tokens']})
    atoms={}
    for atom in entry['atoms']:
        if atom.endswith(':intake') or atom.endswith(':safe-intake'):
            yes=any(acquired and acquired[0]<q[0] for q in meals)
        elif atom=='maze:formal':
            yes=nav_result['first_traversal_tick'] is not None and not nav_result['maze_route_invalid']
        elif atom=='care:alive':
            body=r['private_post_tick_body_observations']
            require(body and len(body)==r['world_ticks'] and body[0]['tick']==FIRST_CUT+1 and body[-1]['alive']==r['alive'] and setup['hazard']['contact'],'unverified native care clock or hazard')
            yes=body[-1]['alive']
        elif atom=='receptive:first-signed':
            row=rows[1]
            primitive=102 if 'retreat' in case else 101
            target=2 if case.endswith('food') else 4
            require(hearing[1]==[7 if primitive==102 else 6,1 if target==2 else 2],'receptive directed cue definition changed')
            move=next((z for z in row['joint_physical']['channel_outcomes'] if z['channel']=='Locomotion' and z['execution_evidence']['succeeded']),None)
            gap=range_change(row,setup['food' if target==2 else 'toy']['p'])
            yes=bool(move and move['physical']['contact']=='Moved' and move['physical']['energy_cost']>0 and move['physical']['target_entity']==target
                     and move['execution_evidence']['primitive']==primitive and move['execution_evidence']['command_target']==target and (gap>0 if primitive==102 else gap<0))
        elif atom=='grounded_expression:grounded-food':yes=any(u['grounded'] for u in vocal)
        elif atom=='grounded_expression:quiet-without-food':yes=not vocal
        else: raise ValueError(f'unknown frozen score atom {atom}')
        atoms[atom]=bool(yes)
    possible=Fraction(*entry['weight_rational'])
    earned=sum((Fraction(*entry['atoms'][k]) for k,v in atoms.items() if v),Fraction(0))
    return {'schema':'qualia.disclosed_native_case_evidence.v1','case_id':case,'scorecard_id':card['scorecard_id'],'rubric_sha256':CARD_PIN,'evaluator_sha256':ELF_PIN,
            'scenario_sha256':entry['scenario_sha256'],'decisions':len(rows),'score':float(100*earned/possible) if possible else 0.0,
            'score_atoms':atoms,'earned_rational':[earned.numerator,earned.denominator],
            'native_receipt_sha256':p['native_receipt_sha256'],'rows_sha256':r['rows_sha256'],'final_checkpoint_sha256':end['sha256'],
            'native_manifest_sha256':p['native_manifest_sha256'],'native_process_sha256':digest(work/'process.json'),
            'source_current_sha256':source_sha,'effective_current_sha256':effective_source,'intervention':mode,
            'candidate_actor_plane_sha256':candidate_actor_plane,
            'started_at_utc':p['started_at_utc'],'completed_at_utc':p['finished_at_utc'],
            'opening_cut_sha256':digest_value(cut_common(rows[0])),'independently_accepted':True,'real_game_inputs':True,
            'first_ownership_tick':next(iter(acquired),None),'paid_meals':meals,'paid_words':vocal}

def baseline(batch, published):
    card=score_card()
    evidence={}; totals=defaultdict(Fraction)
    leaves={}
    for case, entry in card['cases'].items():
        p=batch/case/CASE_PROOF
        d=read(p)
        require(d['schema']=='qualia.disclosed_native_case_evidence.v1' and d['case_id']==case and d['intervention']=='control','wrong baseline case identity')
        require(d['source_current_sha256']==d['effective_current_sha256']==R41_CURRENT and d['scorecard_id']==card['scorecard_id'],'wrong baseline mature source')
        require(d['rubric_sha256']==CARD_PIN and d['evaluator_sha256']==ELF_PIN and d['scenario_sha256']==entry['scenario_sha256'] and d['decisions']==entry['decisions'],'baseline case contract mismatch')
        require(d['independently_accepted'] is True and d['real_game_inputs'] is True,'baseline proof not accepted')
        require(d['native_receipt_sha256']==digest(p.parent/'native/retention.json') and d['rows_sha256']==digest(p.parent/'native/rows.jsonl') and d['native_process_sha256']==digest(p.parent/'process.json'),'baseline trace or receipt has changed')
        evidence[case]=d
        totals[entry['lesson']]+=Fraction(*d['earned_rational'])
        leaves[case]={k:d[k] for k in ('scenario_sha256','decisions','native_receipt_sha256','rows_sha256','score','final_checkpoint_sha256','native_process_sha256')}
        leaves[case]['case_evidence_sha256']=digest(p)
    earliest=min(d['started_at_utc'] for d in evidence.values())
    latest=max(d['completed_at_utc'] for d in evidence.values())
    report={'schema':'qualia.saved_life_baseline.v1','scorecard_id':card['scorecard_id'],'rubric_sha256':CARD_PIN,'evaluator_sha256':ELF_PIN,
            'base_id':'R41','source_current_sha256':R41_CURRENT,'earliest_started_at_utc':earliest,'measured_at_utc':latest,
            'score':float(sum(totals.values(),Fraction(0))),'cases':leaves,'actual_decisions':sum(x['decisions'] for x in leaves.values()),
            'native_case_count':len(leaves),'zero_point_controls_run':sum(entry['weight']==0 for entry in card['cases'].values()),
            'domain_scores':{k:float(v*5) for k,v in sorted(totals.items())},'independently_accepted':True,
            'historical_30_case_score_unchanged_and_not_asserted':True,'scorer_sha256':digest(Path(__file__))}
    emit(published,report)
    print(json.dumps({k:v for k,v in report.items() if k!='cases'},sort_keys=True),flush=True)


def admitted_patch(patch, state):
    claimed=read(patch)
    pin=digest(patch); patch_id=claimed['patch_id']
    accepted=[r for r in state['in_flight_patches']
              if r['patch_id']==patch_id and r['patch_sha256']==pin and r['status']=='unscored']
    require(len(accepted)==1 and Path(accepted[0]['patch_path']).resolve()==patch.resolve(),'unregistered or changed patch')
    return pin,patch_id


def paired_cases(pair, case, card, state, pin, patch_id):
    a,b=(measure(pair/k,case,card) for k in ('A','B'))
    aa,bb=(read(pair/k/'authority.json') for k in ('A','B'))
    require(aa['patch_id']==bb['patch_id']==patch_id and aa['patch_sha256']==bb['patch_sha256']==pin,'unmatched registered authority')
    require(a['intervention']=='control' and b['intervention']=='direct_weights','no actual A/B intervention isolation')
    require(a['opening_cut_sha256']==b['opening_cut_sha256'] and a['source_current_sha256']==b['source_current_sha256']==state['saved_life_controls']['full_saved_life_current_sha256'],'nonmatching actual initial cut')
    require(a['effective_current_sha256']==a['source_current_sha256'] and b['effective_current_sha256']!=a['effective_current_sha256'],'no distinct authenticated mature challenger')
    for side, evidence in (('A',a),('B',b)):
        p=pair/side/CASE_PROOF
        if not p.exists():emit(p,evidence)
        else:require(read(p)==evidence,'case verifier receipt bytes changed')
    return a,b


def prove_pair(pair, case, patch, output):
    card=score_card()
    state=read(LAB/'SURGERY_STATE.json')
    pin,patch_id=admitted_patch(patch,state)
    a,b=paired_cases(pair,case,card,state,pin,patch_id)
    out={'schema':'qualia.saved_life_completed_comparison.v1','scorecard_id':card['scorecard_id'],
         'rubric_sha256':CARD_PIN,'evaluator_sha256':ELF_PIN,'base_id':state['active_base_id'],
         'source_current_sha256':a['source_current_sha256'],'patch_id':patch_id,'patch_sha256':pin,
         'candidate_current_sha256':b['effective_current_sha256'],'case_id':case,
         'candidate_actor_sha256':b['candidate_actor_plane_sha256'],
         'scenario_sha256':a['scenario_sha256'],'decisions':a['decisions'],
         'completed_at_utc':max(a['completed_at_utc'],b['completed_at_utc']),
         'independently_accepted':True,'matched':True,'real_game_inputs':True,
         'base_native_receipt_sha256':a['native_receipt_sha256'],'candidate_native_receipt_sha256':b['native_receipt_sha256'],
         'base_rows_sha256':a['rows_sha256'],'candidate_rows_sha256':b['rows_sha256'],
         'opening_cut_sha256':a['opening_cut_sha256'],
         'original_case_atoms':{'base':a['score_atoms'],'candidate':b['score_atoms']},
         'paid_words':{'base':a['paid_words'],'candidate':b['paid_words']},
         'paid_meals':{'base':a['paid_meals'],'candidate':b['paid_meals']},
         'full_score':None}
    emit(output,out)
    print(json.dumps(out,sort_keys=True),flush=True)


def complete_pair_card(batch, patch, candidate_id, output):
    card=score_card()
    state=read(LAB/'SURGERY_STATE.json')
    gate=state['saved_life_controls']
    pin,patch_id=admitted_patch(patch,state)
    require(gate['active_card'].get('baseline') is not None,'fresh registered baseline required for a complete report')
    base_current=gate['full_saved_life_current_sha256']
    require(isinstance(candidate_id,str) and candidate_id and candidate_id!=state['active_base_id'],'candidate ID must be distinct')
    cases={}
    candidate_current=None
    for case in card['cases']:
        a,b=paired_cases(batch/case,case,card,state,pin,patch_id)
        this=b['effective_current_sha256']
        if candidate_current is None:
            candidate_current=this
            persisted=parse_current(batch/case/'B/native/CALIBRATED-PARENT.alife-checkpoint',this)
            foundation=bytes(persisted['handoff']['actor']['source_foundation_digest']).hex()
        else:
            require(this==candidate_current,'candidate Current differs across exact cases')
        cases[case]={'base':a,'candidate':b,'matched':True,'real_game_inputs':True,'same_disposable_start':True}
    branches=state['active_base_dendritic_sha256']
    require(read(patch)['dendritic_edits']=={'op':'keep'},'candidate branch changes require other verified native authority')
    out={'schema':'qualia.accepted_saved_life_active_card.v1','scorecard_id':card['scorecard_id'],
         'rubric_sha256':CARD_PIN,'executable_sha256':ELF_PIN,'independently_accepted':True,'full_score':True,
         'candidate_patch_id':patch_id,'patch_sha256':pin,'cases':cases,
         'base':{'base_id':state['active_base_id'],'foundation_digest':state['active_base_foundation_digest'],
                 'dendritic_branches_sha256':branches,'source_current_sha256':base_current},
         'candidate':{'base_id':candidate_id,'foundation_digest':foundation,
                      'dendritic_branches_sha256':branches,'source_current_sha256':candidate_current}}
    require(digest(patch)==pin,'patch bytes changed during independent native closure')
    emit(output,out)
    print(json.dumps({'qualified_matched_active_card':str(output),'case_count':len(cases),'sha256':digest(output)}),flush=True)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    sub=p.add_subparsers(dest='command',required=True)
    m=sub.add_parser('measure'); m.add_argument('--work',type=Path,required=True);m.add_argument('--case',required=True)
    b=sub.add_parser('baseline'); b.add_argument('--batch',type=Path,required=True);b.add_argument('--output',type=Path,required=True)
    q=sub.add_parser('pair');q.add_argument('--work',type=Path,required=True);q.add_argument('--case',required=True)
    q.add_argument('--patch',type=Path,required=True);q.add_argument('--output',type=Path,required=True)
    f=sub.add_parser('complete');f.add_argument('--batch',type=Path,required=True);f.add_argument('--patch',type=Path,required=True)
    f.add_argument('--candidate-id',required=True);f.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    if a.command=='measure':
        s=measure(a.work,a.case,score_card())
        emit(a.work/CASE_PROOF,s); print(json.dumps(s,sort_keys=True))
    elif a.command=='pair':prove_pair(a.work,a.case,a.patch,a.output)
    elif a.command=='complete':complete_pair_card(a.batch,a.patch,a.candidate_id,a.output)
    else:baseline(a.batch,a.output)
if __name__=='__main__':main()
