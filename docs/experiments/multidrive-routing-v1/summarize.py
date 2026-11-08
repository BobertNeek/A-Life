"""Read only this assay's detached JSON (plain or gzip); no life/checkpoint IO."""
import gzip
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
with (gzip.open(path, 'rt') if path.suffix == '.gz' else path.open()) as f:
    data = json.load(f)
assert data['schema'] == 'detached_multidrive_routing_v1'
summary = {'backend': data['backend'], 'world_steps': data['world_steps'], 'training_steps': data['training_steps'], 'arms': []}
for arm in data['arms']:
    out = {k: arm[k] for k in ['arm', 'families', 'base_compiled_asset_digest', 'evaluated_weights_f32le_sha256', 'input_cut_exact', 'weights_optimizer_unchanged']}
    out['reachability'] = []
    for r in arm['reachability']:
        out['reachability'].append({'lane': r['lane'], 'label': r['label'], **{kind: {'neurons': r[kind]['reachable_neurons'], 'motor_counts': [f['reachable_motor_count'] for f in r[kind]['families']]} for kind in ['structural_recurrent', 'nonzero_recurrent', 'nonzero_with_dendrites']}})
    out['responses'] = []
    for r in arm['rows']:
        s = r['samples'][-1]
        row = {k:r[k] for k in ['context','perturbation','drives']}
        row.update({k:s[k] for k in ['neural_logits','baseline_neural_logits','neural_delta','direct_innate','neural_plus_innate']})
        row['first_sample_with_neural_delta_over_1e_6'] = next((s['perturbation_frame'] for s in r['samples'] if max(map(abs,s['neural_delta']))>1e-6),None)
        row['motor_activation_range'] = [[min(m['activation']),max(m['activation'])] for m in s['motor_deltas']]
        row['motor_max_abs_delta'] = [max(map(abs,m['delta'])) for m in s['motor_deltas']]
        out['responses'].append(row)
    summary['arms'].append(out)
print(json.dumps(summary, indent=2))
