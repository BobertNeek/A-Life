"""A meadow flower with real sepals, nodding bells and a sun-warmed throat.

Five uneven flowers grow from three rooted stems. They are quiet botanical
waypoints, not glowing task markers, beads on sticks, or flat star decals.
All colours are later encoded as the same opaque native vertex material.
"""

from math import cos, sin, tau, pi
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
        # Each basil-like basal leaf opens on a short petiole, rolls one
        # shoulder back and returns to the ground. The older plant used a
        # three-step rectangular midsection that read as folded sheet metal.
        axis = (cos(a), sin(a), 0)
        cross = (-sin(a), cos(a), 0)
        sway = 0.27 * sin(3.6 * a + ln)
        arch = 0.75 + 0.10 * cos(2.1 * a)
        last = None
        stations = (
            (0, 0.17, 0.35, 0.55, 0.72, 0.88, 1)
            if level == 0
            else ((0, 0.22, 0.51, 0.79, 1) if level == 1 else (0, 0.34, 0.70, 1))
        )
        base_ink = (0.27, 0.40, 0.235)
        rim = (0.49, 0.63, 0.36)
        for t in stations:
            point = add(start, axis, ln * (0.21 * t + 0.85 * t * t))
            point = add(point, cross, sway * ln * sin(pi * t) * 0.23)
            point = add(point, (0, 0, 1), ht * (0.21 * t + arch * sin(pi * t) ** 0.93))
            ink = mix(base_ink, rim, 0.18 + 0.78 * t)
            if t == 0:
                last = [m.add(point, base_ink)]
                continue
            if t == 1:
                tip = m.add(point, light(ink, 1.04))
                for i in range(len(last) - 1):
                    m.triangle(last[i + 1], last[i], tip)
                break
            width = (
                w
                * (0.056 + 0.99 * sin(pi * t**0.83) ** 0.78)
                * (1 + 0.075 * sin(t * pi * 7 + a))
            )
            row = []
            ribs = (
                (-1.0, -0.50, 0.0, 0.54, 1.0)
                if level == 0 and w > 0.027
                else (-1.0, 0.0, 1.0)
            )
            for k in ribs:
                lr = 1 + 0.17 * sin(3.8 * a + t * 2.7) * k
                pos = add(point, cross, k * width * lr)
                fold = 0.43 * width * (1 - abs(k)) + 0.16 * width * k * sin(
                    a * 3 + 4 * t
                )
                pos = add(pos, (0, 0, 1), fold)
                col = mix(
                    ink,
                    (0.62, 0.72, 0.44),
                    0.195 * (1 - abs(k)) ** 5 * sin(pi * t) ** 0.7,
                )
                row.append(m.add(pos, light(col, 1 + 0.042 * k)))
            if len(last) == 1:
                for i in range(len(row) - 1):
                    m.triangle(last[0], row[i], row[i + 1])
            else:
                for i in range(len(row) - 1):
                    m.triangle(last[i + 1], last[i], row[i + 1])
                    m.triangle(last[i], row[i], row[i + 1])
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
        petal = [
            (0.85, 0.67, 0.55),
            (0.77, 0.65, 0.76),
            (0.86, 0.77, 0.59),
            (0.72, 0.65, 0.78),
            (0.84, 0.74, 0.64),
        ][i]
        # A real bending neck joins upright stem to pendant corolla.
        end = p
        for bend_t, up in ((0.54, 0.020), (1, 0.015)):
            end = tuple(
                p[k] + axis[k] * 0.031 * bend_t + (up if k == 2 else 0)
                for k in range(3)
            )
            rr = circle(
                end,
                side,
                bend,
                0.0025 if level == 0 else 0.003,
                [4, 3, 3][level],
                (0.44, 0.50, 0.29),
            )
            join(last, rr)
            last = rr
        from island_meadow_calyx import petal_bell

        petal_bell(m, end, axis, side, bend, length, rad, petal, level, i)
    return m
