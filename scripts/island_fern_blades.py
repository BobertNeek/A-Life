"""Curved, folded fern blades, grown outward from one grounded crown."""

import math
from random import Random
from mathutils import Vector


def leaves(level):
    rng = Random(993871)
    verts = []
    uv = []
    faces = []
    n = (9, 7, 4)[level]
    across = ((-1, -0.54, 0, 0.54, 1), (-1, 0, 1), (-1, 0, 1))[level]
    # Long low fronds surround a few ascending spring leaves. The one low
    # opening keeps the plant's interior legible in the original profile.
    profiles = [
        (0.53, 0.40, 0.13),
        (0.39, 0.59, 0.14),
        (0.23, 0.76, 0.08),
        (0.48, 0.39, 0.17),
        (0.42, 0.54, 0.13),
        (0.26, 0.65, 0.10),
        (0.50, 0.43, 0.15),
        (0.31, 0.49, 0.12),
        (0.37, 0.54, 0.16),
    ]
    for j, (radius, high, droop) in enumerate(profiles):
        a = j * 2.399963 + rng.uniform(-0.19, 0.19)
        radial = Vector((math.cos(a), math.sin(a), 0))
        side = Vector((-radial.y, radial.x, 0))
        z = Vector((0, 0, 1))
        lean = rng.uniform(-0.065, 0.065)
        tilt = rng.uniform(-0.50, 0.50)
        spin = rng.uniform(-0.44, 0.44)
        curl = rng.uniform(-0.18, 0.18)
        width = (0.108 + 0.025 * math.sin(j * 1.76)) * (1 + 0.09 * level)
        for i in range(n + 1):
            t = i / n
            # A frond arches through a broad shoulder rather than terminating
            # at the top of a straight stick; each side rolls toward the soil.
            axis = (
                radial * (0.015 + radius * (0.22 * t + 0.78 * t**1.8))
                + side * (lean * math.sin(math.pi * t) + curl * 0.16 * t * t)
                + z * (high * (1 - (1 - t) ** 2.22) - droop * t**3)
            )
            roll = tilt + spin * math.sin(t * math.pi)
            cross = side * math.cos(roll) + z * math.sin(roll)
            for q in across:
                width_t = width * (0.10 + 0.90 * min(1, t / 0.26) ** 1.2)
                p = (
                    axis
                    + cross * (q * width_t)
                    + radial * (0.12 * q * q * width_t)
                    + z
                    * (-0.24 * q * q * width_t * (0.25 + 0.75 * math.sin(math.pi * t)))
                )
                verts.append(p)
                uv.append(((j % 4 + (q + 1) / 2) * 0.25, 0.008 + 0.984 * t))
            if i:
                last = len(verts) - len(across) * 2
                for k in range(len(across) - 1):
                    p0, p1 = last + k, last + k + len(across)
                    faces.extend([(p0, p1, p1 + 1), (p0, p1 + 1, p0 + 1)])
    return verts, uv, faces
