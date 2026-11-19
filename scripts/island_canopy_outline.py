"""Projected silhouettes at actual stock LODs, independent of the crown generator."""

import math
import numpy as np
from PIL import Image, ImageDraw


def screen_mask(p, tri, phi, elev):
    right = np.array((math.cos(phi), 0, -math.sin(phi)))
    up = np.array(
        (
            -math.sin(phi) * math.sin(elev),
            math.cos(elev),
            -math.cos(phi) * math.sin(elev),
        )
    )
    # Fixed metres-to-pixels across every LOD of every tree. These are
    # geometric occupancy masks, not a substitute for matched studio images.
    xy = np.rint(np.column_stack((64 + p @ right * 8.5, 115 - p @ up * 8.5))).astype(
        int
    )
    im = Image.new("1", (128, 128))
    draw = ImageDraw.Draw(im)
    for face in xy[tri]:
        draw.polygon([tuple(q) for q in face.tolist()], fill=1)
    return np.asarray(im, dtype=bool)


def score(levels):
    stats = [[], []]
    for angle in (0.0, 0.28 * math.pi, 0.61 * math.pi, 0.90 * math.pi):
        for elev in (0.04, 0.43):
            mask = [screen_mask(p, t, angle, elev) for p, t in levels]
            for k in (0, 1):
                stats[k].append(
                    (
                        np.sum(mask[0] & mask[k + 1]) / np.sum(mask[0] | mask[k + 1]),
                        mask[k + 1].sum() / mask[0].sum(),
                    )
                )
    return stats
