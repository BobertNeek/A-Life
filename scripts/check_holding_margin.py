"""One-scalar gate feasibility on existing captures; no actor or neural call.

The direction is the mean signed response to the two saved holding interventions,
unit normalized before looking at factual labels. Only a hypothetical presence
weight column could change. No proposed coefficient is installed or optimized.
"""
import argparse
import ctypes
import hashlib
import io
import json
import math
import struct
from pathlib import Path

import zstandard as zstd

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--workspace', type=Path, required=True)
parser.add_argument('--preregistration', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
inputs = {}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    inputs[str(path)] = sha(path)
    if path.name.endswith('.zst'):
        with zstd.ZstdDecompressor().stream_reader(path.open('rb')) as f:
            return json.load(f)
    return json.loads(path.read_text())


def dot(a, b):
    return math.fsum(x * y for x, y in zip(a, b))


def norm(x):
    return math.sqrt(dot(x, x))


def f32(x):
    # Typed Rust f32 fields can use a shorter JSON decimal than serde Value's
    # f64 presentation of the same f32. Restore the actual native lane bits.
    return struct.unpack('<f', struct.pack('<f', x))[0]


def interval(constraints):
    lower = [c for c in constraints if c['bound_kind'] == 'lower']
    upper = [c for c in constraints if c['bound_kind'] == 'upper']
    bad_constants = [c for c in constraints if c['bound_kind'] == 'impossible_constant']
    lo = max(lower, key=lambda c: c['lambda_bound']) if lower else None
    hi = min(upper, key=lambda c: c['lambda_bound']) if upper else None
    gap = lo['lambda_bound'] - hi['lambda_bound'] if lo and hi else None
    # At zero margin, supported tokens require a strictly positive presence;
    # unsupported tokens allow zero. Positive-margin calculations avoid ties.
    feasible = not bad_constants and (gap is None or gap < 0 or (
        gap == 0 and not lo['strict'] and not hi['strict']))
    return {'feasible': feasible, 'lower': lo, 'upper': hi,
            'lower_minus_upper': gap, 'impossible_constants': bad_constants}


prereg = read(args.preregistration)
for path, expected in prereg['protected_files'].items():
    assert sha(Path(path)) == expected, path

v30 = read(args.workspace / 'language-detached-encoder-v30-run/results.json')
assert v30['neural_forwards'] == 8
responses = []
for row in v30['rows']:
    conditions = {c['condition']: c for c in row['conditions']}
    baseline = list(map(f32, conditions['baseline']['speech_source_features']))
    toggled = list(map(f32, conditions['holding_toggle']['speech_source_features']))
    # Row 10 is 1->0; row 14 is 0->1. This sign uses raw input only.
    raw = next(p['holding_raw'] for p in v30['reconstruction'] if p['row'] == row['row'])
    responses.append([(b - a) / (1 - 2 * raw) for a, b in zip(baseline, toggled)])
mean = [(a + b) / 2 for a, b in zip(*responses)]
direction = [x / norm(mean) for x in mean]
assert len(direction) == 32 and abs(norm(direction) - 1) < 1e-12
# No factual label, intercept, margin or free coefficient is used above.

v26 = args.workspace / 'language-gate-retention-v26-run'
corpus = read(v26 / 'grounded-gate-corpus.json.zst')
v27 = read(args.workspace / 'language-gate-separability-v27-analysis/separability-results.json')
v25 = read(args.workspace / 'language-bounded-tape-v25-canonical-run/results.json')
assert len(corpus) == 5
assert v27['all_analysis_inputs_unchanged']
for path in [v26 / 'actor-Untouched.json.zst', v26 / 'grounded-gate-corpus.json.zst']:
    assert sha(path) == v27['input_sha256'][str(path)]

libm = ctypes.CDLL('libm.so.6')
libm.tanhf.argtypes = [ctypes.c_float]
libm.tanhf.restype = ctypes.c_float
examples = []


def from_captured_raw(features, raw):
    """Read captured decoder controls; no dense head or neural forward."""
    x = list(map(f32, features))
    steps = []
    for k, outputs in enumerate(raw):
        token = sum(1 << bit for bit in range(8) if outputs[8 + bit] > 0)
        steps.append({'step': k, 'features': x[:], 'presence': f32(outputs[16]),
                      'stop': f32(outputs[17]), 'token_bits': token})
        if outputs[17] > 0:
            break
        x[18:] = [float(libm.tanhf(v)) for v in outputs[18:]]
    assert steps and steps[-1]['stop'] > 0, 'uncaptured stop: abstain'
    return steps


for i, example in enumerate(corpus):
    meta = example['provenance']
    row = example['row']
    holding = row['patch']['pre_action']['perception']['base']['sensory']['channels']['tactile_contact'][2]
    rows = [(j, m) for j, m in enumerate(v27['row_metadata']) if m['example'] == i]
    assert rows[0][1]['native_row_sha256'] == meta['native_row_sha256']
    assert list(map(f32, row['final_features'][1856:1888])) == v27['features_X'][rows[0][0]]
    if example['positive']:
        assert len(rows) == 2
        tokens = [example['target']['token'], example['target']['continuation'][0]]
        steps = []
        for (j, m), token in zip(rows, tokens):
            steps.append({'step': m['speech_decode_step'], 'features': v27['features_X'][j],
                          'presence': v27['source_raw_gate_logits'][j][0],
                          'stop': v27['source_raw_gate_logits'][j][1], 'token_bits': token})
        assert steps[0]['stop'] < 0 and steps[1]['stop'] > 0
    else:
        witnessed = next(r for r in v25['records'] if
                         r['native_full_row_sha256'] == meta['native_row_sha256'])
        assert witnessed['prior_actual_event'] is None
        assert witnessed['owned_before'] == witnessed['owned_after'] == [None, None]
        assert witnessed['actual_food_consumption'] is False
        steps = from_captured_raw(row['final_features'][1856:1888],
                                  witnessed['raw_speech_outputs_six_steps'])
        assert steps[0]['presence'] == v27['source_raw_gate_logits'][rows[0][0]][0]
    assert all(s['presence'] > 0 and s['token_bits'] for s in steps)
    examples.append({'id': f'v26-{i}', 'seed': meta['world_seed'], 'positive': example['positive'],
                     'requested': meta['requested'], 'recorded_step': meta['step'],
                     'holding': holding, 'native_row_sha256': meta['native_row_sha256'],
                     'actual_prior_event': meta.get('actual_prior_event'), 'steps': steps})

v28root = args.workspace / 'language-within-seed-gate-v28-corrected-run'
for n in range(1, 17):
    item = read(v28root / f'native-decision-{n:02}.json.zst')
    record = item['record']
    if record['label'] not in ['supported_recent', 'unsupported_observed_prefix']:
        continue
    assert record['actual_matching_declare']
    positive = record['label'] == 'supported_recent'
    steps = from_captured_raw(record['source_decoder_inputs_32'],
                              record['raw_speech_outputs_six_steps'])
    holding = item['full_native_row']['patch']['pre_action']['perception']['base']['sensory']['channels']['tactile_contact'][2]
    examples.append({'id': f'v28-{n:02}', 'seed': record['seed'], 'positive': positive,
                     'requested': record['requested'], 'recorded_step': record['offset'],
                     'holding': holding, 'native_row_sha256': record['native_full_row_sha256'],
                     'actual_prior_event': record['recent_matching_support'], 'steps': steps})
assert len(examples) == 9 and sum(e['positive'] for e in examples) == 4


def constraints(margin, first_only=False):
    result = []
    for e in examples:
        for s in e['steps']:
            if first_only and s['step'] != 0:
                continue
            coefficient = dot(s['features'], direction)
            sign = 1 if e['positive'] else -1
            a = sign * coefficient
            b = margin - sign * s['presence']
            if a == 0:
                kind = 'satisfied_constant' if b <= 0 else 'impossible_constant'
                bound = None
            else:
                kind = 'lower' if a > 0 else 'upper'
                bound = b / a
            result.append({'id': e['id'], 'seed': e['seed'], 'positive': e['positive'],
                           'requested': e['requested'], 'holding': e['holding'],
                           'speech_step': s['step'], 'base_raw_presence': s['presence'],
                           'holding_direction_coefficient': coefficient,
                           'required_signed_margin': margin, 'bound_kind': kind,
                           'lambda_bound': bound, 'strict': e['positive'] and margin == 0})
    return result


analysis = []
for margin in [0., .01, .1]:
    rows = constraints(margin)
    per_seed = []
    for seed in sorted({e['seed'] for e in examples}):
        selected = [c for c in rows if c['seed'] == seed]
        per_seed.append({'seed': seed, 'constraints': selected, 'interval': interval(selected)})
    conflicts = []
    for lo in (c for c in rows if c['bound_kind'] == 'lower'):
        for hi in (c for c in rows if c['bound_kind'] == 'upper'):
            if lo['lambda_bound'] > hi['lambda_bound']:
                conflicts.append({'lower': lo, 'upper': hi,
                                  'gap': lo['lambda_bound'] - hi['lambda_bound']})
    analysis.append({'raw_margin': margin, 'full_silence_interval': interval(rows),
                     'weaker_first_step_only_interval': interval(constraints(margin, True)),
                     'per_seed': per_seed, 'conflicting_pair_count': len(conflicts),
                     'strongest_conflicting_pair': max(conflicts, key=lambda c: c['gap'])
                     if conflicts else None})

# Retained real reports with omitted neural inputs cannot enter an inequality.
unknown = []
path = v26 / 'hot-Untouched.jsonl.zst'
inputs[str(path)] = sha(path)
with zstd.ZstdDecompressor().stream_reader(path.open('rb')) as stream:
    for line in io.TextIOWrapper(stream):
        e = json.loads(line)
        if e.get('phase') != 'balanced_readout_evaluation':
            continue
        target = e.get('grounded_speech_target')
        if not target or target['token'] is None:
            continue
        wanted = [target['token'], target['continuation'][0]]
        if any(u['tokens'] == wanted for u in e['emitted']) and 'final_features' not in e['row']:
            unknown.append({'seed': e['world_seed'], 'post_sleep': e['post_sleep'],
                            'requested': e['requested'], 'actual_spoken_tokens': wanted,
                            'native_row_sha256': e['native_full_row_sha256'],
                            'reason': 'actual positive report; neural features not retained in this trace row'})

for path, expected in prereg['protected_files'].items():
    assert sha(Path(path)) == expected, path
assert all(sha(Path(path)) == expected for path, expected in inputs.items())
result = {
    'kind': 'read-only-holding-direction-one-scalar-margin-v31',
    'source_head': prereg['source_head'],
    'direction_without_labels': {'formula': prereg['direction'], 'unit_vector': direction,
                                 'nonzero_lanes': [i for i, x in enumerate(direction) if x != 0],
                                 'two_signed_response_norms': [norm(r) for r in responses],
                                 'signed_response_cosine': dot(*responses) / norm(responses[0]) / norm(responses[1])},
    'gate_model': 'raw_presence(lambda)=captured_raw_presence+lambda*dot(captured_decoder_input,d); no bias, no modified actor; word/act/stop/control weights and private offsets fixed',
    'later_steps': 'reuse recorded raw controls18..31 via native tanhf only; presence16 does not feed these controls; no dense head or neural forward',
    'sample': {'positive_tapes': 4, 'negative_tapes': 5, 'independent_seed_count': 4,
               'total_decoder_constraints': sum(len(e['steps']) for e in examples),
               'captured_positive_consumption_without_holding': ['v26-0'],
               'unknown_additional_positive_report_rows': unknown},
    'examples': examples, 'margin_analyses': analysis,
    'feasible_even_at_zero_margin': analysis[0]['full_silence_interval']['feasible'],
    'new_neural_forwards': 0, 'world_decisions': 0, 'gradients': 0,
    'native_or_shadow_optimizer_updates': 0, 'external_model_calls': 0,
    'installed_hypothetical_weights': False, 'protected_prior_files_exact': len(prereg['protected_files']),
    'input_sha256': inputs,
    'limits': ['Finite captured-state feasibility, not behavior or generalization.',
               'Real-arithmetic intervals use native captured f32 intercepts; no quantized hypothetical actor is created.',
               'Additional retained positive behaviors without neural features remain unknown; never recaptured.'],
}
with args.output.open('x') as f:
    json.dump(result, f, indent=2, sort_keys=True)
    f.write('\n')
print(json.dumps({'feasible_at_zero_margin': result['feasible_even_at_zero_margin'],
                  'full_interval_at_zero': analysis[0]['full_silence_interval'],
                  'first_step_only_at_zero': analysis[0]['weaker_first_step_only_interval'],
                  'per_seed_at_zero': [{'seed': p['seed'], 'interval': p['interval']} for p in analysis[0]['per_seed']],
                  'protected_prior_files': result['protected_prior_files_exact'],
                  'unknown_positive_reports': len(unknown)}, indent=2))
