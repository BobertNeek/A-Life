"""Analyze LOCAL diagnostic wrappers; never admit rows to a trainer."""
import argparse
import hashlib
import json
import math
import struct
from collections import Counter
from pathlib import Path

PARENT = 'd83a995e802e41b8e17799b0871f75d9ede35f95850049777323112e636654d1'
GENOME = '557da394c045d0b4353009c842825677aa7505f636d6252b20f67bfe0c3a5189'
WRAPPER = 'terminal_context2135_research_record_v1'


def load(path):
    return json.loads(path.read_text())


def enum_name(value):
    return value if isinstance(value, str) else next(iter(value))


def unwrap(value):
    if WRAPPER in value:
        assert set(value) == {WRAPPER}
        row = value[WRAPPER]
        assert row.get('terminal_context2135') is not None
        return row
    assert value.get('terminal_context2135') is None
    return value


def event(interval, sequence, **fields):
    return dict(host_interval_zero_based=interval, sequence_id=sequence, **fields)


def biology(cp):
    records = cp['world']['world']['organism_records']
    assert len(records) == 1
    return records[0]['biochemistry']


def analyze_arm(path):
    request = load(path / 'REQUEST.json')
    provenance = load(path / 'PROVENANCE.json')
    assert provenance['parent_sha256'] == PARENT
    assert provenance['construction_genome_sha256'] == GENOME
    completed = (path / 'COMPLETED.json').exists()
    result = load(path / 'COMPLETED.json')['summary'] if completed else None
    sources, signatures, meals, source_uses, arrivals = {}, {}, [], [], {}
    read_admission, observation_admission = Counter(), Counter()
    toy_ingests = toy_manipulations = toy_contacts = ingest_commands = nonzero = unfunded = work = 0
    max_delta = 0.0
    failed_ingest_bundles = 0
    first_nonzero = first_capacity = last_capacity = None
    batch_totals = Counter()
    record_without_consumption = invalid_source_uses = 0
    trace_paths = sorted(path.glob('TRACE-*.json'))
    for index, trace_path in enumerate(trace_paths):
        try:
            trace = load(trace_path)
        except json.JSONDecodeError:
            assert not completed and index == len(trace_paths) - 1
            break  # A live writer may have created its final file, not finished it.
        interval = trace['host_interval']
        assert interval == index
        for entity in trace['food_entity_ids']:
            arrivals.setdefault(entity, interval)
        decisions = [unwrap(r) for r in trace['decisions']]
        step_signature = dict(state=trace['state_without_recipe_sha256'], actions=[], motor_choice=[], representative_choice=[], logp=[], logits=[])
        for row in decisions:
            patch = row['patch']
            frame = patch['pre_action']['perception']['base']
            candidates = frame['candidates']
            sequence = patch['header']['sequence_id']
            bundle = patch['decision']['selected_bundle']
            channels = {c['channel']: c for c in bundle['channels']}
            outcomes = {c['channel']: c['physical'] for c in patch['outcome']['joint']['channel_outcomes']}
            manipulation = channels.get('Manipulation', {})
            toy_manipulations += (manipulation.get('target') or {}).get('entity') == trace['toy_entity_id']
            toy_contacts += outcomes.get('Manipulation', {}).get('target_entity') == trace['toy_entity_id'] and outcomes['Manipulation']['contact'] in ['Touch', 'Blocked', 'Collision']
            choices = [candidates[i] for i in row['action']['motor_candidates'] if i is not None]
            ingests = [c for c in choices if c['family'] == 'Ingest']
            ingest_commands += len(ingests)
            failed_ingest_bundles += bool(ingests) and not patch['outcome']['success']
            toy_ingests += sum(c['target']['entity'] == trace['toy_entity_id'] for c in ingests)
            consumed = outcomes.get('Manipulation', {}).get('contact') == 'Consumed'
            representative = candidates[row['action']['representative']]
            if consumed:
                physical = outcomes['Manipulation']
                physiology = patch['outcome']['measured_physiology']
                meals.append(event(interval, sequence,
                    elapsed_host_intervals=interval + 1,
                    target_entity=physical['target_entity'],
                    representative_family=representative['family'],
                    representative_action_id=representative['action_id'],
                    manipulation_action_id=channels['Manipulation']['primitive'],
                    hunger_delta=physiology['homeostatic_delta']['drives']['hunger']))
            context = row.get('terminal_context2135')
            if context:
                assert context['config']['schema_version'] == request.get('recipe_version', 3)
                assert context['config']['parent_sha256'] == PARENT
                assert context['config']['construction_genome_sha256'] == GENOME
                read_admission[enum_name(context['admission'])] += 1
                if enum_name(context['admission']) == 'BankCapacityExceeded':
                    denied = context['admission']['BankCapacityExceeded']
                    assert denied['records'] > denied['limit'] == 64
                    assert not context['candidates']
                    last_capacity = event(interval, sequence, **denied)
                    first_capacity = first_capacity or last_capacity
                if enum_name(context['admission']) == 'InsufficientAtp':
                    denied = context['admission']['InsufficientAtp']
                    assert denied['required_atp_q16'] > denied['available_atp_q16']
                    assert not context['candidates']
                if context.get('batch_work'):
                    stats = context['batch_work']
                    if enum_name(context['admission']) == 'Admitted':
                        assert stats['bank_exports'] == stats['reader_bank_validations'] == stats['combined_digest_builds'] == 1
                        assert stats['failed_contact_index_records'] == stats['ordinary_records']
                        assert stats['channel_contract_validations'] == stats['channel_records']
                        assert stats['query_contract_validations'] == len(context['candidates'])
                        assert stats['query_source_visits'] == stats['channel_records'] * len(context['candidates'])
                        assert stats['appearance_comparisons'] <= 9 * stats['query_source_visits']
                    else:
                        assert stats['bank_exports'] == stats['reader_bank_validations'] == stats['combined_digest_builds'] == 0
                    for key, value in stats.items(): batch_totals[key] += value
                unfunded += context['admission_check_unfunded_atp_q16']
                work += context['reserved_work_units']
                observation = context['observation']
                observation_admission[enum_name(observation['admission'])] += 1
                recorded = observation['recorded']
                if recorded:
                    record_without_consumption += not consumed
                    assert consumed and recorded['channel'] == 'Manipulation'
                    assert recorded['sequence_id'] == sequence and sequence not in sources
                    assert recorded['action_id'] == channels['Manipulation']['primitive']
                    assert outcomes['Manipulation']['target_entity'] == channels['Manipulation']['target']['entity']
                    assert outcomes['Manipulation']['target_entity'] in trace['food_entity_ids']
                    source_candidate = candidates[row['action']['motor_candidates'][2]]
                    assert source_candidate['family'] == 'Ingest'
                    slot = source_candidate['observation']['ObjectSlot']
                    witnessed = next(s for s in frame['grounded_object_slots'] if s['slot_index'] == slot)
                    assert witnessed['tracked_object_id'] == recorded['tracked_object_id']
                    sources[sequence] = event(interval, sequence,
                        target_entity=outcomes['Manipulation']['target_entity'],
                        tracked_object_id=recorded['tracked_object_id'],
                        action_id=recorded['action_id'],
                        observation_digest=recorded['observation_digest'],
                        representative_family=representative['family'])
                for c in context['candidates']:
                    delta = c['delta']
                    assert math.isfinite(delta) and abs(delta) <= 0.5
                    if delta != 0:
                        nonzero += 1
                        first_nonzero = first_nonzero or event(interval, sequence, delta=delta)
                    max_delta = max(max_delta, abs(delta))
                    for ref in c.get('source_channel_observations', []):
                        known = sources.get(ref['sequence_id'])
                        valid = known is not None and ref['sequence_id'] < sequence
                        valid = valid and ref['observation_digest'] == known['observation_digest']
                        valid = valid and ref['action_id'] == known['action_id']
                        valid = valid and ref['tracked_object_id'] == known['tracked_object_id']
                        valid = valid and ref['channel'] == 'Manipulation'
                        invalid_source_uses += not valid
                        assert valid, 'source use must follow a genuine earlier committed event'
                        target = candidates[c['candidate_index']]
                        assert target['family'] == 'Approach'
                        fresh = target['target']['entity'] != known['target_entity']
                        source_uses.append(event(interval, sequence,
                            source_sequence_id=ref['sequence_id'],
                            target_entity=target['target']['entity'],
                            fresh_target=fresh,
                            target_arrival_interval=arrivals.get(target['target']['entity']),
                            new_scheduled_food_target=target['target']['entity'] in arrivals and arrivals[target['target']['entity']] >= known['host_interval_zero_based'],
                            desirability=c['desirability'], delta=delta))
            step_signature['actions'].append(dict(
                representative=row['action']['representative'],
                motor_candidates=row['action']['motor_candidates'],
                committed_bundle=bundle))
            step_signature['motor_choice'].append([(c['channel'], c['primitive'], (c.get('target') or {}).get('entity')) for c in bundle['channels']])
            selected = patch['decision']['selected_action']
            step_signature['representative_choice'].append((selected['action_id'], selected.get('target_entity')))
            step_signature['logp'].append(row['action']['old_joint_log_probability'])
            step_signature['logits'].append(patch['decision']['evidence']['NeuralClosedLoopCpu']['logit'])
        signatures[interval] = step_signature
    birth = load(path / 'PREPARED-BIRTH.cp.json')
    born = biology(birth)
    recipe = birth.get('terminal_context2135')
    actual_gain = struct.unpack('<f', bytes(recipe['head_recipe'][-4:]))[0] if recipe else None
    assert actual_gain == (None if request['arm'] == 'off' else 0 if request['arm'] == 'zero' else request['gain'])
    public = dict(evidence_id=path.name, arm=request['arm'], seed=request['seed'], completed=completed,
        actual_recipe_gain=actual_gain, mask_new_context=recipe['mask_new_context'] if recipe else None,
        compiled_phenotype_hash=recipe['compiled_phenotype_hash'] if recipe else None,
        compiler_inputs_digest=recipe['compiler_inputs_digest'] if recipe else None,
        trace_intervals=len(signatures), decisions=result['decisions'] if result else sum(len(s['actions']) for s in signatures.values()),
        first_consumption=meals[0] if meals else None, meals=len(meals), meal_intervals=[m['host_interval_zero_based'] for m in meals],
        first_authenticated_source=next(iter(sources.values()), None), authenticated_sources=len(sources),
        first_source_use=source_uses[0] if source_uses else None,
        first_fresh_target_source_use=next((u for u in source_uses if u['fresh_target']), None),
        first_new_scheduled_food_target_source_use=next((u for u in source_uses if u['new_scheduled_food_target']), None),
        new_scheduled_food_target_source_uses=sum(u['new_scheduled_food_target'] for u in source_uses),
        fresh_target_source_uses=sum(u['fresh_target'] for u in source_uses), source_uses=len(source_uses),
        first_nonzero_head_contribution=first_nonzero, nonzero_candidate_contributions=nonzero, max_abs_delta=max_delta,
        first_capacity_rejection=first_capacity, last_capacity_rejection=last_capacity,
        read_admission=dict(read_admission), observation_admission=dict(observation_admission),
        batch_stage_totals=dict(batch_totals),
        unfunded_checks_atp_q16=unfunded, reserved_research_work_units=work,
        ingest_commands=ingest_commands, failed_ingest_containing_bundles=failed_ingest_bundles,
        toy_ingest_commands=toy_ingests,
        toy_manipulation_commands=toy_manipulations, toy_physical_contacts=toy_contacts,
        initial_energy=born['body']['energy'], initial_atp=born['homeostasis']['drives']['brain_atp'],
        initial_hunger=born['homeostasis']['drives']['hunger'],
        record_without_consumption=record_without_consumption, invalid_source_uses=invalid_source_uses,
        source_provenance=provenance.get('compile_commit'), recipe_version=recipe['schema_version'] if recipe else None)
    if completed:
        final = load(path / 'FINAL.cp.json')
        end = biology(final)
        assert result['parent_weights_exact'] and result['typed_cold_exact']
        assert result['decisions'] == sum(len(s['actions']) for s in signatures.values())
        assert result['meals'] == len(meals)
        assert result['failed_ingest_containing_bundles'] == failed_ingest_bundles
        horizon = 256
        consumed_by_target = {m['target_entity']: m['host_interval_zero_based'] for m in meals}
        opportunities = [dict(target_entity=entity, arrival_interval=start,
            consumed_interval=consumed_by_target.get(entity),
            censored=entity not in consumed_by_target or consumed_by_target[entity] >= horizon,
            time_to_meal_or_censor=min(consumed_by_target.get(entity, horizon), horizon) - start)
            for entity, start in sorted(arrivals.items()) if start < horizon]
        public.update(food_opportunities=opportunities, censor_horizon_host_intervals=horizon,
            summed_time_to_meal_or_censor=sum(v['time_to_meal_or_censor'] for v in opportunities))
        founder_before = (path / 'FOUNDER-OPTIMIZER-BIRTH.json').read_bytes()
        founder_after = (path / 'FOUNDER-OPTIMIZER-FINAL.json').read_bytes()
        assert founder_before == founder_after
        acquired = {}
        for name in ['fast', 'eligibility', 'lifetime']:
            a, b = birth['residents'][0]['neural'][name], final['residents'][0]['neural'][name]
            acquired[name + '_changed_coordinates'] = sum(x != y for x, y in zip(a, b))
        public.update(final_energy=end['body']['energy'], final_atp=end['homeostasis']['drives']['brain_atp'],
            final_health=end['body']['health'], founder_optimizer_unchanged=True,
            founder_checkpoint_sha256=hashlib.sha256(founder_before).hexdigest(),
            cold_exact=True, final_channel_observations=len(final['residents'][0].get('terminal_channel2135', {}).get('observations', [])),
            ordinary_learning=acquired, summary=result)
        public['decisions_after_first_capacity_rejection'] = result['decisions'] - first_capacity['sequence_id'] if first_capacity else None
    return public, signatures


def compare(a, b, signatures_a, signatures_b):
    common = sorted(set(signatures_a) & set(signatures_b))
    origins = [x['first_authenticated_source']['host_interval_zero_based'] for x in [a, b] if x['first_authenticated_source']]
    source_boundary = min(origins) if origins else len(common)
    differences = {}
    for label, key in [('state', 'state'), ('actions', 'motor_choice'), ('representative_choice', 'representative_choice'), ('committed_bundle_details', 'actions'), ('log_probability', 'logp'), ('representative_logit', 'logits')]:
        mismatch = [i for i in common if signatures_a[i][key] != signatures_b[i][key]]
        differences[label] = dict(count=len(mismatch), first_interval=mismatch[0] if mismatch else None,
            before_first_source=sum(i < source_boundary for i in mismatch))
    return dict(arms=[a['arm'], b['arm']], common_intervals=len(common),
        first_source_boundary=source_boundary, differences=differences)


def summarize_group(arms, traces):
    comparisons = []
    for first, second in [('zero', 'mask'), ('active', 'zero'), ('active', 'mask'), ('off', 'zero')]:
        if first in arms and second in arms:
            comparisons.append(compare(arms[first], arms[second], traces[first], traces[second]))
    complete = set(arms) >= {'zero', 'active', 'mask'} and all(a['completed'] for a in arms.values())
    gate = complete and arms['active']['new_scheduled_food_target_source_uses'] > 0 and arms['active']['nonzero_candidate_contributions'] > 0
    if complete:
        for item in comparisons:
            if 'off' not in item['arms']:
                assert all(d['before_first_source'] == 0 for d in item['differences'].values())
        zero_mask = next(c for c in comparisons if c['arms'] == ['zero', 'mask'])
        assert zero_mask['differences']['state']['count'] == 0
        assert zero_mask['differences']['actions']['count'] == 0
        assert len({a['founder_checkpoint_sha256'] for a in arms.values()}) == 1
    return dict(complete=complete, followup_gate=bool(gate), arms=list(arms.values()), comparisons=comparisons)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    groups = {}
    analyzed_runs = 0
    for path in sorted(args.root.iterdir()):
        if path.is_dir() and (path / 'REQUEST.json').exists() and list(path.glob('TRACE-*.json')):
            request = load(path / 'REQUEST.json')
            key = (request['seed'], request['reverse_appearance'], request['alias'], request.get('food_nutrition', 0.6), request.get('recipe_version', 3))
            arms, traces = groups.setdefault(key, ({}, {}))
            public, signatures = analyze_arm(path)
            arms[public['arm']], traces[public['arm']] = public, signatures
            analyzed_runs += 1
    off_arms, off_traces = {}, {}
    for (seed, reverse, alias, nutrition, recipe), (arms, traces) in groups.items():
        if 'off' in arms:
            off_arms[(seed,reverse,alias,nutrition)] = arms['off']
            off_traces[(seed,reverse,alias,nutrition)] = traces['off']
    for (seed, reverse, alias, nutrition, recipe), (arms, traces) in groups.items():
        if 'off' not in arms and (seed,reverse,alias,nutrition) in off_arms:
            arms['off'] = off_arms[(seed,reverse,alias,nutrition)]
            traces['off'] = off_traces[(seed,reverse,alias,nutrition)]
    cases = []
    for (seed, reverse, alias, nutrition, recipe), (arms, traces) in sorted(groups.items()):
        cases.append(dict(seed=seed, reverse_appearance=reverse, alias=alias, food_nutrition=nutrition, recipe_version=recipe,
            **summarize_group(arms, traces)))
    result = dict(schema='terminal_physical_safe_aggregate_v1', parent_sha256=PARENT,
        fresh_target_definition='different world entity from the consumed source; newly arrived food is reported separately',
        construction_genome_sha256=GENOME)
    if len(cases) == 1:
        result.update(cases[0])
    else:
        result.update(complete=bool(cases) and all(c['complete'] for c in cases), cases=cases)
    requests = args.root / 'requests-final.jsonl'
    if not requests.exists(): requests = args.root / 'requests.jsonl'
    expected_runs = len(requests.read_text().splitlines())
    result.update(expected_runs=expected_runs, analyzed_runs=analyzed_runs)
    result['complete'] = result['complete'] and analyzed_runs == expected_runs
    launch_path = args.root / 'LAUNCHED.json'
    if launch_path.exists():
        launch = load(launch_path)
        result['provenance'] = dict(assay_commit=launch.get('assay_commit', launch.get('source_commit')),
            host_base=launch['host_base'], binary_sha256=launch['binary_sha256'],
            profile=launch.get('profile', 'debug'), requests_sha256=launch['requests_sha256'])
    def safe(value):
        if isinstance(value, dict):
            assert not set(value) & {'patch', 'initial', 'final_features', 'replay_tick', 'neural', 'residents', 'weights', 'first_moment', 'second_moment', 'update_ages', 'query_feature_bits'}
            for child in value.values(): safe(child)
        elif isinstance(value, list):
            assert len(value) <= 64, 'unexpected large vector in a safe aggregate'
            for child in value: safe(child)
    safe(result)
    encoded = json.dumps(result, indent=2)
    if args.output:
        with args.output.open('x') as handle: handle.write(encoded + '\n')
    else:
        print(encoded)


if __name__ == '__main__':
    main()
