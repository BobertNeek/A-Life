#!/usr/bin/env python3
"""Describe real frozen cue episodes; reused swap/mute contrasts are not new samples."""
import collections
import io
import json
import pathlib
import sys
import zstandard
from summarize_balanced_grounding import audit, WORDS

GOALS = ['EatFood', 'GetFood', 'GetToy']
MAPS = {
    'normal': {g: g for g in GOALS},
    'muted': {g: None for g in GOALS},
    'noun_swapped': {'GetFood': 'GetToy', 'GetToy': 'GetFood'},
    'verb_swapped': {'EatFood': 'GetFood', 'GetFood': 'EatFood'},
}

def score(t, goal):
    nourished = t['nourished_consumption_steps']
    events = t['food_consumption_steps'] if goal == 'EatFood' else t['food_acquisition_steps'] if goal == 'GetFood' else t['toy_acquisition_steps']
    def appropriate(window):
        if goal == 'EatFood':
            return any(s < window for s in nourished)
        noun = 'food' if goal == 'GetFood' else 'toy'
        return bool(t['first_acquisition'] == noun and any(s < window for s in events)
                    and (goal != 'GetFood' or not any(s < window for s in t['food_consumption_steps'])))
    first = t['utterance_events'][0] if t['utterance_events'] else None
    match = bool(first and first['declare'] and first['step'] < 8 and first['tokens'] == WORDS[goal])
    matching = [u for u in t['utterance_events'] if u['declare'] and u['tokens'] == WORDS[goal]]
    timely = sum(u['step'] < 8 and u['recent_actual_activity_match'] for u in matching)
    premature = sum(not any(s < u['step'] for s in events) for u in matching)
    noun = 'toy' if goal == 'GetToy' else 'food'
    acquired = t['toy_acquisition_steps'] if noun == 'toy' else t['food_acquisition_steps']
    return {'first_acquisition_nominal_object8': bool(t['first_acquisition'] == noun and any(x < 8 for x in acquired)), 'physical_appropriate8': appropriate(8), 'physical_appropriate24': appropriate(24),
            'nourished8': any(s < 8 for s in nourished), 'nourished24': bool(nourished),
            'first_spoken_nominal_phrase8': match, 'timely_nominal_declarations8': timely,
            'premature_nominal_declarations': premature,
            'food_acquisition_steps': t['food_acquisition_steps'],
            'toy_acquisition_steps': t['toy_acquisition_steps'],
            'nourished_consumption_steps': nourished,
            'actual_utterance_events': t['utterance_events']}

def analyze_model(directory, name, cases):
    path = directory / f'recovery-{name}' / 'reloaded-evaluation.jsonl.zst'
    strata = {'known': {c['seed'] for c in cases if c['stratum'] != 'fresh'},
              'fresh': {c['seed'] for c in cases if c['stratum'] == 'fresh'}}
    complete = audit(path)
    result = {'all_actual_episodes': complete, 'strata': {}}
    for label, seeds in strata.items():
        observed = audit(path, seeds)
        indexed = {(t['world_seed'], t['post_sleep'], t['requested']): t for t in observed['trials']}
        modes = {}
        for mode, mapping in MAPS.items():
            phases = {}
            for post in [False, True]:
                units = []
                for seed in sorted(seeds):
                    scores = {goal: score(indexed[(seed, post, delivered)], goal) for goal, delivered in mapping.items()}
                    noun = all(scores[g]['first_acquisition_nominal_object8'] for g in ['GetFood', 'GetToy']) if all(g in scores for g in ['GetFood', 'GetToy']) else None
                    verb8 = all(scores[g]['physical_appropriate8'] for g in ['EatFood', 'GetFood']) if all(g in scores for g in ['EatFood', 'GetFood']) else None
                    verb24 = all(scores[g]['physical_appropriate24'] for g in ['EatFood', 'GetFood']) if verb8 is not None else None
                    spoken_noun = all(scores[g]['first_spoken_nominal_phrase8'] for g in ['GetFood', 'GetToy']) if noun is not None else None
                    spoken_verb = all(scores[g]['first_spoken_nominal_phrase8'] for g in ['EatFood', 'GetFood']) if verb8 is not None else None
                    joint_noun = bool(noun and all(scores[g]['timely_nominal_declarations8'] for g in ['GetFood', 'GetToy'])) if noun is not None else None
                    joint_verb = bool(verb24 and all(scores[g]['timely_nominal_declarations8'] for g in ['EatFood', 'GetFood'])) if verb24 is not None else None
                    units.append({'world_seed': seed, 'scores_by_nominal_goal': scores,
                                  'physical_noun_pair8': noun, 'physical_verb_pair8': verb8,
                                  'physical_verb_pair24': verb24, 'first_spoken_noun_pair8': spoken_noun,
                                  'first_spoken_verb_pair8': spoken_verb,
                                  'joint_timely_noun_pair8': joint_noun,
                                  'joint_timely_strict_verb_pair': joint_verb})
                keys = ['physical_noun_pair8', 'physical_verb_pair8', 'physical_verb_pair24',
                        'first_spoken_noun_pair8', 'first_spoken_verb_pair8',
                        'joint_timely_noun_pair8', 'joint_timely_strict_verb_pair']
                totals = {k: sum(bool(u[k]) for u in units) if any(u[k] is not None for u in units) else None for k in keys}
                totals['by_nominal_goal'] = {g: {k: sum(bool(u['scores_by_nominal_goal'][g][k]) for u in units) for k in ['first_acquisition_nominal_object8', 'physical_appropriate8', 'physical_appropriate24', 'nourished8', 'nourished24', 'first_spoken_nominal_phrase8']} | {k: sum(u['scores_by_nominal_goal'][g][k] for u in units) for k in ['timely_nominal_declarations8', 'premature_nominal_declarations']} for g in mapping}
                phases['post_sleep' if post else 'pre_sleep'] = {'seed_units': len(seeds), 'totals': totals, 'per_seed': units}
            modes[mode] = phases
        result['strata'][label] = {'seeds': sorted(seeds), 'actual_episode_audit': observed, 'nominal_contrasts_reuse_actual_episodes': modes}
    first, comparisons = [], []
    for line in io.TextIOWrapper(zstandard.ZstdDecompressor().stream_reader(path.open('rb'))):
        e = json.loads(line)
        if e.get('phase') == 'frozen_causal_pair_inputs':
            comparisons.append(e)
        elif e.get('phase') == 'balanced_readout_evaluation' and 'causal_input_evidence' in e:
            p = e['causal_input_evidence']
            assert p['setup']['all_nonaudio_physical_private_optimizer_binding_and_rng_fields_identical']
            first.append({'world_seed': e['world_seed'], 'post_sleep': e['post_sleep'], 'delivered_cue': e['requested'], 'causal_stratum': e['causal_stratum'], 'evidence': p})
    assert len(first) == 48 and len(comparisons) == 12
    for key in {(p['world_seed'], p['post_sleep']) for p in first}:
        group = [p for p in first if (p['world_seed'], p['post_sleep']) == key]
        assert len(group) == 4
        assert len({p['evidence']['setup']['before_audio_full_checkpoint_sha256'] for p in group}) == 1
        assert len({p['evidence']['setup']['nonaudio_checkpoint_sha256'] for p in group}) == 1
    result['actual_input_and_setup_verification'] = {'first_rows': first, 'pairwise_first_inputs': comparisons, 'all_matched_parent_and_nonaudio_setup_checks_passed': True}
    return result

if __name__ == '__main__':
    directory = pathlib.Path(sys.argv[1])
    cases = json.loads((directory / 'cases.json').read_text())
    names = sys.argv[2:] or ['Untreated', 'CareOnly4', 'MixedGrounded4', 'MixedNoCommand4']
    print(json.dumps({'kind': 'frozen-native-cue-causal-audit', 'unique_actual_cues_per_parent': 4,
                      'swap_and_muted_contrasts_are_not_extra_samples': True,
                      'models': {name: analyze_model(directory, name, cases) for name in names}}, indent=2))
