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
    # Replacing a whole view is safe only when it belongs to this accessor.
    # glTF permits multiple accessors (or images) to share the same bytes.
    # Reject unsupported layouts before changing metadata or writing the file.
    for index in colors:
        accessor = doc['accessors'][index]
        if accessor['componentType'] == 5121:
            continue
        if 'sparse' in accessor:
            raise ValueError(f'COLOR_0 accessor {index} uses unsupported sparse storage')
        other_attribute = any(
            value == index and semantic != 'COLOR_0'
            for mesh in doc.get('meshes', [])
            for primitive in mesh['primitives']
            for attributes in [primitive.get('attributes', {}),
                               *primitive.get('targets', [])]
            for semantic, value in attributes.items())
        if other_attribute:
            raise ValueError(f'COLOR_0 accessor {index} also serves another attribute')
        view_index = accessor['bufferView']
        aliases = [i for i, other in enumerate(doc['accessors'])
                   if i != index and other.get('bufferView') == view_index]
        sparse_alias = any(
            part.get('bufferView') == view_index
            for other in doc['accessors']
            for part in (other.get('sparse', {}).get('indices', {}),
                         other.get('sparse', {}).get('values', {})))
        image_alias = any(image.get('bufferView') == view_index
                          for image in doc.get('images', []))
        if aliases or sparse_alias or image_alias:
            raise ValueError(f'COLOR_0 accessor {index} shares bufferView {view_index}')
    replacements = {}
    for index in colors:
        accessor = doc['accessors'][index]
        if accessor['componentType'] == 5121:
            continue
        view = doc['bufferViews'][accessor['bufferView']]
        assert accessor.get('byteOffset', 0) == 0 and 'byteStride' not in view
        components = {'VEC3': 3, 'VEC4': 4}[accessor['type']]
        count = accessor['count'] * components
        if accessor['componentType'] == 5126:
            # Blender 4.3 emits linear float colors; newer exporters use UNORM16.
            assert view['byteLength'] == count * 4
            values = struct.unpack_from('<'+'f'*count, binary, view.get('byteOffset', 0))
            assert all(0 <= value <= 1 for value in values)
            replacements[accessor['bufferView']] = bytes(round(v*255) for v in values)
            accessor['normalized'] = True
        else:
            assert accessor['componentType'] == 5123 and accessor.get('normalized')
            assert view['byteLength'] == count * 2
            values = struct.unpack_from('<'+'H'*count, binary, view.get('byteOffset', 0))
            replacements[accessor['bufferView']] = bytes(round(v/257) for v in values)
        if components == 3:
            # Vertex attributes need a four-byte element stride, including RGB8.
            rgb = replacements[accessor['bufferView']]
            replacements[accessor['bufferView']] = b''.join(
                rgb[start:start+3] + b'\0' for start in range(0, len(rgb), 3))
            view['byteStride'] = 4
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
