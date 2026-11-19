"""Low triple-leaved creepers and a few cream seed buttons on real herb beds.

Accepted habitat and original Flowers near the lower wild valley choose which
small hollows fill. Visual only: no new stock Flowers, pickups or world IDs.
"""

import math
import numpy as np


def creepers(root, meadow):
    r = np.random.default_rng(314865)
    clusters = trifoliate = buttons = 0
    for a, b in meadow.herbs:
        direction = 0.61 + r.normal(0, 0.54)
        az = np.array([math.cos(direction), math.sin(direction)])
        side = np.array([-az[1], az[0]])
        for i in range(35):
            t = r.normal(0, 1.63)
            u = r.normal(0, 0.55)
            q = np.array([a, b]) + az * t + side * (u + 0.2 * math.sin(1.5 * t))
            x, z = q
            if not meadow.safe_mask(x, z) or meadow.field(x, z) < 0.06:
                continue
            tile = root(x, z)
            clusters += 1
            spread = r.uniform(0.12, 0.26)
            plain = r.random() < 0.68
            for w in range(r.integers(5, 10)):
                radial = r.uniform(0, math.tau)
                d = np.array([math.cos(radial), math.sin(radial)])
                length = spread * r.uniform(0.38, 1.5)
                start = q + length * d
                bed = float(meadow.heights(*start))
                y = bed + r.uniform(0.028, 0.08)
                scale = r.uniform(0.063, 0.12)
                rgb = np.array([108, 149, 86]) if plain else np.array([129, 157, 92])
                tint = r.uniform(0.9, 1.12)
                # Three slender, cupped round lobes with a pale inverted V on
                # each midrib; unlike grass even an unlit silhouette is distinct.
                for fing in (-1, 0, 1):
                    ang = radial + fing * r.uniform(0.74, 1.12)
                    along = np.array([math.cos(ang), math.sin(ang)])
                    ac = np.array([-along[1], along[0]])
                    ln = scale * (1 if not fing else 0.8)
                    ww = ln * r.uniform(0.33, 0.46)
                    mid = start + along * 0.64 * ln
                    tip = start + along * ln
                    coords = [
                        (start, y - 0.006, 0.77),
                        (mid - ac * ww, y + 0.24 * ln, 1.03),
                        (mid + along * 0.08 * ln, y + 0.32 * ln, 1.18),
                        (mid + ac * ww, y + 0.25 * ln, 0.98),
                        (tip - 0.47 * ww * ac, y + 0.22 * ln, 1.06),
                        (tip + 0.47 * ww * ac, y + 0.20 * ln, 1.0),
                    ]
                    ids = [
                        tile.point((u[0], h, u[1]), rgb * val * tint)
                        for u, h, val in coords
                    ]
                    for v in [(0, 1, 2), (0, 2, 3), (1, 4, 2), (4, 5, 2), (2, 5, 3)]:
                        tile.face(*[ids[n] for n in v])
                trifoliate += 1
                if not plain and w == 2 and i % 3 == 0:
                    # Small butter-and-cream buttons; never oversized bright
                    # balls on every green patch. Cup joins its real leaf base.
                    h = y + 0.075
                    top = start + d * 0.033
                    pos = [
                        tile.point((start[0], y, start[1]), (124, 143, 76)),
                        tile.point((top[0] - 0.004, h, top[1]), (154, 164, 86)),
                        tile.point((top[0] + 0.004, h, top[1]), (164, 168, 92)),
                    ]
                    tile.face(*pos)
                    for i2 in range(6):
                        ang = math.tau * i2 / 6
                        v = np.array([math.cos(ang), math.sin(ang)]) * 0.022
                        u = np.array([-v[1], v[0]]) * 0.5
                        leaf = [
                            tile.point((p[0], h + hh, p[1]), col)
                            for p, hh, col in [
                                (top, -0.002, (165, 135, 69)),
                                (top + v * 0.65 + u, 0.006, (220, 190, 112)),
                                (top + v * 1.23, 0.012, (239, 220, 159)),
                                (top + v * 0.61 - u, 0.009, (221, 198, 126)),
                            ]
                        ]
                        tile.face(leaf[0], leaf[1], leaf[2])
                        tile.face(leaf[0], leaf[2], leaf[3])
                    buttons += 1
    return dict(
        creeping_groups=clusters,
        trifoliate_leaflets=3 * trifoliate,
        cream_seed_buttons=buttons,
    )
