"""Transfer Blender-exported colors into the untouched committed GLB layout.

Blender 4.3's round-trip can perturb split normals and create extra near-LOD
vertices. A color-only override has no reason to ship those changes. Pair the
exact same triangle corners, require exact positions, and copy only normalized
RGB bytes into the baseline COLOR_0 accessor. Everything else is byte-retained.
"""
import json
import struct
from collections import defaultdict
from pathlib import Path


class Glb:
    def __init__(self, path):
        self.raw = Path(path).read_bytes()
        size = struct.unpack_from('<I', self.raw, 12)[0]
        self.doc = json.loads(self.raw[20:20 + size])
        self.binary = 28 + size

    def rows(self, index):
        a = self.doc['accessors'][index]
        v = self.doc['bufferViews'][a['bufferView']]
        n = {'SCALAR': 1, 'VEC3': 3, 'VEC4': 4}[a['type']]
        fmt = '<' + {5126: 'f', 5125: 'I', 5123: 'H', 5121: 'B'}[a['componentType']] * n
        stride = v.get('byteStride', struct.calcsize(fmt))
        offset = self.binary + v.get('byteOffset', 0) + a.get('byteOffset', 0)
        return [struct.unpack_from(fmt, self.raw, offset + i * stride) for i in range(a['count'])]

    def byte_offsets(self, index):
        a = self.doc['accessors'][index]
        v = self.doc['bufferViews'][a['bufferView']]
        assert a['componentType'] == 5121 and a.get('normalized')
        n = {'VEC3': 3, 'VEC4': 4}[a['type']]
        stride = v.get('byteStride', n)
        offset = self.binary + v.get('byteOffset', 0) + a.get('byteOffset', 0)
        return [offset + i * stride for i in range(a['count'])]


def retain_geometry(baseline, native_export, output):
    old, new = Glb(baseline), Glb(native_export)
    assert old.doc['materials'] == new.doc['materials']
    assert old.doc['nodes'] == new.doc['nodes']
    assert not new.doc.get('textures')
    data = bytearray(old.raw)
    report = []
    for previous, current in zip(old.doc['meshes'], new.doc['meshes']):
        assert previous['name'] == current['name']
        assert len(previous['primitives']) == len(current['primitives']) == 1
        p, q = previous['primitives'][0], current['primitives'][0]
        old_indices = [r[0] for r in old.rows(p['indices'])]
        new_indices = [r[0] for r in new.rows(q['indices'])]
        assert len(old_indices) == len(new_indices)
        old_pos, new_pos = old.rows(p['attributes']['POSITION']), new.rows(q['attributes']['POSITION'])
        old_norm, new_norm = old.rows(p['attributes']['NORMAL']), new.rows(q['attributes']['NORMAL'])
        colors = new.rows(q['attributes']['COLOR_0'])
        assert all(old_pos[i] == new_pos[j] for i, j in zip(old_indices, new_indices)), (
            'Source edit altered geometry or triangle order', previous['name'])
        normal_error = max(abs(x - y) for i, j in zip(old_indices, new_indices)
                           for x, y in zip(old_norm[i], new_norm[j]))
        assert normal_error < .0003, ('Unexpected source normal edit', normal_error)
        mapping = defaultdict(set)
        for i, j in zip(old_indices, new_indices):
            mapping[i].add(colors[j][:3])
        assert set(mapping) == set(range(len(old_pos)))
        assert all(len(options) == 1 for options in mapping.values()), 'Color seam ambiguity'
        offsets = old.byte_offsets(p['attributes']['COLOR_0'])
        for index, values in mapping.items():
            data[offsets[index]:offsets[index] + 3] = bytes(next(iter(values)))
        report.append(dict(mesh=previous['name'], triangles=len(old_indices) // 3,
                           shipped_vertices=len(old_pos), source_export_vertices=len(new_pos),
                           source_export_normal_roundtrip_error=normal_error,
                           shipped_position_normal_topology_changes=0))
    Path(output).write_bytes(data)
    return report
