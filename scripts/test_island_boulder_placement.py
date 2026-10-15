"""Narrow geometry and authoring regressions, no Blender or GPU required."""
import copy
import json
import struct
import unittest

from check_island_art import Glb
from check_island_boulder_placement import HERE, ROOT, check, minimum_clearance, transform
from island_placement_corrections import CORRECTIONS, corrected_height


class PlacementTests(unittest.TestCase):
    def test_shipping_support_lods_have_shallow_support_and_matching_proxies(self):
        self.assertEqual(check()['result'], 'PASS')

    def test_original_elevation_reproduces_geometric_air_gap(self):
        props = json.loads((ROOT / HERE / 'props.json').read_text())
        raw = (ROOT / 'crates/alife_world/assets/island-v1.bin').read_bytes()
        for index, correction in CORRECTIONS.items():
            p = copy.deepcopy(props[index])
            p['position'][1] = correction['original_y']
            kind = correction['kind']
            glb = Glb(ROOT / HERE / (kind + '.glb'))
            for level in range(3):
                with self.subTest(index=index, lod=level):
                    mesh = next(m for m in glb.doc['meshes'] if m['name'] == f'{kind}_L{level}')
                    primitive = mesh['primitives'][0]
                    vertices = transform(glb.accessor(primitive['attributes']['POSITION']), p)
                    indices = [row[0] for row in glb.accessor(primitive['indices'])]
                    clearance, _ = minimum_clearance(vertices, indices, raw)
                    self.assertGreater(clearance, .08)
                    self.assertAlmostEqual(clearance, correction['original_clearance_m'][level], delta=.0001)

    def test_authoring_correction_is_scoped_and_rejects_identity_drift(self):
        props = json.loads((ROOT / HERE / 'props.json').read_text())
        unchanged = copy.deepcopy(props)
        for i, p in enumerate(props):
            x, y, z = p['position']
            p['position'][1] = corrected_height(i, p['kind'], x, z, y)
        self.assertEqual(props, unchanged)
        for index, c in CORRECTIONS.items():
            with self.subTest(index=index):
                x, z = c['xz']
                self.assertEqual(corrected_height(index, c['kind'], x, z, c['original_y']), c['corrected_y'])
                for kind, xx, zz, y in [('Cliff', x, z, c['original_y']),
                                        (c['kind'], x + 1, z, c['original_y']),
                                        (c['kind'], x, z, c['original_y'] + 1)]:
                    with self.assertRaises(AssertionError):
                        corrected_height(index, kind, xx, zz, y)

    def test_only_five_reviewed_instances_are_corrected(self):
        self.assertEqual(set(CORRECTIONS), {1319, 1325, 1364, 1541, 1730})
        self.assertEqual(CORRECTIONS[1319]['corrected_y'], 154.78496851454123)

    def test_hidden_and_tiny_candidates_are_left_unchanged(self):
        props = json.loads((ROOT / HERE / 'props.json').read_text())
        for index in (1659, 1620, 1543, 1674):
            with self.subTest(index=index):
                p = props[index]
                x, y, z = p['position']
                self.assertNotIn(index, CORRECTIONS)
                self.assertEqual(corrected_height(index, p['kind'], x, z, y), y)

    def test_overlap_clipping_finds_contact_between_original_vertices(self):
        # A peak in the cell diagonal intersects the middle of a flat rock face;
        # checking only that face's three original vertices would miss it.
        raw = b'HLAND001' + struct.pack('<IIfffI', 2, 2, 0., 0., 1., 0)
        raw += struct.pack('<4f', 0., 0., 0., 2.)
        triangle = [(0., .8, 1.), (1., .8, 0.), (0., .8, 0.)]
        clearance, contact = minimum_clearance(triangle, [0, 1, 2], raw)
        self.assertAlmostEqual(clearance, -.2)
        self.assertAlmostEqual(contact[1], 1.)


if __name__ == '__main__':
    unittest.main()
