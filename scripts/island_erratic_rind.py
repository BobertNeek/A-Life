"""One shared object-direction atlas: lichen colonies, iron and glacial rub.

Unlike wrapping a photograph of a cracked wall round every face, the
same mineral bed crosses faces, lichen favours the upper weather skin and
damp, warmer patina has a grounded height. Irregular torn patches include
clear openings rather than coating each stone in a uniform green cap.
"""

from pathlib import Path
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter, map_coordinates

MARGIN = 1.44
SIDES = ((0, 1), (0, -1), (1, 1), (1, -1), (2, 1), (2, -1))
AXES = ((2, 1), (2, 1), (0, 2), (0, 2), (0, 1), (0, 1))


def unit(p):
    return p / np.maximum(1e-9, np.linalg.norm(p, axis=-1, keepdims=True))


def soft(a, b, v):
    t = np.clip((v - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def project(p, t, angle):
    raw = unit(
        (p - (p.max(0) + p.min(0)) / 2) / np.maximum(0.01, np.ptp(p, axis=0) / 2)
    )
    c, s = np.cos(angle), np.sin(angle)
    q = raw @ np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])
    q = q[t]
    mid = q.mean(1)
    main = np.abs(mid).argmax(1)
    face = main * 2 + (mid[np.arange(len(mid)), main] < 0)
    uv = np.empty((len(t), 3, 2))
    for i, (axis, sign) in enumerate(SIDES):
        mask = face == i
        pts = q[mask]
        if not len(pts):
            continue
        den = np.maximum(0.30, pts[:, :, axis] * sign)
        a, b = AXES[i]
        u = np.clip(0.5 + pts[:, :, a] / den / (2 * MARGIN), 0.003, 0.997)
        v = np.clip(0.5 - pts[:, :, b] / den / (2 * MARGIN), 0.003, 0.997)
        uv[mask] = np.stack(((i % 3 + u) / 3, (i // 3 + v) / 2), axis=-1)
    return uv.reshape(-1, 2)


def paint(dest, tile=320):
    dest = Path(dest)
    dest.mkdir(exist_ok=True, parents=True)
    rng = np.random.default_rng(316574)
    fields = []
    # Sample a continuous 3D source rather than inventing unrelated edges
    # along the atlas charts. Frequencies are art units on one closed stone.
    for count in range(8):
        p = gaussian_filter(rng.standard_normal((42, 42, 42)), 0.95, mode="wrap")
        fields.append(((p - p.mean()) / p.std()).astype("f"))

    def f(i, vec, scale):
        coords = ((vec * scale + np.array([7.6, 8.1, 13.9])) * 2.3).transpose(2, 0, 1)
        return map_coordinates(fields[i], coords, order=1, mode="grid-wrap")

    sheets = [np.empty((tile * 2, tile * 3, 3), dtype="u1") for k in range(3)]
    v, u = (np.mgrid[:tile, :tile] + 0.5) / tile
    for i, (axis, sign) in enumerate(SIDES):
        vec = np.ones((tile, tile, 3))
        a, b = AXES[i]
        vec[:, :, axis] = sign
        vec[:, :, a] = (u - 0.5) * 2 * MARGIN
        vec[:, :, b] = (0.5 - v) * 2 * MARGIN
        vec = unit(vec)
        x, y, z = vec.transpose(2, 0, 1)
        bed = f(0, vec, 1.0) * 0.58 + f(1, vec, 3.8) * 0.42
        # A definite cut through one flank, changing mineral, rather than
        # endless etched lines: an exposed granitic/migmatitic contact.
        w = 0.38 * x + 0.16 * y + 0.44 * z + 0.13 * bed
        lens = np.exp(-(((w + 0.12) / 0.15) ** 2)) * soft(
            -0.68, 0.35, 0.71 * x - 0.38 * z + 0.20 * f(2, vec, 2.2)
        )
        foil = f(4, vec, 18.5)
        pip = f(7, vec, 42)
        lath = soft(0.56, 1.47, foil * 0.69 + pip * 0.31)
        mica = soft(0.73, 1.47, -foil * 0.54 - f(3, vec, 34) * 0.52)
        vein = np.exp(-(((w + 0.075 + 0.011 * f(6, vec, 7)) / 0.018) ** 2)) * soft(
            -0.52, 0.34, 0.55 * x - 0.59 * z + 0.21 * bed
        )
        fav = (
            0.49 * y
            + 0.14 * z
            + 0.16 * x
            + 0.28 * f(2, vec, 1.6)
            + 0.11 * f(3, vec, 5.8)
            + 0.046 * f(6, vec, 17)
        )
        patch = soft(0.33, 0.44, fav) * soft(-0.52, -0.16, y)
        # Reddish rims of a few older pale grey-green areoles; stippled
        # body, dark sori, and holes allow the actual host to show through.
        rim = soft(0.30, 0.40, fav) - soft(0.40, 0.48, fav)
        pores = soft(0.68, 1.5, f(6, vec, 14) + 0.28 * foil)
        colony = patch * (1 - 0.66 * pores)
        damp = 1 - soft(-0.82, -0.18, y + 0.19 * bed)
        host = (
            207 + 5 * bed + 2.0 * foil - 21 * mica + 11 * lath + 11 * lens + 18 * vein
        )
        iron = soft(0.7, 1.55, f(2, vec, 5.1) + 0.21 * bed) * (0.4 + 0.6 * damp)
        rgb = np.stack(
            (
                host - 0.8 + 8 * iron + 6 * lens,
                host + 1.8 - 3 * iron,
                host + 5 - 6 * iron - 3 * lens,
            ),
            -1,
        )
        rgb *= 1 - 0.095 * damp[..., None]
        rgb += rim[..., None] * np.array([3.5, 1, -2])
        skin = np.stack(
            (
                221 + 4 * bed + 4 * foil,
                223 + 4 * bed + 3 * foil,
                192 + 5 * bed + 3 * foil,
            ),
            axis=-1,
        )
        rgb = rgb * (1 - 0.72 * colony[..., None]) + skin * 0.72 * colony[..., None]
        height = (
            0.25 * foil
            + 0.27 * lath
            - 0.32 * mica
            + 0.85 * vein
            + 0.26 * colony
            - 0.30 * pores * patch
        )
        gy, gx = np.gradient(height)
        n = unit(np.stack((-0.22 * gx, 0.22 * gy, np.ones_like(x)), axis=-1))
        rough = np.clip(
            0.80
            + 0.025 * bed
            + 0.08 * damp
            + 0.08 * colony
            - 0.12 * lath
            - 0.19 * vein,
            0.46,
            0.95,
        )
        orm = np.stack((1 - 0.055 * pores * patch, rough, np.zeros_like(x)), -1)
        row, col = divmod(i, 3)
        for im, val in zip(sheets, (rgb / 255, n * 0.5 + 0.5, orm)):
            im[row * tile : (row + 1) * tile, col * tile : (col + 1) * tile] = np.rint(
                255 * np.clip(val, 0, 1)
            ).astype("u1")
    for name, a in zip(("granite", "granite-normal", "granite-orm"), sheets):
        Image.fromarray(a).save(dest / (name + ".png"))
