"""Paint suspended fines only where the southern native tidal cut meets the sea.

No new depth, mesh, water, physics, or animation. World map projection and
ocean/chop remain those of the selected original V1 material; shade is built
on top of the actual existing shallow water chart rather than replacing it.
"""

import json
from pathlib import Path
from shutil import copyfile

import numpy as np
from PIL import Image

from island_inlet_ground import Inlet, ease
from island_cove_optics import linear, srgb


def edges(land):
    """Find the original two waterlines. March along the connected inlet,
    so the nearby open sea west of the mouth cannot be mistaken for a bank.
    Past the last intact cross section, spread into the *existing* sea.
    """
    rows = np.arange(365.0, 464.0, 0.50)
    xs = np.arange(44.0, 156.0, 0.125)
    center = 76.0
    left, right = [], []
    for z in rows:
        h = land.floor(xs, z)[0]
        wet = np.flatnonzero(h < 0)
        runs = np.split(wet, np.flatnonzero(np.diff(wet) > 1) + 1)
        candidates = [a for a in runs if len(a) and xs[a[0]] < center < xs[a[-1]]]
        if not candidates:
            raise ValueError(f"Lost mapped V1 inlet at {z}")
        chosen = candidates[0]
        lo, hi = xs[chosen[0]], xs[chosen[-1]]
        if hi - lo > 32:
            break
        left.append(lo)
        right.append(hi)
        center = (lo + hi) * 0.5
    return rows[: len(left)], np.array(left), np.array(right)


def author(surface, selected, destination):
    selected, out = Path(selected), Path(destination)
    out.mkdir(parents=True, exist_ok=True)
    land = Inlet(surface)
    prior = np.array(Image.open(selected / "shore.png").convert("RGB"))
    rgb = prior.copy()
    meta = json.loads((selected / "water.json").read_text())
    height, width = rgb.shape[:2]
    ox, oz = meta["origin_xz"]
    ux, uz = meta["span_xz"][0] / width, meta["span_xz"][1] / height
    ix0, ix1 = int((48 - ox) / ux), int((157 - ox) / ux) + 1
    iz0, iz1 = int((365 - oz) / uz), int((507 - oz) / uz) + 1
    iy, ix = np.mgrid[iz0:iz1, ix0:ix1]
    x, z = ox + (ix + 0.5) * ux, oz + (iy + 0.5) * uz
    h, _, _ = land.floor(x, z)
    depth = np.maximum(0, -h)
    rows, left, right = edges(land)
    mouth = rows[-1]
    after_mouth = np.maximum(z - mouth, 0)
    L = np.interp(z, rows, left) + after_mouth * 0.23
    R = np.interp(z, rows, right) + after_mouth * 0.23
    width_m = R - L
    normal = (x - L) / width_m

    # West z387 and east z459 are not invented side streams. Multiple paths
    # on the actual earthen walls reach the existing waterline at these two
    # outlets. Let material from each feather away downstream in still water.
    # Drift/diffusion scales are artistic, not a hydrology simulation.
    def fan(z0, side, tail, amplitude):
        down = z - z0
        # Fade in before the source rather than a line across the first
        # coarsely sampled one-metre water pixel.
        t = np.maximum(down, 0)
        cross = x - L if side < 0 else R - x
        thread = 0.65 + 0.125 * t + 0.82 * np.sin(t * 0.103 + 0.19 * np.sin(0.21 * t))
        spread = 1.8 + 0.060 * t
        lateral = np.exp(-0.5 * ((cross - thread) / spread) ** 2)
        fine = 0.88 + 0.12 * np.cos(t * 0.25 + 0.59 * cross)
        return amplitude * ease(-4, 2.2, down) * np.exp(-t / tail) * lateral * fine

    cloud = fan(387.0, -1, 39.0, 0.72) + fan(457.8, 1, 20.5, 0.88)
    # Sheltered inner channel has a quiet jade water body, NOT a fictitious
    # deeper centre. The existing true damps/gradient stay visible. Fade this
    # very gently as our cut opens, never colour the ocean outside its estuary.
    cross_fade = ease(-0.11, 0.07, normal) * (1 - ease(0.94, 1.13, normal))
    local = ease(365, 382, z) * (1 - ease(461, 493, z)) * cross_fade
    sub = ease(0.01, 0.40, depth)
    inner = local * sub * (0.38 + 0.11 * ease(0.18, 1.5, depth))
    gold = np.clip(
        cloud * ease(0.05, 0.36, depth) * ease(373, 381, z) * (1 - ease(479, 503, z)),
        0,
        0.86,
    )
    old = linear(prior[iz0:iz1, ix0:ix1].astype(float))
    # Apply ratios in linear colour so depths and shallow edge shading are
    # never overwritten with a flat swatch. Suspended ochre fines raise
    # red/green scattering and gently absorb the blue end in shallow water.
    mix = old * (1 + inner[..., None] * np.array([0.056, -0.24, -0.34]))
    ochre = linear(np.array([157.0, 152.0, 111.0]))
    strength = 0.71 * gold
    mix = mix * (1 - strength[..., None]) + ochre * strength[..., None]
    rgb[iz0:iz1, ix0:ix1] = np.uint8(np.rint(np.clip(srgb(mix), 0, 1) * 255))
    Image.fromarray(rgb).save(out / "shore.png", compress_level=6)
    copyfile(selected / "ripple.png", out / "ripple.png")
    meta[
        "art_inputs"
    ] += " Optional V1 estuary artist: two actual drainage reaches, local suspended fines; no false trench or new water motion."
    (out / "water.json").write_text(json.dumps(meta, indent=2) + "\n")
    receipt = {
        "scope": "original V1 estuary optics only",
        "changed_world_pixels": int(np.any(prior != rgb, axis=2).sum()),
        "world_atlas_pixels": width * height,
        "last_intact_channel_cross_section_z": float(mouth),
        "source_waterline_west": [74.9, 387.25],
        "source_waterline_east": [121.38, 458.88],
        "unchanged": "actual bed/datum, water geometry, ocean crop and wave normals",
        "photograph": "DCC material study, not runtime motion or physics",
    }
    (out / "estuary-note.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return out
