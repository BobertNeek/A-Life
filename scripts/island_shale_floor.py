"""Centimetre-scale stone flakes, fines and intermittent bedrock on real talus."""

import math
import numpy as np
from island_geology_paint import ramp


def floor(r, x, y, field, brush, grain, fleck, seam, face):
    n = x.shape[0]
    bare = ramp(-0.24, 0.52, 0.65 * field + 0.49 * brush)
    exposed = ramp(0.13, 0.73, 0.52 * field + 0.36 * grain + 0.30 * brush)
    joint = (1 - ramp(0.015, 0.106, seam)) * (0.08 + 0.71 * exposed)
    lip = ramp(0.04, 0.15, seam) * (1 - ramp(0.17, 0.32, seam)) * exposed
    rgb = (
        np.array((203, 206, 206))
        + (
            15 * face * (0.27 + 0.73 * bare)
            + 3.3 * grain
            + 7.4 * fleck
            + 7.2 * lip
            - 19 * joint
        )[..., None]
    )
    rgb += (1 - bare)[..., None] * np.array((9.0, 0.0, -9.0))
    h = 0.0042 * face * bare - 0.008 * joint + 0.0025 * lip + 0.0018 * fleck
    occ = 1 - 0.17 * joint
    rough = 0.76 + 0.08 * (1 - bare) + 0.055 * grain
    # Deposits collect in patches, not an even all-over print: a large piece,
    # two broken neighbours and an irregular outer scatter. Each cut plate
    # has a planar top, one bevel and a grounded rim shared by PBR channels.
    for _ in range(64):
        ox, oy = r.uniform(0, n, 2)
        if bare[int(oy), int(ox)] > 0.91 and r.random() < 0.8:
            continue
        heading = r.uniform(-math.pi, math.pi)
        for j in range(r.integers(5, 15)):
            radius = r.uniform(2.3, 5.4) if j < 2 else r.uniform(0.65, 2.75)
            cx, cy = ox + r.normal(0, 7.4), oy + r.normal(0, 6.2)
            angle = heading + r.normal(0, 0.71)
            width = radius * r.uniform(0.34, 0.75)
            cap = math.ceil(radius * 1.6 + 2)
            yy, xx = np.mgrid[-cap : cap + 1, -cap : cap + 1]
            uu = (xx * math.cos(angle) + yy * math.sin(angle)) / radius
            vv = (-xx * math.sin(angle) + yy * math.cos(angle)) / width
            edge = np.maximum.reduce(
                (
                    np.abs(vv),
                    np.abs(uu - 0.17 * vv),
                    0.69 * np.abs(uu + vv),
                    0.75 * np.abs(uu - 0.60 * vv),
                )
            )
            mask = 1 - ramp(0.75, 1.13, edge)
            where = np.ix_(
                (round(cy) + np.arange(-cap, cap + 1)) % n,
                (round(cx) + np.arange(-cap, cap + 1)) % n,
            )
            mineral = r.choice(
                [
                    (216, 218, 212),
                    (176, 186, 191),
                    (190, 187, 175),
                    (225, 214, 191),
                    (202, 209, 208),
                ]
            )
            top = np.asarray(mineral) + (5 * uu - 8 * ramp(0.42, 0.94, edge))[..., None]
            rgb[where] += mask[..., None] * (top - rgb[where])
            h[where] += r.uniform(0.0029, 0.0093) * mask * (1 + 0.21 * uu)
            occ[where] *= 1 - 0.10 * np.exp(-(((edge - 1.03) / 0.18) ** 2))
            rough[where] += mask * (r.uniform(0.61, 0.79) - rough[where])
    return rgb, h, occ, rough
