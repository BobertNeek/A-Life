"""Local parallel sunlight from the *actual* GLB triangles at the desk station.
No painted contact rings. Outside the local study, leave the earlier matte draw
unchanged. The image remains an offline software illustration, not native PBR.
"""

import math
import numpy as np


class LocalSun:
    def __init__(self, origin, sun):
        self.sun = sun
        self.right = np.cross([0, 1, 0], sun)
        self.right /= np.linalg.norm(self.right)
        self.up = np.cross(sun, self.right)
        self.axes = np.column_stack((self.right, self.up, sun)).astype("f")
        self.origin = np.array(origin, "f")
        self.res = 640
        self.width = 13.0
        self.depth = np.full((self.res, self.res), -np.inf, "f")

    def project(self, points):
        q = (points - self.origin) @ self.axes
        q[:, :2] = q[:, :2] * (self.res / self.width) + self.res * 0.5
        return q

    def cast(self, points, tri, uv=None, cut=None):
        q = self.project(points)
        if np.any(q[:, :2].max(0) < 0) or np.any(q[:, :2].min(0) >= self.res):
            return
        p = q[tri]
        look = (p[:, :, :2].max(1) >= 0).all(1) & (p[:, :, :2].min(1) < self.res).all(1)
        for k in tri[look]:
            vv = q[k]
            xy = vv[:, :2]
            a, b, c = xy
            lo = np.maximum(0, np.floor(xy.min(0))).astype(int)
            hi = np.minimum(self.res - 1, np.ceil(xy.max(0))).astype(int)
            x0, y0 = lo
            x1, y1 = hi
            u = b - a
            v = c - a
            det = u[0] * v[1] - u[1] * v[0]
            if abs(det) < 0.25:
                continue
            x = np.arange(x0, x1 + 1, dtype="f")[None, :] + 0.5 - a[0]
            y = np.arange(y0, y1 + 1, dtype="f")[:, None] + 0.5 - a[1]
            w1 = (x * v[1] - y * v[0]) / det
            w2 = (u[0] * y - u[1] * x) / det
            w0 = 1 - w1 - w2
            inside = (w0 >= 0) & (w1 >= 0) & (w2 >= 0)
            if not inside.any():
                continue
            yi, xi = np.nonzero(inside)
            w = np.stack((w0[yi, xi], w1[yi, xi], w2[yi, xi]), axis=1)
            if cut is not None:
                pic, threshold = cut
                uu = w @ uv[k]
                h, l = pic.shape[:2]
                shows = (
                    pic[
                        ((uu[:, 1] % 1) * h).astype(int),
                        ((uu[:, 0] % 1) * l).astype(int),
                        3,
                    ]
                    >= threshold * 255
                )
                yi, xi, w = yi[shows], xi[shows], w[shows]
            yi += y0
            xi += x0
            self.depth[yi, xi] = np.maximum(self.depth[yi, xi], w @ vv[:, 2])

    def transmission(self, world):
        p = self.project(world)
        ix = np.floor(p[:, 0]).astype(int)
        iy = np.floor(p[:, 1]).astype(int)
        margin = np.minimum.reduce((ix, iy, self.res - 1 - ix, self.res - 1 - iy))
        active = margin > 2
        vis = np.ones(len(ix), "f")
        if active.any():
            x, y, z = ix[active], iy[active], p[active, 2]
            transmission = np.zeros(len(x), "f")
            # A few source-space samples soften subpixel stems. A slope-biased
            # 2.4 cm floor guards against self-shadow acne in the 2 cm map.
            for dx, dy, w in (
                (0, 0, 0.4),
                (-1, 0, 0.15),
                (1, 0, 0.15),
                (0, -1, 0.15),
                (0, 1, 0.15),
            ):
                transmission += w * (self.depth[y + dy, x + dx] <= z + 0.024)
            w = np.clip((margin[active] - 2) / 10, 0, 1)
            vis[active] = 1 - w * (1 - transmission)
        return vis
