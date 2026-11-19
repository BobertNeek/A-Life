"""Meadow, feldspar neck, alder shelter and storm strand on real V1 triangles."""

import math
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter
from reach import BOUNDS


def ramp(a, b, x):
    p = np.clip((x - a) / (b - a), 0, 1)
    return p * p * (3 - 2 * p)


def locality(s, m, w):
    return np.exp(-0.5 * ((s - m) / w) ** 2)


def width(s):
    return (
        1.4
        + 0.42 * np.sin(0.047 * s + 0.13)
        + 0.20 * np.cos(0.137 * s)
        + 0.62 * locality(s, 104, 14)
        + 1.30 * locality(s, 213, 13)
    )


def paint(reach, out):
    x0, x1, z0, z1 = BOUNDS
    w, h = 844, 1636
    z, x = np.mgrid[:h, :w]
    x = x0 + (x + 0.5) * (x1 - x0) / w
    z = z0 + (z + 0.5) * (z1 - z0) / h
    s, d = reach.address(x, z)
    floor, gx, gz = reach.floor(x, z)
    rng = np.random.default_rng(30593482)

    def n(sc):
        f = gaussian_filter(rng.normal(size=x.shape).astype("f"), sc)
        return np.clip((f - f.mean()) / max(f.std(), 0.00001), -3, 3)

    broad = n(19)
    mid = n(5)
    fine = n(0.8)
    grain = n(0)
    # Worn humus is not a road. Sections close over; windswept pans and old
    # ice-deposit flutes split/coalesce asymmetrically across the open swale.
    end = ramp(-3, 9, s) * (1 - ramp(reach.m[-1] - 1, reach.m[-1] + 6, s))
    zone = (1 - ramp(9, 25, np.abs(d + 0.24 * broad))) * end * ramp(-0.02, 0.55, floor)
    spine = width(s)
    dev = d + mid * 0.18 + broad * 0.42
    tread = 1 - ramp(0.50 * spine, 1.65 * spine, np.abs(dev))
    fallback = (
        0.42
        + 0.31 * locality(s, 102, 20)
        + 0.23 * locality(s, 203, 18)
        + 0.24 * locality(s, 30, 12)
    )
    # Broad flanks widen where soil catches on the low, dry native saddle.
    west = 2.8 + 2.8 * np.sin(0.067 * s) + 1.3 * np.sin(0.114 * s + 3)
    seam = np.abs(d + west + 0.8 * broad + 0.31 * mid)
    pan = np.maximum(0.0, 1 - ramp(0.75, 4.8, seam)) * (
        0.94 * locality(s, 107, 23) + 0.46 * locality(s, 167, 13)
    )
    low = locality(s, 93, 27)
    heath = locality(s, 145, 27)
    shore = ramp(184, 216, s) * ramp(0.1, 4, 9.2 - floor)
    # Instead of encircling trunks in circles, wash collects downwind of
    # three original alders on the physical bluff. Offset each wake.
    shelter = np.zeros_like(x)
    for px, pz, span in [(130.4, 419.4, 6.9), (132.7, 424.1, 8.1), (133.0, 444.8, 8.0)]:
        shelter = np.maximum(
            shelter,
            np.exp(
                -0.5
                * (((x - px + 2.0) / span) ** 2 + ((z - pz - 1.1) / (span * 0.63)) ** 2)
            ),
        )
    base = np.empty((*x.shape, 3), "f")
    base[:] = [117, 136, 83]

    def glaze(t, ink):
        nonlocal base
        t = np.clip(t, 0, 1)[..., None]
        base = base * (1 - t) + np.array(ink, "f") * t

    glaze(0.62 * low, [151, 152, 99])
    glaze(0.75 * heath, [132, 146, 100])
    glaze(0.82 * shelter, [101, 107, 69])
    glaze(0.92 * pan, [161, 154, 123])
    glaze(0.83 * shore, [161, 157, 127])
    swept = (0.5 + 0.5 * np.sin(0.81 * d + 0.10 * s + 0.68 * broad + 0.16 * mid)) ** 4
    glaze((0.28 * low + 0.22 * shore) * swept, [179, 169, 133])
    glaze((0.30 + 0.42 * shore) * tread, [173, 149, 111])
    glaze(
        (0.17 + 0.53 * shore) * tread
        + 0.61 * shore * (1 - ramp(0.45, 2.15, np.abs(dev - 2.5 + 0.6 * broad))),
        [203, 183, 145],
    )
    glaze(
        0.35
        * (1 - ramp(0.1, 0.68, np.abs(np.sin(0.62 * s + 0.9 * dev) + 0.40 * mid)))
        * pan,
        [177, 172, 150],
    )
    # The colours under genuine shrubs must read as connected ground at
    # guardian scale, not a beige line with unrelated tiny twigs on top.
    from scrub import beds

    nurse = beds(s, d + broad * 0.65 + 0.25 * mid)
    off = 1 - 0.91 * tread
    glaze(off * nurse["hazel"] * 0.84, [89, 117, 77])
    glaze(off * nurse["myrtle"] * 0.90, [102, 128, 105])
    glaze(off * nurse["salt"] * 0.79, [136, 136, 97])
    # Six actual till windows surface beyond the clear lane, dappled by
    # seed growth rather than a continuous manufactured promenade.
    lens = np.zeros_like(x)
    for ss, dd, along, across in (
        (58, -3.7, 4.5, 1.5),
        (82, 3.8, 6, 2.1),
        (102, -3.3, 6, 2.3),
        (128, 4.5, 3.5, 1.2),
        (198, -3.1, 4.6, 1.9),
        (210, 4.9, 4.3, 2.5),
    ):
        lens = np.maximum(
            lens,
            np.exp(
                -0.5
                * (
                    ((s - ss) / along) ** 2
                    + ((d - dd + 0.7 * broad + 0.28 * mid) / across) ** 2
                )
            ),
        )
    glaze(lens * 0.65, [185, 173, 144])
    strata = 0.46 * swept * pan + 0.43 * shore
    base += (
        (
            2.3 * broad
            + 2.9 * mid
            + (2.2 + 2 * strata) * fine
            + (1.3 + 2 * strata) * grain
        )[..., None]
    ) * [1.1, 1, 0.71]
    a = zone * np.clip(
        0.32
        + 0.22 * low
        + 0.18 * heath
        + 0.29 * shore
        + 0.22 * shelter
        + 0.33 * pan
        + fallback * tread
        + 0.5 * np.maximum.reduce(list(nurse.values()))
        + 0.24 * lens
        + np.clip(0.058 * broad, -0.10, 0.14),
        0.09,
        0.96,
    )
    rgba = np.uint8(np.clip(np.dstack((base, a * 255)), 0, 255))
    Image.fromarray(rgba).save(out / "reach-wash.png")
    return {
        "bounds": BOUNDS,
        "wash_size": [w, h],
        "surface_method": "Pigment on existing original triangles. No second ground or displacement.",
    }
