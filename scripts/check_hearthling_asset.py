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
coat=next(m for m in gltf['materials'] if m['name']=='Hearthling | vertex-colored coat')
normal=gltf['textures'][coat['normalTexture']['index']]
assert gltf['images'][normal['source']]['mimeType']=='image/png'
assert 'bufferView' in gltf['images'][normal['source']], 'Fur normal must be embedded'
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
    rotating_bones = {gltf['nodes'][c['target']['node']].get('name')
                      for c in animation['channels'] if c['target']['path'] == 'rotation'}
    assert {'lid_upper.L', 'lid_lower.L', 'lid_upper.R', 'lid_lower.R'} <= rotating_bones
triangles = sum(gltf['accessors'][p['indices']]['count'] // 3
                for m in gltf['meshes'] for p in m['primitives'])
assert triangles <= 32768, triangles
assert len(gltf['meshes']) == 1, 'Run optimize_hearthling_runtime.py after the character export'
assert sum(len(m['primitives']) for m in gltf['meshes']) <= 6, 'Shared-material draw budget exceeded'
def colors(primitive):
    a=gltf['accessors'][primitive['attributes']['COLOR_0']];v=gltf['bufferViews'][a['bufferView']]
    fmt,div={5126:('4f',1),5123:('4H',65535),5121:('4B',255)}[a['componentType']]
    offset=28+length+v.get('byteOffset',0)+a.get('byteOffset',0)
    stride=v.get('byteStride',struct.calcsize('<'+fmt))
    return [tuple(x/div for x in struct.unpack_from('<'+fmt,data,offset+i*stride)) for i in range(a['count'])]
for p in gltf['meshes'][0]['primitives']:
    for texture in [gltf['materials'][p['material']].get('normalTexture')]:
        if texture:
            uv=texture.get('texCoord',0)
            assert uv>=0 and f'TEXCOORD_{uv}' in p['attributes'], 'Invalid texture coordinate binding'
    name=gltf['materials'][p['material']]['name']
    if name=='Hearthling | spherical eyes':
        values=colors(p)
        assert any(g>r*1.2 and g>b*1.2 for r,g,b,a in values), 'Iris green was lost'
        assert any(max(r,g,b)<0.05 for r,g,b,a in values), 'Dark pupils were lost'
    if name=='Hearthling | vertex-colored coat':
        assert any(r>g*1.2 and r>b*1.2 for r,g,b,a in colors(p)), 'Coat paint was lost'
assert any('JOINTS_0' in p['attributes'] for m in gltf['meshes'] for p in m['primitives'])
print(f'PASS: {path.name}: one skin, {len(gltf["meshes"])} meshes, {triangles} triangles, {sorted(names)}')
