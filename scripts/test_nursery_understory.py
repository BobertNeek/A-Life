"""Check the shipped Blender export, not a parallel reconstruction of its geometry."""

import io
import json
import struct
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


class UnderstoryExport(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        file = (
            ROOT / "crates/alife_game_app/assets/care/nursery/understory.glb"
        ).read_bytes()
        n = struct.unpack_from("<I", file, 12)[0]
        cls.doc = json.loads(file[20 : 20 + n])
        cls.bin = file[28 + n :]
        cls.island = (ROOT / "crates/alife_world/assets/island-v1.bin").read_bytes()

    def test_floor_matches_each_near_terrain_vertex_without_covering_the_trail(self):
        w, _, ox, oz, step, _ = struct.unpack_from("<IIfffI", self.island, 8)

        def island_height(i, j):
            return struct.unpack_from("<f", self.island, 32 + 4 * (j * w + i))[0]

        center = island_height(round((81.25 - ox) / step), round((250.0 - oz) / step))
        floor = next(m for m in self.doc["meshes"] if m["name"] == "Patchwork earth")
        p = floor["primitives"][0]
        pos = self.doc["accessors"][p["attributes"]["POSITION"]]
        view = self.doc["bufferViews"][pos["bufferView"]]
        start = view.get("byteOffset", 0) + pos.get("byteOffset", 0)
        points = set()
        for a in range(pos["count"]):
            x, y, z = struct.unpack_from(
                "<3f", self.bin, start + a * view.get("byteStride", 12)
            )
            i, j = (x + 81.25 - ox) / step, (z + 250.0 - oz) / step
            self.assertAlmostEqual(i, round(i), places=5)
            self.assertAlmostEqual(j, round(j), places=5)
            lift = center + y - island_height(round(i), round(j))
            self.assertGreater(lift, 0.002)
            self.assertLess(lift, 0.006)  # Observed footfalls sit at 0.011 m.
            points.add((x, z))
        self.assertEqual(len(points), 81)
        self.assertEqual(self.doc["accessors"][p["indices"]]["count"], 8 * 8 * 6)

    def test_small_fixed_render_budget_and_fully_transparent_texture_edges(self):
        meshes = self.doc["meshes"]
        self.assertEqual(len(meshes), 3)
        self.assertTrue(all(len(m["primitives"]) == 1 for m in meshes))
        tris = sum(
            self.doc["accessors"][mesh["primitives"][0]["indices"]]["count"] // 3
            for mesh in meshes
        )
        self.assertLess(tris, 16000)
        self.assertNotIn("animations", self.doc)
        self.assertNotIn("skins", self.doc)
        self.assertFalse(any("uri" in i for i in self.doc["images"]))
        texture = self.doc["bufferViews"][self.doc["images"][0]["bufferView"]]
        start = texture.get("byteOffset", 0)
        im = Image.open(
            io.BytesIO(self.bin[start : start + texture["byteLength"]])
        ).convert("RGBA")
        alpha = im.getchannel("A")
        coverage = alpha.getbbox()
        self.assertIsNotNone(coverage)
        a, b, c, d = coverage
        self.assertGreater(min(a, b, im.width - c, im.height - d), 5)
        self.assertGreater(alpha.getpixel((im.width // 2, im.height // 2)), 100)
        self.assertLess(alpha.getextrema()[1], 240)


if __name__ == "__main__":
    unittest.main()
