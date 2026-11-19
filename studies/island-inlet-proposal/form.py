"""Original southern inlet sculpt, in V1 metre coordinates. DCC only.

This is a local, piecewise-affine deform of an existing mesh, never a native
height field or collision export. The same barycentric displacements keep
the local fragments, vegetation feet and the water's optical chart attached.
"""

import struct
from pathlib import Path
import numpy as np

CONTROLS = ("Thalweg bend", "Tapered point bar", "Outer scour pocket")
BOX = (23.0, 172.0, 350.0, 451.0)


def ease(a, b, x):
    q = np.clip((x - a) / (b - a), 0, 1)
    return q * q * (3 - 2 * q)


class Reach:
    def __init__(self, surface):
        b = Path(surface).read_bytes()
        w, d, self.x0, self.z0, self.s, _ = struct.unpack_from("<IIfffI", b, 8)
        self.h = np.frombuffer(b, "<f4", w * d, 32).reshape(d, w)
        j, i = np.indices((d, w))
        z = self.z0 + j * self.s
        x = self.x0 + i * self.s
        # Measured native thalweg at z390 ~85.75, z422 ~98, z448 ~106.5.
        q = x - (97.05 + 0.366 * (z - 419))
        boundary = ease(353, 377, z) * (1 - ease(439, 450, z))
        outer = 1 - ease(16, 49, np.abs(q))
        peak = np.exp(-(((z - 425.0) / 15.8) ** 2))
        upstream = np.exp(-(((z - 393.5) / 13.0) ** 2))
        self.parts = []
        self.parts.append(
            ((10.5 * peak - 6.7 * upstream) * outer * boundary, np.zeros_like(x))
        )
        apron = np.exp(-(((z - 427) / 14.0) ** 2))
        # Extend the inside (west) waterline, but open out the upper bank
        # behind it: a low, asymmetric bar, not simply a sideways canal.
        toe = np.exp(-(((q + 8.5) / 8.3) ** 2))
        bench = np.exp(-(((q + 16) / 8.4) ** 2))
        bar_x = (5.2 * toe - 2.6 * bench) * apron * boundary
        dry = np.maximum(0, self.h)
        bar_y = (
            -np.minimum(4.4, 0.66 * dry)
            * ease(0.35, 2.1, dry)
            * apron
            * bench
            * boundary
        )
        self.parts.append((bar_x, bar_y))
        # A shorter outside cut concentrates erosion just upstream of the
        # point. The reciprocal upstream reach erodes the opposite bank.
        long = np.exp(-(((z - 416) / 13.5) ** 2))
        spot = np.exp(-(((q - 11.8) / 8.7) ** 2))
        back = np.exp(-(((q + 12) / 9.1) ** 2))
        xx = (4.8 * long * spot - 3.7 * upstream * back) * boundary
        yy = (
            -np.minimum(1.25, 0.16 * dry)
            * ease(0.35, 2.1, dry)
            * (long * spot + upstream * back)
            * boundary
        )
        self.parts.append((xx, yy))
        self.total = tuple(sum(k[j] for k in self.parts) for j in range(2))

    def _sample(self, a, x, z):
        u = (x - self.x0) / self.s
        v = (z - self.z0) / self.s
        i = np.clip(np.floor(u).astype(int), 0, self.h.shape[1] - 2)
        j = np.clip(np.floor(v).astype(int), 0, self.h.shape[0] - 2)
        u = np.clip(u - i, 0, 1)
        v = np.clip(v - j, 0, 1)
        p, q, r, s = (a[j, i], a[j, i + 1], a[j + 1, i + 1], a[j + 1, i])
        return p + np.where(
            u >= v, (q - p) * u + (r - q) * v, (r - s) * u + (s - p) * v
        )

    def delta(self, x, z, part=None):
        field = self.total if part is None else self.parts[part]
        return tuple(self._sample(a, x, z) for a in field)

    def height(self, x, z):
        return self._sample(self.h, x, z)

    def unwarp_x(self, x, z):
        # z is unchanged. Find the native x on a monotonically ordered
        # section by bisection; the matching painted water cannot leave
        # a conspicuous straight stripe from the old channel.
        lo = x - 25.0
        hi = x + 25.0
        for _ in range(16):
            mid = (lo + hi) / 2
            beyond = mid + self.delta(mid, z)[0] > x
            hi = np.where(beyond, mid, hi)
            lo = np.where(beyond, lo, mid)
        return (hi + lo) / 2
