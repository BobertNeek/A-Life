"""Eastern dry slack. An unmodified V1 profile selects the colour and plants."""

import csv, hashlib, json, math, struct
from pathlib import Path
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter1d
from island_inlet_ground import Inlet, ease

SHA = "8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38"
BOUNDS = (290.0, 372.0, -111.0, 18.0)


def noise(x, z, m, s):
    x, z = x / m, z / m
    i, j = np.floor(x), np.floor(z)
    a, b = x - i, z - j
    a, b = a * a * (3 - 2 * a), b * b * (3 - 2 * b)

    def h(i, j):
        q = np.sin(i * 123.7 + j * 311.9 + s * 78.31) * 43751.54
        return q - np.floor(q)

    return (1 - b) * ((1 - a) * h(i, j) + a * h(i + 1, j)) + b * (
        (1 - a) * h(i, j + 1) + a * h(i + 1, j + 1)
    )


class Hollow(Inlet):
    def __init__(self, src):
        raw = Path(src).read_bytes()
        if hashlib.sha256(raw).hexdigest() != SHA:
            raise ValueError("Only the held V1 surface.")
        super().__init__(src)
        self.sections = np.arange(-112.0, 19.0, 0.5)
        probe = np.arange(305.0, 360.01, 0.125)
        x, z = np.meshgrid(probe, self.sections)
        hh = self.floor(x, z)[0]
        mi = hh.argmin(axis=1)
        self.axis = probe[mi]
        self.low = hh[np.arange(len(mi)), mi]
        # The lowest samples of the original triangulation stay available for
        # the receipt. Only the *botanical* response is gently interpolated.
        self.soft = gaussian_filter1d(self.axis, 6.0)
        w, d, _, _, _, n = struct.unpack_from("<IIfffI", raw, 8)
        all_boxes = np.frombuffer(raw, "<f4", 6 * n, 32 + 4 * w * d).reshape(n, 6)
        x0, x1, z0, z1 = BOUNDS
        self.boxes = all_boxes[
            (all_boxes[:, 0] < x1 + 2)
            & (all_boxes[:, 2] > x0 - 2)
            & (all_boxes[:, 1] < z1 + 2)
            & (all_boxes[:, 3] > z0 - 2)
        ]

    def blocks(self, x, z, pad):
        b = self.boxes
        return np.any(
            (x > b[:, 0] - pad)
            & (x < b[:, 2] + pad)
            & (z > b[:, 1] - pad)
            & (z < b[:, 3] + pad)
        )

    def masks(self, x, z):
        h, gx, gz = self.floor(x, z)
        w = x - np.interp(z, self.sections, self.soft)
        rise = h - np.interp(z, self.sections, self.low)
        n = noise(x, z, 11, 4) - 0.5
        n2 = noise(x, z, 3.7, 7) - 0.5
        dk = noise(x, z, 1.45, 13) - 0.5
        # Three joined but not concentric communities; none is an
        # isocontour stripe. A real dry eastern dune shelters the lowest sedge.
        rag = 4.8 * n + 2.0 * n2 + 0.75 * dk
        outline = np.sqrt(
            ((w + 3 + 0.12 * (z + 40) + rag) / 32) ** 2 + ((z + 47 + 3 * n2) / 64) ** 2
        )
        cover = (1 - ease(0.81, 1.05, outline)) * ease(-0.02, 0.25, h)
        ribbon = (
            w + 2.4 + 3.7 * np.sin(z * 0.069) + 2.15 * np.sin(z * 0.126 + 0.8) + rag
        )
        slit = np.exp(-((0.42 * ribbon) ** 2)) * (
            0.3 + 0.7 * ease(0.26, 0.65, noise(x, z, 14.8, 30))
        )
        # Sediment fans and airier intervals run across the slack, so it is
        # not lined with continuous equidistant carpets or a footpath.
        lilac = 0.0
        russet = 0.0
        chalk = 0.0

        def lens(z0, w0, along, across, tilt):
            return np.exp(
                -1.50
                * (
                    ((z - z0 + 2 * n2) / along) ** 2
                    + ((w - w0 - (z - z0) * tilt + rag) / across) ** 2
                )
            )

        for v in [
            (-94.0, -5, 12, 11, 0.34),
            (-67.0, 3, 17, 11, -0.23),
            (-45.0, -8, 9, 15, 0.32),
            (-21.0, 1, 13.6, 8, -0.36),
            (-6, -6, 9.8, 11, 0.45),
        ]:
            lilac += lens(*v)
        for v in [
            (-89.0, 5, 12, 14, -0.42),
            (-73.0, -12, 20, 10, 0.24),
            (-52.0, 7, 14, 7.6, 0.25),
            (-35.0, -7, 14, 12, -0.45),
            (-11.0, 5, 14, 9.8, 0.41),
            (2.0, -9, 11, 8.2, 0.22),
        ]:
            russet += lens(*v)
        for v in [
            (-88.0, -18, 10, 8, -0.25),
            (-56.0, -12, 9, 14, -0.42),
            (-37.0, 3, 10, 8, 0.41),
            (-20.0, -11, 14, 7.8, -0.23),
            (-2.0, 2, 8, 6, 0.38),
        ]:
            chalk += lens(*v)
        damp = (1 - ease(0.12, 1.8, rise + 0.32 * n2)) * (
            0.44 + 0.55 * noise(x, z, 15.7, 6)
        )
        return dict(
            cover=cover,
            w=w,
            rise=rise,
            lilac=np.clip(lilac, 0, 1),
            russet=np.clip(russet, 0, 1),
            chalk=np.clip(chalk, 0, 1),
            slit=slit,
            damp=damp,
            n=n,
            n2=n2,
            dk=dk,
        )

    def paint(self, out, rows):
        x0, x1, z0, z1 = BOUNDS
        W, H = 820, 1290
        from seedbed import trace_roots

        cool, copper = trace_roots(rows, BOUNDS, (W, H))
        im = np.zeros((H, W, 4), "B")
        for j in range(0, H, 40):
            x, z = np.meshgrid(
                x0 + (np.arange(W) + 0.5) * (x1 - x0) / W,
                z0 + (np.arange(j, min(j + 40, H)) + 0.5) * (z1 - z0) / H,
            )
            f = self.masks(x, z)

            def colour(c, v):
                return np.asarray(c) + v[..., None]

            def blend(c, a, t):
                return c + (a - c) * np.clip(t, 0, 1)[..., None]

            n, n2, dk = f["n"], f["n2"], f["dk"]
            small = noise(x * 0.95, z * 1.24, 0.38, 18) - 0.5
            sand = colour([178, 166, 120], 15 * n + 11 * n2 + 6 * small)
            turf = colour([139, 151, 96], 21 * n + 15 * n2 + 9 * dk + 7 * small)
            c = blend(turf, sand, np.clip(0.33 + f["rise"] * 0.19 + n * 0.22, 0, 0.77))
            rush = colour([107, 125, 94], 16 * n + 10 * n2 + 7 * small)
            c = blend(c, rush, np.clip(f["damp"] * 0.47 + f["lilac"] * 0.55, 0, 0.84))
            c = blend(
                c,
                colour([160, 133, 86], 13 * n2 + 13 * dk + 5 * small),
                f["russet"] * 0.78,
            )
            c = blend(
                c, colour([207, 188, 140], 12 * n2 + 7 * small), f["chalk"] * 0.83
            )
            c = blend(c, colour([109, 110, 85], 7 * small), f["slit"] * 0.57)
            grain = noise(x * 1.12, z * 0.73, 0.11, 13) - 0.5
            # Tangled dry sheaths and loam aggregates, not an RGB noise overlay.
            ribs = np.abs(
                np.sin(
                    (z + 0.48 * x) * 6.8
                    + 3.2 * dk
                    + 1.6 * n2
                    + 0.68 * np.sin(x * 3.92 + z * 0.42)
                )
            )
            litter = (1 - ease(0.05, 0.22, ribs)) * ease(
                0.35, 0.71, noise(x * 0.65, z * 1.8, 1.65, 29)
            )
            c += ((1.8 + 1.5 * f["damp"]) * grain - 6 * litter + 3.6 * small)[..., None]
            # Sedge peat, grey shell litter and honey ligules occur in linked
            # scatter beds. The original sand survives visibly between them.
            ck = cool[j : j + len(z)]
            cm = copper[j : j + len(z)]
            tuft = np.clip(ck + cm, 0, 1)
            mixd = np.clip(0.32 + 1.3 * n2 + 0.6 * dk + 0.36 * small, 0, 1)
            c = blend(
                c,
                colour([87, 113, 91], 14 * n2 + 15 * dk + 11 * small),
                ck * (0.38 + 0.48 * mixd),
            )
            c = blend(
                c,
                colour([145, 108, 65], 13 * n2 + 15 * dk + 11 * small),
                cm * (0.39 + 0.45 * mixd),
            )
            c = blend(
                c,
                colour([179, 166, 132], 13 * dk + 9 * small),
                tuft * (1 - mixd) * 0.27,
            )
            c += ((9 * small + 8 * dk - 12 * litter) * (0.32 + 0.68 * tuft))[..., None]
            # Don't replace the whole dune floor with one smooth oval. Fold
            # in the actual substrate between fingers of wind-laid material.
            alpha = f["cover"] * np.clip(
                0.15
                + 0.27 * f["damp"]
                + 0.31 * tuft
                + 0.17 * (f["chalk"] + f["russet"])
                + 0.33 * n
                + 0.25 * n2
                + 0.12 * dk,
                0.09,
                0.87,
            )
            im[j : j + len(z)] = np.uint8(np.clip(np.dstack((c, alpha * 255)), 0, 255))
        Image.fromarray(im).save(out / "hollow-ground.png", compress_level=7)

    def record(self, out):
        with (out / "sections.csv").open("w", newline="") as file:
            q = csv.writer(file)
            q.writerow(["z", "lowest_x", "lowest_y", "west_18m_y", "east_18m_y"])
            for k in range(0, len(self.sections), 4):
                x, z = self.axis[k], self.sections[k]
                q.writerow(
                    [
                        round(float(a), 5)
                        for a in [
                            z,
                            x,
                            self.low[k],
                            self.floor(x - 18, z)[0],
                            self.floor(x + 18, z)[0],
                        ]
                    ]
                )


def community(b, props):
    R = np.random.default_rng(70095)
    rows = []
    pp = json.loads(Path(props).read_text())
    landmarks = {str(i): pp[i] for i in (1284, 1311, 1400, 1454, 1648, 1253, 222)}
    # Broad established colonies use the actual low axis, not the camera.
    z = R.uniform(-108, 13, 2700)
    x = np.interp(z, b.sections, b.soft) + R.uniform(-37, 28, len(z))
    f = b.masks(x, z)
    for i, (px, pz) in enumerate(zip(x, z)):
        if R.random() > f["cover"][i] or f["slit"][i] > 0.88:
            continue
        b0 = f["lilac"][i]
        b1 = f["russet"][i]
        b2 = f["chalk"][i]
        ph = noise(px, pz, 5.3, 35)
        choices = np.array(
            [
                0.125 * b0**1.55 + 0.03 * f["damp"][i],
                0.16 * b1**1.60,
                0.055 * b2**1.6,
                0.037 * max(0, ph - 0.24) ** 1.3,
            ]
        )
        k = R.random()
        if k >= choices.sum():
            continue
        typ = int(np.searchsorted(np.cumsum(choices), k))
        kind = ("milkspindle", "copper-rush", "silver-mat", "windwillow")[typ]
        family = R.integers([5, 7, 9, 2][typ], [15, 23, 27, 6][typ])
        span = R.uniform(0.7, 2.25)
        for _ in range(family):
            xx, zz = np.array([px, pz]) + R.normal(0, [1.1, 1.5]) * span
            yy, gx, gz = b.floor(xx, zz)
            size = R.uniform(0.54, 1.16) * (1.0 if typ != 3 else 0.98)
            # All sculpture is non-rigid plant tissue. Leave existing native
            # solid footprints and a full worst-case crown radius unplanted.
            if (
                math.hypot(gx, gz) > 0.48
                or yy <= 0.35
                or b.blocks(xx, zz, size * [0.49, 0.57, 0.61, 1.11][typ] + 0.06)
            ):
                continue
            if b.masks(xx, zz)["cover"] < 0.11:
                continue
            v = int(R.integers(3))
            ang = R.uniform(-math.pi, math.pi)
            rows.append(
                [
                    kind + "-" + str(v + 1),
                    round(xx, 5),
                    round(float(yy), 6),
                    round(zz, 5),
                    round(size, 5),
                    round(ang, 5),
                ]
            )
    return rows, landmarks
