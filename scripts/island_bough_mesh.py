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
    count = (25, 27, 29)[profile - 1]
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
            (2.60, 2.25, 1.97)[profile - 1]
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


def spray(out, b, lod, boundary, seed, sub=False):
    rng = np.random.default_rng(seed)
    half = b["width"] * boundary
    reach = b["reach"]
    # Four-point cross section is closed even at L2. The cluster is very
    # unequal vertically: short sunlit spine, long torn lower needles.
    if lod == 0:
        stations = [0.21, 0.40, 0.61, 0.79, 0.91] if not sub else [0.31, 0.62, 0.82]
    elif lod == 1:
        stations = [0.36, 0.71] if not sub else [0.5]
    else:
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
        low *= (1, 1.12, 1.42)[lod]
        rise *= (1, 1.08, 1.27)[lod]
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


def twig(b, forward, hand, size, angle, seed):
    d = unit(b["d"] * math.cos(angle) + b["side"] * (hand * math.sin(angle)))
    return dict(
        b,
        org=axis(b, forward),
        d=d,
        side=np.array((-d[2], 0, d[0])),
        reach=b["reach"] * size,
        width=b["width"] * (0.49 + 0.35 * size),
        drop=b["drop"] * (0.55 + size * 0.30),
        rise=b["rise"] * (1.08 + size * 0.23),
        curve=-b["curve"] * hand * 0.85,
        seed=seed,
        tint=b["tint"] * (1.02 + 0.05 * hand),
    )


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
        spray(out, b, lod, 1.0 if lod == 0 else 1.32 if lod == 1 else 1.66, b["seed"])
        if lod == 0:
            for f, hand, l, a in (
                (0.22, -1, 0.66, 0.53),
                (0.34, 1, 0.64, 0.52),
            ):
                t = twig(b, f, hand, l, a, b["seed"] + int(f * 100))
                spray(out, t, 0, 0.88, t["seed"], sub=True)
            for idx, (fr, hand) in enumerate(
                ((0.36, -1), (0.49, 1), (0.71, -1), (0.79, 1))
            ):
                # Short pendulous needle fingers divide a broad spray's
                # silhouette. Four little irregular closed solids, no alpha.
                t = twig(
                    b,
                    fr,
                    hand,
                    0.32 + 0.08 * (idx % 2),
                    0.77 + 0.05 * idx,
                    b["seed"] + 113 + idx,
                )
                origin = axis(t, 0.0) + b["side"] * (0.37 * hand * b["width"])
                end = (
                    axis(t, 1.0)
                    + b["side"] * (0.67 * hand * b["width"])
                    - np.array((0, 0.16 + 0.11 * b["reach"], 0))
                )
                mid = origin + 0.49 * (end - origin)
                n = t["side"] * t["width"] * 0.65
                col = b["tint"] * (0.84 + 0.07 * idx)
                begin = out.vertex(origin, col * 0.79, uv(0, 0, idx % 2))
                tip = out.vertex(end, col * 1.18, uv(1, 0, idx % 2))
                ring = (
                    out.vertex(mid + n, col * 0.91, uv(0.48, 1, idx % 2)),
                    out.vertex(
                        mid + np.array((0, t["width"] * 0.73, 0)),
                        col * 1.15,
                        uv(0.48, 0, idx % 2),
                    ),
                    out.vertex(mid - n, col * 0.90, uv(0.48, -1, idx % 2)),
                )
                for v in range(3):
                    out.tri(begin, ring[v], ring[(v + 1) % 3])
                    out.tri(ring[v], tip, ring[(v + 1) % 3])
        elif lod == 1 and k % 4 == 1:
            t = twig(b, 0.27, -1, 0.66, 0.54, b["seed"] + 27)
            spray(out, t, 1, 1.15, t["seed"], sub=True)
    # Connecting needles belong to several height ranges at once, not a disk
    # at each growth year. Seven unequal oblique ridges leave room for the
    # independently drooping sprays and interrupt the straight visible bole.
    rows = []
    angles = (7, 6, 4)[lod]
    fractions = (
        (0.12, 0.27, 0.42, 0.57, 0.71, 0.86, 0.96)
        if lod < 2
        else (0.13, 0.38, 0.69, 0.95)
    )
    for j, t in enumerate(fractions):
        y = 1.7 + 10.23 * t
        axis_p = center(profile, y, quarter)
        rad = (
            (1.10, 0.99, 0.89)[profile - 1]
            * (1 - 0.81 * t) ** 0.7
            * (0.72 + 0.27 * math.sin(t * math.pi))
        )
        rad *= (1, 1.14, 1.67)[lod]
        row = []
        for a in range(angles):
            az = (
                a * math.tau / angles
                + 0.24 * math.sin(0.3 * quarter + 4 * t + a * 0.81)
                + quarter * 0.53
            )
            width = rad * (
                0.84
                + 0.25 * math.sin(a * 2.36 + profile + 3 * t)
                + 0.18 * math.cos(9 * t - a)
            )
            lean = np.array((math.cos(az), 0, math.sin(az)))
            delta = (0.25 + 0.22 * (1 - t)) * math.sin(a * 2.4 + t * 3.7 + quarter)
            if j == 0:
                delta -= 0.31 + 0.36 * (0.5 + 0.5 * math.sin(a * 1.92 + quarter))
            pos = axis_p + width * lean + np.array((0, delta, 0))
            pos[1] = min(pos[1], (7.96, 9.84, 12.0)[profile - 1] - 0.19)
            hue = np.array((0.091, 0.191, 0.105)) * (
                0.84 + 0.17 * t + 0.065 * math.sin(a * 2.35 + t * 3.91)
            )
            row.append(out.vertex(pos, hue))
        rows.append(row)
    for lo, hi in zip(rows, rows[1:]):
        for j in range(angles):
            out.quad(lo[j], hi[j], hi[(j + 1) % angles], lo[(j + 1) % angles])
    # A small forked terminal plume replaces the old tree-top button.
    H = (7.96, 9.84, 12.0)[profile - 1]
    top = center(profile, 12.0, quarter)
    top[1] = H
    base = center(profile, 10.63, quarter)
    last = top - base
    direction = unit(np.array((0.69, 0, 0.34)))
    rad = 0.37 if profile != 3 else 0.29
    rings = []
    for t, rv in ((0.0, 0.34), (0.36, 1.0), (0.67, 0.75)):
        p = base + last * t
        j = range(5 if lod < 2 else 4)
        rings.append(
            [
                out.vertex(
                    p
                    + rad
                    * rv
                    * np.array(
                        (
                            math.cos(a * math.tau / len(j) + 0.48 * t),
                            -0.11 * math.sin(a * 2.4),
                            math.sin(a * math.tau / len(j) + 0.48 * t),
                        )
                    ),
                    np.array((0.14, 0.269, 0.152)) * (0.74 + 0.31 * t),
                )
                for a in j
            ]
        )
    tip = out.vertex(top, np.array((0.172, 0.304, 0.166)))
    # Keep the leader at exactly the same elevation even from far away.
    n = len(rings[0])
    for lo, hi in zip(rings, rings[1:]):
        for j in range(n):
            out.quad(lo[j], hi[j], hi[(j + 1) % n], lo[(j + 1) % n])
    for j in range(n):
        out.tri(rings[-1][j], tip, rings[-1][(j + 1) % n])
    return out
