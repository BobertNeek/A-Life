"""Small, dependency-free GLB reader for the exact uncompressed fixture."""
import json
import struct
from pathlib import Path


class Glb:
    def __init__(self, source):
        self.raw = source if isinstance(source, (bytes, bytearray)) else Path(source).read_bytes()
        if len(self.raw) < 20:
            raise ValueError('Truncated GLB')
        magic, version, length = struct.unpack_from('<III', self.raw)
        if magic != 0x46546C67 or version != 2 or length != len(self.raw):
            raise ValueError('Expected a complete GLB 2 file')
        offset, chunks = 12, []
        while offset < length:
            size, kind = struct.unpack_from('<II', self.raw, offset)
            start, end = offset + 8, offset + 8 + size
            if size % 4 or end > length:
                raise ValueError('Invalid GLB chunk')
            chunks.append((kind, start, end))
            offset = end
        if [c[0] for c in chunks] != [0x4E4F534A, 0x004E4942]:
            raise ValueError('Expected exactly JSON and BIN chunks')
        self.doc = json.loads(self.raw[chunks[0][1]:chunks[0][2]])
        self.binary, self.binary_end = chunks[1][1:]

    def layout(self, index):
        a = self.doc['accessors'][index]
        v = self.doc['bufferViews'][a['bufferView']]
        if v.get('buffer', 0) != 0 or 'sparse' in a:
            raise ValueError('Unsupported accessor storage')
        n = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[a['type']]
        fmt = '<' + {5126: 'f', 5125: 'I', 5123: 'H', 5121: 'B'}[a['componentType']] * n
        width = struct.calcsize(fmt)
        stride = v.get('byteStride', width)
        offset = self.binary + v.get('byteOffset', 0) + a.get('byteOffset', 0)
        if stride < width or (a['count'] and offset + (a['count']-1)*stride + width > self.binary_end):
            raise ValueError('Invalid accessor span')
        return offset, stride, fmt, a['count']

    def rows(self, index):
        offset, stride, fmt, count = self.layout(index)
        return [struct.unpack_from(fmt, self.raw, offset + i*stride) for i in range(count)]

    def offset(self, accessor, vertex):
        offset, stride, _, count = self.layout(accessor)
        if not 0 <= vertex < count:
            raise ValueError('Vertex outside accessor')
        return offset + vertex*stride
