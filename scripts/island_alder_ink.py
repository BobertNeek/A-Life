"""Original summer alder leaf sprays and salt-weathered wood, one cutout PNG.

Four small sprays vary their number of shoots. Each oval blade has a softly
serrated margin and a pale midrib, with warm new growth at the distal tips.
No source photograph or screen-space coloring.
"""

import math
from random import Random
import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter


def paint(path):
    rng = Random(119023)
    width, height, cell, leafheight = 1024, 512, 256, 416
    scale = 2
    ink = Image.new("RGBA", (width * scale, leafheight * scale), (93, 128, 78, 0))
    p = ImageDraw.Draw(ink)

    def mark(pts, c, width=1, fill=False):
        pts = [tuple(round(x * scale) for x in a) for a in pts]
        rgb = tuple(round(min(255, max(0, x))) for x in c) + (255,)
        if fill:
            p.polygon(pts, fill=rgb)
        else:
            p.line(pts, fill=rgb, width=max(1, round(width * scale)), joint="curve")

    for variant in range(4):
        x0 = (variant + 0.5) * cell
        phase = rng.uniform(-0.4, 0.4)

        def leader(t):
            return np.array([x0 + 12 * math.sin(t * 3 + phase) - 9 * t, 403 - 389 * t])

        def woody(a, b, c, w):
            mark([a, (a + b) / 2 + [1, -2], b], c, w)

        def leaf(base, end, w, light, seed):
            lr = Random(seed)
            vec = end - base
            side = np.array([-vec[1], vec[0]]) / np.linalg.norm(vec)
            bow = lr.uniform(-0.4, 0.4) * w

            def middle(t):
                return base + vec * t + side * bow * math.sin(math.pi * t)

            halves = []
            for sign in (-1, 1):
                coords = []
                for j in range(31):
                    t = j / 30
                    envelope = math.sin(math.pi * t) ** 0.72 * (0.74 + 0.5 * t)
                    rim = 1 - (0.055 if j % 3 == 1 else 0)
                    coords.append(middle(t) + side * (sign * envelope * w * rim))
                halves.append(coords)
            base_col = np.array(
                [(79, 118, 76), (91, 132, 75), (101, 139, 76), (85, 127, 84)][variant]
            )
            col = base_col * light
            mark(halves[0] + halves[1][::-1], col * 0.94, fill=True)
            mark(
                halves[0] + [middle(j / 14) for j in range(14, -1, -1)],
                col * 1.08 + [3, 0, 0],
                fill=True,
            )
            mark([middle(j / 12) for j in range(13)], col * 1.19 + [9, 4, 0], 1.25)
            for j in range(1, 5):
                t = 0.14 + j * 0.126
                for sign in (-1, 1):
                    tip = middle(t + 0.16) + side * (
                        w * 0.77 * sign * math.sin((t + 0.14) * math.pi)
                    )
                    mark([middle(t), tip], col * 1.02 + [4, 4, 1], 0.7)

        n = (6, 5, 7, 6)[variant]
        for j in range(n):
            t = 0.16 + 0.72 * j / max(1, n - 1)
            for sign in (-1, 1):
                v = t + (0.045 if sign == 1 else -0.025) + rng.uniform(-0.026, 0.026)
                pos = leader(v)
                reach = (102 - 41 * v) * rng.uniform(0.79, 1.05)
                dest = pos + np.array(
                    [sign * reach, -(29 + 29 * v) * rng.uniform(0.80, 1.15)]
                )
                dest[1] = max(12.0, dest[1])
                dest[0] = min(x0 + 116, max(x0 - 116, dest[0]))
                base = pos + 0.22 * (dest - pos)
                woody(pos, base, (124, 126, 75), 3 * (1 - 0.47 * v))
                leaf(
                    base,
                    dest,
                    12 + 4.2 * (1 - v),
                    rng.uniform(0.90, 1.14) * (0.95 + 0.14 * v),
                    rng.getrandbits(30),
                )
                if j % 3 == variant % 3 and v < 0.73:
                    base = pos + 0.16 * (dest - pos)
                    tip = base + 0.55 * (dest - base) + np.array([-11 * sign, -21])
                    leaf(base, tip, 8.4, rng.uniform(0.97, 1.10), rng.getrandbits(30))
        for j in range(40):
            t = j / 40
            woody(
                leader(t),
                leader((j + 1) / 40),
                (106 + 33 * t, 119 + 27 * t, 70 + 12 * t),
                4.5 - 2.4 * t,
            )
        leaf(leader(0.91), leader(1) + [4, -5], 6.9, 1.16, rng.getrandbits(30))
    painted = ink.resize((width, leafheight), Image.Resampling.LANCZOS)
    # Hue in transparent texels makes clean filtered edges against sky.
    bg = Image.new("RGB", painted.size, (94, 130, 79))
    bg.paste(painted.convert("RGB"), mask=painted.getchannel("A"))
    bg.putalpha(painted.getchannel("A"))
    atlas = Image.new("RGBA", (width, height), (94, 130, 79, 0))
    atlas.paste(bg)
    yy, xx = np.mgrid[0:92, 0:1024]
    nr = np.random.default_rng(994004)
    long = gaussian_filter(nr.normal(0, 1, xx.shape), (6.4, 1.3), mode="wrap")
    grain = gaussian_filter(nr.normal(0, 1, xx.shape), (0.8, 0.8), mode="wrap")
    colour = np.array([119, 105, 83]) + long[..., None] * 65 + grain[..., None] * 15
    bark = Image.fromarray(np.clip(colour, 0, 255).astype("u1")).convert("RGBA")
    d = ImageDraw.Draw(bark)
    for i in range(870):
        x, y = rng.randrange(1024), rng.randrange(92)
        ln = rng.uniform(1.5, 13)
        rgb = rng.choice([(93, 85, 69), (153, 144, 119), (112, 107, 88)])
        d.line((x, y, x + ln, y - 0.3), fill=rgb, width=1)
    atlas.paste(bark, (0, 420))
    # Opaque, padded distant foliage chips are never present on near shoots.
    brush = ImageDraw.Draw(atlas)
    for i, tone in enumerate(
        (
            (72, 100, 74),
            (85, 118, 77),
            (102, 134, 81),
            (119, 144, 82),
            (80, 112, 83),
            (90, 125, 81),
        )
    ):
        brush.rectangle((i * 16, 430, i * 16 + 15, 501), fill=(*tone, 255))
    from island_alder_clusters import clumps

    whole = Image.new("RGBA", (1024, 1024))
    whole.paste(atlas, (0, 0))
    whole.paste(clumps(), (0, 512))
    whole.save(path, optimize=True)
