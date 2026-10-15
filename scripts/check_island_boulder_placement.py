"""CPU regression: five reviewed rocks touch real near terrain in their support LODs.

Clip each rock triangle's XZ projection against every overlapping authoritative
terrain triangle. Their vertical separation is linear over each overlap, so its
minimum is at a clipped polygon vertex. This tests the actual support footprint,
including edge/edge contacts, rather than a mesh AABB or a centre-height sample.
"""
import argparse
import itertools
import json
import math
import struct
from pathlib import Path

from check_island_art import Glb
from island_placement_corrections import CORRECTIONS, corrected_height

ROOT = Path(__file__).resolve().parents[1]
HERE = Path('crates/alife_game_app/assets/landscape/island')


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def cross(a, b, c):
    return (b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])


def clip(poly, a, b, orientation):
    result = []
    for p, q in zip(poly, poly[1:] + poly[:1]):
        dp, dq = orientation * cross(a, b, p), orientation * cross(a, b, q)
        if dp >= 0:
            result.append(p)
        if (dp < 0) != (dq < 0):
            t = dp / (dp - dq)
            result.append(tuple(p[k] + t * (q[k] - p[k]) for k in range(3)))
    return result


def transform(vertices, placement):
    x, y, z = map(f32, placement['position'])
    scale, yaw = f32(placement['scale']), f32(placement['yaw'])
    co, si = math.cos(yaw), math.sin(yaw)
    return [(x + scale * (co * a + si * c), y + scale * b,
             z + scale * (-si * a + co * c)) for a, b, c in vertices]


def minimum_clearance(vertices, indices, raw):
    w, d, ox, oz, spacing, _ = struct.unpack_from('<IIfffI', raw, 8)
    heights = struct.unpack_from('<' + 'f' * (w * d), raw, 32)
    lo = [min(v[k] for v in vertices) for k in range(3)]
    hi = [max(v[k] for v in vertices) for k in range(3)]
    terrain = []
    for i in range(math.floor((lo[0] - ox) / spacing), math.ceil((hi[0] - ox) / spacing)):
        for j in range(math.floor((lo[2] - oz) / spacing), math.ceil((hi[2] - oz) / spacing)):
            assert 0 <= i < w - 1 and 0 <= j < d - 1
            a, b, c, dd = [(ox + ii * spacing, heights[jj * w + ii], oz + jj * spacing)
                          for ii, jj in ((i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1))]
            terrain.extend(((a, b, c), (a, c, dd)))
    best = math.inf
    contact = None
    for start in range(0, len(indices), 3):
        triangle = [vertices[index] for index in indices[start:start + 3]]
        if abs(cross(*triangle)) < 1e-12:
            continue  # A vertical face has no projected interior.
        for a, b, c in terrain:
            orientation = 1 if cross(a, b, c) > 0 else -1
            polygon = triangle
            for p, q in ((a, b), (b, c), (c, a)):
                polygon = clip(polygon, p, q, orientation)
            for point in polygon:
                area = cross(a, b, c)
                wa, wb = cross(b, c, point) / area, cross(c, a, point) / area
                ground = wa * a[1] + wb * b[1] + (1 - wa - wb) * c[1]
                clearance = point[1] - ground
                if clearance < best:
                    best, contact = clearance, [point[0], ground, point[2]]
    assert contact is not None
    return best, contact


def check_placement(root, props, raw, index, correction):
    placement = props[index]
    kind = correction['kind']
    proxy_index = correction['proxy_index']
    assert placement['kind'] == kind
    x, y, z = placement['position']
    assert (x, z) == correction['xz'] and y == correction['corrected_y']
    assert corrected_height(index, kind, x, z, y) == y
    w, d, _, _, _, count = struct.unpack_from('<IIfffI', raw, 8)
    solid = lambda p: p['kind'] in ('Broadleaf', 'Conifer', 'Sapling', 'Cliff') or p['kind'].startswith('Boulder')
    assert sum(map(solid, props[:index])) == proxy_index
    assert sum(map(solid, props)) == count
    offset = 32 + w * d * 4 + proxy_index * 24
    proxy = struct.unpack_from('<6f', raw, offset)
    glb = Glb(root / HERE / (kind + '.glb'))
    assert all(not any(k in n for k in ('matrix', 'translation', 'rotation', 'scale')) for n in glb.doc['nodes'])
    rows = []
    for level in range(3):
        mesh = next(m for m in glb.doc['meshes'] if m['name'] == f'{kind}_L{level}')
        assert len(mesh['primitives']) == 1
        primitive = mesh['primitives'][0]
        local = glb.accessor(primitive['attributes']['POSITION'])
        indices = [row[0] for row in glb.accessor(primitive['indices'])]
        vertices = transform(local, placement)
        clearance, contact = minimum_clearance(vertices, indices, raw)
        supported = level in correction['support_lods']
        if supported:
            # A 1 mm allowance covers runtime f32 trigonometry. Keep the
            # independently reviewed per-instance embed budget, never hide a
            # floating LOD by silently widening a global tolerance.
            assert -correction['maximum_embed_m'] < clearance < -.019, (
                'Boulder unsupported or overburied', index, level, clearance)
            assert max(v[1] for v in vertices) > contact[1] + .5, ('Boulder buried', index, level)
        else:
            assert index == 1325 and level == 2
            assert .367 < clearance < .370, ('Approved hidden L2 shape changed', clearance)
        rows.append({'lod': level, 'support_required': supported,
                     'minimum_clearance_m': clearance, 'support_ground_xyz': contact})
        if level == 0:
            bounds = [(min(v[k] for v in local), max(v[k] for v in local)) for k in range(3)]
            corners = transform(list(itertools.product(*bounds)), placement)
            expected = [min(v[0] for v in corners), min(v[2] for v in corners),
                        max(v[0] for v in corners), max(v[2] for v in corners),
                        f32(y), max(v[1] for v in corners)]
            assert max(abs(a - b) for a, b in zip(proxy, expected)) < .0001, (
                'Stale collision proxy', index, proxy, expected)
    return {'placement_index': index, 'kind': kind, 'proxy_index': proxy_index,
            'authored_y_m': y, 'lod_support': rows}


def check(root=ROOT):
    root = Path(root)
    props = json.loads((root / HERE / 'props.json').read_text())
    raw = (root / 'crates/alife_world/assets/island-v1.bin').read_bytes()
    assert raw[:8] == b'HLAND001'
    rows = [check_placement(root, props, raw, index, correction)
            for index, correction in CORRECTIONS.items()]
    return {'result': 'PASS', 'corrected_placements': len(rows),
            'placements': rows, 'game_rendering_proof': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    args = parser.parse_args()
    print(json.dumps(check(args.root), indent=2))
