"""Clip a few millimetres of surface water to the exact physical ground planes.
The dry strand and shallow swash meet at sea level, never on an inferred outline.
"""

import io
import math
import numpy as np
from PIL import Image

# Outer slopes can be cliffs. Restrict a visible sand film to actual beaches.
WET_MAX = 1.85
WASH_BOTTOM = -1.45


def fade(a, b, t):
    v = max(0, min(1, (t - a) / (b - a)))
    return v * v * (3 - 2 * v)


def clipped(points, level, above):
    result = []
    a = points[-1]
    for b in points:
        inside_a = (a[2] >= level) if above else (a[2] <= level)
        inside_b = (b[2] >= level) if above else (b[2] <= level)
        if inside_a != inside_b:
            t = (level - a[2]) / (b[2] - a[2])
            result.append((a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]), level))
        if inside_b:
            result.append(b)
        a = b
    return result


def wash_lace(size=256):
    """Tileable pinholes and fine walls of retreating foam, not a blurry border."""
    rng = np.random.default_rng(11759)
    nx, nz = 15, 10
    seeds = rng.uniform(0.11, 0.89, size=(nz, nx, 2))
    j, i = np.indices((size, size), dtype=np.float32)
    u = (
        i / size * nx
        + 0.52 * np.sin(j / size * math.tau)
        + 0.17 * np.cos(i / size * math.tau * 3 + j / size * math.tau * 2)
    )
    v = (
        j / size * nz
        + 0.31 * np.sin(i / size * math.tau * 2)
        + 0.18 * np.sin(j / size * math.tau * 3)
    )
    cx, cz = np.floor(u).astype(int), np.floor(v).astype(int)
    closest = np.full_like(u, 10)
    second = closest.copy()
    for dj in (-1, 0, 1):
        for di in (-1, 0, 1):
            cell = seeds[(cz + dj) % nz, (cx + di) % nx]
            ds = (u - cx - di - cell[:, :, 0]) ** 2 + (v - cz - dj - cell[:, :, 1]) ** 2
            win = ds < closest
            second = np.minimum(second, np.where(win, closest, ds))
            closest = np.minimum(closest, ds)
    channels = np.sqrt(second) - np.sqrt(closest)
    local = (
        0.5
        + 0.23 * np.sin(i / size * math.tau * 2 + j / size * math.tau)
        + 0.17 * np.cos(j / size * math.tau * 3 - i / size * math.tau)
    )
    walls = 1 - np.clip((channels - 0.025 - local * 0.06) / (0.10 + local * 0.15), 0, 1)
    film = np.clip(0.18 + 0.79 * walls * (0.35 + 0.65 * local) + 0.14 * local, 0, 1)
    rgba = np.full((size, size, 4), 255, dtype=np.uint8)
    rgba[:, :, 3] = np.rint(255 * film).astype(np.uint8)
    out = io.BytesIO()
    Image.fromarray(rgba, "RGBA").save(out, "PNG")
    return out.getvalue()


def paint(x, z, h, mat, slope, field, tint):
    run = field(x * 0.83 + 18, z * 0.83 + 21)
    broken = field(x * 3.7 - 14, z * 3.7 - 8)
    liquid = math.sin(x * 0.43 + z * 0.34 + 3.1 * broken)
    support = 1 - fade(0.53, 0.91, slope)
    if mat == 2:
        # Darker and smoother below the irregular drying front. This is a
        # translucent film: the actual sand microtexture is visible underneath.
        front = 0.59 + 0.78 * run + 0.11 * liquid
        wet = 1 - fade(0.05, front + 0.40, h)
        edge = 1 - fade(0.10, 0.38, abs(h - front))
        pigment = [
            102 + 18 * broken + 11 * edge,
            101 + 16 * broken + 6 * edge,
            80 + 15 * broken,
        ]
        coverage = 0.83 * wet * support
    else:
        depth = -h
        shallow = 1 - fade(0.25, 1.45, depth)
        # Spent lace families, not an unbroken white ring.
        breaker = 0.09 + 0.19 * run + 0.048 * liquid
        lace = (1 - fade(0.025, 0.15, abs(depth - breaker))) * fade(0.22, 0.78, broken)
        fringe = (1 - fade(0.04, 0.23, abs(depth - 0.58 - 0.17 * liquid))) * fade(
            0.62, 0.89, run
        )
        foam = min(1, lace + 0.35 * fringe) * shallow
        pigment = [
            78 + 132 * foam + 15 * run,
            150 + 77 * foam + 9 * run,
            141 + 68 * foam + 10 * broken,
        ]
        coverage = (0.46 * shallow + 0.52 * foam) * support
    return tint(pigment, 1)[:3] + [round(255 * coverage)]


def lay(beach, patch, field, tint):
    h = beach.h
    a, b, c, d = h[:-1, :-1], h[:-1, 1:], h[1:, 1:], h[1:, :-1]
    lo = np.minimum.reduce([a, b, c, d])
    hi = np.maximum.reduce([a, b, c, d])
    for j, i in zip(*np.where((lo < WET_MAX) & (hi > WASH_BOTTOM)), strict=True):
        x = beach.x0 + i * beach.s
        z = beach.z0 + j * beach.s
        if math.hypot((x + 1.56) / 445, (z + 1.56) / 535) < 0.68:
            continue
        corners = [
            (x, z, float(a[j, i])),
            (x + beach.s, z, float(b[j, i])),
            (x + beach.s, z + beach.s, float(c[j, i])),
            (x, z + beach.s, float(d[j, i])),
        ]
        for indices in [(0, 1, 2), (0, 2, 3)]:
            tri = [corners[k] for k in indices]
            cx = sum(q[0] for q in tri) / 3
            cz = sum(q[1] for q in tri) / 3
            _, dx, dz = beach.surface(cx, cz)
            slope = math.hypot(dx, dz)
            if slope > 0.9:
                continue
            # Contour knots capture the decimetre-scale drying front and foam
            # width without building a uniformly subdivided coastal mesh.
            for mat, levels in [
                (2, (0.0, 0.22, 0.52, 0.87, 1.28, WET_MAX)),
                (3, (WASH_BOTTOM, -0.76, -0.42, -0.25, -0.12, -0.035, 0.0)),
            ]:
                p = patch(cx, cz, mat)
                for low, high in zip(levels[:-1], levels[1:], strict=True):
                    poly = clipped(tri, low, True)
                    if len(poly) < 3:
                        continue
                    poly = clipped(poly, high, False)
                    if len(poly) < 3:
                        continue
                    cols = [
                        paint(xx, zz, hh, mat, slope, field, tint)
                        for xx, zz, hh in poly
                    ]
                    if max(v[3] for v in cols) < 2:
                        continue
                    pts = [
                        p.put_at(xx, zz, (hh + 0.003 if mat == 2 else 0.009), color)
                        for (xx, zz, hh), color in zip(poly, cols, strict=True)
                    ]
                    for k in range(1, len(pts) - 1):
                        p.fan(pts[0], pts[k], pts[k + 1])
