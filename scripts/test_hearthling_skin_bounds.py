"""CPU-only containment regression for the shipping Hearthling's local culling box.

Rotations (including procedural gaze/expression/outcome rotations) are unrestricted:
only their norm-preserving property matters. Translation and scale interpolation
must remain LINEAR/STEP. Inherited scale maxima mirror hearthling.rs; update this
fixture if that presentation behavior changes. This is not a frame-rate benchmark.
"""
import json
import math
from pathlib import Path
import re
import struct
import unittest

ROOT = Path(__file__).resolve().parents[1]
INHERITED_SCALE_MAX = {
    'head': 1.06, 'ear.L': 1.15, 'ear.R': 1.15, 'tail.00': 1.10,
    'hand.L': 1.07, 'hand.R': 1.07,
}


def skin_radius(payload):
    size, kind = struct.unpack_from('<I4s', payload, 12)
    assert kind == b'JSON'
    document = json.loads(payload[20:20 + size])
    binary_size, kind = struct.unpack_from('<I4s', payload, 20 + size)
    assert kind == b'BIN\x00'
    binary = payload[28 + size:28 + size + binary_size]

    def accessor(index):
        item = document['accessors'][index]
        assert not item.get('sparse') and not item.get('normalized')
        view = document['bufferViews'][item['bufferView']]
        assert view.get('buffer', 0) == 0
        component = {5126: 'f', 5123: 'H', 5121: 'B'}[item['componentType']]
        width = {'SCALAR': 1, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[item['type']]
        fmt = '<' + component * width
        stride = view.get('byteStride', struct.calcsize(fmt))
        offset = view.get('byteOffset', 0) + item.get('byteOffset', 0)
        return [struct.unpack_from(fmt, binary, offset + row * stride)
                for row in range(item['count'])]

    norm = lambda values: math.sqrt(sum(value * value for value in values))
    nodes = document['nodes']
    assert all('matrix' not in node for node in nodes)
    parents = {}
    for parent, node in enumerate(nodes):
        for child in node.get('children', []):
            assert child not in parents
            parents[child] = parent
    translations = [norm(node.get('translation', [0, 0, 0])) for node in nodes]
    scales = [max(map(abs, node.get('scale', [1, 1, 1]))) for node in nodes]
    for animation in document['animations']:
        # Runtime multipliers are reapplied after animation. Without a sampled
        # scale track they could accumulate, invalidating any fixed envelope.
        scaled_names = {nodes[channel['target']['node']].get('name')
                        for channel in animation['channels']
                        if channel['target']['path'] == 'scale'}
        assert INHERITED_SCALE_MAX.keys() <= scaled_names
        for channel in animation['channels']:
            sampler = animation['samplers'][channel['sampler']]
            assert sampler.get('interpolation', 'LINEAR') in ('LINEAR', 'STEP')
            node, path = channel['target']['node'], channel['target']['path']
            assert path in ('translation', 'rotation', 'scale')
            values = accessor(sampler['output'])
            assert all(math.isfinite(value) for row in values for value in row)
            if path == 'translation':
                translations[node] = max(translations[node], *(norm(row) for row in values))
            elif path == 'scale':
                scales[node] = max(scales[node], *(max(map(abs, row)) for row in values))
    for index, node in enumerate(nodes):
        scales[index] *= INHERITED_SCALE_MAX.get(node.get('name'), 1)

    largest = 0
    for mesh_node in (node for node in nodes if 'mesh' in node):
        # Bound in primitive-local space, before organism-root transforms. The
        # shipping mesh and its ancestors are identity; reject a changed basis.
        index = nodes.index(mesh_node)
        ancestors = set()
        while True:
            assert index not in ancestors
            ancestors.add(index)
            node = nodes[index]
            assert translations[index] == 0 and scales[index] == 1
            assert node.get('rotation', [0, 0, 0, 1]) == [0, 0, 0, 1]
            if index not in parents:
                break
            index = parents[index]
        skin = document['skins'][mesh_node['skin']]
        inverse_binds = accessor(skin['inverseBindMatrices'])
        for primitive in document['meshes'][mesh_node['mesh']]['primitives']:
            assert not primitive.get('targets')
            attributes = primitive['attributes']
            assert not any(name.startswith(('JOINTS_1', 'WEIGHTS_1')) for name in attributes)
            positions = accessor(attributes['POSITION'])
            weights = accessor(attributes['WEIGHTS_0'])
            joints = accessor(attributes['JOINTS_0'])
            assert len(positions) == len(weights) == len(joints)
            for position, row_weights, row_joints in zip(positions, weights, joints):
                assert all(math.isfinite(w) and w >= 0 for w in row_weights)
                total = sum(row_weights)
                assert total > 0
                radius = 0
                for weight, joint in zip(row_weights, row_joints):
                    matrix = inverse_binds[joint]
                    point = (*position, 1)
                    local = [sum(matrix[column * 4 + row] * point[column]
                                 for column in range(4)) for row in range(3)]
                    bound = norm(local)
                    index = skin['joints'][joint]
                    visited = set()
                    while True:
                        assert index not in visited
                        visited.add(index)
                        bound = translations[index] + scales[index] * bound
                        if index not in parents:
                            break
                        index = parents[index]
                    radius += weight * bound
                # Be conservative for both raw and normalized shader weights.
                largest = max(largest, radius, radius / total)
    assert math.isfinite(largest) and largest > 0
    return largest


class HearthlingSkinBoundsTests(unittest.TestCase):
    def test_shipping_skin_and_inherited_scales_fit_runtime_box(self):
        payload = (ROOT / 'crates/alife_game_app/assets/creatures/hearthling/hearthling.glb').read_bytes()
        source = (ROOT / 'crates/alife_game_app/src/production_voxel_renderer/hearthling.rs').read_text()
        # These small guards detect edits to the mirrored formula fragments,
        # not arbitrary new transform mutations or Rust semantics.
        compact = ''.join(source.split())
        for formula in [
            '0.85+ear*0.30', '0.94+ear*0.12', '0.90+tail*0.20',
            'Vec3::new(0.95,0.94,0.98)', 'Vec3::new(1.07,1.02,1.06)',
            'let ear = f32::from(visual.appearance.ear_muzzle_trait) / 15.0;',
            'let tail = f32::from(visual.appearance.tail_trait) / 15.0;',
        ]:
            self.assertIn(''.join(formula.split()), compact)
        extent = float(re.search(r'const SKINNED_BOUND_HALF_EXTENT: f32 = ([0-9.]+);', source)[1])
        self.assertIn('Vec3::splat(-SKINNED_BOUND_HALF_EXTENT)', compact)
        self.assertIn('Vec3::splat(SKINNED_BOUND_HALF_EXTENT)', compact)
        radius = skin_radius(payload)
        # Reserve numerical headroom rather than relying on a rounded equality.
        self.assertLess(radius + 1e-4, extent)


if __name__ == '__main__':
    unittest.main()
