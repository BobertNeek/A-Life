#!/usr/bin/env python3
"""Audit local v12 replay snapshots with PyTorch; print aggregate errors only.

Use --inspect output from the request's unchanged parent. Full replay/state files
stay local. This checks the frozen forward update, not learning or generalization.
"""
import argparse
import json
from pathlib import Path

import torch

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('inspection', type=Path)
parser.add_argument('native_result', type=Path)
args = parser.parse_args()
p = json.loads(args.inspection.read_text())
r = json.loads(args.native_result.read_text())
if (any(r['params']) or any(r['hold_readout']) or r['bearing_gain'] != 0
        or r['memory_aversion'] != 0 or r['readout_scale'] != [1, 1]):
    raise ValueError('this audit reconstructs weight_edits only; inspect the fully edited asset for other recipes')
torch.set_num_threads(1)
weights = torch.tensor(p['weights'], dtype=torch.float32)
for index, delta in r['weight_edits']:
    weights[index] += delta
ids = torch.tensor([i for i, s in enumerate(p['synapses']) if s['kind'] == 'Recurrent'])
edges = [p['synapses'][i] for i in ids.tolist()]
sources = torch.tensor([s['source'] for s in edges])
targets = torch.tensor([s['target'] for s in edges])
dynamics = p['neuron_dynamics']
values = lambda key: torch.tensor([d[key] for d in dynamics], dtype=torch.float32)
bias, leak = values('bias'), values('leak')
homeo, decay = values('homeostatic_gain'), values('metabolic_decay')
activation_functions = {'Tanh': torch.tanh, 'Relu': torch.relu,
                        'Sigmoid': torch.sigmoid, 'Linear': lambda x: x,
                        'Identity': lambda x: x}


def fires(cadence, step):
    if cadence not in range(5):
        raise ValueError(f'unknown cadence {cadence}')
    return cadence == 0 or (cadence in (1, 2) and step % 2 == 0) or (cadence in (3, 4) and step == 0)


results = []
for trial in r['trials']:
    for row in trial['trace']:
        snapshot = row.get('replay_snapshot')
        if not snapshot:
            continue
        tick = snapshot['tick']
        a = torch.tensor(snapshot['initial']['activations'], dtype=torch.float32)
        m = torch.tensor(snapshot['initial']['metabolic_load'], dtype=torch.float32)
        effective = weights + torch.tensor(tick['effective_weight_offsets'])
        encoded = torch.tensor(tick['encoded_inputs'])
        for step in range(tick['microstep_count']):
            mask = torch.tensor([tick['enabled_routes'][e['route']] and
                                 fires(p['projection_cadences'][e['route']], step) for e in edges])
            recurrent = torch.zeros_like(a)
            recurrent.scatter_add_(0, targets[mask], a[sources[mask]] * effective[ids[mask]])
            for e in tick['structural_synapses']:
                if tick['enabled_routes'][e['route']] and fires(e['cadence'], step):
                    recurrent[e['target']] += a[e['source']] * e['effective_weight']
            dendritic = torch.zeros_like(a)
            for branch in snapshot['initial']['dendrites']['branches']:
                total = torch.tensor(0.)
                for item in branch['inputs']:
                    total += a[item['source']] * item['weight']
                dendritic[branch['target']] += torch.tanh(torch.relu(total - branch['threshold'])) * branch['output_gain']
            pre = bias + encoded + tick['projection_gain'] * (recurrent + dendritic) - homeo * m - tick['local_threshold_shift']
            activated = pre.clone()
            for kind in {d['activation'] for d in dynamics}:
                indices = torch.tensor([i for i, d in enumerate(dynamics) if d['activation'] == kind])
                activated[indices] = activation_functions[kind](pre[indices])
            a = (1 - leak) * a + leak * activated
            m = (decay * m + (1 - decay) * a * a).clamp(0, 1)
        expected = torch.tensor(snapshot['expected_final'])
        error = float((a - expected).abs().max())
        retained_error = float((expected - torch.tensor(snapshot['retained_final'])).abs().max())
        results.append({'scene': trial['scene'], 'tick': row['tick'],
                        'max_activation_error': error, 'expected_retained_max_error': retained_error})
        if error >= 2e-6 or retained_error != 0:
            raise AssertionError(results[-1])
if not results:
    raise ValueError('no complete local replay snapshot: enable trace and run beyond tick 32')
print(json.dumps({'torch': torch.__version__, 'results': results}, indent=2))
