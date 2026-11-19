"""Waxleaf calyx: fused scalloped petals, a cupped throat and drooping anthers.

Rounded edge topology and pastel vein paint live in the editable native mesh.
No billboard, emission or floating particle substitutes for the actual flower.
"""

from math import sin, cos, pi, tau
from island_meadow_sedges import mix, light


def petal_bell(m, center, axis, side, bend, length, radius, petal, level, phase):
    def pos(t, r, angle):
        return tuple(
            center[j]
            + axis[j] * length * t
            + (side[j] * cos(angle) + bend[j] * sin(angle)) * radius * r
            for j in range(3)
        )

    def stitch(lo, hi):
        for k in range(len(lo)):
            k2 = (k + 1) % len(lo)
            m.triangle(lo[k], lo[k2], hi[k2])
            m.triangle(lo[k], hi[k2], hi[k])

    sides = (25, 15, 10)[level]
    fade = (0.28, 0.27, 0.12)
    green = (0.34, 0.43, 0.24)
    throat = mix((0.87, 0.71, 0.39), petal, 0.25)

    def ring(t, r, stage, internal=False):
        row = []
        for k in range(sides):
            q = tau * k / sides + 0.10 * phase
            finger = 0.5 + 0.5 * cos(5 * q + 0.38 * phase)
            angle = q + 0.055 * sin(3 * q + phase) * stage
            vein = (0.5 + 0.5 * cos(5 * q + 0.40 * phase)) ** 8
            if internal:
                rr = r * (1 + 0.06 * stage * cos(5 * q))
                end = t + stage * (0.10 * finger - 0.028 * (1 - finger))
                c = mix(throat, mix(petal, (0.99, 0.94, 0.82), 0.33), stage)
                c = light(c, 1 - 0.13 * vein * (1 - 0.52 * stage))
            else:
                rr = r * (1 + 0.040 * stage * cos(5 * q) + 0.019 * sin(3 * q + phase))
                end = t + stage**2 * (0.18 * finger - 0.06 * (1 - finger))
                c = mix(mix(green, petal, 0.33), petal, min(1, stage * 1.65))
                c = mix(c, (0.98, 0.91, 0.76), 0.16 * stage)
                c = light(c, 1 - 0.095 * vein * (1 - 0.6 * stage))
            row.append(m.add(pos(end, rr, angle), c))
        return row

    lo = ring(0, 0.17, 0)
    outline = (
        [
            (0.16, 0.41, 0.14),
            (0.35, 0.71, 0.32),
            (0.65, 0.81, 0.59),
            (0.84, 0.92, 0.78),
            (0.90, 1, 0.98),
        ]
        if level == 0
        else [(0.26, 0.61, 0.22), (0.64, 0.82, 0.58), (0.90, 1, 0.98)]
    )
    for t, r, p in outline:
        hi = ring(t, r, p)
        stitch(lo, hi)
        lo = hi
    # A narrow rolled lip turns into a separately lit inner corolla. The
    # aperture has physical depth rather than a single flat closing polygon.
    for t, r, p in (
        ((0.94, 0.94, 0.97), (0.74, 0.74, 0.67), (0.43, 0.53, 0.38))
        if level == 0
        else ((0.92, 0.89, 0.95), (0.43, 0.51, 0.38))
    ):
        hi = ring(t, r, p, True)
        stitch(lo, hi)
        lo = hi
    heel = m.add(pos(0.35, 0, 0), throat)
    for k in range(sides):
        m.triangle(lo[k], lo[(k + 1) % sides], heel)
    # Four overlapping green sepals spread away from the hidden join, leaving
    # a recognisable green collar rather than the flower stuck to a straight pin.
    for s in range(5 if level == 0 else 3):
        q = tau * s / (5 if level == 0 else 3) + 0.18 * phase
        c = [
            m.add(pos(t, r, q + a), tone)
            for t, r, a, tone in [
                (-0.13, 0.11, 0, green),
                (0.24, 0.66, -0.19, (0.47, 0.54, 0.29)),
                (0.54, 0.78, 0.03, (0.53, 0.56, 0.33)),
                (0.23, 0.63, 0.18, (0.42, 0.49, 0.28)),
            ]
        ]
        m.triangle(c[0], c[1], c[2])
        m.triangle(c[0], c[2], c[3])
    # The pollen sits inside, barely above the rim of two early flowers; no
    # detached sparkle or bright yellow ball floating in front of a bloom.
    for s in range(3 if level == 0 else 1):
        q = tau * s / 3 + 0.45 * phase
        u = 0.19 if level == 0 else 0.13
        pts = [
            m.add(pos(t, r, q + a), c)
            for t, r, a, c in [
                (0.40, 0.12, 0, (0.69, 0.51, 0.24)),
                (0.75, u, -0.10, (0.91, 0.70, 0.31)),
                (0.80, u, 0.12, (0.92, 0.73, 0.32)),
                (0.90, u + 0.085, -0.04, (1, 0.84, 0.43)),
                (0.90, u - 0.10, 0.07, (0.99, 0.83, 0.46)),
                (0.97, u, 0, (0.98, 0.83, 0.47)),
            ]
        ]
        m.triangle(pts[0], pts[2], pts[1])
        m.triangle(pts[1], pts[2], pts[3])
        m.triangle(pts[1], pts[4], pts[2])
        m.triangle(pts[3], pts[2], pts[5])
        m.triangle(pts[5], pts[2], pts[4])
