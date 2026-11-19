"""Dappled interior foliage. Individual ovate leaves overlap on four forked stems.

This is the body of a load-bearing hardwood bough, not a solid ellipse. Its
edges are entirely the interlocking ends of its own drawn leaves. Individual
loose sprays are modelled beyond and beneath it.
"""

import math
from random import Random
import numpy as np
from PIL import Image, ImageDraw


def clumps():
    w, h = 1024, 512
    ss = 2
    result = Image.new("RGBA", (w * ss, h * ss), (89, 123, 80, 0))
    d = ImageDraw.Draw(result)

    def point(p):
        return tuple(round(v * ss) for v in p)

    def line(points, c, t):
        d.line(
            [point(p) for p in points],
            tuple(c) + (255,),
            width=max(1, round(t * ss)),
            joint="curve",
        )

    def poly(points, c):
        d.polygon([point(p) for p in points], tuple(c) + (255,))

    for variety in range(4):
        rng = Random(92173 + variety * 361)
        ctr = 128 + 256 * variety
        leafs = []
        stems = []

        def stem(t):
            return np.array((ctr + 9 * math.sin(t * 4 + variety), 485 - 454 * t))

        def place(pos, axis, length, width, rise, shade):
            end = pos + axis * length
            if (
                min(pos[1], end[1]) < 13
                or max(pos[1], end[1]) > 496
                or min(pos[0], end[0]) < ctr - 110
                or max(pos[0], end[0]) > ctr + 110
            ):
                return
            leafs.append(
                (rng.uniform(-0.25, 0.25) + rise, pos, axis, length, width, shade)
            )

        for j in range(13):
            t = 0.045 + 0.067 * j
            for side in (-1, 1):
                tv = t + (0.021 if side > 0 else 0) + rng.uniform(-0.012, 0.012)
                a = stem(tv)
                ratio = math.sin(math.pi * (0.11 + 0.77 * tv)) ** 0.76 * (1 - 0.35 * tv)
                b = a + np.array(
                    (
                        side * rng.uniform(72, 111) * ratio,
                        -rng.uniform(22, 64) * (1 - 0.15 * tv),
                    )
                )
                c = a * 0.52 + b * 0.48 + np.array((-side * 6, -7))
                stems.append((a, c, b, 2.0 - 1.1 * tv))
                for k in range(5):
                    p = 0.08 + 0.16 * k
                    e = (1 - p) ** 2 * a + 2 * p * (1 - p) * c + p * p * b
                    for flank in (-1, 1):
                        angle = math.atan2(
                            b[1] - a[1], b[0] - a[0]
                        ) + flank * rng.uniform(0.52, 1.05)
                        direction = np.array((math.cos(angle), math.sin(angle)))
                        L = rng.uniform(21, 42) * (1 - 0.25 * tv)
                        place(
                            e,
                            direction,
                            L,
                            rng.uniform(0.20, 0.32) * L,
                            tv * 0.05 + p * 0.16,
                            rng.uniform(0.83, 1.13),
                        )
        for a, c, b, th in stems:
            line([a, c, b], (97, 106, 68), th)
        base = np.array(
            ((93, 124, 77), (90, 128, 83), (103, 135, 77), (86, 120, 79))[variety]
        )
        for _, p, n, L, width, shade in sorted(leafs, key=lambda q: q[0]):
            transverse = np.array((-n[1], n[0]))
            bow = rng.uniform(-0.34, 0.34) * width

            def mid(t):
                return p + n * L * t + transverse * bow * math.sin(math.pi * t)

            def edge(t, side):
                wid = (
                    math.sin(math.pi * t) ** 0.70
                    * (0.81 + 0.32 * t)
                    * width
                    * (1 - 0.065 * (round(t * 18) % 3 == 1))
                )
                return mid(t) + transverse * wid * side

            t = [k / 18 for k in range(19)]
            col = base * shade
            one = [edge(q, 1) for q in t]

            def rgb(x):
                return tuple(map(int, np.clip(x, 0, 255)))

            poly(one + [edge(q, -1) for q in reversed(t)], rgb(col * 0.91))
            poly(one + [mid(q) for q in reversed(t)], rgb(col * 1.055 + [2, 0, 0]))
            line(
                [mid(q) for q in (0, 0.35, 0.74, 0.93)],
                rgb(col * 1.13 + [3, 4, 0]),
                0.8,
            )
        for j in range(33):
            # An occasional glimpse of a main shoot amid the foliage.
            if j < 4 or j > 29:
                line([stem(j / 33), stem((j + 1) / 33)], (116, 130, 76), 2.3)
    result = result.resize((w, h), Image.Resampling.LANCZOS)
    bg = Image.new("RGB", result.size, (93, 126, 80))
    bg.paste(result.convert("RGB"), mask=result.getchannel("A"))
    bg.putalpha(result.getchannel("A"))
    return bg
