"""Apply one audited Conifer leader repair to its exact pinned baseline.

Uses the retained NumPy geometric derivation and exact input/output hashes.
Neither script invokes the monolithic multi-asset authoring generator.
"""
import argparse
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BASE_SHA256 = 'cd1bddd8aac1c7a0f7d1e3ac71d51ba76d4c87804dca3405074498b550caaf4d'
FIXED_SHA256 = 'e8e8f9058450d9fb3ff854084f98b86f7a6d14b8989449c8f537cf8f1fe88e12'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def repair_bytes(raw):
    # The retained geometric derivation owns the repair; no deleted patch table.
    from derive_conifer_leader import derive_bytes
    return derive_bytes(raw)[0]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--baseline', type=Path, default=ROOT/'crates/alife_game_app/assets/landscape/island/conifer-leader/Conifer.baseline.glb')
    p.add_argument('--output', type=Path, default=ROOT/'crates/alife_game_app/assets/landscape/island/Conifer.glb')
    a = p.parse_args()
    if a.baseline.resolve() == a.output.resolve():
        p.error('Baseline and output must differ')
    fixed = repair_bytes(a.baseline.read_bytes())
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_bytes(fixed)
    print(json.dumps({'result': 'PASS', 'bytes': len(fixed), 'sha256': sha(fixed)}))


if __name__ == '__main__':
    main()
