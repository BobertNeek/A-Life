#!/usr/bin/env python3
"""Plan, run or reconcile ONE finite same-world resident Feeding session.

The accepted founder starts one ordinary fresh life. No policy is installed
mid-life. The canonical checkpoint changes only after native stopped-run recovery
and atomic full-state admission. This does not schedule a night or close apps.
"""
import argparse
from copy import deepcopy
import datetime as dt
import json
import math
import os
from pathlib import Path
import subprocess
import time
import uuid

import current_checkpoint_store as cp
import run_current_care as current

require = cp.require
host = current.host
STATE = 'continuous-session.json'
REQUEST = 'continuous-request.json'
RECEIPT = 'last-continuous-session.json'
LOG = 'continuous-native'
NATIVE_RETIREMENT = 'resident-retirement.bin'
NATIVE_RETIREMENT_NEXT = 'resident-retirement.next'
NATIVE_INITIALIZATION = 'resident-initialization.bin'
NATIVE_INITIALIZATION_NEXT = 'resident-initialization.next'
# Retain STATE until last, but keep independent committed authority for retries
# even if STATE has already disappeared. Missing recorded files stay missing.
SIDECARS = (REQUEST, LOG + '.stdout.log', LOG + '.stderr.log',
            STATE + '.tmp', REQUEST + '.tmp', STATE)
MAX_LOG_BYTES = 8 * 1024 * 1024
HELPERS = ('run_continuous_care.py', 'run_current_care.py', 'current_checkpoint_store.py',
           'run_mixed_care_serial.py', 'run_eat_held_food_night.py',
           'grab_food_seed_reservations.py', 'summarize_ppo_diagnostics.py')


def request_for(args, now):
    require(type(args.decisions) is int and 1 <= args.decisions <= 36000,
            'finite Feeding decisions must be in 1..36000')
    require(type(args.seed) is int and type(args.last_seed) is int and
            0 <= args.seed <= args.last_seed <= 2**64 - 1, 'invalid seed allocation')
    require(type(args.final_reserve_seconds) is int and 180 <= args.final_reserve_seconds <= 3600,
            'reserve 180..3600 seconds for stopped recovery/publication')
    require(now < args.hard_end <= now + dt.timedelta(hours=24), 'fresh hard end required within 24 hours')
    cutoff = args.hard_end - dt.timedelta(seconds=args.final_reserve_seconds)
    require(cutoff > now, 'not enough time before final-save reserve')
    require(type(args.max_temporary_bytes) is int and args.max_temporary_bytes >= 2**30,
            'continuous session requires at least 1 GiB temporary allowance')
    require(args.food_need_condition in ('newborn_v1', 'ordinary_healthy_aging_v1'),
            'unsupported food need initialization')
    return {'schema': 1, 'seed_range': [args.seed, args.last_seed],
            'rows': [{'lesson': 'feeding', 'decisions': args.decisions,
                      'food_need_condition': args.food_need_condition}],
            'checkpoint_every_episodes': 1, 'sampling_temperature': 1.0,
            'collection_cutoff_unix_seconds': int(cutoff.timestamp()),
            'max_temporary_bytes': args.max_temporary_bytes}


def capabilities(report, decisions):
    require(report.get('schema') == 1 and report.get('kind') == 'native-resident-serial-v1' and
            report.get('finite_continuous_feeding') is True and
            report.get('max_continuous_feeding_decisions', 0) >= decisions and
            report.get('mid_world_offline_policy_install') is False and
            report.get('bound_run_cleanup') is True and
            report.get('recoverable_predispatch_initialization') is True and
            report.get('temporary_budget_enforced_during_writes') is True,
            'native finite-session, bound-cleanup, recoverable initialization and active-writer-budget capabilities required')


def check_pins(state):
    require(current.checkout_context() == state['checkout'], 'source checkout changed')
    require(current.prior_environment() == state['prior_environment'], 'semantic prior environment changed')
    for path, expected in state['executables'].items():
        require(host.sha256(Path(path)) == expected, 'pinned executable changed')
    for name, expected in state['helpers'].items():
        require(host.sha256(Path(__file__).parent / name) == expected, 'pinned host helper changed')


def gpu_snapshot(index):
    """Read-only NVIDIA telemetry for this first Windows/NVIDIA host path."""
    result = subprocess.run(['nvidia-smi', f'--id={index}',
        '--query-gpu=name,temperature.gpu,memory.free,utilization.gpu',
        '--format=csv,noheader,nounits'], check=True, capture_output=True, text=True, timeout=5)
    fields = result.stdout.strip().split(',')
    require(len(fields) == 4, 'unknown NVIDIA telemetry; no GPU admission')
    name, temperature, free_mib, utilization = (v.strip() for v in fields)
    return {'adapter': name, 'temperature_c': float(temperature),
            'free_memory_mib': float(free_mib), 'utilization_percent': float(utilization)}


def make_guard(args, lane, store):
    cached_at, gpu = -math.inf, None

    def guard():
        nonlocal cached_at, gpu
        require(dt.datetime.now(host.UTC) < args.hard_end, 'hard end reached')
        require(not (store / cp.RUN / 'STOP').exists(), 'owner STOP requested')
        resources = host.resource_snapshot(store)
        require(resources['disk_free_bytes'] >= args.min_disk_gib * 2**30 and
                resources['available_memory_bytes'] is not None and
                resources['available_memory_bytes'] >= args.min_memory_gib * 2**30,
                'disk/memory reserve reached')
        require(os.name != 'nt' or resources['idle_seconds'] >= args.idle_seconds,
                'actual interactive input resumed')
        if time.monotonic() - cached_at >= 5:
            gpu = gpu_snapshot(args.gpu_index)
            cached_at = time.monotonic()
        require(math.isfinite(gpu['temperature_c']) and gpu['temperature_c'] < args.max_gpu_c and
                gpu['free_memory_mib'] >= args.min_gpu_free_mib, 'GPU thermal/memory reserve reached')
        for suffix in ('.stdout.log', '.stderr.log'):
            path = lane / (LOG + suffix)
            require(not path.is_symlink(), 'native log alias refused')
            require(not path.exists() or path.stat().st_size <= MAX_LOG_BYTES, 'native log limit reached')
        return {'host': resources, 'gpu': gpu}

    return guard


def read_events(lane):
    path = lane / (LOG + '.stdout.log')
    if not path.exists():
        return []
    cp.regular(path)
    require(path.stat().st_size <= MAX_LOG_BYTES, 'oversized native log; preserve for reconciliation')
    events = []
    for line in path.read_text(encoding='utf-8', errors='replace').splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue  # Device warnings are not fabricated telemetry.
        if isinstance(event, dict) and event.get('kind') in (
                'resident-provisional-episode-v1', 'resident-serial-finished-v1'):
            events.append(event)
    return events


def sidecar_identity(path):
    if not path.exists():
        require(not path.is_symlink(), 'sidecar alias refused')
        return None
    before = cp.regular(path)
    digest = cp.digest(path)
    after = cp.regular(path)

    def fields(info):
        return {'device': info.st_dev, 'inode': info.st_ino, 'bytes': info.st_size,
                'mtime_ns': info.st_mtime_ns, 'ctime_ns': info.st_ctime_ns}

    require(fields(before) == fields(after), 'sidecar changed while recording retirement')
    return {**fields(after), 'sha256': digest}


def committed_retirement(lane, state=None):
    path = lane / RECEIPT
    if not path.exists():
        require(not path.is_symlink(), 'receipt alias refused')
        return None
    receipt = cp.read_json(path)
    commitment = receipt.get('retirement')
    if not isinstance(commitment, dict) or commitment.get('kind') != 'finite-continuous-retirement-v1':
        return None  # Historical receipts have no authority to remove leftovers.
    require(commitment.get('schema') == 1 and receipt.get('kind') == 'finite-continuous-care-v1',
            'invalid final retirement commitment')
    if state is None or commitment.get('owner') == state:
        return receipt
    # An old completed receipt may coexist with a genuinely new admitted run.
    # Same-session owner changes must not fall through and rebuild known results
    # from partially deleted logs. New admission binds the exact prior receipt.
    require(isinstance(state.get('session_id'), str) and bool(state['session_id']) and
            state['session_id'] != commitment['owner'].get('session_id') and
            state.get('previous_receipt_sha256') == cp.digest(path),
            'committed retirement owner conflict; preserving original outcomes')
    return None


def native_cleanup_pending(store):
    return any((store / name).exists() or (store / name).is_symlink()
               for name in (NATIVE_RETIREMENT, NATIVE_RETIREMENT_NEXT,
                            NATIVE_INITIALIZATION, NATIVE_INITIALIZATION_NEXT))


def retire_sidecars(lane, receipt, validate, after_remove=lambda _: None):
    """Retry the committed final housekeeping without rebuilding outcome counts."""
    commitment = receipt['retirement']
    state = commitment['owner']
    require(state.get('schema') == 1 and state.get('kind') == 'finite-continuous-owner-v1' and
            receipt['source'] == state['checkout'] and receipt['executables'] == state['executables'] and
            receipt['request'] == state['request'] and
            state['native_argv'] == [state['binary'], '--resident-serial', state['store'], str(lane / REQUEST)],
            'retirement owner/receipt/path mismatch')
    check_pins(state)
    worker = state['worker']
    require(worker['stopped'] or (worker['pid'] is not None and not cp.pid_alive(worker['pid'])),
            'retirement writer still alive or launch outcome unknown')
    store_root = Path(state['store'])
    # Native retirement must already have completed before this receipt commits.
    # Never use an old receipt to finish a different/new Native run.
    require(not (store_root / cp.RUN).exists() and not (store_root / cp.RUN).is_symlink() and
            not native_cleanup_pending(store_root),
            'unexpected native run/retirement after final receipt; preserving sidecars')
    require(set(commitment['sidecars']) == set(SIDECARS), 'incomplete sidecar retirement binding')
    allowed = {cp.LOCK, RECEIPT, *SIDECARS}
    require({p.name for p in lane.iterdir()} <= allowed, 'unknown lane entries during retirement')
    require(committed_retirement(lane) == receipt, 'final retirement receipt changed')
    with cp.Store(store_root, validate) as store:
        require(not (store_root / cp.RUN).exists() and
                not native_cleanup_pending(store_root) and
                cp.digest(store.current) == receipt['current_sha256'],
                'published current changed before sidecar retirement')
        # Validate every survivor BEFORE removing any more. Identity and digest
        # reject a new session's identical-looking request or recreated log.
        for name, expected in commitment['sidecars'].items():
            actual = sidecar_identity(lane / name)
            require(actual is None or actual == expected, 'sidecar retirement identity/digest conflict')
        for name in SIDECARS:
            path = lane / name
            if path.exists():
                require(sidecar_identity(path) == commitment['sidecars'][name], 'sidecar changed before removal')
                path.unlink()
                cp.sync_directory(lane)
                after_remove(name)
    return receipt


def reconcile(lane, state, validate):
    """Native and Python publishers never hold the same store lease together."""
    check_pins(state)
    worker = state['worker']
    require(worker['stopped'] or (worker['pid'] is not None and not cp.pid_alive(worker['pid'])),
            'native writer still alive or launch outcome unknown; preserving recovery files')
    store_root = Path(state['store'])
    native = Path(state['binary'])
    pending_cleanup = None
    initialization_pending = any((store_root / name).exists() or (store_root / name).is_symlink()
        for name in (NATIVE_INITIALIZATION, NATIVE_INITIALIZATION_NEXT))
    if native_cleanup_pending(store_root):
        # Python Store deliberately refuses Native's root-level markers.
        # Native owns its validation/retry; do this before any Python store lease.
        pending_cleanup = current.tool(native, '--finish-resident-run', store_root)
        if initialization_pending:
            require(pending_cleanup.get('kind') == 'resident-initialization-aborted-v1' and
                    pending_cleanup.get('already_published_current') is True and
                    pending_cleanup.get('cleanup_completed') is True and
                    pending_cleanup.get('initialization_aborted') is True and
                    isinstance(pending_cleanup.get('initialization_binding_verified'), bool) and
                    pending_cleanup.get('dispatched_worlds') == 0 and
                    pending_cleanup.get('seeds_consumed') == 0 and
                    pending_cleanup.get('new_request_allocation_committed') is False and
                    pending_cleanup.get('neural_execution') is False and
                    not (store_root / cp.RUN).exists() and not (store_root / cp.RUN).is_symlink() and
                    not native_cleanup_pending(store_root),
                    'native initialization abort did not finish; preserving owner state')
    # Finish any interrupted Python atomic publication before Native takes its lease.
    with cp.Store(store_root, validate) as store:
        actual = cp.digest(store.current)
    if not (store_root / cp.RUN).exists():
        # Possible crash after native cleanup but before sidecar retirement.
        require(actual == state.get('published_sha256', state['baseline_sha256']),
                'no native WAL and unexpected canonical state; preserve owner record')
        recovery = {'already_published_current': actual != state['baseline_sha256']}
        cleanup = pending_cleanup or {'already_clean': True}
        if initialization_pending:
            require(actual == state['baseline_sha256'] == pending_cleanup['candidate_sha256'] and
                    os.path.samefile(pending_cleanup['candidate'], store_root / cp.CURRENT),
                    'initialization abort changed its bound current')
            recovery = pending_cleanup
    else:
        recovery = current.tool(native, '--recover-resident', store_root)
        after = recovery['candidate_sha256']
        if not recovery['already_published_current']:
            candidate = Path(recovery['candidate'])
            require(os.path.samefile(candidate, store_root / cp.RUN / 'recovered-current.alife-checkpoint'),
                    'unexpected resident recovery candidate')
            require(cp.digest(candidate) == after, 'resident recovery candidate changed')
            state['publication_sha256'] = after
            cp.write_json(lane / STATE, state)
            with cp.Store(store_root, validate) as store:
                require(cp.digest(store.current) in (state['baseline_sha256'], after),
                        'canonical current changed outside this session')
                store.publish(candidate, cp.digest(store.current))
        state['published_sha256'] = after
        cp.write_json(lane / STATE, state)
        # Native validates the recovery intent + full current + WAL before removal.
        cleanup = current.tool(native, '--finish-resident-run', store_root)
    with cp.Store(store_root, validate) as store:
        admitted = validate(store.current)
        final_digest = cp.digest(store.current)
    check_pins(state)
    return admitted, final_digest, recovery, cleanup


def finish(lane, state, validate):
    committed = committed_retirement(lane, state)
    if committed is not None:
        return retire_sidecars(lane, committed, validate)
    admitted, digest, recovery, cleanup = reconcile(lane, state, validate)
    events = read_events(lane)
    rows = [e for e in events if e['kind'] == 'resident-provisional-episode-v1']
    elapsed = (dt.datetime.now(host.UTC) - dt.datetime.fromisoformat(state['started_utc'])).total_seconds()
    retained_rows = None
    if admitted['policy_version'] == state['before']['policy_version']:
        retained_rows = 0
    elif len(rows) == 1 and admitted['policy_version'] == rows[0]['policy_version']:
        retained_rows = rows[0]['trained_rows']
    receipt = {'kind': 'finite-continuous-care-v1', 'ended_utc': dt.datetime.now(host.UTC).isoformat(),
        'source': state['checkout'], 'executables': state['executables'],
        'native_argv': state['native_argv'], 'prior_environment': state['prior_environment'],
        'preflight': state['preflight'], 'guard_configuration': state['guard_configuration'],
        'baseline_sha256': state['baseline_sha256'], 'current_sha256': digest,
        'before': state['before'], 'after': {key: admitted.get(key) for key in state['before']},
        'request': state['request'], 'host_wall_seconds': elapsed,
        'host_wall_interval_scope': 'host invocation through admission, native recovery/publication/cleanup, final admission and event read; includes crash/reconciliation downtime; excludes final compact receipt and sidecar housekeeping',
        'retained_trained_rows': retained_rows,
        'retained_decision_rows_per_wall_hour': retained_rows * 3600 / elapsed
            if retained_rows is not None and elapsed > 0 else None,
        'actor_optimizer_step_delta': admitted['actor_optimizer_step'] - state['before']['actor_optimizer_step'],
        'value_optimizer_step_delta': admitted['value_optimizer_step'] - state['before']['value_optimizer_step'],
        'native_exit_error': state.get('native_exit_error'), 'observed_rows': rows,
        'native_exit_code': state.get('native_exit_code'),
        'recovery': recovery, 'cleanup': cleanup,
        'one_fresh_world': not recovery.get('initialization_aborted', False),
        'same_individual_across_sessions': False,
        'causal_speed': rows[0].get('causal_collection') if len(rows) == 1 else None,
        'outcome_scope': 'provisional row outcomes; only native admitted snapshot is retained',
        'recent_unsnapshotted_updates_may_be_lost': True}
    require(cp.read_json(lane / STATE) == state and cp.read_json(lane / REQUEST) == state['request'],
            'owner/request changed before committing retirement')
    receipt['retirement'] = {'schema': 1, 'kind': 'finite-continuous-retirement-v1',
        'owner': deepcopy(state), 'sidecars': {name: sidecar_identity(lane / name) for name in SIDECARS}}
    cp.write_json(lane / RECEIPT, receipt)
    return retire_sidecars(lane, receipt, validate)


def run(args):
    invocation_started = dt.datetime.now(host.UTC).isoformat()
    store_root = args.store.resolve(strict=True)
    lane = args.lane.resolve(strict=True)
    require(cp.real_directory(store_root) and cp.real_directory(lane), 'real store/lane directories required')
    current.disjoint(store_root, lane)
    checkout_root = Path(__file__).resolve().parent.parent
    current.disjoint(checkout_root, lane)
    current.disjoint(checkout_root, store_root)
    native = args.binary.resolve(strict=True)
    inspector = args.checkpoint_tool.resolve(strict=True)
    validate = current.validator(inspector)
    with cp.writer_lock(lane):
        allowed = {cp.LOCK, RECEIPT, RECEIPT + '.tmp', *SIDECARS}
        require({p.name for p in lane.iterdir()} <= allowed, 'unknown lane entries; no cleanup')
        if args.reconcile:
            if (lane / STATE).exists():
                state = cp.read_json(lane / STATE)
                committed = committed_retirement(lane, state)
            else:
                committed = committed_retirement(lane)
                require(committed is not None, 'no bound retirement authority for missing owner state')
                state = committed['retirement']['owner']
            require(state.get('schema') == 1 and state.get('kind') == 'finite-continuous-owner-v1' and
                    state['store'] == str(store_root) and state['binary'] == str(native) and
                    state['inspector'] == str(inspector), 'different resident owner')
            return retire_sidecars(lane, committed, validate) if committed is not None else finish(lane, state, validate)
        require(not (lane / STATE).exists() and not (store_root / cp.RUN).exists() and
                not native_cleanup_pending(store_root),
                'reconcile existing native run/owner before dispatch')
        require(all(not (lane / n).exists() and not (lane / n).is_symlink() for n in SIDECARS),
                'unexplained prior sidecars; preserving files')
        request = request_for(args, dt.datetime.now(host.UTC))
        require(host.sha256(native) == args.binary_sha256 and host.sha256(inspector) == args.checkpoint_tool_sha256,
                'explicit admitted executable hashes required')
        capabilities(current.tool(native, '--resident-contract'), args.decisions)
        checkout = current.checkout_context()
        require(current.prior_environment().get('ALIFE_SLM_PRIOR_BACKEND') == 'deterministic' and
                current.prior_environment().get('ALIFE_SLM_PRIOR') == 'on',
                'explicit ALIFE_SLM_PRIOR=on and ALIFE_SLM_PRIOR_BACKEND=deterministic required')
        guard = make_guard(args, lane, store_root)
        preflight = guard()
        host.assert_lane_idle()
        with cp.Store(store_root, validate) as store:
            before = validate(store.current)
            baseline = cp.digest(store.current)
            current.tool(inspector, '--check-current-allocation', store.current, args.seed, args.last_seed)
        state = {'schema': 1, 'kind': 'finite-continuous-owner-v1', 'store': str(store_root),
            'session_id': str(uuid.uuid4()),
            'previous_receipt_sha256': cp.digest(lane / RECEIPT) if (lane / RECEIPT).exists() else None,
            'binary': str(native), 'inspector': str(inspector), 'baseline_sha256': baseline,
            'before': {key: before[key] for key in ('policy_version', 'actor_optimizer_step', 'value_optimizer_step')},
            'request': request, 'checkout': checkout, 'prior_environment': current.prior_environment(),
            'executables': {str(native): args.binary_sha256, str(inspector): args.checkpoint_tool_sha256},
            'helpers': {name: host.sha256(Path(__file__).parent / name) for name in HELPERS},
            'started_utc': invocation_started, 'preflight': preflight,
            'native_argv': [str(native), '--resident-serial', str(store_root), str(lane / REQUEST)],
            'guard_configuration': {key: getattr(args, key) for key in ('idle_seconds', 'min_disk_gib',
                'min_memory_gib', 'gpu_index', 'max_gpu_c', 'min_gpu_free_mib', 'final_reserve_seconds')},
            'worker': {'pid': None, 'stopped': True}}
        cp.write_json(lane / STATE, state)
        cp.write_json(lane / REQUEST, request)
        active = []

        def started(process):
            active.append(process)
            state['worker'] = {'pid': process.pid, 'stopped': False}
            cp.write_json(lane / STATE, state)

        check_pins(state)
        state['worker'] = {'pid': None, 'stopped': False}
        cp.write_json(lane / STATE, state)  # Unknown launch outcomes fail closed.
        try:
            host.run_child(state['native_argv'],
                lane / LOG, dt.datetime.fromtimestamp(request['collection_cutoff_unix_seconds'], host.UTC),
                guard, host.prior_environment(), on_started=started)
        except Exception as error:
            state['native_exit_error'] = f'{type(error).__name__}: {error}'
        finally:
            if active and active[0].poll() is not None:
                state['worker']['stopped'] = True
                state['native_exit_code'] = active[0].poll()
            # If Popen never called back, retain unknown launch outcome for manual reconciliation.
            cp.write_json(lane / STATE, state)
        return finish(lane, state, validate)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('store', 'lane', 'binary', 'checkpoint-tool'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--binary-sha256')
    parser.add_argument('--checkpoint-tool-sha256')
    parser.add_argument('--seed', type=int)
    parser.add_argument('--last-seed', type=int)
    parser.add_argument('--decisions', type=int, default=512)
    parser.add_argument('--food-need-condition', choices=('newborn_v1', 'ordinary_healthy_aging_v1'), default='newborn_v1')
    parser.add_argument('--hard-end', type=host.cutoff_time)
    parser.add_argument('--final-reserve-seconds', type=int, default=180)
    parser.add_argument('--max-temporary-bytes', type=int, default=2 * 2**30)
    parser.add_argument('--idle-seconds', type=int, default=300)
    parser.add_argument('--min-disk-gib', type=float, default=20)
    parser.add_argument('--min-memory-gib', type=float, default=8)
    parser.add_argument('--gpu-index', type=int, default=0)
    parser.add_argument('--max-gpu-c', type=float, default=75)
    parser.add_argument('--min-gpu-free-mib', type=float, default=2048)
    parser.add_argument('--reconcile', action='store_true')
    parser.add_argument('--run', action='store_true')
    args = parser.parse_args(argv)
    require(0 <= args.idle_seconds <= 3600 and args.gpu_index >= 0 and
            all(math.isfinite(v) and v > 0 for v in (args.min_disk_gib, args.min_memory_gib,
                args.max_gpu_c, args.min_gpu_free_mib)), 'invalid resource guards')
    if not args.reconcile:
        require(args.hard_end is not None, '--hard-end is required')
        request = request_for(args, dt.datetime.now(host.UTC))
    else:
        request = None
    if not args.run:
        return {'kind': 'finite-continuous-plan-v1', 'request': request,
                'mutation': False, 'native_execution': False,
                'required_next_step': 'fresh owner/resource and exact executable admission, then --run'}
    return run(args)


if __name__ == '__main__':
    print(json.dumps(main(), allow_nan=False, indent=2))
