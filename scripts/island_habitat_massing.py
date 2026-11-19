"""A common colour field for sun meadows, cool coppices and their real gaps.

Trees, trails and rocks never move. Centred smoothing makes a group of trunks
read as one sheltered floor instead of stamping an identical disc per tree.
"""

import numpy as np
from scipy.ndimage import gaussian_filter, map_coordinates

from island_geology_paint import grain, ramp


class Cover:
    def __init__(self, h, x0, z0, step, stems):
        self.x0, self.z0, self.step = x0, z0, step
        j, i = np.indices(h.shape, dtype="f")
        x, z = x0 + i * step, z0 + j * step
        gz, gx = np.gradient(gaussian_filter(h, 1.2), step)
        grade = np.hypot(gz, gx)
        broad = grain(x + 8 * grain(x, z, 34, 135), z, 38, 827)
        ribbons = grain(x * 0.84 + z * 0.52, z * 0.87 - x * 0.91, 16, 477)
        # Two widths combine real nearest rooted stands with longer coppice mass.
        crowns = (
            0.65 * gaussian_filter(stems, 3.15) * 98
            + 0.35 * gaussian_filter(stems, 6.8) * 98
        )
        grove = ramp(0.50, 1.48, crowns + 0.14 * broad + 0.10 * ribbons)
        shoulder = (gx * 0.74 - gz * 0.25) / (0.3 + grade)
        opening = ramp(
            -0.54, 0.59, 0.60 * broad + 0.26 * ribbons + 0.28 * shoulder + 0.12
        ) * (1 - 0.93 * grove)
        meadow = (
            ramp(2.5, 6.5, h) * (1 - ramp(65, 128, h)) * (1 - ramp(0.41, 0.95, grade))
        )
        # Linear-light factors, not painted shadows. Deep leaf loam joins real
        # stems into masses; the sheltered glades keep a honey-green open side.
        self.data = 1 + meadow[..., None] * (
            grove[..., None] * np.array([-0.29, -0.34, -0.36])
            + opening[..., None] * np.array([0.24, 0.093, -0.19])
        )
        self.data = np.clip(self.data, 0.64, 1.24).astype("f")

    def multiplier(self, x, z):
        u, v = np.broadcast_arrays((x - self.x0) / self.step, (z - self.z0) / self.step)
        return np.stack(
            [
                map_coordinates(self.data[..., k], [v, u], order=1, mode="nearest")
                for k in range(3)
            ],
            axis=-1,
        )


def linear(v):
    return np.where(v <= 0.04045, v / 12.92, ((v + 0.055) / 1.055) ** 2.4)


def srgb(v):
    return np.where(
        v <= 0.0031308, v * 12.92, 1.055 * np.maximum(v, 0) ** (1 / 2.4) - 0.055
    )
