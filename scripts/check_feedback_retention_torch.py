#!/usr/bin/env python3
"""Independent LOCAL Torch replay of Contact20 feedback same-state interventions.

Consumes private native evidence; prints/writes only error magnitudes and scalar
policy metrics. No training, host trajectory, checkpoint mutation, or upload.
"""
import argparse
import json
from pathlib import Path
import torch

FAMILIES = ['Idle', 'Rest', 'Inspect', 'Approach', 'Avoid', 'Contact', 'Ingest', 'Other']


def tensor(x):
    return torch.tensor(x, dtype=torch.float32)


def maxerr(a, b):
    return float((tensor(a) - tensor(b)).abs().max()) if len(a) else 0.0


def fires(cadence, step):
    return cadence == 0 or cadence in (1, 2) and step % 2 == 0 or cadence in (3, 4) and step == 0


class Graph:
    def __init__(self, data):
        self.data = data
        self.edges = data['edges']
        self.n = len(data['dynamics'])
        self.weights = tensor(data['weights'])
        self.dynamics = {key: tensor([d[key] for d in data['dynamics']]) for key in
                         ('bias', 'leak', 'activity_ema_decay', 'metabolic_decay', 'homeostatic_gain')}
        self.activations = [d['activation'] for d in data['dynamics']]
        self.biases = dict(data['biases'])
        self.incoming = [[] for _ in range(self.n)]
        self.decoders = [[] for _ in FAMILIES]
        for i, edge in enumerate(self.edges):
            source, target, route, head, family, lane, _, alpha = edge
            if head == 0:
                self.incoming[target].append(i)
            elif head != 3:
                self.decoders[family].append(i)
        # Parallelize targets, preserve native within-target summation order.
        width = max(map(len, self.incoming))
        self.ids = torch.tensor([row + [0] * (width-len(row)) for row in self.incoming])
        self.valid = torch.tensor([[True]*len(row)+[False]*(width-len(row)) for row in self.incoming])
        self.sources = torch.tensor([e[0] for e in self.edges])[self.ids]
        self.routes = torch.tensor([e[2] for e in self.edges])[self.ids]

    def forward(self, q):
        assert len(q['ticks']) == 1
        tick = q['ticks'][0]
        a = tensor(q['initial']['activations'])
        m = tensor(q['initial']['metabolic_load'])
        ema = tensor(q['initial']['activity_ema'])
        weights = self.weights + tensor(tick['effective_weight_offsets'])
        routes = torch.tensor(tick['enabled_routes'], dtype=torch.bool)
        d = self.dynamics
        for step in range(tick['microstep_count']):
            gate = torch.tensor([fires(c, step) for c in self.data['cadence']])
            active = self.valid & routes[self.routes] & gate[self.routes]
            total = torch.zeros(self.n)
            for rank in range(self.ids.shape[1]):
                total += torch.where(active[:, rank], a[self.sources[:, rank]] * weights[self.ids[:, rank]], 0.)
            for edge in tick['structural_synapses']:
                if tick['enabled_routes'][edge['route']] and fires(edge['cadence'], step):
                    total[edge['target']] += a[edge['source']] * edge['effective_weight']
            dendritic = torch.zeros(self.n)
            for branch in q['initial']['dendrites']['branches']:
                value = tensor(0.)
                for item in branch['inputs']:
                    value += a[item['source']] * item['weight']
                dendritic[branch['target']] += torch.tanh(torch.relu(value-branch['threshold'])) * branch['output_gain']
            z = d['bias'] + tensor(tick['encoded_inputs']) + tick['projection_gain']*(total+dendritic) - d['homeostatic_gain']*m - tick['local_threshold_shift']
            activated = torch.empty(self.n)
            for kind, fn in [('Identity', lambda x:x), ('Relu', torch.relu), ('Tanh', torch.tanh), ('Logistic', lambda x:1./(1.+torch.exp(-x)))]:
                mask = torch.tensor([k == kind for k in self.activations])
                activated[mask] = fn(z[mask])
            a = (1.-d['leak'])*a + d['leak']*activated
            m = torch.clamp(d['metabolic_decay']*m + ((1.-d['metabolic_decay'])*a)*a, 0., 1.)
            ema = torch.clamp(d['activity_ema_decay']*ema + (1.-d['activity_ema_decay'])*a.abs(), 0., 1.)
        logits = []
        for candidate in tick['candidates']:
            family = FAMILIES.index(candidate['family'])
            value = tensor(self.biases[family]) + candidate['innate_bias']
            memory, cognitive = tensor(0.), tensor(0.)
            for i in self.decoders[family]:
                source, _, _, head, _, lane, _, _ = self.edges[i]
                contribution = tensor(candidate['decoder_inputs'][lane]) * weights[i]
                if head == 1:
                    value += a[source] * contribution
                elif head == 2:
                    memory += contribution
                elif head == 4:
                    cognitive += contribution
            cap = q['memory_candidate_gain']
            value += memory.clamp(-cap, cap) + cognitive.clamp(-cap, cap)
            logits.append(float(value))
        return {'logits':logits, 'activations':a.tolist()}


def distribution(logits, mask, temperature):
    ids = [i for i in range(len(logits)) if mask & (1 << i)]
    values = torch.tensor([logits[i] for i in ids], dtype=torch.float64)/temperature
    lp = torch.log_softmax(values, 0)
    return dict(zip(ids, lp.tolist()))


def joint_logp(logits, action):
    r = action['representative']
    value = distribution(logits, action['representative_mask'], action['temperature'])[r]
    for slot, index in enumerate(action['motor_candidates']):
        if index is not None and action['forced_slots'][r] != slot:
            value += distribution(logits, action['motor_masks'][slot], action['temperature'])[index]
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('local_run', type=Path)
    parser.add_argument('--intervals', default='5,7')
    parser.add_argument('--output', default='TORCH-AGGREGATES.json')
    args = parser.parse_args()
    torch.set_num_threads(1)
    if Path(args.output).name != args.output or args.output in ('', '.', '..'):
        raise ValueError('simple create-only output filename required')
    graph = Graph(json.loads((args.local_run/'GRAPH-LOCAL.json').read_text()))
    results = []
    for arm in ['None', 'Praise', 'Scold', 'Pet', 'Slap']:
        for interval in map(int,args.intervals.split(',')):
            stem = f'{arm}-{interval:03}'
            diagnostic = json.loads((args.local_run/f'{stem}-DIAGNOSTIC.json').read_text())
            row = json.loads((args.local_run/f'{stem}.rows.json').read_text())[0]
            native, ablated = diagnostic['native'], diagnostic['ablated']
            # Require every other piece of frozen evidence to be byte-equivalent.
            import copy
            control = copy.deepcopy(ablated)
            control['ticks'][0]['effective_weight_offsets'] = native['ticks'][0]['effective_weight_offsets']
            assert native == control
            arm_cp = json.loads((args.local_run/f'{stem}-PRE.cp.json').read_text())
            none_cp = json.loads((args.local_run/f'None-{interval:03}-PRE.cp.json').read_text())
            assert arm_cp['world']['world']['tick'] == none_cp['world']['world']['tick']
            arm_neural = arm_cp['residents'][0]['neural']
            none_neural = none_cp['residents'][0]['neural']
            assert native['initial']['dendrites'] == arm_neural['neural']['dendrites']
            for field in ('activations', 'activity_ema', 'metabolic_load'):
                assert torch.equal(tensor(native['initial'][field]), tensor(arm_neural['neural'][field]))
            alpha = tensor([edge[7] for edge in graph.edges])
            actual_offsets = tensor(arm_neural['lifetime']) + alpha*tensor(arm_neural['fast'])
            fast_component = alpha*(tensor(arm_neural['fast'])-tensor(none_neural['fast']))
            assert torch.equal(actual_offsets, tensor(native['ticks'][0]['effective_weight_offsets']))
            assert torch.equal(actual_offsets-fast_component, tensor(ablated['ticks'][0]['effective_weight_offsets']))
            actual = graph.forward(native)
            intervention = graph.forward(ablated)
            errs = {key:maxerr(actual[torch_key], diagnostic[key]) for key,torch_key in
                    [('native_logits','logits'),('native_final','activations')]}
            errs.update({key:maxerr(intervention[torch_key], diagnostic[key]) for key,torch_key in
                         [('ablated_logits','logits'),('ablated_final','activations')]})
            assert max(errs.values()) < 2e-5, errs
            nlp = joint_logp(diagnostic['native_logits'],row['action'])
            alp = joint_logp(diagnostic['ablated_logits'],row['action'])
            results.append({'arm':arm,'interval':interval,'torch_max_abs_errors':errs,
                            'native_fixed_action_logp_f64':nlp, 'ablated_fixed_action_logp_f64':alp,
                            'fixed_action_logp_delta_f64':nlp-alp,
                            'only_fast_component_reconstructed_exactly':True,
                            'torch_logit_effect_max_abs':maxerr(actual['logits'],intervention['logits']),
                            'torch_activity_effect_max_abs':maxerr(actual['activations'],intervention['activations'])})
    summary = {'torch_version':torch.__version__,'threads':1,'dtype':'float32',
               'tolerance':2e-5,'rows':results,'passed':True,'training_calls':0}
    # Create-only local evidence. Explicitly choose another file for another run.
    with (args.local_run/args.output).open('x') as f:
        json.dump(summary,f,indent=2)
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
