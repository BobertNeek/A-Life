"""Sunspindle's true upper slope: native bed, chosen outcrops and weathering response."""

import csv, hashlib, json, math, struct
from pathlib import Path
import numpy as np
from PIL import Image
from scipy.spatial import cKDTree
from island_inlet_ground import Inlet, ease

HOLD = "41fb18b5045b0fbcee58e3192d3554539c91c7d2"
HASH = "8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38"
BOX = (193.0, 314.0, -182.0, -62.0)
# Adjacent, unequally truncated exposures. Base widths are half-widths in m.
# One opens downhill into the dry hollow; a second strikes across its head.
VEINS = (
    [
        (197, -164, 4),
        (206, -158, 12),
        (220, -149, 15),
        (233, -146, 9),
        (242, -138, 4),
        (248, -136, 0.7),
    ],
    [
        (216, -175, 1),
        (225, -164, 7),
        (236, -161, 11),
        (249, -153, 7),
        (260, -148, 7.6),
        (271, -143, 3.2),
        (277, -143, 0.6),
    ],
    [
        (223, -129, 0.7),
        (234, -128, 6.6),
        (247, -125, 12),
        (256, -122, 10),
        (265, -116, 7),
        (276, -105, 3.1),
        (283, -101, 0.8),
    ],
    [(247, -126, 4), (255, -131, 4.5), (264, -135, 3.1), (274, -132, 0.6)],
    [
        (199, -98, 1.2),
        (211, -104, 5),
        (224, -101, 9),
        (233, -94, 10),
        (246, -91, 5),
        (259, -82, 1.2),
    ],
    [(249, -103, 1.1), (258, -99, 4), (270, -94, 5), (275, -92, 0.8)],
    [(274, -155, 0.7), (281, -151, 3.5), (288, -152, 3), (293, -149, 0.7)],
    [(285, -125, 0.6), (291, -122, 3.8), (297, -119, 2), (302, -112, 0.4)],
)


def nz(x, z, s, seed):
    u = x / s
    v = z / s
    i = np.floor(u)
    j = np.floor(v)
    a = u - i
    b = v - j
    a = a * a * (3 - 2 * a)
    b = b * b * (3 - 2 * b)

    def R(i, j):
        w = np.sin(i * 127.1 + j * 311.7 + seed * 19.37) * 43758.543
        return w - np.floor(w)

    return ((1 - a) * R(i, j) + a * R(i + 1, j)) * (1 - b) + (
        (1 - a) * R(i, j + 1) + a * R(i + 1, j + 1)
    ) * b


def tracks(x, z):
    broad = nz(x, z, 12.3, 19) - 0.5
    mid = nz(x, z, 4.7, 26) - 0.5
    xx = x + 4.2 * broad + 1.36 * mid
    zz = z + 3.8 * (nz(x, z, 11.1, 13) - 0.5) + 1.4 * (nz(x, z, 2.3, 8) - 0.5)
    best = np.full_like(xx, 17.0, dtype=float)
    for line in VEINS:
        for (x0, z0, w0), (x1, z1, w1) in zip(line, line[1:]):
            dx, dz = x1 - x0, z1 - z0
            t = np.clip(((xx - x0) * dx + (zz - z0) * dz) / (dx * dx + dz * dz), 0, 1)
            d = np.hypot(xx - x0 - t * dx, zz - z0 - t * dz) / (
                w0 + (w1 - w0) * t + 0.12
            )
            best = np.minimum(best, d)
    return best, broad, mid


class Rim(Inlet):
    def __init__(self, src):
        src = Path(src)
        raw = src.read_bytes()
        if hashlib.sha256(raw).hexdigest() != HASH:
            raise ValueError("Landform must be exact frozen V1.")
        super().__init__(src)
        w, d, _, _, _, n = struct.unpack_from("<IIfffI", raw, 8)
        bb = np.frombuffer(raw, "<f4", n * 6, 32 + 4 * w * d).reshape(n, 6)
        self.boxes = bb[
            (bb[:, 0] < BOX[1] + 2)
            & (bb[:, 2] > BOX[0] - 2)
            & (bb[:, 1] < BOX[3] + 2)
            & (bb[:, 3] > BOX[2] - 2)
        ]

    def blocked(self, x, z, pad):
        b = self.boxes
        return np.any(
            (b[:, 0] - pad < x)
            & (x < b[:, 2] + pad)
            & (b[:, 1] - pad < z)
            & (z < b[:, 3] + pad)
        )

    def field(self, x, z):
        xx = np.asarray(x, float)
        zz = np.asarray(z, float)
        h, gx, gz = self.floor(xx, zz)
        sl = np.hypot(gx, gz)
        d, br, mi = tracks(xx, zz)
        f = nz(xx, zz, 1.32, 15) - 0.5
        dx = np.maximum(0, np.minimum(xx - BOX[0], BOX[1] - xx))
        dz = np.maximum(0, np.minimum(zz - BOX[2], BOX[3] - zz))
        edge = ease(0, 7, dx) * ease(0, 7, dz)
        fabric = (
            (1 - ease(0.70, 1.13, d + 0.17 * mi + 0.12 * f))
            * ease(0.23, 0.53, sl)
            * edge
        )
        skirt = (1 - ease(1.05, 1.79, d + 0.21 * br + 0.22 * mi)) * edge
        return h, sl, d, br, mi, f, fabric, skirt, edge

    def chart(self, out):
        x0, x1, z0, z1 = BOX
        w, h = 968, 960
        chart = np.zeros((h, w, 4), "B")
        weight = np.zeros((h, w), "B")
        xs = x0 + (np.arange(w) + 0.5) * (x1 - x0) / w
        for k in range(0, h, 32):
            xx, zz = np.meshgrid(
                xs, z0 + (np.arange(k, min(k + 32, h)) + 0.5) * (z1 - z0) / h
            )
            yy, s, d, br, mi, fi, rock, skirt, edge = self.field(xx, zz)
            sr = nz(xx, zz, 0.3, 31) - 0.5
            ambient = nz(xx, zz, 25, 10)
            weak = nz(xx, zz, 8.7, 2)
            # Gneiss and hornfels contacts. Mineral ribbons are short veins with
            # offsets and local extinction, not contours painted across the hill.
            along = 0.83 * xx + 0.55 * zz
            across = -0.55 * xx + 0.83 * zz
            drift = (
                across + 6.8 * br + 3.1 * mi + 0.79 * fi + 0.9 * np.sin(0.095 * along)
            )
            seam = np.abs(np.sin(drift * 0.39 + 0.14 * np.sin(along * 0.4)))
            quartz = (1 - ease(0.05, 0.25, seam)) * ease(
                0.39, 0.71, nz(along * 0.49, across, 4.5, 6)
            )
            flush = ease(0.3, 0.7, nz(along * 0.62, across, 8.3, 32) + 0.19 * mi)

            def shade(c, v):
                return np.asarray(c) + v[..., None]

            def mix(a, c, v):
                return a + (c - a) * np.clip(v, 0, 1)[..., None]

            schist = shade([124, 140, 137], 26 * br + 18 * mi + 14 * fi + 9 * sr)
            schist = mix(
                schist, shade([171, 159, 132], 13 * mi + 10 * fi), flush * 0.72
            )
            schist = mix(
                schist,
                shade([213, 201, 162], 10 * mi + 8 * fi),
                quartz * (0.43 + 0.25 * weak),
            )
            oxid = ease(0.50, 0.78, nz(xx * 0.75, zz * 1.25, 2.62, 19) + mi * 0.18)
            schist = mix(schist, shade([167, 125, 86], 11 * fi), oxid * 0.46)
            verge = shade([143, 148, 97], 21 * br + 14 * mi + 12 * fi + 8 * sr)
            verge = mix(verge, shade([112, 129, 94], 12 * mi + 12 * fi), weak * 0.80)
            verge = mix(
                verge,
                shade([165, 149, 108], 14 * mi),
                ease(0.3, 0.8, ambient + 0.27 * mi),
            )
            col = mix(verge, schist, rock)
            alpha = np.clip(
                (0.11 * edge + 0.46 * skirt + 0.41 * rock) * (0.96 + 0.10 * fi), 0, 0.94
            )
            chart[k : k + len(zz)] = np.uint8(
                np.clip(np.dstack((col, 255 * alpha)), 0, 255)
            )
            weight[k : k + len(zz)] = np.uint8(rock * 255)
        Image.fromarray(chart).save(out / "rim-ground.png", compress_level=6)
        Image.fromarray(weight).save(out / "bedding-weight.png", compress_level=6)


def frame(tag, ground):
    if tag == "upper-rim":
        return (tag, (375.0, 130.0, -55.0), (246.0, 42.5, -123.0), 143.0, 42, 1)
    if tag == "stone-eye":
        x, z, ax, az = 245.0, -140.6, 246.9, -150.4
        return (
            tag,
            (x, float(ground(x, z)) + 1.04, z),
            (ax, float(ground(ax, az)) + 0.33, az),
            0,
            25,
            0,
        )
    raise ValueError(tag)
