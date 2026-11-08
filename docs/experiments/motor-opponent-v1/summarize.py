"""Validate fixed opponent receipts against the frozen public baseline."""
import gzip
import json
import sys
from pathlib import Path

def read(path):
    p=Path(path)
    with (gzip.open(p,'rt') if p.suffix=='.gz' else p.open()) as f:
        return json.load(f)

j=read(sys.argv[1]); baseline=read(sys.argv[2])
assert j['schema']=='fatigue_opponent_replay_v1'
assert j['world_steps']==j['learning_steps']==0
families=j['families']; assert len(families)==8
expected={(f,s,d) for f in ['Original','Opponent','SourceCut','RelayCut'] for s in [0,-.5,.5] for d in [False,True]}
assert len(j['arms'])==24 and {(a['factor'],a['motor_start'],a['differential_readout']) for a in j['arms']}==expected
frames=[32,33,34,36,40,48,64,65,66,68,72,80,96]
for a in j['arms']:
    assert len(a['rows'])==75 and len({(r['condition'],r['context']) for r in a['rows']})==75
    assert a['initial_metabolic_load']==a['initial_activity_ema']==0
    for flag in ['optimizer_preserved','inherited_branches_preserved','input_cuts_exact','readout_pair_traces_exact']: assert a[flag]
    for r in a['rows']:
        assert len(r['samples'])==13 and [s['frame'] for s in r['samples']]==frames
        for s in r['samples']:
            for k in ['neural_logits','baseline_logits','neural_delta','innate','total']:assert len(s[k])==8
            assert len(s['motors'])==8 and [m['family'] for m in s['motors']]==families
            assert len(s['path_neurons'])==5 and {n['role'] for n in s['path_neurons']}=={'source','reserve','relay','rest_reference','approach_signal'}
            assert s['phase']==('baseline' if s['frame']<=32 else 'perturbation' if s['frame']<=64 else 'washout')
            assert all(abs(n+i-t)<1e-5 for n,i,t in zip(s['neural_logits'],s['innate'],s['total']))
orig=next(a for a in j['arms'] if (a['factor'],a['motor_start'],a['differential_readout'])==('Original',0,False))
frozen=next(a for a in baseline['arms'] if a['arm']=='whole_care6')
assert len(frozen['rows'])==45 and frozen['families']==families
lookup={(r['context'],r['condition']):r for r in orig['rows']}
checks=0
for r in frozen['rows']:
    assert len(r['samples'])==6
    current=lookup[r['context'],r['perturbation']]
    for old in r['samples']:
        new=next(s for s in current['samples'] if s['frame']==32+old['perturbation_frame'])
        for k,previous in [('neural_logits','neural_logits'),('baseline_logits','baseline_neural_logits'),('neural_delta','neural_delta'),('innate','direct_innate')]:assert new[k]==old[previous]
        assert len(old['motor_deltas'])==8
        for m,n in zip(new['motors'],old['motor_deltas']):
            assert m['family']==n['family'] and m['min']==min(n['activation']) and m['max']==max(n['activation'])
            assert abs(m['mean']-sum(n['activation'])/16)<1e-6
            assert m['max_abs_delta']==max(map(abs,n['delta']))
        motor_values={i:v for m in old['motor_deltas'] for i,v in zip(m['motor_ids'],m['activation'])}
        for n in new['path_neurons']:
            if n['role'] in ['rest_reference','approach_signal']:assert n['activation']==motor_values[n['id']]
        checks+=1
assert checks==270
summary={'schema':j['schema'],'families':families,'exact_frozen_snapshot_checks':checks,'inspection':j['inspection'],'arms':[]}
for a in j['arms']:
    out={k:a[k] for k in ['factor','motor_start','differential_readout','candidate_asset_digest','candidate_phenotype_hash','initial_state_sha256','plan']}
    out['responses']=[];out['fatigue_curves']={}
    for r in a['rows']:
        peak=next(s for s in r['samples'] if s['frame']==64);wash=r['samples'][-1]
        row={k:r[k] for k in ['condition','context','perturbed_drives']}
        row.update({k:peak[k] for k in ['neural_logits','baseline_logits','neural_delta','innate','total','motors','path_neurons']})
        row['washout_max_abs_logit_delta']=max(map(abs,wash['neural_delta']))
        row['washout_max_abs_motor_delta']=max(m['max_abs_delta'] for m in wash['motors'])
        out['responses'].append(row)
    for context in ['nutritious_contact','aversive_contact','distant']:
        base=next(r['perturbed_drives'] for r in a['rows'] if r['context']==context and r['condition']=='balanced')
        rows=[r for r in out['responses'] if r['context']==context and r['perturbed_drives'][:1]+r['perturbed_drives'][2:]==base[:1]+base[2:]]
        assert len(rows)==9
        out['fatigue_curves'][context]=sorted(rows,key=lambda r:r['perturbed_drives'][1])
    summary['arms'].append(out)
# Compact JSON keeps the complete family/condition vectors without repeated whitespace.
print(json.dumps(summary,separators=(',',':')))
