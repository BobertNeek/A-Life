"""Small turf communities with real roots on the convex rim of a tidal bank.

No boulders, bushes or giant tree roots are invented. The bank's existing
physical terrain and its actual hollow govern which stems can be planted.
"""

from collections import defaultdict
import math
import numpy as np
from scipy.ndimage import gaussian_filter, map_coordinates


class Fibres:
    def __init__(self):
        self.v = []
        self.c = []
        self.tri = []

    def point(self, p, rgb):
        col = np.clip(np.asarray(rgb) / 255, 0, 1)
        lin = np.where(col <= 0.04045, col / 12.92, ((col + 0.055) / 1.055) ** 2.4)
        self.c.append(tuple(np.uint8(np.rint(255 * lin))) + (255,))
        self.v.append(tuple(float(e) for e in p))
        return len(self.v) - 1

    def face(self, a, b, c):
        va, vb, vc = [np.asarray(self.v[q]) for q in (a, b, c)]
        if np.cross(vb - va, vc - va)[1] < 0:
            b, c = c, b
        self.tri.append((a, b, c))


def fringe(coast, variant):
    from island_inlet_ground import X0, X1, Z0, Z1

    r = np.random.default_rng(673103)
    tiles = defaultdict(Fibres)
    H = gaussian_filter(coast.h, 0.60)
    gz, gx = np.gradient(H, coast.s)
    grade = np.hypot(gx, gz)

    def samp(f, x, z):
        return float(
            map_coordinates(
                f,
                [[(z - coast.z0) / coast.s], [(x - coast.x0) / coast.s]],
                order=1,
                mode="nearest",
            )[0]
        )

    def ground(p):
        return float(coast.floor(np.array(p[0]), np.array(p[1]))[0])

    def seated(p, up):
        return (p[0], ground(p) + up, p[1])

    def safe(x, z):
        i, j = round((x - coast.x0) / coast.s), round((z - coast.z0) / coast.s)
        return np.array_equal(
            coast.h[j - 2 : j + 3, i - 2 : i + 3],
            variant.h[j - 2 : j + 3, i - 2 : i + 3],
        )

    def turf(p, center, down, side):
        core = r.uniform(0.10, 0.23)
        green = np.array(
            r.choice(
                [
                    (108, 136, 71),
                    (124, 144, 75),
                    (111, 136, 76),
                    (119, 143, 85),
                    (135, 143, 80),
                ]
            )
        )
        for _ in range(r.integers(18, 38)):
            drift = r.normal(0, 0.66)
            a = down * math.cos(drift) + side * math.sin(drift)
            ac = np.array((-a[1], a[0]))
            root = center + r.normal(0, core * 0.50, 2)
            length = r.uniform(0.16, 0.48)
            height = r.uniform(0.10, 0.32)
            w = r.uniform(0.008, 0.024)
            if r.random() < 0.22:
                length *= 1.29
                height *= 0.80
            rooty = ground(root)
            tint = r.uniform(0.83, 1.12)
            last = []
            for t in (0, 0.32, 0.69, 1.0):
                q = (
                    root
                    + a * length * t**1.5
                    + ac * (0.05 * length * math.sin(t * math.pi))
                )
                lift = height * math.sin(math.pi * (0.77 * t)) * (1 - 0.54 * t)
                yy = max(ground(q) + 0.005, rooty + lift - 0.13 * t * t)
                ww = w * (0.44 + 0.56 * math.sin(t * math.pi)) * (1 - t) ** 0.5
                rgb = green * (0.61 + 0.45 * t) * tint + np.array((15, 7, -3)) * t
                if t == 1:
                    tip = p.point((q[0], yy, q[1]), rgb)
                    p.face(last[0], last[1], tip)
                    p.face(last[1], last[2], tip)
                    break
                row = [
                    p.point(
                        (
                            q[0] + ac[0] * ww * k,
                            yy + (0.18 * ww if k == 0 else 0),
                            q[1] + ac[1] * ww * k,
                        ),
                        rgb * (1.12 if k == 0 else 0.95),
                    )
                    for k in (-1, 0, 1)
                ]
                if last:
                    for i in (0, 1):
                        p.face(last[i], row[i], row[i + 1])
                        p.face(last[i], row[i + 1], last[i + 1])
                last = row

    def rootlet(p, origin, a, ac, length, width, col, branch=True):
        turn = r.uniform(-0.30, 0.30)
        last = None
        # A true root clings to the underlying cut earth, not a dangling beam.
        for i, t in enumerate((0, 0.19, 0.40, 0.64, 0.85, 1)):
            q = origin + length * (a * t + ac * turn * math.sin(math.pi * t))
            w = width * (1 - 0.81 * t)
            rows = []
            for k, l in ((-1, 0.0043), (0, 0.0075 + w * 0.66), (1, 0.0045)):
                xy = q + ac * w * k
                rows.append(
                    p.point(seated(xy, l), col * (0.82 + 0.12 * t + 0.16 * (k == 0)))
                )
            if last:
                for k in (0, 1):
                    p.face(last[k], rows[k], rows[k + 1])
                    p.face(last[k], rows[k + 1], last[k + 1])
            last = rows
            if branch and i in (2, 3) and r.random() < 0.74:
                vec = a + ac * r.choice((-1, 1)) * r.uniform(0.36, 0.75)
                vec /= np.linalg.norm(vec)
                rootlet(
                    p,
                    q,
                    vec,
                    np.array((-vec[1], vec[0])),
                    length * r.uniform(0.23, 0.43),
                    w * 0.57,
                    col * 0.94,
                    False,
                )

    sites = []
    for _ in range(6000):
        x, z = r.uniform(X0 + 6, X1 - 6), r.uniform(Z0 + 4, Z1 - 6)
        h = ground((x, z))
        gr = samp(grade, x, z)
        if not 4.8 < h < 11.2 or not 0.28 < gr < 0.87 or not safe(x, z):
            continue
        down = -np.array([samp(gx, x, z), samp(gz, x, z)])
        down /= max(0.001, np.linalg.norm(down))
        side = np.array((-down[1], down[0]))
        center = np.array((x, z))
        if (
            samp(grade, *(center + 2.4 * down)) - samp(grade, *(center - 2.4 * down))
            < 0.18
        ):
            continue
        if min((np.linalg.norm(center - q) for q, _ in sites), default=100) < r.uniform(
            5.6, 9.3
        ):
            continue
        # Leave a couple of extended bare reaches, not a trim along both banks.
        if x < 100 and 405 < z < 417 or x > 100 and 428 < z < 437:
            continue
        clusters = int(r.integers(3, 8))
        sites.append((center, clusters))
        for j in range(clusters):
            q = center + side * r.normal(0, 0.89) + down * r.normal(0, 0.24)
            if not safe(*q):
                continue
            tile = tiles[math.floor(q[0] / 50), math.floor(q[1] / 50)]
            turf(tile, q, down, side)
            for k in range(int(r.integers(3, 8))):
                o = q + side * r.normal(0, 0.18) + down * r.uniform(0.02, 0.25)
                vec = down + r.uniform(-0.43, 0.42) * side
                vec /= np.linalg.norm(vec)
                rootlet(
                    tile,
                    o,
                    vec,
                    np.array((-vec[1], vec[0])),
                    r.uniform(0.24, 0.75),
                    r.uniform(0.004, 0.014),
                    np.array((136, 111, 68)) * r.uniform(0.85, 1.11),
                )
        if len(sites) >= 12:
            break
    # A scatter inside a few washed pockets, not a chain of boulders across
    # the bank. The field stones are seated lenses, less than 33 mm proud at
    # their high point. No toy-scale obstacles are fabricated.
    gravels = 0
    pockets = []
    for _ in range(6000):
        x, z = r.uniform(X0 + 7, X1 - 7), r.uniform(Z0 + 5, Z1 - 5)
        h, dx, dz = coast.floor(np.array(x), np.array(z))
        fall = math.hypot(dx, dz)
        if (
            not 0.55 < h < 6.7
            or not 0.7 < fall < 2.4
            or abs(dx) < 0.70 * fall
            or not safe(x, z)
        ):
            continue
        c = np.array([x, z])
        if min((np.linalg.norm(c - k) for k in pockets), default=99) < 6.5:
            continue
        pockets.append(c)
        dd = np.array([dx, dz]) / fall
        along = np.array([-dd[1], dd[0]])
        for i in range(r.integers(9, 25)):
            p = (
                c
                + r.normal(0, 1.30) * along
                + r.normal(0, 0.44) / math.sqrt(1 + fall**2) * dd
            )
            h, px, pz = coast.floor(np.array(p[0]), np.array(p[1]))
            if h < 0.2 or not safe(*p):
                continue
            tile = tiles[math.floor(p[0] / 50), math.floor(p[1] / 50)]
            turn = r.normal(0, 0.30)
            a = along * math.cos(turn) + dd * math.sin(turn) / math.sqrt(1 + fall**2)
            t = -along * math.sin(turn) + dd * math.cos(turn) / math.sqrt(1 + fall**2)
            ln = r.uniform(0.07, 0.38)
            w = ln * r.uniform(0.41, 0.74)
            lift = r.uniform(0.012, 0.030)
            palette = np.array(
                r.choice(
                    [
                        (128, 127, 114),
                        (132, 133, 121),
                        (119, 128, 124),
                        (146, 137, 114),
                        (123, 118, 107),
                    ]
                )
            ) * r.uniform(0.92, 1.08)
            rim = []
            crown = []
            offset = r.uniform(-0.22, 0.22)
            for j in range(6):
                theta = j * math.tau / 6 + offset
                d = (ln * a * math.cos(theta) + w * t * math.sin(theta)) * r.uniform(
                    0.82, 1.11
                )
                q = p + d
                q2 = p + 0.57 * d + a * (0.08 * ln)
                rim.append(tile.point(seated(q, 0.0005), palette * 0.83))
                crown.append(
                    tile.point(
                        seated(q2, 0.007 + 0.62 * lift),
                        palette * r.uniform(0.97, 1.035),
                    )
                )
            cap = tile.point(seated(p, 0.003 + lift), palette * 1.01)
            for j in range(6):
                k = (j + 1) % 6
                tile.face(rim[j], rim[k], crown[j])
                tile.face(crown[j], rim[k], crown[k])
                tile.face(crown[j], crown[k], cap)
            gravels += 1
        if len(pockets) >= 10:
            break
    return tiles, dict(
        convex_rim_communities=len(sites),
        rooted_grass_clumps=sum(c for _, c in sites),
        embedded_gravel_pockets=len(pockets),
        flat_field_stones=gravels,
    )
