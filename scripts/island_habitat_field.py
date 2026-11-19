"""Select metre-scaled habitat signatures from immutable island slopes and real tree stands."""

import json
import struct
from collections import Counter
from pathlib import Path

import numpy as np
from scipy.ndimage import gaussian_filter

TURF, SAND, STONE, GROVE = range(4)
PATCH_CELLS = 2


def habitat(surface, placements):
    raw = Path(surface).read_bytes()
    w, d, x0, z0, step, _ = struct.unpack_from("<IIfffI", raw, 8)
    h = np.frombuffer(raw, "<f4", w * d, 32).reshape(d, w)
    gz, gx = np.gradient(gaussian_filter(h, 0.85), step)
    grade = np.hypot(gx, gz)
    xx = x0 + np.arange(w)[None, :] * step
    zz = z0 + np.arange(d)[:, None] * step
    # A real grove is a sustained group of stems, not proximity to one flower.
    stems = np.zeros_like(h)
    for prop in json.loads(Path(placements).read_text()):
        if prop["kind"] not in ("Broadleaf", "Conifer", "Sapling"):
            continue
        px, _, pz = prop["position"]
        i, j = round((px - x0) / step), round((pz - z0) / step)
        if 0 <= i < w and 0 <= j < d:
            stems[j, i] += prop["scale"] ** 1.4
    canopy = gaussian_filter(stems, 4.4) * 98
    broken = np.sin(xx / 31 + 0.68 * np.cos(zz / 27)) + 0.6 * np.sin(zz / 18 - xx / 41)
    label = np.full(h.shape, TURF, np.uint8)
    label[(canopy > 1.38 + 0.16 * broken) & (h > 3.25)] = GROVE
    coast = np.hypot(xx / 445, zz / 535) > 0.64
    label[(h < 2.85 + 0.85 * broken) & (grade < 0.58) & coast] = SAND
    label[
        (grade > 0.68 + 0.07 * broken) | ((h > 50) & (grade > 0.48 + 0.06 * broken))
    ] = STONE
    label[h < 0] = SAND
    return h, label[::PATCH_CELLS, ::PATCH_CELLS], (x0, z0, step)


def signature(labels, ix, iz):
    a = labels[iz : iz + 2, ix : ix + 2]
    corners = tuple(map(int, (a[0, 0], a[0, 1], a[1, 0], a[1, 1])))
    seed = (ix * 1597334677 ^ iz * 3812015801) & 0xFFFFFFFF
    variation = (seed ^ (seed >> 15)) % 4 if len(set(corners)) == 1 else 0
    return corners + (variation,)


def signatures(labels):
    return Counter(
        signature(labels, i, j)
        for j in range(labels.shape[0] - 1)
        for i in range(labels.shape[1] - 1)
    )


def orient_uv(u, v, turn):
    if turn & 4:
        u = 1 - u
    for _ in range(turn & 3):
        u, v = 1 - v, u
    return u, v


def canonical(key):
    """Shared layouts for rotation/reflection equivalent transitions; 142 -> ~40 tiles."""
    cases = []
    for turn in range(8):
        tile = [None] * 4
        for i, ground in enumerate(key[:4]):
            u, v = orient_uv(i % 2, i // 2, turn)
            tile[int(v) * 2 + int(u)] = ground
        cases.append((tuple(tile) + (key[4],), turn))
    return min(cases)
