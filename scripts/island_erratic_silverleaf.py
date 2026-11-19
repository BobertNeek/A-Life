"""Low, wind-combed silverleaf runner beside the real glacial erratics.

Continuous bronze stems, opposing slightly curled sessile leaves, and sparse
ivory spent tufts. A rooted habitat mat in two irregular lobes rather than
planted rows or bright polygon buttons. Cosmetic, not a harvestable plant.
"""

import math
import numpy as np


def cover(g, land, center, down, cross, edge, outside, scale, seed):
    r = np.random.default_rng(97011 + seed * 39)

    def h(p):
        return land.surface(*p)[0]

    def tip(p, up, c):
        return g.point((p[0], h(p) + up, p[1]), np.clip(c, 0, 255))

    def norm(p):
        return p / max(0.00001, np.linalg.norm(p))

    def stem(a, b, ya, yb, wide, ink):
        along = norm(b - a)
        normal = np.array((-along[1], along[0]))
        aa = []
        bb = []
        for side, lift in ((-1, 0), (0, 0.72), (1, 0)):
            aa.append(
                tip(a + normal * wide * side, ya + wide * lift, ink * (1 + 0.08 * lift))
            )
            bb.append(
                tip(
                    b + normal * wide * 0.67 * side,
                    yb + wide * lift * 0.7,
                    ink * (1 + 0.08 * lift),
                )
            )
        for j in (0, 1):
            g.face(aa[j], bb[j], bb[j + 1])
            g.face(aa[j], bb[j + 1], aa[j + 1])

    def oval(p, d, L, w, up, ink, turn):
        side = np.array((-d[1], d[0]))
        rows = []
        for t in (0, 0.30, 0.67, 0.94, 1):
            spread = w * (math.sin(t * math.pi) ** 0.68 if 0 < t < 1 else 0.025)
            q = p + d * L * t + side * L * turn * t * t
            y = up + L * (0.60 * t + 0.14 * math.sin(np.pi * t))
            row = []
            for k in (-1, 0, 1):
                # Old curled lip catches cool light, midrib is not a dark
                # drawn outline. Each leaf is solid folded geometry.
                lift = spread * (0.18 if k == 0 else -0.18 + (0.17 if k > 0 else 0))
                color = ink * (
                    0.82 + 0.17 * t + (0.055 if k > 0 else 0.018 if k == 0 else 0)
                )
                row.append(tip(q + k * spread * side, y + lift, color))
            rows.append(row)
        for a, b in zip(rows, rows[1:]):
            for j in (0, 1):
                g.face(a[j], b[j], b[j + 1])
                g.face(a[j], b[j + 1], a[j + 1])

    def plume(p, d, L, ink):
        along = p + d * L * 0.32
        stem(p, along, 0.032, L, 0.003, ink)
        for k in range(5):
            a = k * math.tau / 5 + r.uniform(-0.18, 0.18)
            axis = np.array((math.cos(a), math.sin(a)))
            side = np.array((-axis[1], axis[0]))
            root = tip(along, L, (159, 147, 107))
            q = along + axis * L * r.uniform(0.15, 0.24)
            l = tip(q - side * 0.003, L + 0.021, (193, 186, 155))
            rr = tip(q + side * 0.003, L + 0.024, (211, 205, 171))
            t = tip(q + axis * 0.019, L + 0.047, (211, 206, 175))
            g.face(root, l, rr)
            g.face(l, t, rr)

    # One long crescent and a smaller daughter, with open mineral space
    # between them. Actual footprint separates their roots from the prop.
    side_sign = 1 if seed % 3 else -1
    leaf_count = stems = flowers = 0
    for lobe in range(2):
        direct = norm(
            down * (0.38 if lobe == 0 else 0.63)
            + cross * side_sign * (1 if lobe == 0 else -0.65)
        )
        hub = edge(direct) + direct * (r.uniform(0.13, 0.25) + lobe * 0.27)
        roots = 13 if lobe == 0 else 5
        for k in range(roots):
            origin = (
                hub
                + down * r.normal(0.18 if lobe == 0 else 0.43, 0.32) * scale
                + cross * r.normal(0, 0.14) * scale
            )
            if not outside(origin, 0.016):
                continue
            axis = norm(
                down * r.uniform(-0.46, 0.92) + cross * side_sign * r.uniform(-0.85, 1)
            )
            az = math.atan2(axis[1], axis[0])
            length = scale * r.uniform(0.27, 0.69) * (1.45 if k % 6 == 0 else 1)
            last = origin
            high = 0.009
            stems += 1
            nodes = r.integers(5, 9)
            tint = np.array(
                r.choice(
                    (
                        (104, 125, 88),
                        (116, 135, 93),
                        (125, 139, 102),
                        (111, 132, 97),
                        (135, 143, 104),
                    )
                )
            )
            for j in range(nodes):
                az += r.normal(0, 0.19)
                d = np.array((math.cos(az), math.sin(az)))
                q = last + d * length / nodes
                if not outside(q, 0.012):
                    break
                y = r.uniform(0.014, 0.030) * scale
                stem(last, q, high, y, 0.0037 * scale, np.array([129.0, 119, 86]))
                for sign in (-1, 1):
                    branch = az + sign * r.uniform(0.73, 1.31)
                    ld = np.array((math.cos(branch), math.sin(branch)))
                    L = scale * r.uniform(0.060, 0.126) * (1 - 0.24 * j / nodes)
                    oval(
                        q,
                        ld,
                        L,
                        L * r.uniform(0.19, 0.29),
                        y,
                        tint * r.uniform(0.94, 1.07),
                        sign * r.uniform(0.04, 0.16),
                    )
                    leaf_count += 1
                if j == nodes - 2 and k % 4 == 1 and lobe == 0:
                    plume(q, d, r.uniform(0.12, 0.20) * scale, np.array((127, 140, 91)))
                    flowers += 1
                last = q
                high = y
    return dict(runners=stems, leaves=leaf_count, spent_plumes=flowers)
