"""A compact closed-mesh spruce bough, from woody attachment to a tapered tip.

The same bough endpoints, droop and crown mass generate all three LODs. The
near version has branched sprays; distant groups keep the asymmetric envelope
and some clear sky, not a stack of horizontal discs or camera-facing cards.
"""

import math
import numpy as np
from island_needle_surface import uv


def unit(v):
    return v / max(1e-9, np.linalg.norm(v))


class Crown:
    def __init__(self):
        self.v = []
        self.c = []
        self.uv = []
        self.f = []

    def vertex(self, p, c, tex=None):
        if tex is None:
            tex = (
                0.15 + 0.76 * np.clip(p[1] / 12, 0, 1),
                0.89 + 0.072 * math.sin(0.87 * p[0] + 0.68 * p[2] + 0.16 * p[1]),
            )
        self.uv.append(tex)
        self.v.append(np.array(p))
        self.c.append(np.array(c))
        return len(self.v) - 1

    def tri(self, a, b, c):
        self.f.append((a, b, c))

    def quad(self, a, b, c, d):
        self.tri(a, b, c)
        self.tri(a, c, d)


def center(profile, y, quarter):
    """A smooth main leader consistent with the untouched lower trunk."""
    wind = np.array(
        (math.cos(quarter * math.pi / 2), 0, math.sin(quarter * math.pi / 2))
    )
    t = np.clip((y - 3.2) / (12 - 3.2), 0, 1)
    q = 0.48 * t * t * (3 - 2 * t) + 0.52 * t**3
    bend = (1.35, 0.72, 0.34)[profile - 1]
    # Keep bark within its exact original proxy through 3.2m.
    k = (0.51, 0.245, 0.0)[profile - 1]
    return wind * (bend * q) + np.array(
        (0.0, y - (12 - 3.2) * k * t * t * (1.5 - 0.5 * t), 0.0)
    )


def scaffold(profile, quarter):
    rng = np.random.default_rng(7263 + profile * 193 + quarter * 271)
    count = (23, 25, 25)[profile - 1]
    # The sequence is phyllotactic in height; no repeated star of six
    # equidistant boughs in a single horizontal plane.
    heights = np.r_[0, np.cumsum(rng.uniform(0.52, 1.55, count - 1))]
    heights = heights / heights[-1]
    turns = (
        rng.uniform(-0.42, 0.42, count)
        + np.arange(count) * 2.39996
        + quarter * 0.84
        + 0.67 * profile
    )
    wind = quarter * math.pi / 2

    def branch(i, t, a):
        skirt = (1.82, 2.18, 2.63)[profile - 1] + 0.12 * math.sin(quarter * 2.10)
        level = skirt + (10.35 - skirt) * t
        start = center(profile, level, quarter)
        direction = np.array((math.cos(a), 0, math.sin(a)))
        side = np.array((-direction[2], 0, direction[0]))
        sidewind = np.cos(a - wind)
        # Old crowns got narrower in ten identical concentric rings. These
        # hang in offset, unequal groups and form intermittent negative spaces.
        reach = (
            (3.04, 2.77, 2.45)[profile - 1]
            * (1 - 0.83 * t) ** 0.68
            * (0.83 + 0.25 * np.sin(a * 2.17 + t * 4.2 + profile) + 0.12 * sidewind)
            * rng.uniform(0.80, 1.17)
        )
        if i in (3, 10, 16):
            reach *= 0.53
        width = reach * (0.24 + 0.12 * rng.random())
        drop = (
            (0.39 + 0.62 * rng.random())
            * (1 - 0.33 * t)
            * (0.95 if profile == 3 else 1.10)
        )
        rise = 0.13 + 0.20 * rng.random() + 0.14 * t
        d_bend = rng.uniform(-0.26, 0.37)
        tint = np.array(
            ([0.107, 0.221, 0.131], [0.113, 0.218, 0.108], [0.094, 0.201, 0.119])[
                profile - 1
            ]
        ) * (0.89 + 0.21 * rng.random())
        return dict(
            org=start,
            d=direction,
            side=side,
            reach=reach,
            width=width,
            drop=drop,
            rise=rise,
            curve=d_bend,
            tint=tint,
            seed=int(rng.integers(1, 2**31)),
            t=t,
        )

    return [
        branch(i, t, a) for i, (t, a) in enumerate(zip(heights, turns, strict=True))
    ]


def axis(b, t):
    # Drooping inner fans and a rising mobile tip form a broken, non-level
    # underside. Wood/needles are a single 3D mass, never horizontal pancakes.
    h = (
        b["rise"] * math.sin(math.pi * t)
        - b["drop"] * (0.32 * t + 0.79 * t * t)
        + 0.13 * b["rise"] * t**5
    )
    return (
        b["org"]
        + b["d"] * b["reach"] * t
        + b["side"] * (b["curve"] * math.sin(math.pi * t) * b["reach"] * 0.45)
        + np.array((0.0, h, 0.0))
    )


def far_mass(out, b, boundary, seed):
    rng = np.random.default_rng(seed)
    half = b["width"] * boundary
    reach = b["reach"]
    # One small, closed envelope for a whole branch at the farthest LOD.
    stations = [0.48]
    alt = bool(seed % 2)
    start = out.vertex(axis(b, 0.0), b["tint"] * 0.76, uv(0, 0, alt))
    rows = []
    for j, t in enumerate(stations):
        p = axis(b, t)
        swell = math.sin(math.pi * (0.08 + 0.91 * t)) ** 0.72
        a = half * swell * (0.8 + 0.33 * rng.random())
        low = reach * (0.25 + 0.19 * rng.random()) * (1 - 0.32 * t)
        rise = reach * (0.13 + 0.14 * rng.random()) * swell
        low *= 1.42
        rise *= 1.27
        # Ordered in the cross section (viewed from tip), not flat billboards.
        vs = [
            p + np.array((0.0, rise, 0.0)),
            p
            + b["side"] * (a * (1.03 + 0.18 * math.sin(j * 3 + seed)))
            + np.array((0.0, -0.19 * low, 0.0)),
            p + b["side"] * (a * 0.10) + np.array((0.0, -low, 0.0)),
            p
            - b["side"] * (a * (0.88 + 0.24 * math.cos(j * 2.18 + seed)))
            + np.array((0.0, -0.31 * low, 0.0)),
        ]
        # Palette is by tissue, not painted direct light: desaturated tips and
        # the cooler, darker interior can still receive any native sunlight.
        cols = [
            b["tint"] * (1.16 + 0.12 * t) + np.array((0.012, 0.013, 0.002)),
            b["tint"] * (0.88 + 0.16 * t),
            b["tint"] * (0.66 + 0.11 * t),
            b["tint"] * (0.92 + 0.13 * t),
        ]
        rows.append(
            [
                out.vertex(v, c, uv(t, f, alt))
                for v, c, f in zip(vs, cols, (0, 1, 0, -1), strict=True)
            ]
        )
    end = out.vertex(
        axis(b, 1.0), b["tint"] * 1.25 + np.array((0.025, 0.027, 0.007)), uv(1, 0, alt)
    )
    for j in range(4):
        out.tri(start, rows[0][(j + 1) % 4], rows[0][j])
    for a, bb in zip(rows, rows[1:]):
        for j in range(4):
            out.quad(a[j], a[(j + 1) % 4], bb[(j + 1) % 4], bb[j])
    for j in range(4):
        out.tri(rows[-1][j], rows[-1][(j + 1) % 4], end)


def mesh(boughs, profile, quarter, lod):
    out = Crown()
    # The far envelope keeps the longest irregular forces from each crown
    # section; short interior shoots disappear before they become noise.
    if lod < 2:
        chosen = set(range(len(boughs)))
    else:
        # First protect the actual radial silhouette in eight directions.
        # A simple "longest of each pair" lost half a lee-side outline at L2.
        keep = set()
        directions = np.array(
            [
                (math.cos(i * math.tau / 8), 0.0, math.sin(i * math.tau / 8))
                for i in range(8)
            ]
        )
        for direction in directions:
            keep.add(
                max(
                    range(len(boughs)),
                    key=lambda j: max(
                        axis(boughs[j], t) @ direction
                        + 0.72
                        * boughs[j]["width"]
                        * abs(direction @ boughs[j]["side"])
                        * math.sin(t * math.pi)
                        for t in (0.35, 0.65, 1.0)
                    ),
                )
            )
        # Then preserve vertical/azimuthal holes, including upper levels.
        keep.update((0, len(boughs) - 1))
        while len(keep) < min(12, len(boughs)):

            def separation(j):
                v = boughs[j]
                return min(
                    3.1 * abs(v["t"] - boughs[i]["t"])
                    + 0.55 * (1 - v["d"] @ boughs[i]["d"])
                    for i in keep
                )

            keep.add(
                max((j for j in range(len(boughs)) if j not in keep), key=separation)
            )
        chosen = keep
    for k, b in enumerate(boughs):
        if k not in chosen:
            continue
        if lod < 2:
            from island_pine_sprays import emit

            emit(out, b, lod, axis)
        else:
            far_mass(out, b, 1.56, b["seed"])
    # No opaque green sleeve down the centre. Shorter forks from the real
    # woody leader fill the inner canopy irregularly. The original stem
    # remains visible naturally through negatives between sprays.
    H = (7.96, 9.84, 12.0)[profile - 1]
    top = center(profile, 12.0, quarter)
    top[1] = H
    foot = center(profile, 10.68, quarter)
    direction = unit(np.array((0.48, 0, 0.81)))
    for j in range(3 if lod < 2 else 2):
        az = j * 2.4 + quarter * 0.73
        side = np.array((math.cos(az), 0, math.sin(az)))
        base = foot + side * 0.06
        tip = top - side * (0.16 if j else 0)
        a = out.vertex(base, np.array((0.10, 0.20, 0.11)), uv(0, 0.02, j % 2))
        b = out.vertex(
            foot + 0.38 * (tip - foot) + side * 0.29,
            np.array((0.125, 0.24, 0.126)),
            uv(0.38, -0.95, j % 2),
        )
        c = out.vertex(
            foot + 0.48 * (tip - foot) - side * 0.27,
            np.array((0.14, 0.25, 0.13)),
            uv(0.48, 0.95, j % 2),
        )
        e = out.vertex(tip, np.array((0.18, 0.31, 0.15)), uv(1, 0.02, j % 2))
        out.tri(a, b, c)
        out.tri(c, b, e)
    return out
