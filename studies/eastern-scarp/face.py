"""Elevation-addressed broken stone, restricted to the real exterior face at Gullglass Head.

Top-down pigment alone stretches a footprint under 10 m over nearly 20 m of
elevation. This artwork lives in (along-coast z, height), with a separate
world-xz mask derived from the original V1 surface.
"""

import json, sys, math
from pathlib import Path
import numpy as np
from scipy.ndimage import gaussian_filter
from PIL import Image, ImageDraw

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE.parents[1] / "scripts"), str(HERE.parent / "lanternback-rim")]
from island_inlet_ground import Inlet, ease
from strata import nz

PLAN = (357.0, 433.0, -170.0, -59.0)
SIDE = (-170.0, -59.0, -3.0, 29.0)


def art(native, out):
    out.mkdir(exist_ok=True, parents=True)
    bed = Inlet(native / "island-v1.bin")
    xmin, xmax, zmin, zmax = PLAN
    nx, ny = 608, 888
    x, z = np.meshgrid(
        xmin + (np.arange(nx) + 0.5) * (xmax - xmin) / nx,
        zmin + (np.arange(ny) + 0.5) * (zmax - zmin) / ny,
    )
    y, gx, gz = bed.floor(x, z)
    grade = np.hypot(gx, gz)
    # Never mark the landward face or the top plateau. Leave the grazing
    # upper lip transitional and the actual lower tide apron sparsely bare.
    facing = ease(0.30, 0.63, -gx / np.maximum(0.01, grade))
    frac = nz(x, z, 2.3, 33)
    inside = ease(0, 5, np.minimum(x - xmin, xmax - x)) * ease(
        0, 7, np.minimum(z - zmin, zmax - z)
    )
    strength = (
        ease(0.33, 0.94, grade)
        * facing
        * inside
        * ease(-2.4, 0.28, y)
        * (1 - ease(20.0, 25.0, y + 0.65 * (frac - 0.5)))
    )
    Image.fromarray(np.uint8(np.clip(strength * 255, 0, 255))).save(
        out / "scarp-mask.png", optimize=True
    )
    wd, ht = 888, 512
    u0, u1, v0, v1 = SIDE
    u, y = np.meshgrid(
        u0 + (np.arange(wd) + 0.5) * (u1 - u0) / wd,
        v1 - (np.arange(ht) + 0.5) * (v1 - v0) / ht,
    )
    A = nz(u, y, 9.3, 191) - 0.5
    B = nz(u, y, 3.1, 19) - 0.5
    C = nz(u, y, 1.14, 66) - 0.5
    fine = nz(u, y, 0.23, 12) - 0.5
    # Bedded cleavage changes orientation and dies into unweathered lenses.
    # Unequal layer spacing is not a contour ribbon or terrain silhouette.
    along = u + 0.37 * y
    cross = (
        y
        + 0.22 * (u + 116.0)
        - 2.4 * np.sin((u + 127.0) * 0.065)
        + 3.4 * A
        + 1.08 * B
        + 0.41 * C
    )
    widths = 0.58 + 0.63 * nz(along * 0.61, cross, 5.1, 24)
    fabric = nz(along * 0.31, cross * 1.7, 1.9, 91)
    strands = nz(along * 0.28, cross * 2.3, 0.69, 33)
    seams = (
        1 - ease(0.08, 0.31, np.abs(np.sin(cross * 1.1 + 0.29 * np.sin(cross * 1.8))))
    ) * ease(0.44, 0.69, nz(u * 0.58, y, 3.9, 49))
    mineral = ease(0.34, 0.72, 0.70 * fabric + 0.3 * strands)
    cool = np.array([113.0, 124.0, 125.0])
    pale = np.array([163.0, 159.0, 139.0])
    rgb = cool[None, None, :] + mineral[..., None] * (pale - cool)
    rust = ease(0.42, 0.72, nz(along * 0.75, cross * 0.73, 4.7, 27) + 0.19 * B) * (
        0.40 + 0.60 * ease(3, 16, y)
    )
    rgb = (
        rgb * (1 - 0.54 * rust[..., None])
        + np.array([158.0, 126.0, 97.0]) * 0.54 * rust[..., None]
    )
    # Coarse fracture families intersect and terminate, with discontinuous
    # glancing mineral edges. Draw in native metres, then modulate with grain.
    lines = Image.new("L", (wd, ht))
    lit = Image.new("L", (wd, ht))
    p = ImageDraw.Draw(lines)
    l = ImageDraw.Draw(lit)
    rng = np.random.default_rng(18019)

    def px(a, b):
        return ((a - u0) * wd / (u1 - u0), (v1 - b) * ht / (v1 - v0))

    for i in range(145):
        uu = rng.uniform(u0 - 1, u1)
        vv = rng.uniform(v0, v1 + 1)
        step = rng.uniform(0.42, 0.83)
        tilt = rng.choice([-0.46, 0.32, 0.66]) * rng.uniform(0.46, 1.38)
        pts = [(uu, vv)]
        for j in range(int(rng.integers(3, 13))):
            uu += tilt * step + rng.normal(0, 0.13)
            vv -= step * rng.uniform(0.72, 1.23)
            pts.append((uu, vv))
        amp = int(rng.integers(95, 205))
        thick = int(rng.choice([1, 2, 2, 3]))
        p.line([px(a, b) for a, b in pts], fill=amp, width=thick)
        for a, b in zip(pts, pts[1:]):
            if rng.random() < 0.53:
                l.line(
                    [px(a[0] + 0.065, a[1] + 0.023), px(b[0] + 0.065, b[1] + 0.023)],
                    fill=int(amp * 0.8),
                    width=1,
                )
    cracks = gaussian_filter(np.asarray(lines, dtype="f") / 255, 0.5)
    lip = gaussian_filter(np.asarray(lit, dtype="f") / 255, 0.58)
    cracks *= 0.56 + 0.44 * nz(u, y, 0.89, 52)
    spray = (1 - ease(0.25, 2.35, y + 0.32 * (nz(u, y, 0.9, 37) - 0.5))) * (
        0.61 + 0.39 * ease(0.2, 0.68, nz(u, y, 3.7, 28))
    )
    rgb += (-21 * cracks + 9 * lip + 9 * seams + 11 * C + 13 * fine)[..., None]
    rgb = (
        rgb * (1 - 0.22 * spray[..., None])
        + np.array([-2.0, 1.0, 0.5]) * spray[..., None]
    )
    winter = ease(0.64, 0.85, nz(u, y, 1.68, 51) + 0.22 * nz(u, y, 0.32, 13)) * ease(
        9.0, 22.0, y
    )
    rgb = (
        rgb * (1 - 0.27 * winter[..., None])
        + np.array([149.0, 151.0, 104.0]) * 0.27 * winter[..., None]
    )
    relief = (
        128
        + 15 * (fabric - 0.5)
        + 9 * strands
        - 64 * cracks
        + 27 * lip
        + 22 * fine
        + 7 * seams
    )
    Image.fromarray(np.uint8(np.clip(rgb, 0, 255))).save(
        out / "face-pigment.png", optimize=True
    )
    Image.fromarray(np.uint8(np.clip(relief, 0, 255))).save(
        out / "face-relief.png", optimize=True
    )
    (out / "face.json").write_text(
        json.dumps(
            dict(
                name="Gullglass Head",
                plan=PLAN,
                side=SIDE,
                axes="YZ vertical elevation texture, native z horizontal and true native y up; world xz gate",
                mineral=["mica-grey", "faded iron", "pale interrupted seams"],
                new_vertices=0,
                native_bound=False,
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    art(*(Path(s) for s in sys.argv[1:3]))
