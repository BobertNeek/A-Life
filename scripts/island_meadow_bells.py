"""A meadow flower with real sepals, nodding bells and a sun-warmed throat.

Five uneven flowers grow from three rooted stems. They are quiet botanical
waypoints, not glowing task markers, beads on sticks, or flat star decals.
All colours are later encoded as the same opaque native vertex material.
"""

from math import cos, sin, tau, pi, sqrt
import random
from island_meadow_sedges import Tussock, mix, light


def bellflowers(level):
    rng = random.Random(239512)
    m = Tussock()

    def circle(c, a, b, r, around, shade):
        return [
            m.add(
                tuple(
                    c[k]
                    + r * (cos(tau * j / around) * a[k] + sin(tau * j / around) * b[k])
                    for k in range(3)
                ),
                light(shade, 0.90 + 0.1 * cos(tau * j / around)),
            )
            for j in range(around)
        ]

    def join(last, row):
        for k in range(len(row)):
            l = (k + 1) % len(row)
            m.triangle(last[k], last[l], row[l])
            m.triangle(last[k], row[l], row[k])

    def add(a, b, weight=1):
        return tuple(a[i] + weight * b[i] for i in range(3))

    def leaf(start, a, ln, ht, w):
        axis = (cos(a), sin(a), 0)
        cross = (-sin(a), cos(a), 0)
        last = None
        for t in (0, 0.34, 0.67, 1) if level == 0 else (0, 0.48, 1):
            c = add(start, axis, ln * (0.5 * t + 0.5 * t * t))
            c = add(c, (0, 0, 1), ht * (0.55 * t + 0.65 * sin(pi * t)))
            color = mix((0.21, 0.33, 0.16), (0.50, 0.62, 0.34), 0.1 + 0.76 * t)
            if t == 1:
                tip = m.add(c, color)
                m.triangle(last[0], last[1], tip)
                m.triangle(last[1], last[2], tip)
                break
            width = w * (0.14 + 0.86 * sin(pi * t) ** 0.63)
            row = [
                m.add(
                    add(add(c, cross, width * j), (0, 0, 1), width * 0.32 * (j == 0)),
                    light(color, 1.08 if not j else 0.94),
                )
                for j in (-1, 0, 1)
            ]
            if last:
                for j in (0, 1):
                    m.triangle(last[j], last[j + 1], row[j + 1])
                    m.triangle(last[j], row[j + 1], row[j])
            last = row

    bases = ((-0.073, 0.035), (0.05, -0.04), (0.038, 0.067))
    for j in range([14, 9, 6][level]):
        r = bases[j % 3]
        a = (j * 0.382 * tau) + rng.uniform(-0.33, 0.33)
        leaf(
            (*r, 0.005),
            a,
            0.13 + rng.random() * 0.10,
            0.07 + rng.random() * 0.064,
            0.013 + rng.random() * 0.017,
        )
    count = [5, 4, 3][level]
    for i in range(count):
        root = bases[i % 3]
        a = i * 2.34 + 0.33
        d = (cos(a), sin(a), 0.0)
        side = (-sin(a), cos(a), 0.0)
        h = (0.36, 0.43, 0.33, 0.28, 0.41)[i]
        span = (0.09, 0.11, 0.17, 0.14, 0.13)[i]
        last = None
        rings = [0, 0.47, 0.79, 1] if level == 0 else [0, 0.74, 1]
        for j, t in enumerate(rings):
            # Offset source pairs leave three distinct basal crowns rather
            # than one upright stem for each regular point on a circle.
            p = (
                root[0] + span * cos(a) * t**1.8,
                root[1] + span * sin(a) * t**1.8,
                0.004 + h * (1.18 * t - 0.26 * t * t),
            )
            row = circle(
                p,
                (1, 0, 0),
                (0, 1, 0),
                (0.0037 if level != 2 else 0.005) * (1 - 0.43 * t),
                [4, 3, 3][level],
                mix((0.22, 0.30, 0.17), (0.41, 0.53, 0.28), t),
            )
            if last:
                join(last, row)
            if j == 1 and i < 3:
                leaf(p, a + 0.8 * (1 if i % 2 else -1), 0.115, 0.07, 0.010)
            last = row
        # A nodding flower axis, outward and downward. The aperture still
        # presents its warm centre to a low horizontal eye.
        axis = (d[0] * 0.91, d[1] * 0.91, -0.4146)
        bend = (d[0] * 0.4146, d[1] * 0.4146, 0.91)
        length = (0.065, 0.062, 0.053, 0.047, 0.061)[i]
        rad = (0.042, 0.04, 0.035, 0.029, 0.038)[i]
        count_side = [10, 8, 6][level]
        petal = [
            (0.85, 0.67, 0.55),
            (0.77, 0.65, 0.76),
            (0.86, 0.77, 0.59),
            (0.72, 0.65, 0.78),
            (0.84, 0.74, 0.64),
        ][i]
        center = p
        # Branch attachment and calyx are part of the body of each bell.
        last = circle(center, side, bend, rad * 0.19, count_side, (0.37, 0.46, 0.23))
        for j, (t, mult) in enumerate(((0.20, 0.52), (0.62, 0.81), (1, 1))):
            row = []
            for k in range(count_side):
                theta = k * tau / count_side
                lip = 1 if k % 2 == 0 else 0.76
                endt = t + (0.09 * (1 if k % 2 == 0 else -1) if j == 2 else 0)
                rr = rad * mult * (lip if j == 2 else 1)
                c = add(
                    add(add(center, axis, length * endt), side, cos(theta) * rr),
                    bend,
                    sin(theta) * rr,
                )
                row.append(m.add(c, mix((0.50, 0.50, 0.33), petal, (0.33, 0.87, 1)[j])))
            join(last, row)
            last = row
        throat = add(center, axis, length * 0.56)
        inner = circle(throat, side, bend, rad * 0.43, count_side, (0.80, 0.61, 0.32))
        join(last, inner)
        kernel = m.add(add(throat, axis, 0.009), (0.92, 0.75, 0.37))
        for k in range(count_side):
            m.triangle(inner[k], inner[(k + 1) % count_side], kernel)
    return m
