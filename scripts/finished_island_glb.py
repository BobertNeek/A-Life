"""Deterministic, uncompressed GLB access for the six saved island refinements."""
import json
import math
import struct
from pathlib import Path

FORMATS = {5120: 'b', 5121: 'B', 5122: 'h', 5123: 'H', 5125: 'I', 5126: 'f'}
WIDTHS = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}


def parse(path):
    raw = Path(path).read_bytes()
    assert struct.unpack_from('<III', raw) == (0x46546c67, 2, len(raw)), 'Invalid GLB header'
    offset, document, binary = 12, None, None
    while offset < len(raw):
        size, kind = struct.unpack_from('<II', raw, offset)
        offset += 8
        assert size % 4 == 0 and offset + size <= len(raw), 'Invalid GLB chunk'
        chunk = raw[offset:offset + size]
        offset += size
        if kind == 0x4e4f534a:
            assert document is None
            document = json.loads(chunk)
        elif kind == 0x004e4942:
            assert binary is None
            binary = bytearray(chunk)
        else:
            raise AssertionError('Unexpected GLB chunk')
    assert document is not None and binary is not None
    return document, binary


def layout(document, index):
    accessor = document['accessors'][index]
    assert not accessor.get('sparse')
    view = document['bufferViews'][accessor['bufferView']]
    assert view.get('buffer', 0) == 0
    fmt = '<' + FORMATS[accessor['componentType']] * WIDTHS[accessor['type']]
    size = struct.calcsize(fmt)
    stride = view.get('byteStride', size)
    assert stride >= size
    offset = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
    return accessor, fmt, offset, stride, size


def values(document, binary, index, normalize=True):
    accessor, fmt, offset, stride, size = layout(document, index)
    assert offset + (accessor['count'] - 1) * stride + size <= len(binary)
    rows = [struct.unpack_from(fmt, binary, offset + i * stride)
            for i in range(accessor['count'])]
    if normalize and accessor.get('normalized'):
        factor = {5121: 255, 5123: 65535}[accessor['componentType']]
        rows = [tuple(value / factor for value in row) for row in rows]
    return rows


def payload(document, binary, index):
    accessor, _, offset, stride, size = layout(document, index)
    return b''.join(bytes(binary[offset + i * stride:offset + i * stride + size])
                    for i in range(accessor['count']))


def set_values(document, binary, index, rows):
    accessor, fmt, offset, stride, size = layout(document, index)
    assert len(rows) == accessor['count']
    for i, row in enumerate(rows):
        binary[offset + i * stride:offset + i * stride + size] = struct.pack(fmt, *row)


def save(document, binary, path):
    metadata = json.dumps(document, separators=(',', ':')).encode()
    metadata += b' ' * (-len(metadata) % 4)
    binary = bytes(binary) + b'\0' * (-len(binary) % 4)
    raw = struct.pack('<III', 0x46546c67, 2, 28 + len(metadata) + len(binary))
    raw += struct.pack('<II', len(metadata), 0x4e4f534a) + metadata
    raw += struct.pack('<II', len(binary), 0x004e4942) + binary
    Path(path).write_bytes(raw)


def transplant(baseline_path, native_path, output_path, *, color_only=False):
    """Use the saved source-pack layout, without rerunning a sculpt/paint pass."""
    old, binary = parse(baseline_path)
    native, native_binary = parse(native_path)
    for key in ('nodes', 'materials', 'scenes', 'scene'):
        assert old.get(key) == native.get(key), ('Native identity changed', key)
    assert len(old['meshes']) == len(native['meshes']) == 3
    original = bytes(binary)
    allowed = set()
    for old_mesh, native_mesh in zip(old['meshes'], native['meshes']):
        assert old_mesh['name'] == native_mesh['name']
        assert len(old_mesh['primitives']) == len(native_mesh['primitives']) == 1
        previous, current = old_mesh['primitives'][0], native_mesh['primitives'][0]
        assert previous['material'] == current['material'] == 0
        assert set(previous['attributes']) == set(current['attributes']) == {'POSITION', 'NORMAL', 'COLOR_0'}
        assert payload(old, binary, previous['indices']) == payload(native, native_binary, current['indices']), 'Index order changed'
        for semantic in ('POSITION', 'NORMAL', 'COLOR_0'):
            oi, ni = previous['attributes'][semantic], current['attributes'][semantic]
            a, b = old['accessors'][oi], native['accessors'][ni]
            assert a['count'] == b['count'], ('Vertex count changed', semantic)
            rows = values(native, native_binary, ni)
            assert all(math.isfinite(value) for row in rows for value in row)
            if semantic == 'POSITION':
                assert a['min'] == b['min'] and a['max'] == b['max'], 'Bounds metadata changed'
                assert [min(row[k] for row in rows) for k in range(3)] == a['min']
                assert [max(row[k] for row in rows) for k in range(3)] == a['max']
            if color_only and semantic != 'COLOR_0':
                if semantic == 'POSITION':
                    assert rows == values(old, binary, oi), 'Color-only source moved geometry'
                continue
            if semantic == 'COLOR_0':
                assert a['componentType'] == 5121 and a.get('normalized') and a['type'] == 'VEC3'
                assert all(0 <= value <= 1 for row in rows for value in row)
                assert all(len(row) == 3 or row[3] == 1 for row in rows), 'Unexpected source alpha'
                rows = [tuple(round(value * 255) for value in row[:3]) for row in rows]
            set_values(old, binary, oi, rows)
            accessor, _, offset, stride, size = layout(old, oi)
            for i in range(accessor['count']):
                allowed.update(range(offset + i * stride, offset + i * stride + size))
    assert len(original) == len(binary)
    assert all(a == b for i, (a, b) in enumerate(zip(original, binary)) if i not in allowed)
    save(old, binary, output_path)
    return {'only_color_bytes': color_only, 'layout_and_indices_retained': True,
            'changed_binary_bytes': sum(a != b for a, b in zip(original, binary))}
