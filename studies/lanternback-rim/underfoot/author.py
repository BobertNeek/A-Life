"""Windfalls and mineral fines at real Lanternback heath/rock contacts.

A limited, meter-registered artist patch. It neither samples nor writes a new
ground height; all geometry uses existing native diagonals.
"""

from pathlib import Path
from collections import defaultdict, Counter
import csv, json, math, sys
import numpy as np
from PIL import Image
from scipy.spatial import cKDTree

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE.parent), str(HERE.parents[2] / "scripts")]
from strata import Rim, HASH, nz
from fissures import Piece, export
from island_inlet_ground import ease

BOX = (234.0, 264.0, -165.0, -134.0)


def rows(folder, name):
    with (folder / (name + ".csv")).open() as handle:
        return list(csv.DictReader(handle))


def inside(x, z, margin=0):
    a, b, c, d = BOX
    return (x > a + margin) & (x < b - margin) & (z > c + margin) & (z < d - margin)


class Contacts:
    def __init__(self, r, held):
        self.r = r
        self.roots = [
            q
            for q in rows(held, "heath-roots")
            if inside(float(q["x"]), float(q["z"]), -0.5)
        ]
        self.beds = [
            q
            for q in rows(held, "outcrop-ledger")
            if inside(float(q["x"]), float(q["z"]), -4)
        ]
        self.rootxy = np.array([[float(q["x"]), float(q["z"])] for q in self.roots])
        self.woody = self.rootxy[
            [q["specimen"].startswith(("heather", "pennon")) for q in self.roots]
        ]
        self.tree = cKDTree(self.woody)
        centers = []
        for q in self.beds:
            x, z = float(q["x"]), float(q["z"])
            h, gx, gz = r.floor(x, z)
            d = np.array([-gx, -gz])
            d /= np.linalg.norm(d)
            scale = float(q["half_length_m"])
            # Sheltered downslope bed tails; not lines drawn along the contour.
            for lateral in (-0.45, 0, 0.52):
                center = (
                    np.array([x, z])
                    + d * (0.44 * scale + 0.28)
                    + np.array([-d[1], d[0]]) * scale * lateral
                )
                centers.append(center)
        self.gullyxy = np.asarray(centers)
        self.scree = cKDTree(self.gullyxy)

    def fields(self, x, z):
        x, z = np.broadcast_arrays(x, z)
        shape = x.shape
        _, gx, gz = self.r.floor(x, z)
        g = np.maximum(0.05, np.hypot(gx, gz))
        dx, dz = -gx / g, -gz / g
        # Shelter pockets retain dark olive duff. Drift tails taper downhill.
        pts = np.column_stack(((x - dx * 0.31).ravel(), (z - dz * 0.31).ravel()))
        d, _ = self.tree.query(pts)
        d = d.reshape(shape)
        freq = nz(x, z, 0.38, 72) - 0.5
        ec = nz(x, z, 0.14, 17) - 0.5
        duff = 1 - ease(0.11, 1.24, np.maximum(0, d + freq * 0.55 + ec * 0.23))
        pts = np.column_stack((x.ravel(), z.ravel()))
        s, _ = self.scree.query(pts)
        s = s.reshape(shape)
        fines = 1 - ease(0.14, 1.36, s + freq * 0.62)
        fade = ease(
            0, 2.0, np.minimum.reduce((x - BOX[0], BOX[1] - x, z - BOX[2], BOX[3] - z))
        )
        return fade * duff, fade * fines, dx, dz

    def paint(self, out):
        # 2.4 cm/texel at the camera, not a tiling hill-wide gravel blanket.
        w, h = 1200, 1240
        a, b, c, d = BOX
        rgba = np.empty((h, w, 4), "B")
        micro = np.empty((h, w), "B")
        for k in range(0, h, 32):
            x, z = np.meshgrid(
                a + (np.arange(w) + 0.5) * (b - a) / w,
                c + (np.arange(k, min(k + 32, h)) + 0.5) * (d - c) / h,
            )
            hum, wet, gx, gz = self.fields(x, z)
            n1 = nz(x, z, 0.070, 10)
            n2 = nz(x, z, 0.028, 8)
            n3 = nz(x, z, 0.23, 61)
            # Fine mineral grains accumulate inside the lighter part; exposed
            # humus reads as warm umber, not a uniformly black contact shadow.
            soil = (
                np.array([108, 91, 64])
                + ((n1 - 0.5) * 20 + (n2 - 0.5) * 13 + (n3 - 0.5) * 19)[..., None]
            )
            silt = (
                np.array([161, 148, 119])
                + ((n1 - 0.5) * 29 + (n2 - 0.5) * 26 + (n3 - 0.5) * 21)[..., None]
            )
            wgt = wet / (0.0001 + hum + wet)
            alpha = np.maximum(hum * 0.78, wet * 0.64) * (0.57 + 0.43 * n3)
            color = soil * (1 - wgt[..., None]) + silt * wgt[..., None]
            rgba[k : k + len(x)] = np.uint8(
                np.clip(np.dstack((color, alpha * 255)), 0, 255)
            )
            micro[k : k + len(x)] = np.uint8(
                np.clip(
                    122 + alpha * (60 * (n1 - 0.5) + 99 * (n2 - 0.5) + 53 * (n3 - 0.5)),
                    0,
                    255,
                )
            )
        Image.fromarray(rgba).save(out / "contact-soil.png", optimize=True)
        Image.fromarray(micro).save(out / "soil-tooth.png", optimize=True)


def basis(x, z, angle):
    v = np.array([math.cos(angle), math.sin(angle)])
    return np.array([x, z]), v, np.array([-v[1], v[0]])


class Objects:
    def __init__(self, site):
        self.site = site
        self.rng = np.random.default_rng(1321)
        self.meshes = defaultdict(Piece)
        self.mid = defaultdict(Piece)
        self.count = Counter()
        self.anchors = []

    def draw(self, k, x, z, angle, L, W, T, pick):
        a, v, u = basis(x, z, angle)
        rng = self.rng
        p = []
        c = []
        f = []
        base = np.array(pick, float)

        def point(s, t, y, lum):
            xz = a + s * v + t * u
            ground = float(self.site.r.floor(*xz)[0])
            p.append([xz[0], ground + y, xz[1]])
            c.append(base * lum)
            return len(p) - 1

        def tri(i, j, k):
            f.append((i, j, k))

        if k == "split-seedcase":
            # Two curled-open pennon valves still joined at the sunken hinge.
            # Actual artisted calyx is rose, older split valves dry to copper.
            for side in (-1, 1):
                ids = []
                for j, progress in enumerate((0, 0.27, 0.59, 0.83, 1)):
                    shape = math.sin(math.pi * progress) ** 0.65
                    y = -0.001 + T * (0.08 + (0.95 if j in (2, 3) else 0.3) * shape)
                    bend = side * W * (0.12 + 0.8 * progress**1.9)
                    ids.append(
                        [
                            point(
                                (progress - 0.5) * L,
                                bend + side * t * shape * W,
                                y + vfac * T,
                                l,
                            )
                            for t, vfac, l in [
                                (0, -0.07, 0.72),
                                (0.43, -0.16, 0.95),
                                (0.75, 0.24, 1.13),
                            ]
                        ]
                    )
                for aa, bb in zip(ids, ids[1:]):
                    for j in range(2):
                        tri(aa[j], aa[j + 1], bb[j])
                        tri(aa[j + 1], bb[j + 1], bb[j])
        elif k == "weathered-sheath":
            # Long fallen sheaths have a bent spine, lifted edges and rubbed tips.
            bulge = rng.uniform(-0.32, 0.32)
            rows = []
            for j, t in enumerate((0, 0.23, 0.57, 0.82, 1)):
                s = math.sin(math.pi * t)
                spine = bulge * W * (2 * t - 1) ** 2
                rows.append(
                    [
                        point(
                            (t - 0.5) * L,
                            spine + i * W * s,
                            -0.001
                            + T
                            * (
                                0.1
                                + (0.43 - j * 0.06) * s
                                + (abs(i) * 0.68 + (0.43 if j == 4 else 0))
                            ),
                            lum,
                        )
                        for i, lum in ((-1, 0.75), (0, 1.06), (0.73, 0.9))
                    ]
                )
            for aa, bb in zip(rows, rows[1:]):
                for j in range(2):
                    tri(aa[j], aa[j + 1], bb[j])
                    tri(aa[j + 1], bb[j + 1], bb[j])
        else:
            sides = int(rng.integers(5, 8))
            ring = []
            for j in range(sides):
                t = (j + rng.uniform(-0.14, 0.14)) * math.tau / sides
                ring.append(
                    (
                        math.cos(t) * L * 0.5 * rng.uniform(0.82, 1.14),
                        math.sin(t) * W * 0.5 * rng.uniform(0.76, 1.19),
                    )
                )
            lower = [point(i, j, -0.002, rng.uniform(0.62, 0.83)) for i, j in ring]
            upper = [
                point(
                    i * 0.71 - L * 0.052,
                    j * 0.71,
                    T * rng.uniform(0.46, 0.90),
                    rng.uniform(0.87, 1.09),
                )
                for i, j in ring
            ]
            ridge = point(-0.055 * L, 0.09 * W, T, 1.13)
            for j in range(sides):
                n = (j + 1) % sides
                tri(lower[j], lower[n], upper[j])
                tri(lower[n], upper[n], upper[j])
                tri(upper[j], upper[n], ridge)
        # Organise by small real geographic cells, not by botanical name
        # over the whole slope; unseen cell geometry can genuinely disappear.
        cell = f"{math.floor((x-BOX[0])/5):02d}_{math.floor((z-BOX[2])/5):02d}"
        self.meshes["Clod_Near_" + cell].face(np.asarray(p), np.array(c), f)
        if L >= (0.037 if k == "slate-grit" else 0.048):
            # At middle distance only a long edge and glancing facet survive.
            # Same anchor, same palette, same source; no random second scatter.
            pp = []
            cc = []
            for along, across, up, shade in (
                (-0.47, 0.0, -0.03, 0.75),
                (-0.17, -0.43, 0.10, 0.81),
                (0.5, -0.02, -0.02, 0.93),
                (0.08, 0.46, 0.09, 0.98),
                (-0.05, 0.04, 0.88, 1.07),
            ):
                q = a + along * L * v + across * W * u
                bed = float(self.site.r.floor(*q)[0])
                pp.append([q[0], bed + T * up, q[1]])
                cc.append(base * shade)
            self.mid["Clod_Mid_" + cell].face(
                np.asarray(pp),
                np.asarray(cc),
                [(0, 1, 4), (1, 2, 4), (2, 3, 4), (3, 0, 4)],
            )
        self.count[k] += 1
        self.anchors.append([k, x, float(self.site.r.floor(x, z)[0]), z, L, W, T])

    def author(self):
        rng = self.rng
        # Each plant sheds a finite number of items. Outside lee pockets the
        # mineral bed can remain completely exposed. A few windfalls travel.
        for parent in self.site.roots:
            kind = parent["specimen"]
            if not kind.startswith(("heather", "pennon")):
                continue
            px, pz = float(parent["x"]), float(parent["z"])
            _, sx, sz = self.site.r.floor(px, pz)
            mag = np.hypot(sx, sz)
            d = np.array([-sx, -sz]) / mag
            u = np.array([-d[1], d[0]])
            for _ in range(rng.integers(6, 15)):
                reach = rng.gamma(1.6, 0.32)
                s = rng.normal(0, 0.24 + 0.12 * reach)
                x, z = np.array([px, pz]) + d * reach + u * s
                if not inside(x, z, 0.22) or self.site.r.blocked(x, z, 0.13):
                    continue
                a = math.atan2(d[1], d[0]) + rng.normal(0.24, 1.02)
                size = rng.uniform(0.045, 0.145)
                if kind.startswith("pennon") and rng.random() < 0.34:
                    col = np.array([154, 100, 72]) * rng.uniform(0.78, 1.12)
                    self.draw(
                        "split-seedcase",
                        x,
                        z,
                        a,
                        size * 0.72,
                        size * 0.22,
                        size * 0.19,
                        col,
                    )
                else:
                    col = np.array([146, 116, 76]) * rng.uniform(0.72, 1.12)
                    self.draw(
                        "weathered-sheath",
                        x,
                        z,
                        a,
                        size,
                        size * rng.uniform(0.055, 0.14),
                        size * 0.065,
                        col,
                    )
        # Gravity-sorted fans: coarser near a real bed/woody cluster, tapered
        # into irregular intermittent grit. Never span or move a saved solid.
        sources = list(self.site.gullyxy)
        sources += list(self.site.woody[::4])
        for center in sources:
            _, sx, sz = self.site.r.floor(*center)
            d = np.array([-sx, -sz]) / np.hypot(sx, sz)
            u = np.array([-d[1], d[0]])
            for _ in range(rng.integers(25, 54)):
                run = rng.gamma(1.45, 0.43)
                off = rng.normal(0, 0.18 + 0.18 * min(run, 1.8))
                x, z = center + d * run + u * off
                if not inside(x, z, 0.16) or self.site.r.blocked(x, z, 0.13):
                    continue
                cl = np.array(
                    rng.choice(
                        [
                            [142, 141, 124],
                            [110, 113, 108],
                            [160, 142, 112],
                            [128, 117, 100],
                            [177, 165, 135],
                        ]
                    )
                ) * rng.uniform(0.86, 1.08)
                size = np.clip(
                    rng.gamma(1.8, 0.031) * (0.96 - 0.19 * min(run, 2)), 0.014, 0.138
                )
                angle = math.atan2(d[1], d[0]) + rng.normal(0, 0.68)
                self.draw(
                    "slate-grit",
                    x,
                    z,
                    angle,
                    size,
                    size * rng.uniform(0.37, 0.83),
                    size * rng.uniform(0.095, 0.25),
                    cl,
                )

    def output(self, out):
        g = export(
            self.meshes,
            out / "windfall-and-shale.glb",
            "Flaked shale, papery valves and bleached fallen sheaths",
        )
        mid = export(
            self.mid,
            out / "windfall-middle.glb",
            "Silhouettes of the same worn shale and valves",
        )
        boxes = {}
        for k, g0 in {**self.meshes, **self.mid}.items():
            pp = np.asarray(g0.p)
            boxes[k] = {
                "min": pp.min(axis=0).round(5).tolist(),
                "max": pp.max(axis=0).round(5).tolist(),
                "triangles": len(g0.t),
            }
        (out / "cells.json").write_text(
            json.dumps(boxes, sort_keys=True, indent=2) + "\n"
        )
        with (out / "contact-ledger.csv").open("w", newline="") as f:
            w = csv.writer(f)
            w.writerow(("kind", "x", "native_y", "z", "length_m", "width_m", "roof_m"))
            for k, *v in self.anchors:
                w.writerow([k, *[round(float(q), 5) for q in v]])
        return dict(**g, middle=mid, counts=self.count)


def main(native, held, out):
    out.mkdir(parents=True, exist_ok=True)
    r = Rim(native / "island-v1.bin")
    s = Contacts(r, held)
    s.paint(out)
    o = Objects(s)
    o.author()
    report = o.output(out)
    ground = json.loads((held / "site.json").read_text())
    report.update(
        name="Lanternback / Pennon Drift",
        bounds=BOX,
        terrain_sha256=HASH,
        parent_roots=len(s.roots),
        parent_woody_roots=len(s.woody),
        parent_beds=len(s.beds),
        stations=ground["stations"],
        heights_changed=False,
        native_registered=False,
    )
    (out / "contact.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main(*(Path(a) for a in sys.argv[1:4]))
