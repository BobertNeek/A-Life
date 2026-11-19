"""Design a discontinuous clear-water / spent-wave fringe on *original submerged* V1.

One local warped metric atlas is enough; no wave meshes, no elevated ribbons.
This sheet is a future isolated art option, never secretly bound in the
Gullglass geology or cast A/B photographs.
"""

from pathlib import Path
import json, sys
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parent
sys.path[:0] = [
    str(ROOT.parents[2] / "scripts"),
    str(ROOT.parents[1] / "lanternback-rim"),
]
from island_inlet_ground import Inlet, ease
from strata import nz

PLAN = (363.0, 432.0, -165.0, -64.0)


def pick_heads(table):
    z = table["z"]
    prom = table["local_prominence_m"]
    okay = (z > z.min() + 5) & (z < z.max() - 5)
    # Select three genuine local noses, separately from the artist's previously
    # measured micro-shell recesses. Never put a crest on every rocky tooth.
    picks = []
    for i in np.argsort(prom)[::-1]:
        if not okay[i] or any(abs(z[i] - z[j]) < 17.0 for j in picks):
            continue
        picks.append(int(i))
        if len(picks) == 3:
            break
    return sorted(picks, key=lambda i: z[i])


def paint(native, field):
    bed = Inlet(native / "island-v1.bin")
    a = np.genfromtxt(field / "waterline.csv", names=True, delimiter=",")
    picks = pick_heads(a)
    xmin, xmax, zmin, zmax = PLAN
    W, H = 828, 1212
    # Rows are built independently to keep memory below the art lane reserve.
    colour = np.zeros((H, W, 4), "B")
    response = np.zeros((H, W, 3), "B")
    xx = xmin + (np.arange(W) + 0.5) * (xmax - xmin) / W
    spectrum = [(0.74, 0.27, 7.3), (0.54, 0.35, 4.8), (0.63, 0.23, 5.3)]
    active = 0
    foam = 0
    supported = 0
    for start in range(0, H, 28):
        row = np.arange(start, min(start + 28, H))
        zz = zmin + (row + 0.5) * (zmax - zmin) / H
        x, z = np.meshgrid(xx, zz)

        def column(k, position):
            return np.interp(position, a["z"], a[k])

        # Project each water pixel to its nearest normal foot on the GIVEN
        # original curve. Three fixed-point steps converge on this smooth arc.
        toe = z.copy()
        for k in range(3):
            cx = column("coast_x", toe)
            nx = column("normal_x", toe)
            nz_ = column("normal_z", toe)
            toe = z - (x - cx) * nz_ / np.maximum(0.55, nx)
        cx = column("coast_x", toe)
        nx = column("normal_x", toe)
        off = (x - cx) / np.maximum(nx, 0.55)
        h, _, _ = bed.floor(x, z)
        depth = -h
        valid = (depth > 0) & (off > 0) & (toe > a["z"].min()) & (toe < a["z"].max())
        near = 1 - ease(0.56, 1.55, depth)
        supported += int((valid & (depth <= 1.55)).sum())
        q = nz(toe, off, 0.47, 411) - 0.5
        r = nz(toe, off, 0.15, 415) - 0.5
        w = nz(toe, off, 2.15, 431) - 0.5
        sea = np.zeros_like(z)
        lips = sea.copy()
        arc_rough = sea.copy()
        for j, i in enumerate(picks):
            L0, L1, span = spectrum[j]
            cu = a["z"][i]
            # Rounded, open crescent at a true prominence, then a narrower
            # fragment nearby. Major axis follows the alongshore coordinate;
            # pattern is truncated down-wave rather than meeting in a ring.
            t = (toe - cu) / span
            sh = np.maximum(0, 1 - t * t)
            gate = (
                np.maximum(0, 1 - np.abs(t) ** 2.5)
                * ease(-1.04, -0.59, t)
                * (1 - ease(0.75, 1.12, t))
            )
            reach = (column("depth065_x", toe) - cx) / np.maximum(nx, 0.55)
            d1 = (0.60 + L0 * sh + L1 * t + 0.13 * w) * reach
            warp = 0.18 * q + 0.075 * r + 0.11 * np.sin(2.7 * t + j * 1.27)
            delta = off - d1 - warp
            # Presence in the optical shallows does not imply constantly
            # breaking surf. Irregular calm holes leave most water unchanged.
            run = ease(-0.05, 0.24, 0.77 * w + 0.57 * q + 0.26 * sh - 0.08)
            line = np.exp(-((delta / (0.07 + 0.10 * sh)) ** 2)) * gate * run
            grains = ease(0.02, 0.30, q + 0.43 * r)
            lace = (
                np.exp(-(((delta - 0.37 - 0.12 * w) / 0.38) ** 2))
                * gate
                * grains
                * (0.25 + 0.35 * run)
            )
            sea = np.maximum(
                sea,
                gate * near * ease(0.05, 0.55, sh) * np.exp(-abs(delta) / 2.1) * 0.53,
            )
            lips = np.maximum(lips, line * 0.81 + lace * 0.25)
            arc_rough = np.maximum(arc_rough, line * 0.38 + lace * 0.20)
        # Alpha controls a local optical mix, not the transparency of the
        # actual ocean. Nothing is applied over land or offshore dark water.
        hgate = valid * ease(0.015, 0.14, depth) * (1 - ease(0.95, 1.65, depth))
        smooth = sea * hgate
        crest = np.minimum(1, lips) * hgate
        alpha = np.minimum(0.76, smooth * 0.52 + crest)
        rgb = np.array([110.0, 158.0, 151.0]) + np.minimum(
            1, crest / (np.maximum(0.01, alpha))
        )[:, :, None] * np.array([86.0, 52.0, 40.0])
        rgb += (q * 7 + r * 4)[:, :, None]
        colour[row] = np.uint8(np.clip(np.dstack([rgb, 255 * alpha]), 0, 255))
        # R = mildly calm stock capillary normal in protected swash pockets;
        # G = spent crest roughness. B unused and zero.
        response[row] = np.uint8(
            np.clip(
                np.dstack([255 * smooth, 255 * arc_rough * hgate, np.zeros_like(z)]),
                0,
                255,
            )
        )
        active += int((alpha > 0.04).sum())
        foam += int((crest > 0.22).sum())
    # Pillow top row maps to Blender V=1; row above stores smallest native z,
    # so material uses corresponding Blender Y-to-V plan transform.
    Image.fromarray(colour).save(field / "wash-colour.png", optimize=True)
    Image.fromarray(response).save(field / "wash-response.png", optimize=True)
    (field / "wash.json").write_text(
        json.dumps(
            dict(
                plan=PLAN,
                noses_native_z=[float(a["z"][i]) for i in picks],
                original_submerged_shallows_texels=supported,
                meaningfully_tinted_texels=active,
                spent_crest_texels=foam,
                stock_material_underneath=True,
                terrain_or_tide_changed=False,
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    paint(*map(Path, sys.argv[1:3]))
