"""Pull a cosmetic rock's unsupported underside *inward*, never fake a pillar.

Inside the original frozen x/z/y obstacle: this is a cliff wall keyed into
higher native bedrock where its real height reaches the old sole. Do not
extend geometry down across empty space that a creature could see through.
The full original upper rock remains in place, including the yawed supports.
"""

import math
import numpy as np
from scipy.spatial import cKDTree, ConvexHull
from island_inlet_ground import Inlet, ease


class Foot:
    def __init__(self, surface, prop, source_verts):
        self.terrain = Inlet(surface)
        self.center = np.array(prop["position"])
        self.scale = prop["scale"]
        a = prop["yaw"]
        self.rot = np.array([[math.cos(a), -math.sin(a)], [math.sin(a), math.cos(a)]])
        sole = source_verts[source_verts[:, 1] < 0.02][:, [0, 2]]
        hull = ConvexHull(sole)
        xx, zz = np.meshgrid(
            np.arange(sole[:, 0].min(), sole[:, 0].max() + 0.03, 0.083),
            np.arange(sole[:, 1].min(), sole[:, 1].max() + 0.03, 0.083),
        )
        q = np.stack((xx.ravel(), zz.ravel()), -1)
        inside = np.all(
            q @ hull.equations[:, :2].T + hull.equations[:, 2] <= 1e-5, axis=1
        )
        q = q[inside]
        dh = self.ground(q)
        # Use only origin-footprint locations that actually intersect the
        # bank, with a little burial below the true crossing. No new anchor.
        self.touch = q[(dh > 0.10) & (dh < 2.8)]
        if len(self.touch) < 4:
            self.touch = q[dh > -0.015]
        self.tree = cKDTree(self.touch) if len(self.touch) >= 4 else None
        self.receipt = {
            "available_contact_points": len(self.touch),
            "surface_under_foot_min_m": float(dh.min() * self.scale),
            "surface_under_foot_max_m": float(dh.max() * self.scale),
            "strategy": (
                "pull unsupported toes diagonally into true uphill contact, inside frozen native box; do not bridge air below original floor"
                if self.tree is not None
                else "no supported original contact; withheld"
            ),
        }

    def ground(self, xz):
        xz = np.asarray(xz)
        wxz = (xz * self.scale) @ self.rot + self.center[[0, 2]]
        return (
            self.terrain.floor(wxz[:, 0], wxz[:, 1])[0] - self.center[1]
        ) / self.scale

    def seat(self, points):
        if self.tree is None:
            return points
        p = np.asarray(points).copy()
        xz = p[:, [0, 2]]
        ground = self.ground(xz)
        # A true bed joint, turned, with a different height on each flank.
        # The lowest original horizontal support of this asset is at 3.43m.
        # We are completely back to the unchanged mesh by 3.32m.
        along = 0.28 * xz[:, 0] - 0.23 * xz[:, 1]
        heel = np.clip(
            2.54 + 0.26 * along + 0.17 * np.sin(1.4 * xz[:, 0] - 0.4 * xz[:, 1]),
            1.82,
            3.28,
        )
        # A long slope can legitimately leave a cantilever at the outside.
        # Make its underside a continuous inclined broken plane, not a flat
        # bottom or an invented down-to-ground wall below the saved obstacle.
        t = np.clip((p[:, 1] - 0.06) / heel, 0, 1)
        pull = (1 - (0.65 * t + 0.35 * t * t)) ** 0.96
        pull *= 1 - ease(-0.10, 0.10, ground)
        _, near = self.tree.query(xz, k=min(5, len(self.touch)))
        q = self.touch[near].mean(1)
        p[:, [0, 2]] = xz + (q - xz) * pull[:, None]
        return p
