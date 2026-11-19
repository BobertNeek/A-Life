"""Cutout conifer spray texture. Needles follow their bough rather than world UVs.

Two separately seeded profiles occupy the upper image. The bottom is a softer
mass of background twigs, plus a padded untextured bark swatch. Every exposed
needle uses coordinated pigment, roughness and raised relief, not painted sun.
"""

import math
import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter


def bake(folder):
    folder.mkdir(parents=True, exist_ok=True)
    N = 512
    ss = 2
    rng = np.random.default_rng(971935)
    fine = rng.uniform(-1, 1, (N, N)).astype("f")
    grain = gaussian_filter(fine, 1.6) * 6
    base = np.stack(
        (
            219.0 + 5.5 * grain + 2 * fine,
            229.0 + 5.0 * grain + 2 * fine,
            215.0 + 5.2 * grain + 2 * fine,
        ),
        axis=-1,
    )
    col = Image.fromarray(base.clip(0, 255).astype("u1")).resize(
        (N * ss, N * ss), Image.Resampling.BILINEAR
    )
    height = Image.new("L", col.size, 65)
    rough = Image.new("L", col.size, 203)
    co = ImageDraw.Draw(col)
    ht = ImageDraw.Draw(height)
    rr = ImageDraw.Draw(rough)

    def needle(x0, y0, x1, y1, w, pigment, relief):
        dx, dy = x1 - x0, y1 - y0
        length = math.hypot(dx, dy)
        if length < 0.2:
            return
        nx, ny = -dy / length * w, dx / length * w
        # Unequal blade outline and a tapered cusp. It darkens within the
        # twig/interior, not at a fixed 2D screen side.
        verts = [
            (x0, y0),
            (0.70 * x0 + 0.30 * x1 + nx, 0.70 * y0 + 0.30 * y1 + ny),
            (x1, y1),
            (0.58 * x0 + 0.42 * x1 - nx * 0.66, 0.58 * y0 + 0.42 * y1 - ny * 0.66),
        ]
        points = [(round(x * ss), round(y * ss)) for x, y in verts]
        co.polygon(points, fill=tuple(np.clip(pigment, 0, 255).astype("u1")))
        ht.polygon(points, fill=int(relief))
        rr.polygon(points, fill=int(rng.integers(162, 208)))

    from island_needle_sprigs import articulated

    alpha = articulated(col, height, rough, ss)
    for _ in range(4800):
        x = rng.uniform(-9, 515)
        y = rng.uniform(416, 517)
        a = rng.normal(-0.38, 0.56)
        l = rng.uniform(2, 10)
        ca = rng.uniform(203, 244)
        needle(
            x,
            y,
            x + math.cos(a) * l,
            y + math.sin(a) * l,
            rng.uniform(0.3, 0.8),
            np.array([ca - 5, ca + 2, ca - 6]),
            rng.uniform(90, 164),
        )
    col = col.resize((N, N), Image.Resampling.LANCZOS)
    h = np.asarray(height.resize((N, N), Image.Resampling.LANCZOS), dtype="f") / 255
    rough = np.asarray(rough.resize((N, N), Image.Resampling.LANCZOS), dtype="f") / 255
    # The shaders decay smoothly to neutral despite different bough lengths.
    dy, dx = np.gradient(gaussian_filter(h, 0.65))
    n = np.stack((-1.37 * dx, -1.37 * dy, np.ones_like(h)), axis=-1)
    n /= np.linalg.norm(n, axis=-1)[..., None]
    normal = Image.fromarray(np.rint((n + 1) * 127.5).clip(0, 255).astype("u1"))
    orm = Image.fromarray(
        np.rint(255 * np.stack((0.90 + 0.10 * h, rough, np.zeros_like(h)), axis=-1))
        .clip(0, 255)
        .astype("u1")
    )
    # Original solid stem samples this safe island; keep bark vertex pigment
    # and the original surface normal entirely unchanged.
    for im, color in (
        (col, (255, 255, 255)),
        (normal, (128, 128, 255)),
        (orm, (255, 209, 0)),
    ):
        ImageDraw.Draw(im).rectangle((0, 461, 47, 511), fill=color)
    alpha = alpha.resize((N, N), Image.Resampling.LANCZOS)
    col.putalpha(alpha)
    col.save(folder / "needles.png", compress_level=6)
    normal.save(folder / "needles-normal.png", compress_level=6)
    orm.resize((256, 256), Image.Resampling.BOX).save(
        folder / "needles-orm.png", compress_level=6
    )
    from island_habitat_massing import linear

    px = np.asarray(col, dtype=float)[12:404]
    return linear(px[..., :3][px[..., 3] > 130] / 255).mean(axis=0)


def uv(t, cross, alternate=False):
    start = 215 if alternate else 13
    return (0.018 + 0.961 * t, (start + 8 + (cross * 0.5 + 0.5) * 172) / 512)
