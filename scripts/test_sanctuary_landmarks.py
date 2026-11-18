"""Check the shipped GLBs, not just the Blender generator's intentions."""

import json
import math
import struct
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates/alife_game_app/assets/care/landmarks"


def read_mesh(name):
    data = (OUT / f"{name}.glb").read_bytes()
    assert data[:4] == b"glTF"
    count = struct.unpack_from("<I", data, 12)[0]
    scene = json.loads(data[20 : 20 + count])
    blob = memoryview(data)[28 + count :]
    (primitive,) = scene["meshes"][0]["primitives"]

    def positions():
        a = scene["accessors"][primitive["attributes"]["POSITION"]]
        view = scene["bufferViews"][a["bufferView"]]
        start = view.get("byteOffset", 0) + a.get("byteOffset", 0)
        stride = view.get("byteStride", 12)
        return [
            struct.unpack_from("<fff", blob, start + i * stride)
            for i in range(a["count"])
        ]

    return scene, primitive, positions()


class PhysicalLandmarkAssets(unittest.TestCase):
    def test_silhouettes_are_closed_to_the_real_body_not_arbitrary_dressing(self):
        heights = {}
        for name in ("waykeeper", "emberstone"):
            scene, primitive, verts = read_mesh(name)
            self.assertEqual(len(scene["meshes"]), 1)
            self.assertEqual(len(scene["meshes"][0]["primitives"]), 1)
            self.assertIn("COLOR_0", primitive["attributes"])
            self.assertIn("NORMAL", primitive["attributes"])
            self.assertLess(
                scene["accessors"][primitive["indices"]]["count"] // 3, 6500
            )
            self.assertFalse(
                scene.get("images"), "These authored forms need no texture streaming"
            )
            radii = [math.hypot(x, z) for x, y, z in verts]
            self.assertLessEqual(max(radii), 1.025)
            self.assertGreater(max(radii), 0.95)
            # A ground-level shoulder in every octant, not an invisible circular collider.
            support = [0.0] * 8
            for x, y, z in verts:
                if -0.02 <= y <= 0.20:
                    i = int((math.atan2(z, x) % math.tau) * 8 / math.tau) % 8
                    support[i] = max(support[i], math.hypot(x, z))
            self.assertGreater(min(support), 0.88)
            self.assertLess(min(y for x, y, z in verts), 0)
            heights[name] = max(y for x, y, z in verts)
        self.assertGreater(heights["waykeeper"], 1.65)
        self.assertLess(heights["emberstone"], 0.82)
        self.assertTrue((OUT / "landmarks.blend").is_file())


if __name__ == "__main__":
    unittest.main()
