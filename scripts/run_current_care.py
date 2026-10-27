#!/usr/bin/env python3
"""Native serial care with one self-contained durable checkpoint.

Plan by default. Import a verified legacy source once; future accepted episodes
atomically replace current. All episode/replay/private-life files live in .run.
No existing legacy lane/history is removed by this command.
"""
import argparse
from collections import OrderedDict
import datetime as dt
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import uuid

import current_checkpoint_store as checkpoints
import run_mixed_care_serial as mixed

require = checkpoints.require
host = mixed.host
# Per-attempt hashes, a UUID filename, worker identity, cursor and reconciliation
# fields are bounded bookkeeping. Reserve space before creating any run files;
# the shared JSON writer also enforces the reader's limit on every later update.
RUN_BOOKKEEPING_RESERVE = 2048


def tool(binary, *arguments, on_started=None):
    result = mixed.inspector_io.run_inspector([str(binary), *map(str, arguments)], timeout=90, on_started=on_started)
    result.check_returncode()
    return mixed.inspector_io.decode_json(result.stdout, 'current checkpoint admission')


def validator(binary):
    """Repeat publication copies need hashes, not repeated full private admission.

    The cache is private to this validator/OS-writer lifetime and contains only
    eight successful content digests. New bytes always receive native admission.
    Only the native self-contained format (no external dependencies) is eligible.
    """
    binary_digest = host.sha256(binary)
    admitted = OrderedDict()

    def validate(path):
        require(host.sha256(binary) == binary_digest, 'checkpoint admission binary changed')
        before = checkpoints.digest(path)  # Also rejects aliases/nonregular/oversized files.
        if before in admitted:
            admitted.move_to_end(before)
            return None
        try:
            report = tool(binary, '--inspect-current', path)
            require(checkpoints.digest(path) == before, 'checkpoint changed during native admission')
            require(host.sha256(binary) == binary_digest, 'checkpoint admission binary changed')
        except BaseException:
            admitted.clear()
            raise
        if isinstance(report, dict) and report.get('kind') == 'single-current-training-checkpoint' and report.get('external_dependencies') == []:
            admitted[before] = None
            if len(admitted) > 8:
                admitted.popitem(last=False)
        return report

    return validate


def checkout_context():
    root = Path(__file__).resolve().parent.parent
    require(Path.cwd().resolve() == root, 'run from the admitted source checkout')
    require(not subprocess.check_output(['git', 'status', '--porcelain'], text=True).strip(),
            'run requires a clean source checkout')
    return {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
            'tree': subprocess.check_output(['git', 'rev-parse', 'HEAD^{tree}'], text=True).strip(),
            'cargo_lock_sha256': host.sha256(root / 'Cargo.lock')}


def prior_environment():
    return {key: value for key, value in host.prior_environment().items() if key.startswith('ALIFE_SLM_')}


def disjoint(a, b):
    a, b = a.resolve(), b.resolve()
    require(a != b and a not in b.parents and b not in a.parents, 'store must be separate from legacy source/lane')


def import_seeds(source, history, lane, policy):
    """Reduce exact admitted legacy evidence to exclusions; never infer freshness."""
    pins = mixed.file_hashes(source)
    receipt, _ = mixed.inspector_io.load_json(source / 'cycle.json', mixed.seeds.MAX_DOCUMENT_BYTES)
    admission = {'policy': policy, 'historical_freshness': mixed.FRESHNESS[policy]}
    doc, _, ranges, assessments = mixed.history(history, pins, receipt['seed'], lane, admission=admission)
    # The owned unused tail may override its own reservation, never a consumed
    # seed or an exclusion imported from another lane. Keep this distinction
    # until migration admission; the durable checkpoint then needs only union
    # ranges and its proven-unused cursor.
    hard_exclusions = mixed.seeds._ranges(doc.get('consumed_seed_ranges', []))
    for item in doc.get('imports', []):
        imported, _ = mixed.grab_import(Path(item['path']).resolve(strict=True), item['sha256'])
        hard_exclusions += imported
    ledger_path = lane / mixed.LEDGER
    pending = None
    if ledger_path.exists():
        ledger = mixed.read_ledger(ledger_path)
        if policy == 'known-exclusions':
            mixed.reconcile_prior_runs(doc, ledger['runs'], check_payloads=False)
        runs = ledger['runs']
        if runs:
            require(all(a['status'] in ('accepted', 'abandoned') for a in runs[-1]['attempts']),
                    'legacy pending work must be reconciled before migration')
            accepted = [a for a in runs[-1]['attempts'] if a['status'] == 'accepted']
            latest = accepted[-1]['directory'] if accepted else runs[-1]['plan']['source']
            expected = accepted[-1]['sha256'] if accepted else runs[-1]['plan']['source_sha256']
            require(Path(latest).resolve() == source.resolve() and
                    {name: expected[name] for name in mixed.FILES} == {name: pins[name] for name in mixed.FILES},
                    'migration must select the latest accepted legacy brain')
            first, last = runs[-1]['plan']['seed_range']
            next_seed = first + len(runs[-1]['attempts'])
            if next_seed <= last:
                pending = [next_seed, last]
        ranges += ledger['excluded_seed_ranges'] + [run['plan']['seed_range'] for run in runs]
        hard_exclusions += ledger['excluded_seed_ranges']
        assessments |= set(ledger['assessment_seeds'])
    require(pending is None or not any(pending[0] <= seed <= pending[1] for seed in assessments),
            'unused legacy allocation overlaps declared assessment seeds; reconcile before migration')
    require(pending is None or not any(pending[0] <= end and start <= pending[1] for start, end in hard_exclusions),
            'unused legacy allocation overlaps consumed/imported seed exclusions; reconcile before migration')
    require(mixed.file_hashes(source) == pins, 'legacy source changed during seed admission')
    return {'historical_freshness': admission['historical_freshness'],
            'excluded_ranges': mixed.seeds._ranges(ranges + [[seed, seed] for seed in assessments]),
            'pending_range': pending}


def migrate(root, source, history, lane, binary, policy):
    root, source, lane = root.resolve(), source.resolve(strict=True), lane.resolve(strict=True)
    disjoint(root, lane)
    disjoint(root, source)
    require(not root.exists(), 'migration requires a new store; existing data remains untouched')
    host.assert_lane_idle()
    tool(binary, '--check-current-destination', source, root)
    metadata = import_seeds(source, history, lane, policy)
    seed_bytes = json.dumps(metadata, allow_nan=False, separators=(',', ':')).encode()
    require(len(seed_bytes) <= checkpoints.MAX_BYTES, 'seed input exceeds checkpoint byte budget')
    # Seed input is consumed by the native checkpoint codec, not the 1 MiB run
    # metadata reader. Preflight both documents before creating the destination.
    state = {'schema': 1, 'kind': 'single-current-temporary-run', 'mode': 'import',
             'source': str(source), 'tool': str(binary), 'tool_sha256': host.sha256(binary),
             'worker': None}
    checkpoints.encode_json(state)
    root.mkdir(parents=True)
    with checkpoints.Store(root, validator(binary)) as store:
        run = root / checkpoints.RUN
        run.mkdir()
        # This owned import run survives interruption; its source remains read-only.
        checkpoints.write_json(run / 'run.json', state)
        checkpoints.write_new(run / 'seeds.json', seed_bytes)
        candidate = run / 'import.alife-checkpoint'
        report = owned_tool(store, state, '--import-current', source, run / 'seeds.json', candidate)
        state['candidate_sha256'] = checkpoints.digest(candidate)
        checkpoints.write_json(run / 'run.json', state)
        published = store.publish(candidate, None)
        state['reconciled_current'] = published
        checkpoints.write_json(run / 'run.json', state)
        store.remove_run(published)
        return report


def pinned(state):
    for name, digest in state['executables'].items():
        require(host.sha256(name) == digest, 'pinned executable changed; no dispatch or cleanup')
    for name, digest in state['helpers'].items():
        require(host.sha256(Path(__file__).parent / name) == digest, 'run helper changed; reconcile with its original revision')
    require(prior_environment() == state['prior_environment'], 'semantic-prior environment changed')
    require(checkout_context() == state['checkout'], 'admitted source revision changed')


def save(store, state):
    checkpoints.write_json(store.root / checkpoints.RUN / 'run.json', state)


def owned_tool(store, state, *arguments):
    active = []
    state['worker'] = {'pid': None, 'stopped': False}
    save(store, state)

    def started(process):
        active.append(process)
        state['worker']['pid'] = process.pid
        save(store, state)
    try:
        return tool(state['tool'], *arguments, on_started=started)
    finally:
        if active and active[0].poll() is not None:
            state['worker']['stopped'] = True
            save(store, state)


def new_candidate(run):
    return run / (uuid.uuid4().hex + '.alife-checkpoint')


def clean_attempt(store, state):
    """Called only after durable acceptance or explicit stopped-unsealed reconciliation."""
    run = store.root / checkpoints.RUN
    for name in ('episode', 'source.alife-checkpoint', 'episode.stdout.log', 'episode.stderr.log'):
        path = run / name
        require(not path.is_symlink(), 'temporary run symlink refused')
        if path.is_dir():
            shutil.rmtree(path)
        elif path.exists():
            checkpoints.regular(path)
            path.unlink()
    for path in run.glob('*.alife-checkpoint'):
        checkpoints.regular(path)
        path.unlink()
    state['attempt'] = None
    state['worker'] = None
    save(store, state)


def reconcile(store, *, abandon_unsealed=False):
    """No training. Complete publication/accepted seals before temporary cleanup."""
    state = store.run_state()  # Refuses an active/unknown writer before touching its files.
    if state['mode'] == 'import':
        binary = Path(state['tool'])
        require(host.sha256(binary) == state['tool_sha256'], 'import tool changed')
        candidate = store.root / checkpoints.RUN / 'import.alife-checkpoint'
        require('candidate_sha256' in state and checkpoints.digest(candidate) == state['candidate_sha256'],
                'interrupted import is not fully admitted; legacy source retained, reconcile manually')
        if not store.current.exists():
            store.publish(candidate, None)
        require(checkpoints.digest(store.current) == state['candidate_sha256'], 'import current collision')
        state['reconciled_current'] = checkpoints.digest(store.current)
        save(store, state)
        store.remove_run(state['reconciled_current'])
        return None
    require(state['mode'] == 'serial', 'unknown run mode')
    pinned(state)
    binary = Path(state['tool'])
    run = store.root / checkpoints.RUN
    if state['phase'] == 'preparing':
        require(checkpoints.digest(store.current) == state['before_reservation'], 'current changed during preparation')
        state['reconciled_current'] = checkpoints.digest(store.current)
        save(store, state)
        store.remove_run(state['reconciled_current'])
        return None
    if state['phase'] == 'reserving':
        actual = checkpoints.digest(store.current)
        if actual == state['before_reservation']:
            store.publish(run / 'reserved.alife-checkpoint', actual)
        require(checkpoints.digest(store.current) == state['reserved_sha256'], 'reservation/current collision')
        state['phase'] = 'ready'
        save(store, state)
    attempt = state.get('attempt')
    if attempt is None:
        return state
    actual = checkpoints.digest(store.current)
    publication = attempt.get('publication')
    if publication is not None and actual == publication['after']:
        state['cursor'] += 1
        clean_attempt(store, state)
        return state
    if publication is not None:
        require(actual == publication['before'], 'current differs from admitted candidate predecessor')
        candidates = [path for path in run.glob('*.alife-checkpoint')
                      if checkpoints.digest(path) == publication['after']]
        require(candidates, 'admitted candidate missing/corrupt; accepted current and temporary work retained')
        # The exact candidate hash was durably recorded only after native
        # admission and biological/predecessor checks. Recovery needs that full
        # candidate, not the disposable episode/replay inputs used to produce it.
        report = tool(binary, '--inspect-current', candidates[0])
        old = tool(binary, '--inspect-current', store.current)
        require(report['seed'] == attempt['seed'] and report['policy_version'] == old['policy_version'] + 1,
                'admitted candidate identity changed')
        store.publish(candidates[0], actual)
        state['cursor'] += 1
        clean_attempt(store, state)
        return state
    consumption = attempt['consumption']
    if actual == consumption['before']:
        store.publish(run / consumption['file'], actual)
        actual = checkpoints.digest(store.current)
    require(actual == attempt['before'], 'current differs from interrupted attempt predecessor')
    episode = run / 'episode'
    if not (episode / 'cycle.json').exists():
        # A partial learner may contain recoverable updates. Never discard it by
        # inferring failure from a missing seal or by reusing its reserved seed.
        never_launched = state.get('worker') is None and not episode.exists()
        require(never_launched or abandon_unsealed and state.get('worker') is not None and
                state['worker']['pid'] is not None,
                'unsealed interrupted work retained; reconcile/recover it or explicitly --abandon-unsealed after writer exit')
        clean_attempt(store, state)
        return state
    receipt, _ = mixed.inspector_io.load_json(episode / 'cycle.json', mixed.seeds.MAX_DOCUMENT_BYTES)
    old = tool(binary, '--inspect-current', store.current)
    mixed.validate_receipt(receipt, old, state['rows'][state['cursor']], attempt['seed'])
    require(host.sha256(episode / 'initial.alife-foundation') == old['asset_sha256'], 'episode initial asset changed')
    candidate = new_candidate(run)
    report = owned_tool(store, state, '--advance-current', store.current, episode, candidate)
    require(report['seed'] == attempt['seed'], 'candidate seed differs from reserved attempt')
    attempt['publication'] = {'before': actual, 'after': checkpoints.digest(candidate)}
    save(store, state)
    store.publish(candidate, actual)
    state['cursor'] += 1
    clean_attempt(store, state)
    return state


def start(store, args):
    require(not (store.root / checkpoints.RUN).exists(), 'reconcile previous temporary run before starting another')
    schedule, _ = mixed.load(args.schedule)
    rows = mixed.plan_rows({'schedule': schedule, 'food_need_condition': args.food_need_condition})
    require(rows, 'empty schedule')
    first = mixed.seeds._seed(args.seed)
    last = mixed.seeds._seed(first + args.seed_count - 1)
    require(args.seed_count >= len(rows), 'allocation must cover schedule')
    require(dt.datetime.now(host.UTC) < args.cutoff <= dt.datetime.now(host.UTC) + dt.timedelta(hours=24), 'cutoff must be within 24 hours')
    mixed.integer(args.command_seconds, 1, 3600)
    mixed.integer(args.idle_seconds, 0, 3600)
    require(all(math.isfinite(value) and value > 0 for value in (args.min_disk_gib, args.min_memory_gib)), 'invalid resource reserves')
    checkout = checkout_context()
    binaries = {str(path.resolve(strict=True)): host.sha256(path) for path in (args.binary, args.checkpoint_tool)}
    tool_binary = args.checkpoint_tool.resolve(strict=True)
    tool(tool_binary, '--check-current-allocation', store.current, first, last)
    run = store.root / checkpoints.RUN
    before = checkpoints.digest(store.current)
    reserved = run / 'reserved.alife-checkpoint'
    helpers = ('run_current_care.py', 'current_checkpoint_store.py', 'run_mixed_care_serial.py',
               'run_eat_held_food_night.py', 'grab_food_seed_reservations.py', 'summarize_ppo_diagnostics.py')
    state = {'schema': 1, 'kind': 'single-current-temporary-run', 'mode': 'serial',
             'phase': 'preparing', 'before_reservation': before,
             'tool': str(tool_binary), 'binary': str(args.binary.resolve(strict=True)), 'executables': binaries,
             'helpers': {name: host.sha256(Path(__file__).parent / name) for name in helpers},
             'prior_environment': prior_environment(), 'checkout': checkout,
             'rows': rows, 'cursor': 0, 'next_seed': first, 'last_seed': last,
             'cutoff': args.cutoff.isoformat(), 'command_seconds': args.command_seconds,
             'min_disk_gib': args.min_disk_gib, 'min_memory_gib': args.min_memory_gib,
             'idle_seconds': args.idle_seconds, 'attempt': None, 'worker': None}
    require(len(checkpoints.encode_json(state)) + RUN_BOOKKEEPING_RESERVE <= checkpoints.MAX_METADATA_BYTES,
            'expanded schedule exceeds temporary metadata budget; use smaller explicit schedules')
    run.mkdir()
    shutil.copyfile(store.current, run / 'source.alife-checkpoint')
    save(store, state)
    owned_tool(store, state, '--reserve-current', store.current, first, last, reserved)
    state['phase'] = 'reserving'
    state['reserved_sha256'] = checkpoints.digest(reserved)
    state['worker'] = None
    save(store, state)
    return reconcile(store)


def execute(store, state):
    run = store.root / checkpoints.RUN
    cutoff = host.cutoff_time(state['cutoff'])

    def guard():
        require(not (run / 'STOP').exists(), 'owner STOP requested interruption')
        require(dt.datetime.now(host.UTC) < cutoff, 'schedule cutoff reached')
        pinned(state)
        resources = host.resource_snapshot(store.root)
        require(resources['disk_free_bytes'] >= state['min_disk_gib'] * 2**30 and
                resources['available_memory_bytes'] is not None and
                resources['available_memory_bytes'] >= state['min_memory_gib'] * 2**30, 'resource reserve reached')
        require(os.name != 'nt' or resources['idle_seconds'] >= state['idle_seconds'], 'interactive PC use resumed')

    while state['cursor'] < len(state['rows']):
        guard()
        host.assert_lane_idle()
        seed = state['next_seed']
        require(seed <= state['last_seed'], 'reserved seeds exhausted; reconcile before a new allocation')
        source = run / 'source.alife-checkpoint'
        if source.exists():
            source.unlink()
        shutil.copyfile(store.current, source)
        before = checkpoints.digest(store.current)
        consumed = new_candidate(run)
        owned_tool(store, state, '--consume-current', store.current, seed, consumed)
        after = checkpoints.digest(consumed)
        state['attempt'] = {'seed': seed, 'before': after,
                            'consumption': {'before': before, 'after': after, 'file': consumed.name}}
        state['next_seed'] += 1
        state['worker'] = None
        save(store, state)
        store.publish(consumed, before)  # Durable cursor advance precedes any native world work.
        state['worker'] = {'pid': None, 'stopped': False}
        save(store, state)  # Unknown launch outcomes fail closed after a crash.
        row = state['rows'][state['cursor']]
        argv = [state['binary'], '--resume-cycle', str(source), str(run / 'episode'), str(row['decisions']),
                '--seed', str(seed), '--lesson', row['lesson']]
        if 'food_need_condition' in row:
            argv += ['--food-need-condition', row['food_need_condition']]
        active = []

        def started(process):
            active.append(process)
            state['worker']['pid'] = process.pid
            save(store, state)

        try:
            host.run_child(argv, run / 'episode', min(cutoff, dt.datetime.now(host.UTC) +
                           dt.timedelta(seconds=state['command_seconds'])), guard, host.prior_environment(), on_started=started)
        finally:
            if active and active[0].poll() is not None:
                state['worker']['stopped'] = True
                save(store, state)
        state = reconcile(store)
    state['reconciled_current'] = checkpoints.digest(store.current)
    save(store, state)
    store.remove_run(state['reconciled_current'])
    return tool(state['tool'], '--inspect-current', store.current)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--store', type=Path, required=True)
    parser.add_argument('--checkpoint-tool', type=Path, required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--migrate', type=Path)
    mode.add_argument('--schedule', type=Path)
    mode.add_argument('--resume', action='store_true')
    mode.add_argument('--reconcile', action='store_true')
    mode.add_argument('--inspect', action='store_true')
    parser.add_argument('--seed-history', type=Path)
    parser.add_argument('--legacy-lane', type=Path)
    parser.add_argument('--seed-admission', choices=tuple(mixed.FRESHNESS), default='strict')
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--seed', type=int)
    parser.add_argument('--seed-count', type=int)
    parser.add_argument('--cutoff', type=host.cutoff_time)
    parser.add_argument('--command-seconds', type=int, default=1200)
    parser.add_argument('--min-disk-gib', type=float, default=8)
    parser.add_argument('--min-memory-gib', type=float, default=2)
    parser.add_argument('--idle-seconds', type=int, default=300)
    parser.add_argument('--food-need-condition', choices=('newborn_v1', 'ordinary_healthy_aging_v1'), default='newborn_v1')
    parser.add_argument('--abandon-unsealed', action='store_true', help='explicitly discard stopped, unsealed temporary work; accepted current stays authoritative')
    parser.add_argument('--run', action='store_true', help='perform migration, reconciliation or training; otherwise describe only')
    args = parser.parse_args(argv)
    try:
        if args.inspect:
            print(json.dumps(tool(args.checkpoint_tool, '--inspect-current', args.store / checkpoints.CURRENT)))
            return 0
        if not args.run:
            print(json.dumps({'execute': False, 'retention': 'single-current-checkpoint',
                              'store': str(args.store.resolve()), 'temporary_run': str(args.store.resolve() / checkpoints.RUN)}))
            return 0
        require(not args.abandon_unsealed or args.reconcile, '--abandon-unsealed requires --reconcile')
        if args.migrate:
            require(args.seed_history and args.legacy_lane, 'migration requires exact history and legacy lane')
            result = migrate(args.store, args.migrate, args.seed_history, args.legacy_lane,
                             args.checkpoint_tool.resolve(strict=True), args.seed_admission)
        else:
            with checkpoints.Store(args.store.resolve(strict=True), validator(args.checkpoint_tool)) as store:
                host.assert_lane_idle()
                if args.schedule:
                    require(args.binary and args.seed and args.seed_count and args.cutoff, 'schedule requires binary, seed allocation and cutoff')
                    if (store.root / checkpoints.RUN).exists():
                        state = reconcile(store)
                        if state is not None:
                            require(state.get('attempt') is None, 'unreconciled attempt')
                            state['reconciled_current'] = checkpoints.digest(store.current)
                            save(store, state)
                            store.remove_run(state['reconciled_current'])
                    state = start(store, args)
                    result = execute(store, state)
                else:
                    state = reconcile(store, abandon_unsealed=args.abandon_unsealed) if (store.root / checkpoints.RUN).exists() else None
                    if args.resume and state is not None:
                        if args.cutoff is not None:
                            require(dt.datetime.now(host.UTC) < args.cutoff <= dt.datetime.now(host.UTC) + dt.timedelta(hours=24), 'cutoff must be within 24 hours')
                            state['cutoff'] = args.cutoff.isoformat()
                            save(store, state)
                        result = execute(store, state)
                    else:
                        if state is not None:
                            state['reconciled_current'] = checkpoints.digest(store.current)
                            save(store, state)
                            store.remove_run(state['reconciled_current'])
                        result = tool(args.checkpoint_tool, '--inspect-current', store.current)
        print(json.dumps(result, allow_nan=False))
        return 0
    except (ValueError, OSError, subprocess.SubprocessError, RuntimeError) as error:
        print(f'current-checkpoint: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
