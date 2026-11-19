"""Draw three lace-cut woodland fronds for one embedded sRGB + coverage image.

Each pinna has its own lobes, vein and tip and is attached to the winding
rachis. There is no photographed leaf or unlicensed input in the atlas.
"""

import math
from random import Random
from PIL import Image, ImageDraw


def paint(path):
    scale, cell, height = 2, 256, 512
    img = Image.new("RGBA", (4 * cell * scale, height * scale), (111, 148, 83, 0))
    draw = ImageDraw.Draw(img)

    def pen(pts, rgb, width=1, poly=False):
        coords = [(round(x * scale), round(y * scale)) for x, y in pts]
        col = tuple(round(min(255, max(0, c))) for c in rgb) + (255,)
        if poly:
            draw.polygon(coords, fill=col)
        else:
            draw.line(
                coords, fill=col, width=max(1, round(width * scale)), joint="curve"
            )

    for k in range(4):
        rng = Random(29931 + 317 * k)
        phase = rng.uniform(-1, 1)
        cx = cell * (k + 0.5)

        def axis(t):
            return (cx + 6 * t * math.sin(3.8 * t + phase), 502 - 492 * t)

        n = (15, 13, 17, 14)[k]
        start = (0.22, 0.17, 0.24, 0.18)[k]
        tone = (95 + 5 * k, 139 + 2 * k, 78 + 2 * k)
        for j in range(n):
            for sign in (-1, 1):
                p = (j + (0.19 if sign == 1 else -0.08) + rng.uniform(-0.07, 0.07)) / n
                t = start + (1 - start) * p
                ax, ay = axis(t)
                extent = (
                    0.39 + 0.61 * math.sin(math.pi * min(1, p + 0.18)) ** 0.60
                ) * (1 - p) ** 0.46
                length = (95, 106, 94, 100)[k] * extent * rng.uniform(0.88, 1.04)
                incline = (0.35 + 0.25 * p + rng.uniform(-0.07, 0.07)) * length
                curve = rng.uniform(-0.13, 0.13) * length
                half = (4.4, 4.8, 4.2, 4.5)[k] + 4.4 * (1 - p)

                def line(s):
                    return (
                        ax + sign * length * s,
                        ay - incline * s - curve * math.sin(math.pi * s),
                    )

                # Individual leaflets are mildly asymmetric; alternating lobes
                # cut into both margins but keep a broad opaque silhouette.
                def edge(s, side):
                    x, y = line(s)
                    count = (7, 6, 7, 6)[k] - round(2 * p)
                    q = (s * count + (0.21 if side > 0 else 0.57)) % 1
                    teeth = 0.62 + 0.38 * (1 - abs(2 * q - 1))
                    w = (
                        half
                        * math.sin(math.pi * s) ** 0.57
                        * (0.85 + 0.25 * (1 - s))
                        * teeth
                    )
                    return (x + sign * w * 0.26 * side, y + w * side)

                steps = 32
                rim = [edge(i / steps, -1) for i in range(steps + 1)] + [
                    edge(i / steps, 1) for i in range(steps, -1, -1)
                ]
                g = rng.uniform(0.89, 1.11) * (0.98 + 0.13 * t)
                pen(rim, [v * g for v in tone], poly=True)
                narrow = [edge(i / steps, -1) for i in range(steps + 1)] + [
                    line(i / steps) for i in range(steps, -1, -1)
                ]
                pen(
                    narrow,
                    [v * g * 1.07 + (2 if q == 0 else 0) for q, v in enumerate(tone)],
                    poly=True,
                )
                pen([line(i / 12) for i in range(13)], (133 * g, 161 * g, 88 * g), 0.76)
                # Pinna veinlets branch from the same rachis as their teeth.
                for q in range(1, 5):
                    s = q / 6
                    for side in (-1, 1):
                        pen(
                            [line(s), edge(min(0.97, s + 0.12), side)],
                            (113 * g, 150 * g, 85 * g),
                            0.50,
                        )
        for j in range(80):
            a, b = j / 80, (j + 1) / 80
            pen(
                [axis(a), axis(b)],
                (108 + 42 * b, 132 + 36 * b, 75 + 18 * b),
                4.6 * (1 - 0.73 * b),
            )
        x, y = axis(0.993)
        pen(
            [(x, y + 14), (x - 2.5, y + 8), (x - 1, y - 1), (x + 2.7, y + 6)],
            (146, 169, 88),
            poly=True,
        )
    img = img.resize((4 * cell, height), Image.Resampling.LANCZOS)
    # Keep sensible RGB where alpha is zero so mip filtering cannot introduce
    # a dark fringe outside the feathered edge of a printed leaflet.
    back = Image.new("RGB", img.size, (105, 146, 80))
    back.paste(img.convert("RGB"), mask=img.getchannel("A"))
    back.putalpha(img.getchannel("A"))
    back.save(path, optimize=True)
