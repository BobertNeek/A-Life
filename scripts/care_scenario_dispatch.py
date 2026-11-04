#!/usr/bin/env python3
"""Private, one-shot native dispatcher for disposable scenario discovery only.

`init` is unavailable until a qualified construction-only native founder backend
exists. The old --fixture path trains and is deliberately not supported. `run`
requires a sealed zero-update construction identity, durably claims one proposed
batch, and evaluates only reserved discovery seeds. Never resumes uncertain
work, trains, uses pool/holdout/reference seeds, or invokes a model or shell.
Only care_scenario_batches.prepare() output belongs in a designer context.
"""
import argparse
import os
from pathlib import Path
import re
import selectors
import signal
import subprocess
import sys
import time

import care_scenario_batches as batches
from care_frontier_sampler import REQUEST_FIELDS, _score_archive_metrics, integer, validate_bank
from compare_cpu_care_rounds import file_pin, native_pins, raw_tree
from import_care_scenario_recipe import reject_symlinks

require = batches.require
MAX_COMMAND_BYTES = 8 * 1024 * 1024  # Combined stdout and stderr, including failures.
COMMAND_TIMEOUT = 60
CONSTRUCTION_MODE = 'construction_only_zero_updates_v1'
EVALUATION_SOURCE_FILES = {
    'cpu-cycle.json', 'training-handoff.json', 'trained.alife-foundation',
    'actor-checkpoint.json', 'value-checkpoint.json', 'phase.txt',
}
MANIFEST_FIELDS = {
    'schema', 'scope', 'run_root', 'ledger_root', 'plan_sha256', 'founder_seed',
    'source_root', 'care_binary', 'native_evaluator', 'executable_pins',
    'assessment_seeds', 'command_timeout_seconds', 'command_output_bytes', 'construction_mode',
}


class ConstructionOnlyUnavailable(ValueError):
    pass


def construction_only_backend():
    """Fail closed: no supported native zero-update founder exporter exists.

    This internal seam is mocked in lifecycle tests only. No environment
    variable, CLI switch, training fixture fallback, or source import enables it.
    A future implementation must construct and natively seal a founder without
    stepping a world, running PPO/Adam, or inventing training provenance.
    """
    raise ConstructionOnlyUnavailable('a qualified construction-only native founder backend is unavailable')


def verify_construction(source, founder_seed, receipt):
    zero_fields = {'world_steps', 'training_rows', 'actor_optimizer_updates',
                   'value_optimizer_updates', 'actor_optimizer_step', 'value_optimizer_step'}
    require(isinstance(receipt, dict) and set(receipt) == zero_fields | {
                'mode', 'founder_seed', 'source_sha256', 'teacher_present', 'semantic_prior_enabled'}
            and receipt['mode'] == CONSTRUCTION_MODE and receipt['founder_seed'] == founder_seed
            and all(type(receipt[key]) is int and receipt[key] == 0 for key in zero_fields)
            and receipt['teacher_present'] is False and receipt['semantic_prior_enabled'] is False
            and receipt['source_sha256'] == evaluation_source_pins(source),
            'founder construction must prove no world steps or offline updates')
    actor = batches.read_record(source / 'actor-checkpoint.json')
    value = batches.read_record(source / 'value-checkpoint.json')
    require(type(actor.get('optimizer_step')) is int and actor['optimizer_step'] == 0
            and type(value.get('optimizer_step')) is int and value['optimizer_step'] == 0,
            'founder construction checkpoints contain offline updates')


def construct_founder(backend, source, founder_seed, binary, evidence, guard):
    """One claimed construction attempt; partial evidence is never removed."""
    guard.check()
    require(not source.exists(), 'founder output exists; never replay construction')
    mkdir_new(evidence)
    batches.write_new(evidence / 'request.json', {'mode': CONSTRUCTION_MODE, 'source': str(source),
                                               'founder_seed': founder_seed, 'binary': str(binary)})
    batches.fsync_dir(evidence)
    outcome = {'status': 'uncertain', 'retry_allowed': False}
    try:
        receipt = backend(source, founder_seed, binary, evidence, guard)
        batches.write_new(evidence / 'receipt.json', receipt)
        verify_construction(source, founder_seed, receipt)
        outcome['status'] = 'completed'
    finally:
        batches.write_new(evidence / 'outcome.json', outcome)
        batches.fsync_dir(evidence)
        guard.check()
    return receipt


def explicit_path(path, *, exists=True):
    path = Path(path)
    reject_symlinks(path)
    return path.resolve(strict=exists)


def separate(left, right):
    return not (left.is_relative_to(right) or right.is_relative_to(left))


def mkdir_new(path):
    reject_symlinks(path)
    path.mkdir(mode=0o700)
    batches.fsync_dir(path.parent)


def tree_pin(path):
    """Pin empty directories too, especially interrupted ledger claims."""
    files = raw_tree(path)
    directories = sorted(str(Path(root).relative_to(path)) for root, _, _ in os.walk(path))
    return {'files': files, 'directories': directories}


def completed_batch_inventory(path):
    inventory = tree_pin(path)
    inventory['files'].pop('completed.json', None)  # The seal cannot hash itself.
    return inventory


def discovery_seeds(plan):
    return sorted(task['seed'] for index in range(plan['batches'])
                  for task in batches.inputs(plan, index, plan['initial_difficulty'])[1])


def evaluation_source_pins(source):
    # Native evaluation admits six standard files plus optional compiler inputs.
    # The older four-file native_pins mapping is still used by seed history.
    names = EVALUATION_SOURCE_FILES | ({'compiler-inputs.json'}
                                      if (source / 'compiler-inputs.json').exists() else set())
    return {name: file_pin(source / name)['sha256'] for name in sorted(names)}


class Guard:
    def __init__(self, ledger):
        self.ledger = ledger
        self.ledger_pin = tree_pin(ledger)
        self.files, self.trees = {}, {}

    def freeze_file(self, path, pin=None):
        self.files[path] = file_pin(path) if pin is None else pin

    def freeze_tree(self, path, pin=None):
        self.trees[path] = tree_pin(path) if pin is None else pin

    def check(self):
        require(tree_pin(self.ledger) == self.ledger_pin, 'ledger changed during dispatch')
        for path, pin in self.files.items():
            reject_symlinks(path)
            require(file_pin(path) == pin, 'pinned input or native executable changed')
        for path, pin in self.trees.items():
            reject_symlinks(path)
            require(tree_pin(path) == pin, 'frozen source/archive changed')

    def ledger_stage(self, index, *, complete):
        """Accept only our own expected append; all prior bytes remain pinned."""
        before, after = self.ledger_pin, tree_pin(self.ledger)
        name = '%04d' % index
        expected_dirs = sorted(set(before['directories']) | {name})
        require(after['directories'] == expected_dirs, 'unexpected ledger stage change')
        allowed = {name + '/record.json', name + '/complete.json'} if complete else set()
        require(set(after['files']) == set(before['files']) | allowed
                and all(after['files'][key] == pin for key, pin in before['files'].items()),
                'changed or unexpected ledger evidence')
        self.ledger_pin = after
        self.check()


def _kill(process):
    # Each fixed native command owns its process group. Descendants must not
    # continue mutating evidence after a timeout/interruption.
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait(timeout=5)


class NativeCommandTerminated(subprocess.SubprocessError):
    def __init__(self, signum):
        self.signum = signum
        super().__init__('native command interrupted by signal %d; completion remains uncertain' % signum)


def _bounded_process(argv, cwd, stdout, stderr):
    """Bound output/time and kill/reap the session on exceptions or TERM/HUP.

    SIGKILL and power loss cannot run cleanup. Their surviving evidence remains
    uncertain and must never authorize automatic replay or seed reuse.
    """
    environment = {'PATH': os.defpath, 'LANG': 'C.UTF-8', 'LC_ALL': 'C.UTF-8',
                   'ALIFE_SLM_PRIOR': 'off', 'ALIFE_SLM_PRIOR_BACKEND': 'off'}
    process, pending_signal, cleaning_up, completed = None, None, False, False
    previous_handlers = {}
    selector = selectors.DefaultSelector()

    def terminate(signum, _frame):
        nonlocal pending_signal
        pending_signal = signum
        # Popen may be between spawning and returning its process handle. Defer
        # raising in that tiny interval so the child can still be killed/reaped.
        if process is not None and not cleaning_up:
            raise NativeCommandTerminated(signum)

    try:
        for signum in (signal.SIGTERM, signal.SIGHUP):
            previous_handlers[signum] = signal.getsignal(signum)
            signal.signal(signum, terminate)  # Before spawn; main-thread ownership required.
        if pending_signal is not None:
            raise NativeCommandTerminated(pending_signal)
        process = subprocess.Popen(argv, cwd=cwd, env=environment, stdin=subprocess.DEVNULL,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   shell=False, start_new_session=True)
        if pending_signal is not None:
            raise NativeCommandTerminated(pending_signal)
        deadline, total = time.monotonic() + COMMAND_TIMEOUT, 0
        for pipe, destination in ((process.stdout, stdout), (process.stderr, stderr)):
            os.set_blocking(pipe.fileno(), False)
            selector.register(pipe, selectors.EVENT_READ, destination)
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise subprocess.TimeoutExpired(argv, COMMAND_TIMEOUT)
            for key, _ in selector.select(min(remaining, 0.1)):
                chunk = os.read(key.fileobj.fileno(), min(64 * 1024, MAX_COMMAND_BYTES - total + 1))
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                available = MAX_COMMAND_BYTES - total
                key.data.write(chunk[:available])
                total += len(chunk)
                require(total <= MAX_COMMAND_BYTES, 'native command output exceeds bound; never retry')
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise subprocess.TimeoutExpired(argv, COMMAND_TIMEOUT)
        code = process.wait(timeout=remaining)
        completed = True
        return code
    except BaseException:
        cleaning_up = True
        if process is not None:
            _kill(process)
        raise
    finally:
        cleaning_up = True
        selector.close()
        if process is not None:
            process.stdout.close()
            process.stderr.close()
        for signum, previous in previous_handlers.items():
            signal.signal(signum, previous)
        if completed and pending_signal is not None:
            raise NativeCommandTerminated(pending_signal)


def command(binary, arguments, cwd, evidence, guard):
    """Only callers below construct argv; every attempt has exclusive evidence."""
    guard.check()
    mkdir_new(evidence)
    argv = [str(binary), *map(str, arguments)]
    manifest = batches.read_record(cwd / 'run-manifest.json')
    require(manifest.get('construction_mode') == CONSTRUCTION_MODE,
            'construction-only run identity required')
    require(len(argv) == 4 and argv[:3] == [manifest['native_evaluator'], 'evaluate', '--spec'],
            'only fixed native assessment commands are permitted; fixture training is prohibited')
    spec = explicit_path(argv[3])
    relative = spec.relative_to(cwd / 'batches')
    require(len(relative.parts) == 2 and re.fullmatch(r'[0-9]{4}', relative.parts[0])
            and re.fullmatch(r'assessment-[0-9]{4}\.request\.json', relative.parts[1])
            and spec in guard.files, 'assessment requires a generated pinned request')
    batches.write_new(evidence / 'command.json', {'argv': argv, 'cwd': str(cwd),
                      'timeout_seconds': COMMAND_TIMEOUT, 'output_bytes': MAX_COMMAND_BYTES,
                      'semantic_prior': 'off'})
    batches.fsync_dir(evidence)
    outcome = {'status': 'uncertain', 'retry_allowed': False}
    try:
        with (evidence / 'stdout.json').open('xb') as stdout, (evidence / 'stderr.log').open('xb') as stderr:
            try:
                code = _bounded_process(argv, cwd, stdout, stderr)
            finally:
                for stream in (stdout, stderr):
                    stream.flush(); os.fsync(stream.fileno())
        outcome.update(status='completed' if code == 0 else 'failed', exit_code=code)
        require(code == 0, 'native command failed; preserve evidence and never retry')
    except subprocess.TimeoutExpired:
        outcome['status'] = 'timeout_uncertain'
        raise
    except BaseException:
        # Includes malformed execution, output overflow and interruption. Never
        # convert these to a negative biological score or retry permission.
        raise
    finally:
        batches.write_new(evidence / 'outcome.json', outcome)
        batches.fsync_dir(evidence)
        guard.check()
    return batches.read_record(evidence / 'stdout.json')


def initialize(run_root, ledger_root, care_binary, evaluator, founder_seed):
    require(os.name == 'posix' and hasattr(os, 'O_DIRECTORY'), 'local POSIX filesystem required')
    run_root = explicit_path(run_root, exists=False)
    ledger = explicit_path(ledger_root)
    require(separate(run_root, ledger), 'run and ledger roots must be separate')
    require(not run_root.exists(), 'fresh disposable run root required; never resume initialization')
    care, evaluator = explicit_path(care_binary), explicit_path(evaluator)
    records = batches.load(ledger)
    require(all(record['kind'] in {'plan', 'prepared', 'proposed'} for record in records),
            'cannot initialize after any dispatch')
    plan = records[0]['payload']
    require(integer(founder_seed, 1, 2**64-1) and founder_seed in plan['excluded_seeds'],
            'explicit founder seed must be in plan excluded_seeds')
    pins = {str(path): file_pin(path) for path in (care, evaluator)}
    require(pins[str(evaluator)]['sha256'] == plan['native_evaluator_sha256'],
            'native evaluator differs from plan')
    source = run_root / 'initial-source'
    manifest = {'schema': 2, 'scope': batches.SCOPE, 'run_root': str(run_root),
                'construction_mode': CONSTRUCTION_MODE,
                'ledger_root': str(ledger), 'plan_sha256': batches.digest(plan),
                'founder_seed': founder_seed, 'source_root': str(source),
                'care_binary': str(care), 'native_evaluator': str(evaluator),
                'executable_pins': pins, 'assessment_seeds': discovery_seeds(plan),
                'command_timeout_seconds': COMMAND_TIMEOUT, 'command_output_bytes': MAX_COMMAND_BYTES}
    backend = construction_only_backend()  # Preflight before claims, directories, or native calls.
    founder = batches.claim_founder(ledger, {'run_root': str(run_root), 'founder_seed': founder_seed,
        'care_binary_sha256': pins[str(care)]['sha256'], 'evaluator_sha256': pins[str(evaluator)]['sha256']})
    require(founder['plan_sha256'] == manifest['plan_sha256'] and batches.load(ledger) == records,
            'ledger changed during founder claim')
    guard = Guard(ledger)
    for path in (care, evaluator):
        guard.freeze_file(path, pins[str(path)])
    mkdir_new(run_root)
    batches.write_new(run_root / 'run-manifest.json', manifest)
    guard.freeze_file(run_root / 'run-manifest.json')
    construct_founder(backend, source, founder_seed, care, run_root / 'founder-construction', guard)
    source_manifest = {'schema': 1, 'run_manifest_sha256': file_pin(run_root / 'run-manifest.json')['sha256'],
                       'source_root': str(source), 'inventory': tree_pin(source),
                       'source_sha256': evaluation_source_pins(source), 'history_source_sha256': native_pins(source)}
    guard.freeze_tree(source, source_manifest['inventory'])
    batches.write_new(run_root / 'source-manifest.json', source_manifest)
    guard.freeze_file(run_root / 'source-manifest.json')
    mkdir_new(run_root / 'batches')
    guard.check()
    batches.write_new(run_root / 'initialized.json', {
        'run_manifest': file_pin(run_root / 'run-manifest.json'),
        'source_manifest': file_pin(run_root / 'source-manifest.json'),
        'founder_construction': tree_pin(run_root / 'founder-construction')})
    batches.fsync_dir(run_root)
    return {'scope': batches.SCOPE, 'status': 'initialized', 'training_enabled': False}


def load_run(run_root):
    root = explicit_path(run_root)
    seal = batches.read_record(root / 'initialized.json')  # Missing/partial init never resumes.
    manifest = batches.read_record(root / 'run-manifest.json')
    require(isinstance(manifest, dict) and set(manifest) == MANIFEST_FIELDS
            and type(manifest['schema']) is int and manifest['schema'] == 2
            and manifest['construction_mode'] == CONSTRUCTION_MODE
            and manifest['scope'] == batches.SCOPE and manifest['run_root'] == str(root)
            and manifest['source_root'] == str(root / 'initial-source')
            and manifest['command_timeout_seconds'] == COMMAND_TIMEOUT
            and manifest['command_output_bytes'] == MAX_COMMAND_BYTES, 'invalid bound run manifest')
    require(seal == {'run_manifest': file_pin(root / 'run-manifest.json'),
                     'source_manifest': file_pin(root / 'source-manifest.json'),
                     'founder_construction': tree_pin(root / 'founder-construction')}, 'initialization evidence changed')
    ledger = explicit_path(manifest['ledger_root'])
    require(separate(root, ledger), 'run and ledger roots must be separate')
    records = batches.load(ledger)
    plan = records[0]['payload']
    require(manifest['plan_sha256'] == batches.digest(plan)
            and manifest['founder_seed'] in plan['excluded_seeds']
            and manifest['assessment_seeds'] == discovery_seeds(plan), 'bound ledger plan changed')
    care, evaluator = explicit_path(manifest['care_binary']), explicit_path(manifest['native_evaluator'])
    require(manifest['executable_pins'] == {str(path): file_pin(path) for path in (care, evaluator)}
            and file_pin(evaluator)['sha256'] == plan['native_evaluator_sha256'], 'native executable changed')
    founder = batches.read_record(ledger / 'founder-claim.json')
    require(founder['identity'] == {'run_root': str(root), 'founder_seed': manifest['founder_seed'],
            'care_binary_sha256': manifest['executable_pins'][str(care)]['sha256'],
            'evaluator_sha256': manifest['executable_pins'][str(evaluator)]['sha256']},
            'run differs from claimed founder initialization')
    source = root / 'initial-source'
    source_manifest = batches.read_record(root / 'source-manifest.json')
    require(source_manifest == {'schema': 1, 'run_manifest_sha256': file_pin(root / 'run-manifest.json')['sha256'],
                               'source_root': str(source), 'inventory': tree_pin(source),
                               'source_sha256': evaluation_source_pins(source),
                               'history_source_sha256': native_pins(source)}, 'frozen source manifest changed')
    verify_construction(source, manifest['founder_seed'], batches.read_record(root / 'founder-construction' / 'receipt.json'))
    guard = Guard(ledger)
    for path in (root / 'run-manifest.json', root / 'source-manifest.json', root / 'initialized.json', care, evaluator):
        guard.freeze_file(path)
    guard.freeze_tree(source, source_manifest['inventory'])
    guard.freeze_tree(root / 'founder-construction', seal['founder_construction'])
    for record in records:
        if record['kind'] == 'assessed':
            payload = record['payload']
            batch_root = root / 'batches' / ('%04d' % payload['batch'])
            completed = batches.read_record(batch_root / 'completed.json')
            require(completed == {'batch': payload['batch'], 'aggregate': payload['aggregate'],
                                  'training_enabled': False, 'promotion': 'none',
                                  'inventory': completed_batch_inventory(batch_root)},
                    'completed batch controls or evidence changed')
            guard.freeze_tree(batch_root)
            for evidence in payload['archives']:
                archive = explicit_path(evidence['archive'])
                require(archive.is_relative_to(root) and batches.digest(raw_tree(archive)) == evidence['inventory_sha256'],
                        'previous discovery archive changed or belongs to another run')
                guard.freeze_tree(archive)
    guard.check()
    return root, ledger, manifest, source_manifest, records, guard


def run(run_root):
    root, ledger, manifest, source_manifest, records, guard = load_run(run_root)
    require(records[-1]['kind'] == 'proposed', 'run requires a new proposal; never replay a dispatch')
    evaluator, source = Path(manifest['native_evaluator']), Path(manifest['source_root'])
    identity = {'source_sha256': source_manifest['source_sha256'], 'run_root': str(root),
                'evaluator_sha256': manifest['executable_pins'][str(evaluator)]['sha256']}
    guard.check()
    private = batches.claim_dispatch(ledger, identity)  # Durable consumption BEFORE any assessment command.
    guard.ledger_stage(len(records), complete=True)
    plan = records[0]['payload']
    expected_host, expected_discovery = batches.inputs(plan, private['batch'], private['host']['current_difficulty'])
    require(private['host'] == expected_host and private['discovery'] == expected_discovery
            and batches.digest(private['bank']) == records[-1]['payload']['bank_sha256']
            and private['bank']['decision_budget'] == plan['decision_budget'], 'private handoff differs from bound plan')
    families = validate_bank(private['bank'])
    batch_root = root / 'batches' / ('%04d' % private['batch'])
    mkdir_new(batch_root)
    batches.write_new(batch_root / 'bank.json', private['bank'])
    guard.freeze_file(batch_root / 'bank.json')
    history = batch_root / 'seed-history.json'
    batches.write_new(history, {'schema': 1, 'kind': 'mixed-care-seed-history', 'complete': True,
        'evidence': 'Only this zero-update constructed disposable founder; no retained lineage or generalization claim',
        'source_sha256': source_manifest['history_source_sha256'],
        'consumed_seed_ranges': [[manifest['founder_seed'], manifest['founder_seed']]],
        'reserved_seed_ranges': [], 'assessment_seeds': manifest['assessment_seeds'], 'imports': []})
    guard.freeze_file(history)
    archives, requests = [], {}
    for index, task in enumerate(private['discovery']):
        prefix = batch_root / ('assessment-%04d' % index)
        family = families[task['family']]
        request = {'previous': str(source), 'output': str(prefix), 'seed_history': str(history),
                   'seed': task['seed'], 'decisions': plan['decision_budget'], 'temperature': 1.0,
                   'lesson': family['lesson'], 'food_need_condition': family['food_need_condition'],
                   'known_exclusions': False}
        for key in ('food_offset', 'layout'):
            if key in family:
                request[key] = family[key]
        spec = prefix.with_suffix('.request.json')
        batches.write_new(spec, request)
        guard.freeze_file(spec)
        require(not prefix.exists(), 'discovery output already exists; never replay')
        native = command(evaluator, ['evaluate', '--spec', spec], root,
                         batch_root / ('dispatch-%04d' % index), guard)
        receipt = batches.read_record(prefix / 'care-evaluation.json')
        actual_request = dict(receipt['request'])
        actual_request.setdefault('food_need_condition', 'newborn_v1')
        require(actual_request == request and receipt['result'] == native
                and all(native[key] == request[key] for key in ('seed', 'lesson', 'decisions'))
                and native['source_sha256'] == identity['source_sha256'], 'native receipt/request/source mismatch')
        guard.freeze_tree(prefix)
        archives.append(prefix)
        requests[prefix] = (request, spec, index)

    assessment_stage = len(records) + 1
    stage_seen = False

    def score(archive, binary):
        nonlocal stage_seen
        if not stage_seen:
            guard.ledger_stage(assessment_stage, complete=False)
            stage_seen = True
        guard.check()
        archive = explicit_path(archive)
        require(archive in requests and explicit_path(binary) == evaluator, 'unexpected archive admission')
        request, spec, index = requests[archive]
        receipt = batches.read_record(archive / 'care-evaluation.json')
        require(set(receipt) == {'schema_version', 'request', 'input_sha256', 'artifacts', 'result', 'payload_sha256'}
                and receipt['schema_version'] == 1, 'unsupported native assessment receipt')
        actual = receipt['request']
        require(set(actual) - {'food_offset', 'layout'} in (set(REQUEST_FIELDS), set(REQUEST_FIELDS)-{'food_need_condition'})
                and actual['temperature'] == 1.0 and explicit_path(actual['output']) == archive,
                'unsupported native assessment request')
        normalized = dict(actual); normalized.setdefault('food_need_condition', 'newborn_v1')
        require(normalized == request, 'completed native request changed')
        native = command(evaluator, ['evaluate', '--spec', spec], root,
                         batch_root / ('verify-%04d' % index), guard)
        require(native == receipt['result'], 'native completed assessment validation failed')
        guard.check()
        scored = _score_archive_metrics(archive, receipt)  # Only after guarded native admission.
        guard.check()
        return scored

    aggregate = batches.assess(ledger, archives, evaluator, score=score)
    guard.ledger_stage(assessment_stage, complete=True)
    batches.load(ledger)
    batches.write_new(batch_root / 'completed.json', {'batch': private['batch'], 'aggregate': aggregate,
                                                    'training_enabled': False, 'promotion': 'none',
                                                    'inventory': completed_batch_inventory(batch_root)})
    guard.freeze_tree(batch_root)
    batches.fsync_dir(batch_root)
    guard.check()
    return {'scope': batches.SCOPE, 'batch': private['batch'], 'aggregate': aggregate,
            'training_enabled': False, 'promotion': 'none'}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run-root', type=Path, required=True)
    sub = parser.add_subparsers(dest='command', required=True)
    init = sub.add_parser('init')
    init.add_argument('--ledger-root', type=Path, required=True)
    init.add_argument('--care-binary', type=Path, required=True)
    init.add_argument('--native-evaluator', type=Path, required=True)
    init.add_argument('--founder-seed', type=int, required=True)
    sub.add_parser('run')
    args = parser.parse_args(argv)
    try:
        result = initialize(args.run_root, args.ledger_root, args.care_binary,
                            args.native_evaluator, args.founder_seed) if args.command == 'init' else run(args.run_root)
    except ConstructionOnlyUnavailable:
        print('Initialization unavailable: no qualified construction-only native founder backend. '
              'No founder, claim, output directory, or native command was created.', file=sys.stderr)
        return 1
    except (OSError, ValueError, TypeError, KeyError, RecursionError, subprocess.SubprocessError):
        print('Disposable dispatch stopped. Preserve all evidence; do not retry uncertain native work.', file=sys.stderr)
        return 1
    print(batches.encode(result).decode(), end='')
    return 0


if __name__ == '__main__':
    sys.exit(main())
