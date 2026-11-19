"""A tiling granodiorite surface photographed at island UV scale (1.35m/tile).

The original pale Voronoi beds look like cracked plaster when they wrap round
all three stones. Build actual dark flakes in a coarse-grained matrix, pale
plagioclase laths, narrow quartz fingers and short arrested joints. Use this
material only on the glacial erratics: the talus and tall sea scarps keep
their selected, separately authored fracture.
"""

from pathlib import Path
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter


def granite(dest, size=512):
    rng = np.random.default_rng(185726)
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    z, x = np.mgrid[:size, :size] / size

    def field(w):
        raw = rng.standard_normal((size, size))
        y = gaussian_filter(raw, w * size, mode="wrap")
        return (y - y.mean()) / max(1e-9, y.std())

    def soft(lo, hi, a):
        t = np.clip((a - lo) / (hi - lo), 0, 1)
        return t * t * (3 - 2 * t)

    domain = field(0.11) * 0.70 + field(0.037) * 0.30
    coarse = field(0.014)
    fine = field(0.0028)
    # Tapered crystalline flecks: not an uncorrelated salt/pepper screen.
    # Add a *few* long intergrown laths rather than a regular cellular net.
    ink = np.zeros_like(x)
    crystal = np.zeros_like(x)
    iron = np.zeros_like(x)
    flake_width = size / 512

    def flecks(layer, count, long, short, favor, light):
        for _ in range(count):
            cx, cz = rng.uniform(0, size, 2)
            ix, iz = int(cx), int(cz)
            if rng.random() > np.clip(0.67 + favor * domain[iz, ix], 0.17, 1):
                continue
            angle = (
                rng.normal(0.13, 0.55)
                if rng.random() < 0.59
                else rng.uniform(-np.pi, np.pi)
            )
            lx = max(0.65, rng.lognormal(np.log(long), 0.39)) * flake_width
            lz = max(0.45, rng.lognormal(np.log(short), 0.28)) * flake_width
            w = int(np.ceil(max(lx, lz) * 1.45)) + 1
            zz, xx = np.mgrid[-w : w + 1, -w : w + 1]
            xx = xx + ix - cx
            zz = zz + iz - cz
            along = (xx * np.cos(angle) + zz * np.sin(angle)) / lx
            across = (-xx * np.sin(angle) + zz * np.cos(angle)) / lz
            # Mineral rhombs have one broken end and a clean short axis.
            shut = 0.84 * abs(along) + 0.28 * abs(across) + 0.17 * along
            dist = np.maximum(0.76 * abs(across) + 0.12 * abs(along), shut)
            cut = (1 - soft(0.57, 1.07, dist)) * rng.uniform(*light)
            rr = (np.arange(iz - w, iz + w + 1) % size)[:, None]
            cc = (np.arange(ix - w, ix + w + 1) % size)[None, :]
            layer[rr, cc] = np.maximum(layer[rr, cc], cut)

    flecks(ink, 3900, 1.8, 0.71, 0.29, (0.32, 0.98))
    flecks(crystal, 2300, 2.7, 1.15, -0.18, (0.28, 1.0))
    flecks(iron, 315, 1.95, 0.96, 0.26, (0.25, 0.82))
    # A short, apatite-white vein branches and disappears in the matrix.
    # Wrapped ridges at opposing edges are continuous, not a tiled fault grid.
    phase = (
        z
        - 0.20 * np.sin(2 * np.pi * x)
        - 0.10 * np.sin(2 * np.pi * (x + z) + 0.78)
        + 0.023 * coarse
    )
    stripe = abs((phase - 0.28 + 0.5) % 1 - 0.5)
    taper = soft(-0.22, 0.63, 0.57 * np.cos(2 * np.pi * (x - 0.66)) + 0.24 * domain)
    quartz = np.exp(-((stripe / (0.004 + 0.006 * taper)) ** 2)) * taper
    line = abs(
        (x + z + 0.045 * np.sin(4 * np.pi * z) + 0.012 * coarse - 0.38 + 0.5) % 1 - 0.5
    )
    stops = soft(0.12, 0.72, 0.58 * np.sin(2 * np.pi * (z + 0.21)) + 0.38 * domain)
    joint = np.exp(-((line / 0.0028) ** 2)) * stops
    rim = np.exp(-((line / 0.013) ** 2)) * stops
    ground = (
        222
        + 2.3 * domain
        + 2.7 * coarse
        + 2.1 * fine
        - 35 * ink
        + 12 * crystal
        + 12 * quartz
        - 7 * joint
    )
    ochre = 4.5 * iron + 2.6 * rim
    rgb = np.stack(
        (ground + 1.3 + ochre, ground + 0.9 - 0.15 * ochre, ground + 1.7 - ochre), -1
    )
    relief = (
        0.18 * gaussian_filter(fine, 0.7, mode="wrap")
        - 0.64 * ink
        + 0.44 * crystal
        + 0.58 * quartz
        - 0.94 * joint
    )
    gx = (np.roll(relief, -1, axis=1) - np.roll(relief, 1, axis=1)) * 0.23
    gy = (np.roll(relief, -1, axis=0) - np.roll(relief, 1, axis=0)) * 0.23
    n = np.stack((-gx, gy, np.ones_like(x)), axis=-1)
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    rough = np.clip(
        0.79
        + 0.02 * domain
        + 0.04 * ink
        - 0.10 * crystal
        - 0.17 * quartz
        + 0.075 * joint,
        0.50,
        0.91,
    )
    orm = np.stack((1 - 0.045 * joint, rough, np.zeros_like(x)), axis=-1)
    for name, a in [
        ("granite", rgb / 255),
        ("granite-normal", n * 0.5 + 0.5),
        ("granite-orm", orm),
    ]:
        Image.fromarray(np.rint(255 * np.clip(a, 0, 1)).astype("u1")).save(
            dest / (name + ".png")
        )
