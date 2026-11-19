"""Periodic centimetre-scale PBR paintings for turf, dry forest litter, sand and rock.
Transition layouts use shared corner labels and D4-stable edge treatments.
"""

import math
from pathlib import Path

import numpy as np
from PIL import Image
from scipy.ndimage import map_coordinates

from island_habitat_field import TURF, SAND, STONE, GROVE

N = 320
GUTTER = 8
METRES = 6.25


def ease(a, b, x):
    t = np.clip((x - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def noise(n, seed, x, y):
    grid = np.random.default_rng(seed).uniform(-1, 1, (n, n)).astype("f")
    return map_coordinates(grid, [y * n, x * n], order=3, mode="grid-wrap")


def leaf_stamp(rgb, h, occ, x, y, angle, length, width, bend, tint, lift):
    """Bent pointed leaf lit by the material, not a baked light direction."""
    n = h.shape[0]
    r = math.ceil(length * 1.28 + width + 3)
    yy, xx = np.mgrid[-r : r + 1, -r : r + 1]
    ca, sa = math.cos(angle), math.sin(angle)
    t = (xx * ca + yy * sa) / length
    across = (-xx * sa + yy * ca) / width - bend * (1 - t * t)
    half = 0.08 + 0.92 * np.maximum(0, 1 - t * t)
    q = np.maximum(abs(t), abs(across) / half)
    mask = 1 - ease(0.82, 1.08, q)
    where = np.ix_(
        (round(y) + np.arange(-r, r + 1)) % n, (round(x) + np.arange(-r, r + 1)) % n
    )
    # Vein and raised shoulders are shared between pigment and normal height.
    vein = (1 - ease(0.06, 0.25, abs(across))) * mask
    colour = np.asarray(tint) + ((0.5 * t + vein) * 9 - 6 * q.clip(0, 1))[..., None]
    rgb[where] += mask[..., None] * (colour - rgb[where])
    h[where] += lift * (0.78 * mask + 0.22 * vein)
    occ[where] *= 1 - 0.11 * np.exp(-(((q - 1.0) / 0.14) ** 2))


def panel_noise(x, y, nx, ny, seed):
    """First/second-distance threshold gives unequal isolated fracture planes."""
    g = np.random.default_rng(seed).uniform(0.14, 0.86, (ny, nx, 2))
    u = x * nx
    v = y * ny
    i = np.floor(u).astype(int)
    j = np.floor(v).astype(int)
    near = np.full_like(x, 1e3)
    second = near.copy()
    stone = np.zeros_like(x)
    for dj in (-1, 0, 1):
        for di in (-1, 0, 1):
            off = g[(j + dj) % ny, (i + di) % nx]
            d = (i + di + off[..., 0] - u) ** 2 + (j + dj + off[..., 1] - v) ** 2
            win = d < near
            second = np.minimum(second, np.where(win, near, d))
            near = np.minimum(near, d)
            stone = np.where(win, off[..., 0] - off[..., 1], stone)
    return np.sqrt(second) - np.sqrt(near), stone


def substrate(kind, variation, rock_ground=False):
    y, x = (np.indices((N, N), dtype="f") + 0.5) / N
    k = 9271 + kind * 21 + variation * 7919
    r = np.random.default_rng(k)
    field = noise(5, k, x, y)
    brush = noise(13, k + 1, x, y)
    grain = noise(45, k + 2, x, y)
    # A 2–5 cm granular base persists between individual leaf/root strokes.
    fleck = 0.65 * noise(104, k + 5, x, y) + 0.35 * r.normal(0, 0.66, x.shape).astype(
        "f"
    )
    h = np.zeros_like(x)
    occ = np.ones_like(x)
    if kind in (TURF, GROVE):
        forest = kind == GROVE
        # Grass is broken into bright tufts with exposed dark soil. The grove
        # floor has fewer living blades and broad, curled fallen leaves.
        pillow = ease(-0.15, 0.50, 0.72 * field + 0.46 * brush + 0.14 * grain)
        dark = np.array((182, 181, 158) if forest else (179, 181, 154))
        light = np.array((214, 209, 179) if forest else (224, 233, 192))
        rgb = (
            dark
            + (light - dark) * pillow[..., None]
            + (grain * 6 + fleck * 12)[..., None]
        )
        h += pillow * 0.004 + brush * 0.001 + fleck * 0.0017
        rough = 0.89 - 0.055 * pillow
        for _ in range(260 if forest else 940):
            cx, cy = r.uniform(0, N, 2)
            a = r.uniform(0, math.tau)
            tuft = r.uniform(0.72, 1.33)
            for side in range(r.integers(3, 7) if not forest else r.integers(2, 5)):
                ang = (
                    a + side * 2.40 if forest else a * 0.23 + 0.41 + side * 0.48
                ) + r.normal(0, 0.35)
                length = tuft * (r.uniform(2.4, 6.5) if forest else r.uniform(1.6, 4.4))
                width = (
                    length * r.uniform(0.21, 0.40) if forest else r.uniform(0.44, 0.95)
                )
                tint = r.choice(
                    [(190, 157, 110), (207, 181, 132), (158, 158, 119), (224, 196, 145)]
                    if forest
                    else [
                        (230, 240, 194),
                        (196, 217, 164),
                        (163, 184, 146),
                        (229, 220, 167),
                        (238, 231, 183),
                    ]
                )
                sx = cx + math.cos(ang) * length * 0.66 + r.normal(0, 2)
                sy = cy + math.sin(ang) * length * 0.66 + r.normal(0, 2)
                leaf_stamp(
                    rgb,
                    h,
                    occ,
                    sx,
                    sy,
                    ang,
                    length,
                    width,
                    r.uniform(-0.65, 0.65),
                    tint,
                    r.uniform(0.0022, 0.0055),
                )
    elif kind == SAND:
        # The forked section drifts rather than lining every beach in parallel.
        tau = math.tau
        phase = (
            tau * (15 * x + 29 * y)
            + field * 3.4
            + 1.8 * np.sin(tau * (2 * x - y))
            + 0.45 * brush
        )
        crest = (0.5 + 0.5 * np.cos(phase)) ** 3.7
        reach = ease(-0.75, 0.66, 0.80 * field + 0.25 * brush)
        h += 0.0061 * crest * (0.24 + 0.76 * reach) + grain * 0.0013 + fleck * 0.0011
        rgb = (
            np.array((233, 221, 196))
            + (
                4.6 * field
                + 4.0 * grain
                + 6.2 * fleck
                + 14 * (crest - 0.23) * (0.3 + 0.7 * reach)
            )[..., None]
        )
        rough = 0.80 + 0.085 * reach - 0.035 * crest
        for _ in range(210):
            length = r.uniform(0.55, 1.9)
            leaf_stamp(
                rgb,
                h,
                occ,
                *r.uniform(0, N, 2),
                r.uniform(0, tau),
                length,
                length * r.uniform(0.4, 0.85),
                0.1,
                r.choice([(239, 236, 218), (185, 177, 151), (215, 207, 182)]),
                0.0019
            )
    else:
        # Oblique plates and fine mineral splinters. No repeating contour rows.
        seam, face = panel_noise(x + field * 0.014, y + brush * 0.012, 13, 9, k)
        slit = 1 - ease(0.015, 0.115, seam)
        lip = ease(0.04, 0.14, seam) * (1 - ease(0.16, 0.36, seam))
        pale = (1 - ease(0.017, 0.06, seam)) * ease(0.05, 0.59, field + brush * 0.3)
        lichen = ease(0.48, 0.85, field * 0.6 + brush * 0.60 + grain * 0.22) * (
            1 - slit
        )
        h += 0.009 * face - 0.014 * slit + 0.003 * lip + 0.0012 * grain
        rgb = (
            np.array((210, 211, 210))
            + (26 * face + 2.8 * grain + 10 * lip - 32 * slit + 32 * pale)[..., None]
        )
        rgb += lichen[..., None] * np.array((6, 8, -19))
        occ *= 1 - 0.27 * slit
        rough = 0.69 + 0.17 * lichen + 0.08 * slit + 0.04 * brush
        if rock_ground:
            from island_shale_floor import floor

            rgb, h, occ, rough = floor(r, x, y, field, brush, grain, fleck, seam, face)
    # Match average reflected light, not average encoded RGB. Each biome then
    # preserves the world pigment as L0 hands off to the world-space L1 map.
    # Keep the in-tile variation, with no large green/dark tile at a seam.
    from island_habitat_massing import linear, srgb

    reflectance = linear(np.clip(rgb / 255, 0, 1))
    gain = linear(np.array([220, 220, 220]) / 255.0) / reflectance.mean(axis=(0, 1))
    rgb = 255 * srgb(np.clip(reflectance * gain, 0, 1))
    return np.dstack((rgb, h, rough, occ)).astype("f")


def symmetric_edge(image):
    return np.mean(
        [np.rot90(a, k) for a in (image, np.flip(image, axis=1)) for k in range(4)],
        axis=0,
    )


def bake(keys, dest, rock_ground=False):
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    y, x = (np.indices((N, N), dtype="f") + 0.5) / N
    edge = np.minimum.reduce((x, y, 1 - x, 1 - y))
    preserve = ease(0.012, 0.067, edge)[..., None]
    treatments = {}
    for k in range(4):
        base = substrate(k, 0, rock_ground)
        rim = symmetric_edge(base)
        for v in range(4):
            raw = base if v == 0 else substrate(k, v, rock_ground)
            treatments[k, v] = rim + (raw - rim) * preserve
    m = noise(9, 730, x, y) * 0.23 + noise(35, 17, x, y) * 0.065
    m = symmetric_edge(m) + (m - symmetric_edge(m)) * preserve[:, :, 0]
    tx = ease(0, 1, x)
    ty = ease(0, 1, y)
    corner = ((1 - tx) * (1 - ty), tx * (1 - ty), (1 - tx) * ty, tx * ty)
    cols = math.ceil(math.sqrt(len(keys)))
    rows = math.ceil(len(keys) / cols)
    size = N + 2 * GUTTER
    maps = [Image.new("RGB", (cols * size, rows * size)) for _ in range(3)]
    for i, key in enumerate(keys):
        val = np.zeros((N, N, 6), "f")
        weights = []
        for k in range(4):
            w = sum(c for c, label in zip(corner, key[:4]) if k == label)
            w = w + m * w * (1 - w) * (1 if k in (TURF, SAND) else -1)
            weights.append(w)
        weight_sum = sum(weights)
        for k, w in enumerate(weights):
            if isinstance(w, np.ndarray):
                val += (w / weight_sum)[..., None] * treatments[k, key[4]]
        rgb, h, rough, occ = val[:, :, :3], val[:, :, 3], val[:, :, 4], val[:, :, 5]
        dz, dx = np.gradient(h, N**-1 * METRES)
        normal = np.stack((-dx, -dz, np.ones_like(dx)), axis=-1)
        normal /= np.linalg.norm(normal, axis=-1, keepdims=True)
        orm = np.stack((occ, rough, np.zeros_like(occ)), axis=-1) * 255
        for canvas, part in zip(maps, (rgb, (normal + 1) * 127.5, orm)):
            a = np.pad(
                np.uint8(np.clip(np.rint(part), 0, 255)),
                ((GUTTER, GUTTER), (GUTTER, GUTTER), (0, 0)),
                mode="edge",
            )
            canvas.paste(Image.fromarray(a), ((i % cols) * size, (i // cols) * size))
    names = ("substrates.png", "substrates-normal.png", "substrates-orm.png")
    for canvas, name in zip(maps, names):
        if name.endswith("-orm.png"):
            # Each 336 px slot and its eight-pixel gutter remain exactly aligned.
            canvas = canvas.resize(
                (canvas.width // 4, canvas.height // 4), Image.Resampling.BOX
            )
        canvas.save(dest / name, optimize=True)
    return cols, rows
