"""Seven modelled, divided woodland fern fronds; Blender coordinates (Z up).

Source-only component for the next habitat art board. The former island fern
had continuous ribs but inherited small folded umbrella tops. Here pinnae run
along their own leaf axis and each leaflet connects to the raised midrib.
"""

import math
from random import Random

from mathutils import Vector


def linear(rgb):
    def decode(c):
        v = c / 255
        return v / 12.92 if v < 0.04045 else ((v + 0.055) / 1.055) ** 2.4

    return tuple(decode(v) for v in rgb)


class Fern:
    def __init__(self):
        self.v = []
        self.rgb = []
        self.faces = []

    def add(self, p, c):
        self.v.append(Vector(p))
        self.rgb.append(linear(c))
        return len(self.v) - 1

    def face(self, *ids):
        a, b, c = (self.v[q] for q in ids)
        self.faces.append(tuple(ids if (b - a).cross(c - a).z >= 0 else reversed(ids)))


def levels(level):
    rng = Random(993871)
    plant = Fern()
    for j in range(7):
        a = j * 2.399963 + rng.uniform(-0.17, 0.17)
        rad = Vector((math.cos(a), math.sin(a), 0))
        cross = Vector((-rad.y, rad.x, 0))
        base = rad * rng.uniform(0.005, 0.025)
        reach = (0.27 + 0.225 * (j % 3) / 2) * rng.uniform(0.91, 1.03)
        rise = (0.43 + 0.29 * ((j * 2 + 1) % 5) / 4) * rng.uniform(0.95, 1.04)
        sweep = rng.uniform(-0.10, 0.10)
        skew = rng.uniform(-0.16, 0.13)
        leafgreen = rng.choice(
            ((102, 151, 82), (100, 147, 91), (113, 155, 78), (98, 145, 82))
        )

        def color(k, russet=0):
            return (
                max(8, min(248, leafgreen[0] * k + russet)),
                min(249, leafgreen[1] * k),
                min(243, leafgreen[2] * k),
            )

        def mid(t):
            # Fresh xylem rises steeply, then the last third nods outward.
            return (
                base
                + rad * (reach * (0.13 * t + 0.87 * t * t))
                + cross * (sweep * math.sin(math.pi * t))
                + Vector((0, 0, rise * (2.08 * t - 1.24 * t * t) + skew * t**3))
            )

        rungs = (
            (0, 0.16, 0.26, 0.36, 0.46, 0.56, 0.66, 0.76, 0.85, 0.93, 1)
            if level == 0
            else (
                (0, 0.18, 0.34, 0.52, 0.69, 0.86, 1)
                if level == 1
                else (0, 0.30, 0.58, 0.81, 1)
            )
        )
        rib = []
        for k, t in enumerate(rungs):
            wid = 0.0077 * (1 - 0.83 * t) * (1.0 + 0.15 * level)
            stemcolor = color(0.71 + 0.38 * t, 2)
            rib.append(
                [plant.add(mid(t) + cross * wid * sign, stemcolor) for sign in (-1, 1)]
            )
        for l, h in zip(rib, rib[1:]):
            plant.face(l[0], l[1], h[1])
            plant.face(l[0], h[1], h[0])
        for k, t in enumerate(rungs[1:-1], 1):
            # Hold the crotch free rather than attaching a comblike blade
            # all the way to the soil. The nearest serrations curl down.
            if (level == 0 and j % 3 == 0 and k in (1, 8)) or (
                level == 2 and j % 2 == 1 and k == 1
            ):
                continue
            local = Random(41987 + j * 1741 + round(t * 100) * 17)
            env = math.sin(math.pi * (0.08 + t * 0.87)) ** 0.73 * (1 - 0.60 * t)
            length = (0.078 + 0.064 * (j % 3) / 2) * env
            for side in (-1, 1):
                length_s = length * local.uniform(0.84, 1.09)
                attach = rib[k][int(side > 0)]
                stem = mid(t)
                blade = cross * (side * length_s) + rad * (0.39 * length_s)
                blade.z = length_s * (0.23 - 0.51 * t) + skew * 0.03
                tangent = (mid(min(1, t + 0.01)) - mid(max(0, t - 0.01))).normalized()
                width = length_s * (
                    0.151 if level == 0 else 0.185 if level == 1 else 0.233
                )
                ridge = stem + blade * 0.47 + Vector((0, 0, 0.073 * length_s))
                lo = plant.add(
                    stem + blade * 0.28 - tangent * width, color(0.87 + 0.10 * t)
                )
                hi = plant.add(
                    stem + blade * 0.51 + tangent * width * 0.91, color(0.98 + 0.07 * t)
                )
                tip = plant.add(
                    stem + blade + rad * 0.035 * length_s, color(1.01 + 0.10 * t)
                )
                if level == 2 or (level == 1 and (j + k) % 4 != 0):
                    plant.face(attach, lo, tip)
                    plant.face(attach, tip, hi)
                    continue
                c = plant.add(ridge, color(1.07 + 0.09 * t, 2))
                if level == 0 and t < 0.67 and j % 2 == 0:
                    tooth = plant.add(
                        stem + blade * 0.57 - tangent * width * 1.01,
                        color(0.92 + 0.09 * t),
                    )
                    plant.face(attach, lo, c)
                    plant.face(c, lo, tooth)
                    plant.face(c, tooth, tip)
                else:
                    plant.face(attach, lo, c)
                    plant.face(c, lo, tip)
                plant.face(attach, c, hi)
                plant.face(c, tip, hi)
        # A slender unfurled terminal lance is continuous with both sides of
        # the existing rachis. Do not finish every frond in a broad hood.
        stem = mid(rungs[-2])
        wid = 0.017 if level < 2 else 0.023
        sides = [
            plant.add(stem + rad * 0.015 + cross * q * wid, color(1.05))
            for q in (-1, 1)
        ]
        plant.face(rib[-2][0], sides[0], rib[-1][0])
        plant.face(rib[-2][1], rib[-1][1], sides[1])
    return plant
