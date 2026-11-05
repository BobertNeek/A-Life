"""Read-only audit of native outcome evidence in saved V26/V28 speech captures.

No simulation, neural forward, fitting, optimizer, actor construction or model
call. Scalar gate terms below are observational arithmetic, not causal ablations.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import struct

import zstandard as zstd

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--captures', type=Path, required=True)
parser.add_argument('--collection', type=Path, required=True)
parser.add_argument('--actor', type=Path, required=True)
parser.add_argument('--protected', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
inputs = {}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    inputs[str(path)] = sha(path)
    with zstd.ZstdDecompressor().stream_reader(open(path, 'rb')) as stream:
        return json.load(stream)


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def bits_to_float(values):
    return [struct.unpack('<f', struct.pack('<I', v))[0] for v in values]


actor = read(args.actor)
captured = {n: read(args.captures / f'native-decision-{n:02}.json.zst')
            for n in [9, 10, 13, 14]}
holding = []
recall = []
food = captured[9]['full_native_row']['patch']['decision']['selected_action']['target_entity']
for number, item in captured.items():
    row = item['full_native_row']
    record = item['record']
    frame = row['patch']['pre_action']['perception']['base']
    holding.append({
        'captured_row': number, 'seed': record['seed'],
        'condition': record['condition'], 'offset': record['offset'],
        'tactile_holding': frame['sensory']['channels']['tactile_contact'][2],
        'owned_before': record['owned_before'], 'owned_after': record['owned_after'],
        'encoded_neuron_1858': row['replay_tick']['encoded_inputs'][1858],
        'initial_neuron_1858': row['initial']['activations'][1858],
        'final_neuron_1858': row['final_features'][1858],
        'raw_first_presence_stop': record['raw_speech_outputs_six_steps'][0][16:18],
        'actual_event': record['current_actual_event'],
        'recent_support': record['recent_matching_support'],
        'label': record['label'],
    })
    for candidate in frame['candidates']:
        if candidate['action_id'] == 400 or (
                candidate['action_id'] == 210 and candidate['target']['entity'] == food):
            values = row['replay_tick']['candidates'][candidate['candidate_index']]['decoder_inputs']
            recall.append({
                'row': number, 'action_id': candidate['action_id'],
                'target': candidate['target']['entity'],
                'target_memory_decoder_inputs': values[24:32],
                'family_memory_decoder_inputs': values[32:36],
            })
near = captured[10]['full_native_row']
far = captured[14]['full_native_row']
assert holding[1]['tactile_holding'] == 1 and holding[3]['tactile_holding'] == 0
assert near['initial']['activations'][1858] == far['initial']['activations'][1858]
assert (near['replay_tick']['effective_weight_offsets'][27648:28672]
        == far['replay_tick']['effective_weight_offsets'][27648:28672])
terms = []
for output in [16, 17]:
    index = 27648 + 32 * 2 + output
    weight = f32(actor['weights'][index])
    offset = near['replay_tick']['effective_weight_offsets'][index]
    effective = f32(weight + offset)
    left = f32(near['final_features'][1858] * effective)
    right = f32(far['final_features'][1858] * effective)
    raw_left = captured[10]['record']['raw_speech_outputs_six_steps'][0][output]
    raw_right = captured[14]['record']['raw_speech_outputs_six_steps'][0][output]
    terms.append({
        'output': output, 'synapse_index': index, 'source_neuron': 1858,
        'base_weight': weight, 'identical_private_offset': offset,
        'effective_weight': effective, 'near_native_term': left,
        'far_native_term': right, 'term_difference': left - right,
        'whole_gate_difference': raw_left - raw_right,
        'interpretation': 'Mixed holding/word/recurrent source-neuron term; not isolated holding causation.',
    })
post_pickup = captured[9]['full_host_after']['residents'][0]['memory']['records']
next_bank = captured[10]['full_host_before']['residents'][0]['memory']['records']
record = next(r for r in post_pickup if r['sealed_sequence_id_raw'] == 3)
assert record in next_bank
memory = {
    'source_sequence': record['sealed_sequence_id_raw'],
    'selected_action_id': record['action_id_raw'],
    'first_tick': record['first_tick_raw'], 'last_tick': record['last_tick_raw'],
    'tracked_object_id': record['tracked_object_id_raw'],
    'target_latent': bits_to_float(record['target_latent_bits']),
    'family_value': bits_to_float(record['family_value_bits']),
    'same_complete_record_present_before_next_decision': True,
    'exact_grip_transfer_or_action_phase_fields_in_candidate_record':
        [k for k in record if 'grip' in k or 'phase' in k or 'owner' in k],
}
inputs[str(args.collection)] = sha(args.collection)
meal = []
meal_food = None
with zstd.ZstdDecompressor().stream_reader(open(args.collection, 'rb')) as stream:
    for line in io.TextIOWrapper(stream):
        event = json.loads(line)
        if event['requested'] != 'EatFood':
            continue
        row = event['row']
        frame = row['patch']['pre_action']['perception']['base']
        sensory = frame['sensory']['channels']['tactile_contact']
        if event['step'] == 0:
            meal_food = row['patch']['decision']['selected_action']['target_entity']
        meal.append({
            'offset': event['step'], 'tick': frame['tick'],
            'selected_action_id': row['patch']['decision']['selected_action']['action_id'],
            'physical_contact': row['patch']['outcome']['physical']['contact'],
            'holding': sensory[2], 'positive_taste': sensory[5], 'negative_taste': sensory[6],
            'encoded_positive_taste_neuron_1861': row['replay_tick']['encoded_inputs'][1861],
            'encoded_negative_taste_neuron_1862': row['replay_tick']['encoded_inputs'][1862],
            'hunger': frame['homeostasis']['drives']['hunger'],
            'actual_event': event['actual_event'],
            'speech_teaching_target': event['target'],
            'actual_emitted': event['actual_emitted'],
            'has_food_target_candidate': any(c['target']['entity'] == meal_food for c in frame['candidates']),
        })
assert len(meal) == 5
assert meal[0]['physical_contact'] == 'Touch' and meal[0]['actual_event']['lesson'] == 'GetFood'
assert meal[1]['physical_contact'] == 'Consumed' and meal[1]['actual_event']['lesson'] == 'EatFood'
assert meal[2]['negative_taste'] > 0
assert meal[3]['positive_taste'] == meal[3]['negative_taste'] == 0
assert meal[4]['positive_taste'] == meal[4]['negative_taste'] == 0
assert all(r['speech_teaching_target']['token'] == 8 for r in meal[2:])
assert meal[4]['tick'] - meal[1]['tick'] == 3
assert all(not r['has_food_target_candidate'] for r in meal[2:])
vocal_recalls = [r for r in recall if r['action_id'] == 400]
assert all(r['target_memory_decoder_inputs'] == [0.] * 8 for r in vocal_recalls)
protected = json.load(open(args.protected))['protected_files']
changed = [p for p, digest in protected.items() if sha(p) != digest]
assert not changed, changed
assert all(sha(p) == digest for p, digest in inputs.items())
result = {
    'kind': 'read-only-native-outcome-to-speech-evidence-audit-v29',
    'new_world_decisions': 0, 'new_neural_forwards_or_gradient_probes': 0,
    'native_or_shadow_optimizer_updates': 0, 'external_model_calls': 0,
    'holding_observations': holding,
    'holding_receiving_neuron_original_gate_terms': terms,
    'pickup_candidate_memory_retained': memory,
    'actual_candidate_recall_inputs': recall,
    'source_memory_decoder_nonzero_base_weights': sum(w != 0 for w in actor['weights'][28672:32768]),
    'meal_and_delayed_label_observations': meal,
    'input_sha256': inputs,
    'protected_original_files_checked': len(protected),
    'all_byte_identical': not changed,
    'conclusion': 'Immediate holding/taste paths are active. Delayed exact event identity is compressed and has no dedicated direct episodic-to-speech feature. No implementation correction or new training is justified by this audit alone.',
    'limits': [
        'Observational term decomposition is not a sensory-channel ablation.',
        'No isolated acquired/consumed-event-to-speech test across sleep was captured or newly run.',
        'Existing test sources were reviewed, not rerun during this zero-decision audit.',
        'Architecture limitations do not identify the cause of the failed offline separator by themselves.',
    ],
}
with open(args.output, 'x') as stream:
    json.dump(result, stream, indent=2)
print(json.dumps({k: v for k, v in result.items() if k != 'input_sha256'}, indent=2))
