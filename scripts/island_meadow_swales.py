"""Irregular windward stands around the *existing* wild valley plants.

The two nursery sites and the player's cultivated meadow are farther north.
This is an original, compact visual-support field, not a new vegetation
population, continuous planted border, terrain replacement or gameplay crop.
"""

from pathlib import Path
import json, math
import numpy as np
from island_inlet_ground import Inlet, ease

# Multiple overlapping fragments follow planted areas; the open drift between
# them is left alone. There is no global Perlin carpet or generic tree ring.
STANDS = [
    (66, 294, 8.5, 2.0, 0.65),
    (70, 297, 9.8, 2.4, 0.33),
    (74, 302, 6.2, 2.5, 1.05),
    (65, 306.5, 6.7, 2.0, -0.16),
    (79, 307.8, 7.7, 2.4, 0.17),
    (83, 311.4, 5.5, 1.75, 0.48),
    (72.5, 287.8, 6.3, 1.5, 0.03),
    (61.5, 293, 4.3, 1.75, 1.38),
    (75.8, 315.0, 5.1, 2.2, 0.84),
]
BOUNDS = (46.875, 100, 275.0, 328.125)


class Meadow:
    def __init__(self, surface, variant, placements):
        self.g = Inlet(surface)
        self.v = Inlet(variant)
        self.roots = np.array(
            [
                [p["position"][0], p["position"][2]]
                for p in json.loads(Path(placements).read_text())
                if p["kind"] in ("Grass", "Flowers")
                and 43 < p["position"][0] < 103
                and 273 < p["position"][2] < 331
            ]
        )

    def field(self, x, z):
        x = np.asarray(x)
        z = np.asarray(z)
        odds = np.ones_like(x + z, dtype=float)
        for i, (cx, cz, ln, wide, az) in enumerate(STANDS):
            dx = x - cx
            dz = z - cz
            u = (dx * math.cos(az) + dz * math.sin(az)) / ln
            v = (-dx * math.sin(az) + dz * math.cos(az)) / wide
            curved = v - 0.29 * np.sin(3.2 * u + i * 0.61) - 0.21 * u * u
            odds *= 1 - 0.94 * np.exp(-(1.75 * u * u + 1.18 * curved * curved))
        grass = 1 - odds
        close = np.zeros_like(grass)
        for a, b in self.roots:
            close = np.maximum(close, np.exp(-((x - a) ** 2 + (z - b) ** 2) / 21.0))
        soil, dx, dz = self.g.floor(x, z)
        land = ease(0.8, 2.8, soil) * (1 - ease(0.27, 0.5, np.hypot(dx, dz)))
        return grass * (0.33 + 0.67 * close) * land

    def safe_mask(self, x, z):
        i = np.floor((x - self.g.x0) / self.g.s).astype(int)
        j = np.floor((z - self.g.z0) / self.g.s).astype(int)
        same = np.ones_like(i, dtype=bool)
        for a, b in ((0, 0), (1, 0), (0, 1), (1, 1)):
            same &= self.g.h[j + b, i + a] == self.v.h[j + b, i + a]
        return same

    def multiplier(self, x, z):
        f = self.field(x, z)
        # Shelter gets deeper olive; the gaps retain the original pale dune
        # material, normal chart and actual UVs right through the transition.
        return np.stack((1 - 0.31 * f, 1 - 0.13 * f, 1 - 0.33 * f), axis=-1)

    def heights(self, x, z):
        return self.g.floor(x, z)[0]


def paint_grass(p, meadow):
    r = np.random.default_rng(831091)
    x0, x1, z0, z1 = BOUNDS
    xx = r.uniform(x0 + 1, x1 - 1, 13700)
    zz = r.uniform(z0 + 1, z1 - 1, 13700)
    stand = meadow.field(xx, zz)
    breath = (
        0.64 + 0.36 * np.sin(xx * 1.57 + zz * 0.36 + 1.17 * np.sin(0.71 * zz))
    ) ** 1.2
    occupied = r.random(len(xx)) < 0.79 * stand**1.45 * breath
    ok = occupied & meadow.safe_mask(xx, zz)
    xyz = np.column_stack([xx[ok], meadow.heights(xx[ok], zz[ok]), zz[ok]])
    count = n = 0
    for q in xyz:
        x, base, z = q
        tile = p(x, z)
        # One connected low crown; different footprints with a common wind.
        count += 1
        orientation = 0.41 + 0.47 * math.sin(0.09 * x - 0.12 * z) + r.normal(0, 0.20)
        h = r.uniform(0.077, 0.245)
        kind = r.choice([0, 0, 0, 1, 2, 3])
        leaves = r.integers(6, 12)
        low = np.array(
            [(80, 96, 60), (90, 109, 76), (104, 100, 67), (87, 105, 73)][kind]
        )
        bright = np.array(
            [(132, 151, 91), (140, 157, 111), (164, 153, 99), (131, 148, 106)][kind]
        )
        for j in range(leaves):
            ang = orientation + r.normal(0, 0.76)
            if j % 4 == 0:
                ang += r.choice((-1, 1)) * r.uniform(1.3, 2.15)
            az = ang + r.uniform(-0.29, 0.29)
            a = np.array([math.cos(az), math.sin(az)])
            b = np.array([-a[1], a[0]])
            tint = r.uniform(0.86, 1.09)
            span = h * r.uniform(0.38, 1.33)
            top = h * r.uniform(0.54, 1.12)
            stem = r.normal(0, 0.019, 2)
            w = r.uniform(0.0030, 0.0105) * (1.25 if kind == 1 else 1)
            root = np.array([x, z]) + stem
            bed = float(meadow.heights(*root))
            row = None
            for t in (0.0, 0.43, 0.76, 1.0):
                c = root + a * span * t**1.65
                ht = (
                    bed + top * math.sin(math.pi * t * (0.67 + 0.075 * (j % 3))) - 0.005
                )
                col = (low * (1 - t) + bright * t) * tint
                if t == 1.0:
                    tip = tile.point((c[0], ht, c[1]), col)
                    tile.face(row[0], row[1], tip)
                    break
                ww = w * (0.87 + 0.20 * math.sin(t * math.pi)) * (1 - t) ** 0.56
                left, right = (c - b * ww, c + b * ww)
                current = [
                    tile.point((left[0], ht, left[1]), col * 0.96),
                    tile.point((right[0], ht + 0.22 * ww, right[1]), col * 1.035),
                ]
                if row:
                    tile.face(row[0], row[1], current[1])
                    tile.face(row[0], current[1], current[0])
                row = current
            n += 1
    return {
        "low_crowns": count,
        "true_blades": n,
        "existing_grass_or_flower_anchors_in_study": len(meadow.roots),
    }
