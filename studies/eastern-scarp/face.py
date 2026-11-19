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
    # Regional rock remains massive, not a zebra-striped wall. Foliation is
    # readable in only three separated ore lenses. No stripe crosses the face.
    window = np.zeros_like(y)
    for cu, cy, wx, wy, tilt in (
        (-149.0, 12.8, 6.3, 2.7, 0.26),
        (-115.0, 5.7, 7.8, 2.9, -0.10),
        (-86.0, 15.3, 4.5, 3.6, -0.41),
    ):
        q = np.hypot((u - cu + 0.7 * B) / wx, (y - cy + (u - cu) * tilt + 0.7 * C) / wy)
        window = np.maximum(window, 1 - ease(0.35, 1.21, q))
    mass = 0.72 * nz(u + 2 * B, y + 0.78 * A, 5.1, 72) + 0.28 * nz(
        u * 1.41 + 0.3 * C, y * 0.78, 1.92, 73
    )
    mineral = 0.77 * ease(0.17, 0.85, mass) + window * 0.28 * ease(
        0.36, 0.69, 0.68 * fabric + 0.32 * strands
    )
    cool = np.array([115.0, 123.0, 121.0])
    pale = np.array([154.0, 147.0, 132.0])
    rgb = cool + (pale - cool) * mineral[..., None]
    seams *= window
    rust = ease(0.56, 0.81, nz(u * 0.86, y * 1.11, 4.3, 29) + 0.13 * A) * (
        0.24 + 0.60 * ease(3, 17, y)
    )
    rgb = (
        rgb * (1 - 0.52 * rust[..., None])
        + np.array([153.0, 115.0, 88.0]) * 0.52 * rust[..., None]
    )
    # Diffuse, broken rain seepage follows gullies beneath a few genuine
    # upper ledges. Non-uniform length, width, colour, and lateral position.
    seep = np.zeros_like(y)
    for cu, top, reach, ww in (
        (-157, 18, 8, 0.32),
        (-139, 17, 12, 0.38),
        (-133, 21, 7, 0.78),
        (-118, 19, 9, 0.38),
        (-106, 20, 15, 0.46),
        (-99, 18, 11, 0.53),
        (-78, 19, 9, 0.72),
    ):
        t = (top - y) / reach
        centre = cu + 0.51 * np.sin(t * 2.2 + cu) + 0.32 * C + 0.30 * t
        seep += (
            np.exp(
                -(((u - centre) / (ww * (0.5 + 0.47 * np.clip(t, 0, 1)) + 0.09)) ** 2)
            )
            * ease(-0.09, 0.18, t)
            * (1 - ease(0.67, 1.06, t))
        )
    rgb -= np.minimum(1.0, seep)[..., None] * np.array([15.0, 13.0, 8.0])
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
        + (5 + 14 * window) * (fabric - 0.5)
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
    # Small-metre non-colour tile; readable one centimetre mica flecks are
    # physically separate from the giant same-colour masses above them.
    nn = 384
    rng = np.random.default_rng(735700)
    field = rng.normal(size=(nn, nn)).astype("f")
    a0 = gaussian_filter(field, 2.7, mode="wrap")
    a0 /= max(0.01, a0.std())
    a1 = gaussian_filter(field, (0.83, 1.42), mode="wrap")
    a1 /= max(0.01, a1.std())
    ht = 125 + 10 * a0 + 11 * a1 + 5 * field
    tile = np.empty((nn, nn, 3), "f")
    tile[:] = [173, 174, 165]
    tile += (11 * a0 + 10 * a1 + 6 * field)[..., None]
    stone = Image.fromarray(np.uint8(np.clip(tile, 0, 255)))
    rel = Image.fromarray(np.uint8(np.clip(ht, 0, 255)))
    pen = ImageDraw.Draw(stone)
    ch = ImageDraw.Draw(rel)
    for j in range(1140):
        x0, y0 = rng.uniform(0, nn, 2)
        length = rng.uniform(1.5, 9.4)
        th = rng.uniform(0.72, 2.8)
        ang = rng.normal(0.33, 0.47)
        vv = np.array([math.cos(ang), math.sin(ang)])
        tt = np.array([-vv[1], vv[0]])
        shape = np.asarray(
            [
                -0.52 * length * vv,
                -0.19 * length * vv + tt * th,
                0.47 * length * vv + tt * 0.32 * th,
                0.43 * length * vv - tt * 0.52 * th,
                -0.13 * length * vv - tt * 0.64 * th,
            ]
        ) + [x0, y0]
        warm = j % 6 == 0
        value = rng.integers(95, 137) if j % 3 == 0 else rng.integers(169, 219)
        tint = (int(value + 8), int(value), int(value - 16 if warm else value - 4))
        for sx in (0, nn) if x0 < 12 else (-nn, 0) if x0 > nn - 12 else (0,):
            for sy in (0, nn) if y0 < 12 else (-nn, 0) if y0 > nn - 12 else (0,):
                pl = [tuple(p) for p in shape + [sx, sy]]
                pen.polygon(pl, fill=tint)
                ch.polygon(pl, fill=int(164 if j % 3 else 105))
                ch.line(pl[:2], fill=184, width=1)
    stone.save(out / "mica-colour.png", optimize=True)
    rel.save(out / "mica-height.png", optimize=True)
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
