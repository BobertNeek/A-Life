"""Paint scaly maritime bark and unwrap the stock bole, without moving its surface."""

import math
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter, map_coordinates


def smooth(x):
    t = np.clip(x, 0, 1)
    return t * t * (3 - 2 * t)


def pigment(tex):
    return Image.fromarray(np.rint(tex).clip(0, 255).astype("u1"))


def maps(rgb, relief, rough, ao, power):
    y, x = np.gradient(gaussian_filter(relief, 0.46))
    n = np.stack((-power * x, -power * y, np.ones_like(x)), -1)
    n /= np.linalg.norm(n, axis=-1)[..., None]
    return (
        pigment(rgb),
        pigment(127.5 * (1 + n)),
        pigment(np.stack((ao, rough, 0 * ao), -1)),
    )


def paint():
    # Overrun on either side of the 344 px repeating circumference. A rolled
    # bark polygon can cross its UV meridian without crossing the foliage panel.
    H = W = 512
    j, i = np.indices((H, W), dtype=float)
    u = ((i - 84) / 344) % 1
    v = j / H
    rng = np.random.default_rng(8561951)

    def noise(nx, ny, seed):
        sample = np.random.default_rng(seed).uniform(-1, 1, (ny, nx))
        return map_coordinates(sample, [v * ny, u * nx], order=3, mode="grid-wrap")

    broad = noise(7, 11, 46591)
    wu = u + 0.012 * noise(7, 7, 46) + 0.007 * np.sin(24 * v + broad)
    wv = v + 0.011 * noise(7, 14, 239)
    # Elongated, broken scales, not a vertical sine-wave stripe.
    cols, rows = 31, 29
    a, b = wu * cols, wv * rows
    gx, gy = np.floor(a).astype(int), np.floor(b).astype(int)
    tables = rng.uniform(0.14, 0.88, (rows, cols, 3))
    near = np.full_like(u, 1000)
    second = near.copy()
    age = np.zeros_like(u)
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            sx, sy = gx + dx, gy + dy
            param = tables[sy % rows, sx % cols]
            x, y = a - sx - param[..., 0], b - sy - param[..., 1]
            dist = np.sqrt(x * x + (y + 0.19 * x) ** 2)
            better = dist < near
            age = np.where(better, param[..., 2], age)
            second = np.where(better, near, np.minimum(second, dist))
            near = np.minimum(near, dist)
    grain = noise(87, 95, 2377)
    gap = second - near + 0.067 * grain
    worn = 0.27 + 0.64 * smooth((noise(12, 19, 1682) + 0.4) / 1.02)
    ridge = 1 - (1 - smooth(gap / 0.19)) * worn
    lip = (1 - smooth(np.abs(gap - 0.14) / 0.11)) * (0.24 + 0.23 * age) * worn
    lace = smooth((noise(11, 16, 812) + 0.38 * noise(52, 73, 412) - 0.38) / 0.40)
    mix = 11 * (age - 0.5) + 9 * broad + 3.8 * grain + 3 * noise(64, 34, 121)
    rgb = np.array((126, 112, 96)) + mix[..., None]
    rgb += (ridge[..., None] - 0.68) * np.array([26, 24, 22])
    rgb += lip[..., None] * (10, 10, 9)
    rgb += lace[..., None] * (11, 15, 12)
    return maps(
        rgb,
        0.42 * ridge + 0.15 * lip + 0.055 * grain + 0.075 * age,
        209 + 11 * ridge + 11 * lace,
        234 + 17 * ridge,
        0.89,
    )


def atlas(folder, wood):
    names = ("needles.png", "needles-normal.png", "needles-orm.png")
    for k, (name, bark) in enumerate(zip(names, wood, strict=True)):
        old = Image.open(folder / name).copy()
        bark = bark.convert(old.mode)
        if bark.size != old.size:
            bark = bark.resize(old.size, Image.Resampling.BOX)
        side = old.width
        both = Image.new(old.mode, (side * 2, old.height))
        both.paste(old, (0, 0))
        both.paste(bark, (side, 0))
        both.save(folder / name, compress_level=6)


def rolled_u(theta):
    return (512 + 84 + 344 * theta) / 1024


def wood_v(y):
    # More texels down where a creature's head, paws and the roots meet.
    return np.interp(
        y, [-0.25, 0, 1.8, 4.6, 8.3, 12.2], [0.992, 0.965, 0.37, 0.17, 0.079, 0.009]
    )


def unwrap(old, moved, norm, tri, profile, quarter):
    angle = ((np.arctan2(old[:, 2], old[:, 0]) / math.tau) + 1) % 1
    y = wood_v(old[:, 1])
    pos, nn, colors, tex, indices = [], [], [], [], []
    lookup = {}
    # No source polygon is added or removed, including the stock collision
    # stem. Only unwrap the visible wood that used to sample a white square.
    for face in tri:
        a = angle[face].copy()
        if np.ptp(a) > 0.58:
            a[a < 0.5] += 1
            if np.mean(a) > 1:
                a -= 1
        uv = np.column_stack((rolled_u(a), y[face]))
        ab, ac = uv[1] - uv[0], uv[2] - uv[0]
        if (
            abs(ab[0] * ac[1] - ab[1] * ac[0]) < 0.0000004
            or np.min(uv[:, 0]) <= 0.503
            or np.max(uv[:, 0]) >= 0.997
        ):
            # Tip/end cap; planar miniature section avoids a collapsed tangent.
            uv = np.array(((0.715, 0.21), (0.732, 0.225), (0.738, 0.209)))
        fresh = []
        for src, coords in zip(face, uv, strict=True):
            key = (int(src), *np.rint(coords * 10**6).astype(int))
            if key not in lookup:
                lookup[key] = len(pos)
                pos.append(moved[src])
                nn.append(norm[src])
                tex.append(coords)
                gain = 0.97 + 0.024 * math.sin(quarter * 8.31 + profile * 4.1)
                colors.append(
                    (round(246 * gain), round(247 * gain), round(243 * gain), 255)
                )
            fresh.append(lookup[key])
        indices.append(fresh)
    return tuple(np.array(item) for item in (pos, nn, colors, indices, tex))
