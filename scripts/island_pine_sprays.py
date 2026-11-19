"""Bent evergreen boughs, not tree-facing billboards or closed leaf paddles.

Each woody axis bears forked sprays in different planes. The view through
their tips is the actual negative space between drawn twigs in the needle
atlas. Middle-distance boughs preserve the same asymmetric endpoints.
"""

import math
import numpy as np
from island_needle_surface import uv


def emit(out, b, lod, axis):
    r = np.random.default_rng(b["seed"] + 9045)
    right = b["side"]
    direction = b["d"]
    up = np.array((0.0, 1.0, 0.0))
    length = b["reach"]
    tint = b["tint"]
    alt = b["seed"] % 2

    def wood_uv(t, q):
        return (0.02 + 0.033 * t, 0.931 + 0.013 * q + 0.010 * t)

    # Actual woody armature works from any view, including looking uphill
    # into the underside. It never retrofits the frozen solid trunk.
    shaft = [axis(b, t) for t in (0, 0.47, 1.04)]
    wood = np.array((0.13, 0.097, 0.063))
    root = out.vertex(shaft[0], wood * 0.75, wood_uv(0, 0))
    end = out.vertex(shaft[2], wood * 0.92, wood_uv(1, 0))
    width = max(0.016, 0.035 * length)
    angles = (
        ((0, -0.8), (2.095, -0.04), (4.19, 0.8))
        if lod == 0
        else ((0, -0.8), (math.pi, 0.8))
    )
    ring = [
        out.vertex(
            shaft[1] + width * (right * math.cos(a) + up * math.sin(a)),
            wood * (0.92 + 0.09 * math.sin(a)),
            wood_uv(0.49, q),
        )
        for a, q in angles
    ]
    if lod == 0:
        for i in range(3):
            out.tri(root, ring[(i + 1) % 3], ring[i])
            out.tri(ring[i], ring[(i + 1) % 3], end)
    else:
        out.tri(root, ring[1], ring[0])
        out.tri(ring[0], ring[1], end)

    def plume(t0, reach, width, side_angle, hang, roll, seed, wide=False):
        rr = np.random.default_rng(seed)
        org = axis(b, t0)
        d = direction * math.cos(side_angle) + right * math.sin(side_angle)
        side = np.array((-d[2], 0, d[0]))
        # Look at any one limb and it is several overlapping banks rather
        # than one smooth island of leaf pigment. Its top and sides twist.
        bounds = (
            ((0, 0.20, 0.45, 0.72, 0.90, 1) if t0 < 0.1 else (0, 0.29, 0.58, 0.81, 1))
            if lod == 0
            else (0, 0.47, 1)
        )
        offset = rr.uniform(-0.14, 0.14)
        vert = rr.uniform(0.10, 0.21) * reach
        col = tint * (0.95 + rr.uniform(-0.11, 0.07)) + (
            np.array((0.010, 0.014, -0.002)) if rr.random() < 0.25 else 0
        )
        rows = []
        for t in bounds:
            bend = t + offset * math.sin(t * math.pi)
            p = (
                org
                + d * reach * bend
                + right * offset * reach * 0.23 * math.sin(t * math.pi)
            )
            p += up * (vert * math.sin(math.pi * t) - hang * (0.23 * t + 0.77 * t * t))
            # Each side falls out and away from the central twig, in
            # planes set by the bough, never the camera.
            breadth = width * (
                0.16 + 0.84 * math.sin(math.pi * (0.10 + 0.80 * t)) ** 0.72
            )
            c = math.cos(roll + 0.24 * math.sin(t * 4 + seed % 7))
            s = math.sin(roll + 0.24 * math.sin(t * 4 + seed % 7))
            lateral = side * c + up * s
            ridge = 0.17 * width * math.sin(math.pi * t)
            rows.append(
                (
                    out.vertex(
                        p - lateral * breadth - up * (0.21 * breadth),
                        col * (0.92 + 0.11 * t),
                        uv(t, -1, seed % 2),
                    ),
                    out.vertex(
                        p + up * ridge, col * (1.05 + 0.12 * t), uv(t, 0.025, seed % 2)
                    ),
                    out.vertex(
                        p + lateral * breadth - up * (0.37 * breadth),
                        col * (0.92 + 0.15 * t),
                        uv(t, 1, seed % 2),
                    ),
                )
            )
        for a, c in zip(rows[:-1], rows[1:]):
            out.quad(a[0], c[0], c[1], a[1])
            out.quad(a[1], c[1], c[2], a[2])

    # The long central stem supports its own three-dimensional spray, plus
    # unequal lateral and pendulous banks. These are physical twig sheets
    # from a shared morphology, not repeated viewer-facing crossed cards.
    breadth = b["width"]
    plume(
        0.035,
        length * 1.05,
        breadth * 0.84,
        0,
        b["drop"] * 0.93,
        0.17 * math.sin(b["seed"]),
        b["seed"] + 71,
    )
    if lod == 0:
        arms = (
            (0.10, -1, 0.74, 0.55, 0.86, -0.44),
            (0.21, 1, 0.75, 0.57, 1.0, 0.70),
            (0.38, -1, 0.63, 0.74, 0.84, 0.58),
            (0.49, 1, 0.53, 0.81, 0.92, -0.48),
            (0.70, -1, 0.45, 0.79, 0.73, -0.54),
            (0.76, 1, 0.39, 0.84, 0.71, 0.40),
        )
    else:
        arms = ((0.13, -1, 0.79, 0.60, 0.96, -0.50), (0.27, 1, 0.77, 0.64, 1.10, 0.64))
    for j, (t, sign, k, a, down, roll) in enumerate(arms):
        plume(
            t,
            length * k,
            breadth * (0.73 if lod == 0 else 0.94) * (1 - 0.35 * t),
            sign * a,
            b["drop"] * down + length * 0.18,
            roll,
            b["seed"] + 123 + j,
        )
