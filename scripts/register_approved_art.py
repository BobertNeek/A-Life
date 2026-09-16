"""Refresh production manifest receipts after exporting approved Blender art."""
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
manifest = ROOT / 'crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json'
data = json.loads(manifest.read_text())
data['entries'] = [e for e in data['entries'] if not e['asset_id'].startswith('approved-')]
paths = [ROOT / 'crates/alife_game_app/assets/creatures/hearthling/hearthling.glb']
paths += sorted((ROOT / 'crates/alife_game_app/assets/landscape').glob('*.glb'))
paths += [ROOT / 'crates/alife_game_app/assets/landscape/ground-detail.png']
for path in paths:
    payload = path.read_bytes()
    digest = 0xcbf29ce484222325
    for byte in payload:
        digest = ((digest ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    entry = dict(data['entries'][0])
    entry.update(asset_id='approved-' + path.stem.lower().replace('_', '-'),
        author='A-Life approved Blender art', digest=f'fnv1a64:{digest:016x}',
        size_bytes=len(payload), local_path=path.relative_to(ROOT).as_posix(),
        usage_category='creatures' if 'hearthling' in path.name else 'environment-dressing',
        source='generated:approved-blender-model',
        replacement_policy='edit-blender-source-and-refresh-manifest',
        generator=dict(config_path=('scripts/export_hearthling.py' if 'hearthling' in path.name else
            'crates/alife_game_app/assets/landscape/landscape.blend'), date='2026-09-07',
            seed='approved-creatures-modern-black-and-white', tool='Blender-5.1'))
    if path.suffix == '.png':
        entry.update(author='A-Life generated art with OpenAI image generation', source='generated:ground-detail-prompt')
        entry['generator'].update(config_path='crates/alife_game_app/assets/landscape/ground-detail-prompt.txt',
            tool='OpenAI-image-generation')
    data['entries'].append(entry)
manifest.write_text(json.dumps(data, indent=2) + '\n')
print('Registered', len(paths), 'approved production assets')
