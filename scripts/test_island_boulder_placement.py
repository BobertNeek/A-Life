"""Narrow geometry and authoring regressions, no Blender or GPU required."""
import copy
import json
import struct
import unittest

from check_island_art import Glb
from check_island_boulder_placement import HERE, ROOT, check, minimum_clearance, transform
from island_placement_corrections import (
    CORRECTED_Y, KIND, ORIGINAL_Y, PLACEMENT_INDEX, XZ, corrected_height,
)


class PlacementTests(unittest.TestCase):
    def test_shipping_lods_have_shallow_support_and_matching_proxy(self):
        self.assertEqual(check()['result'], 'PASS')

    def test_original_elevation_reproduces_geometric_air_gap(self):
        p = json.loads((ROOT / HERE / 'props.json').read_text())[PLACEMENT_INDEX]
        p['position'][1] = ORIGINAL_Y
        raw = (ROOT / 'crates/alife_world/assets/island-v1.bin').read_bytes()
        glb = Glb(ROOT / HERE / (KIND + '.glb'))
        for level in range(3):
            mesh = next(m for m in glb.doc['meshes'] if m['name'] == f'{KIND}_L{level}')
            primitive = mesh['primitives'][0]
            vertices = transform(glb.accessor(primitive['attributes']['POSITION']), p)
            indices = [row[0] for row in glb.accessor(primitive['indices'])]
            clearance, _ = minimum_clearance(vertices, indices, raw)
            self.assertGreater(clearance, .87, (level, clearance))

    def test_authoring_correction_is_scoped_and_rejects_identity_drift(self):
        props = json.loads((ROOT / HERE / 'props.json').read_text())
        unchanged = copy.deepcopy(props)
        for i, p in enumerate(props):
            x, y, z = p['position']
            p['position'][1] = corrected_height(i, p['kind'], x, z, y)
        self.assertEqual(props, unchanged)
        self.assertEqual(corrected_height(PLACEMENT_INDEX, KIND, *XZ, ORIGINAL_Y), CORRECTED_Y)
        for kind, x, z, y in [('Cliff', *XZ, ORIGINAL_Y), (KIND, XZ[0] + 1, XZ[1], ORIGINAL_Y),
                              (KIND, *XZ, ORIGINAL_Y + 1)]:
            with self.assertRaises(AssertionError):
                corrected_height(PLACEMENT_INDEX, kind, x, z, y)

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
