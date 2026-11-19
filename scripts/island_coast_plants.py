"""Wind-shaped, non-interactive shore thickets and dry seed combs.

Each rosette grows off a bent spray; the exposed edge stays lower than the
leeward clumps. No collidable trunks or new food objects are introduced.
"""

import math
import numpy as np


def sail(p, r, x, z, u, v, h, length, width, tilt, turn, tones, tint):
    """A folded leaf with a continuous vein and an asymmetric pointed end."""
    c, s = math.cos(turn), math.sin(turn)
    arc = r.uniform(-0.21, 0.21)
    rows = []
    for t, w in [(0, 0.07), (0.38, 0.96), (0.73, 0.80), (1, 0.035)]:
        run = length * t
        bend = arc * length * t * t
        yy = h + tilt * t + math.sin(t * math.pi) * length * 0.15
        row = []
        for side in (-1, 0, 1):
            spread = bend + side * w * width
            xx = x + u + c * run - s * spread
            zz = z + v + s * run + c * spread
            col = np.array(tones[0]) * (1 - t) + np.array(tones[1]) * t
            row.append(
                p.put(
                    xx,
                    zz,
                    yy + (0.23 * w * width if side == 0 else 0),
                    tint(col, 0.87 + (0.17 if side == 0 else 0.04 * side) + 0.08 * t),
                )
            )
        rows.append(row)
    for a, b in zip(rows[:-1], rows[1:], strict=True):
        for j in range(2):
            p.fan(a[j], b[j], b[j + 1])
            p.fan(a[j], b[j + 1], a[j + 1])


def cane(p, x, z, points, width, angle, tint):
    c, s = math.cos(angle), math.sin(angle)
    rings = []
    for j, (u, v, h) in enumerate(points):
        w = width * (1 - 0.70 * j / (len(points) - 1))
        rings.append(
            [
                p.put(
                    x + u + c * w * side,
                    z + v + s * w * side,
                    h,
                    tint((106, 94, 73), 0.80 + 0.1 * j),
                )
                for side in (-1, 1)
            ]
        )
    for a, b in zip(rings[:-1], rings[1:], strict=True):
        p.fan(a[0], b[0], b[1])
        p.fan(a[0], b[1], a[1])


def bush(p, r, x, z, size, wind, tint):
    base = r.choice([(75, 103, 87), (86, 111, 78), (91, 105, 71)])
    tip = r.choice([(150, 164, 120), (165, 163, 111), (135, 157, 118)])
    for j in range(r.integers(3, 5)):
        heading = wind + r.uniform(-1.22, 1.22)
        c, s = math.cos(heading), math.sin(heading)
        reach = size * r.uniform(0.55, 0.96)
        rise = size * r.uniform(0.60, 1.04)
        points = []
        for t in (0, 0.24, 0.52, 0.76, 1):
            u = c * reach * (0.16 * t + 0.84 * t * t)
            v = s * reach * (0.16 * t + 0.84 * t * t)
            h = rise * (1.65 * t - 0.79 * t * t)
            points.append((u, v, h))
        cane(p, x, z, points, max(0.004, size * 0.019), heading + 1.4, tint)
        for k, (u, v, h) in enumerate(points[2::2]):
            for side in (-1, 1):
                turn = heading + side * r.uniform(0.50, 1.14)
                length = size * r.uniform(0.20, 0.35) * (1.1 - k * 0.09)
                sail(
                    p,
                    r,
                    x,
                    z,
                    u,
                    v,
                    h,
                    length,
                    length * r.uniform(0.18, 0.31),
                    length * r.uniform(0.11, 0.43),
                    turn,
                    (base, tip),
                    tint,
                )
        u, v, h = points[-1]
        sail(
            p,
            r,
            x,
            z,
            u,
            v,
            h,
            size * 0.22,
            size * 0.039,
            size * 0.12,
            heading,
            (base, tip),
            tint,
        )


def comb(p, r, x, z, size, angle, tint):
    c, s = math.cos(angle), math.sin(angle)
    pts = [
        (0, 0, 0),
        (c * size * 0.08, s * size * 0.08, size * 0.36),
        (c * size * 0.20, s * size * 0.20, size * 0.76),
        (c * size * 0.38, s * size * 0.38, size * 0.86),
    ]
    cane(p, x, z, pts, 0.008, angle + 0.9, tint)
    u, v, h = pts[-1]
    # A paired, drooping bronze plume, not a row of glowing billboard dots.
    for side in (-1, 1):
        t = angle + side * 0.42
        sail(
            p,
            r,
            x,
            z,
            u,
            v,
            h,
            size * 0.24,
            size * 0.024,
            -size * 0.075,
            t,
            ((130, 109, 68), (201, 173, 108)),
            tint,
        )


def thickets(beach, patch, field, tint):
    candidate = np.random.default_rng(18637)
    tiles = {}
    count = 0
    tufts = 0
    # Sample a bounded number of candidate stands in metres, not by tessellation.
    for sample in range(8000):
        x = candidate.uniform(-432, 432)
        z = candidate.uniform(-491, 491)
        if math.hypot(x / 445, z / 535) < 0.665:
            continue
        h, dx, dz = beach.surface(x, z)
        if not (4.1 < h < 24 and math.hypot(dx, dz) < 0.52):
            continue
        if field(x * 0.62 + 19, z * 0.62 - 25) < 0.52:
            continue
        # Prefer windward shelf lips near water, not an arbitrary ring at altitude.
        length = math.hypot(x / 445, z / 535)
        ox, oz = (x / 445) / length, (z / 535) / length
        seaward = beach.surface(x + ox * 22, z + oz * 22)[0]
        if seaward > 3.0 or h - seaward < 2.5:
            continue
        key = (math.floor(x / 20), math.floor(z / 20))
        if tiles.get(key, 0) >= 2:
            continue
        tiles[key] = tiles.get(key, 0) + 1
        r = np.random.default_rng(18637 + sample * 6791)
        count += 1
        wind = math.atan2(-oz, -ox) + 0.40
        phase = r.uniform(-0.61, 0.61)
        long = r.uniform(2.1, 5.4)
        short = r.uniform(0.65, 1.7)
        for k in range(r.integers(3, 6)):
            along = np.clip(r.normal(0, 0.62), -1, 1) * long
            cross = r.normal(0, short)
            u = x - oz * along + ox * cross
            v = z + ox * along + oz * cross
            ground, gx, gz = beach.surface(u, v)
            if ground < 3.4 or math.hypot(gx, gz) > 0.58:
                continue
            size = r.uniform(0.36, 1.02) * max(0.65, 1 - 0.13 * abs(along))
            p = patch(u, v, 4)
            bush(p, r, u, v, size, wind + phase, tint)
            tufts += 1
            for _ in range(r.integers(0, 3)):
                comb(
                    p,
                    r,
                    u + r.normal(0, 0.47),
                    v + r.normal(0, 0.42),
                    r.uniform(0.48, 0.95),
                    wind + r.uniform(-0.30, 0.30),
                    tint,
                )
    return count, tufts
