"""Salt meadow around windbreaks, built in connected pockets with clear runnels.
Packed into the existing opaque headland batches. No new scene entities,
cutout cards, food props or collision proxies.
"""

import math
import numpy as np


def tussock(p, r, x, z, size, wind, heath, tint):
    # Five points give a thick root, rounded bent shoulder and taper. Blade
    # shadows and highlights come from the shape, never camera-facing quads.
    phase = r.uniform(0, math.tau)
    root = (61, 86, 58) if not heath else (76, 78, 52)
    top = r.choice(
        [(176, 177, 103), (195, 181, 114), (141, 167, 96), (184, 185, 124)]
        if not heath
        else [(157, 147, 101), (178, 155, 107), (133, 148, 103)]
    )
    for j in range(5):
        t = phase + j * 2.40 + r.uniform(-0.22, 0.22)
        a, b = math.cos(t), math.sin(t)
        h = size * r.uniform(0.64, 1.08)
        run = h * r.uniform(0.45, 0.95)
        k = h * r.uniform(0.055, 0.095)
        sx, sz = a * run * 0.23, b * run * 0.23
        dx = sx + 0.40 * run * a + 0.23 * run * math.cos(wind)
        dz = sz + 0.40 * run * b + 0.23 * run * math.sin(wind)
        base = p.put(x + sx * 0.20, z + sz * 0.20, 0.0006, tint(root, 0.84))
        left = p.put(x + sx - b * k, z + sz + a * k, h * 0.38, tint(top, 0.78))
        ridge = p.put(
            x + dx + b * k * 0.24, z + dz - a * k * 0.24, h * 0.74, tint(top, 0.99)
        )
        right = p.put(x + sx + b * k, z + sz - a * k, h * 0.36, tint(top, 0.86))
        tip = p.put(
            x + dx + a * run * 0.51, z + dz + b * run * 0.51, h, tint(top, 1.07)
        )
        p.fan(base, left, ridge)
        p.fan(base, ridge, right)
        p.fan(left, tip, ridge)
        p.fan(ridge, tip, right)


def floret(p, r, x, z, scale, wind, tint):
    # Sea-heath makes powder-pink and pale apricot bracts, not berry-like
    # glazed spheres. Tiny side sprays break up identical grass silhouettes.
    u, v = math.cos(wind) * 0.14 * scale, math.sin(wind) * 0.14 * scale
    root = p.put(x, z, 0, tint((74, 98, 70), 0.82))
    k = scale * 0.012
    a = p.put(x + u - k, z + v, scale * 0.58, tint((133, 142, 86), 0.99))
    b = p.put(x + u + k, z + v, scale * 0.58, tint((142, 147, 92), 1))
    p.fan(root, a, b)
    for j in range(3):
        angle = j * 2.4 + r.uniform(-0.28, 0.28)
        t, s = math.cos(angle), math.sin(angle)
        cx, cz = x + u + t * 0.075 * scale, z + v + s * 0.075 * scale
        h = scale * (0.65 + 0.10 * j)
        center = p.put(cx, cz, h, tint((189, 148, 106), 0.91))
        p.fan(a, b, center)
        phase = r.uniform(0, math.tau)
        for i in range(4):
            ang = phase + i * math.tau / 4
            c, d = math.cos(ang), math.sin(ang)
            length = scale * r.uniform(0.036, 0.057)
            L = p.put(
                cx + c * length - d * length * 0.37,
                cz + d * length + c * length * 0.37,
                h + length * 0.77,
                tint((217, 171, 153), 1),
            )
            R = p.put(
                cx + c * length + d * length * 0.37,
                cz + d * length - c * length * 0.37,
                h + length * 0.57,
                tint((194, 158, 158), 0.98),
            )
            p.fan(center, L, R)


def colony(beach, patch, x, z, ox, oz, wind, seed, tint):
    r = np.random.default_rng(seed)
    tufts = flowers = 0
    # An inner dune forms two uneven crescents: a dense high patch tucked
    # behind the thicket, a little outlying tongue, and a true bare gap.
    for k in range(2):
        along = r.uniform(-2, 2) + (1 if k else -1) * r.uniform(2.4, 5.7)
        inland = r.uniform(0.5, 3.8)
        px = x - oz * along - ox * inland
        pz = z + ox * along - oz * inland
        heading = math.atan2(ox, -oz) + r.uniform(-0.40, 0.40)
        c, s = math.cos(heading), math.sin(heading)
        extent = r.uniform(1.8, 3.8) if not k else r.uniform(0.7, 1.8)
        for t in range(10 if not k else 6):
            u = np.clip(r.normal(0, 0.65), -1, 1) * extent
            v = r.normal(0, 0.45) + 0.19 * u * u / extent
            tx, tz = px + c * u - s * v, pz + s * u + c * v
            h, dx, dz = beach.surface(tx, tz)
            if h < 2.5 or h > 31 or math.hypot(dx, dz) > 0.56:
                continue
            p = patch(tx, tz, 4)
            size = r.uniform(0.16, 0.45) * (1 - 0.20 * abs(u) / extent)
            tussock(p, r, tx, tz, size, wind, k, tint)
            tufts += 1
            if k and r.random() < 0.25:
                floret(p, r, tx - 0.13, tz + 0.08, r.uniform(0.35, 0.56), wind, tint)
                flowers += 1
    return tufts, flowers
