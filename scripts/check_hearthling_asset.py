"""Check the shipping GLB, including the skin and state-specific animation clips."""
import json
import struct
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / 'crates/alife_game_app/assets/creatures/hearthling/hearthling.glb'
data = path.read_bytes()
assert data[:4] == b'glTF'
length, kind = struct.unpack_from('<II', data, 12)
gltf = json.loads(data[20:20 + length])
assert kind == 0x4E4F534A
assert len(gltf['skins']) == 1
assert {'Hearthling | vertex-colored coat', 'Hearthling | warm ochre',
        'Hearthling | brows and tuft shadows'} <= {m['name'] for m in gltf['materials']}
names = {a['name'] for a in gltf['animations']}
assert {'Hearthling_CuriousIdle', 'Hearthling_Walk', 'Hearthling_Sleep'} <= names, names
for animation in gltf['animations']:
    assert len(animation['channels']) > 20
    assert all(gltf['accessors'][s['input']]['max'][0] > 0 for s in animation['samplers'])
    scaled_bones = {gltf['nodes'][c['target']['node']].get('name')
                    for c in animation['channels'] if c['target']['path'] == 'scale'}
    # The renderer applies inherited proportions after the clip's scale samples.
    # Missing channels would allow those multipliers to accumulate while paused.
    assert {'head', 'ear.L', 'ear.R', 'tail.00'} <= scaled_bones
triangles = sum(gltf['accessors'][p['indices']]['count'] // 3
                for m in gltf['meshes'] for p in m['primitives'])
assert triangles <= 32000, triangles
assert any('JOINTS_0' in p['attributes'] for m in gltf['meshes'] for p in m['primitives'])
print(f'PASS: {path.name}: one skin, {len(gltf["meshes"])} meshes, {sorted(names)}')
