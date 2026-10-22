#!/usr/bin/env python3
"""Synthetic CPU fixture only: native restore admission before/after storage reuse."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

import training_artifact_store as artifacts


def check(binary):
    binary = str(Path(binary).resolve(strict=True))
    environment = {**os.environ, 'ALIFE_SLM_PRIOR': 'on', 'ALIFE_SLM_PRIOR_BACKEND': 'deterministic'}
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        original, copied = root / 'original', root / 'copied'
        subprocess.run([binary, '--fixture', '--output', str(original), '--seed', '9147',
                        '--lesson', 'eat_held_food', '--decisions', '2'],
                       env=environment, check=True, capture_output=True, text=True)
        receipt = json.loads(subprocess.check_output([binary, '--validate-output', str(original)],
                                                    env=environment, text=True))
        shutil.copytree(original, copied)
        before = {str(p.relative_to(original)): artifacts.digest(p)
                  for p in original.rglob('*') if p.is_file()}
        sealed = artifacts.cpu_sealed_files(receipt, original)
        for checkpoint in (original, copied):
            artifacts.reuse_sealed_files(checkpoint, sealed, root / 'store')
            after = {str(p.relative_to(checkpoint)): artifacts.digest(p)
                     for p in checkpoint.rglob('*') if p.is_file()}
            assert before == after, 'storage reuse changed checkpoint bytes'
            restored = json.loads(subprocess.check_output([binary, '--validate-output', str(checkpoint)],
                                                         env=environment, text=True))
            assert restored == receipt, 'native admission changed after sharing'
        assert all(os.path.samefile(original / name, copied / name) for name in sealed)
        print(f'Native CPU admission and every payload hash preserved across {len(sealed)} shared artifacts.')


if __name__ == '__main__':
    check(sys.argv[1])
