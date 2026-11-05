"""Score the completed fixed native V32 trial; no simulation or optimizer call."""
import argparse
import hashlib
import io
import json
import math
from pathlib import Path
import struct

import zstandard as zstd

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--run', type=Path, required=True)
parser.add_argument('--measured', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
ARMS = ['Untouched', 'ZeroGradient4', 'Balanced4', 'MatchedFactual4']
WORDS = {'EatFood': [8, 1], 'GetFood': [9, 1], 'GetToy': [9, 2]}


def read(path):
    if path.name.endswith('.zst'):
        with zstd.ZstdDecompressor().stream_reader(path.open('rb')) as f:
            return json.load(f)
    return json.loads(path.read_text())


def sha(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def f32(x):
    return struct.unpack('<f', struct.pack('<f', x))[0]


def same_bits(a, b):
    return len(a) == len(b) and all(struct.pack('<f', x) == struct.pack('<f', y)
                                  for x, y in zip(a, b))


def l2(a, b):
    return math.sqrt(math.fsum((f32(x) - f32(y)) ** 2 for x, y in zip(a, b)))


complete = read(args.run / 'native-comparison-complete.json')
assert complete['scored_decisions'] == 704 and complete['new_native_adam_updates'] == 4
assert complete['awake_preparation_decisions'] == 6 and complete['sleep_requests'] == 12
assert complete['unexpected_awake_sleep_decisions'] == 0 and not complete['controls_retrained']
prereg = read(args.measured / 'preregistration.json')
provenance = read(args.measured / 'measured-provenance.json')
for path, expected in prereg['protected_files'].items():
    assert sha(Path(path)) == expected, path
assert sha(args.measured / 'native-test-measured') == provenance['binary_sha256']

old = Path('/workspace/language-gate-retention-v26-run')
initial = read(old / 'actor-Untouched.json.zst')
ids = [i for i, v in enumerate(initial['stage_mask']['words']) if v]
assert ids == [27648 + 32 * i + k for i in range(32) for k in [16, 17]]
before = initial
updates = []
for epoch in range(1, 5):
    cp = read(args.run / f'actor-MatchedFactual4-after-{epoch}.json.zst')
    g = read(args.run / f'gradient-MatchedFactual4-{epoch}.json.zst')
    frozen = [i for i in range(len(g)) if i not in ids]
    for name in ['weights', 'first_moment', 'second_moment']:
        assert same_bits([cp[name][i] for i in frozen], [initial[name][i] for i in frozen])
    assert [cp['update_ages'][i] for i in frozen] == [initial['update_ages'][i] for i in frozen]
    assert all(cp['update_ages'][i] == before['update_ages'][i] + 1 for i in ids)
    assert cp['optimizer_step'] == 436 + epoch
    assert cp['config'] == initial['config'] and cp['stage_mask'] == initial['stage_mask']
    assert all(g[i] == 0 for i in frozen) and any(g[i] != 0 for i in ids)
    updates.append({'epoch': epoch, 'optimizer_step': cp['optimizer_step'],
                    'changed_gate_weights': sum(not same_bits([cp['weights'][i]], [before['weights'][i]]) for i in ids),
                    'gate_delta_l2': l2([cp['weights'][i] for i in ids], [before['weights'][i] for i in ids]),
                    'all_frozen_weights_moments_integer_ages_bit_exact': True})
    before = cp
final = read(args.run / 'actor-MatchedFactual4.json.zst')
assert final == before
zero = read(old / 'actor-ZeroGradient4.json.zst')
target_effect = {'gate_weights_l2_from_source': l2([final['weights'][i] for i in ids], [initial['weights'][i] for i in ids]),
                 'gate_weights_l2_versus_matched_zero_gradient_control': l2([final['weights'][i] for i in ids], [zero['weights'][i] for i in ids]),
                 'first_moment_l2_versus_zero_gradient_control': l2([final['first_moment'][i] for i in ids], [zero['first_moment'][i] for i in ids]),
                 'actual_target_specific_difference': not same_bits(final['weights'], zero['weights'])}


def score_episode(e):
    rows = e['rows']
    assert len(rows) == 8
    requested = e['requested']
    counts = dict(utterances=0, taught_declarations=0, timely_latest_reports=0,
                  timely_recent_reports=0, requested_timely_latest_reports=0,
                  premature_observed_prefix=0, concurrent_unobserved=0,
                  stale_true_history=0, partial_taught_declarations=0, wrong_taught_activity_strings=0, other_utterances=0)
    spoken = []
    first_phrase = None
    for row in rows:
        events = row['prior_actual_events']
        tick = row['tick']
        for utterance in row['actual_emitted']:
            counts['utterances'] += 1
            tokens = utterance['tokens']
            lesson = next((k for k, w in WORDS.items() if w == tokens), None)
            label = 'other'
            if lesson and row['native_declare_selected']:
                counts['taught_declarations'] += 1
                if first_phrase is None:
                    first_phrase = lesson
                recent = [v for v in events if v['lesson'] == lesson and 0 < tick - v['tick'] <= 3]
                latest = bool(events and events[-1]['lesson'] == lesson and 0 < tick - events[-1]['tick'] <= 3)
                if latest:
                    label = 'timely_latest'
                    counts['timely_latest_reports'] += 1
                    counts['requested_timely_latest_reports'] += int(lesson == requested)
                elif recent:
                    label = 'timely_recent_history'
                elif any(v['lesson'] == lesson for v in events):
                    label = 'stale_true_history'
                    counts['stale_true_history'] += 1
                elif row['current_actual_event'] and row['current_actual_event']['lesson'] == lesson:
                    label = 'concurrent_unobserved'
                    counts['concurrent_unobserved'] += 1
                else:
                    label = 'premature_observed_prefix'
                    counts['premature_observed_prefix'] += 1
                counts['timely_recent_reports'] += int(bool(recent))
            elif row['native_declare_selected'] and tokens and any(t in [8, 9, 1, 2] for t in tokens):
                if len(tokens) < 2:
                    label = 'partial_taught_activity'
                    counts['partial_taught_declarations'] += 1
                elif tokens[0] in [8, 9]:
                    label = 'wrong_taught_activity_string'
                    counts['wrong_taught_activity_strings'] += 1
                else:
                    counts['other_utterances'] += 1
            else:
                counts['other_utterances'] += 1
            spoken.append({'step': row['step'], 'tokens': tokens, 'lesson': lesson, 'classification': label})
    meals = sum(r['food_consumed'] for r in rows)
    food = any(r['current_actual_event'] and r['current_actual_event']['lesson'] == 'GetFood' for r in rows)
    toy = any(r['current_actual_event'] and r['current_actual_event']['lesson'] == 'GetToy' for r in rows)
    physical = {'EatFood': bool(meals), 'GetFood': food and not toy, 'GetToy': toy and not food}.get(requested)
    return dict(counts, requested=requested, spoken=spoken, first_taught_phrase=first_phrase,
                meals=meals, food_acquired=food, toy_acquired=toy,
                command_appropriate_physical=physical,
                joint_physical_and_timely_report=bool(physical and counts['requested_timely_latest_reports']),
                final_owned=rows[-1]['owned_after'],
                silent_episode_utterances=counts['utterances'] if requested is None else 0,
                all_survive=all(r['alive'] for r in rows),
                minimum_energy=min(r['energy'] for r in rows), minimum_health=min(r['health'] for r in rows),
                maximum_injury=max(r['injury'] for r in rows), maximum_pain=max(r['pain'] for r in rows))


phases = []
all_ids = set()
traces = []
for path in sorted(args.run.glob('phase-*-results.json')):
    p = read(path)
    name = path.name.removesuffix('-results.json')
    recovery = read(args.run / f'{name}-fresh-recovery.json')
    assert recovery['actor_and_full_optimizer_exact'] and recovery['new_world_decisions'] == 0
    assert recovery['checkpoints_exact_in_fresh_process'] == len(p['checkpoints'])
    assert p['extra_awake_sleep_decisions'] == 0
    assert not p['post_sleep'] or p['consolidations'] > 0
    episodes = [score_episode(e) for e in p['episodes']]
    by_request = {e['requested']: e for e in episodes}
    noun = all(by_request[k]['command_appropriate_physical'] for k in ['GetFood', 'GetToy'])
    verb = bool(by_request['EatFood']['meals'] and by_request['GetFood']['food_acquired'] and not by_request['GetFood']['meals'])
    metrics = {}
    for k in ['utterances', 'taught_declarations', 'timely_latest_reports', 'timely_recent_reports',
              'requested_timely_latest_reports', 'premature_observed_prefix', 'concurrent_unobserved',
              'stale_true_history', 'partial_taught_declarations', 'wrong_taught_activity_strings', 'other_utterances', 'meals',
              'silent_episode_utterances']:
        metrics[k] = sum(e[k] for e in episodes)
    metrics.update(physical_noun_pair=bool(noun), physical_verb_pair=bool(verb),
                   joint_noun_pair=all(by_request[k]['joint_physical_and_timely_report'] for k in ['GetFood', 'GetToy']),
                   joint_verb_pair=bool(verb and all(by_request[k]['joint_physical_and_timely_report'] for k in ['EatFood', 'GetFood'])),
                   first_phrase_noun_distinction=all(by_request[k]['first_taught_phrase'] == k for k in ['GetFood', 'GetToy']),
                   first_phrase_verb_distinction=all(by_request[k]['first_taught_phrase'] == k for k in ['EatFood', 'GetFood']),
                   all_survive=all(e['all_survive'] for e in episodes),
                   minimum_energy=min(e['minimum_energy'] for e in episodes),
                   minimum_health=min(e['minimum_health'] for e in episodes),
                   maximum_injury=max(e['maximum_injury'] for e in episodes),
                   maximum_pain=max(e['maximum_pain'] for e in episodes))
    # Validate complete native evidence, not just the short outcome summary.
    trace_path = args.run / f'{name}-scored.jsonl.zst'
    expected = {r['scored_decision']: r for e in p['episodes'] for r in e['rows']}
    count = 0
    with zstd.ZstdDecompressor().stream_reader(trace_path.open('rb')) as stream:
        for line in io.TextIOWrapper(stream):
            row = json.loads(line)
            n = row['metrics']['scored_decision']
            assert n in expected and row['metrics'] == expected[n] and n not in all_ids
            assert 'final_features' in row['full_native_row'] and 'initial' in row['full_native_row']
            all_ids.add(n)
            count += 1
    assert count == len(expected)
    traces.append({'file': trace_path.name, 'sha256': sha(trace_path), 'full_native_rows': count})
    phases.append({'case': p['case'], 'seed': p['seed'], 'seen_retention': p['seen'], 'arm': p['arm'],
                   'optimizer_step': p['optimizer_step'], 'post_sleep': p['post_sleep'],
                   'sleep_world_ticks': p['sleep_world_ticks'], 'sleep_consolidations': p['consolidations'],
                   'fresh_recovery_checkpoints': recovery['checkpoints_exact_in_fresh_process'],
                   'metrics': metrics, 'episodes': episodes})
assert len(phases) == 24 and all_ids == set(range(1, 705))

# Compare actual outcomes and utterances, independently of scalar gate values.
behavior_equal = []
for case in range(3):
    for post in [False, True]:
        group = [p for p in phases if p['case'] == case and p['post_sleep'] == post]
        reference = next(p for p in group if p['arm'] == 'Untouched')
        behavior_equal.append({'case': case, 'post_sleep': post,
                               'all_arms_episode_metrics_identical': all(p['episodes'] == reference['episodes'] for p in group)})
seed_decisions = []
for seed in prereg['fresh_held_out_seeds']:
    grouped = {a: [p for p in phases if p['seed'] == seed and p['arm'] == a] for a in ARMS}
    totals = {a: {k: sum(p['metrics'][k] for p in ps) for k in [
        'premature_observed_prefix', 'partial_taught_declarations', 'wrong_taught_activity_strings', 'requested_timely_latest_reports',
        'meals', 'utterances']} for a, ps in grouped.items()}
    candidate = totals['MatchedFactual4']
    improved = all(candidate['premature_observed_prefix'] + candidate['partial_taught_declarations'] + candidate['wrong_taught_activity_strings']
                   < totals[a]['premature_observed_prefix'] + totals[a]['partial_taught_declarations'] + totals[a]['wrong_taught_activity_strings']
                   for a in ['ZeroGradient4', 'Balanced4'])
    retained = all(candidate['requested_timely_latest_reports'] >= totals[a]['requested_timely_latest_reports']
                   and candidate['meals'] >= totals[a]['meals'] for a in ['Untouched', 'ZeroGradient4', 'Balanced4'])
    seed_decisions.append({'seed': seed, 'paired_pre_post_totals': totals,
                           'unsupported_reporting_improved_against_both_trained_controls': improved,
                           'timely_reports_and_meals_retained_against_controls': retained})

gate_comparisons = []
for case in range(3):
    for post in [False, True]:
        base = read(args.run / f'phase-{case}-Untouched-{str(post).lower()}-results.json')
        for arm in ARMS[1:]:
            changed = read(args.run / f'phase-{case}-{arm}-{str(post).lower()}-results.json')
            dp, ds, premature = [], [], []
            payload_changes = 0
            for b, c in zip(base['episodes'], changed['episodes']):
                scored = score_episode(c)
                false_steps = {s['step'] for s in scored['spoken'] if s['classification'] == 'premature_observed_prefix'}
                for br, cr in zip(b['rows'], c['rows']):
                    bp, bs = br['raw_speech_outputs_six_steps'][0][16:18]
                    cp, cs = cr['raw_speech_outputs_six_steps'][0][16:18]
                    dp.append(cp - bp)
                    ds.append(cs - bs)
                    payload_changes += int(cr['conditional_native_payload_not_emission'] != br['conditional_native_payload_not_emission'])
                    if cr['step'] in false_steps:
                        premature.append({'step': cr['step'], 'requested': c['requested'],
                                          'untouched_first_presence_stop': [bp, bs],
                                          'arm_first_presence_stop': [cp, cs]})
            gate_comparisons.append({'case': case, 'post_sleep': post, 'arm': arm,
                                     'max_abs_first_presence_change': max(map(abs, dp)),
                                     'max_abs_first_stop_change': max(map(abs, ds)),
                                     'conditional_payload_changes_not_new_behavior': payload_changes,
                                     'actual_premature_report_gates': premature})

retention_checks = []
for p in (p for p in phases if p['arm'] == 'MatchedFactual4'):
    baseline = next(b for b in phases if b['case'] == p['case']
                    and b['post_sleep'] == p['post_sleep'] and b['arm'] == 'Untouched')
    care = all(c['meals'] >= b['meals'] and (not b['command_appropriate_physical']
               or c['command_appropriate_physical']) for c, b in zip(p['episodes'], baseline['episodes']))
    positive_reports = all(c['requested_timely_latest_reports'] >= b['requested_timely_latest_reports']
                           for c, b in zip(p['episodes'], baseline['episodes']))
    safe = (p['metrics']['all_survive'] and p['metrics']['maximum_injury'] <= baseline['metrics']['maximum_injury']
            and p['metrics']['minimum_health'] >= baseline['metrics']['minimum_health'])
    retention_checks.append({'seed': p['seed'], 'seen_retention': p['seen_retention'],
                             'post_sleep': p['post_sleep'], 'meal_and_physical_care_retained': care,
                             'genuine_timely_reports_retained': positive_reports,
                             'survival_health_injury_retained': safe})

result = {'kind': 'experimental-four-update-matched-factual-gate-curriculum-v32',
          'measured_source_head': provenance['source_head'], 'measured_binary_sha256': provenance['binary_sha256'],
          'execution_domain': 'native CPU; real world biology and recurrent/private learning',
          'budget': complete, 'setup': read(args.run / 'preparation-complete.json'),
          'known_positive_control': read(args.run / 'known-positive-control-passed.json'),
          'fresh_seeds': prereg['fresh_held_out_seeds'], 'seen_positive_seed': 10309219,
          'seed_exclusions': prereg['seed_exclusion_scan'],
          'updates': updates, 'native_gradient_receipt': read(args.run / 'four-updates-complete.json'),
          'supervision_effect_vs_momentum': target_effect,
          'protected_original_files_byte_exact': len(prereg['protected_files']),
          'all_176_private_checkpoints_recovered_exact_in_fresh_processes': sum(p['fresh_recovery_checkpoints'] for p in phases) == 176,
          'phase_results': phases, 'behavior_comparisons': behavior_equal, 'fresh_seed_decisions': seed_decisions,
          'actual_captured_gate_comparisons': gate_comparisons,
          'per_phase_positive_care_and_safety_retention': retention_checks,
          'candidate_rejected': not (all(s['unsupported_reporting_improved_against_both_trained_controls']
                                        and s['timely_reports_and_meals_retained_against_controls'] for s in seed_decisions)
                                     and all(c['meal_and_physical_care_retained']
                                             and c['genuine_timely_reports_retained']
                                             and c['survival_health_injury_retained'] for c in retention_checks)),
          'promotion': False, 'controls_retrained': False, 'external_models_or_jules': 0,
          'training_after_fixed_four_updates': 0, 'trace_receipts': traces,
          'receipt_bytes': sum(p.stat().st_size for p in args.run.rglob('*') if p.is_file()),
          'limitations': ['Two independent fresh seeds, one founder, familiar geometries; sleep phases are paired.',
                          'Seen retention seed is disclosed and is part of the teaching corpus.',
                          'Sleep branches start from common pre-assessment states; not longitudinal sleep after pretest episodes.',
                          'Unknown or partial taught vocabulary is reported separately; unlabeled rows are not absence training.',
                          'No generalization or comprehension claim from loss, readout differences or source checks.']}
with args.output.open('x') as f:
    json.dump(result, f, indent=2, sort_keys=True)
    f.write('\n')
print(json.dumps({'candidate_rejected': result['candidate_rejected'], 'fresh_seed_decisions': seed_decisions,
                  'behavior_comparisons': behavior_equal, 'target_effect': target_effect,
                  'protected_files': result['protected_original_files_byte_exact'],
                  'checkpoints_recovered': sum(p['fresh_recovery_checkpoints'] for p in phases),
                  'receipt_bytes': result['receipt_bytes']}, indent=2))
