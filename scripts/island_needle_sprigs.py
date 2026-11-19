"""Paint hierarchies of spruce side-twigs across the non-alpha needle atlas.

The large members matter at three-to-six metres; individual needles emerge
only close to the outer twigs. All brightness differences describe pigment or
a local recess. The native sun remains free to light the branch from any side.
"""

import math
import numpy as np
from PIL import ImageDraw


def articulated(col, height, rough, ss):
    rng = np.random.default_rng(680115)
    rgb = ImageDraw.Draw(col)
    rel = ImageDraw.Draw(height)
    rr = ImageDraw.Draw(rough)

    def stroke(points, px, tone, lift, rrv):
        pts = [(round(x * ss), round(y * ss)) for x, y in points]
        width = max(1, round(px * ss))
        rgb.line(pts, fill=tuple(int(v) for v in tone), width=width)
        rel.line(pts, fill=int(lift), width=width)
        rr.line(pts, fill=int(rrv), width=width)

    for q, (start, end) in enumerate(((9, 205), (211, 406))):
        mid = 0.5 * (start + end)
        maxh = 0.5 * (end - start)
        phase = rng.uniform(0, 8)
        # Begin with shaded fine needles instead of a white carpet. The
        # coloured gaps show through the bright clusters as narrow pockets.
        rect = (0, start * ss, 512 * ss, end * ss)
        rgb.rectangle(rect, fill=(165, 185, 159))
        rel.rectangle(rect, fill=61)
        rr.rectangle(rect, fill=210)

        def stem(x):
            return (
                mid + 4.2 * math.sin(x * 0.012 + phase) + 1.9 * math.sin(x * 0.031 + q)
            )

        stroke(
            [(-8, stem(-8)), *[(x, stem(x)) for x in range(0, 523, 8)]],
            3.7,
            (148, 152, 113),
            125,
            209,
        )
        for layer, n in enumerate((17, 22, 15)):
            for sign in (-1, 1):
                anchors = np.linspace(-61, 501, n) + rng.uniform(-11, 11, n)
                for j, x in enumerate(anchors):
                    y = stem(x) + sign * rng.uniform(-4, 8)
                    length = rng.uniform(38, 76) * (1 - 0.09 * layer)
                    spread = rng.uniform(0.48, 0.93) * maxh
                    if layer == 2:
                        spread *= rng.uniform(0.53, 0.83)
                    bend = rng.uniform(-0.21, 0.23)

                    def branch(t):
                        return (
                            x + length * (t + bend * math.sin(math.pi * t)),
                            y
                            + sign * spread * (0.98 * t + 0.19 * math.sin(math.pi * t)),
                        )

                    basecol = (180, 192, 155) if layer else (145, 166, 143)
                    stroke(
                        [branch(t) for t in np.linspace(0, 1, 12)],
                        1.4,
                        basecol,
                        119,
                        204,
                    )
                    val = rng.uniform(-9, 11)
                    nn = 12 if layer == 0 else 14
                    for k, t in enumerate(np.linspace(0.10, 1.0, nn)):
                        x0, y0 = branch(t)
                        twig = 5 + 0.38 * length * (0.94 - 0.55 * t) * rng.uniform(
                            0.65, 1.1
                        )
                        for hand in (-1, 1):
                            dx = (0.62 + 0.37 * t) * twig
                            dy = (
                                sign
                                * (0.52 + 0.17 * t + hand * (0.84 - 0.30 * t))
                                * twig
                            )
                            light = (
                                169 + layer * 22 + 20 * t + val + rng.uniform(-9, 10)
                            )
                            # Two fine parallel needles and a short tip glint,
                            # not a filled painted almond.
                            for off in (0, 1.3 if hand == 1 else -1.3):
                                l = light - (8 if off else 0)
                                tone = (l - 4 + q * 4, min(255, l + 8), l - 8 - q * 5)
                                stroke(
                                    [
                                        (x0, y0 + off),
                                        (x0 + dx * 0.47, y0 + dy * 0.44 + off),
                                        (x0 + dx, y0 + dy + off),
                                    ],
                                    1.45 if layer else 1.0,
                                    tone,
                                    123 + layer * 27,
                                    185 + layer * 3,
                                )
                        if layer == 2 and k in (8, 12) and j % 3 == 1:
                            stroke(
                                [(x0 + 3, y0 + sign * 2), (x0 + 12, y0 + sign * 6)],
                                1.55,
                                (239, 248, 203),
                                190,
                                172,
                            )
