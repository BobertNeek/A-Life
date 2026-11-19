"""Clipped, shallow rock plates; each roof is split on the real V1 edges."""

import math
from collections import defaultdict, Counter
from pathlib import Path
import numpy as np
from scipy.ndimage import gaussian_filter
from PIL import Image, ImageDraw
from strata import BOX, nz
from finished_island_glb import save
from island_erratic_glb import streams


class Piece:
    def __init__(self):
        self.p = []
        self.c = []
        self.t = []

    def face(self, xyz, colors, tri):
        a = len(self.p)
        self.p.extend(xyz)
        self.c.extend(np.broadcast_to(colors, (len(xyz), 3)))
        for i, j, k in tri:
            if np.cross(xyz[j] - xyz[i], xyz[k] - xyz[i])[1] < 0:
                j, k = k, j
            self.t.append((a + i, a + j, a + k))


def export(meshes, target, mat):
    doc = dict(
        asset=dict(
            version="2.0",
            generator="Lanternback Rim / V1-normal mineral and heath study",
        ),
        scene=0,
        scenes=[dict(nodes=[])],
        nodes=[],
        meshes=[],
        bufferViews=[],
        accessors=[],
        materials=[
            dict(
                name=mat,
                doubleSided=True,
                pbrMetallicRoughness=dict(metallicFactor=0, roughnessFactor=0.87),
            )
        ],
    )
    buf = bytearray()
    emit = streams(doc, buf)
    total = 0
    for name, g in sorted(meshes.items()):
        if not g.t:
            continue
        p = np.array(g.p, "<f4")
        pivot = (p.min(0) + p.max(0)) * 0.5
        p -= pivot
        t = np.array(g.t)
        n = np.zeros_like(p)
        cross = np.cross(p[t[:, 1]] - p[t[:, 0]], p[t[:, 2]] - p[t[:, 0]])
        for j in range(3):
            np.add.at(n, t[:, j], cross)
        n /= np.maximum(1e-12, np.linalg.norm(n, axis=1, keepdims=True))
        c = np.clip(np.array(g.c) / 255, 0, 1)
        c = np.where(c < 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
        rgba = np.column_stack((np.uint8(np.rint(255 * c)), np.full(len(p), 255, "B")))
        att = dict(
            POSITION=emit(p, 5126, "VEC3", bounds=True),
            NORMAL=emit(n, 5126, "VEC3"),
            COLOR_0=emit(rgba, 5121, "VEC4", normalized=True),
        )
        ty = 5123 if len(p) < 65536 else 5125
        prim = dict(
            attributes=att,
            indices=emit(
                t.ravel().astype("<u2" if ty == 5123 else "<u4"),
                ty,
                "SCALAR",
                indices=True,
            ),
            material=0,
        )
        idx = len(doc["nodes"])
        doc["meshes"].append(dict(name=name, primitives=[prim]))
        doc["nodes"].append(dict(name=name, mesh=idx, translation=pivot.tolist()))
        doc["scenes"][0]["nodes"].append(idx)
        total += len(t)
    doc["buffers"] = [dict(byteLength=len(buf))]
    save(doc, buf, target)
    return dict(meshes=len(doc["meshes"]), triangles=total)


def clip(poly, n, k):
    if len(poly) < 3:
        return []
    v = []
    for a, b in zip(poly, np.roll(poly, -1, axis=0)):
        da, db = np.dot(a, n) - k, np.dot(b, n) - k
        if da <= 1e-9:
            v.append(a)
        if (da < 0) != (db < 0):
            v.append(a + (b - a) * da / (da - db))
    return np.array(v)


def terrain_tiles(b, polygon):
    lo = polygon.min(0)
    hi = polygon.max(0)
    s = b.s
    ox, oz = b.x0, b.z0
    for i in range(math.floor((lo[0] - ox) / s), math.floor((hi[0] - ox) / s) + 1):
        xx = ox + i * s
        for j in range(math.floor((lo[1] - oz) / s), math.floor((hi[1] - oz) / s) + 1):
            zz = oz + j * s
            cell = polygon
            for n, k in [
                ([-1, 0], -xx),
                ([1, 0], xx + s),
                ([0, -1], -zz),
                ([0, 1], zz + s),
            ]:
                cell = clip(cell, n, k)
            if len(cell) < 3:
                continue
            for sign in [1, -1]:
                p = clip(cell, np.array([1.0, -1.0]) * sign, (xx - zz) * sign)
                if len(p) >= 3:
                    yield p


def build(b, out):
    R = np.random.default_rng(9832)
    grp = defaultdict(Piece)
    stats = Counter()
    x0, x1, z0, z1 = BOX
    # Clusters are registered in real bedrock windows. The apron breaks up
    # into small clasts on its lower side, never an entire hillside tile grid.
    nr = 2300
    xx = R.uniform(x0 + 1, x1 - 1, nr)
    zz = R.uniform(z0 + 1, z1 - 1, nr)
    h, sl, d, a, l, f, rock, skirt, edge = b.field(xx, zz)
    occupied = []
    positions = []
    lifts = []
    for i, (x, z) in enumerate(zip(xx, zz)):
        if rock[i] < 0.54 or R.random() > 0.38:
            continue
        span = R.uniform(1.55, 4.8) * (1 + 0.30 * min(0.9, sl[i]))
        if any(
            (x - u) ** 2 + (z - v) ** 2 < (0.49 * (span + rad)) ** 2
            for u, v, rad in occupied
        ):
            continue
        if b.blocked(x, z, span * 1.02):
            continue
        occupied.append((x, z, span))
        ang = (
            0.18
            + 0.9 * nz(x, z, 41, 2)
            + 0.57 * math.sin(0.045 * z)
            + R.normal(0, 0.19)
        )
        u = np.array([math.cos(ang), math.sin(ang)])
        v = np.array([-u[1], u[0]])
        cen = np.array([x, z])
        width = span * R.uniform(0.30, 0.63)
        ring = np.array(
            [
                [-1, -0.09],
                [-0.78, -0.58],
                [-0.30, -0.96],
                [0.13, -0.76],
                [0.57, -0.67],
                [0.92, -0.26],
                [1.0, 0.08],
                [0.59, 0.44],
                [0.38, 0.86],
                [-0.18, 1],
                [-0.64, 0.74],
                [-0.87, 0.45],
            ]
        ) * R.uniform(0.89, 1.08, (12, 1))
        poly = cen + (ring[:, 0, None] * u * span + ring[:, 1, None] * v * width)
        # Asymmetrical rake and one cross-offset intersect, without force-field
        # circles or regular repeated stripes cut into the physical slope.
        polys = [poly]
        for cut in range(2 if span > 2.35 else 1):
            n = (u + 0.32 * v) if cut == 0 else (0.52 * u + 0.91 * v)
            n = n / np.linalg.norm(n)
            line = np.dot(cen, n) + R.uniform(-0.44, 0.36) * (
                span if cut == 0 else width
            )
            halves = []
            for q in polys:
                if cut and R.random() < 0.51:
                    halves.append(q)
                    continue
                for sg in [-1, 1]:
                    p = clip(q, n * sg, (line * sg) - R.uniform(0.018, 0.050))
                    if len(p) > 2:
                        halves.append(p)
            polys = halves
        for pol in polys:
            center = pol.mean(0)
            r = np.max(np.linalg.norm(pol - center, axis=1))
            if r < 0.35:
                continue
            top = center + (pol - center) * 0.953 + u * min(0.023, r * 0.012)
            maxh = 0.023 + min(1.0, max(0, (sl[i] - 0.44) / 0.29)) * 0.067
            maxh *= R.uniform(0.69, 1)
            lifts.append(maxh)
            basecol = np.array([153, 154, 139]) * R.uniform(0.90, 1.07)
            basecol += np.array([16, -1, -15]) * R.uniform(-0.35, 0.76)
            g = grp[
                "Gneiss / fitted district %02d-%02d"
                % (int((x - x0) // 27), int((z - z0) // 26))
            ]

            def lift(p):
                q = np.clip(
                    0.48
                    + 0.44 * ((p - center) @ v) / max(0.3, r)
                    + 0.14 * ((p - center) @ u) / max(0.3, r),
                    0.12,
                    1.0,
                )
                # Flush on the upslope shoulder and intermittently buried by
                # earth in softer seams; only the broken downhill lip peels up.
                wear = (
                    0.13
                    + 0.36 * nz(p[:, 0], p[:, 1], 1.7, 34)
                    + 0.24 * nz(p[:, 0], p[:, 1], 4.1, 71)
                )
                return np.maximum(-0.026, maxh * (q - wear) * 1.36)

            def xyz(p, amt):
                return np.column_stack(
                    (p[:, 0], b.floor(p[:, 0], p[:, 1])[0] + amt, p[:, 1])
                )

            # The top cannot cut across a physical diagonal between the two rim
            # contacts; split it into the body's real native triangles first.
            for p in terrain_tiles(b, top):
                c = basecol * (0.92 + 0.12 * nz(p[:, 0], p[:, 1], 2.47, 4))[:, None]
                g.face(
                    xyz(p, lift(p)), c, [(0, j, j + 1) for j in range(1, len(p) - 1)]
                )
            for j, k in zip(range(len(top)), np.roll(np.arange(len(top)), -1)):
                # Add edge samples where either side crosses a native line. This keeps
                # the tail keyed into the slope instead of hanging at a cell break.
                pts = np.array([pol[j], pol[k], top[k], top[j]])
                for patch in terrain_tiles(b, pts):
                    seg = pol[k] - pol[j]
                    dist = np.abs(
                        np.cross(np.broadcast_to(seg, patch.shape), patch - pol[j])
                    ) / max(1e-6, np.linalg.norm(seg))
                    spanedge = max(
                        0.004,
                        0.5
                        * (
                            np.linalg.norm(pol[j] - top[j])
                            + np.linalg.norm(pol[k] - top[k])
                        ),
                    )
                    fac = np.clip(dist / spanedge, 0, 1)

                    # Derive the exact interpolated edge height from the convex quad's two
                    # triangles rather than from the horizontal plan width.
                    def bary(p, a0, a1, a2):
                        A = np.column_stack((a1 - a0, a2 - a0))
                        if abs(np.linalg.det(A)) < 1e-10:
                            return np.zeros((len(p), 3))
                        q = (p - a0) @ np.linalg.inv(A).T
                        return np.column_stack((1 - q.sum(1), q))

                    one = bary(patch, *pts[:3])
                    two = bary(patch, *pts[[0, 2, 3]])
                    heights = np.array(
                        [
                            -0.011,
                            -0.011,
                            lift(top[k : k + 1])[0],
                            lift(top[j : j + 1])[0],
                        ]
                    )
                    rh = np.where(
                        one.min(1) >= -1e-6, one @ heights[:3], two @ heights[[0, 2, 3]]
                    )
                    g.face(
                        xyz(patch, rh),
                        basecol * (0.61 + 0.23 * fac[:, None]),
                        [(0, k, k + 1) for k in range(1, len(patch) - 1)],
                    )
            stats["riven_surfaces"] += 1
        stats["outcrop_groups"] += 1
        positions.append((round(x, 3), round(z, 3), round(span, 3)))
    # Pebble trains fan out over the downhill lip only, buried on either
    # end. Keep wide gaps and don't lay a ring or a path around each exposure.
    for x, z, span in occupied:
        if R.random() > 0.55:
            continue
        hh, gx, gz = b.floor(x, z)
        u = -np.array([gx, gz])
        u /= max(0.001, np.linalg.norm(u))
        v = np.array([-u[1], u[0]])
        for k in range(R.integers(6, 23)):
            px, pz = (
                np.array([x, z])
                + u * R.uniform(0.35, 1.85) * span
                + v * R.normal(0, 0.41 * span)
            )
            rad = R.uniform(0.042, 0.30)
            angle = R.uniform(0, math.tau)
            th = np.arange(6) * math.pi / 3 + angle
            p = np.column_stack((np.cos(th), np.sin(th))) * [rad, rad * 0.55]
            p += np.array([px, pz])
            if b.blocked(px, pz, rad):
                continue
            hi = p * 0.0 + [px, pz] + (p - [px, pz]) * 0.72
            rr = min(0.023, rad * 0.12)
            top = np.column_stack(
                (
                    hi[:, 0],
                    b.floor(hi[:, 0], hi[:, 1])[0] + rr * (1 + 0.21 * np.sin(th)),
                    hi[:, 1],
                )
            )
            low = np.column_stack(
                (p[:, 0], b.floor(p[:, 0], p[:, 1])[0] - 0.008, p[:, 1])
            )
            color = np.array([146, 149, 136]) * R.uniform(0.76, 1.12)
            gr = grp["Fine apron / %d" % int((z - z0) // 30)]
            gr.face(
                np.row_stack((low, top)),
                np.row_stack(([color * 0.62] * 6, [color] * 6)),
                [(j, (j + 1) % 6, 6 + j) for j in range(6)]
                + [((j + 1) % 6, 6 + (j + 1) % 6, 6 + j) for j in range(6)]
                + [(6, 6 + j, 7 + j) for j in range(1, 5)],
            )
            stats["embedded_clasts"] += 1
    return dict(
        **stats,
        **export(
            grp,
            out / "fitted-outcrops.glb",
            "Lanternback / sheared hornfels, quartz and lichen",
        ),
        max_roof_m=round(max(lifts), 4),
        outcrop_centroids=positions
    )


def grain(out):
    n = 480
    r = np.random.default_rng(44981)
    yy, xx = np.mgrid[0:n, 0:n] / n
    # Frequencies wrap exactly; layers curve and offset in mineral space,
    # not in world space at a fixed walking elevation.
    tau = math.tau
    warp = (
        0.84 * np.sin(tau * (xx + yy))
        + 0.53 * np.sin(tau * (2 * xx - yy))
        + 0.18 * np.sin(tau * (xx + 3 * yy))
    )
    phase = (2 * xx + 3 * yy) * tau + warp
    folded = 0.5 + 0.5 * np.sin(phase)
    seam = np.exp(-(((np.sin(phase * 0.5)) / 0.14) ** 2))
    f = gaussian_filter(r.normal(size=(n, n)), 3.1, mode="wrap")
    f /= max(0.001, f.std())
    fine = 0.58 * r.normal(size=(n, n)) + 0.42 * f
    stone = (
        np.array([177, 186, 182]) + ((folded - 0.5) * 24 + 8 * f + 9 * fine)[:, :, None]
    )
    q = seam * (0.32 + 0.68 * (0.5 + 0.5 * np.sin(tau * (xx - 2 * yy))) ** 1.3)
    stone = stone * (1 - q[:, :, None]) + np.array([238, 225, 189]) * q[:, :, None]
    h = 132 + 12 * f + 8 * fine + 24 * q + 9 * folded
    pigment = Image.fromarray(np.uint8(np.clip(stone, 0, 255)))
    ht = Image.fromarray(np.uint8(np.clip(h, 0, 255)))
    pd = ImageDraw.Draw(pigment)
    rd = ImageDraw.Draw(ht)
    for k in range(335):
        pt = r.uniform(0, n, 2)
        a = r.normal(-0.55, 0.21)
        u = np.array([math.cos(a), math.sin(a)])
        v = np.array([-u[1], u[0]])
        ll = r.uniform(1.0, 8.8)
        ww = r.uniform(0.65, 2.3)
        t = r.uniform(-0.24, 0.24)
        mid = pt + u * 0.53 * ll + v * t * ll
        rock = [213, 204, 177] if k % 4 else [106, 117, 109]
        for dx in (-n, 0, n):
            for dy in (-n, 0, n):
                points = (
                    np.array([pt, mid + v * ww, pt + u * ll, mid - v * 0.55 * ww])
                    + [dx, dy]
                ).tolist()
                shape = [tuple(v) for v in points]
                pd.polygon(shape, fill=tuple(rock))
                rd.polygon(shape, fill=149 if k % 4 else 102)
    for im, name in [(pigment, "mineral-grain.png"), (ht, "mineral-lip.png")]:
        im.save(out / name, compress_level=6)
