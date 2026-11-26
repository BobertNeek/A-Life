"""Verify and reconstruct the exact evaluator source closure in a new directory."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ANCHOR = 'c5d250178cb70a10f99f78ebcc974e0ad5533b41'
PACKAGE = Path(__file__).resolve().parent

def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args])

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('checkout', type=Path)
    ap.add_argument('output', type=Path)
    args = ap.parse_args()
    repo, output = args.checkout.resolve(), args.output.resolve()
    assert git(repo, 'rev-parse', ANCHOR).decode().strip() == ANCHOR
    entries = json.loads((PACKAGE / 'SOURCE.json').read_bytes())
    assert len(entries) == 827 and len({p for p, _, _ in entries}) == 827
    tracked = set(git(repo, 'ls-tree', '-r', '--name-only', '-z', ANCHOR).decode().split('\0'))
    output.mkdir(parents=True, exist_ok=False)
    for path, _, _ in entries:
        if path in tracked:
            dest = output / path
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(git(repo, 'show', f'{ANCHOR}:{path}'))
    patches = sorted((PACKAGE / 'patches').rglob('*.patch'))
    assert len(patches) == 69
    subprocess.run(['git', 'apply', '--check', *map(str, patches)], cwd=output, check=True)
    subprocess.run(['git', 'apply', *map(str, patches)], cwd=output, check=True)
    assert {p.relative_to(output).as_posix() for p in output.rglob('*') if p.is_file()} == {p for p, _, _ in entries}
    for path, size, digest in entries:
        raw = (output / path).read_bytes()
        if len(raw) != size or hashlib.sha256(raw).hexdigest() != digest:
            raise RuntimeError(f'exact-source mismatch: {path}')
    print(f'{len(entries)} exact source files verified')

if __name__ == '__main__':
    main()
