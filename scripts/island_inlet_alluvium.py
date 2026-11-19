"""A few finite gravel lenses on the exposed banks, never contour rings.

Lenses have an upstream/downstream axis, a gentle local dip and a pinched
end in the native bank section. A face can plausibly expose laminations
where another face a few metres away shows only undisturbed material.
"""

import numpy as np
from scipy.ndimage import gaussian_filter


def alluvium(xx, zz, height, gx, grade, fine, vein, rng):
    # Four pockets on each bank with bare/broken clay in between.
    # Each description is (shore, along-channel centre, half-span, height,
    # half-thickness, dip, rust vs washed sand). All distances in metres.
    lenses = [
        (-1, 402, 7.8, 4.0, 0.42, 0.08, 0.33),
        (-1, 413, 10.5, 3.0, 0.65, -0.09, 0.68),
        (-1, 426, 6.5, 6.3, 0.44, -0.03, 0.18),
        (-1, 434, 8.6, 2.3, 0.72, 0.09, 0.92),
        (1, 401, 6.6, 5.4, 0.45, -0.06, 0.24),
        (1, 416, 8.3, 2.5, 0.51, 0.11, 0.86),
        (1, 428, 9.5, 4.2, 0.82, -0.13, 0.42),
        (1, 439, 5.6, 2.4, 0.40, -0.09, 0.80),
    ]
    # On the west side -gx is positive; resolving slope, rather than a fixed
    # X split, means the northwest shore cannot receive northeast strata.
    weight = np.zeros_like(xx)
    tint = np.zeros((*xx.shape, 3))
    lift = weight.copy()
    for side, z, length, high, thickness, dip, warm in lenses:
        along = (zz - z) / length
        pinch = np.maximum(0, 1 - along**2)
        profile = (
            high + dip * (zz - z) + 0.30 * (along * along) + 0.17 * vein + 0.045 * fine
        )
        signed = (height - profile) / np.maximum(0.12, thickness * pinch**0.60)
        val = (
            np.exp(-2.0 * signed**4) * pinch**0.75 * (side * gx > 0.42) * (grade > 0.55)
        )
        val *= np.clip(0.77 + 0.23 * vein + 0.12 * fine, 0.14, 1)
        # Within a finite window, laminae drift together; no repeated, global
        # parallel noise or baked directional light on the entire wall.
        beds = 0.5 + 0.5 * np.sin(12 * signed + 0.7 * along + 0.60 * fine)
        ink = np.array([119.0, 115.0, 103.0]) + warm * np.array([22.0, 9.0, -6.0])
        color = ink + (11 * beds + 2.1 * fine)[..., None]
        tint += val[..., None] * color
        weight += val
        lift += val * (0.014 + 0.013 * beds)
    coat = np.minimum(weight * 0.88, 0.9)
    color = tint / np.maximum(1e-8, weight)[..., None]
    return coat, color, gaussian_filter(lift, 0.45)
