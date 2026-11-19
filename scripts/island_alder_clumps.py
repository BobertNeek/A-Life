"""Paint a dense spray as overlapping individual alder leaves, never a solid dome."""

import math
from random import Random
import numpy as np
from PIL import Image, ImageDraw


def compact(seed):
    n = 256
    s = 2
    rr = Random(seed)
    im = Image.new("RGBA", (n * s, n * s), (91, 123, 78, 0))
    d = ImageDraw.Draw(im)

    def pt(p):
        return tuple(int(round(q * s)) for q in p)

    lobes = []
    all_leaves = []
    for j in range(8):
        a = (j + 0.31 * rr.random()) * math.tau / 8
        radius = rr.uniform(31, 59) if j else 0
        center = np.array((128 + radius * math.cos(a), 132 + radius * math.sin(a)))
        spread = np.array((rr.uniform(17, 31), rr.uniform(19, 30)))
        lobes.append((center, spread))
        for k in range(rr.randint(43, 63)):
            t = rr.uniform(0, math.tau)
            r = min(1.6, abs(rr.gauss(0.68, 0.52)))
            q = center + np.array((math.cos(t), math.sin(t))) * spread * r
            swing = t + 0.9 * math.sin(j + q[0] * 0.02) + rr.uniform(-0.8, 0.8)
            dist = rr.uniform(10, 24)
            width = dist * rr.uniform(0.24, 0.38)
            drift = np.array((math.cos(swing), math.sin(swing))) * dist
            if (
                min(*(q - drift * 0.10), *(q + drift)) < 8
                or max(*(q + drift * 0.10), *(q + drift)) > 248
            ):
                continue
            shade = (
                rr.uniform(0.86, 1.13)
                + 0.12 * (1 - q[1] / 210)
                + 0.07 * math.sin(j * 1.97)
            )
            all_leaves.append(
                (rr.random() + 0.002 * (200 - q[1]), q, drift, width, shade)
            )
    for _, q, v, w, g in sorted(all_leaves, key=lambda x: x[0]):
        side = np.array((-v[1], v[0])) / np.linalg.norm(v)
        bend = rr.uniform(-0.32, 0.32) * w

        def mid(t):
            return q + v * t + side * bend * math.sin(math.pi * t)

        def edge(t, sgn):
            return mid(t) + side * sgn * math.sin(math.pi * t) ** 0.74 * w * (
                0.78 + 0.38 * t
            ) * (1 - 0.076 * (round(12 * t) % 3 == 1))

        one = [edge(i / 12, 1) for i in range(13)]
        other = [edge(i / 12, -1) for i in range(12, -1, -1)]
        shade = np.array((99, 135, 81)) * g

        def rgb(v):
            return tuple(np.uint8(np.clip(v, 0, 255))) + (255,)

        d.polygon([pt(vv) for vv in one + other], rgb(shade * 0.91))
        d.polygon(
            [pt(vv) for vv in one + [mid(i / 9) for i in range(9, -1, -1)]],
            rgb(shade * 1.07),
        )
        if rr.random() < 0.53:
            d.line(
                [pt(mid(0)), pt(mid(0.51)), pt(mid(0.87))],
                rgb(shade * 1.08 + [4, 4, 0]),
                width=1,
            )
    im = im.resize((n, n), Image.Resampling.LANCZOS)
    bg = Image.new("RGB", (n, n), (98, 130, 80))
    bg.paste(im.convert("RGB"), mask=im.getchannel("A"))
    bg.putalpha(im.getchannel("A"))
    return bg
