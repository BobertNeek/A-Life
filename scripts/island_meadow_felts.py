"""The sheltered grass-pelt: close, opaque rolled leaves over a fitted turf wash.

The middle height comes from the living native plants. The seedbank hugs the
soil and leans into a shared wind; it is intentionally not alpha shrub cards.
"""

import math
import numpy as np
from island_meadow_swales import BOUNDS


def carpet(x, z, original):
    # These normalized wind combs follow the same established community, with
    # a softer local fringe. They modify pigment, not an existing height.
    t = 1.0 - np.exp(-2.6 * np.maximum(0, original))
    flow = 0.76 + 0.24 * np.sin(2.3 * x + 1.2 * z + 0.95 * np.sin(z * 0.56))
    fleck = 0.5 + 0.5 * np.sin(4.5 * x - 0.8 * z + 1.7 * np.sin(x * 1.8 + z * 0.8))
    rich = t * (0.74 + 0.18 * flow + 0.08 * fleck)
    return np.stack((1 - 0.28 * rich, 1 + 0.10 * rich, 1 - 0.37 * rich), axis=-1)


def turf(v, n, uv, c, faces, meadow):
    # Tiny planar sampling trianglelets avoid a coarse series of vertex-colour
    # polygons. Interpolate all original source attributes on their own faces
    # so tangent-space material seams, slope and tile edges remain exact.
    steps = 8
    bary = []
    local = []
    ids = {}
    for i in range(steps + 1):
        for j in range(steps + 1 - i):
            ids[i, j] = len(bary)
            bary.append([1 - (i + j) / steps, i / steps, j / steps])
    for i in range(steps):
        for j in range(steps - i):
            local.append((ids[i, j], ids[i + 1, j], ids[i, j + 1]))
            if i + j < steps - 1:
                local.append((ids[i + 1, j], ids[i + 1, j + 1], ids[i, j + 1]))
    bary = np.array(bary, "<f4")
    local = np.array(local, "<i4")
    faces = np.asarray(faces)

    def sample(a):
        return np.einsum(
            "ij,fjk->fik", bary, np.asarray(a, dtype="<f4")[faces]
        ).reshape(-1, a.shape[-1])

    pos = sample(v)
    nn = sample(n)
    nn /= np.maximum(1e-8, np.linalg.norm(nn, axis=1))[:, None]
    original = meadow.field(pos[:, 0], pos[:, 2])
    col = sample(c)
    col[:, :3] *= carpet(pos[:, 0], pos[:, 2], original)
    f = (local[None, :, :] + np.arange(len(faces))[:, None, None] * len(bary)).reshape(
        -1, 3
    )
    return pos, nn, sample(uv), np.uint8(np.rint(col).clip(0, 255)), f


def felt(root, meadow):
    r = np.random.default_rng(831091)
    x0, x1, z0, z1 = BOUNDS
    xx = r.uniform(x0 + 1, x1 - 1, 22500)
    zz = r.uniform(z0 + 1, z1 - 1, 22500)
    f = meadow.field(xx, zz)
    gust = 0.84 + 0.16 * np.sin(xx * 1.6 + zz * 0.41 + 0.79 * np.sin(0.69 * zz))
    keep = (r.random(len(xx)) < 0.9 * f**1.17 * gust) & meadow.safe_mask(xx, zz)
    stems = 0
    for x, z in np.column_stack((xx[keep], zz[keep])):
        tile = root(x, z)
        yaw = 0.52 + 0.43 * math.sin(z * 0.13 - x * 0.16) + r.normal(0, 0.27)
        along = np.array([math.cos(yaw), math.sin(yaw)])
        across = np.array([-along[1], along[0]])
        family = r.choice([0, 0, 0, 1, 1, 2])
        high = r.uniform(0.043, 0.115)
        near = np.array([(123, 147, 74), (129, 151, 99), (150, 152, 81)][family])
        sunny = np.array([(182, 192, 111), (175, 192, 139), (200, 186, 113)][family])
        for j in range(r.integers(8, 15)):
            center = (
                np.array([x, z])
                + along * r.normal(0, 0.15)
                + across * r.normal(0, 0.091)
            )
            az = yaw + r.normal(0, 0.64) + (math.pi if j % 4 == 1 else 0)
            leaf = np.array([math.cos(az), math.sin(az)])
            side = np.array([-leaf[1], leaf[0]])
            length = r.uniform(0.13, 0.37)
            h = high * r.uniform(0.58, 1.24)
            w = r.uniform(0.010, 0.021)
            if j % 7 == 0:
                h *= 1.33
                length *= 0.76
            tint = r.uniform(0.89, 1.09)
            rows = []
            for k, (walk, lift, width) in enumerate(
                ((0, -0.10, 0.35), (0.33, 0.82, 1), (0.68, 1, 0.75), (1, 0.59, 0))
            ):
                mid = center + leaf * length * walk + along * (walk**2 * 0.052)
                ground = float(meadow.heights(*mid))
                y = ground + h * lift - 0.0018
                # No squared-off stalk: initial 1/3 sits below or against
                # the slope, then opens into a shallow folded shoulder.
                col = ((1 - k / 3) * near + (k / 3) * sunny) * tint
                if k == 3:
                    tip = tile.point((mid[0], y, mid[1]), col)
                    tile.face(rows[-1][0], rows[-1][1], tip)
                    continue
                row = []
                for s in (0,) if k == 0 else ((-1, 0, 1) if k == 1 else (-1, 1)):
                    p = mid + side * w * width * s
                    row.append(
                        tile.point(
                            (p[0], y + (0.21 * w if not s else 0), p[1]),
                            col * (1.05 if not s else (1.02 if s < 0 else 0.95)),
                        )
                    )
                if k == 1:
                    tile.face(rows[0][0], row[1], row[0])
                    tile.face(rows[0][0], row[2], row[1])
                elif k == 2:
                    l = rows[-1]
                    tile.face(l[0], l[1], row[0])
                    tile.face(l[1], row[1], row[0])
                    tile.face(l[1], l[2], row[1])
                rows.append(row)
            stems += 1
    from island_meadow_medick import creepers

    return dict(
        **creepers(root, meadow),
        low_crowns=int(keep.sum()),
        true_blades=stems,
        existing_grass_or_flower_anchors_in_study=len(meadow.roots),
        area_unit="metres; opaque vascular seedbank",
        native_height_subdivision="pigment-only, eight segments per unchanged triangle side",
    )
