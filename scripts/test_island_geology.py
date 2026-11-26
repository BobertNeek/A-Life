"""Regression gates for the shipping island: real triangle heights, shoreline and UVs."""

import argparse
from collections import Counter
import io
from pathlib import Path
import struct
import unittest

import numpy as np
from PIL import Image

from build_island_geology import Glb, ROOT, SURFACE, TERRAIN


def field():
    blob = SURFACE.read_bytes()
    w, d, ox, oz, spacing, _ = struct.unpack_from("<IIfffI", blob, 8)
    return np.frombuffer(blob, "<f4", w * d, 32).reshape(d, w), ox, oz, spacing


def measure(path, level=2):
    """Raster the actual far assets onto every full-resolution height sample.

    This tests the shipped indexed triangles, not the baker's error estimate.
    The projected area of a skirt triangle is zero, so it never stands in for land.
    """
    glb = Glb(path)
    h, ox, oz, spacing = field()
    far = np.full_like(h, np.nan)
    tri_count = 0
    for mesh in glb.doc["meshes"]:
        if not mesh["name"].endswith(f"_L{level}"):
            continue
        p = mesh["primitives"][0]
        vertices = glb.attr(p["attributes"]["POSITION"]).astype(np.float64)
        x = (vertices[:, 0] - ox) / spacing
        z = (vertices[:, 2] - oz) / spacing
        y = vertices[:, 1]
        triangles = glb.attr(p["indices"]).reshape(-1, 3)
        tri_count += len(triangles)
        a, b, c = triangles.T
        det = (z[b] - z[c]) * (x[a] - x[c]) + (x[c] - x[b]) * (z[a] - z[c])
        for k in np.nonzero(det)[0]:
            ids = triangles[k]
            xx, zz, yy = x[ids], z[ids], y[ids]
            lo, hi = xx.min().astype(int), xx.max().astype(int)
            bottom, top = zz.min().astype(int), zz.max().astype(int)
            px, pz = np.mgrid[lo : hi + 1, bottom : top + 1]
            l0 = (
                (zz[1] - zz[2]) * (px - xx[2]) + (xx[2] - xx[1]) * (pz - zz[2])
            ) / det[k]
            l1 = (
                (zz[2] - zz[0]) * (px - xx[2]) + (xx[0] - xx[2]) * (pz - zz[2])
            ) / det[k]
            l2 = 1 - l0 - l1
            hit = (l0 >= -1e-7) & (l1 >= -1e-7) & (l2 >= -1e-7)
            far[pz[hit], px[hit]] = (l0 * yy[0] + l1 * yy[1] + l2 * yy[2])[hit]
    if not np.isfinite(far).all():
        raise AssertionError("Overview surface contains holes")
    err = abs(far - h)
    near_sea = abs(h) <= 2
    return dict(
        triangles=tri_count,
        wrong_side_of_sea=int(((far >= 0) != (h >= 0)).sum()),
        worst_height_error_m=float(err.max()),
        mean_height_error_m=float(err.mean()),
        coast_rmse_m=float(np.sqrt(np.mean((far[near_sea] - h[near_sea]) ** 2))),
    )


class ShippingCoast(unittest.TestCase):
    def test_contact_geometry_and_materials_across_all_lods(self):
        g = Glb(TERRAIN)
        h, ox, oz, spacing = field()
        self.assertEqual(len(g.doc["materials"]), 2)
        macro, close = g.doc["materials"]
        self.assertEqual(
            macro["pbrMetallicRoughness"]["baseColorTexture"], {"index": 1}
        )
        self.assertEqual(close["normalTexture"], {"index": 4, "texCoord": 1})
        self.assertEqual(
            close["pbrMetallicRoughness"]["baseColorTexture"],
            {"index": 3, "texCoord": 1},
        )
        self.assertEqual(
            close["pbrMetallicRoughness"]["metallicRoughnessTexture"],
            {"index": 5, "texCoord": 1},
        )

        def linear(c):
            return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)

        art = TERRAIN.parent / "geology"
        world_rgb = np.asarray(Image.open(art / "pigment.png")) / 255
        close_mean = linear(
            np.asarray(Image.open(art / "close-pigment.png")) / 255
        ).mean(axis=(0, 1))
        by_node = {}
        self.assertEqual(len(g.doc["meshes"]), 360)
        counts = Counter()
        edge = {}
        for m in g.doc["meshes"]:
            _, cx, cz, level = m["name"].split("_")
            lod = int(level[-1])
            self.assertEqual(len(m["primitives"]), 1)
            p = m["primitives"][0]
            self.assertEqual(p["material"], int(lod == 0))
            pos = g.attr(p["attributes"]["POSITION"])
            norm = g.attr(p["attributes"]["NORMAL"])
            uv = g.attr(p["attributes"]["TEXCOORD_0"])
            x, z = np.rint((pos[:, 0] - ox) / spacing).astype(int), np.rint(
                (pos[:, 2] - oz) / spacing
            ).astype(int)
            source = h[z, x]
            surface = abs(source - pos[:, 1]) < 1e-4
            self.assertTrue((surface | (abs(source - pos[:, 1] - 20) < 1e-4)).all())
            np.testing.assert_allclose(
                uv,
                np.column_stack((x / (h.shape[1] - 1), z / (h.shape[0] - 1))),
                atol=1e-6,
            )
            np.testing.assert_allclose(np.linalg.norm(norm, axis=1), 1, atol=2e-4)
            if lod == 0:
                vcol = p["attributes"]["COLOR_0"]
                self.assertTrue(g.doc["accessors"][vcol]["normalized"])
                color = g.attr(vcol)
                # At a fully mip-filtered (distant) contact vertex the new tile
                # must return to its macro pigment instead of darkening the snow.
                v, u = np.minimum(
                    (uv[:, ::-1] * world_rgb.shape[:2]).astype(int),
                    np.array(world_rgb.shape[:2]) - 1,
                ).T
                np.testing.assert_allclose(
                    color[:, :3] / 255 * close_mean, linear(world_rgb[v, u]), atol=0.002
                )
                fine = g.attr(p["attributes"]["TEXCOORD_1"])
                self.assertTrue((color[:, 3] == 255).all())
                # Both sides of a chunk boundary must see the same pigment and pore.
                border = surface & (((x % 32) == 0) | ((z % 32) == 0))
                for k in np.where(border)[0]:
                    key = (int(x[k]), int(z[k]))
                    val = tuple(color[k]) + tuple(fine[k])
                    self.assertEqual(by_node.setdefault(key, val), val)
                self.assertTrue(np.isfinite(fine).all())
            indices = g.attr(p["indices"])
            self.assertLess(int(indices.max()), len(pos))
            counts[lod] += indices.size // 3
            if lod > 0:
                # Every neighbouring far chunk must agree point-for-point along
                # the visible rim, not merely cover a T-junction with a skirt.
                i, j = int(cx), int(cz)
                for axis, line, along, side in [("x", x, z, i), ("z", z, x, j)]:
                    for which in (0, 1):
                        mask = surface & (line == (side + which) * 32)
                        key = (lod, axis, side + which, j if axis == "x" else i)
                        knots = set(zip(along[mask].tolist(), pos[mask, 1].tolist()))
                        if key in edge:
                            self.assertEqual(knots, edge[key])
                        edge[key] = knots
        self.assertEqual(counts[0], 276480)
        self.assertLess(counts[1], 128000)
        self.assertLess(counts[2], 46000)
        total_size = TERRAIN.stat().st_size
        self.assertEqual(len(g.doc["images"]), 6)
        for i in g.doc["images"]:
            if "uri" in i:
                self.assertIn(
                    i["uri"],
                    {
                        "geology/relief.png",
                        "geology/pigment.png",
                        "geology/close-normal.png",
                        "geology/close-pigment.png",
                        "geology/close-roughness.png",
                    },
                )
                data = (TERRAIN.parent / i["uri"]).read_bytes()
                total_size += len(data)
            else:
                data = g.view(i["bufferView"])
            with Image.open(io.BytesIO(data)) as im:
                im.load()
                self.assertLessEqual(max(im.size), 2048)
        self.assertLess(total_size, 17 * 1024 * 1024)

    def test_coarse_shore_remains_close_to_the_real_waterline(self):
        middle = measure(TERRAIN, 1)
        self.assertLess(middle["worst_height_error_m"], 1.8)
        self.assertLess(middle["coast_rmse_m"], 0.12)
        result = measure(TERRAIN)
        self.assertLess(result["coast_rmse_m"], 0.25)
        self.assertLess(result["wrong_side_of_sea"], 110)
        self.assertLess(result["mean_height_error_m"], 0.20)


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--measure", type=Path)
    cli.add_argument("--level", type=int, choices=(0, 1, 2), default=2)
    args, rest = cli.parse_known_args()
    if args.measure:
        print(measure(args.measure, args.level))
    else:
        unittest.main(argv=["island-geology", *rest])
