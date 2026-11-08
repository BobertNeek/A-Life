"""Predeclared interval extraction. No world execution, tuning or private banks.

Founder B is the initial -4 coefficient on observed B, evaluated on the current
signal/reference activations. Aggregate somatic readout is reported separately.
"""
import collections
import hashlib
import json
import sys
from pathlib import Path

raw = Path(sys.argv[1])
study = json.loads(raw.read_text())
assert study['schema'] == 'ordinary_physiology_b_results_v1'
B_LANES = list(range(6, 15)) + [18]
rows = []
for trial in study['trials']:
    arm = trial['recipe']['arm']
    case = trial['recipe']['case']['name']
    for interval, frame in enumerate(trial['rows'], 1):
        record = dict(arm=arm, case=case, interval=interval,
                      tick_before=frame['tick_before'], tick_after=frame['tick_after'],
                      sleeping=frame['sleeping_organisms'], early_stop=trial['early_stop'],
                      before_drives=frame['biology_before']['homeostasis']['drives'],
                      after_drives=frame['biology_after']['homeostasis']['drives'],
                      before_energy=frame['biology_before']['body']['energy'],
                      after_energy=frame['biology_after']['body']['energy'],
                      before_resident=frame['resident_before'], after_resident=frame['resident_after'],
                      position_before=frame['position_before'], position_after=frame['position_after'],
                      decisions=[])
        for decision in frame['decisions']:
            path = {p['id']: p for p in decision['path']}
            activation = lambda n: path[n]['activation']
            candidates = []
            for c in decision['candidates']:
                out = {k: c[k] for k in ['index', 'family', 'kind', 'neural_action_readout', 'total_logit', 'innate_bias']}
                out['has_target'] = c['candidate']['target']['entity'] is not None
                if c['family'] == 'Approach':
                    x = c['features']
                    basis = sum(x[i] for i in B_LANES) / 10.0
                    assert basis >= 0
                    signal_reference = activation(1779) - activation(1791)
                    b_score = -4.0 * basis * signal_reference if arm['dedicated_b'] else 0.0
                    hunger_score = (activation(1776) - activation(1791)) * (4.0*x[15] - 2.0*x[2] - 2.0*x[18])
                    out.update(observed_B=basis, fatigue_signal=activation(1779), reference=activation(1791),
                               signal_minus_reference=signal_reference, founder_B_contribution=b_score,
                               founder_hunger_contribution=hunger_score,
                               aggregate_somatic_readout_plus_roundoff=c['neural_action_readout']-hunger_score-b_score)
                candidates.append(out)
            record['decisions'].append(dict(candidates=candidates, path=decision['path'],
                receptor_effects=decision['receptor_effects'], microsteps=decision['microsteps'],
                atp_available=decision['available_atp_q16'], atp_debit=decision['debit_atp_q16'],
                tonic_effective_weights=decision['tonic_effective_weights'],
                representative_index=decision['action']['representative'],
                selected_primitives=[{k:c[k] for k in ['channel','primitive']} for c in decision['selected_bundle']['channels']],
                physical_outcomes=decision['outcome'].get('joint',{}).get('channel_outcomes',[]),
                production_replay_exact=decision['production_replay_exact']))
        rows.append(record)
by = {(r['arm']['bounded'], r['arm']['dedicated_b'], r['arm']['tonic_bridges'], r['case'], r['interval']): r for r in rows}
contrasts = []
def scored(row, family):
    if row is None or not row['decisions']:
        return None
    candidates = [c for c in row['decisions'][0]['candidates'] if c['family'] == family and (family == 'Rest' or c['has_target'])]
    assert len(candidates) <= 1
    return candidates[0] if candidates else None
for bounded in [False, True]:
    for dedicated in [False, True]:
        for tonic in [False, True]:
            for interval in range(1, study['ticks_per_case'] + 1):
                for changed, baseline, family in [('neutral_fatigued','neutral_comfortable','Rest'), ('food_fatigued','food_comfortable','Approach'), ('bitter_fatigued','bitter_comfortable','Approach'), ('food_hungry','food_comfortable','Ingest'), ('bitter_hungry','bitter_comfortable','Ingest'), ('object_curious','object_comfortable','Inspect')]:
                    a = scored(by.get((bounded,dedicated,tonic,changed,interval)),family)
                    b = scored(by.get((bounded,dedicated,tonic,baseline,interval)),family)
                    row = dict(bounded=bounded,dedicated_b=dedicated,tonic_bridges=tonic,interval=interval,
                               contrast=changed+'-'+baseline,family=family,opportunity_present_both=a is not None and b is not None)
                    for field in ['neural_action_readout','total_logit','innate_bias','founder_B_contribution','founder_hunger_contribution','aggregate_somatic_readout_plus_roundoff','signal_minus_reference']:
                        row[field+'_delta'] = a[field]-b[field] if a is not None and b is not None and field in a and field in b else None
                    contrasts.append(row)
print(json.dumps(dict(schema='ordinary_physiology_b_interval_summary_v1',raw_sha256=hashlib.sha256(raw.read_bytes()).hexdigest(),
    seed=study['seed'],ticks_per_case=study['ticks_per_case'],world_ticks=len(rows),
    interpretation='Initial-gene B contribution on current states; somatic readout separated. Later paired-world contrasts include trajectory/physiology divergence, not matched-input interventions.',
    contrasts=contrasts,rows=rows),indent=2))
