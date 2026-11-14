#!/usr/bin/env python3
"""LOCAL independent Torch audit; emits scalars, never raw neural/state arrays."""
import argparse
import importlib.util
import json
from pathlib import Path
import torch

spec = importlib.util.spec_from_file_location('native_graph_reference', Path(__file__).with_name('check_feedback_retention_torch.py'))
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)

INTERVALS = (102, 108, 110)
GAINS = (-16, -4, 0, 4, 16)


def maximum(a, b):
    return float((torch.tensor(a, dtype=torch.float32)-torch.tensor(b, dtype=torch.float32)).abs().max())


def audit_run(run, native_only=False):
    """Missing expected evidence never qualifies, including zero comparisons.

    Source qualification can explicitly request native-only replay. The default
    requires all five coefficient interventions at every expected boundary.
    Preflight stops before numerical evaluation if any required evidence is absent.
    """
    missing = []
    required = [run/'GRAPH-LOCAL.json']
    for interval in INTERVALS:
        required.extend([run/f'STEP-{interval:03}-0-TAPE.json',
                         run/f'STEP-{interval:03}.rows.json'])
        if not native_only:
            required.append(run/f'STEP-{interval:03}-0-COEFFICIENT-COPY.json')
    missing.extend(p.name for p in required if not p.is_file())
    if not native_only:
        for interval in INTERVALS:
            file = run/f'STEP-{interval:03}-0-COEFFICIENT-COPY.json'
            if file.is_file():
                data = json.loads(file.read_text())
                gains = [trial.get('gain') for trial in data.get('trials', [])]
                if len(gains) != len(GAINS) or set(gains) != set(GAINS):
                    missing.append(f'{file.name}: expected each declared gain exactly once')
    result = {'passed':False, 'complete':False, 'status':'incomplete',
              'mode':'native_replay_only' if native_only else 'native_and_coefficient_copies',
              'expected_rows':len(INTERVALS), 'evaluated_rows':0,
              'expected_coefficient_copy_evaluations':0 if native_only else len(INTERVALS)*len(GAINS),
              'coefficient_copy_evaluations':0, 'missing_evidence':missing,
              'torch_version':torch.__version__, 'threads':1, 'tolerance':2e-5,
              'rows':[], 'diagnostic_copies_only':True, 'training_calls':0}
    if missing:
        return result
    graph = reference.Graph(json.loads((run/'GRAPH-LOCAL.json').read_text()))
    rows = []
    for interval in INTERVALS:
        path = run/f'STEP-{interval:03}-0-TAPE.json'
        tape = json.loads(path.read_text())
        actual = graph.forward(tape['sequence'])
        errors = {'activity':maximum(actual['activations'],tape['native_final']),
                  'logits':maximum(actual['logits'],tape['native_logits'])}
        assert max(errors.values()) < 2e-5, errors
        row = json.loads((run/f'STEP-{interval:03}.rows.json').read_text())[0]
        trials = []
        file = run/f'STEP-{interval:03}-0-COEFFICIENT-COPY.json'
        if not native_only:
            native = json.loads(file.read_text())
            ids = [i for i,e in enumerate(graph.edges) if e[3]==2 and e[4]==6 and e[5]==35]
            assert len(ids)==40
            base = tape['sequence']
            for experiment in native['trials']:
                intervention = experiment['sequence']
                # Check only the intended decoder coordinates changed on the COPY.
                import copy
                control = copy.deepcopy(intervention)
                control['ticks'][0]['effective_weight_offsets'] = base['ticks'][0]['effective_weight_offsets']
                assert control == base
                old = torch.tensor(base['ticks'][0]['effective_weight_offsets'],dtype=torch.float32)
                expected = old.clone()
                expected[ids] += (experiment['gain']-native['actual_gain'])/40
                assert torch.equal(expected,torch.tensor(intervention['ticks'][0]['effective_weight_offsets'],dtype=torch.float32))
                masked = experiment['source_masked_sequence']
                control = copy.deepcopy(masked)
                for i,c in enumerate(control['ticks'][0]['candidates']):
                    if c['family']=='Ingest':
                        assert masked['ticks'][0]['candidates'][i]['decoder_inputs'][35]==0
                        c['decoder_inputs'][35]=intervention['ticks'][0]['candidates'][i]['decoder_inputs'][35]
                assert control == intervention
                changed = graph.forward(intervention)
                removed = graph.forward(masked)
                error = max(maximum(changed['logits'],experiment['logits']),maximum(removed['logits'],experiment['source_masked_logits']))
                assert error < 2e-5, error
                lp = reference.joint_logp(experiment['logits'],row['action'])
                no_lp = reference.joint_logp(experiment['source_masked_logits'],row['action'])
                trials.append({'gain':experiment['gain'],'torch_logit_max_abs_error':error,
                    'fixed_actual_action_logp':lp,'source_masked_fixed_action_logp':no_lp,
                    'source_logp_contribution':lp-no_lp,
                    'source_logit_max_abs_effect':maximum(experiment['logits'],experiment['source_masked_logits']),
                    'resampled_choice_changed_from_actual':experiment['resampled_action']['representative']!=row['action']['representative'] or experiment['resampled_action']['motor_candidates']!=row['action']['motor_candidates']})
        rows.append({'interval':interval,'native_torch_max_abs_errors':errors,'coefficient_copy_trials':trials})
    result.update(passed=True, complete=True, status='passed', rows=rows,
                  evaluated_rows=len(rows),
                  coefficient_copy_evaluations=sum(len(r['coefficient_copy_trials']) for r in rows))
    assert result['evaluated_rows'] == result['expected_rows'] > 0
    assert result['coefficient_copy_evaluations'] == result['expected_coefficient_copy_evaluations']
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run', type=Path)
    parser.add_argument('--output', default='READOUT-TORCH-AGGREGATES.json')
    parser.add_argument('--native-only', action='store_true',
                        help='explicit native replay qualification; no coefficient-copy qualification')
    args = parser.parse_args()
    if Path(args.output).name != args.output or args.output in ('', '.', '..'):
        raise ValueError('simple create-only output name required')
    torch.set_num_threads(1)
    result = audit_run(args.run, args.native_only)
    with (args.run/args.output).open('x') as f:
        json.dump(result,f,indent=2)
    print(json.dumps({key:result[key] for key in
                     ('passed','complete','status','mode','expected_rows','evaluated_rows',
                      'expected_coefficient_copy_evaluations','coefficient_copy_evaluations',
                      'missing_evidence')}))
    return 0 if result['passed'] else 2


if __name__ == '__main__':
    raise SystemExit(main())
