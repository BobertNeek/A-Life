"""Broken mountain granite, scree and rooted heath on the original V1 slope.
Slope morphology rather than elevation bands determines rock vs mantle.
The material is authored in native metres and does not displace terrain.
"""

from pathlib import Path
import sys, json, math
import numpy as np
from PIL import Image

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parents[1] / "scripts"), str(H.parent / "lanternback-rim")]
from island_inlet_ground import Inlet, ease
from strata import nz

BOX = (-39.0, 132.0, -436.0, -253.0)


def gauge(b, x, z, d):
    return (
        b.floor(x + d, z)[0]
        + b.floor(x - d, z)[0]
        + b.floor(x, z + d)[0]
        + b.floor(x, z - d)[0]
        - 4 * b.floor(x, z)[0]
    ) / d


def plates(u, v):
    # Irregular non-periodic joint spacings on an anisotropic structural
    # coordinate. Only some boundaries become open, never a tiled pattern.
    ix, iz = np.floor(u), np.floor(v)
    best = np.full_like(u, 1e6)
    second = best.copy()
    chosen = np.zeros_like(u)

    def r(a, b, seed):
        h = np.sin(a * 217.89 + b * 149.19 + seed * 17.71) * 48329.384
        return h - np.floor(h)

    for i in (-1, 0, 1):
        for j in (-1, 0, 1):
            a = ix + i
            c = iz + j
            dx = u - (a + 0.16 + 0.68 * r(a, c, 5))
            dz = v - (c + 0.13 + 0.74 * r(a, c, 17))
            dist = dx * dx + dz * dz
            nearer = dist < best
            second = np.minimum(second, np.where(nearer, best, dist))
            chosen = np.where(nearer, r(a, c, 30), chosen)
            best = np.minimum(best, dist)
    return np.sqrt(second) - np.sqrt(best), chosen


def field(b, x, z):
    y, dx, dz = b.floor(x, z)
    grade = np.hypot(dx, dz)
    # Same physical coordinates across the cliff-asset contact. x-z projection
    # is only a storage convenience; the structure uses original 3D elevation.
    w = nz(x, z, 23.8, 671) - 0.5
    q = nz(x, z, 7.6, 313) - 0.5
    t = nz(x, z, 2.37, 589) - 0.5
    f = nz(x, z, 0.64, 365) - 0.5
    c7 = gauge(b, x, z, 7.3)
    c22 = gauge(b, x, z, 18.5)
    convex = ease(-0.22, 0.09, -c7 * 0.57 - c22 * 0.43)
    # Two asymmetrical, contiguous geological families flank a true drainage.
    # The low original pine shelf remains intact (forest soil, not bare rock).
    axis = 0.89 * (x - 49.0) + 0.44 * (z + 371.0)
    along = -0.44 * (x - 49.0) + 0.89 * (z + 371.0)
    massif = 1 - ease(25, 57, abs(axis) + 0.16 * abs(along) + 5.5 * q + 8 * w)
    flank = 1 - ease(16, 39, abs(axis - 43) + 0.22 * abs(along + 11) - 5.0 * q)
    exposed = ease(0.41, 1.12, grade + 0.15 * w + 0.25 * q + 0.16 * t) * (
        0.48 + 0.52 * convex
    )
    rock = np.clip(np.maximum(massif, 0.76 * flank) * exposed, 0, 1)
    # Deliberate natural gaps contain patches of existing original terrain.
    fresh = ease(
        0.12, 0.65, 0.48 + 0.93 * q + 0.81 * t + 0.40 * w - 0.46 * (1 - convex)
    )
    edge = ease(0.0, 8.0, np.minimum(x - BOX[0], BOX[1] - x)) * ease(
        0.0, 8.0, np.minimum(z - BOX[2], BOX[3] - z)
    )
    rock *= edge
    stable = rock * (0.50 + 0.50 * fresh)
    u = (0.86 * x + 0.34 * z - 0.29 * y + 3.0 * w + 0.65 * q) / 6.2
    v = (-0.46 * x + 0.69 * z + 0.61 * y + 0.94 * q + 0.28 * t) / 2.48
    joint, cell = plates(u, v)
    openjoint = (1 - ease(0.024, 0.15, joint)) * (
        0.24 + 0.76 * ease(0.39, 0.72, 0.54 + 0.70 * t + 0.48 * q)
    )
    facets = (cell - 0.5) * 0.59 + 0.21 * t + 0.12 * f
    # One diagonal granite lens and one rusty fault. Their dipping normals
    # differ: these features cannot become regular horizontal stripes.
    lens_d = (
        (x - 62) * 0.52
        + (z + 378) * 0.43
        - (y - float(b.floor(62.0, -378.0)[0])) * 0.61
        + 4.9 * w
        + 1.3 * q
    )
    vein = (1 - ease(0.20, 1.6, abs(lens_d))) * ease(0.14, 0.65, 0.41 + q + 0.50 * t)
    fault_d = (
        (x - 51) * 0.68
        - (z + 354) * 0.49
        + (y - float(b.floor(51.0, -354.0)[0])) * 0.17
        + 3.5 * w
        + 0.9 * q
    )
    rust = (1 - ease(0.48, 3.8, abs(fault_d))) * ease(
        0.18, 0.63, 0.50 + 0.65 * q + 0.36 * t
    )
    pale = stable * (0.31 + 0.94 * fresh) + 0.69 * vein * rock
    crack = stable * openjoint
    hollow = ease(0.10, 0.76, c7 * 0.53 + c22 * 0.47) * rock
    # Stone is visibly different, not an imperceptible grey multiplier. Dark
    # fractures break the light slope into large-scale beds, not pin noise.
    gain = np.stack(
        (
            1
            + pale * (0.80 + facets)
            + 0.48 * rock * rust
            - 0.63 * crack
            - 0.13 * hollow,
            1
            + pale * (0.57 + 0.92 * facets)
            + 0.14 * rock * rust
            - 0.56 * crack
            - 0.09 * hollow,
            1
            + pale * (0.37 + 0.83 * facets)
            - 0.09 * rock * rust
            - 0.44 * crack
            - 0.04 * hollow,
        ),
        -1,
    )
    return np.clip(gain, 0.55, 2.8), rock, crack, stable


def make(native, out):
    b = Inlet(native / "island-v1.bin")
    a, c, d, e = BOX
    W, H = 650, 695
    data = np.empty((H, W, 3), "B")
    selected = 0
    for j in range(0, H, 16):
        end = min(H, j + 16)
        x, z = np.meshgrid(
            a + (np.arange(W) + 0.5) * (c - a) / W,
            d + (np.arange(j, end) + 0.5) * (e - d) / H,
        )
        g, r, *_ = field(b, x, z)
        data[j:end] = np.uint8(np.clip(np.rint(g * 90), 0, 255))
        selected += (r > 0.31).sum()
    Image.fromarray(data).save(out / "broken-granite.png", optimize=True)
    meta = dict(
        native_plan_xz=BOX,
        neutral=90,
        m_per_texel=[(c - a) / W, (e - d) / H],
        optical_slope_coverage_percent=round(100 * float(selected) / (W * H), 2),
        geometry_change_m=0,
        original_shelf_woodland_held=True,
    )
    (out / "broken-granite.json").write_text(json.dumps(meta, indent=2) + "\n")


if __name__ == "__main__":
    make(*(Path(q) for q in sys.argv[1:]))
