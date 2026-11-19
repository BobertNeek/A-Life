"""Shellback Reach: artist's coordinates on the ordinary V1; no pathfinding hooks.

East of the lower nursery approach, follow the native broad lip of the
estuary to its southern strand. The line is a composition guide, not
an agent route, map pin, new habitat or change to a saved height.
"""

import json, math, struct
from pathlib import Path
import numpy as np
from scipy.interpolate import PchipInterpolator
from scipy.spatial import cKDTree

# North to south. Bend inland to avoid the steep canal wall; then to the
# sea around the original bluff alders rather than crossing their trunks.
CORD = [
    (80.7, 266.4),
    (79.2, 274.5),
    (76.7, 279.1),
    (73.9, 286.3),
    (75.1, 297.5),
    (73.8, 305.2),
    (76.8, 315.7),
    (80.3, 322.7),
    (84.8, 327.2),
    (87.6, 335.4),
    (87.3, 343.2),
    (94, 351),
    (99.8, 359.5),
    (100.8, 366.3),
    (103.7, 372.6),
    (106.4, 382.4),
    (111.1, 390.4),
    (115.5, 397.1),
    (118.2, 407),
    (122.7, 414.2),
    (124.4, 425),
    (125.7, 432.1),
    (129.2, 438.2),
    (127.1, 444.9),
    (126.2, 451.2),
    (130.1, 457.2),
    (132.3, 463),
    (133.8, 466.2),
]
BOUNDS = (48.0, 161.0, 255.0, 474.0)
ANCHORS = {
    "meadow doorstep": 1414,
    "old wild sedge": 3406,
    "salt bush": 2907,
    "inner-bank shrub": 2049,
    "glacial stone": 1364,
    "scree tongue": 1753,
    "bluff alders": [1067, 213, 559],
    "outer shingle": 1833,
}


class Reach:
    def __init__(self, source):
        self.raw = Path(source).read_bytes()
        w, d, self.x0, self.z0, self.s, n = struct.unpack_from("<IIfffI", self.raw, 8)
        self.h = np.frombuffer(self.raw, "<f4", w * d, 32).reshape(d, w)
        self.boxes = np.frombuffer(self.raw, "<f4", n * 6, 32 + 4 * w * d).reshape(n, 6)
        k = np.array(CORD)
        z = np.linspace(k[0, 1], k[-1, 1], 321)
        x = PchipInterpolator(k[:, 1], k[:, 0])(z)
        self.p = np.column_stack((x, z))
        self.t = np.gradient(self.p, axis=0)
        self.t /= np.linalg.norm(self.t, axis=1)[:, None]
        self.n = np.column_stack((self.t[:, 1], -self.t[:, 0]))
        self.m = np.r_[0, np.cumsum(np.linalg.norm(np.diff(self.p, axis=0), axis=1))]
        self.tree = cKDTree(self.p)

    def floor(self, x, z):
        u = (np.asarray(x) - self.x0) / self.s
        v = (np.asarray(z) - self.z0) / self.s
        i = np.floor(u).astype(int)
        j = np.floor(v).astype(int)
        u = u - i
        v = v - j
        a, b, c, d = (
            self.h[j, i],
            self.h[j, i + 1],
            self.h[j + 1, i + 1],
            self.h[j + 1, i],
        )
        gx, gz = np.where(u >= v, b - a, c - d), np.where(u >= v, c - b, d - a)
        return a + gx * u + gz * v, gx / self.s, gz / self.s

    def address(self, x, z):
        x, z = np.broadcast_arrays(x, z)
        q = np.stack((x, z), axis=-1)
        _, i = self.tree.query(q)
        delta = q - self.p[i]
        return self.m[i] + (delta * self.t[i]).sum(axis=-1), (delta * self.n[i]).sum(
            axis=-1
        )

    def station(self, s, d):
        x, z = (
            np.interp(s, self.m, self.p[:, i]) + np.interp(s, self.m, self.n[:, i]) * d
            for i in range(2)
        )
        return x, z

    def profile(self, props):
        # At each grid side AND each diagonal u=v, the affine height gradient
        # may change. Record precisely the crossed native terrain triangles.
        # Distances to obstacles use each full segment, not spaced point tests.
        from hashlib import sha256

        hh = []
        grades = []
        mingap = np.inf
        nearest = None
        for a, b in zip(self.p, self.p[1:]):
            v = b - a
            t = [0.0, 1.0]
            for q in range(3):
                a0 = (
                    (a[q] - (self.x0, self.z0)[q]) / self.s
                    if q < 2
                    else (a[0] - a[1] - self.x0 + self.z0) / self.s
                )
                dv = v[q] / self.s if q < 2 else (v[0] - v[1]) / self.s
                if abs(dv) > 1e-12:
                    for k in range(
                        math.ceil(min(a0, a0 + dv)), math.floor(max(a0, a0 + dv)) + 1
                    ):
                        t.append(float(np.clip((k - a0) / dv, 0, 1)))
            t = np.unique(t)
            xx, zz = (a[:, None] + v[:, None] * t).tolist()
            hh.extend(self.floor(xx, zz)[0])
            mid = (t[1:] + t[:-1]) / 2
            xx, zz = a[:, None] + v[:, None] * mid
            h, gx, gz = self.floor(xx, zz)
            grades.extend(np.hypot(gx, gz))
            # Segment versus expanded AABBs: exact in L-infinity via interval
            # clipping. Gives a conservative body clearance in any direction.
            qq = self.boxes
            lo = np.min([a, b], axis=0)
            hi = np.max([a, b], axis=0)
            ids = np.flatnonzero(
                (qq[:, 0] < hi[0] + 14)
                & (qq[:, 2] > lo[0] - 14)
                & (qq[:, 1] < hi[1] + 14)
                & (qq[:, 3] > lo[1] - 14)
            )

            def overlaps(box, d):
                t0, t1 = 0, 1
                for i in (0, 1):
                    c, e = box[i] - d, box[i + 2] + d
                    if abs(v[i]) < 1e-13:
                        if not c <= a[i] <= e:
                            return False
                    else:
                        t2, t3 = sorted(((c - a[i]) / v[i], (e - a[i]) / v[i]))
                        t0 = max(t0, t2)
                        t1 = min(t1, t3)
                        if t1 < t0:
                            return False
                return True

            for idx in ids:
                box = qq[idx]
                lower, upper = 0.0, min(14.0, mingap)
                if not overlaps(box, upper):
                    continue
                for _ in range(17):
                    m = (lower + upper) / 2
                    if overlaps(box, m):
                        upper = m
                    else:
                        lower = m
                if upper < mingap:
                    mingap = upper
                    nearest = int(idx)
        ps = json.loads(Path(props).read_text())
        indices = [
            i
            for ids in ANCHORS.values()
            for i in (ids if isinstance(ids, list) else [ids])
        ]
        return dict(
            terrain_sha256=sha256(self.raw).hexdigest(),
            coordinate_system="game x,z horizontal, y up; metres",
            approach_m=round(self.m[-1], 2),
            height_min_max_m=list(np.round((min(hh), max(hh)), 3)),
            max_triangle_rise_run=round(float(max(grades)), 4),
            min_conservative_obstacle_clearance_m=round(float(mingap) - 0.0002, 3),
            nearest_obstacle_box=nearest,
            ordinary_body_limits={
                "radius_m": 0.2,
                "max_rise_run": 0.85,
                "water_y": 0.0,
                "max_wading_m": 0.6,
            },
            points_xz=CORD,
            native_anchors={str(i): ps[i] for i in indices},
            native_or_playtested=False,
        )
