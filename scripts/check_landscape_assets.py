"""Focused export checks for the refined landscape library and highland scene."""
import json
import struct
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'crates/alife_game_app/assets/landscape'
expected=[*(f'Oak_variant_{i}' for i in range(1,4)),
          *(f'Boulder_variant_{i}' for i in range(1,4)),
          'Berry_shrub','Grass_clump','Cliff_outcrop','Fern_patch','Wildflower_patch']
paths=[HERE/(n+'.glb') for n in expected]+[HERE/'highlands/Mountain_Valley.glb']
for path in paths:
    raw=path.read_bytes()
    assert raw[:4]==b'glTF' and struct.unpack_from('<I',raw,8)[0]==len(raw),path
    length,kind=struct.unpack_from('<II',raw,12)
    assert kind==0x4e4f534a,path
    doc=json.loads(raw[20:20+length])
    assert not doc.get('skins') and not doc.get('animations'),path
    assert all('uri' not in row for row in doc.get('buffers',[])),path
    assert all('uri' not in row for row in doc.get('images',[])),path
    triangles=sum(doc['accessors'][p['indices']]['count']//3 for m in doc['meshes'] for p in m['primitives'])
    budget=350000 if path.stem=='Mountain_Valley' else 12000
    assert 0<triangles<budget,(path,triangles)
    if path.stem=='Mountain_Valley':
        assert 'Highlands_continuous_terrain' in {n.get('name') for n in doc['nodes']}
        assert len(doc['nodes'])>1000
        terrain=next(m for m in doc['materials'] if m.get('name')=='Highlands | meadow and strata')
        assert 'normalTexture' in terrain, 'Missing baked terrain relief'
        assert 'baseColorTexture' in terrain['pbrMetallicRoughness'], 'Missing packed ground grain'
        terrain_mesh=next(m for m in doc['meshes'] if m.get('name')=='Highlands_continuous_terrain')
        assert all('COLOR_0' in p['attributes'] for p in terrain_mesh['primitives']), 'Lost painted biomes'
        color=doc['accessors'][terrain_mesh['primitives'][0]['attributes']['COLOR_0']]
        view=doc['bufferViews'][color['bufferView']]
        binary_start=28+length
        offset=binary_start+view.get('byteOffset',0)+color.get('byteOffset',0)
        fmt,divisor={5121:('4B',255),5123:('4H',65535),5126:('4f',1)}[color['componentType']]
        rgba=[v/divisor for v in struct.unpack_from('<'+fmt,raw,offset)]
        assert rgba[1]>rgba[0]*1.1 and rgba[1]<.5, ('Lost green meadow paint',rgba)
    if path.stem in {'Grass_clump','Fern_patch','Wildflower_patch','Cliff_outcrop'}:
        assert all('COLOR_0' in p['attributes'] for m in doc['meshes'] for p in m['primitives']),path
    print(f'PASS {path.name}: {triangles} unique triangles, {len(doc["nodes"])} nodes, {len(raw)} bytes')
