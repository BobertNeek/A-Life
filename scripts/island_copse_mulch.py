"""A pair of broken, wind-gathered duff beds and their triangle-contact geometry."""

import math
import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter
from island_spruce_bark import maps, paint, smooth


def plate(seed):
    rng = np.random.default_rng(seed)
    w, h = 512, 228
    j, i = np.indices((h, w))
    u, v = i / (w - 1), j / (h - 1)
    coarse = gaussian_filter(rng.normal(size=(h, w)), 11.4)
    coarse /= coarse.std()
    medium = gaussian_filter(rng.normal(size=(h, w)), 2.8)
    medium /= medium.std()
    grain = rng.uniform(-1, 1, (h, w))
    # Two merged longitudinal falls with grazed, patchy ends and interior
    # gaps. No opaque disc around each trunk.
    primary = (
        1 - ((u - 0.45) / 0.48) ** 2 - ((v - 0.50 - 0.12 * np.sin(u * 7.3)) / 0.37) ** 2
    )
    secondary = 0.79 - ((u - 0.74) / 0.22) ** 2 - ((v - 0.51) / 0.31) ** 2
    edge = np.maximum(primary, secondary) + 0.33 * coarse + 0.17 * medium
    # Keep padded atlas borders transparent.
    guard = np.minimum.reduce((u, 1 - u, v, 1 - v)) * 16
    edge = np.minimum(edge, guard - 0.18)
    # Weathered gaps run into the bed, not merely around a hard decal edge.
    # Both the underlying moss and loose needles show through this broken fall.
    edge -= 0.20 + 0.14 * np.sin(9.1 * u - 10.3 * v) + 0.12 * medium
    alpha = Image.fromarray(
        (255 * smooth((edge + 0.25 * medium + 0.15 * grain - 0.18) * 2.70)).astype("u1")
    )
    opening = ImageDraw.Draw(alpha)
    # Dark humus, beech-scale russet exuviae and bent pine needles; neutral
    # high-frequency counter-colour only. Lighting remains directional in 3D.
    value = 5.8 * coarse + 4.7 * medium + 4.5 * grain
    base = np.clip(
        np.array([100, 96, 75])
        + value[..., None]
        + smooth((0.6 * coarse + 0.4 * medium + 1) / 2)[..., None] * (-9, 4, -1),
        0,
        255,
    ).astype("u1")
    col = Image.fromarray(base)
    hmap = Image.fromarray((88 + 7 * medium + 7 * grain).clip(0, 255).astype("u1"))
    draw, relief = ImageDraw.Draw(col), ImageDraw.Draw(hmap)
    for k in range(3600):
        px, py = rng.uniform(-5, w + 5), rng.uniform(-4, h + 4)
        length = rng.uniform(1.25, 4.2)
        a = rng.normal(0.32 * math.sin(5 * px / w + 1.9 * py / h) - 0.37, 0.72)
        d = np.array([math.cos(a) * length, math.sin(a) * length])
        perp = np.array([-d[1], d[0]]) * rng.uniform(0.04, 0.12)
        center = np.array([px, py])
        tint = (
            (134, 111, 72),
            (148, 121, 82),
            (115, 99, 71),
            (151, 133, 96),
            (129, 106, 75),
        )[k % 5]
        pts = [
            tuple(center - d),
            tuple(center - 0.15 * d + perp),
            tuple(center + d),
            tuple(center + 0.12 * d - perp * 0.52),
        ]
        draw.polygon(pts, fill=tint)
        relief.polygon(pts, fill=int(rng.integers(94, 124)))
        iy, ix = int(np.clip(py, 0, h - 1)), int(np.clip(px, 0, w - 1))
        if -0.1 < edge[iy, ix] < 0.5 and guard[iy, ix] > 0.6 and k % 3 != 0:
            opening.polygon(pts, fill=245)
    for k in range(125):
        px, py = rng.uniform(5, w - 5), rng.uniform(5, h - 5)
        a = rng.normal(0.12, 1.2)
        v0 = np.array([px, py])
        t = np.array([math.cos(a), math.sin(a)]) * rng.uniform(2.35, 5.6)
        n = np.array([-t[1], t[0]]) * rng.uniform(0.24, 0.48)
        verts = [
            v0 - t,
            v0 - 0.35 * t + n,
            v0 + 0.58 * t + 0.79 * n,
            v0 + t,
            v0 + 0.15 * t - n,
        ]
        tint = ((135, 110, 69), (158, 130, 82), (130, 110, 81))[k % 3]
        draw.polygon([tuple(a) for a in verts], fill=tint)
        relief.polygon([tuple(a) for a in verts], fill=114)
        draw.line(
            [tuple(v0 - 0.7 * t), tuple(v0 + 0.6 * t)],
            fill=tuple(c - 14 for c in tint),
            width=1,
        )
    surface = np.asarray(hmap, dtype=float) / 115
    rgb, norm, orm = maps(
        np.asarray(col), surface, 211 + 7 * medium, 240 + 5 * grain, 0.65
    )
    rgb.putalpha(alpha)
    return rgb, norm, orm


def atlas(dest):
    dest.mkdir(parents=True, exist_ok=True)
    bark = paint()
    variants = [plate(s) for s in (43192, 954731)]
    for k, name in enumerate(
        ("duff-bark.png", "duff-bark-normal.png", "duff-bark-orm.png")
    ):
        mode = "RGBA" if k == 0 else "RGB"
        fill = ((255, 255, 255, 255), (128, 128, 255), (252, 212, 0))[k]
        out = Image.new(mode, (1024, 512), fill)
        out.paste(bark[k].convert(mode), (512, 0))
        for row, items in enumerate(variants):
            out.paste(items[k].convert(mode), (0, 228 * row))
        if k == 2:
            out = out.resize((512, 256), Image.Resampling.BOX)
        out.save(dest / name, compress_level=6)


def clip(polygon, edge_a, edge_b):
    # Clip a terrain triangle against a CCW rectangle, interpolating ordinary
    # contact-space XZ. Its source Y plane is retained separately.
    if not polygon:
        return []
    start = np.array(edge_a)
    dx, dz = np.array(edge_b) - start

    def signed(q):
        return dx * (q[1] - start[1]) - dz * (q[0] - start[0])

    previous = polygon[-1]
    ap = signed(previous)
    result = []
    for cur in polygon:
        ac = signed(cur)
        if (ap >= 0) != (ac >= 0):
            result.append(previous + (cur - previous) * (ap / (ap - ac)))
        if ac >= 0:
            result.append(cur)
        previous, ap = cur, ac
    return result


def bed(p, r, x, z, a, long, wide, variant):
    # Draw only original height-field triangles intersecting this paint. A
    # single flat quad would float off the downslope on the next cell's ridge.
    axis = np.array([math.cos(a), math.sin(a)])
    cross = np.array([-axis[1], axis[0]])
    origin = np.array([x, z])
    rect = [
        origin + axis * u * long + cross * v * wide
        for u, v in ((-1, -1), (1, -1), (1, 1), (-1, 1))
    ]
    land = p.beach
    grid = (np.array(rect) - [land.x0, land.z0]) / land.s
    lo = np.floor(grid.min(0)).astype(int)
    hi = np.floor(grid.max(0)).astype(int)
    tone = round(r.uniform(224, 251))
    for j in range(max(0, lo[1]), min(land.h.shape[0] - 2, hi[1]) + 1):
        for i in range(max(0, lo[0]), min(land.h.shape[1] - 2, hi[0]) + 1):
            corners = [
                np.array([land.x0 + (i + u) * land.s, land.z0 + (j + v) * land.s])
                for u, v in ((0, 0), (1, 0), (1, 1), (0, 1))
            ]
            for ids in ((0, 1, 2), (0, 2, 3)):
                poly = [corners[q] for q in ids]
                for e in range(4):
                    poly = clip(poly, rect[e], rect[(e + 1) % 4])
                verts = []
                for point in poly:
                    delta = point - origin
                    u = 0.5 + np.dot(delta, axis) / (2 * long)
                    v = 0.5 + np.dot(delta, cross) / (2 * wide)
                    pixel_u = 2 + 507 * u
                    pixel_v = 2 + 228 * variant + 223 * v
                    verts.append(
                        p.surface(
                            *point,
                            0.0052,
                            (tone, tone, tone, 255),
                            (pixel_u / 1024, pixel_v / 512)
                        )
                    )
                for k in range(1, len(verts) - 1):
                    p.fan(verts[0], verts[k], verts[k + 1])
