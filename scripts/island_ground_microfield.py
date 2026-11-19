"""A close soil relief sheet shared by the whole selected island.

Four distinct centimetre-scale surfaces are data, not a camera filter or
tiled greyscale noise. A tiny companion uses the *same* four original
habitat labels as the native transition atlas. Kept separate from the
successful shipped macro layer; no terrain displacement or collision.
"""

from pathlib import Path
import json
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter
from island_habitat_field import habitat

METRES = 0.72
N = 256


def ramp(a, b, x):
    t = np.clip((x - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def noise(r, sd):
    a = gaussian_filter(r.normal(size=(N, N)), sd, mode="wrap")
    return (a - a.mean()) / max(1e-9, a.std())


def dab(layer, x, y, long, wide, theta, bend, power, profile):
    # All shapes wrap exactly across both margins. No solid graphic silhouettes.
    w = int(np.ceil(long + wide + abs(bend) + 2))
    v, u = np.mgrid[-w : w + 1, -w : w + 1]
    i, j = int(x), int(y)
    u = u + i - x
    v = v + j - y
    lx = (u * np.cos(theta) + v * np.sin(theta)) / long
    ly = (-u * np.sin(theta) + v * np.cos(theta) - bend * (1 - lx * lx)) / wide
    if profile == "stalk":
        relief = np.exp(-((ly * 1.4) ** 2)) * np.clip(1 - abs(lx) ** 1.8, 0, 1)
    elif profile == "flake":
        relief = 1 - ramp(
            0.49,
            1.0,
            np.maximum(
                0.84 * abs(ly) + 0.13 * abs(lx), 0.78 * abs(lx) + 0.30 * abs(ly)
            ),
        )
    else:
        relief = np.clip(1 - lx * lx - ly * ly, 0, 1) ** 0.8
    zids = (np.arange(j - w, j + w + 1) % N)[:, None]
    xids = (np.arange(i - w, i + w + 1) % N)[None, :]
    layer[zids, xids] += power * relief


def sheet():
    r = np.random.default_rng(61429)
    zz, xx = (np.mgrid[:N, :N] + 0.5) / N
    # Heights in millimetres; choose sparse relief rather than covering a
    # creature's screen in glitter. Macro pigment is unmodified.
    col = []
    for k in range(4):
        low, mid, grain = noise(r, 13), noise(r, 3.6), noise(r, 0.84)
        if k == 0:  # turf: clean openings between bowed dry blades on crumb soil
            h = 0.42 * mid + 0.23 * grain
            centres = r.uniform(0, N, (14, 2))
            for cx, cy in centres:
                yaw = r.uniform(-np.pi, np.pi)
                for j in range(r.integers(7, 18)):
                    a = yaw + r.normal(0, 0.77)
                    L = r.uniform(4.0, 13.0)
                    x = cx + r.normal(0, 8)
                    y = cy + r.normal(0, 8)
                    dab(
                        h,
                        x % N,
                        y % N,
                        L,
                        r.uniform(0.48, 1.02),
                        a,
                        r.normal(0, 1.7),
                        r.uniform(0.75, 2.4),
                        "stalk",
                    )
            for j in range(310):
                cx, cy = r.uniform(0, N, 2)
                dab(
                    h,
                    cx,
                    cy,
                    r.uniform(0.9, 2.3),
                    r.uniform(0.5, 1.5),
                    r.uniform(0, 6.3),
                    0,
                    r.uniform(0.40, 1.25),
                    "grain",
                )
        elif k == 1:  # intertidal skin: grains collect between two low ripple trains
            line = np.sin(
                2 * np.pi * (3 * xx + 2 * zz) + 0.6 * np.sin(2 * np.pi * (xx - zz))
            )
            h = 0.49 * line + 0.26 * grain + 0.13 * mid
            for j in range(550):
                cx, cy = r.uniform(0, N, 2)
                if r.random() > 0.48 + 0.28 * line[int(cy), int(cx)]:
                    continue
                dab(
                    h,
                    cx,
                    cy,
                    r.uniform(0.8, 2),
                    r.uniform(0.45, 1.05),
                    r.normal(0.5, 0.57),
                    0,
                    r.uniform(0.40, 1.1),
                    "grain",
                )
        elif k == 2:  # exposed rock: a few tiny step edges and quartz inclusions
            a = xx + zz + 0.045 * np.sin(2 * np.pi * zz) + 0.011 * mid
            joint = np.exp(-((abs((a - 0.31 + 0.5) % 1 - 0.5) / 0.007) ** 2)) * ramp(
                -0.55, 0.42, low
            )
            h = 0.25 * mid + 0.15 * grain - 0.8 * joint
            for j in range(255):
                cx, cy = r.uniform(0, N, 2)
                dab(
                    h,
                    cx,
                    cy,
                    r.uniform(0.9, 3.25),
                    r.uniform(0.37, 1.25),
                    r.normal(-0.39, 0.95),
                    0,
                    r.uniform(0.22, 1.20),
                    "flake",
                )
        else:  # grove: needles with overlapping moss scales and damp duff voids
            h = 0.41 * mid + 0.24 * grain
            for cx, cy in r.uniform(0, N, (10, 2)):
                yaw = r.uniform(-np.pi, np.pi)
                for j in range(16):
                    x = (cx + r.normal(0, 7)) % N
                    y = (cy + r.normal(0, 7)) % N
                    dab(
                        h,
                        x,
                        y,
                        r.uniform(2.0, 4.6),
                        r.uniform(0.85, 2.1),
                        yaw + r.normal(0, 0.52),
                        0,
                        r.uniform(0.50, 1.6),
                        "grain",
                    )
            for j in range(97):
                cx, cy = r.uniform(0, N, 2)
                dab(
                    h,
                    cx,
                    cy,
                    r.uniform(4.3, 13),
                    r.uniform(0.34, 0.75),
                    r.uniform(0, 6.3),
                    r.normal(0, 1.15),
                    r.uniform(0.65, 1.65),
                    "stalk",
                )
        col.append(np.clip(h - np.median(h), -3.9, 3.9))
    return np.rint((np.stack(col, -1) * 0.125 + 0.5) * 255).astype("u1")


def build(surface, props, dest):
    dest = Path(dest)
    dest.mkdir(exist_ok=True, parents=True)
    _, labels, (x0, z0, step), _ = habitat(Path(surface), Path(props), True)
    # Label vertices are on 6.25 m intervals and bilinear between neighbouring
    # samples, matching the source four-corner signature rather than guessing a
    # biome from screen tint or a height range. All four weights sum to one.
    weights = np.stack([np.uint8(labels == k) * 255 for k in range(4)], -1)
    Image.fromarray(weights).save(dest / "ground-habitat-weights.png")
    Image.fromarray(sheet()).save(dest / "ground-microheight.png")
    note = dict(
        surface="RGBA channels = turf, sand, stone, grove; linear data, alpha is independent",
        tile_metres=METRES,
        millimetres_per_normalized_unit=8.0,
        mask_width=labels.shape[1],
        mask_height=labels.shape[0],
        mask_origin_xz=[x0, z0],
        mask_spacing=step * 2,
        near_fade_metres=[2.5, 13.0],
        scope="shared close shading, no extra terrain positions or replacement for native macro materials",
    )
    (dest / "microfield.json").write_text(json.dumps(note, indent=2) + "\n")
    return note
