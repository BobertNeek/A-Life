"""Apply one audited Conifer leader repair to its exact pinned baseline.

Python standard library only. The patch stores original and replacement float32
bytes, avoiding platform-dependent normal recalculation during reproduction.
derive_conifer_leader.py independently reconstructs these fields from the geometric
rule. Neither script invokes the monolithic multi-asset authoring generator.
"""
import argparse
import hashlib
import json
from pathlib import Path

from conifer_glb import Glb

ROOT = Path(__file__).resolve().parents[1]
BASE_SHA256 = 'cd1bddd8aac1c7a0f7d1e3ac71d51ba76d4c87804dca3405074498b550caaf4d'
FIXED_SHA256 = 'e8e8f9058450d9fb3ff854084f98b86f7a6d14b8989449c8f537cf8f1fe88e12'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def repair_bytes(raw, patch_path=ROOT/'scripts/conifer-leader-patch.json'):
    if sha(raw) != BASE_SHA256:
        raise ValueError('Baseline SHA256 mismatch; refuses other inputs or repeat application')
    base = Glb(raw)
    patch = json.loads(Path(patch_path).read_text())
    if patch['baseline_sha256'] != BASE_SHA256 or patch['candidate_sha256'] != FIXED_SHA256:
        raise ValueError('Patch identity mismatch')
    data = bytearray(raw)
    seen = set()
    for row in patch['fields']:
        mesh = base.doc['meshes'][row['mesh_index']]
        if mesh['name'] != row['mesh'] or row['attribute'] not in ('POSITION', 'NORMAL'):
            raise ValueError('Patch target mismatch')
        attr = mesh['primitives'][0]['attributes'][row['attribute']]
        accessor = base.doc['accessors'][attr]
        if attr != row['accessor'] or accessor['type'] != 'VEC3' or accessor['componentType'] != 5126:
            raise ValueError('Patch accessor mismatch')
        offset = base.offset(attr, row['vertex'])
        if offset != row['byte_offset'] or offset in seen:
            raise ValueError('Patch byte address mismatch or duplicate')
        seen.add(offset)
        before, after = bytes.fromhex(row['before_hex']), bytes.fromhex(row['after_hex'])
        if len(before) != 12 or len(after) != 12 or raw[offset:offset+12] != before:
            raise ValueError('Patch original float field mismatch')
        data[offset:offset+12] = after
    if sha(data) != FIXED_SHA256:
        raise ValueError('Candidate SHA256 mismatch; refusing output')
    return bytes(data)


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
