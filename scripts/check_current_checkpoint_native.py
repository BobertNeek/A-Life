#!/usr/bin/env python3
"""Exercise actual native admission/publication with the disposable Rust fixture.

No training, no GPU, no legacy/PC cleanup. The input is never changed or removed.
"""
import json
from pathlib import Path
import shutil
import sys
import tempfile

import current_checkpoint_store as cp
import run_current_care as runner


def main():
    binary, fixture = map(lambda p: Path(p).resolve(strict=True), sys.argv[1:])
    validate = runner.validator(binary)
    source = validate(fixture)
    with tempfile.TemporaryDirectory(prefix='alife-current-native-') as temporary:
        root = Path(temporary)
        store_root = root / 'store'
        store_root.mkdir()
        with cp.Store(store_root, validate) as store:
            store.publish(fixture, None)
            allocation = root / 'allocation'
            first = source['seed'] + 100
            runner.tool(binary, '--reserve-current', store.current, first, first + 7, allocation)
            store.publish(allocation, cp.digest(store.current))
        measurements = []
        for seed in range(first, first + 8):
            candidate = root / f'candidate-{seed}'
            with cp.Store(store_root, validate) as store:
                result = runner.tool(binary, '--consume-current', store.current, seed, candidate)
                store.publish(candidate, cp.digest(store.current))
                for key in ('policy_version', 'actor_optimizer_step', 'value_optimizer_step', 'asset_sha256'):
                    assert result[key] == source[key]
                metadata = store.current.stat()
                measurements.append({'logical_bytes': metadata.st_size,
                    'inode_allocated_bytes': getattr(metadata, 'st_blocks', 0) * 512 if hasattr(metadata, 'st_blocks') else None,
                    'store_files': len(list(store_root.iterdir()))})
            candidate.unlink()
        # Just current is enough; neither the source fixture nor any run files
        # are copied into the relocated recovery location.
        relocated = root / 'relocated'
        relocated.mkdir()
        copied = relocated / cp.CURRENT
        shutil.copyfile(store_root / cp.CURRENT, copied)
        shutil.rmtree(store_root)
        report = validate(copied)
        assert report['external_dependencies'] == [] and report['seeds']['pending_range'] is None
        runner.tool(binary, '--extract-current-life', copied, relocated / 'private-life')
        assert (relocated / 'private-life/world.json').is_file()
        assert all(m['store_files'] == 2 for m in measurements)
        print(json.dumps({'kind': 'synthetic-native-publication', 'neural_execution': False,
                          'measurements': measurements, 'relocated_private_extraction': True}))


if __name__ == '__main__':
    main()
