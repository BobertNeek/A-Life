"""World-metric, non-repeating ground response to the actual stitchgrass shoots.

This is gentle fallen thatch and exposed organics, NOT painted ambient shadows.
The old ground colour survives between living mats, including at all tile edges.
"""

import io, sys
from pathlib import Path
import numpy as np
from scipy.spatial import cKDTree
from PIL import Image

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parent), str(H.parents[2] / "lanternback-rim")]
from portable_ground import CUT, sample, linear, srgb
from strata import nz
from island_inlet_ground import ease

BOX = (362.5, 381.25, -90.625, -78.125)


def chart(src, records, paint_dir=None, woody=()):
    a, b, c, d = BOX
    L, R, B, T = CUT
    W, H = 960, 640
    result = np.empty((H, W, 3), "B")
    transfer = np.empty((H, W, 4), "B")
    xx = np.array([[float(r["x"]), float(r["z"])] for r in records])
    tree = cKDTree(xx)
    # True old and proposed woody roots, not dots painted where the camera
    # happens to be. Broad broken soils between running grass shoots are warm
    # organic beds rather than blurry green grass with a varnish on top.
    trees = (
        cKDTree(np.asarray([[float(r["x"]), float(r["z"])] for r in woody]))
        if woody
        else None
    )
    for start in range(0, H, 16):
        end = min(H, start + 16)
        x, z = np.meshgrid(
            a + (np.arange(W) + 0.5) * (b - a) / W,
            c + (np.arange(start, end) + 0.5) * (d - c) / H,
        )
        base = linear(sample(src, (x - L) / (R - L), (z - B) / (T - B))[:, :, :3])
        dist, ind = tree.query(np.column_stack((x.ravel(), z.ravel())), k=3)
        nearest = xx[ind[:, 0]]
        # Downwind short dead blades tangle past surviving stems. Do not
        # fabricate a black contact halo beneath an absent canopy.
        dx = x - nearest[:, 0].reshape(x.shape)
        dz = z - nearest[:, 1].reshape(z.shape)
        fw = 1 / (
            1 + np.exp(np.clip((np.hypot(dx / 0.69, dz / 0.43) - 1.42) * 4, -30, 30))
        )
        amt = fw * 0.78 + 0.13
        edge = ease(0, 0.62, np.minimum(x - a, b - x)) * ease(
            0, 0.62, np.minimum(z - c, d - z)
        )
        m = nz(x, z, 0.31, 230) - 0.5
        flake = nz(x + 5 * m, z - 3 * m, 0.087, 249) - 0.5
        grain = nz(x, z, 0.037, 511) - 0.5
        # Sparse soft-ended fallen blade rims and creased warm wet organics.
        warp = nz(x, z, 0.86, 309) - 0.5
        wind = nz(x * 0.55 + warp * 0.18, z * 1.32 - warp * 0.46, 0.070, 751) - 0.5
        thatch = ease(0.10, 0.38, wind + 0.38 * flake + 0.24 * m) * ease(
            -0.24, 0.12, m + flake * 0.42
        )
        loam = ease(0.08, 0.37, -wind - 0.62 * m + 0.34 * flake)
        change = amt * edge
        stain = np.stack(
            (
                0.042 * thatch - 0.008 * loam,
                0.023 * thatch - 0.013 * loam,
                0.003 * thatch - 0.009 * loam,
            ),
            -1,
        )
        gain = 1 + change * (0.22 * flake + 0.13 * grain + 0.16 * m - 0.22 * loam)
        dense = 1 - ease(0.28, 1.17, dist[:, 2].reshape(x.shape))
        if trees is not None:
            wd, wi = trees.query(np.column_stack((x.ravel() + 0.11, z.ravel() - 0.04)))
            wood = 1 - ease(0.15, 0.85, wd.reshape(x.shape) + m * 0.20 + flake * 0.13)
        else:
            wood = np.zeros_like(x)
        belt = np.maximum(wood * 0.89, dense * (0.59 + 0.37 * fw))
        broken = ease(-0.28, 0.31, m + 0.64 * flake + 0.33 * grain + 0.10)
        peat = np.clip(belt * (0.46 + 0.54 * broken) * edge, 0, 1)
        gain *= 1 - peat * (0.25 + 0.18 * loam)
        # Olive turf survives between communities. Soil interiors can vary
        # from umber to exposed sandy loam, with genuinely placed hulls.
        ox = peat * (0.026 + 0.018 * thatch)
        oy = -peat * (0.022 + 0.014 * loam)
        oz = -peat * (0.015 + 0.010 * loam)
        offset = stain * change[..., None] + np.stack((ox, oy, oz), axis=-1)
        col = np.clip(base * gain[..., None] + offset, 0, 1)
        # RGB stores bounded signed additive linear offset, A the linear
        # multiplier. Border gives exact neutral (128,128,128,128).
        transfer[start:end, :, :3] = np.uint8(
            np.clip(np.rint(128 + offset * 1600), 0, 255)
        )
        transfer[start:end, :, 3] = np.uint8(np.clip(np.rint(128 * gain), 0, 255))
        result[start:end] = np.uint8(np.rint(255 * srgb(col)))
    if paint_dir:
        Image.fromarray(transfer).save(paint_dir / "thatch-transfer.png", optimize=True)
    dst = io.BytesIO()
    Image.fromarray(result).save(dst, format="PNG", optimize=True)
    return dst.getvalue(), [W, H]
