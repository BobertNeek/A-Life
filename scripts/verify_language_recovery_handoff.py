"""Verify an existing metadata handoff; never construct a trainer or world.

Reads only manifest-selected files. No archives, payload copies, historical
discovery scan, external calls, neural inference, simulator or Adam execution.
"""
import argparse
import hashlib
import json
from pathlib import Path

import zstandard as zstd

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('manifest', type=Path)
parser.add_argument('--profile', default='actor_history')
parser.add_argument('--rescore', action='store_true')
parser.add_argument('--require-joint-critic', action='store_true')
parser.add_argument('--root-map', action='append', default=[], metavar='NAME=PATH')
args = parser.parse_args()
m = json.loads(args.manifest.read_text())
assert m['schema'] == 1
for override in args.root_map:
    key, value = override.split('=', 1)
    assert key in m['roots'], f'unknown root: {key}'
    m['roots'][key] = value
profile = m['profiles'][args.profile]
selected = set(profile['files'])


def path(fid):
    v = m['files'][fid]
    return Path(m['roots'][v['root']]) / v['path']


def read(fid):
    assert fid in selected, f'{fid} is not in this dependency profile'
    p = path(fid)
    if p.name.endswith('.zst'):
        with zstd.ZstdDecompressor().stream_reader(p.open('rb')) as f:
            return json.load(f)
    return json.loads(p.read_text())


for fid in sorted(selected):
    f = m['files'][fid]
    assert set(f['requires']) <= selected, f'incomplete dependency closure for {fid}'
    p = path(fid)
    assert p.stat().st_size == f['bytes'], f'size mismatch: {fid}'
    with p.open('rb') as stream:
        assert hashlib.file_digest(stream, 'sha256').hexdigest() == f['sha256'], f'hash mismatch: {fid}'
    if f.get('native_typed_identity_sha256'):
        # These files were written directly with serde_json::to_writer(typed_cp).
        # The decompressed bytes equal serde_json::to_vec used by identity().
        # This verifies the recorded native identity without an engine call.
        with zstd.ZstdDecompressor().stream_reader(p.open('rb')) as stream:
            assert hashlib.file_digest(stream, 'sha256').hexdigest() == f['native_typed_identity_sha256'], f'native identity mismatch: {fid}'
assert sum(m['files'][fid]['bytes'] for fid in selected) == profile['bytes']
original = read(m['original']['recovery_file'])
assert not any('critic' in key.lower() for key in original)
assert path(m['original']['checksum_file']).read_text() == m['files'][m['original']['recovery_file']]['sha256']

# Hash the original lexical subtrees, retaining Rust's field order and exact f32
# formatting. json.dumps of the parsed object would not reproduce this identity.
text = path(m['original']['recovery_file']).read_text()
decoder = json.JSONDecoder()
cursor = 1
found = {}
while cursor < len(text):
    while text[cursor] in ' \n\r\t,':
        cursor += 1
    if text[cursor] == '}':
        break
    key, cursor = decoder.raw_decode(text, cursor)
    while text[cursor] in ' \n\r\t:':
        cursor += 1
    start = cursor
    _, cursor = decoder.raw_decode(text, cursor)
    if key in ['actor', 'host']:
        found[key] = hashlib.sha256(text[start:cursor].encode()).hexdigest()
assert found['actor'] == m['original']['native_actor_identity_sha256']
assert found['host'] == m['original']['native_private_identity_sha256']


def actor_shape(cp):
    assert cp['schema_version'] == 2
    n = len(cp['weights'])
    assert n and all(len(cp[k]) == n for k in ['first_moment', 'second_moment', 'update_ages'])
    assert cp['phenotype_hash'] == original['actor']['phenotype_hash']
    assert cp['source_foundation_digest'] == original['actor']['source_foundation_digest']


actor_shape(original['actor'])
actors_checked = []
for arm, v in m['actors'].items():
    if v['actor_file'] not in selected:
        continue
    cp = read(v['actor_file'])
    actor_shape(cp)
    assert cp['optimizer_step'] == v['step']
    actors_checked.append(arm)
    for u in v['updates']:
        if not {u['before'], u['gradient'], u['after']} <= selected:
            continue
        before, after = read(u['before']), read(u['after'])
        actor_shape(after)
        assert after['optimizer_step'] == before['optimizer_step'] + 1
        assert len(read(u['gradient'])) == len(after['weights'])
        assert after['config'] == before['config'] and after['stage_mask'] == before['stage_mask']

result = {'profile': args.profile, 'selected_files_byte_exact': len(selected),
          'selected_existing_file_bytes': profile['bytes'], 'actor_shapes_checked': actors_checked,
          'complete_joint_actor_critic_handoff': False,
          'critic_status': m['critic']['reason'],
          'new_native_restore_or_neural_world_optimizer_calls': 0,
          'payload_copied_archived_transferred_or_changed': 0,
          'independent_trained_state_backup_verified': False,
          'semantic_native_recovery_not_reexecuted': True}
result['saved_native_identities_verified_as_json_bytes'] = 2 + sum(bool(m['files'][fid].get('native_typed_identity_sha256')) for fid in selected)

if args.rescore:
    evidence = m['evidence']
    needed = set(evidence['phase_result_ids'] + evidence['cold_receipt_ids'] + [evidence['source_report_id']])
    assert needed <= selected, 'rescore requires compact_evidence or a superset'
    report = read(evidence['source_report_id'])
    cold_count = 0
    for fid in evidence['cold_receipt_ids']:
        v = read(fid)
        assert v['actor_and_full_optimizer_exact'] and v['new_world_decisions'] == 0
        cold_count += v['checkpoints_exact_in_fresh_process']
    assert cold_count == 176
    words = {(8, 1): 'EatFood', (9, 1): 'GetFood', (9, 2): 'GetToy'}
    ids = set()
    phases = []
    for fid in evidence['phase_result_ids']:
        p = read(fid)
        counts = dict(meals=0, utterances=0, premature_observed_prefix=0,
                      requested_timely_latest_reports=0)
        for e in p['episodes']:
            assert len(e['rows']) == 8
            for row in e['rows']:
                n = row['scored_decision']
                assert n not in ids
                ids.add(n)
                counts['meals'] += row['food_consumed']
                counts['utterances'] += len(row['actual_emitted'])
                for u in row['actual_emitted']:
                    lesson = words.get(tuple(u['tokens']))
                    if not lesson or not row['native_declare_selected']:
                        continue
                    events, tick = row['prior_actual_events'], row['tick']
                    latest = bool(events and events[-1]['lesson'] == lesson
                                  and 0 < tick - events[-1]['tick'] <= 3)
                    counts['requested_timely_latest_reports'] += int(latest and lesson == e['requested'])
                    historical = any(v['lesson'] == lesson for v in events)
                    concurrent = bool(row['current_actual_event'] and row['current_actual_event']['lesson'] == lesson)
                    counts['premature_observed_prefix'] += int(not historical and not concurrent)
        original_phase = next(v for v in report['phase_results'] if v['case'] == p['case']
                              and v['arm'] == p['arm'] and v['post_sleep'] == p['post_sleep'])
        assert all(original_phase['metrics'][k] == value for k, value in counts.items())
        phases.append(dict(seed=p['seed'], seen=p['seen'], arm=p['arm'], post=p['post_sleep'], **counts))
    assert ids == set(range(1, 705)) and len(phases) == 24
    fresh = []
    for seed in evidence['fresh_seeds']:
        totals = {arm: {k: sum(p[k] for p in phases if p['seed'] == seed and p['arm'] == arm)
                        for k in ['meals', 'utterances', 'premature_observed_prefix', 'requested_timely_latest_reports']}
                  for arm in m['actors']}
        assert all(v == totals['Untouched'] for v in totals.values())
        fresh.append({'seed': seed, 'all_four_arms_identical': True, 'per_arm_paired_totals': totals['Untouched']})
    assert report['candidate_rejected']
    result.update(saved_negative_scores_reproduced=True, scored_rows_checked=704,
                  previous_cold_receipt_checkpoint_count=176, fresh_seed_totals=fresh,
                  native_trace_to_summary_semantics='Prior V32 agreement receipt cited; no full trace decoded here.')

print(json.dumps(result, indent=2))
if args.require_joint_critic:
    raise SystemExit('Joint actor/critic continuity unavailable: this lineage saved no matching critic/version state.')
