"""One original waterline rock at near-body scale: splinter joint, folded chip and rind.

The nine manually drawn interruptions are in the actual along-coast/height
coordinates that the 1.04 m camera sees, not a screen-space filter or a
repeating brick texture. A specimen broad enough for guardian photographs
cannot supply the centimetre scale of this one exposed rock panel.
"""

from pathlib import Path
import sys, json
import numpy as np
from PIL import Image

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parents[2] / "scripts"), str(H.parents[1] / "lanternback-rim")]
from island_inlet_ground import ease
from strata import nz

BOUNDS = (-107.35, -96.20, 0.16, 5.72)
# The two bold through-joints do NOT close a polygon. Lower branches step
# back and peter out into wet fine stone; weathering is asymmetric across them.
SEAMS = (
    (
        "shelf-rift",
        0.031,
        [
            (-97.23, 0.64),
            (-97.73, 0.94),
            (-98.07, 1.12),
            (-98.26, 1.38),
            (-98.63, 1.56),
            (-98.86, 1.92),
            (-99.38, 2.16),
            (-99.58, 2.45),
            (-100.17, 2.72),
            (-100.85, 3.27),
            (-101.42, 3.48),
        ],
    ),
    (
        "fold-back",
        0.020,
        [(-98.67, 1.6), (-98.72, 1.84), (-98.62, 2.04), (-98.69, 2.40), (-98.62, 2.62)],
    ),
    (
        "rind-shear",
        0.027,
        [
            (-97.9, 2.5),
            (-98.09, 2.62),
            (-98.41, 2.7),
            (-98.74, 2.94),
            (-99.21, 3.04),
            (-99.61, 3.29),
            (-99.89, 3.25),
        ],
    ),
    (
        "toe-fissure",
        0.038,
        [
            (-98.19, 0.47),
            (-98.58, 0.65),
            (-98.98, 0.70),
            (-99.32, 0.91),
            (-99.81, 0.99),
            (-100.05, 1.14),
            (-100.65, 1.26),
            (-100.93, 1.5),
            (-101.64, 1.69),
            (-102.27, 2.07),
            (-103.02, 2.27),
        ],
    ),
    (
        "split-root",
        0.018,
        [
            (-99.81, 0.99),
            (-99.85, 1.31),
            (-100.04, 1.53),
            (-100.07, 1.76),
            (-100.23, 1.88),
        ],
    ),
    (
        "sea-nick",
        0.023,
        [
            (-100.83, 0.36),
            (-101.14, 0.57),
            (-101.51, 0.6),
            (-101.7, 0.76),
            (-102.18, 0.7),
        ],
    ),
    (
        "shoulder",
        0.030,
        [
            (-101.83, 3.12),
            (-102.03, 3.42),
            (-102.42, 3.61),
            (-102.79, 4.09),
            (-103.38, 4.37),
            (-103.65, 4.70),
        ],
    ),
    (
        "far-fold",
        0.024,
        [
            (-102.66, 0.65),
            (-103.36, 1.12),
            (-103.82, 1.19),
            (-104.22, 1.50),
            (-104.91, 1.59),
            (-105.43, 1.89),
        ],
    ),
    (
        "break-return",
        0.020,
        [(-104.49, 2.75), (-104.35, 2.46), (-104.40, 2.17), (-104.22, 1.53)],
    ),
)


def generate(out):
    out.mkdir(exist_ok=True, parents=True)
    a, b, c, d = BOUNDS
    W, H = 920, 458
    u, y = np.meshgrid(
        a + (np.arange(W, dtype="f") + 0.5) * (b - a) / W,
        d - (np.arange(H, dtype="f") + 0.5) * (d - c) / H,
    )
    g = nz(u, y, 0.62, 301) - 0.5
    h = nz(u, y, 0.19, 302) - 0.5
    micro = nz(u, y, 0.045, 303) - 0.5
    speck = nz(u, y, 0.021, 304) - 0.5
    # At this sightline the unbroken toe steps in and out of spray. The
    # oxidised interior and rubbed dry face need not end at one horizontal y.
    mass = nz(u + 0.23 * g, y - 0.12 * h, 1.09, 305)
    wet = (1 - ease(0.45, 1.55, y + 0.40 * g + 0.22 * h)) * (0.65 + 0.34 * mass)
    ux = u + 0.052 * h + 0.021 * micro
    hy = y + 0.038 * h + 0.015 * micro
    groove = np.zeros_like(y)
    edge = groove.copy()
    flank = groove.copy()
    recess = groove.copy()
    weather = groove.copy()
    for s, (name, width, pts) in enumerate(SEAMS):
        Q = np.array(pts)
        closest = np.full_like(y, 99)
        side = np.zeros_like(y)
        along = np.zeros_like(y)
        cum = 0.0
        for p0, p1 in zip(Q, Q[1:]):
            delta = p1 - p0
            L = np.linalg.norm(delta)
            i = np.clip(
                ((ux - p0[0]) * delta[0] + (hy - p0[1]) * delta[1]) / (L * L), 0, 1
            )
            dx = ux - p0[0] - i * delta[0]
            dy = hy - p0[1] - i * delta[1]
            dd = np.hypot(dx, dy)
            better = dd < closest
            side = np.where(better, (delta[0] * dy - delta[1] * dx) / L, side)
            along = np.where(better, cum + i * L, along)
            closest = np.where(better, dd, closest)
            cum += L
        end = ease(0, 0.12, along) * (1 - ease(cum - 0.16, cum, along))
        width = (
            width * (0.75 + 0.52 * (nz(u * 0.6 + 0.25 * h, y, 0.21, 324 + s))) + 0.003
        )
        broken = 0.43 + 0.57 * ease(-0.21, 0.18, h + 0.38 * micro)
        channel = np.exp(-((closest / width) ** 2)) * end
        lip = (
            np.exp(-(((side + width * 1.58) / (width * 0.72 + 0.009)) ** 2))
            * np.exp(-((closest / (width * 4.1)) ** 2))
            * end
        )
        # For each interval only a few parts catch a hard pale cut, so this
        # does not read like light paint outlined around each large crack.
        chip = (
            lip
            * ease(-0.12, 0.20, g + 0.44 * h + 0.13 * np.sin(3 * along + s))
            * (0.65 + 0.35 * broken)
        )
        plane = np.exp(-((closest / (0.19 + 0.13 * mass)) ** 1.38)) * end
        sunken = plane * ease(-0.006, 0.061, side)
        polished = plane * (1 - ease(-0.054, 0.018, side))
        groove = np.maximum(groove, channel * (0.66 + 0.34 * broken))
        edge = np.maximum(edge, chip)
        recess = np.maximum(recess, sunken * broken)
        flank = np.maximum(flank, polished)
        weather = np.maximum(weather, sunken * ease(-0.12, 0.27, h + 0.52 * g))
    # Grain orientation follows the rock panel; sulphur freckles live only
    # inside the open, older joints, not as confetti across the entire tile.
    fabric = nz(u * 0.54 + 0.17 * g, (y + 0.25 * (u + 100)) * 1.56, 0.21, 342)
    rgb = np.array([130.0, 139.0, 136.0]) + mass[..., None] * np.array(
        [23.0, 19.0, 13.0]
    )
    rgb += (17 * g + 11 * h + 11 * micro + 8 * speck + (fabric - 0.5) * 8)[..., None]
    rgb += flank[..., None] * np.array([21.0, 17.0, 10.0])
    rgb -= recess[..., None] * np.array([18.0, 17.0, 14.0])
    rgb += weather[..., None] * np.array([15.0, 0.0, -10.0])
    rgb -= groove[..., None] * np.array([61.0, 59.0, 53.0])
    rgb += edge[..., None] * np.array([49.0, 47.0, 35.0])
    rgb *= 1 - wet[..., None] * 0.14
    # The held cliff colour shows through thin edges; tighter authored source
    # aligns very closely at the real toe without stamping the whole coast.
    margin = ease(0, 0.79, np.minimum(u - a, b - u)) * ease(
        0, 0.54, np.minimum(y - c, d - y)
    )
    window = ease(-0.05, 0.83, margin + 0.11 * g + 0.04 * h) * (0.83 + 0.12 * mass)
    response = np.clip(
        0.72 + 0.12 * flank - 0.18 * wet - 0.22 * weather + 0.052 * h, 0.26, 0.95
    )
    relief = np.clip(
        0.54
        + 0.055 * h
        + 0.037 * (fabric - 0.5)
        + 0.036 * micro
        - 0.37 * groove
        - 0.095 * recess
        + 0.115 * edge
        + 0.064 * flank,
        0.04,
        0.94,
    )
    Image.fromarray(np.uint8(np.clip(np.dstack((rgb, 255 * window)), 0, 255))).save(
        out / "shelf-colour.png", optimize=True
    )
    Image.fromarray(np.uint8(255 * response)).save(
        out / "shelf-roughness.png", optimize=True
    )
    Image.fromarray(np.uint8(255 * relief)).save(
        out / "shelf-height.png", optimize=True
    )
    (out / "toe.json").write_text(
        json.dumps(
            dict(
                side=BOUNDS,
                plan=[357.0, 433.0, -170.0, -59.0],
                elevation_pixel_metres=round((b - a) / W, 6),
                joint_networks=len(SEAMS),
                max_shading_depth_m=0.047,
                native_changed=False,
                geometric_offset_m=0,
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    generate(Path(sys.argv[1]))
