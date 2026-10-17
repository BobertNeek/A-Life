"""Bounded CPU-only native reload/export-order rehearsal; requires Blender 4.3.2.

Run only in a disposable checkout: historical exports temporarily replace the
two runtime assets. No authoring, render, GPU, frontend build or install occurs.
The saved current assets are restored if an export or audit fails.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
import check_fern_coastal_composition as check


def run(blender, output):
    root = check.ROOT
    output.mkdir(parents=True, exist_ok=True)
    check.check()
    executable = shutil.which(blender)
    assert executable, 'Blender unavailable; no installation attempted'
    version = subprocess.check_output([executable, '--version'], text=True, timeout=30).splitlines()[0]
    assert version == 'Blender 4.3.2', ('Exact reviewed Blender required', version)
    names = check.RUNTIMES | {check.MANIFEST.as_posix(), check.SPECIFICATION.as_posix(),
                              (check.HERE / 'BerryBush.glb').as_posix()}
    saved = {name: (root / name).read_bytes() for name in names}
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
    records = []

    def command(label, args):
        result = subprocess.run(args, cwd=root, capture_output=True, text=True, env=env, timeout=120)
        (output / (label + '.log')).write_text(result.stdout + result.stderr)
        records.append({'step': label, 'exit_code': result.returncode})
        assert result.returncode == 0, ('Export step failed', label, result.stderr[-2000:])

    def export(label, script):
        command(label, [executable, '--background', '--threads', '2', '--factory-startup',
                        '--python-exit-code', '1', '--python', 'scripts/' + script])

    try:
        export('01-historical-fern', 'build_fern_readability.py')
        export('02-historical-shrubs', 'build_island_biological_shrubs.py')
        historical = {name: check.coastal.sha((root / name).read_bytes()) for name in sorted(check.RUNTIMES)}
        assert historical[check.fern.RUNTIME] != check.FERN_RUNTIME_SHA256
        assert historical[check.coastal.RUNTIME] != check.coastal.RUNTIME_SHA256
        export('03-current-fern', 'build_fern_fronds.py')
        export('04-current-coastal', 'build_coastal_leaf_sprays.py')
        command('05-register', [sys.executable, '-S', 'scripts/register_approved_art.py'])
        first = check.check()
        assert all((root / name).read_bytes() == raw for name, raw in saved.items()), 'First order changed final bytes'
        export('06-reverse-current-coastal', 'build_coastal_leaf_sprays.py')
        export('07-reverse-current-fern', 'build_fern_fronds.py')
        command('08-register', [sys.executable, '-S', 'scripts/register_approved_art.py'])
        second = check.check()
        assert all((root / name).read_bytes() == raw for name, raw in saved.items()), 'Reverse order changed final bytes'
        return {'result': 'PASS', 'blender': version, 'steps': records,
                'combined_profile_sha256': check.coastal.sha((root / check.PROFILE).read_bytes()),
                'historical_exporters_overwrite_current_assets': True,
                'current_exporters_restore_both_approved_assets': True,
                'current_exporter_orders_byte_identical': True,
                'native_sources_and_unrelated_art_byte_retained': True,
                'audited_art_files_after_each_order': [first['art_blobs_audited'], second['art_blobs_audited']],
                'runtime_sha256': {name: check.coastal.sha((root / name).read_bytes()) for name in sorted(check.RUNTIMES)},
                'rendered': False, 'physical_gpu': False, 'remote_writes': False}
    except BaseException:
        for name, raw in saved.items():
            (root / name).write_bytes(raw)
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--blender', default='blender')
    parser.add_argument('--output-dir', type=Path, default=check.ROOT / 'target/artifacts/fern-coastal-export-order')
    args = parser.parse_args()
    result = run(args.blender, args.output_dir)
    text = json.dumps(result, indent=2) + '\n'
    (args.output_dir / 'receipt.json').write_text(text)
    print(text, end='')
