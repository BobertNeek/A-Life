#!/usr/bin/env python3
"""LOCAL CPU research readout audit. No training, world steps or raw-array output."""
import argparse
import copy
import importlib.util
import json
from pathlib import Path
import torch

spec = importlib.util.spec_from_file_location('ordinary_graph', Path(__file__).with_name('check_feedback_retention_torch.py'))
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)
INTERVALS = (102, 108, 110)


def unpack_sequence(sequence):
    ordinary = copy.deepcopy(sequence)
    terms = {}
    assert len(sequence['ticks']) == 1
    for index, candidate in enumerate(sequence['ticks'][0]['candidates']):
        if 'memory_readout_research_v1' in candidate:
            assert set(candidate) == {'memory_readout_research_v1'}
            payload = candidate['memory_readout_research_v1']
            assert set(payload) == {'ordinary', 'term'}
            ordinary['ticks'][0]['candidates'][index] = copy.deepcopy(payload['ordinary'])
            terms[index] = payload['term']
    return ordinary, terms


def forward(graph, sequence):
    ordinary, terms = unpack_sequence(sequence)
    state = graph.forward(ordinary)
    tick = ordinary['ticks'][0]
    weights = graph.weights + reference.tensor(tick['effective_weight_offsets'])
    activity = reference.tensor(state['activations'])
    logits = []
    for index, candidate in enumerate(tick['candidates']):
        family = reference.FAMILIES.index(candidate['family'])
        value = reference.tensor(graph.biases[family]) + candidate['innate_bias']
        memory, cognitive = reference.tensor(0.), reference.tensor(0.)
        for edge in graph.decoders[family]:
            source, _, _, head, _, lane, _, _ = graph.edges[edge]
            contribution = reference.tensor(candidate['decoder_inputs'][lane]) * weights[edge]
            if head == 1:
                value += activity[source] * contribution
            elif head == 2:
                memory += contribution
            elif head == 4:
                cognitive += contribution
        if index in terms:
            term = terms[index]
            memory += reference.tensor(term['coefficient']) * (reference.tensor(term['source_energy']) * reference.tensor(term['source_confidence']))
        cap = ordinary['memory_candidate_gain']
        value += memory.clamp(-cap, cap) + cognitive.clamp(-cap, cap)
        logits.append(float(value))
    return {'logits':logits, 'activations':state['activations']}


def validate_binding(row, sequence, coefficient):
    ordinary, terms = unpack_sequence(sequence)
    frame = row['patch']['pre_action']['perception']
    base = frame['base']
    candidates = base['candidates']
    profile = row['patch']['decision']['episodic_key']['query']['profile']
    phases = {}
    for index, term in terms.items():
        destination = candidates[index]
        assert term['schema_version'] == 1
        assert term['coefficient'] == coefficient
        assert term['destination_family'] == destination['family'] == 'Approach'
        assert term['source_family'] == 'Ingest'
        assert term['organism_id'] == base['organism_id']
        assert term['query_tick'] == base['tick']
        assert term['profile'] == profile
        assert term['frame_digest'] == frame['frame_digest']
        assert term['current_entity'] == destination['target']['entity']
        if term['status'] != 'Bound':
            assert term['source_energy'] == term['source_confidence'] == 0
            assert term['provenance'] is None
            continue
        source = candidates[term['source_candidate_index']]
        assert source['family'] == 'Ingest'
        assert source['target']['entity'] == destination['target']['entity']
        for candidate in (source, destination):
            slot = candidate['observation']['ObjectSlot']
            assert base['grounded_object_slots'][slot]['tracked_object_id'] == term['current_tracked_target']
        assert 0 < term['source_confidence'] <= 1
        weighted = float(reference.tensor(term['source_energy']) * reference.tensor(term['source_confidence']))
        assert weighted == ordinary['ticks'][0]['candidates'][term['source_candidate_index']]['decoder_inputs'][35]
        sources = [s for s in term['provenance']['sources'] if s is not None]
        assert sources
        for item in sources:
            assert item['organism_id'] == term['organism_id']
            assert item['profile'] == term['profile']
            assert item['family'] == term['source_family']
            assert item['first_tick'] <= item['last_tick'] <= term['query_tick']
            phase = item['outcome_contact'] or 'Unknown'
            phases[phase] = phases.get(phase, 0) + 1
    return {'terms':len(terms), 'bound_terms':sum(t['status']=='Bound' for t in terms.values()),
            'selected_source_phases':phases}


def audit(run):
    request = json.loads((run/'REQUEST.json').read_text())
    coefficient = request.get('prepared_readout_coefficient')
    required = ['GRAPH-LOCAL.json','PREPARED-BIRTH.cp.json','FINAL.cp.json',
                'FOUNDER-ADAM-BIRTH.json','FOUNDER-ADAM-FINAL.json','GENOME.json']
    if coefficient is not None:
        required.append('READOUT-CONSTRUCTION.json')
    for interval in INTERVALS:
        required.extend([f'STEP-{interval:03}-0-TAPE.json',f'STEP-{interval:03}.rows.json'])
        if coefficient is not None:
            required.append(f'STEP-{interval:03}-0-PREPARED-READOUT-COPY.json')
    missing = [name for name in required if not (run/name).is_file()]
    if missing:
        raise ValueError('incomplete expected evidence: '+', '.join(missing))
    assert (run/'FOUNDER-ADAM-BIRTH.json').read_bytes() == (run/'FOUNDER-ADAM-FINAL.json').read_bytes()
    construction = None
    if coefficient is not None:
        construction = json.loads((run/'READOUT-CONSTRUCTION.json').read_text())
        assert construction['parent_genome'] == json.loads((run/'GENOME.json').read_text())
        assert construction['coefficient'] == coefficient
        assert construction['readout_abi_version'] == construction['work_policy_version'] == 1
    for name in ['PREPARED-BIRTH.cp.json','FINAL.cp.json']:
        checkpoint = json.loads((run/name).read_text())
        assert checkpoint['schema_version'] == (5 if coefficient is not None else 2)
        assert checkpoint['residents'][0].get('memory_readout') == construction
    graph = reference.Graph(json.loads((run/'GRAPH-LOCAL.json').read_text()))
    rows = []
    for interval in INTERVALS:
        payload = json.loads((run/f'STEP-{interval:03}.rows.json').read_text())[0]
        if coefficient is None:
            assert 'memory_readout_research_record_v1' not in payload
            row = payload
        else:
            assert set(payload) == {'memory_readout_research_record_v1'}
            row = payload['memory_readout_research_record_v1']
            receipt = row['memory_readout_research']
            assert receipt['schema_version'] == 1 and receipt['coefficient'] == coefficient
            assert receipt['readout_abi_version'] == receipt['work_policy_version'] == 1
            assert receipt['additional_work_ops'] > 0
            assert row['brain_work']['memory_context_ops'] == len(row['replay_tick']['candidates']) + receipt['additional_work_ops']
        tape = json.loads((run/f'STEP-{interval:03}-0-TAPE.json').read_text())
        native = forward(graph, tape['sequence'])
        error = max(reference.maxerr(native['logits'], tape['native_logits']),
                    reference.maxerr(native['activations'], tape['native_final']))
        assert error < 2e-5, error
        entry = {'interval':interval,'native_torch_max_abs_error':error,
                 'binding':validate_binding(row,tape['sequence'],coefficient)}
        if coefficient is not None:
            diagnostic = json.loads((run/f'STEP-{interval:03}-0-PREPARED-READOUT-COPY.json').read_text())
            assert diagnostic['native'] == tape['sequence']
            ablated = diagnostic['zeroed_frozen_coefficient']
            control = copy.deepcopy(ablated)
            _, actual_terms = unpack_sequence(tape['sequence'])
            for index, term in actual_terms.items():
                target = control['ticks'][0]['candidates'][index]['memory_readout_research_v1']['term']
                assert target['coefficient'] == 0
                target['coefficient'] = term['coefficient']
            assert control == tape['sequence']
            zero = forward(graph, ablated)
            error = reference.maxerr(zero['logits'], diagnostic['zeroed_logits'])
            assert error < 2e-5, error
            entry.update(zeroed_torch_max_abs_error=error,
                         logit_max_abs_effect=reference.maxerr(native['logits'],zero['logits']),
                         fixed_actual_action_logp_delta=reference.joint_logp(tape['native_logits'],row['action'])-reference.joint_logp(diagnostic['zeroed_logits'],row['action']),
                         zeroed_choice_changed=diagnostic['zeroed_action']['representative']!=row['action']['representative'] or diagnostic['zeroed_action']['motor_candidates']!=row['action']['motor_candidates'])
        rows.append(entry)
    return {'passed':True,'complete':True,'evaluated_rows':len(rows),'expected_rows':len(INTERVALS),
            'intervention_evaluations':len(rows) if coefficient is not None else 0,
            'expected_intervention_evaluations':len(INTERVALS) if coefficient is not None else 0,
            'rows':rows,'training_calls':0,'host_steps_added':0,'backend':'CPU research prototype; no GPU admission'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run',type=Path)
    parser.add_argument('--output',default='PREPARED-READOUT-TORCH-CHECK.json')
    args = parser.parse_args()
    if Path(args.output).name != args.output or args.output in ('','.','..'):
        raise ValueError('simple create-only output name required')
    torch.set_num_threads(1)
    result = audit(args.run)
    assert result['evaluated_rows'] == result['expected_rows'] > 0
    assert result['intervention_evaluations'] == result['expected_intervention_evaluations']
    with (args.run/args.output).open('x') as file:
        json.dump(result,file,indent=2)
    print(json.dumps({key:result[key] for key in ('passed','complete','evaluated_rows','intervention_evaluations')}))


if __name__ == '__main__':
    main()
