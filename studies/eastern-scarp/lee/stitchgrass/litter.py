"""Litter for the six-metre plant LOD, all vertices seated on recorded V1 faces.
A weathered blade is folded, not a dark decal. Mid-mesh 2–8mm increases are
art relief only. No object makes a new hard or walkable surface.
"""

from collections import defaultdict
import math
import numpy as np


def make(points, tris, shoots, older):
    cells = defaultdict(list)
    for t in points[tris]:
        key = (round(t[:, 0].min() / 3.125), round(t[:, 2].min() / 3.125))
        cells[key].append(t)

    def ground(x, z):
        key = (math.floor(x / 3.125), math.floor(z / 3.125))
        pt = np.array([x, z])
        for v in cells[key]:
            a, b, c = v[:, [0, 2]]
            ab, ac = b - a, c - a
            det = ab[0] * ac[1] - ab[1] * ac[0]
            dx, dz = pt - a
            w = (dx * ac[1] - dz * ac[0]) / det
            u = (ab[0] * dz - ab[1] * dx) / det
            if w >= -1e-6 and u >= -1e-6 and u + w <= 1.000001:
                return float((1 - w - u) * v[0, 1] + w * v[1, 1] + u * v[2, 1])
        raise ValueError("Not on a genuine source face")

    rng = np.random.default_rng(723731)
    p = []
    col = []
    ii = []

    def piece(uv, dr, c, fs):
        off = len(p)
        for k, (x, z) in enumerate(uv):
            p.append([x, ground(x, z) + dr[k], z])
            col.append(c[k])
        ii.extend([[off + n for n in f] for f in fs])

    groups = defaultdict(list)
    for r in shoots:
        groups[(r["drift"], r["rhizome"])].append([float(r["x"]), float(r["z"])])
    sites = [(np.array(group).mean(0), False) for group in groups.values()]
    sites += [(np.array([float(r["x"]), float(r["z"])]), True) for r in older]
    counts = defaultdict(int)
    for pos, old in sites:
        if np.linalg.norm(pos - [366.2, -83.7]) > 7.4:
            continue
        # Individual flecks concentrate near the source plant's union with
        # turf. Never uniform salt across the full or even the local patch.
        mass = rng.uniform(0.51, 1.12)
        for k in range(int((28 if old else 17) * mass)):
            th = rng.uniform(-math.pi, math.pi)
            a = np.array([math.cos(th), math.sin(th)])
            rr = rng.uniform(0.04, 0.74 if old else 0.57) ** 1.2
            ctr = pos + rr * a + [-0.028, 0.0]
            ang = rng.normal(0.1, 0.92) if rng.random() > 0.36 else th
            a = np.array([math.cos(ang), math.sin(ang)])
            side = np.array([-a[1], a[0]])
            tag = rng.choice(
                ["spent blade", "split hull", "grit"], p=[0.33, 0.44, 0.23]
            )
            noise = rng.uniform(0.85, 1.12)
            if tag == "spent blade":
                length = rng.uniform(0.036, 0.124)
                w = length * rng.uniform(0.042, 0.074)
                start = ctr - a * 0.45 * length
                mid = ctr + side * rng.uniform(-0.008, 0.008)
                end = ctr + a * 0.55 * length + side * rng.uniform(-0.01, 0.01)
                color = noise * np.array([0.192, 0.149, 0.070])
                piece(
                    [start, mid - side * w, mid, mid + side * w, end],
                    [-0.001, 0.001, 0.0038, 0.001, 0.0005],
                    [color * 0.75, color * 0.91, color * 1.18, color, color * 1.1],
                    [(0, 1, 2), (0, 2, 3), (1, 4, 2), (2, 4, 3)],
                )
            elif tag == "split hull":
                length = rng.uniform(0.023, 0.077)
                w = length * rng.uniform(0.12, 0.21)
                start = ctr - a * 0.55 * length
                end = ctr + a * 0.45 * length
                mid = ctr + side * 0.002
                color = noise * np.array([0.156, 0.105, 0.041])
                piece(
                    [start, mid - side * w, mid - side * 0.24 * w, mid + side * w, end],
                    [-0.001, 0.004, 0.006, 0.001, -0.001],
                    [color * 0.62, color * 0.82, color * 1.21, color, color * 1.10],
                    [(0, 1, 2), (0, 2, 3), (1, 4, 2), (2, 4, 3)],
                )
            else:
                r = rng.uniform(0.008, 0.029)
                base = noise * np.array([0.149, 0.142, 0.100])
                ring = []
                ph = rng.uniform(-math.pi, math.pi)
                for v in range(5):
                    ang = ph + v * 2 * math.pi / 5
                    ring.append(
                        ctr
                        + r
                        * rng.uniform(0.72, 1.18)
                        * (a * math.cos(ang) + side * 0.72 * math.sin(ang))
                    )
                piece(
                    [*ring, ctr + r * (a * 0.13 + side * 0.12)],
                    [0.0006, 0.0009, 0.0004, 0.0007, 0.0005, r * 0.29],
                    [
                        base * 0.75,
                        base * 0.88,
                        base,
                        base * 0.91,
                        base * 0.71,
                        base * 1.22,
                    ],
                    [(v, (v + 1) % 5, 5) for v in range(5)],
                )
            counts[tag] += 1
    p = np.array(p, "<f4")
    ii = np.array(ii, "<u2")
    col = np.array(col, "<f4")
    face = np.cross(p[ii[:, 1]] - p[ii[:, 0]], p[ii[:, 2]] - p[ii[:, 0]])
    anti = face[:, 1] < 0
    ii[anti] = ii[anti][:, [0, 2, 1]]
    face[anti] *= -1
    n = np.zeros_like(p)
    for j in range(3):
        np.add.at(n, ii[:, j], face)
    n /= np.maximum(1e-12, np.linalg.norm(n, axis=1, keepdims=True))
    return p, n, col, ii, dict(counts)
