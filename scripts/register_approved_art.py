"""Refresh approved art receipts without replacing provenance or unrelated entries."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
manifest = ROOT / 'crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json'
data = json.loads(manifest.read_text())
# The island is the only playable map. Retain the reusable Highlands prop library.
retired = {'approved-terrain-chunks', 'approved-props'}
data['entries'] = [e for e in data['entries'] if e['asset_id'] not in retired]
entries = {e['local_path']: e for e in data['entries']}
paths = [ROOT / 'crates/alife_game_app/assets/creatures/hearthling/hearthling.glb']
paths += sorted((ROOT / 'crates/alife_game_app/assets/landscape').glob('*.glb'))
paths += [ROOT / 'crates/alife_game_app/assets/landscape/ground-detail.png']
paths += sorted((ROOT / 'crates/alife_game_app/assets/landscape/island').glob('*.glb'))
paths += [ROOT / 'crates/alife_game_app/assets/landscape/island/props.json']
paths += [ROOT / 'crates/alife_game_app/assets/hand/wizard-god-hand.glb']
for path in paths:
    local_path = path.relative_to(ROOT).as_posix()
    entry = entries[local_path]
    if not entry['asset_id'].startswith('approved-'):
        raise ValueError(f'Expected an approved registration for {local_path}')
    payload = path.read_bytes()
    # PortableAssetDigest canonicalizes text across Windows/Git line endings.
    digest_payload = payload.replace(b'\r\n', b'\n').replace(b'\r', b'\n') if path.suffix == '.json' else payload
    digest = 0xcbf29ce484222325
    for byte in digest_payload:
        digest = ((digest ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    entry.update(digest=f'fnv1a64:{digest:016x}', size_bytes=len(payload))
manifest.write_text(json.dumps(data, indent=2) + '\n')
print('Refreshed', len(paths), 'approved production assets; preserved registration metadata')
