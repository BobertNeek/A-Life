#!/usr/bin/env python3
"""Actual CLI/strict-JSON gates using disposable CPU Current fixtures only.

No learning, GPU dispatch or production input modification. The terminal fixture
comes from the native continuous_store unit test, with its observed disposition.
"""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

import run_current_care as host
import run_continuous_care as continuous
import current_checkpoint_store as checkpoints


def digest(data):
    return hashlib.sha256(data).hexdigest()


def packed(value):
    return json.dumps(value, separators=(',', ':'), ensure_ascii=False).encode()


def envelope(magic, value):
    data = packed(value)
    assert len(magic) == 16
    return magic + len(data).to_bytes(8, 'little') + hashlib.sha256(data).digest() + data


def request(seed):
    now = int(time.time())
    return dict(schema=1, seed_range=[seed, seed + 2], total_trained_decisions=100,
                decisions_per_update=10, snapshot_every_updates=1, sampling_temperature=1.0,
                collection_cutoff_unix_seconds=now + 3600,
                finalization_deadline_unix_seconds=now + 7200,
                max_temporary_bytes=2 * 1024 ** 3)


def abort_gate(report, store, source, bound):
    assert report['kind'] == 'same-individual-initialization-aborted-v1'
    for key in ('initialization_aborted', 'cleanup_completed', 'already_published_current'):
        assert report[key] is True, key
    for key in ('canonical_current_modified', 'new_request_allocation_committed', 'neural_execution'):
        assert report[key] is False, key
    assert report['initialization_binding_verified'] is bound
    assert type(report['dispatched_sessions']) is int and report['dispatched_sessions'] == 0
    assert type(report['seeds_consumed']) is int and report['seeds_consumed'] == 0
    assert report['candidate_sha256'] == digest(source)
    assert Path(report['candidate']).resolve(strict=True) == store / 'current.alife-checkpoint'
    assert report['temporary_budget'] is None
    assert (store / 'current.alife-checkpoint').read_bytes() == source
    assert not (store / '.run').exists()


def main():
    trainer, inspector, fixture, terminal_fixture = [Path(p).resolve(strict=True) for p in sys.argv[1:]]
    validate = host.validator(inspector)
    original, retired = fixture.read_bytes(), terminal_fixture.read_bytes()
    original_hashes = [digest(original), digest(retired)]
    admitted = host.tool(inspector, '--inspect-current', fixture)
    # Exercise the real auto-seed boundary with actual compiled-inspector JSON.
    # Planning a next nonce must not reserve/consume it or enrich the report
    # from the raw checkpoint payload (which hid the missing-field regression).
    admitted_before = deepcopy(admitted)
    assert admitted['seeds']['pending_range'] is None
    known_ranges = admitted['seeds']['excluded_ranges']
    assert admitted['seeds']['excluded_range_count'] == len(known_ranges)
    first_unknown = max((last for _, last in known_ranges), default=0) + 1
    assert continuous.admitted_seed_range(admitted) == [first_unknown, first_unknown]
    assert admitted == admitted_before
    terminal = host.tool(inspector, '--inspect-current', terminal_fixture)
    assert terminal['terminal_disposition']['kind'] == 'observed-terminal-retained-brain-v1'
    seed = admitted['seed'] + 1000
    r = request(seed)
    with tempfile.TemporaryDirectory(prefix='alife-continuous-native-cpu-') as directory:
        root = Path(directory).resolve(strict=True)
        for bound in (False, True):
            store = root / ('bound' if bound else 'torn')
            store.mkdir()
            (store / 'current.alife-checkpoint').write_bytes(original)
            owner = dict(schema=1, kind='native-resident-continuous-run-v1', store=str(store),
                         pid=1, baseline_sha256=digest(original), request_sha256=digest(packed(r)), request=r)
            marker = store / ('resident-initialization.bin' if bound else 'resident-initialization.next')
            marker.write_bytes(envelope(b'ALIFE-CON-INI-1\n', owner) if bound else b'ALIF')
            abort_gate(host.tool(trainer, '--recover-resident', store), store, original, bound)
            assert not marker.exists()

        store = root / 'one-session'
        store.mkdir()
        current = store / 'current.alife-checkpoint'
        current.write_bytes(original)
        run = store / '.run'
        run.mkdir()
        owner = dict(schema=1, kind='native-resident-continuous-run-v1', store=str(store), pid=1,
                     baseline_sha256=digest(original), request_sha256=digest(packed(r)), request=r)
        (run / 'run.json').write_bytes(packed(owner))
        (run / 'baseline.alife-checkpoint').write_bytes(original)
        base = json.loads(original[56:])
        seeds = deepcopy(base['seeds'])
        assert seeds['pending_range'] is None
        assert not any(a <= seed + 2 and seed <= b for a, b in seeds['excluded_ranges'])
        seeds['excluded_ranges'].append([seed, seed + 2])
        seeds['excluded_ranges'].sort()
        seeds['pending_range'] = [seed + 1, seed + 2]
        pins = dict(schema=1, baseline_sha256=owner['baseline_sha256'], request_sha256=owner['request_sha256'])
        (run / 'seed-wal.bin').write_bytes(envelope(b'ALIFE-CON-WAL-1\n', dict(pins, consumed=True, seeds=seeds)))
        (run / 'continuous-frontier.bin').write_bytes(envelope(b'ALIFE-CON-MAN-1\n', dict(pins, snapshot=None, previous_snapshot=None)))
        report = host.tool(trainer, '--recover-resident', store)
        assert report['kind'] == 'same-individual-recovery-v1'
        assert report['dispatched_sessions'] == report['seeds_consumed'] == 1
        assert report['retained_session_trained_rows'] == report['retained_session_updates'] == 0
        assert report['terminal_event'] is None and report['producer_result'] is None
        candidate = Path(report['candidate']).resolve(strict=True)
        assert digest(candidate.read_bytes()) == report['candidate_sha256']
        with checkpoints.Store(store, validate) as publisher:
            publisher.publish(candidate, digest(current.read_bytes()))
        after = host.tool(inspector, '--inspect-current', current)
        assert after['seeds']['excluded_range_count'] == len(after['seeds']['excluded_ranges'])
        assert after['seeds']['pending_range'] == [seed + 1, seed + 2]
        assert continuous.admitted_seed_range(after) == [seed + 1, seed + 2]
        for key in ('seed', 'policy_version', 'actor_optimizer_step', 'value_optimizer_step', 'asset_sha256'):
            assert after[key] == admitted[key], key
        assert host.tool(trainer, '--recover-resident', store)['already_published_current'] is True
        assert host.tool(trainer, '--finish-resident-run', store)['cleanup_completed'] is True
        assert not run.exists()

        store = root / 'copied-terminal-other-lane'
        store.mkdir()
        current = store / 'current.alife-checkpoint'
        current.write_bytes(retired)
        request_path = root / 'request.json'
        request_path.write_bytes(packed(request(seed + 100)))
        result = subprocess.run([str(trainer), '--resident-continuous', str(store), str(request_path)],
                                capture_output=True, text=True, timeout=90)
        assert result.returncode != 0 and 'terminal' in result.stderr.lower(), result
        assert current.read_bytes() == retired and not (store / '.run').exists()
        assert not (store / 'resident-initialization.bin').exists()
    assert [digest(fixture.read_bytes()), digest(terminal_fixture.read_bytes())] == original_hashes
    print(json.dumps(dict(kind='continuous-native-cpu-contracts-v1', neural_execution=False,
                         initialization_bound_and_unbound=True, one_session_recovery_cleanup=True,
                         native_inspector_auto_seed_next_allocation=True, native_inspector_owned_tail=True,
                         copied_terminal_current_refused=True, source_bytes_unchanged=True)))


if __name__ == '__main__':
    main()
