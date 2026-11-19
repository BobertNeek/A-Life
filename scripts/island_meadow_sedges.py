"""Opaque folded tussocks, not a radial rosette of identical spear leaves.

A low glaucous fountain and a few taller hanging fruits span the old footprint.
Few-point ridges and sunny/hollow throat colours read close without an alpha
atlas, while larger silvery pennons carry the shape toward the draw limit.
Blender coordinates only; exported by the weekly meadow art compositor.
"""

import math
import random


class Tussock:
    def __init__(self):
        self.points = []
        self.ink = []
        self.faces = []

    def add(self, p, c):
        self.points.append(tuple(p))
        self.ink.append(tuple(max(0, min(1, x)) for x in c))
        return len(self.ink) - 1

    def triangle(self, a, b, c):
        if a != b and b != c and a != c:
            self.faces.append((a, b, c))


def mix(a, b, t):
    return tuple((1 - t) * x + t * y for x, y in zip(a, b))


def light(c, v):
    return tuple(j * v for j in c)


def sedges(lod, habit="sweep"):
    # A low, raised crown at the root keeps leaves joined from oblique angles.
    # Each original plant has one of these three architectural *families*;
    # colour follows plant age rather than a random hue on every triangle.
    idx = ("sweep", "silver", "amber").index(habit)
    rand = random.Random(685317 + 29 * idx)
    spr = Tussock()
    wind_x, wind_y = (0.73, -0.28) if idx != 1 else (0.32, 0.14)
    # Drawing in gamma-like reference palette; convert to linear vertex paint
    # when assigning BYTE_COLOR in the Blender asset, not twice.
    lo = [(0.20, 0.30, 0.16), (0.23, 0.34, 0.24), (0.31, 0.33, 0.17)][idx]
    hi = [(0.56, 0.65, 0.37), (0.57, 0.68, 0.51), (0.67, 0.65, 0.37)][idx]
    sun = [(0.68, 0.72, 0.42), (0.70, 0.75, 0.57), (0.77, 0.71, 0.42)][idx]
    nests = [(-0.04, -0.025), (0.035, 0.012), (-0.012, 0.045), (0.096, -0.014)]
    keep = (
        range(22),
        {0, 1, 2, 3, 4, 5, 7, 9, 11, 13, 14, 16, 18, 21},
        {0, 1, 2, 4, 7, 11, 14, 18},
    )[lod]
    swaths = [(0.50, 1.03), (0.92, 1.06), (0.59, 0.76), (0.49, 0.86)]
    for i in range(22):
        # Deterministic sub-sampling preserves the original comp's anchors
        # instead of reseeding an unrelated LOD silhouette.
        root = nests[i % len(nests)]
        az = math.tau * (i * 0.3853 % 1) + rand.uniform(-0.39, 0.39)
        ox = root[0] + rand.uniform(-0.024, 0.024)
        oy = root[1] + rand.uniform(-0.027, 0.027)
        length = rand.uniform(0.13, 0.295) * swaths[i % 4][0]
        height = rand.uniform(0.12, 0.32) * swaths[i % 4][1]
        tall = i % 6 == 1
        if tall:
            height *= 1.32
            length *= 0.60
        width = rand.uniform(0.005, 0.0118) * (1.2 if idx == 1 else 1)
        curve = rand.uniform(-0.47, 0.45)
        phase = rand.uniform(-0.5, 0.5)
        warmth = (
            rand.choice((0.05, 0.10, 0.19, 0.30, 0.50))
            if idx == 2
            else rand.uniform(0, 0.22)
        )
        tint = rand.uniform(0.88, 1.10)
        if i not in keep:
            continue
        d = (math.cos(az), math.sin(az))
        side = (-d[1], d[0])
        last = None
        ts = [(0, 0.19, 0.45, 0.70, 1), (0, 0.34, 0.69, 1), (0, 0.46, 1)][lod]
        for t in ts:
            lumen = math.sin(math.pi * t * 0.83)
            along = (0.17 * t + 1.22 * t * t) * length
            div = length * curve * math.sin(math.pi * t) * 0.26
            px = ox + d[0] * along + side[0] * div + wind_x * length * t**2
            py = oy + d[1] * along + side[1] * div + wind_y * length * t**2
            zz = height * (
                math.sin(math.pi * (0.67 if tall else 0.80) * t)
                + (0.09 if tall else 0) * t
            )
            sx, sy = math.cos(az + math.pi / 2 + phase * t), math.sin(
                az + math.pi / 2 + phase * t
            )
            c = mix(lo, hi, min(1, 0.17 + 1.03 * t))
            c = mix(c, (0.74, 0.61, 0.36), warmth * t)
            c = light(c, tint)
            if t == 1:
                tip = spr.add((px, py, zz), light(mix(c, sun, 0.35), 1.05))
                if len(last) == 3:
                    spr.triangle(last[0], last[1], tip)
                    spr.triangle(last[1], last[2], tip)
                else:
                    spr.triangle(last[0], last[1], tip)
                break
            wd = width * (0.72 + 0.34 * lumen) * (1 - t) ** 0.56
            strip = [-1, 0, 1] if lod == 0 else [-1, 1]
            row = []
            for k in strip:
                crease = 0.40 * wd if k == 0 else 0
                row.append(
                    spr.add(
                        (px + sx * wd * k, py + sy * wd * k, zz + crease),
                        light(c, 1.11 if k == 0 else 0.91 + 0.05 * k),
                    )
                )
            if last is not None:
                for k in range(len(row) - 1):
                    spr.triangle(last[k], last[k + 1], row[k + 1])
                    spr.triangle(last[k], row[k + 1], row[k])
            last = row
    if idx == 2:
        # Only this form carries the few mature fox-tail fruits. Not one
        # floral pompom on every rendered instance of grass.
        for i in range([3, 2, 1][lod]):
            root = nests[(i + 1) % 4]
            dirs = [(-0.3, 0.4), (0.6, 0.15), (0.85, -0.28)]
            dx, dy = dirs[i]
            height = (0.39, 0.46, 0.34)[i]
            size = (0.062, 0.076, 0.052)[i]
            rings = []
            for j, t in enumerate([0, 0.58, 0.84, 1]):
                px = root[0] + dx * 0.13 * t * t
                py = root[1] + dy * 0.11 * t * t
                z = height * (t - 0.13 * t * t)
                rad = (0.0028 if lod == 0 else 0.0035) * (1 - 0.39 * t)
                rings.append(
                    [
                        spr.add(
                            (
                                px + rad * math.cos(k * math.tau / 3),
                                py + rad * math.sin(k * math.tau / 3),
                                z,
                            ),
                            mix(lo, hi, 0.2 + 0.62 * t),
                        )
                        for k in range(3)
                    ]
                )
                if j:
                    for k in range(3):
                        l = (k + 1) % 3
                        spr.triangle(rings[-2][k], rings[-2][l], rings[-1][l])
                        spr.triangle(rings[-2][k], rings[-1][l], rings[-1][k])
            base = (px, py, z)
            axis = (dx * 0.26, dy * 0.26, 0.92)
            count = [5, 4, 3][lod]
            tip = spr.add(
                tuple(base[j] + axis[j] * size for j in range(3)), (0.73, 0.67, 0.41)
            )
            bot = spr.add(base, (0.53, 0.47, 0.26))
            ring = []
            for k in range(count):
                a = k * math.tau / count
                wid = 0.006 + 0.001 * (k % 2)
                ring.append(
                    spr.add(
                        (
                            px + axis[0] * size * 0.48 + math.cos(a) * wid,
                            py + axis[1] * size * 0.48 + math.sin(a) * wid,
                            z + axis[2] * size * (0.43 + 0.07 * (k % 2)),
                        ),
                        mix(
                            (0.49, 0.47, 0.29),
                            (0.78, 0.69, 0.42),
                            0.32 + 0.32 * math.cos(a),
                        ),
                    )
                )
            for k in range(count):
                l = (k + 1) % count
                spr.triangle(bot, ring[l], ring[k])
                spr.triangle(tip, ring[k], ring[l])
    return spr
