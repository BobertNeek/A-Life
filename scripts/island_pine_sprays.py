"""Split spruce sprays with real negative notches (no alpha billboards).

Near observers see a woody axis bearing *separate banks* of short lateral
shoots, rather than giant solid leaves hanging off an otherwise correct tree.
"""

import math
import numpy as np
from island_needle_surface import uv


def emit(out, b, lod, axis):
    rng = np.random.default_rng(b["seed"] + 6691)
    side = b["side"]
    d = b["d"]
    reach = b["reach"]
    width = b["width"]
    up = np.array((0.0, 1.0, 0.0))
    alternate = bool(b["seed"] % 2)
    color = b["tint"]
    wood = np.array((0.103, 0.082, 0.048))

    def vert(pos, c, t, cross):
        return out.vertex(pos, c, uv(t, cross, alternate))

    # Narrow real woody centre, not the former fat faceted envelope.
    p0 = axis(b, 0)
    mid = axis(b, 0.45)
    tip = axis(b, 1.01)
    r = max(0.024, 0.090 * width)
    root = vert(p0, wood * 0.81, 0, 0)
    end = vert(tip, color * 1.16, 1, 0)
    section = [
        vert(mid + side * r + up * r * 0.2, wood, 0.45, 1),
        vert(mid - side * r + up * r * 0.2, wood * 0.98, 0.45, -1),
    ]
    if lod == 0:
        section.append(vert(mid - up * r * 0.62, wood * 0.86, 0.44, -0.04))
        for i in range(3):
            out.tri(root, section[(i + 1) % 3], section[i])
            out.tri(section[i], section[(i + 1) % 3], end)
    else:
        out.tri(root, section[1], section[0])
        out.tri(section[0], section[1], end)
    count = 6
    for j in range(count):
        for hand in (-1, 1):
            t = (
                0.055
                + (j + 0.19 * (hand == 1)) * (0.79 / (count - 1))
                + rng.uniform(-0.025, 0.023)
            )
            org = axis(b, t)
            envelope = (math.sin(math.pi * (0.065 + 0.89 * t)) ** 0.52) * (1 - 0.24 * t)
            w = (
                width
                * (1.40 if lod == 0 else 1.60)
                * envelope
                * rng.uniform(0.81, 1.09)
            )
            along = reach * (0.15 + 0.10 * (1 - t) + rng.uniform(-0.035, 0.045))
            tilt = reach * ((0.19 + 0.15 * (1 - t)) * rng.uniform(0.69, 1.19))
            u = hand * side * w + d * along - up * tilt
            # A twisted lateral shoulder dipping in a separate plane; left and
            # right sub-sprays have distinct start heights and stem lengths.
            transverse = d * 0.83 - hand * side * 0.42
            half = max(
                0.035, reach * (0.044 + 0.044 * (1 - t)) * (1.09 if lod == 0 else 1.76)
            )
            turn = rng.uniform(-0.22, 0.22)

            if lod == 1 and j not in (0, 2, 3, 5):
                continue

            def point(s, across=0, raised=0):
                return (
                    org
                    + u * s
                    + transverse * (half * (across + turn * math.sin(s * math.pi)))
                    + up * (raised + reach * 0.083 * math.sin(s * math.pi))
                )

            root = vert(point(0), color * (0.64 + 0.09 * t), 0.012, 0)
            left = vert(point(0.39, 1, 0.025 * reach), color * 0.85, 0.39, 1)
            right = vert(point(0.35, -0.90, 0.004 * reach), color * 0.72, 0.35, -1)
            if lod == 1:
                distal = vert(point(1, 0.01, -0.09 * reach), color * 1.16, 0.99, 0)
                out.tri(root, left, right)
                out.tri(left, distal, right)
                continue
            # Three separately tapering dark/light needle tufts terminating at
            # different lengths. Two deep V slots are silhouette, not pigment.
            m = vert(point(0.49, 0.02, 0.067 * reach), color * 1.13, 0.49, 0.025)
            a = vert(point(0.89, 1.24, -0.036 * reach), color * 1.06, 0.88, 0.95)
            anotch = vert(point(0.59, 0.43, -0.007 * reach), color * 0.81, 0.60, 0.46)
            central = vert(point(1.12, 0.10, -0.066 * reach), color * 1.24, 1.0, 0.034)
            bnotch = vert(point(0.61, -0.48, -0.018 * reach), color * 0.75, 0.62, -0.46)
            e = vert(point(0.91, -1.16, -0.070 * reach), color * 1.12, 0.89, -0.96)
            for poly in (
                (root, left, m),
                (root, m, right),
                (left, a, anotch),
                (left, anotch, m),
                (m, anotch, central),
                (m, central, bnotch),
                (m, bnotch, right),
                (right, bnotch, e),
            ):
                # The opposite bank is a physical reflection.
                out.tri(*(poly if hand < 0 else tuple(reversed(poly))))
