"""Store linear vertex colors as normalized bytes; preserve all geometry exactly."""
import json
import struct
from pathlib import Path


def compact_colors(path):
    raw = Path(path).read_bytes()
    size = struct.unpack_from('<I', raw, 12)[0]
    doc = json.loads(raw[20:20+size])
    binary = raw[28+size:]
    colors = {p['attributes']['COLOR_0'] for m in doc.get('meshes', [])
              for p in m['primitives'] if 'COLOR_0' in p['attributes']}
    replacements = {}
    for index in colors:
        accessor = doc['accessors'][index]
        if accessor['componentType'] == 5121:
            continue
        view = doc['bufferViews'][accessor['bufferView']]
        assert accessor['componentType'] == 5123 and accessor.get('normalized')
        assert accessor.get('byteOffset', 0) == 0 and 'byteStride' not in view
        components = {'VEC3': 3, 'VEC4': 4}[accessor['type']]
        count = accessor['count'] * components
        assert view['byteLength'] == count * 2
        values = struct.unpack_from('<'+'H'*count, binary, view.get('byteOffset', 0))
        replacements[accessor['bufferView']] = bytes(round(v/257) for v in values)
        accessor['componentType'] = 5121
    if not replacements:
        return
    packed = bytearray()
    for index, view in enumerate(doc['bufferViews']):
        packed.extend(b'\0' * (-len(packed) % 4))
        start = view.get('byteOffset', 0)
        payload = replacements.get(index, binary[start:start+view['byteLength']])
        view['byteOffset'] = len(packed)
        view['byteLength'] = len(payload)
        packed.extend(payload)
    doc['buffers'][0]['byteLength'] = len(packed)
    packed.extend(b'\0' * (-len(packed) % 4))
    metadata = json.dumps(doc, separators=(',', ':')).encode()
    metadata += b' ' * (-len(metadata) % 4)
    output = struct.pack('<III', 0x46546c67, 2, 28+len(metadata)+len(packed))
    output += struct.pack('<II', len(metadata), 0x4e4f534a)+metadata
    output += struct.pack('<II', len(packed), 0x004e4942)+packed
    Path(path).write_bytes(output)


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[1]/'crates/alife_game_app/assets'
    for path in [*(root/'landscape/island').glob('*.glb'), root/'hand/wizard-god-hand.glb']:
        compact_colors(path)
