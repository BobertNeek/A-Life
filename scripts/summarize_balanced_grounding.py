#!/usr/bin/env python3
"""Audit real balanced-curriculum rows; report six seed units separately before/after sleep."""
import collections
import io
import json
import pathlib
import sys
import zstandard
WORDS = {'EatFood': [8, 1], 'GetFood': [9, 1], 'GetToy': [9, 2]}

def audit(path):
    trials = {}
    protocols = set()
    for line in io.TextIOWrapper(zstandard.ZstdDecompressor().stream_reader(path.open('rb'))):
        event = json.loads(line)
        if event.get('phase') != 'balanced_readout_evaluation':
            continue
        protocols.add(event.get('speech_opportunity_protocol', 'quiet-after-cue-v1'))
        row = event['row']
        seed = event['world_seed']
        key = (seed, event['post_sleep'], event['requested'])
        tick = row['patch']['pre_action']['tick']
        t = trials.setdefault(key, {
            'case': event['case'],
            'first_tick': tick,
            'rows': 0,
            'full_native_row_digest_count': 0,
            'food_acquisition_steps': [],
            'toy_acquisition_steps': [],
            'food_consumption_steps': [],
            'nourished_consumption_steps': [],
            'first_acquisition': None,
            'last_owners': None,
            'utterances': 0,
            'utterance_events': [],
            'recent_actual_activity_declarations': 0,
            'premature_requested_phrase_declarations': 0,
            'declarations': 0,
            'timely_factual_declarations': 0,
            'timely_requested_activity_declarations': 0,
            'requested_activity_eligible_rows': 0,
            'eligible_rows': 0,
            'token_sequences': collections.Counter()})
        step = tick - t['first_tick']
        safety = event.get('physical_safety_after_native_tick')
        if safety is not None:
            current = t.setdefault('physical_safety', {'recorded_ticks': 0, 'all_alive': True, 'min_health': 1., 'max_injury': 0., 'min_energy': 1., 'max_pain': 0.})
            current['recorded_ticks'] += 1
            current['all_alive'] &= safety['alive']
            current['min_health'] = min(current['min_health'], safety['health'])
            current['max_injury'] = max(current['max_injury'], safety['injury'])
            current['min_energy'] = min(current['min_energy'], safety['energy'])
            current['max_pain'] = max(current['max_pain'], safety['pain'])
        t['rows'] += 1
        if 'native_full_row_sha256' in event:
            assert len(event['native_full_row_sha256']) == 64
            t['full_native_row_digest_count'] += 1
        organism = row['organism_id']
        for i, kind in enumerate(['food', 'toy']):
            if event['owned_before'][i] != organism and event['owned_after'][i] == organism:
                t[kind + '_acquisition_steps'].append(step)
                if t['first_acquisition'] is None:
                    t['first_acquisition'] = kind
        joint = row['patch']['outcome'].get('joint') or {}
        consumed = any((o['physical']['target_entity'] == event['scene']['food'] and o['physical']['contact'] == 'Consumed' for o in joint.get('channel_outcomes', [])))
        assert consumed == event['actual_food_consumption']
        if consumed:
            t['food_consumption_steps'].append(step)
            if row['patch']['outcome']['energy_delta'] > 0:
                t['nourished_consumption_steps'].append(step)
        t['last_owners'] = event['owned_after']
        target = event['grounded_speech_target']
        if target is not None:
            if event.get('speech_opportunity_protocol') == 'event-confirmed-cue-window-v1':
                assert 0 <= step < 8
                assert event['narration_evidence']['lesson'] == event['requested']
                assert event['narration_evidence']['tick'] < tick
            else:
                assert step >= 8
            assert event['narration_evidence'] is not None
            assert WORDS[event['narration_evidence']['lesson']] == [target['token'], target['continuation'][0]]
            t['eligible_rows'] += 1
            t['requested_activity_eligible_rows'] += event['narration_evidence']['lesson'] == event['requested']
        bundle = row['patch']['decision']['selected_bundle']
        payloads = [c['payload']['values'] for c in bundle['channels'] if c['channel'] == 'Vocal'] if bundle else []
        for u in event['emitted']:
            assert u['source_kind'] == 'Creature' and u['speaker_id'] == organism
            assert payloads and payloads[0][0] == 1397248049 and (u['tokens'] == payloads[0][4:])
            declare = payloads[0][1] == 0
            t['utterances'] += 1
            t['declarations'] += declare
            evidence = event['narration_evidence']
            age = tick - evidence['tick'] if evidence else None
            recent = bool(declare and evidence and 0 < age <= 3 and u['tokens'] == WORDS[evidence['lesson']])
            t['recent_actual_activity_declarations'] += recent
            requested = event['requested']
            completed = (t['food_consumption_steps'] if requested == 'EatFood' else t['food_acquisition_steps'] if requested == 'GetFood' else t['toy_acquisition_steps'] if requested == 'GetToy' else [])
            premature = bool(declare and requested and u['tokens'] == WORDS[requested] and not any(s < step for s in completed))
            t['premature_requested_phrase_declarations'] += premature
            t['utterance_events'].append({'step': step, 'tokens': u['tokens'], 'declare': declare, 'recent_actual_activity_match': recent, 'premature_requested_phrase': premature, 'latest_actual_activity': evidence['lesson'] if evidence else None, 'actual_activity_age': age})
            t['token_sequences'][' '.join(map(str, u['tokens']))] += 1
            timely = bool(declare and target and (u['tokens'] == [target['token'], target['continuation'][0]]))
            t['timely_factual_declarations'] += timely
            t['timely_requested_activity_declarations'] += bool(timely and event['narration_evidence']['lesson'] == event['requested'])
    rows = [{'world_seed': k[0], 'post_sleep': k[1], 'requested': k[2], **v} for k, v in trials.items()]
    summaries = {}
    for post_sleep in [False, True]:
        phase = [t for t in rows if t['post_sleep'] == post_sleep]
        by_seed = collections.defaultdict(dict)
        for t in phase:
            by_seed[t['world_seed']][t['requested']] = t
        pairs = []
        for seed, g in by_seed.items():
            assert len(g) == 4
            eat, food, toy = (g['EatFood'], g['GetFood'], g['GetToy'])
            full = bool(eat['food_consumption_steps']) and bool(food['food_acquisition_steps']) and (not food['food_consumption_steps'])
            cue = any((x < 8 for x in eat['food_consumption_steps'])) and any((x < 8 for x in food['food_acquisition_steps'])) and (not any((x < 8 for x in food['food_consumption_steps'])))
            noun = food['first_acquisition'] == 'food' and toy['first_acquisition'] == 'toy'
            eat_first = eat['utterance_events'][0] if eat['utterance_events'] else None
            food_first = food['utterance_events'][0] if food['utterance_events'] else None
            toy_first = toy['utterance_events'][0] if toy['utterance_events'] else None
            pairs.append({'world_seed': seed,
                'physical_verb_full_window': full,
                'first_spoken_verb_pair_differs': bool(eat_first and food_first and eat_first['tokens'] != food_first['tokens']),
                'first_spoken_noun_pair_differs': bool(food_first and toy_first and food_first['tokens'] != toy_first['tokens']),
                'timely_speech_verb_pair': eat['timely_requested_activity_declarations'] > 0 and food['timely_requested_activity_declarations'] > 0,
                'timely_speech_noun_pair': food['timely_requested_activity_declarations'] > 0 and toy['timely_requested_activity_declarations'] > 0,
                'physical_verb_cue_window': cue,
                'physical_noun_first_acquisition': noun,
                'physical_noun_cue_window': noun and bool(food['food_acquisition_steps']) and (food['food_acquisition_steps'][0] < 8) and bool(toy['toy_acquisition_steps']) and (toy['toy_acquisition_steps'][0] < 8),
                'eat_food_consumption_steps': eat['food_consumption_steps'],
                'get_food_consumption_steps': food['food_consumption_steps'],
                'get_food_acquisition_steps': food['food_acquisition_steps'],
                'get_toy_acquisition_steps': toy['toy_acquisition_steps']})
        summaries['post_sleep' if post_sleep else 'pre_sleep'] = {
            'independent_seed_units': len(by_seed),
            'requested_trials': len(by_seed) * 3,
            'verb_pairs_full_window': sum((p['physical_verb_full_window'] for p in pairs)),
            'verb_pairs_cue_window': sum((p['physical_verb_cue_window'] for p in pairs)),
            'noun_pairs_first_acquisition': sum((p['physical_noun_first_acquisition'] for p in pairs)),
            'noun_pairs_cue_window': sum((p['physical_noun_cue_window'] for p in pairs)),
            'actual_utterances': sum((t['utterances'] for t in phase)),
            'first_spoken_verb_pairs_differ': sum(p['first_spoken_verb_pair_differs'] for p in pairs),
            'first_spoken_noun_pairs_differ': sum(p['first_spoken_noun_pair_differs'] for p in pairs),
            'timely_speech_verb_pairs': sum(p['timely_speech_verb_pair'] for p in pairs),
            'timely_speech_noun_pairs': sum(p['timely_speech_noun_pair'] for p in pairs),
            'joint_physical_speech_verb_pairs': sum(p['physical_verb_full_window'] and p['timely_speech_verb_pair'] for p in pairs),
            'joint_physical_speech_noun_pairs': sum(p['physical_noun_first_acquisition'] and p['timely_speech_noun_pair'] for p in pairs),
            'recent_actual_activity_declarations': sum(t['recent_actual_activity_declarations'] for t in phase),
            'premature_requested_phrase_declarations': sum(t['premature_requested_phrase_declarations'] for t in phase),
            'timely_factual_declarations': sum((t['timely_factual_declarations'] for t in phase)),
            'eligible_speech_rows': sum((t['eligible_rows'] for t in phase)),
            'timely_requested_activity_declarations': sum((t['timely_requested_activity_declarations'] for t in phase)),
            'requested_activity_eligible_rows': sum((t['requested_activity_eligible_rows'] for t in phase)),
            'nourished_meals': sum((len(t['nourished_consumption_steps']) for t in phase)),
            'native_rows': sum((t['rows'] for t in phase)),
            'full_native_row_digests': sum(t['full_native_row_digest_count'] for t in phase),
            'by_seed': pairs}
    return add_actual_first_phrase_metrics({'kind': 'independent-native-ownership-ingestion-speech-audit',
        'speech_opportunity_protocols': sorted(protocols),
        'independent_N_counts_seeds_once': len({t['world_seed'] for t in rows}),
        'pre_post_not_independent_replicates': True,
        'summaries': summaries,
        'trials': rows})

def add_actual_first_phrase_metrics(result):
    """Literal spoken cue mapping; deliberately separate from event-grounded success."""
    for phase, summary in result['summaries'].items():
        trials = [t for t in result['trials'] if t['post_sleep'] == (phase == 'post_sleep')]
        by_seed = collections.defaultdict(dict)
        for trial in trials:
            by_seed[trial['world_seed']][trial['requested']] = trial
        def first_matches(trial):
            first = trial['utterance_events'][0] if trial['utterance_events'] else None
            return bool(first and first['declare'] and first['step'] < 8 and
                        first['tokens'] == WORDS.get(trial['requested']))
        summary['first_spoken_requested_verb_pairs_exact_in_cue_window'] = sum(
            first_matches(g['EatFood']) and first_matches(g['GetFood']) for g in by_seed.values())
        summary['first_spoken_requested_noun_pairs_exact_in_cue_window'] = sum(
            first_matches(g['GetFood']) and first_matches(g['GetToy']) for g in by_seed.values())
        summary['first_spoken_phrase_matches_by_request_in_cue_window'] = {
            request: sum(first_matches(g[request]) for g in by_seed.values()) for request in WORDS}
        summary['silent_first_declare_is_taught_phrase'] = sum(
            bool(g[None]['utterance_events'] and g[None]['utterance_events'][0]['declare'] and
                 g[None]['utterance_events'][0]['tokens'] in WORDS.values()) for g in by_seed.values())
        silent = [g[None] for g in by_seed.values()]
        silent_words = [u for t in silent for u in t['utterance_events']]
        if any('physical_safety' in t for t in trials):
            assert all(t['physical_safety']['recorded_ticks'] == 24 for t in trials)
            summary['physical_safety'] = {
                'trials': len(trials),
                'all_alive_every_recorded_tick': all(t['physical_safety']['all_alive'] for t in trials),
                'min_health': min(t['physical_safety']['min_health'] for t in trials),
                'max_injury': max(t['physical_safety']['max_injury'] for t in trials),
                'min_energy': min(t['physical_safety']['min_energy'] for t in trials),
                'max_pain': max(t['physical_safety']['max_pain'] for t in trials)}
        summary['silence'] = {
            'trials': len(silent),
            'actual_utterances': sum(t['utterances'] for t in silent),
            'taught_phrase_declarations': sum(u['declare'] and u['tokens'] in WORDS.values() for u in silent_words),
            'recent_actual_activity_declarations': sum(t['recent_actual_activity_declarations'] for t in silent),
            'taught_phrase_without_recent_matching_activity': sum(
                u['declare'] and u['tokens'] in WORDS.values() and not u['recent_actual_activity_match'] for u in silent_words),
            'nourished_meals': sum(len(t['nourished_consumption_steps']) for t in silent),
            'first_acquisition_food_trials': sum(t['first_acquisition'] == 'food' for t in silent),
            'first_acquisition_toy_trials': sum(t['first_acquisition'] == 'toy' for t in silent)}
    return result

if __name__ == '__main__':
    directory = pathlib.Path(sys.argv[1])
    names = sys.argv[2:] or ['Untreated', 'Grounded', 'Shuffled', 'NoSpeech']
    print(json.dumps({'arms': {name: audit(directory / f'recovery-{name}' / 'reloaded-evaluation.jsonl.zst') for name in names}}, indent=2))
