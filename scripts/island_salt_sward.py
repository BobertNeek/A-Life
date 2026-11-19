"""Static bent sheaves from real anchored short ribbons, not camera-facing planes.

Each transparent cutout is a painted stand of grass, from deeply shaded bases
to amber spikes. The geometry orders them in irregular arcs with exposed sand
channels; roots track the actual terrain at each corner.
"""

import io
import math

import numpy as np
from PIL import Image, ImageDraw


def brush():
    """Four crowns. Empty transparent colour is green to guard filtered fringes."""
    scale = 2
    w, h = 256 * scale, 192 * scale
    atlas = Image.new("RGBA", (4 * w, h), (118, 143, 90, 0))
    for q in range(4):
        r = np.random.default_rng(582341 + q * 1949)
        im = Image.new("RGBA", (w, h), (118, 143, 90, 0))
        pen = ImageDraw.Draw(im)
        for front in range(3):
            # Unequal inner rosettes break the long flat lower profile; let
            # real ground show through even at the centre of a dense colony.
            centers = np.array((0.17, 0.44, 0.76)) + r.uniform(-0.055, 0.055, 3)
            for j in range(78 if front == 0 else 63):
                x = np.clip(r.choice(centers) + r.normal(0, 0.066), 0.028, 0.967) * w
                crown = (0.18 + 0.82 * math.sin(math.pi * x / w) ** 0.66) * (
                    0.74
                    + 0.19 * math.sin(x / w * 8 + q)
                    + 0.09 * math.sin(x / w * 21 - q)
                )
                height = h * crown * r.uniform(0.44, 1.02) * (1 - front * 0.12)
                base = h * (1 + r.uniform(0.01, 0.09))
                lean = r.uniform(-0.18, 0.31) * height + 0.10 * height
                breadth = r.uniform(0.006, 0.014) * w
                rgb = r.choice(
                    [
                        (109, 139, 80),
                        (150, 160, 91),
                        (130, 151, 91),
                        (174, 164, 96),
                        (125, 146, 77),
                        (164, 171, 104),
                    ]
                )
                if q == 2:
                    rgb = rgb * np.array([1.07, 0.98, 0.89])
                if q == 3:
                    rgb = rgb * np.array([0.90, 1.01, 1.04])
                levels = (0, 0.32, 0.64, 0.83, 1)
                rows = []
                for t in levels:
                    cx = x + lean * t**1.72
                    cy = base - height * t
                    half = (
                        breadth
                        * (0.18 + 0.82 * math.sin(math.pi * t) ** 0.66)
                        * (1 - t * 0.58)
                    )
                    rows.append([(cx - half, cy), (cx + half, cy)])
                for n, (a, b) in enumerate(zip(rows[:-1], rows[1:])):
                    shade = 0.78 + 0.058 * n + front * 0.031 + r.uniform(-0.025, 0.025)
                    col = tuple(int(np.clip(k * shade, 0, 255)) for k in rgb) + (255,)
                    pen.polygon([*a, b[1], b[0]], fill=col)
                if front == 0 and j % 8 == 0 and q != 3:
                    a, b = rows[-1]
                    xx = (a[0] + b[0]) / 2
                    yy = a[1]
                    spike = (184, 165, 100, 255) if q != 2 else (196, 154, 129, 255)
                    for i in range(5):
                        sx = xx + i * height * 0.008
                        sy = yy - i * height * 0.019
                        pen.ellipse(
                            (
                                sx - 1.7 * scale,
                                sy - 2.6 * scale,
                                sx + 1.7 * scale,
                                sy + 2.6 * scale,
                            ),
                            fill=spike,
                        )
        atlas.paste(im, (q * w, 0))
    atlas = atlas.resize((1024, 192), Image.Resampling.LANCZOS)
    out = io.BytesIO()
    atlas.save(out, "PNG", optimize=True)
    return out.getvalue()


def ribbon(p, r, x, z, angle, length, height, variant):
    # Three curved/upright rows and three lateral sections keep the surface
    # from repeatedly exposing a perfectly straight transparent side wall.
    c, s = math.cos(angle), math.sin(angle)
    arc = r.uniform(-0.48, 0.48) * length
    lean = r.uniform(0.13, 0.31) * height
    rows = []
    tint = r.uniform(0.92, 1.0)
    for t in (0, 0.40, 1):
        row = []
        for i in range(4):
            along = i / 3.0
            across = (along - 0.5) * length
            bend = arc * math.sin(math.pi * along) + lean * t**1.55
            xx = x + c * across - s * bend
            zz = z + s * across + c * bend
            yy = t * height + 0.001
            shade = tint * (0.86 + 0.14 * t)
            index = p.put(xx, zz, yy, [round(255 * shade)] * 3 + [255])
            assert index * 2 == len(p.uv)
            p.uv.extend(((variant + 0.013 + 0.974 * along) / 4, 0.994 - 0.985 * t))
            row.append(index)
        rows.append(row)
    for a, b in zip(rows[:-1], rows[1:]):
        for i in range(3):
            p.fan(a[i], a[i + 1], b[i + 1])
            p.fan(a[i], b[i + 1], b[i])


def hummock(beach, patch, r, px, pz, heading, extent, heath):
    beds = 0
    # Windward sheets are shorter. Longer sheltered fronds overlap in
    # pairs at different azimuths; no persistent camera rotation/sorting.
    c, s = math.cos(heading), math.sin(heading)
    for i in range(14 if not heath else 8):
        t = r.uniform(-1, 1)
        along = t * extent * 1.45
        cross = 0.42 * t * t * extent + r.uniform(-0.80, 0.80) * (1 - 0.44 * abs(t))
        x = px + c * along - s * cross
        z = pz + s * along + c * cross
        h, dx, dz = beach.surface(x, z)
        if h < 2.65 or h > 30 or math.hypot(dx, dz) > 0.48:
            continue
        size = r.uniform(0.64, 1.16) * (1 - 0.25 * abs(t))
        p = patch(x, z, 5)
        for k in range(2):
            ribbon(
                p,
                r,
                x + 0.04 * k,
                z - 0.06 * k,
                heading + r.uniform(-0.25, 0.25) + k * r.uniform(0.77, 1.29),
                size * r.uniform(0.85, 1.35),
                size * r.uniform(0.21, 0.44),
                int(r.choice((0, 1, 1, 2) if heath else (0, 1, 2, 3))),
            )
        beds += 1
    return beds
