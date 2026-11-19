"""Paint a seamless, metre-scaled close material: crushed stone, litter and moss.

The images are authored offline from ellipsoidal stones, broken stems and folded
blades, rather than enlarged noise. All brushes wrap at the tile edges.
"""

import io
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter


def png(a):
    out = io.BytesIO()
    Image.fromarray(a).save(out, "PNG")
    return out.getvalue()


def finish(n=768):
    rng = np.random.default_rng(83247)
    noise = rng.normal(size=(n, n))

    def field(s):
        v = gaussian_filter(noise, s, mode="wrap")
        return (v - v.mean()) / (v.std() + 1e-8)

    fine, clod, drift = field(1.0), field(5), field(31)
    height = 0.00065 * fine + 0.0015 * clod
    # Pigments are neutral around their mean. World-space tint supplies each
    # actual locality's colour; the detail is not a green decal on a beach.
    tone = np.zeros((n, n, 3), dtype=np.float32) + np.array([178, 181, 174])
    tone += (3.5 * fine + 4.9 * clod + 6.3 * drift)[..., None]
    cavity = 0.94 + 0.022 * np.tanh(clod)
    for typ, count in [("gravel", 760), ("stem", 88), ("leaf", 1600)]:
        for _ in range(count):
            x, z = rng.integers(0, n, 2)
            if typ == "leaf" and drift[z, x] < -0.18:
                continue
            if typ == "gravel" and drift[z, x] > 0.65:
                continue
            a = rng.random() * np.pi * 2
            ca, sa = np.cos(a), np.sin(a)
            if typ == "gravel":
                radius = rng.uniform(1.1, 6.9) * (2.2 if rng.random() < 0.045 else 1)
                length, width = radius, radius * rng.uniform(0.48, 0.98)
                tint = rng.choice(
                    [
                        [201, 199, 181],
                        [147, 151, 150],
                        [213, 207, 190],
                        [162, 150, 135],
                        [183, 191, 182],
                    ]
                )
                lift = radius * 0.00155
            elif typ == "stem":
                length, width = rng.uniform(5, 19), rng.uniform(0.85, 1.8)
                tint = rng.choice([[183, 158, 124], [157, 148, 130], [194, 178, 146]])
                lift = 0.003
            else:
                length, width = rng.uniform(3.5, 12), rng.uniform(0.80, 2.3)
                tint = rng.choice(
                    [
                        [166, 187, 151],
                        [178, 188, 155],
                        [161, 174, 146],
                        [207, 200, 157],
                        [154, 170, 146],
                    ]
                )
                lift = rng.uniform(0.0015, 0.0045)
            r = int(np.ceil(max(length, width) * 1.45 + 3))
            yy, xx = np.mgrid[-r : r + 1, -r : r + 1]
            along = (xx * ca + yy * sa) / length
            cross = (-xx * sa + yy * ca) / width
            if typ == "gravel":
                distance = np.sqrt(along * along + cross * cross) * (
                    1 + 0.11 * np.sin(3 * np.arctan2(cross, along) + a)
                )
            elif typ == "stem":
                distance = np.maximum(abs(along), abs(cross - 0.50 * along * along))
            else:
                distance = np.maximum(
                    abs(along),
                    abs(cross - 0.62 * (1 - along * along))
                    / (0.15 + 0.84 * np.maximum(0, 1 - along * along)),
                )
            mask = np.clip((1.05 - distance) * 3.1, 0, 1)
            relief = lift * np.sqrt(np.clip(1 - distance * distance, 0, 1))
            where = np.ix_(
                (z + np.arange(-r, r + 1)) % n, (x + np.arange(-r, r + 1)) % n
            )
            height[where] += relief
            rim = np.clip(1 - abs(distance - 1.01) * 3.0, 0, 1)
            cavity[where] *= 1 - 0.19 * rim
            pigment = np.asarray(tint) + (2.1 * fine[where] - 5.5 * rim)[..., None]
            tone[where] += mask[..., None] * (pigment - tone[where])
    # A 4.7 m tile; directions are in the object-aligned glTF UV frame.
    du = (np.roll(height, -1, 1) - np.roll(height, 1, 1)) * (n / 9.4)
    dv = (np.roll(height, -1, 0) - np.roll(height, 1, 0)) * (n / 9.4)
    normal = np.stack((-du, -dv, np.ones_like(du)), axis=2)
    normal /= np.linalg.norm(normal, axis=2, keepdims=True)
    orm = np.zeros((n, n, 3), np.uint8)
    orm[:, :, 0] = np.rint(255 * np.clip(cavity, 0, 1))
    orm[:, :, 1] = np.rint(
        255 * np.clip(0.89 - 0.12 * np.tanh(height * 58) + 0.014 * fine, 0.52, 1)
    )
    # Leave enough linear headroom in the multiplying vertex tint for the
    # brightest alpine pigment; otherwise hills darken at the near LOD handoff.
    tone = 220 + 1.20 * (tone - np.array([178, 181, 174]))
    return {
        "close-pigment.png": png(np.uint8(np.clip(np.rint(tone), 0, 255))),
        "close-normal.png": png(
            np.uint8(np.clip(np.rint((normal + 1) * 127.5), 0, 255))
        ),
        "close-roughness.png": png(orm),
    }
