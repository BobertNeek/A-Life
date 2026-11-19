"""Hand-shaped fern, sorrel, moss and exposed wood for a creature-sized forest floor."""

import math
import numpy as np
from build_island_strandline import tint

TAU = math.tau


def root(p, r, x, z, s, a, delta):
    length = r.uniform(0.58, 1.30) * s
    width = r.uniform(0.098, 0.16) * s
    turn = r.uniform(-0.37, 0.37)
    colour = ((124, 101, 72), (116, 96, 68), (130, 108, 79))[r.integers(3)]
    lines = []
    for t in (0, 0.18, 0.44, 0.69, 0.87, 1):
        d = a + turn * (0.25 + 0.75 * t)
        c, k = math.cos(d), math.sin(d)
        extent = 0.105 * s + length * t
        cx = x + c * extent
        cz = z + k * extent
        w = width * (1 - t) ** 0.77 + 0.009
        hump = max(
            -0.006, 0.123 * s * (1 - t) ** 2.4 + 0.015 * (1 - t) + delta * (1 - t) ** 4
        )
        rib = w * (0.10 + 0.15 * math.sin(3.1 * t + a))
        lines.append(
            [
                p.put(cx - k * w, cz + c * w, -0.008, tint(colour, 0.84)),
                p.put(cx - k * rib, cz + c * rib, hump, tint(colour, 1.02 - 0.15 * t)),
                p.put(cx + k * w * 0.82, cz - c * w * 0.82, -0.008, tint(colour, 0.86)),
            ]
        )
    for lo, hi in zip(lines[:-1], lines[1:], strict=True):
        for j in (0, 1):
            p.fan(lo[j], lo[j + 1], hi[j + 1])
            p.fan(lo[j], hi[j + 1], hi[j])


def moss(p, r, x, z, length, width, a):
    c, s = math.cos(a), math.sin(a)
    h = r.uniform(0.022, 0.047) * min(1.1, (length + 0.2) / 0.75)
    n = int(r.integers(8, 13))
    rad = r.uniform(0.73, 1.17, n)
    palette = (
        (103, 124, 66),
        (114, 135, 74),
        (102, 126, 79),
        (125, 138, 77),
        (106, 117, 65),
    )
    col = palette[r.integers(len(palette))]
    inner = []
    rim = []
    for i in range(n):
        t = TAU * i / n
        u = length * rad[i] * math.cos(t)
        v = width * rad[i] * math.sin(t)
        ox, oz = c * u - s * v, s * u + c * v
        rim.append(p.put(x + ox, z + oz, -0.005, tint(col, 0.77)))
        k = r.uniform(0.52, 0.81)
        inner.append(
            p.put(
                x + k * ox,
                z + k * oz,
                h * r.uniform(0.34, 0.95),
                tint(col, r.uniform(0.84, 1.11)),
            )
        )
    peak = p.put(x + 0.06 * length * c, z + 0.06 * length * s, h, tint(col, 1.03))
    for i in range(n):
        j = (i + 1) % n
        p.fan(rim[i], rim[j], inner[j])
        p.fan(rim[i], inner[j], inner[i])
        p.fan(inner[i], inner[j], peak)
    # Isolated seed leaves the surrounding fern placement unchanged.
    detail = np.random.default_rng((round(x * 11239) ^ round(z * 7193)) & 0xFFFFFFFF)
    if detail.random() < 0.16 and length > 0.21:
        for j in range(int(detail.integers(2, 6))):
            u, v = (
                detail.uniform(-0.4, 0.42) * length,
                detail.uniform(-0.37, 0.36) * width,
            )
            sx, sz = x + c * u - s * v, z + s * u + c * v
            lift = h + detail.uniform(0.024, 0.051)
            ox, oz = detail.uniform(-0.017, 0.016), detail.uniform(-0.017, 0.016)
            b = p.put(sx, sz, h * 0.33, tint((99, 111, 67), 0.80))
            l = p.put(
                sx + ox - 0.0027, sz + oz, lift - 0.007, tint((151, 126, 80), 0.91)
            )
            rr = p.put(
                sx + ox + 0.0027, sz + oz, lift - 0.006, tint((158, 133, 82), 1.02)
            )
            cap = p.put(
                sx + ox + 0.006,
                sz + oz - 0.001,
                lift + 0.006,
                tint((165, 144, 90), 0.97),
            )
            side = p.put(
                sx + ox - 0.005,
                sz + oz + 0.003,
                lift + 0.001,
                tint((127, 109, 74), 0.91),
            )
            p.fan(b, l, rr)
            p.fan(l, side, cap)
            p.fan(l, cap, rr)


def fern(p, r, x, z, size, yaw):
    count = int(r.integers(4, 7))
    leaf = ((86, 127, 72), (101, 139, 79), (98, 136, 63), (121, 146, 76))[r.integers(4)]
    for i in range(count):
        a = yaw + i * TAU / count + r.uniform(-0.21, 0.21)
        d = np.array((math.cos(a), math.sin(a)))
        off = np.array((-d[1], d[0]))
        reach = size * r.uniform(0.43, 0.88)
        height = size * r.uniform(0.52, 0.96)

        def point(t, side=0, lift=0):
            q = (
                np.array((x, z))
                + d * reach * t**1.32
                + off * (side + 0.082 * reach * math.sin(math.pi * t))
            )
            return q[0], q[1], height * (1.88 * t - 1.05 * t * t) + lift

        base = p.put(*point(0, lift=-0.004), tint(leaf, 0.69))
        l = p.put(*point(0.54, -0.012 * size), tint(leaf, 0.81))
        rr = p.put(*point(0.54, 0.012 * size), tint(leaf, 0.99))
        top = p.put(*point(1), tint(leaf, 1.21))
        p.fan(base, rr, l)
        p.fan(rr, top, l)
        for t in np.linspace(0.18, 0.87, int(r.integers(5, 8))):
            width = (
                (0.12 + 0.14 * math.sin(math.pi * (t - 0.13) / 0.88) ** 0.85)
                * size
                * (1 - 0.60 * t)
            )
            for sign in (-1, 1):
                shade = r.uniform(0.96, 1.09) * (1 + 0.06 * t)
                start = p.put(*point(t), tint(leaf, 0.84 * shade))
                l = p.put(
                    *point(t + 0.001, sign * width * 0.44, 0.02 * size),
                    tint(leaf, 0.97 * shade)
                )
                v = p.put(
                    *point(t + 0.067, sign * width * 0.42, 0.034 * size),
                    tint(leaf, 1.09 * shade)
                )
                tip = p.put(
                    *point(min(0.999, t + 0.074), sign * width, 0.014 * size),
                    tint(leaf, 1.08 * shade)
                )
                if sign > 0:
                    p.fan(start, v, l)
                    p.fan(l, v, tip)
                else:
                    p.fan(start, l, v)
                    p.fan(l, tip, v)


def sorrel(p, r, x, z, size, yaw):
    col = ((107, 139, 78), (123, 148, 78), (104, 134, 82))[r.integers(3)]
    for j in range(int(r.integers(3, 7))):
        a = yaw + 2.4 * j + r.uniform(-0.32, 0.32)
        rad = size * r.uniform(0.23, 0.72)
        ox = x + math.cos(a) * rad
        oz = z + math.sin(a) * rad
        h = size * r.uniform(0.40, 0.93)
        b = p.put(x, z, -0.002, tint(col, 0.62))
        neck = p.put(ox, oz, h, tint(col, 0.83))
        side = p.put(ox + 0.004, oz - 0.003, h * 0.95, tint(col, 0.80))
        p.fan(b, neck, side)
        for k in range(3):
            a2 = a + 2.094 * k + r.uniform(-0.13, 0.13)
            c, s = math.cos(a2), math.sin(a2)
            ln = size * r.uniform(0.28, 0.52)
            w = ln * r.uniform(0.36, 0.56)

            def v(u, q, l, light):
                return p.put(
                    ox + c * u - s * q, oz + s * u + c * q, h + l, tint(col, light)
                )

            l = v(ln * 0.56, -w, 0.017, 1.02)
            rr = v(ln * 0.58, w, 0.018, 1.07)
            m = v(ln * 0.54, 0, 0.004, 0.91)
            lt = v(ln, -w * 0.35, 0.009, 1.10)
            rt = v(ln, w * 0.37, 0.010, 1.11)
            n = v(ln * 0.84, 0, 0.002, 0.98)
            for tri in (
                (neck, l, m),
                (m, l, lt),
                (m, lt, n),
                (neck, m, rr),
                (m, rt, rr),
                (m, n, rt),
            ):
                p.fan(*tri)


def duff(p, r, x, z, a, pine):
    for k in range(int(r.integers(14, 28) if pine else r.integers(4, 9))):
        c, s = math.cos(a), math.sin(a)
        u, v = r.normal(0, 0.30), r.normal(0, 0.13)
        cx, cz = x + c * u - s * v, z + s * u + c * v
        heading = a + r.normal(0.10, 0.55 if pine else 1.02)
        c, s = math.cos(heading), math.sin(heading)
        ln = r.uniform(0.045, 0.12) if pine else r.uniform(0.071, 0.148)
        w = r.uniform(0.0023, 0.004) if pine else ln * r.uniform(0.23, 0.38)
        col = (
            ((139, 116, 79), (150, 126, 83), (119, 100, 69), (139, 116, 83))[
                r.integers(4)
            ]
            if pine
            else ((139, 110, 71), (159, 128, 76), (130, 111, 78), (157, 139, 89))[
                r.integers(4)
            ]
        )

        def v(i, j, h, k):
            return p.put(cx + c * i - s * j, cz + s * i + c * j, h, tint(col, k))

        root = v(0, 0, 0.002, 0.80)
        l = v(0.42 * ln, -w, 0.005 if pine else 0.014, 1.02)
        rr = v(0.49 * ln, w, 0.004 if pine else 0.012, 0.91)
        tip = v(ln, 0.12 * w, 0.002 if pine else 0.024, 0.98)
        if pine:
            p.fan(root, l, rr)
            p.fan(l, tip, rr)
        else:
            rib = v(0.53 * ln, 0, 0.004, 1.06)
            l2 = v(0.71 * ln, -0.76 * w, 0.013, 0.98)
            for tri in (
                (root, l, rib),
                (root, rib, rr),
                (rib, l, l2),
                (rib, l2, tip),
                (rib, tip, rr),
            ):
                p.fan(*tri)
