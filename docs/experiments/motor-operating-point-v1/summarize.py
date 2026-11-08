"""Validate and summarize only fresh detached assay outputs and frozen public baseline."""
import gzip
import json
import sys
from pathlib import Path

def read(path):
    path = Path(path)
    with (gzip.open(path, 'rt') if path.suffix == '.gz' else path.open()) as f:
        return json.load(f)

candidate = read(sys.argv[1])
baseline = read(sys.argv[2])
assert candidate['schema'] == 'motor_operating_point_candidate_v1'
assert baseline['schema'] == 'detached_multidrive_routing_v1'
assert candidate['world_steps'] == candidate['learning_steps'] == 0
frozen = next(a for a in candidate['arms'] if a['arm'] == 'frozen_v6')
original = next(a for a in baseline['arms'] if a['arm'] == 'whole_care6')
lookup = {(r['context'],r['condition']):r for r in frozen['rows']}
checks = 0
for r in original['rows']:
    compared = lookup[r['context'],r['perturbation']]
    assert compared['drives'] == r['drives']
    for a,b in zip(compared['samples'],r['samples']):
        assert a['frame'] == b['perturbation_frame']
        assert a['neural_logits'] == b['neural_logits']
        assert a['baseline_logits'] == b['baseline_neural_logits']
        assert a['neural_delta'] == b['neural_delta']
        assert a['innate'] == b['direct_innate']
        for m,n in zip(a['motors'],b['motor_deltas']):
            assert m['family'] == n['family'] and m['motor_ids'] == n['motor_ids']
            assert m['activation'] == n['activation']
            assert m['baseline_activation'] == n['baseline_activation']
        checks += 1
summary = {'schema':candidate['schema'], 'families':candidate['families'], 'frozen_baseline_exact_snapshot_checks':checks, 'arms':[]}
for arm in candidate['arms']:
    for flag in ['default_graph_rejected','dendrites_preserved','optimizer_preserved','input_cut_sequences_exact','differential_pair_activation_trace_exact']:
        assert arm[flag]
    out = {k:arm[k] for k in ['arm','counterbalance_gain','differential_readout','candidate_asset_digest','edits','baseline_convergence','baseline_components']}
    out['responses'] = []
    for r in arm['rows']:
        last = r['samples'][-1]
        row = {k:r[k] for k in ['context','condition','drives']}
        row.update({k:last[k] for k in ['neural_logits','baseline_logits','neural_delta','innate','total']})
        row['motor_activation_range'] = [[min(m['activation']),max(m['activation'])] for m in last['motors']]
        row['positive_saturated_motor_count'] = [sum(v>0.99 for v in m['activation']) for m in last['motors']]
        row['negative_saturated_motor_count'] = [sum(v< -0.99 for v in m['activation']) for m in last['motors']]
        out['responses'].append(row)
    summary['arms'].append(out)
print(json.dumps(summary,indent=2))
