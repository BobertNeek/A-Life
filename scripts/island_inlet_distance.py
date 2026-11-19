"""Fold only the creek-bank paint into its corresponding far-terrain texels.

Original PBR geometry is a near-surface fit. Distance meshes get no decoration
or extra draw call. Downsample masked *linear* colour and preserve every other
source pixel, including parts that differ between the V1 and ridge certificates.
"""

from pathlib import Path
import numpy as np
from PIL import Image
from island_inlet_ground import X0, X1, Z0, Z1, Inlet


def linear(c):
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def srgb(c):
    return np.where(c <= 0.0031308, 12.92 * c, 1.055 * c ** (1 / 2.4) - 0.055)


def far_pigment(base, variant, bed, original, destination):
    base, variant = Inlet(base), Inlet(variant)
    rgb = np.array(Image.open(original).convert("RGB"))
    ink = np.array(Image.open(bed).convert("RGBA"))
    ih, iw = ink.shape[:2]
    mh, mw = rgb.shape[:2]
    dx = (base.h.shape[1] - 1) * base.s / mw
    dz = (base.h.shape[0] - 1) * base.s / mh
    x0, x1 = np.clip(
        [np.floor((X0 - base.x0) / dx), np.ceil((X1 - base.x0) / dx)], 0, mw
    ).astype(int)
    z0, z1 = np.clip(
        [np.floor((Z0 - base.z0) / dz), np.ceil((Z1 - base.z0) / dz)], 0, mh
    ).astype(int)
    kz, kx = np.mgrid[z0:z1, x0:x1]
    old = linear(rgb[z0:z1, x0:x1] / 255.0)
    out = np.zeros_like(old)
    hh = base.h == variant.h
    safe = hh[:-1, :-1] & hh[1:, :-1] & hh[:-1, 1:] & hh[1:, 1:]
    # Twelve aligned subsamples/axis keep sparse shore coverage at far mips,
    # rather than expanding brightly-coloured boxes around individual rocks.
    pitch = 12
    for iz in range(pitch):
        for ix in range(pitch):
            x = base.x0 + (kx + (ix + 0.5) / pitch) * dx
            z = base.z0 + (kz + (iz + 0.5) / pitch) * dz
            a = np.clip(((x - X0) / (X1 - X0) * iw).astype(int), 0, iw - 1)
            b = np.clip(((z - Z0) / (Z1 - Z0) * ih).astype(int), 0, ih - 1)
            near = ink[b, a]
            world_i = np.floor((x - base.x0) / base.s).astype(int)
            world_j = np.floor((z - base.z0) / base.s).astype(int)
            covered = (
                (x >= X0)
                & (x < X1)
                & (z >= Z0)
                & (z < Z1)
                & safe[world_j, world_i]
                & (near[..., 3] >= 110)
            )
            out += np.where(covered[..., None], linear(near[..., :3] / 255.0), old)
    result = np.uint8(np.rint(np.clip(srgb(out / (pitch * pitch)), 0, 1) * 255))
    replaced = np.any(result != rgb[z0:z1, x0:x1], axis=-1)
    rgb[z0:z1, x0:x1] = result
    destination = Path(destination)
    destination.parent.mkdir(exist_ok=True, parents=True)
    Image.fromarray(rgb).save(destination, compress_level=8)
    return {
        "original_pixel_rect_xz": [int(x0), int(z0), int(x1), int(z1)],
        "far_pixels_repainted": int(replaced.sum()),
        "far_atlas_pixels": int(mh * mw),
    }
