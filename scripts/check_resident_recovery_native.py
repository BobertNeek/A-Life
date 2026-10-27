#!/usr/bin/env python3
"""Consume a disposable Rust stopped-run fixture through the real Native CLIs.

No GPU, world dispatch, training, production checkpoint or legacy cleanup.
The source fixture stays unchanged; only a fresh temporary copy is published.
"""
import json
from pathlib import Path
import shutil
import sys
import tempfile

import current_checkpoint_store as cp
import run_current_care as runner


def main():
    native, inspector, fixture = (Path(p).resolve(strict=True) for p in sys.argv[1:])
    validate = runner.validator(inspector)
    with tempfile.TemporaryDirectory(prefix='alife-resident-native-') as temporary:
        store_root = Path(temporary) / 'store'
        shutil.copytree(fixture, store_root)
        before = validate(store_root / cp.CURRENT)
        old_digest = cp.digest(store_root / cp.CURRENT)
        # runner.tool is the real strict stdout json.loads consumer. A second
        # diagnostic object on stdout must fail here, even after Native exit 0.
        result = runner.tool(native, '--recover-resident', store_root)
        repeated = runner.tool(native, '--recover-resident', store_root)
        for response in (result, repeated):
            budget = response['temporary_budget']
            assert 0 <= budget['used'] <= budget['high_water'] <= budget['limit']
            assert budget['used'] + budget['reserved'] <= budget['limit']
        assert ({k: v for k, v in result.items() if k != 'temporary_budget'} ==
                {k: v for k, v in repeated.items() if k != 'temporary_budget'})
        assert result['already_published_current'] is False
        candidate = Path(result['candidate'])
        report = validate(candidate)
        for key in ('policy_version', 'actor_optimizer_step', 'value_optimizer_step', 'asset_sha256'):
            assert report[key] == before[key]
        first = before['seed'] + 100
        assert result['seeds']['pending_range'] == [first + 2, first + 4]
        assert cp.digest(store_root / cp.CURRENT) == old_digest
        with cp.Store(store_root, validate) as store:
            store.publish(candidate, old_digest)
        accepted = cp.digest(store_root / cp.CURRENT)
        cleaned = runner.tool(native, '--finish-resident-run', store_root)
        assert cleaned['retirement_completed'] is True
        assert not (store_root / cp.RUN).exists()
        assert not (store_root / 'resident-retirement.bin').exists()
        assert not (store_root / 'resident-retirement.next').exists()
        assert cp.digest(store_root / cp.CURRENT) == accepted
        assert runner.tool(native, '--finish-resident-run', store_root)['already_clean'] is True
        print(json.dumps({'kind': 'synthetic-native-resident-recovery', 'strict_single_json': True,
                          'consumed_seeds_preserved': True, 'exact_current_published': True,
                          'bound_retirement_completed': True, 'neural_execution': False}))


if __name__ == '__main__':
    main()
