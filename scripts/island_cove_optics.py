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
        [166, 167, 126],
        [150, 168, 134],
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
        [116, 121, 101],
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


def swell(size=256):
    # A seamless height field of long, shallow swell, locally wind-brushed
    # beads and shorter hooked crests. Vary the shape *along* each wave so
    # shelter does not look like a tiled corrugated iron sea.
    y, x = (np.indices((size, size), dtype=float) + 0.5) / size
    tau = math.tau
    a = (
        tau * (3 * x + 2 * y)
        + 0.67 * np.sin(tau * (x - 2 * y))
        + 0.22 * np.cos(tau * (3 * x + y))
    )
    b = (
        tau * (7 * x - 4 * y)
        + 0.72 * np.cos(tau * (2 * x + y))
        + 0.39 * np.sin(tau * (x + 3 * y))
    )
    patch = smooth(
        -0.83,
        0.84,
        0.67 * np.sin(tau * (2 * x + y))
        + 0.44 * np.cos(tau * (x - 3 * y))
        + 0.20 * np.sin(2 * a),
    )
    # Heights in metres. The island is a cove, not an ocean storm; the small
    # crosswind facets catch glints rather than whitening the whole plane.
    height = 0.072 * (np.sin(a) + 0.21 * np.sin(2 * a - 0.35)) + 0.025 * (
        0.40 + patch * 0.60
    ) * np.sin(b)
    for k, (i, j, h, w) in enumerate(
        (
            (13, 8, 0.013, 1),
            (21, 13, 0.0061, 1),
            (-19, 11, 0.005, 0),
            (38, 17, 0.0034, 1),
            (53, -25, 0.0017, 0),
            (66, 31, 0.0016, 1),
        )
    ):
        phase = (
            tau * (i * x + j * y) + 0.89 * np.sin(a + k) + 0.56 * np.sin(b + k * 0.43)
        )
        crest = np.sin(phase) + 0.20 * np.sin(2 * phase + 0.33 * k)
        height += h * crest * (0.21 + 0.79 * (patch if w else 1 - patch))
    # The 18 m tile is fixed in island space and keeps a normalized mip chain.
    metres = 18.0
    dx = (np.roll(height, -1, 1) - np.roll(height, 1, 1)) * size / (2 * metres)
    dy = (np.roll(height, -1, 0) - np.roll(height, 1, 0)) * size / (2 * metres)
    n = np.stack((-dx, -dy, np.ones_like(dx)), -1)
    n /= np.linalg.norm(n, axis=-1)[..., None]
    return image(127.5 * (1 + n))


def author(dest):
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    for key, img in (("shelf", palette()), ("wind-capillary", swell())):
        img.save(dest / (key + ".png"), compress_level=6)


def compose(ground, dist, coords, lookup):
    # Renderer supplies a real local mean from four bottom samples. Lines of
    # constant height by themselves would create bathymetric contour stripes.
    x, z = coords
    gx = np.hypot(
        (np.roll(ground, -2, 1) - np.roll(ground, 2, 1)) / 4.53,
        (np.roll(ground, -2, 0) - np.roll(ground, 2, 0)) / 5.31,
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
    return image(lookup[v, u])


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
    chart = compose(ground, dist, (x, z), np.asarray(palette()))
    chart.save(out / "shore.png", compress_level=6)
    swell().save(out / "ripple.png", compress_level=6)
    meta.update(
        ripple_metres=18.0,
        roughness=0.31,
        reflectance=0.38,
        art_inputs="Matched artwork: bathymetry from the V1 close triangles, painted optical lookup, world-anchored wind capillaries; DCC, no engine verification.",
    )
    (out / "water.json").write_text(json.dumps(meta, indent=2) + "\n")
