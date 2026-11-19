"""Aperiodic exposed rock on two genuinely independent surface directions.

On the low eastern shoulder the previous pair of coordinates became nearly
parallel: changes in height and in one horizontal direction describe almost the
same motion on that slope. Its fault cells collapsed into contour lines. Here
the two main coordinates form an invertible rotation in the ground plane.
"""

import numpy as np
from island_geology_paint import grain, fault_cells, pigment, rgb, ramp


def exposed(x, z, h, g, knot, brush, broken, grit):
    broad = grain(x + 5.5 * knot, z, 48, 6125)
    bend = grain(x, z + 3.7 * knot, 24, 9924)
    strike = 0.83 * x + 0.55 * z
    cross = -0.55 * x + 0.83 * z
    # Small height contribution on ONE coordinate only, never a height strip.
    u = (strike + 3.1 * bend + 2.9 * knot + 0.035 * h) / 13.2
    v = (cross + 3.22 * broad + 2.0 * brush) / 7.9
    seam, facet = fault_cells(u, v)
    fresh = ramp(
        -0.13, 0.66, 0.53 * broad - 0.35 * knot + 0.31 * bend + 0.23 * (g - 0.5)
    )
    panel = ramp(-0.01, 0.63, 0.48 * bend + 0.35 * brush + 0.34 * fresh)
    # Weather breaks a fracture along its length. A uniformly traced Voronoi
    # edge looks like a giant flagstone patio even if its direction is correct.
    erosion = ramp(
        -0.04, 0.52, 0.45 * grain(strike, cross, 5.8, 1186) + 0.36 * brush + 0.25 * bend
    )
    joints = (1 - ramp(0.019, 0.106, seam)) * panel * (0.13 + 0.87 * erosion)
    fresh_chip = (
        ramp(0.05, 0.18, seam)
        * (1 - ramp(0.24, 0.42, seam))
        * ramp(0.23, 0.75, bend + 0.24 * facet)
        * (0.08 + 0.92 * erosion)
    )
    # Extended slate fans join a limited number of warm, weathered outcrops.
    # Broken quartz and lichen stop at panel fractures; no tiled brick pattern.
    iron = ramp(-0.52, 0.63, 0.54 * knot + 0.48 * bend - 0.35 * broad + 0.09 * facet)
    cliff = pigment(rgb(116, 137, 146), rgb(173, 151, 122), iron)
    patina = ramp(0.18, 0.74, 0.55 * brush + 0.49 * broad - 0.29 * bend) * (1 - joints)
    cliff = pigment(cliff, rgb(152, 160, 126), patina * 0.44)
    cliff = pigment(cliff, rgb(203, 192, 163), fresh_chip * (0.27 + 0.23 * fresh))
    cliff *= (
        1
        + 0.060 * facet * (0.35 + 0.65 * fresh)
        + 0.054 * broken
        + 0.023 * grit
        - 0.145 * joints
    )[..., None]
    relief = (
        0.13 * broad
        + 0.16 * bend
        + 0.10 * brush
        + 0.085 * facet * (0.35 + 0.65 * fresh)
        - 0.31 * joints
        + 0.07 * fresh_chip
        + 0.065 * broken
        + 0.025 * grit
    )
    show = ramp(
        0.30,
        0.81,
        0.42 * broad - 0.33 * knot + 0.35 * bend + 0.15 * brush + 0.22 * (g - 0.5),
    )
    return cliff, relief, show


class CornerCorrection:
    """Exact integer albedo difference between recipes at actual source vertices.

    Each indexed L0 chunk (including skirts) already carries the old linear
    macro colour divided by its tile's mean albedo. Apply only the change in
    that quantity: low land and any unaffected pixel remain byte-identical.
    """

    def __init__(self, terrain, near_reference, width=2048):
        from PIL import Image
        from island_habitat_massing import linear

        depth, cols = terrain.h.shape
        i, j = np.meshgrid(np.arange(cols), np.arange(depth))
        u = np.clip((i / (cols - 1) * width).astype(int), 0, width - 1)
        v = np.clip((j / (depth - 1) * width).astype(int), 0, width - 1)
        x = terrain.x + (u.astype("f") + 0.5) * terrain.size[0] / width
        z = terrain.z + (v.astype("f") + 0.5) * terrain.size[1] / width
        ref = linear(
            np.asarray(Image.open(near_reference).convert("RGB"), dtype=float) / 255
        ).mean(axis=(0, 1))
        # No mutation of authoritative samples. Both recipes read the same two
        # arrays and may only change the returned pigment/relief.
        from copy import copy

        base = copy(terrain)
        base.fractured_rock = False
        old = base.paint(x, z)[0]
        new = terrain.paint(x, z)[0]
        read = lambda rgb: np.rint(
            255 * np.clip(linear(rgb.astype(float) / 255) / ref, 0, 1)
        ).astype("i2")
        self.delta = read(new) - read(old)
        self.x0, self.z0, self.step = terrain.x, terrain.z, terrain.spacing

    def at(self, x, z):
        u = np.rint((x - self.x0) / self.step).astype(int)
        v = np.rint((z - self.z0) / self.step).astype(int)
        return self.delta[v, u]
