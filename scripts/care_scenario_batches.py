#!/usr/bin/env python3
"""Opt-in durable handoff for disposable scenario assessment batches.

No agent/model launch, new assessment, training, Current selection or promotion.
An authorized outer controller supplies proposals and completed native archives.
Only prepare() output may be sent to the isolated recipe designer; all other
outputs and ledger data are private controller material.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat

from care_frontier_sampler import integer, score_assessment, select
from compare_cpu_care_rounds import decode_json, file_pin, raw_tree
from import_care_scenario_recipe import read_json, reject_symlinks
from llm_care_scenarios import compile_recipe, LEGEND, RANDOM_ROLL

SCOPE = 'disposable_assessment_only'
MAX_RECORD_BYTES = 8 * 1024 * 1024
MAX_DISCOVERY_PER_BATCH = 64
# Native cooperative_source.rs STANDARD, with optional compiler inputs.
SOURCE_FILES = ('cpu-cycle.json', 'training-handoff.json', 'trained.alife-foundation',
                'actor-checkpoint.json', 'value-checkpoint.json', 'phase.txt')
PLAN_FIELDS = {'schema', 'scope', 'batches', 'family_count', 'levels', 'initial_difficulty',
               'decision_budget', 'slots', 'trials_per_family', 'pool_per_family',
               'holdouts_per_family', 'seed_start', 'sampler_seed', 'excluded_seeds',
               'native_evaluator_sha256'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(value):
    return hashlib.sha256(encode(value)).hexdigest()


def encode(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n').encode()


def validate_plan(plan):
    require(isinstance(plan, dict) and set(plan) == PLAN_FIELDS, 'invalid plan fields')
    require(type(plan['schema']) is int and plan['schema'] == 1 and plan['scope'] == SCOPE,
            'explicit disposable assessment scope required')
    for key, low, high in (('batches', 1, 64), ('family_count', 1, 16), ('levels', 1, 8),
                           ('decision_budget', 1, 128), ('slots', 1, 32),
                           ('trials_per_family', 3, 32), ('pool_per_family', 1, 32),
                           ('holdouts_per_family', 1, 32), ('seed_start', 1, 2**64-1),
                           ('sampler_seed', 0, 2**64-1)):
        require(integer(plan[key], low, high), 'invalid plan bound: ' + key)
    require(plan['family_count'] * plan['trials_per_family'] <= MAX_DISCOVERY_PER_BATCH,
            'at most 64 discovery trials per batch; aggregate file/decision budget exceeded')
    require(plan['levels'] <= plan['family_count'] and
            integer(plan['initial_difficulty'], 0, plan['levels']-1), 'invalid difficulty ladder')
    require(plan['slots'] <= plan['family_count'] * plan['pool_per_family'], 'insufficient pool')
    excluded = plan['excluded_seeds']
    require(isinstance(excluded, list) and 1 <= len(excluded) <= 4096 and
            all(integer(s, 1, 2**64-1) for s in excluded) and len(set(excluded)) == len(excluded),
            'explicit unique founder/prior-use exclusions required')
    count = plan['batches'] * plan['family_count'] * sum(plan[k] for k in
             ('trials_per_family', 'pool_per_family', 'holdouts_per_family'))
    end = plan['seed_start'] + count
    require(end <= 2**64 and all(not plan['seed_start'] <= s < end for s in excluded),
            'reserved seed range overlaps exclusions or overflows')
    require(isinstance(plan['native_evaluator_sha256'], str) and
            re.fullmatch('[0-9a-f]{64}', plan['native_evaluator_sha256']), 'pin the native evaluator')
    return plan


def fsync_dir(path):
    # Durability contract is local POSIX filesystem with working directory fsync.
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def read_record(path):
    reject_symlinks(path)
    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK | getattr(os, 'O_NOFOLLOW', 0))
    with os.fdopen(descriptor, 'rb') as source:
        require(stat.S_ISREG(os.fstat(source.fileno()).st_mode), 'ledger record must be regular')
        raw = source.read(MAX_RECORD_BYTES + 1)
    require(len(raw) <= MAX_RECORD_BYTES, 'ledger record exceeds bound')
    return decode_json(raw, path)


def write_new(path, value):
    raw = encode(value)
    require(len(raw) <= MAX_RECORD_BYTES, 'ledger record exceeds bound')
    with path.open('xb') as output:
        output.write(raw); output.flush(); os.fsync(output.fileno())


def claim(root, index):
    stage = root / ('%04d' % index)
    stage.mkdir()  # Exclusive writer claim. Failure/interruption stays visible.
    fsync_dir(root)
    return stage


def finish(stage, previous, kind, payload):
    record = {'schema': 1, 'previous_sha256': previous, 'kind': kind, 'payload': payload}
    write_new(stage / 'record.json', record)
    write_new(stage / 'complete.json', {'record_sha256': digest(record)})
    fsync_dir(stage)
    return record


def load(root):
    root = Path(root)
    reject_symlinks(root)
    require(root.is_dir(), 'batch root missing')
    names = sorted(p.name for p in root.iterdir() if p.name != 'founder-claim.json')
    require(1 <= len(names) <= 257 and names == ['%04d' % n for n in range(len(names))], 'unexpected batch ledger entries')
    records, previous = [], None
    for name in names:
        stage = root / name
        reject_symlinks(stage)
        require(stage.is_dir() and {p.name for p in stage.iterdir()} == {'record.json', 'complete.json'},
                'incomplete stage: stop; no automatic retry/recovery')
        record = read_record(stage / 'record.json')
        seal = read_record(stage / 'complete.json')
        require(isinstance(record, dict) and set(record) == {'schema', 'previous_sha256', 'kind', 'payload'}
                and record['schema'] == 1 and record['previous_sha256'] == previous
                and seal == {'record_sha256': digest(record)}
                and file_pin(stage / 'record.json')['sha256'] == seal['record_sha256'],
                'changed ledger evidence')
        records.append(record); previous = digest(record)
    require(records[0]['kind'] == 'plan', 'missing plan')
    validate_plan(records[0]['payload'])
    if os.path.lexists(root / 'founder-claim.json'):
        founder = read_record(root / 'founder-claim.json')
        require(set(founder) == {'identity', 'plan_sha256', 'sha256'} and
                founder['plan_sha256'] == digest(records[0]['payload']) and
                founder['sha256'] == digest(founder['identity']) and
                file_pin(root / 'founder-claim.json')['sha256'] == digest(founder), 'changed founder claim')
    # Every later operation must have the only permitted predecessor and index.
    for i, record in enumerate(records[1:]):
        require(record['kind'] == ('prepared', 'proposed', 'dispatched', 'assessed')[i % 4]
                and record['payload']['batch'] == i // 4, 'invalid stage transition')
        if record['kind'] == 'dispatched':
            require(os.path.lexists(root / 'founder-claim.json') and
                    record['payload']['founder_claim_sha256'] == digest(founder), 'dispatch founder claim changed')
    require(len(records) <= 1 + 4 * records[0]['payload']['batches'], 'batch plan exhausted')
    return records


def initialize(root, plan):
    require(os.name == 'posix' and hasattr(os, 'O_DIRECTORY'), 'local POSIX filesystem required')
    validate_plan(plan)
    root = Path(root)
    reject_symlinks(root)
    root.mkdir()  # Must be a NEW root; never adopts retained state or old output.
    fsync_dir(root.parent)
    finish(claim(root, 0), None, 'plan', plan)
    return {'scope': SCOPE, 'batches': plan['batches'], 'training_enabled': False}


def inputs(plan, batch, difficulty):
    names = ['family_%02d' % n for n in range(plan['family_count'])]
    width = sum(plan[k] for k in ('pool_per_family', 'holdouts_per_family', 'trials_per_family'))
    cursor = plan['seed_start'] + batch * len(names) * width
    pool, holdouts, discovery = {}, {}, []
    for name in names:
        pool[name] = list(range(cursor, cursor + plan['pool_per_family'])); cursor += plan['pool_per_family']
        holdouts[name] = list(range(cursor, cursor + plan['holdouts_per_family'])); cursor += plan['holdouts_per_family']
        discovery += [{'family': name, 'seed': s} for s in range(cursor, cursor + plan['trials_per_family'])]
        cursor += plan['trials_per_family']
    host = {k: plan[k] for k in ('decision_budget', 'slots', 'sampler_seed')}
    host.update(current_difficulty=difficulty, pool_seeds=pool, holdout_seeds=holdouts)
    return host, discovery


def public_request(plan, difficulty, feedback):
    # Only fixed rules, bounded integers and canonical generated IDs. Never copy
    # archive rows, source hashes, paths, seeds, model prose or private settings.
    return {'schema': 1, 'kind': 'care_recipe_request', 'legend': LEGEND,
            'goal': 'nourish_and_survive', 'random_roll': RANDOM_ROLL,
            'family_ids': ['family_%02d' % i for i in range(plan['family_count'])],
            'levels': plan['levels'], 'current_difficulty': difficulty,
            'family_difficulties': {('family_%02d' % i): i % plan['levels']
                                    for i in range(plan['family_count'])},
            'lessons': ['feeding'],
            'conditions': ['newborn_v1', 'ordinary_healthy_aging_v1'],
            'rules': ['Return only a recipe with schema, legend, scenarios.',
                      'Each scenario has id, map, condition, difficulty, goal, random_roll.',
                      'Use each family ID once and distinct map/condition pairs.',
                      'Maps are rectangular 3..17 cells wide/high, with a closed # border.',
                      'Use only # . L F, exactly one L and F, connected by cardinal open cells.',
                      'Use the assigned family_difficulties; do not redefine the ladder.'],
            'feedback': feedback}


def verify_native_inputs(pins, run_root):
    require(isinstance(pins, dict) and len(pins) <= 128, 'bounded native input pins required')
    for name, expected in pins.items():
        require(isinstance(name, str) and isinstance(expected, str) and
                re.fullmatch('[0-9a-f]{64}', expected), 'invalid native input pin')
        path = Path(name)
        reject_symlinks(path)
        require(path.is_absolute() and path.resolve(strict=True).is_relative_to(run_root),
                'native input is outside claimed disposable run')
        require(file_pin(path)['sha256'] == expected, 'native assessment input changed')


def verify_feedback_evidence(assessed, identity):
    run_root = Path(identity['run_root']).resolve(strict=True)
    evaluator = assessed['evaluator']
    reject_symlinks(Path(evaluator['path']))
    require(file_pin(Path(evaluator['path'])) == evaluator['pin'], 'feedback evaluator changed')
    for evidence in assessed['archives']:
        archive = Path(evidence['archive'])
        reject_symlinks(archive)
        require(archive.resolve(strict=True).is_relative_to(run_root) and
                digest(raw_tree(archive)) == evidence['inventory_sha256'], 'feedback archive changed')
        verify_native_inputs(evidence['input_sha256'], run_root)


def verify_batch_completion(assessed, identity):
    batch_root = Path(identity['run_root']) / 'batches' / ('%04d' % assessed['batch'])
    marker = read_record(batch_root / 'completed.json')
    files = raw_tree(batch_root)
    files.pop('completed.json', None)
    directories = sorted(str(Path(path).relative_to(batch_root)) for path, _, _ in os.walk(batch_root))
    require(marker == {'batch': assessed['batch'], 'aggregate': assessed['aggregate'],
                       'training_enabled': False, 'promotion': 'none',
                       'inventory': {'files': files, 'directories': directories}},
            'completed batch controls or evidence changed')


def prepare(root):
    root = Path(root); records = load(root); plan = records[0]['payload']
    require(records[-1]['kind'] in ('plan', 'assessed'), 'complete current batch first')
    batch = (len(records)-1)//4
    require(batch < plan['batches'], 'batch plan exhausted')
    prior = records[-1]['payload'] if batch else None
    if prior:
        verify_feedback_evidence(prior, records[-2]['payload']['identity'])
        verify_batch_completion(prior, records[-2]['payload']['identity'])
    difficulty = prior['aggregate']['difficulty_after'] if prior else plan['initial_difficulty']
    feedback = prior['aggregate'] if prior else None
    host, discovery = inputs(plan, batch, difficulty)
    request = public_request(plan, difficulty, feedback)
    payload = {'batch': batch, 'host': host, 'discovery': discovery, 'request': request,
               'host_sha256': digest(host), 'discovery_sha256': digest(discovery),
               'request_sha256': digest(request)}
    finish(claim(root, len(records)), digest(records[-1]), 'prepared', payload)
    return request


def propose(root, recipe_path):
    root = Path(root); records = load(root)
    require(records[-1]['kind'] == 'prepared', 'proposal needs a prepared batch')
    # Claim BEFORE opening untrusted input. Failed proposal is not a free reroll.
    stage = claim(root, len(records))
    recipe, source_hash = read_json(Path(recipe_path))
    prepared = records[-1]['payload']; plan = records[0]['payload']
    require(isinstance(recipe, dict) and type(recipe.get('schema')) is int, 'integer recipe schema required')
    bank = compile_recipe(recipe, prepared['host'])
    require({f['difficulty'] for f in bank['families']} == set(range(plan['levels'])),
            'recipe changed predeclared ladder')
    require(all(f['difficulty'] == int(f['id'].split('_')[1]) % plan['levels']
                for f in bank['families']), 'recipe changed assigned family difficulty')
    payload = {'batch': prepared['batch'], 'source': 'external_json', 'recipe': recipe,
               'recipe_input_sha256': source_hash, 'bank': bank, 'bank_sha256': digest(bank)}
    finish(stage, digest(records[-1]), 'proposed', payload)
    return {'batch': payload['batch'], 'bank_sha256': payload['bank_sha256'], 'scope': SCOPE}


def claim_founder(root, identity):
    """Burn the founder initialization attempt once for the entire ledger."""
    root = Path(root); records = load(root)
    require(not any(r['kind'] in {'dispatched', 'assessed'} for r in records), 'founder claim after dispatch')
    require(isinstance(identity, dict) and set(identity) == {
        'run_root', 'founder_seed', 'care_binary_sha256', 'evaluator_sha256'}, 'invalid founder claim')
    plan = records[0]['payload']
    require(integer(identity['founder_seed'], 1, 2**64-1) and identity['founder_seed'] in plan['excluded_seeds'],
            'founder seed must be explicitly excluded')
    require(isinstance(identity['run_root'], str) and len(identity['run_root']) <= 4096,
            'invalid founder run root')
    run_root = Path(identity['run_root'])
    reject_symlinks(run_root)
    require(run_root.is_absolute() and not run_root.resolve().is_relative_to(root.resolve()) and
            not root.resolve().is_relative_to(run_root.resolve()), 'founder run root must be separate')
    require(all(isinstance(identity[key], str) and re.fullmatch('[0-9a-f]{64}', identity[key])
                for key in ('care_binary_sha256', 'evaluator_sha256')) and
            identity['evaluator_sha256'] == plan['native_evaluator_sha256'], 'invalid founder executable pins')
    payload = {'identity': identity, 'plan_sha256': digest(plan), 'sha256': digest(identity)}
    write_new(root / 'founder-claim.json', payload)  # Exclusive; failure is never retried.
    fsync_dir(root)
    return payload


def claim_dispatch(root, dispatch_identity):
    """Private, one-shot outer-controller handoff. Never sent to the designer.

    Durable claim precedes native execution; an existing dispatched stage may
    only be assessed, never replayed, even if no output is visible yet.
    """
    root = Path(root); records = load(root)
    require(records[-1]['kind'] == 'proposed', 'dispatch requires a new proposal; never replay a dispatch')
    require(isinstance(dispatch_identity, dict) and set(dispatch_identity) == {
        'source_sha256', 'run_root', 'evaluator_sha256'}, 'invalid private dispatch identity')
    require(isinstance(dispatch_identity['source_sha256'], dict) and
            set(dispatch_identity['source_sha256']) in (set(SOURCE_FILES), set(SOURCE_FILES) | {'compiler-inputs.json'}) and
            all(isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value)
                for value in dispatch_identity['source_sha256'].values()), 'invalid native source pins')
    require(isinstance(dispatch_identity['evaluator_sha256'], str) and
            re.fullmatch('[0-9a-f]{64}', dispatch_identity['evaluator_sha256']), 'invalid evaluator hash')
    require(isinstance(dispatch_identity['run_root'], str) and len(dispatch_identity['run_root']) <= 4096,
            'invalid dispatch root')
    run_root = Path(dispatch_identity['run_root'])
    reject_symlinks(run_root)
    require(run_root.is_absolute() and run_root.is_dir() and
            not run_root.resolve().is_relative_to(root.resolve()) and
            not root.resolve().is_relative_to(run_root.resolve()), 'dispatch root must be separate')
    require(dispatch_identity['evaluator_sha256'] == records[0]['payload']['native_evaluator_sha256'],
            'dispatch evaluator differs from plan')
    previous = [r['payload']['identity'] for r in records if r['kind'] == 'dispatched']
    require(not previous or previous[0] == dispatch_identity, 'dispatch source/run changed')
    founder = read_record(root / 'founder-claim.json')
    require(founder['identity']['run_root'] == dispatch_identity['run_root'] and
            founder['identity']['evaluator_sha256'] == dispatch_identity['evaluator_sha256'],
            'dispatch differs from claimed founder run')
    proposed, prepared = records[-1]['payload'], records[-2]['payload']
    payload = {'batch': proposed['batch'], 'identity': dispatch_identity,
               'founder_claim_sha256': digest(founder),
               'bank_sha256': proposed['bank_sha256'], 'discovery_sha256': prepared['discovery_sha256']}
    finish(claim(root, len(records)), digest(records[-1]), 'dispatched', payload)
    return {'batch': proposed['batch'], 'bank': proposed['bank'], 'discovery': prepared['discovery'],
            'host': prepared['host'], 'scope': SCOPE}


def assess(root, archives, evaluator, *, score=None):
    # The optional internal callback preserves native admission while adding
    # bounded dispatch-runner I/O/guards. It is never a model/CLI argument.
    scorer = score_assessment if score is None else score
    root = Path(root); records = load(root)
    require(records[-1]['kind'] == 'dispatched', 'assessment needs a claimed dispatch')
    stage = claim(root, len(records))
    proposed, prepared, plan = records[-2]['payload'], records[-3]['payload'], records[0]['payload']
    require(isinstance(archives, list) and len(archives) == len(prepared['discovery']),
            'one complete archive per reserved discovery trial required')
    evaluator = Path(evaluator)
    reject_symlinks(evaluator)
    require(file_pin(evaluator)['sha256'] == plan['native_evaluator_sha256'], 'native evaluator changed')
    run_root = Path(records[-1]['payload']['identity']['run_root']).resolve(strict=True)
    expected_paths = {run_root / 'batches' / ('%04d' % proposed['batch']) / ('assessment-%04d' % i): task['seed']
                      for i, task in enumerate(prepared['discovery'])}
    actual_paths = [Path(path).resolve(strict=True) for path in archives]
    require(len(set(actual_paths)) == len(actual_paths) and set(actual_paths) == set(expected_paths),
            'archives differ from claimed batch output paths')
    ledger_before = raw_tree(root)
    rows, evidence, archive_pins = [], [], []
    for archive in archives:
        archive = Path(archive)
        reject_symlinks(archive)
        require(not archive.resolve().is_relative_to(root.resolve()) and
                not root.resolve().is_relative_to(archive.resolve()), 'ledger and archive must be separate')
        before = raw_tree(archive)
        receipt = read_record(archive / 'care-evaluation.json')
        verify_native_inputs(receipt['input_sha256'], run_root)
        row = scorer(archive, evaluator)  # Fixed existing native verify API; no shell.
        require(raw_tree(archive) == before, 'archive changed during admission')
        require(row['seed'] == expected_paths[archive.resolve()], 'archive seed differs from claimed output path')
        require(row['receipt_sha256'] == file_pin(archive / 'care-evaluation.json')['sha256'],
                'scored native receipt differs from archive')
        rows.append(row)
        archive_pins.append((archive, before))
        evidence.append({'archive': str(archive.resolve()), 'inventory_sha256': digest(before),
                         'receipt_sha256': row['receipt_sha256'], 'input_sha256': receipt['input_sha256']})
    require(file_pin(evaluator)['sha256'] == plan['native_evaluator_sha256'], 'native evaluator changed')
    expected = {r['seed']: r['family'] for r in prepared['discovery']}
    require(len({r['seed'] for r in rows}) == len(rows) and {r['seed'] for r in rows} == set(expected),
            'archive seeds differ from reserved discovery')
    families = {f['id']: f for f in proposed['bank']['families']}
    for row in rows:
        family = families[expected[row['seed']]]
        require(row['lesson'] == family['lesson'] and row['food_need_condition'] == family['food_need_condition']
                and row.get('layout') == family['layout'], 'archive does not match reserved family')
    require(all(row['source_sha256'] == records[-1]['payload']['identity']['source_sha256']
                for row in rows), 'archive source differs from claimed disposable source')
    identity = {key: rows[0][key] for key in ('actor_sha256', 'value_sha256', 'policy_version', 'source_sha256')}
    previous = [r['payload'] for r in records if r['kind'] == 'assessed']
    require(not previous or identity == previous[0]['frozen_identity'], 'frozen assessment source changed')
    selection = select(proposed['bank'], rows)
    aggregate = {'difficulty_before': selection['difficulty_before'], 'difficulty_after': selection['difficulty_after'],
                 'families': [{'id': name, 'trials': stats['trials'], 'successes': stats['successes']}
                              for name, stats in sorted(selection['statistics'].items())]}
    require(raw_tree(root) == ledger_before, 'ledger changed during native admission')
    require(all(raw_tree(path) == pin for path, pin in archive_pins), 'archive changed before feedback')
    require(file_pin(evaluator)['sha256'] == plan['native_evaluator_sha256'], 'native evaluator changed')
    payload = {'batch': proposed['batch'], 'aggregate': aggregate, 'frozen_identity': identity,
               'archives': evidence, 'selection': selection,
               'evaluator': {'path': str(evaluator.resolve()), 'pin': file_pin(evaluator)}}
    verify_feedback_evidence(payload, records[-1]['payload']['identity'])
    finish(stage, digest(records[-1]), 'assessed', payload)
    return aggregate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    sub = parser.add_subparsers(dest='command', required=True)
    init = sub.add_parser('init'); init.add_argument('--plan', type=Path, required=True)
    sub.add_parser('prepare')
    proposal = sub.add_parser('propose'); proposal.add_argument('--recipe', type=Path, required=True)
    assessment = sub.add_parser('assess')
    assessment.add_argument('--archive', type=Path, action='append', required=True)
    assessment.add_argument('--native-evaluator', type=Path, required=True)
    sub.add_parser('status')
    args = parser.parse_args()
    if args.command == 'init':
        result = initialize(args.root, read_json(args.plan)[0])
    elif args.command == 'prepare':
        result = prepare(args.root)
    elif args.command == 'propose':
        result = propose(args.root, args.recipe)
    elif args.command == 'assess':
        result = assess(args.root, args.archive, args.native_evaluator)
    else:
        records = load(args.root)
        result = {'stage': records[-1]['kind'], 'records': len(records), 'scope': SCOPE}
    print(json.dumps(result, sort_keys=True, allow_nan=False))


if __name__ == '__main__':
    main()
