"""SOURCE-ONLY proposal. Strict PARTIAL receipt admission, never study admission.

Place beside the unchanged reviewed report_contract.py. No executable entrypoint,
filesystem reads, checkpoint input, native launch, retry, or completion rewrite.
"""
from report_contract import (
    ASSET, B, SOURCE, ROW, arr, enum, f32, integer, maybe, need, obj,
    ownership, actual_event, parse,
)

SEED = 0x57535635
STAGES = (
    'before_construction', 'asset_construction', 'fresh_construction', 'preparation',
    'prepared_boundary', 'restore', 'initial_identity', 'ordinary_host',
    'audited_host', 'paired_state', 'replay', 'projection', 'post_projection',
    'safeguards', 'finished',
)
CODES = (
    'native_operation', 'io', 'serialization', 'cpu_host', 'invariant',
    'asset_binding', 'actual_replay', 'founder_mutation', 'transfer_execution',
    'failed_transfer', 'meal_receipt', 'meal_channel', 'ownership', 'target_binding',
    'sequence_binding', 'population', 'clock', 'preservation', 'deadline',
    'resource', 'output_bound', 'handshake', 'accounting', 'safety_stop',
)
CAPS = dict(asset_construction=1, fresh_construction=1, restore=1,
            preparation=600, ordinary_host=4, audited_host=4, replay=4, projection=4)
COUNT = lambda n: obj(requested=integer(0, n), returned=integer(0, n))
PROGRESS = obj(stage=enum(*STAGES), **{k: COUNT(v) for k, v in CAPS.items()},
               paired_states_equal=integer(0, 4), projection_states_unchanged=integer(0, 4),
               rows_collected=integer(0, 4))
EVIDENCE = obj(asset_bound=B, initial_checkpoint_equal=B,
    initial_body_biology_equal=B, initial_scene_equal=B, initial_sampling_equal=B,
    trainer_unchanged=B, nontrivial_neural_activity=B, real_decisions=integer(0, 4),
    candidate_sets=integer(0, 4), manipulation_channels=integer(0, 4),
    manipulation_transfers=integer(0, 8), held_h_safe_eat_candidates=integer(0, 4),
    held_h_safe_eat_attempts=integer(0, 4), held_h_safe_atomic_swaps=integer(0, 4))
RECEIPT = obj(schema_version=enum(1),
    scope=enum('prepared_fixed_fixture_not_a_study_case'), status=enum('PARTIAL'),
    outcome=enum('fixture_passed', 'fixture_rejected'), fixture_seed=enum(SEED),
    world=enum('old_world', 'new_world'), execution_domain=enum('offline_cpu'),
    study_completed=enum(False), strict_validation=enum('pending'),
    asset_sha256=enum(ASSET), evidence=EVIDENCE, rows=arr(ROW, 4),
    manipulation_coverage=enum('actual_transfers_observed', 'no_actual_transfers_observed'),
    progress=PROGRESS, code=maybe(enum(*CODES)), source=SOURCE)


def validate_partial_rows(rows):
    """The reviewed ROW contract with prefix clocks, not 256-tick stop logic."""
    counter = 0
    sequence = 0
    prior_physical = set()
    last_boundary = {'h': 'nobody', 's': 'nobody'}
    decisions = channels = transfers = candidate_sets = 0
    opportunities = attempts = swaps = 0
    for index, row in enumerate(rows, 1):
        need(row['primary_tick'] == index and row['world_tick_before'] == 599 + index
             and row['world_tick_after'] == 600 + index, 'clock')
        need(row['alive'] and f32(row['health']) >= f32(.70)
             and f32(row['injury']) <= f32(.25), 'safety_stop')
        expected_boundary = last_boundary.copy()
        ownership(row['boundary_owners'])
        need(not row['sleeping'] or not row['decisions'], 'sleep_decision')
        for d in row['decisions']:
            decisions += 1
            candidate_sets += bool(d['candidates'])
            need(d['actual_replay_equal'] and d['sampling_counter'] == counter
                 and d['sequence'] > sequence, 'sampling')
            counter += 1
            sequence = d['sequence']
            current, after = ownership(d['owners_before']), ownership(d['owners_after'])
            if current['h'] == 'self':
                opportunities += any(c['primitive'] == 210 and c['target'] == 's'
                                     for c in d['candidates'])
                attempts += any(c['slot'] == 2 and c['primitive'] == 210 and c['target'] == 's'
                                for c in d['channels'])
                swaps += any(c['slot'] == 2 and c['target'] == 's' and c['event'] == 'atomic_swap'
                             for c in d['channels'])
            need(current == last_boundary, 'boundary_continuity')
            need([c['index'] for c in d['candidates']] == list(range(len(d['candidates']))),
                 'candidate_order')
            for c in d['candidates']:
                if c['source_exact_ingest']:
                    need((c['source_sequence'], c['target']) in prior_physical
                         and c['family_sources'] > 0, 'recall_provenance')
            seen = set()
            observed = [False, False]
            for c in d['channels']:
                need(c['slot'] not in seen, 'channel_duplicate')
                seen.add(c['slot'])
                need(c['candidate'] < len(d['candidates']), 'candidate_index')
                selected = d['candidates'][c['candidate']]
                need((selected['primitive'], selected['target']) == (c['primitive'], c['target']),
                     'selected_target')
                event = actual_event(c)
                if c['slot'] == 2:
                    channels += 1
                    transfers += len(c['transfers'])
                for t in c['transfers']:
                    need(current[t['target']] == t['previous_owner'], 'owner_before')
                    if t['kind'] == 'acquired':
                        need(t['previous_owner'] == 'nobody' and t['new_owner'] == 'self',
                             'owner_transfer')
                    else:
                        need(t['previous_owner'] == 'self' and t['new_owner'] == 'nobody',
                             'owner_transfer')
                    current[t['target']] = 'consumed' if t['kind'] == 'consumed' else t['new_owner']
                if event == 'consumed':
                    need(c['target'] in ('h', 's'), 'meal_target')
                    observed[0 if c['target'] == 'h' else 1] = True
            need(current == after, 'owner_after')
            expected_boundary = after.copy()
            need(d['physical_meals'] == observed, 'meal_count')
            for key in ('physical_meals', 'ingest_episode_active', 'ingest_episode_staged'):
                need(len(d[key]) == 2, 'coverage')
            for i in range(2):
                need(not (d['ingest_episode_active'][i] or d['ingest_episode_staged'][i])
                     or observed[i], 'coverage')
                if observed[i]:
                    prior_physical.add((sequence, 'h' if i == 0 else 's'))
        last_boundary = ownership(row['boundary_owners'])
        for role in ('h', 's'):
            need(last_boundary[role] == expected_boundary[role]
                 or (expected_boundary[role], last_boundary[role]) == ('consumed', 'nobody'),
                 'unreceipted_boundary_transfer')
        need(row['sampling_counter_after'] == counter, 'sampling')
    return dict(real_decisions=decisions, candidate_sets=candidate_sets,
                manipulation_channels=channels, manipulation_transfers=transfers,
                held_h_safe_eat_candidates=opportunities, held_h_safe_eat_attempts=attempts,
                held_h_safe_atomic_swaps=swaps)


def validate_prepared_fixture(raw, expected_world, expected_source):
    """Admit only a scalar prefix; return value remains marked PARTIAL/pending.

    Caller provides the pre-dispatch verified world/source/executable binding.
    This function does not trust a process's environment variable as that proof.
    Rejection output can be well formed, but is never a passed fixture.
    """
    need(type(raw) is bytes and 0 < len(raw) <= 256 * 1024, 'output_bound')
    x = parse(raw, RECEIPT)
    need(expected_world in ('old_world', 'new_world') and x['world'] == expected_world
         and x['source'] == expected_source, 'source_binding')
    p, e = x['progress'], x['evidence']
    for name in CAPS:
        c = p[name]
        need(c['returned'] <= c['requested'] <= c['returned'] + 1, 'accounting')
    n = len(x['rows'])
    need(p['rows_collected'] == n, 'row_count')
    need(n <= p['projection_states_unchanged'] <= p['projection']['returned']
         <= p['projection']['requested'] <= p['paired_states_equal']
         <= p['audited_host']['returned'] <= p['audited_host']['requested']
         <= p['ordinary_host']['returned'] <= p['ordinary_host']['requested']
         <= min(4, n + 1), 'accounting')
    need(p['replay']['requested'] <= p['audited_host']['returned'], 'accounting')
    if p['fresh_construction']['requested']:
        need(p['asset_construction']['returned'] == 1, 'accounting')
    if p['fresh_construction']['returned']:
        need(p['preparation']['requested'] == p['preparation']['returned'] == 600,
             'accounting')
    if e['asset_bound']:
        need(p['asset_construction']['returned'] == 1, 'preservation')
    if any(e[k] for k in ('initial_checkpoint_equal', 'initial_body_biology_equal',
                        'initial_scene_equal', 'initial_sampling_equal', 'trainer_unchanged')):
        need(p['restore']['returned'] == 1 and e['asset_bound'], 'preservation')
    if e['nontrivial_neural_activity']:
        need(n == 4 and e['real_decisions'] > 0, 'nontrivial_activity')
    if p['ordinary_host']['requested']:
        need(p['asset_construction']['returned'] == 1
             and p['fresh_construction']['returned'] == 1 and p['restore']['returned'] == 1
             and p['preparation']['requested'] == p['preparation']['returned'] == 600,
             'accounting')
        need(all(e[k] for k in ('asset_bound', 'initial_checkpoint_equal',
             'initial_body_biology_equal', 'initial_scene_equal', 'initial_sampling_equal',
             'trainer_unchanged')), 'preservation')
    if p['restore']['requested']:
        need(p['fresh_construction']['returned'] == 1
             and p['preparation']['returned'] == 600, 'accounting')
    if p['preparation']['requested']:
        need(p['fresh_construction']['requested'] == 1
             and p['asset_construction']['returned'] == 1, 'accounting')
    observed = validate_partial_rows(x['rows'])
    # Only collected, admitted scalar rows can justify behavioral evidence.
    for key, value in observed.items():
        need(e[key] == value, 'coverage')
    need(observed['real_decisions'] <= p['replay']['returned'] <= p['replay']['requested'],
         'accounting')
    need(x['manipulation_coverage'] == ('actual_transfers_observed'
         if e['manipulation_transfers'] else 'no_actual_transfers_observed'), 'coverage')
    if x['outcome'] == 'fixture_passed':
        need(x['code'] is None and p['stage'] == 'finished' and n == 4, 'partial_completion')
        for name in CAPS:
            need(p[name]['requested'] == p[name]['returned'], 'accounting')
        need(p['ordinary_host']['returned'] == p['audited_host']['returned']
             == p['projection']['returned'] == p['paired_states_equal']
             == p['projection_states_unchanged'] == 4, 'accounting')
        need(e['nontrivial_neural_activity'] and e['real_decisions'] > 0
             and e['candidate_sets'] > 0, 'nontrivial_activity')
        need(p['replay']['returned'] == e['real_decisions']
             and all(e[k] == v for k, v in observed.items()), 'accounting')
    else:
        need(x['code'] is not None, 'rejection_code')
    return x
