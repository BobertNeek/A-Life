"""Find a limited bank of actual planted specimens, then fit at the same roots."""

from collections import defaultdict
from dataclasses import dataclass
import math
import numpy as np
from scipy.spatial import ConvexHull
from scipy.spatial.distance import cdist
from forms import mesh

COUNT = {
    "hazel": 2,
    "myrtle": 2,
    "salt": 2,
    "reed": 2,
    "oat": 2,
    "shore": 1,
    "samphire": 1,
}
ANGLE = np.arange(16) * math.tau / 16
AXIS = np.stack((np.cos(ANGLE), np.sin(ANGLE)))


@dataclass
class Piece:
    family: str
    id: int
    zone: int
    anchor: np.ndarray
    xyz: np.ndarray
    rgb: np.ndarray
    tri: np.ndarray
    outline: np.ndarray
    signature: np.ndarray


class Collector:
    def __init__(self, reach):
        self.reach = reach
        self.rows = defaultdict(list)

    def take(self, kind, xz, g, start, zone):
        a, b = start
        h = float(self.reach.floor(*xz)[0])
        anchor = np.array([xz[0], h, xz[1]])
        p = np.array(g.xyz[a:]) - anchor
        c = np.array(g.col[a:])
        t = np.array(g.tri[b:]) - a
        # The original scatter uses individual floor samples for every plant.
        # We read the site before deleting temporary author-only data, not
        # infer 1,578 poses from tile centres or generate a fresh random map.
        q = p[:, [0, 2]]
        hull = ConvexHull(np.vstack([q, [0, 0]]))
        q = np.vstack([q, [0, 0]])[hull.vertices]
        extent = (q @ AXIS).max(0)
        r = np.maximum(0.01, extent.mean())
        stats = np.r_[
            extent / r,
            0.65 * p[:, 1].max() / r,
            0.12 * ((c.mean(0) - 140) / 30),
            0.13 * len(t) / max(1, 500 if kind in ("hazel", "myrtle", "salt") else 80),
        ]
        i = len(self.rows[kind])
        self.rows[kind].append(Piece(kind, i, zone, anchor, p, c, t, q, stats))
        del g.xyz[a:]
        del g.col[a:]
        del g.tri[b:]


def representatives(rows, count):
    A = np.stack([row.signature for row in rows])
    d = cdist(A, A, "sqeuclidean")
    centres = [int(np.argmin(d.sum(0)))]
    while len(centres) < count:
        # A medoid minimizing total error, rather than an exotic farthest outlier.
        nearest = d[:, centres].min(1)
        centres.append(int(np.argmin(np.minimum(d, nearest[:, None]).sum(0))))
    for _ in range(4):
        label = np.argmin(d[:, centres], axis=1)
        centres = [
            int(ix[np.argmin(d[np.ix_(ix, ix)].sum(0))])
            for j in range(count)
            for ix in [np.where(label == j)[0]]
        ]
    return [rows[i] for i in centres]


def fertile(o):
    # The source oat's four culm segments and paired hanging spikelets add
    # 32 triangles beyond its nine-triangle blades. Keep its actual habit.
    return o.family == "oat" and len(o.tri) % 9 != 0


def fitting_library(choices):
    bank = []
    for key, o, levels in choices:
        for deg in (-75, -52, -32, -15, 0, 15, 32, 52, 75):
            ang = math.radians(deg)
            c, s = math.cos(ang), math.sin(ang)
            Q = np.array([[c, -s], [s, c]])
            for stretch in (0.54, 0.69, 0.84, 1.0, 1.19, 1.45, 1.85):
                sk = np.array([math.sqrt(stretch), 1 / math.sqrt(stretch)])
                support = np.maximum(0.00001, ((o.outline * sk) @ Q @ AXIS).max(0))
                base_penalty = 0.001 * (deg / 45) ** 2 + 0.0012 * math.log(stretch) ** 2
                bank.append((key, o, levels, ang, Q, sk, support, base_penalty))
    return bank


def fit(target, bank, reach):
    """Match actual measured clearance in sixteen horizontal directions.

    Small anisotropic changes retain the shape of a wind-combed site. Do not
    average a sterile and hanging-seeded oat, move a root, or raise a slope.
    """
    bank = [b for b in bank if fertile(target) == fertile(b[1])]
    limits = np.maximum(0.00001, (target.outline @ AXIS).max(0))
    supports = np.stack([b[6] for b in bank])
    sf = np.min(limits[None, :] / supports, axis=1)
    ratios = supports * sf[:, None] / limits[None, :]
    base_colors = {b[0]: b[1].rgb.mean(0) for b in bank[::63]}
    # Grouped 9 by 7 range above; no visual effect depends on iteration order.
    for b in bank:
        if b[0] not in base_colors:
            base_colors[b[0]] = b[1].rgb.mean(0)
    col = target.rgb.mean(0)
    scores = (
        np.mean((1 - ratios) ** 2, axis=1)
        + 0.035 * (1 - ratios.min(axis=1)) ** 2
        + np.array(
            [b[7] + 0.004 * np.sum(((col - base_colors[b[0]]) / 30) ** 2) for b in bank]
        )
    )
    j = int(np.argmin(scores))
    name, o, levels, ang, Q, sk, _, _ = bank[j]
    sx, sz = sk * sf[j]
    sy = max(0.015, target.xyz[:, 1].max()) / max(0.015, o.xyz[:, 1].max())
    p = levels[0][0][levels[0][3]]
    xz = (p[:, [0, 2]] * [sx, sz]) @ Q + target.anchor[[0, 2]]
    actual = reach.floor(xz[:, 0], xz[:, 1])[0]
    offset = min(0.0, float(np.min(actual - (target.anchor[1] + p[:, 1] * sy) - 0.003)))
    return dict(
        key=name,
        id=target.id,
        zone=target.zone,
        root=target.anchor.tolist(),
        offset=offset,
        yaw=ang,
        scale=[sx, sy, sz],
        support_fill=float(ratios[j].mean()),
    )
