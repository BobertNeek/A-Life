"""Summarize eight already-recorded native forwards, with no neural/world calls.

Publishes selected readouts and hashes, never actors, optimizer arrays, detached
full neural inputs or canonical checkpoints. No fit or hypothetical weights.
"""
import argparse
import hashlib
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--run', type=Path, required=True)
parser.add_argument('--measured', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text())


r = read(args.run / 'results.json')
provenance = read(args.measured / 'measured-provenance.json')
prereg = read(args.measured / 'preregistration.json')
assert r['kind'] == 'experimental-detached-native-speech-inputs-v30'
assert r['neural_forwards'] == 8
for key in ['world_decisions', 'gradient_probes',
            'native_or_shadow_optimizer_updates', 'provider_or_jules_calls']:
    assert r[key] == 0
assert r['actor_optimizer_and_private_state_exact']
assert sha(args.measured / 'native-test-measured') == provenance['executable_sha256']
assert sha(args.measured / 'preregistration.json') == provenance['preregistration_sha256']
for path, expected in prereg['protected_files'].items():
    assert sha(Path(path)) == expected, path
assert len(prereg['protected_files']) == r['protected_prior_files_exact']
attempts = sorted(args.run.glob('forward-attempt-*.json'))
assert len(attempts) == 8
assert [read(p)['reserved_forward'] for p in attempts] == list(range(1, 9))
assert [read(p)['condition'] for p in attempts[:2]] == ['baseline', 'baseline']
assert read(args.run / 'preflight-complete.json')['neural_forwards_so_far'] == 0
assert read(args.run / 'both-baselines-exact.json')[
    'activations_ema_metabolic_raw_speech_payload_and_sampled_joint_action_exact']

rows = []
for proof, row in zip(r['reconstruction'], r['rows']):
    assert proof['row'] == row['row']
    original = row['recorded_outcome_unchanged']
    conditions = {c['condition']: c for c in row['conditions']}
    assert list(conditions) == ['baseline', 'holding_toggle', 'candidate_memory_zero', 'both']
    baseline = conditions['baseline']
    holding = conditions['holding_toggle']
    memory = conditions['candidate_memory_zero']
    both = conditions['both']
    reports = []
    for label, c in conditions.items():
        changed_actions = [
            dict(a, delta_from_baseline=a['logit'] - b['logit'])
            for a, b in zip(c['action_logits'], baseline['action_logits'])
            if a['logit'] != b['logit']
        ]
        gates = [dict(g,
                      presence_delta=g['presence_raw'] - b['presence_raw'],
                      stop_delta=g['stop_raw'] - b['stop_raw'])
                 for g, b in zip(c['gates'], baseline['gates'])]
        payload = c['conditional_native_payload']
        reports.append({
            'condition': label,
            'speech_source_neurons': c['speech_source_neurons'],
            'all_32_speech_source_features': c['speech_source_features'],
            'speech_source_delta': c['speech_source_delta'],
            'all_six_raw_speech_steps': c['raw_speech_outputs_six_steps'],
            'presence_stop_six_steps': gates,
            'conditional_payload': payload,
            'same_tokens_and_act_as_baseline': (
                payload['tokens'] == baseline['conditional_native_payload']['tokens']
                and payload['speech_act'] == baseline['conditional_native_payload']['speech_act']),
            'payload_including_confidence_exact_to_baseline': (
                payload == baseline['conditional_native_payload']),
            'action_logits': c['action_logits'],
            'action_logit_delta': c['action_logit_delta'],
            'changed_action_logits': changed_actions,
            'changed_memory_lanes': c.get('candidate_memory_changed_lanes', []),
            'new_executed_action_or_utterance': False,
        })
    rows.append({
        'source_row': row['row'], 'seed': original['seed'],
        'original_label': original['label'],
        'original_tick': original['tick'], 'original_heard_tokens': original['first_two_heard_slots'],
        'original_owned_before': original['owned_before'],
        'original_owned_after': original['owned_after'],
        'original_recent_matching_support': original['recent_matching_support'],
        'original_food_consumption': original['actual_food_consumption'],
        'original_emitted': original['actual_emitted'],
        'original_cooldowns': [original['prompted_cooldown_ready'],
                               original['unprompted_cooldown_ready']],
        'native_row_sha256': proof['native_row_sha256'],
        'canonical_frame_sha256': proof['frame_sha256'],
        'recall_receipt_sha256': proof['memory_recall_receipt_sha256'],
        'receptor_effects': proof['receptor_effects'],
        'canonical_and_native_encoder_exact': proof['normal_admission_and_canonical_preparation_exact']
        and proof['captured_inputs_and_native_encoder_bit_exact'],
        'holding_raw': proof['holding_raw'], 'holding_toggled': proof['holding_toggled'],
        'holding_assignments': proof['holding_assignments'],
        'affected_encoder_targets': proof['affected_encoder_targets'],
        'native_encoder_default_microsteps': proof['native_encoder_default_microsteps'],
        'actual_captured_activity_microsteps': proof['captured_activity_microsteps'],
        'memory_only_speech_exact_to_baseline': all(
            memory[k] == baseline[k] for k in ['speech_source_features',
                                               'raw_speech_outputs_six_steps',
                                               'conditional_native_payload']),
        'both_speech_exact_to_holding_only': all(
            both[k] == holding[k] for k in ['speech_source_features',
                                            'raw_speech_outputs_six_steps',
                                            'conditional_native_payload']),
        'conditions': reports,
    })

summary = {
    'kind': r['kind'], 'measured_source_head': provenance['source_head'],
    'measured_executable_sha256': provenance['executable_sha256'],
    'source_files': provenance['source_files'],
    'diagnostic_body_wall_seconds': r['elapsed_seconds'],
    'native_forwards': 8, 'world_decisions': 0, 'gradients': 0,
    'native_or_shadow_optimizer_updates': 0, 'external_model_or_jules_calls': 0,
    'source_actor_full_checkpoint_identity_sha256': r['actor_sha256'],
    'source_actor_optimizer_step': 436,
    'original_actor_adam_private_and_prior_evidence_exact': True,
    'protected_prior_file_count': len(prereg['protected_files']),
    'protected_hash_map_file_sha256': sha(args.measured / 'preregistration.json'),
    'baseline_reproduced': [
        'normal checkpoint admission and exact cold recovery',
        'canonical perception, receptor effects, native encoder and candidate inputs',
        'initial neural state and effective private offsets',
        'final activations, activity EMA and metabolic load',
        'six raw speech steps, conditional payload including confidence',
        'joint action selection and exact joint sampling log probability',
    ],
    'native_action_logits_original_capture_limit': (
        'Original raw action-logit arrays were not recorded; reconstructed baseline '
        'reproduces original joint action and log probability exactly.'),
    'rows': rows,
    'limitations': r['limitations'] + [
        'Raw steps after the native stop are diagnostic decoder recurrence, not emitted tokens.',
        'Holding is not object identity or event recency; the override can conflict with unchanged physical observations.',
        'Memory ablation holds canonical attention and cognitive projection fixed; historical/indirect memory effects are not removed.',
        'No new behavior, nourishment, safety, sleep retention or learning is assessed.',
    ],
    'input_results_sha256': sha(args.run / 'results.json'),
}
with args.output.open('x') as f:
    json.dump(summary, f, indent=2, sort_keys=True)
    f.write('\n')
print(json.dumps({'output': str(args.output), 'sha256': sha(args.output),
                  'forwards_analyzed': 8,
                  'prior_files_exact': len(prereg['protected_files']),
                  'new_neural_or_world_calls': 0}))
