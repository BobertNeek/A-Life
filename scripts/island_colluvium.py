"""Soil and broken bedrock share a source: the real placed cliff or a rock group.

This offline field follows fall lines from those anchors, then stops at fresh
water, coves and rises. It is not a ring of fake occlusion beneath every prop.
Its strengths drive both LOD pigment and the near normal/roughness atlas.
"""

import numpy as np
from scipy.ndimage import gaussian_filter, map_coordinates
from scipy.spatial import cKDTree

from island_geology_paint import grain, ramp


class Deposit:
    def __init__(self, height, x0, z0, spacing, props):
        self.x0, self.z0, self.step = x0, z0, spacing / 2
        depth, width = height.shape
        # Half-grid for the edges of a source fan. Check height on the true
        # surface below; never displace or infer a new physical terrain point.
        self.strength = np.zeros((2 * depth - 1, 2 * width - 1), dtype="f")
        self.ochre = np.zeros_like(self.strength)
        gz, gx = np.gradient(gaussian_filter(height, 1.4), spacing)
        rocks = [
            p
            for p in props
            if p["kind"] in ("Cliff", "Scree") or p["kind"].startswith("Boulder")
        ]
        xz = np.array([[p["position"][0], p["position"][2]] for p in rocks])
        group = cKDTree(xz)
        d_three = group.query(xz, k=3)[0][:, 2]
        self.count = {"cliff": 0, "boulder": 0, "scree": 0}
        for k, prop in enumerate(rocks):
            x, z = xz[k]
            u, v = np.array([x - x0, z - z0]) / spacing
            sample = lambda field: float(
                map_coordinates(field, [[v], [u]], order=1, mode="nearest")[0]
            )
            dx, dz = -sample(gx), -sample(gz)
            grade = np.hypot(dx, dz)
            alt = sample(height)
            if alt < 0.75 or grade < 0.09:
                continue
            cliff = prop["kind"] == "Cliff"
            small = prop["kind"] == "Scree"
            if not cliff and (
                d_three[k] > (21 if small else 28) or grade < (0.38 if small else 0.20)
            ):
                continue
            self.count["cliff" if cliff else "scree" if small else "boulder"] += 1
            size = prop["scale"]
            nx, nz = dx / grade, dz / grade
            # Cliffs collect a narrow seam of angular earth on their steep side
            # then fan into two irregular toes. Small loose sources make short
            # islands of grit. Their overlap is a max, not repeated darkening.
            length = size * (
                30 + 18 * min(1.25, grade) if cliff else (7.5 if small else 14.5)
            )
            span = size * (9.7 if cliff else (2.2 if small else 4.6))
            radius = length + 1.5 * span
            i0, i1 = max(0, int((x - radius - x0) / self.step)), min(
                self.strength.shape[1], 2 + int((x + radius - x0) / self.step)
            )
            j0, j1 = max(0, int((z - radius - z0) / self.step)), min(
                self.strength.shape[0], 2 + int((z + radius - z0) / self.step)
            )
            a, b = np.meshgrid(
                x0 + np.arange(i0, i1) * self.step, z0 + np.arange(j0, j1) * self.step
            )
            ax, bz = a - x, b - z
            along, across = ax * nx + bz * nz, -ax * nz + bz * nx
            weather = grain(a, b, 14.2, 729)
            gravel = grain(a, b, 4.8, 319)
            reach = along / length
            # Smooth at the anchor, narrower on the up-slope side; the lee widens
            # into a bifurcating fan. Edge irregularity is in metres not texels.
            width_m = span * (0.67 + 0.82 * np.clip(reach, 0, 0.94))
            lateral = across / width_m - (
                0.38 * weather
                + 0.14 * gravel
                + 0.16 * np.sin(prop["yaw"] + reach * 4.5)
            )
            root = np.clip(-along / (span * 1.07), 0, 3) ** 2
            toe = np.maximum(0, reach + 0.14 * weather + 0.052 * gravel)
            outline = 1 - ramp(0.49, 1.13, np.sqrt(root + toe**2 + lateral**2))
            split = 1 - 0.58 * ramp(0.20, 0.82, reach) * (
                1 - ramp(0.05, 0.37, np.abs(lateral - 0.12 * gravel))
            )
            support = map_coordinates(
                height,
                [(b - z0) / spacing, (a - x0) / spacing],
                order=1,
                mode="nearest",
            )
            slope_stop = 1 - ramp(
                0.55, 2.5 + grade * 1.4, (support - alt) - np.maximum(0, -along) * grade
            )
            weight = (
                outline
                * split
                * slope_stop
                * ramp(0.1, 1.25, support)
                * (0.89 if cliff else 0.77)
                * (0.94 + 0.06 * gravel)
            )
            s = self.strength[j0:j1, i0:i1]
            m = self.ochre[j0:j1, i0:i1]
            adopt = weight > s
            source_tone = (
                0.22 if prop["kind"] == "BoulderB" else 0.73 if cliff else 0.47
            )
            # Correlated warm shale weathering and cool fresh fragments. No
            # painted sunny/dark side, so the actual island sun remains in charge.
            m[adopt] = np.clip(
                source_tone + 0.27 * weather[adopt] + 0.09 * gravel[adopt], 0, 1
            )
            s[adopt] = weight[adopt]
        self.ochre *= self.strength

    def at(self, x, z):
        u, v = np.broadcast_arrays((x - self.x0) / self.step, (z - self.z0) / self.step)
        return tuple(
            map_coordinates(a, [v, u], order=1, mode="nearest", prefilter=False)
            for a in (self.strength, self.ochre)
        )

    def mix(self, old, x, z, reference=1):
        w, warm = self.at(x, z)
        # Linear-light geologic endpoints. Store the mineral tone divided by
        # the near tile reference if this is the L0 per-corner multiplier.
        # Signed weights are avoided at a boundary by interpolating premultiplied
        # mineral tones, not a categorical nearest-source colour.
        cool = np.array((0.217, 0.258, 0.274))
        buff = np.array((0.365, 0.278, 0.197))
        return (
            old * (1 - w[..., None])
            + (w[..., None] * cool + warm[..., None] * (buff - cool)) / reference
        )

    def receipt(self):
        return dict(
            sources=self.count,
            lattice_metres=self.step,
            strong_m2=round(float((self.strength > 0.38).sum() * self.step**2)),
            affected_m2=round(float((self.strength > 0.055).sum() * self.step**2)),
            max_mix=round(float(self.strength.max()), 4),
        )
