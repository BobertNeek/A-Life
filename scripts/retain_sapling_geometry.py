"""Transfer Blender-exported colors into the untouched committed GLB layout.

Blender 4.3's round-trip can perturb split normals and create extra near-LOD
vertices. A color-only override has no reason to ship those changes. Pair the
exact same triangle corners, require exact positions, and copy only normalized
RGB bytes into the baseline COLOR_0 accessor. Everything else is byte-retained.
"""
import hashlib
import struct
from collections import defaultdict
from pathlib import Path
from conifer_glb import Glb


def byte_offsets(glb, index):
    accessor = glb.doc['accessors'][index]
    assert accessor['componentType'] == 5121 and accessor.get('normalized')
    offset, stride, _, count = glb.layout(index)
    return [offset + i * stride for i in range(count)]


def retain_geometry(baseline, native_export, output):
    old, new = Glb(baseline), Glb(native_export)
    assert len(old.doc['meshes']) == len(new.doc['meshes']), 'LOD mesh count changed'
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
        # Pin the per-triangle normals from an unchanged 4.3.2 import/export.
        # The original smooth LOD0 has a measured .0005272 normal round-trip
        # error; the shipping GLB still retains its original normals bytewise.
        calibrated = {
            'Sapling_L0': 'a208c9daed002895633fc68256ac91987501904f008dfb9c5f7979160f71f5cc',
            'Sapling_L1': 'd95668b8ea66b48015a1811ef184f2d61f2df25108dee61f24392d9b3344fe0d',
            'Sapling_L2': 'edc165e65d2aec404db2ce904319291f70b83067d6280417f766a0d37a08683e',
        }
        normal_corners = b''.join(struct.pack('<fff', *new_norm[j]) for j in new_indices)
        assert hashlib.sha256(normal_corners).hexdigest() == calibrated[previous['name']], (
            'Source normals differ from the unchanged Blender 4.3.2 roundtrip', previous['name'])
        mapping = defaultdict(set)
        for i, j in zip(old_indices, new_indices):
            mapping[i].add(colors[j][:3])
        assert set(mapping) == set(range(len(old_pos)))
        assert all(len(options) == 1 for options in mapping.values()), 'Color seam ambiguity'
        offsets = byte_offsets(old, p['attributes']['COLOR_0'])
        for index, values in mapping.items():
            data[offsets[index]:offsets[index] + 3] = bytes(next(iter(values)))
        report.append(dict(mesh=previous['name'], triangles=len(old_indices) // 3,
                           shipped_vertices=len(old_pos), source_export_vertices=len(new_pos),
                           source_export_normal_roundtrip_error=normal_error,
                           shipped_position_normal_topology_changes=0))
    Path(output).write_bytes(data)
    return report
