"""A painted optical lookup and matched short-wave normal for Qualia's sheltered sea.

The optical chart is a world-aligned material drawn from the physical V1 surface,
never a screen-space outline. Other terrain bindings keep their dynamic colour path.
"""

import math
from pathlib import Path
import numpy as np
from PIL import Image
from scipy.ndimage import distance_transform_edt

# Water covers an immense plane. Shape the coast with the submerged surface,
# not an opaque cartoon outline. Stops are metres of true water above ground.
DEPTHS = np.array([0.0, 0.32, 1.05, 2.4, 5.2, 10, 18, 32], dtype=float)
SHELL = np.array(
    [
        [157, 171, 146],
        [142, 169, 148],
        [109, 169, 148],
        [71, 153, 149],
        [42, 127, 140],
        [33, 104, 127],
        [31, 88, 113],
        [29, 77, 108],
    ],
    dtype=float,
)
ROCK = np.array(
    [
        [114, 128, 111],
        [108, 132, 117],
        [84, 141, 130],
        [57, 132, 135],
        [44, 114, 129],
        [34, 99, 121],
        [31, 86, 111],
        [29, 77, 108],
    ],
    dtype=float,
)


def smooth(a, b, x):
    t = np.clip((x - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def srgb(linear):
    return np.where(
        linear <= 0.0031308,
        12.92 * linear,
        1.055 * np.maximum(linear, 0) ** (1 / 2.4) - 0.055,
    )


def linear(code):
    x = code / 255
    return np.where(x <= 0.04045, x / 12.92, ((x + 0.055) / 1.055) ** 2.4)


def image(rgb):
    return Image.fromarray(np.rint(rgb).clip(0, 255).astype("u1"))


def palette():
    # Nonlinear parameter gives the first two metres far more tonal resolution.
    t = np.arange(256) / 255
    meters = 32 * t**2
    a, b = (
        np.column_stack(
            [np.interp(meters, DEPTHS, linear(arr[:, c])) for c in range(3)]
        )
        for arr in (SHELL, ROCK)
    )
    mineral = (np.arange(64) / 63)[:, None, None]
    return image(255 * srgb(a[None, :] * (1 - mineral) + b[None, :] * mineral))


SWELL_TILE_METRES = 58.0


def swell(size=256):
    # A short-crested directional spectrum avoids the identifiable repeated
    # chevrons from a pair of phase-modulated sine waves. The 58 m swatch has
    # 46 independent large, middle and wind-scuffed modes. No bright motif or
    # camera-dependent sunlight is painted into the normal.
    y, x = (np.indices((size, size), dtype=float) + 0.5) / size
    tau = math.tau
    rng = np.random.default_rng(5308216)
    # A periodic domain warp rotates the local group in shelter without a
    # fold: small compared with a fundamental wavelength at either boundary.
    u = (
        x
        + 0.0048 * np.sin(tau * (x + 2 * y) + 0.91)
        + 0.0033 * np.sin(tau * (3 * x - y) - 0.55)
    )
    v = (
        y
        + 0.0059 * np.sin(tau * (2 * x + y) - 0.72)
        + 0.0027 * np.cos(tau * (x - 3 * y) + 0.80)
    )
    height = np.zeros_like(x)
    used = set()
    for count, k0, k1, slope, spread in (
        (8, 3.7, 8.9, 0.050, 0.42),
        (15, 9, 22, 0.059, 0.48),
        (15, 22, 47, 0.048, 0.57),
        (8, 47, 79, 0.027, 0.71),
    ):
        for j in range(count):
            while True:
                k = rng.uniform(k0, k1)
                angle = 0.41 + rng.normal(0, spread)
                if j % 7 == 6:
                    angle += 1.17
                kx, kz = round(k * math.cos(angle)), round(k * math.sin(angle))
                if (kx, kz) not in used:
                    break
            used.add((kx, kz))
            phase = tau * (kx * u + kz * v) + rng.uniform(-math.pi, math.pi)
            # Short crests arrive in different groups. Their envelopes are
            # independent; several waves never peak in the same grid lane.
            env = 0.86 + 0.14 * np.sin(
                tau * (rng.integers(1, 4) * x + rng.integers(-3, 4) * y)
                + rng.uniform(-3.1, 3.1)
            )
            omega = tau * math.hypot(kx, kz) / SWELL_TILE_METRES
            amp = slope * rng.uniform(0.71, 1.36) / (omega * math.sqrt(count * 0.5))
            wave = np.sin(phase)
            if k1 < 48:
                wave += 0.10 * np.sin(2 * phase + 0.28)
            height += amp * env * wave
    dx = (
        (np.roll(height, -1, 1) - np.roll(height, 1, 1))
        * size
        / (2 * SWELL_TILE_METRES)
    )
    dy = (
        (np.roll(height, -1, 0) - np.roll(height, 1, 0))
        * size
        / (2 * SWELL_TILE_METRES)
    )
    n = np.stack((-dx, -dy, np.ones_like(dx)), -1)
    n /= np.linalg.norm(n, axis=-1)[..., None]
    return image(127.5 * (1 + n))


def author(dest):
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    for key, img in (("shelf", palette()), ("ripple", swell())):
        img.save(dest / (key + ".png"), compress_level=6)


def compose(ground, dist, coords, lookup):
    # Sample a real local mean from four nearby parts of the bed. Lines of
    # constant height by themselves would create bathymetric contour stripes.
    x, z = coords
    four_x = 4 * (x[0, 1] - x[0, 0])
    four_z = 4 * (z[1, 0] - z[0, 0])
    gx = np.hypot(
        (np.roll(ground, -2, 1) - np.roll(ground, 2, 1)) / four_x,
        (np.roll(ground, -2, 0) - np.roll(ground, 2, 0)) / four_z,
    )
    shoulder = (
        ground
        - (
            np.roll(ground, 8, 0)
            + np.roll(ground, -8, 0)
            + np.roll(ground, 8, 1)
            + np.roll(ground, -8, 1)
        )
        * 0.25
    )
    broken = (
        0.50
        + 0.22 * np.sin(x * 0.083 + np.sin(z * 0.071))
        + 0.17 * np.cos(z * 0.053 + x * 0.011)
        + 0.11 * np.sin(x * 0.21 - z * 0.17)
    )
    fac = smooth(-0.05, 0.75, 0.69 * gx + 0.21 * shoulder + 0.38 * broken)
    depth = np.maximum(0, -ground)
    shelf = 1 - smooth(12, 80, dist)
    effective = depth + (32 - depth) * (1 - shelf)
    u = np.rint(255 * np.sqrt(np.clip(effective / 32, 0, 1))).astype(int)
    v = np.rint(63 * fac).astype(int)
    col = linear(lookup[v, u].astype(float))
    # Thin retreat lines stay visible at guardian height, but the water does
    # not become a complete white outline around each metre of coast.
    beat = (
        0.49
        + 0.26 * np.sin(0.092 * x - 0.068 * z + 1.14 * np.sin(0.026 * z))
        + 0.18 * np.cos(0.14 * z + 0.061 * x)
    )
    front = 0.12 + 0.15 * beat + 0.057 * np.sin(0.31 * x - 0.27 * z)
    lip = (1 - smooth(0.025, 0.23, np.abs(depth - front))) * smooth(0.08, 0.81, beat)
    residual = (1 - smooth(0.20, 0.62, depth)) * (0.08 + 0.09 * beat)
    wash = (
        (lip * 0.28 + residual)
        * smooth(0.002, 0.081, depth)
        * (1 - smooth(4, 15, dist))
    )
    col = (
        col * (1 - wash[..., None])
        + linear(np.array([192.0, 212.0, 196.0])) * wash[..., None]
    )
    return image(255 * srgb(col))


def world_paint(surface, metadata, out):
    """Compose an actual DCC material from the original V1 triangles, never move the sea."""
    import json
    from build_island_strandline import Beach

    author(out)
    meta = json.loads((metadata / "water.json").read_text())
    land = Beach(surface)
    n = 1024
    j, i = np.indices((n, n), dtype="f")
    x0, z0 = meta["origin_xz"]
    wx, wz = meta["span_xz"]
    x, z = x0 + (i + 0.5) * wx / n, z0 + (j + 0.5) * wz / n
    u, v = (x - land.x0) / land.s, (z - land.z0) / land.s
    valid = (
        (u >= 0) & (u <= land.h.shape[1] - 1) & (v >= 0) & (v <= land.h.shape[0] - 1)
    )
    ii = np.clip(np.floor(u).astype(int), 0, land.h.shape[1] - 2)
    jj = np.clip(np.floor(v).astype(int), 0, land.h.shape[0] - 2)
    u, v = np.clip(u - ii, 0, 1), np.clip(v - jj, 0, 1)
    a, b, c, d = (
        land.h[jj, ii],
        land.h[jj, ii + 1],
        land.h[jj + 1, ii + 1],
        land.h[jj + 1, ii],
    )
    ground = np.where(
        valid,
        np.where(
            v >= u, a * (1 - v) + d * (v - u) + c * u, a * (1 - u) + b * (u - v) + c * v
        ),
        -32.0,
    )
    dist = distance_transform_edt(ground < 0, sampling=(wz / n, wx / n))
    with Image.open(out / "shelf.png") as colours:
        chart = compose(ground, dist, (x, z), np.asarray(colours))
    chart.save(out / "shore.png", compress_level=6)
    meta.update(
        ripple_metres=SWELL_TILE_METRES,
        roughness=0.36,
        reflectance=0.36,
        art_inputs="Blender artwork: V1 triangle bathymetry, painted optical lookup and a world-aligned wind/capillary normal. Native binding and cost not run this week.",
    )
    (out / "water.json").write_text(json.dumps(meta, indent=2) + "\n")
