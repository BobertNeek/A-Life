"""Dune scrub as overlapping *bodies*, not little objects scattered over a ribbon.

Specific islands of growth: established salt-bush lee, glacial resting place,
sheltered bluff alders, then the windward sand break. The centre of the
surveyed line remains visually open. All the woody stems are soft dressing.
"""

import math
import numpy as np

# (metres along crossing, cross-wind displacement, length, width, habit)
# Asymmetric interruptions and open sweeps separate these larger masses.
BODIES = [
    (10, 8.0, 6.3, 3.3, "hazel"),
    (19, -9, 5.0, 3.2, "hazel"),
    (35, 7.2, 7.4, 4.8, "hazel"),
    (49, -6.6, 7.2, 4.2, "myrtle"),
    (60.3, -3.8, 5.8, 4.0, "myrtle"),
    (72.3, -5.3, 8.9, 4.5, "myrtle"),
    (86, 9.7, 5.9, 4.2, "myrtle"),
    (101, -4.8, 10.2, 5.3, "myrtle"),
    (114, -7.6, 6.8, 4.0, "myrtle"),
    (123, 10.8, 8.6, 4.2, "hazel"),
    (142, 6.8, 9.3, 5.5, "hazel"),
    (155, -8.4, 7.6, 3.2, "myrtle"),
    (165, 9.2, 7.4, 6.8, "hazel"),
    (172, 7.6, 8.9, 5.9, "hazel"),
    (185, 11.0, 5.6, 4.5, "myrtle"),
    (195, 7.8, 6.1, 4.6, "salt"),
    (199, -6.5, 6.0, 4.9, "salt"),
    (209, -7.0, 5.1, 3.6, "salt"),
    (212, 7.3, 3.6, 2.8, "salt"),
]


def beds(s, d):
    values = {k: np.zeros_like(s, dtype=float) for k in ("hazel", "myrtle", "salt")}
    for k, (u, v, sw, dw, kind) in enumerate(BODIES):
        a = (s - u) / sw
        b = (d - v + 0.18 * (s - u) + 0.72 * np.sin(0.57 * (s - u) + k)) / dw
        values[kind] = np.maximum(
            values[kind], np.exp(-0.5 * (a * a * 1.6 + b * b * 1.6))
        )
    return values


def thickets(reach, tiles, count, width, pt, tri, set_mesh):
    rand = np.random.default_rng(17899)

    def Y(p):
        return float(reach.floor(p[0], p[1])[0])

    def direction(a):
        return np.array([math.cos(a), math.sin(a)])

    def perp(u):
        return np.array([-u[1], u[0]])

    def cane(a, ay, b, by, w, c):
        v = b - a
        v = v / max(0.001, np.linalg.norm(v))
        n = perp(v) * w
        i = pt(a - n, ay, c * 0.82)
        j = pt(a + n, ay, c * 0.85)
        k = pt(b + n * 0.65, by, c)
        l = pt(b - n * 0.65, by, c * 0.93)
        tri(i, j, k)
        tri(i, k, l)

    def cup(p, h, u, L, W, up, ink):
        u = u / np.linalg.norm(u)
        n = perp(u)
        q = p + u * L * 0.49
        r = p + u * L * 0.84
        e = p + u * L
        a = pt(p, h, ink * 0.77)
        b = pt(q - n * W, h + up * 0.53, ink * 0.94)
        c = pt(q + n * W, h + up * 0.48, ink * 0.98)
        m = pt(q, h + up * 0.56 + W * 0.4, ink * 1.04)
        d = pt(r - n * W * 0.53, h + up * 0.87, ink)
        f = pt(r + n * W * 0.56, h + up * 0.85, ink * 1.04)
        t = pt(e, h + up, ink * 1.07)
        for face in ((a, b, m), (a, m, c), (b, d, m), (m, d, f), (m, f, c), (d, t, f)):
            tri(*face)

    hues = {
        "hazel": [(115, 142, 94), (138, 154, 99), (147, 158, 105)],
        "myrtle": [(111, 143, 122), (127, 153, 120), (143, 158, 126)],
        "salt": [(153, 158, 124), (159, 151, 113), (136, 156, 130)],
    }
    for k, (s0, d0, span, breadth, kind) in enumerate(BODIES):
        # Several overlapping branches cover an irregular shared root island.
        # Density is highest toward the older windward shoulder.
        for ii in range(int(7 + span * breadth * 0.51)):
            rad = math.sqrt(rand.random())
            az = rand.uniform(0, 2 * math.pi)
            s = s0 + rad * math.cos(az) * span
            d = d0 + rad * math.sin(az) * breadth - 0.16 * (s - s0)
            if abs(d) < width(s) * 1.12 + 0.48 or not 0 < s < reach.m[-1] - 1:
                continue
            p = np.array(reach.station(s, d))
            h, gx, gz = reach.floor(*p)
            if h < 0.79 or np.hypot(gx, gz) > 0.78:
                continue
            g = tiles[(int(s // 28), "scrub")]
            set_mesh(g)
            size = (
                rand.uniform(0.62, 1.37)
                * (1.10 - 0.46 * rad**1.7)
                * (0.95 if kind == "hazel" else 0.84 if kind == "myrtle" else 0.68)
            )
            ink = np.array(hues[kind][rand.integers(3)], dtype=float) * rand.uniform(
                0.91, 1.07
            )
            stemcol = ink * np.array([0.75, 0.61, 0.53])
            aa = rand.uniform(0, 6.28)
            y0 = Y(p) - 0.025
            for jj in range(int(rand.integers(6, 10))):
                az = aa + jj * 2.40 + rand.normal(0, 0.19)
                u = direction(az)
                # Body leans with sea air; each shoot has a separate irregular
                # height. No mound made of stacked discs or alpha-cross cards.
                u = 0.83 * u + np.array([0.50, 0.18])
                n = perp(u)
                L = size * rand.uniform(0.48, 0.90)
                H = size * rand.uniform(0.36, 0.81)
                q = p + u * L * 0.48
                top = p + u * L
                midh = max(Y(q) + 0.05, y0 + H * 0.75)
                ty = max(Y(top) + 0.05, y0 + H)
                cane(p, y0, q, midh, max(0.009, size * 0.017), stemcol)
                cane(q, midh, top, ty, max(0.007, size * 0.011), stemcol * 1.13)
                for j in range(5):
                    t = 0.10 + 0.17 * j
                    v = q + (top - q) * t
                    cy = midh + (ty - midh) * t
                    for side in (-1, 1):
                        out = u * 0.42 + n * side * rand.uniform(0.66, 1.15)
                        leaf = size * rand.uniform(0.13, 0.28) * (1.07 - 0.27 * t)
                        cup(
                            v,
                            cy,
                            out,
                            leaf,
                            leaf * rand.uniform(0.22, 0.37),
                            leaf * rand.uniform(-0.03, 0.47),
                            ink * rand.uniform(0.88, 1.09),
                        )
                cup(
                    top,
                    ty,
                    u + 0.15 * n,
                    size * 0.21,
                    size * 0.068,
                    size * 0.047,
                    ink * 1.035,
                )
            count["dune_scrub_stools"] += 1
