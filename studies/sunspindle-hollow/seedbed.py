"""A continuous seedbed from the actual planting register; no fake shadow planes."""

from pathlib import Path
import math
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
from scipy.ndimage import gaussian_filter


def trace_roots(rows, extent, wh):
    x0, x1, z0, z1 = extent
    w, h = wh
    seed = np.zeros((h, w, 2), "f")
    for kind, x, y, z, s, az in rows:
        i = int((x - x0) / (x1 - x0) * w)
        j = int((z - z0) / (z1 - z0) * h)
        if not (3 <= i < w - 4 and 3 <= j < h - 4):
            continue
        c = int(kind.startswith("copper") or kind.startswith("windwillow"))
        # A shedding canopy deposits material on its lee side, not a black
        # perfectly circular mark at the origin of every stem.
        seed[j, i, c] += s
        seed[j + 2, i + 3, c] += 0.41 * s

    def bands(a):
        young = gaussian_filter(a, (2.2, 3.7)) * 49
        banks = gaussian_filter(a, (15.0, 8.0)) * 570
        broad = gaussian_filter(a, (34.0, 21.0)) * 1260
        return 1 - np.exp(-np.maximum(0, young * 0.71 + banks * 0.51 + broad * 0.38))

    return bands(seed[:, :, 0]), bands(seed[:, :, 1])


def quilt(folder):
    """One seamless 1.78 m plate of split ligules, fines and mica, hand-coloured."""
    R = np.random.default_rng(9629)
    n = 448
    shape = (n, n)
    a = gaussian_filter(R.normal(0, 1, shape), 5.9, mode="wrap")
    b = gaussian_filter(R.normal(0, 1, shape), 1.5, mode="wrap")
    v = 0.67 * a / max(a.std(), 0.0001) + 0.33 * b / max(b.std(), 0.0001)
    trail = gaussian_filter(R.normal(0, 1, shape), (4.3, 1.0), mode="wrap")
    trail /= max(trail.std(), 0.0001)
    grain = R.normal(0, 1, shape)
    v = 10 * v + 4 * trail + 3 * grain
    rgb = np.clip(np.array([201, 195, 177]) + v[..., None], 0, 255).astype("B")
    ht = np.uint8(np.clip(130 + 0.8 * v, 0, 255))
    pigment = Image.fromarray(rgb)
    height = Image.fromarray(ht)
    p = ImageDraw.Draw(pigment)
    h = ImageDraw.Draw(height)

    def drawpoly(xy, colour, rel):
        xy = np.asarray(xy)
        for a in range(-1, 2):
            for b in range(-1, 2):
                q = xy + [a * n, b * n]
                if (
                    np.max(q[:, 0]) < 0
                    or np.max(q[:, 1]) < 0
                    or np.min(q[:, 0]) >= n
                    or np.min(q[:, 1]) >= n
                ):
                    continue
                points = [tuple(pt) for pt in q]
                p.polygon(points, fill=tuple(int(z) for z in colour))
                h.polygon(points, fill=rel)

    for _ in range(390):
        loc = R.uniform(0, n, 2)
        th = R.uniform(0, math.tau)
        d = np.array([math.cos(th), math.sin(th)])
        lat = np.array([-d[1], d[0]])
        f = R.uniform(0.58, 2.3)
        hh = R.uniform(0.48, 1.4)
        c = np.array([156, 146, 125]) if R.random() < 0.64 else [222, 215, 189]
        drawpoly(
            [loc - d * f, loc + lat * hh, loc + d * f, loc - lat * 0.62 * hh],
            c,
            R.integers(87, 155),
        )
    for k in range(121):
        loc = R.uniform(0, n, 2)
        th = R.normal(0.49, 0.60) if R.random() < 0.76 else R.uniform(0, math.tau)
        d = np.array([math.cos(th), math.sin(th)])
        lat = np.array([-d[1], d[0]])
        l = R.uniform(4, 20)
        w = R.uniform(0.52, 1.63)
        c = R.uniform(-0.15, 0.18) * l
        mid = loc + d * 0.56 * l + lat * c
        hue = np.array(
            [160, 149, 120] if R.random() < 0.64 else [218, 202, 157]
        ) + R.uniform(-9, 9)
        drawpoly(
            [loc, mid - lat * w, loc + d * l + lat * c * 0.82, mid + lat * w * 0.62],
            np.clip(hue, 0, 255),
            145,
        )
    pigment.save(folder / "sheath-loam.png", compress_level=7)
    height.save(folder / "sheath-tooth.png", compress_level=7)
