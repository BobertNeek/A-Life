"""Refresh only the approved hand entry after a compatible validated export."""
import json
from pathlib import Path
from check_wizard_hand import check

ROOT=Path(__file__).resolve().parents[1]
directory=ROOT/'crates/alife_game_app/assets/hand'
spec=json.loads((directory/'hand-specification.json').read_text())
assert spec['blender'].startswith('5.2.'), 'Export with compatible Blender 5.2 before registering'
check(require_production=False)
path=directory/'wizard-god-hand.glb';payload=path.read_bytes();digest=0xcbf29ce484222325
for byte in payload:digest=((digest^byte)*0x100000001b3)&0xffffffffffffffff
manifest=ROOT/'crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json'
data=json.loads(manifest.read_text());entries=[e for e in data['entries'] if e['asset_id']=='approved-wizard-god-hand']
assert len(entries)==1
entry=entries[0]
assert entry['local_path']==path.relative_to(ROOT).as_posix()
entry.update(digest=f'fnv1a64:{digest:016x}',size_bytes=len(payload))
entry['generator'].update(date='2026-10-03',seed='approved-right-hand-fuller-relaxed-rounded-back',tool='Blender-'+spec['blender'])
manifest.write_text(json.dumps(data,indent=2)+'\n')
check()
print('Registered only approved-wizard-god-hand')
