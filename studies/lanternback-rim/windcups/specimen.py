"""Three close-only forms for the planted pennon species.

The mother rosette and curved flowering stems are grown as one model. The
capsules are stitched cups with an opening seam, not separate upward triangles.
Unlit ivory flecks are material, not lamps.
"""

from pathlib import Path
import math, sys, json
import numpy as np

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE.parent), str(HERE.parents[2] / "scripts")]
from fissures import Piece, export

Y = np.array([0.0, 1.0, 0.0])


def unit(a):
    return a / max(np.linalg.norm(a), 1e-8)


def pol(a):
    return np.array([math.cos(a), 0, math.sin(a)])


def B(g, points, colors, faces):
    # Keep winding through the cupped lower wall. Mineral surfaces are flipped
    # upwards by the generic Piece.face helper; a closed fruit is not ground.
    start = len(g.p)
    g.p.extend(points)
    g.c.extend(colors)
    g.t.extend((start + i, start + j, start + k) for i, j, k in faces)


def grid(g, rows, colors):
    if g.middle:
        cols = len(rows[0])
        nr = len(rows)
        # Retain both silhouette edges, the fold spine, cup head height and
        # the near tip. Closed collars need three different circumferential
        # vertices rather than an edge + spine triangle degeneracy.
        closed = np.allclose(rows[0][0], rows[0][-1], atol=1e-7)
        jj = (
            sorted({0, (cols - 1) // 3, 2 * (cols - 1) // 3, cols - 1})
            if closed
            else sorted({0, (cols - 1) // 2, cols - 1})
        )
        ii = (
            sorted({0, (nr - 1) // 3, 2 * (nr - 1) // 3, nr - 1})
            if nr >= 5
            else list(range(nr))
        )
        rows = [[rows[i][j] for j in jj] for i in ii]
        colors = [[colors[i][j] for j in jj] for i in ii]
    w = len(rows[0])
    f = []
    for j in range(len(rows) - 1):
        for k in range(w - 1):
            a = j * w + k
            f.extend(((a, a + 1, a + w + 1), (a, a + w + 1, a + w)))
    B(g, np.concatenate(rows), np.concatenate(colors), f)


def tube(g, centers, radii, base, sides=6):
    ring = []
    cols = []
    for i, (p, r) in enumerate(zip(centers, radii)):
        axis = unit(centers[min(i + 1, len(centers) - 1)] - centers[max(i - 1, 0)])
        a = unit(np.cross(axis, [1, 0, 0]))
        b = unit(np.cross(axis, a))
        ring.append(
            [
                p
                + (
                    math.cos(t * math.tau / sides) * a
                    + math.sin(t * math.tau / sides) * b
                )
                * r
                for t in range(sides + 1)
            ]
        )
        cols.append(
            [
                base
                * (
                    0.75
                    + 0.25 * i / (len(centers) - 1)
                    + 0.07 * math.cos(t * math.tau / sides)
                )
                for t in range(sides + 1)
            ]
        )
    grid(g, ring, cols)


def closed_cup(g, origin, lean, size, age, pigment, seed):
    # Four interlocking, boat-shaped valves hinge at a fluted green calyx.
    # Age opens their mouths, then turns the dry margins back toward the wind.
    axis = unit(Y + lean)
    a = unit(np.cross(axis, [0, 0, 1]))
    b = unit(np.cross(axis, a))

    def radial(t):
        return math.cos(t) * a + math.sin(t) * b

    stem = origin
    tube(
        g,
        [stem - Y * 0.009, stem, stem + axis * size * 0.21],
        [size * 0.115, size * 0.21, size * 0.31],
        np.array([112.0, 126.0, 82.0]),
        6,
    )
    for j in range(4):
        aa = seed + j * math.tau / 4
        profile = []
        cols = []
        for t in (0, 0.22, 0.48, 0.69, 0.86, 1.0):
            tau = aa + 0.23 * t + 0.11 * math.sin(aa + 0.6) * t * t
            # Body cups inwards near tip until the split seam opens.
            wide = (0.04 + 0.75 * math.sin(math.pi * (0.04 + 0.90 * t)) ** 0.88) * (
                1 - 0.14 * age
            )
            r = size * (
                0.16 + 0.55 * math.sin(math.pi * t * 0.88) + (0.56 * age - 0.03) * t**3
            )
            y = size * (0.10 + 0.94 * t - 0.43 * age * t**3)
            rr = []
            cc = []
            for cross in (-1.0, -0.48, 0, 0.48, 1.0):
                edge = abs(cross)
                flute = (
                    size
                    * (0.052 * math.cos(cross * math.pi * 1.9) + 0.04 * (1 - edge))
                    * math.sin(math.pi * t)
                )
                rr.append(
                    origin
                    + axis * (y + 0.051 * size * edge * math.sin(math.pi * t))
                    + radial(tau + cross * wide) * (r + flute)
                )
                hue = pigment * (0.67 + 0.28 * t + 0.12 * (1 - edge)) + (
                    np.array([41, 40, 25]) * (t**2 * edge)
                    + np.array([25, 20, 10]) * max(0, 1 - abs(cross) * 2) * t
                )
                cc.append(hue)
            profile.append(rr)
            cols.append(cc)
        grid(g, profile, cols)
    # Short ochre staminal crown emerging inside the open fruit, not oversized
    # fairy lights. Tapered flutes leave open air between the four shells.
    for j in range(1 if g.middle else 3):
        d = radial(aa + 2.094 * j + 0.3)
        p = origin + axis * size * 0.43 + d * size * 0.06
        m = origin + axis * size * (0.94 - 0.1 * age) + d * size * (0.16 + 0.09 * age)
        tv = unit(m - p)
        tip = m + tv * size * 0.12
        tube(
            g,
            [p, m, tip],
            [size * 0.025, size * 0.082, size * 0.024],
            np.array([196, 171, 92.0]),
            5,
        )


def specimen(i, middle=False):
    rng = np.random.default_rng(221318 + i * 97)
    g = Piece()
    g.middle = middle
    base = np.array([0.0, -0.012, 0.0])
    # Two ages of deeply ribbed strap leaves rather than a single radial skirt.
    for j in range(15):
        angle = j * 2.39996 + i + rng.normal(0, 0.13)
        D = pol(angle)
        lateral = pol(angle + math.pi / 2)
        young = j >= 10
        reach = rng.uniform(0.17, 0.33) * (0.72 if young else 1)
        height = rng.uniform(0.105, 0.20) * (1.04 if young else 0.85)
        width = rng.uniform(0.018, 0.031)
        bend = rng.uniform(-0.065, 0.065)
        curl = rng.uniform(0.01, 0.05)
        color = np.array([128, 146, 107] if young else [139, 153, 125]) * rng.uniform(
            0.9, 1.08
        )
        profile = []
        cols = []
        for t in (0, 0.16, 0.38, 0.62, 0.83, 0.94, 1.0):
            curve = 1.26 * t - 0.61 * t * t
            center = (
                base
                + D * (reach * (0.20 * t + 0.80 * t * t))
                + lateral * (bend * t * t)
                + Y * (height * curve + curl * math.sin(math.pi * t))
            )
            w = width * (0.015 + math.sin(math.pi * (0.06 + 0.94 * t)) ** 0.79)
            rr = []
            cc = []
            for cross in (-1.0, -0.48, 0, 0.48, 1.0):
                rr.append(
                    center
                    + lateral * cross * w
                    + Y * w * ((1 - abs(cross)) * 0.43 + abs(cross) ** 2 * 0.13)
                    + D * (w * 0.25 * cross * math.sin(t * math.pi))
                )
                hue = (
                    color * (0.62 + 0.43 * t + 0.07 * abs(cross))
                    + (np.array([17, 21, 6]) * (1 - abs(cross)) ** 4)
                    + np.array([22, 10, 0]) * max(0, t - 0.65) * 2
                )
                cc.append(hue)
            profile.append(rr)
            cols.append(cc)
        grid(g, profile, cols)
    for j in range(3 if i == 2 else 4):
        D = pol(i * 0.72 + j * 2.4 + 0.18)
        L = pol(i * 0.72 + j * 2.4 + 1.75)
        height = rng.uniform(0.22, 0.40)
        spread = rng.uniform(0.085, 0.17)
        pts = [
            base,
            base + D * spread * 0.19 + Y * height * 0.29,
            base + D * spread * 0.48 + L * 0.01 + Y * height * 0.63,
            base + D * spread * 0.81 + Y * height * 0.90,
            base + D * spread + Y * height,
        ]
        tube(
            g,
            pts,
            [0.0068, 0.0058, 0.0044, 0.0035, 0.0030],
            np.array([132.0, 129.0, 87.0]),
            5,
        )
        # Different stages on one colony. Older valves are faded and tipped;
        # the youngest are deep russet, narrow and still nearly conjoined.
        age = ((i + j * 2) % 5) / 4
        ink = np.array([160.0, 105.0, 100.0]) + np.array([35.0, 39.0, 20.0]) * age
        closed_cup(
            g,
            pts[-1],
            D * (0.2 + 0.67 * age) + L * 0.12,
            rng.uniform(0.038, 0.051),
            age,
            ink,
            i + 0.45 * j,
        )
        if j % 2 == 0:
            p = pts[2]
            q = p + L * spread * 0.32 + Y * height * 0.2
            tube(
                g,
                [p, (q + p) * 0.5, q],
                [0.003, 0.0027, 0.002],
                np.array([135.0, 130.0, 89.0]),
                5,
            )
            closed_cup(g, q, L * 0.53, 0.028, 0.19, ink * 0.92, 1.7 + i)
    return g


if __name__ == "__main__":
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    # Local models have offset pivots in glTF; each scene node retains that
    # translation and must be used as a whole at registered root transforms.
    objs = {f"FoldedPennon{i}": specimen(i) for i in (1, 2, 3)}
    s = export(
        objs,
        out / "pennon-windcups.glb",
        "Oxidised rose valves, ivory stamens and waxed strap leaves",
    )
    lod = {f"FoldedPennon{i}": specimen(i, True) for i in (1, 2, 3)}
    mids = export(
        lod,
        out / "pennon-windcups-middle.glb",
        "The same curled valve outlines, simplified ribs",
    )
    (out / "specimens.json").write_text(
        json.dumps(
            dict(
                **s,
                variants={k: len(g.t) for k, g in objs.items()},
                middle=dict(**mids, variants={k: len(g.t) for k, g in lod.items()}),
                scale="metres",
                not_registered=True,
            ),
            indent=2,
        )
        + "\n"
    )
