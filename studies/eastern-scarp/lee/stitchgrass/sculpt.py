"""Four folded windcombed grass/seed-sheath silhouettes in local glTF y-up metres.

A stretched underground tiller sends up two or three fans, not the ubiquitous
radial cone. Twisted leaf planes lift from an iron/olive sheath; a few carry
a tiny foxed awn. Diffuse is linear vertex colour on an ordinary opaque mesh.
"""

import math
import numpy as np


def shape(form, close):
    rng = np.random.default_rng(94211 + form)
    vertices = []
    colours = []
    faces = []

    def add(p, c, t):
        off = len(vertices)
        vertices.extend(p)
        colours.extend(c)
        faces.extend([[off + j for j in f] for f in t])

    def leaf(base, th, lift, stretch, wide, twist, dry):
        a = np.array([math.cos(th), 0.0, math.sin(th)])
        b = np.array([-math.sin(th), 0.0, math.cos(th)])
        ts = np.array(
            [0, 0.16, 0.42, 0.69, 0.87, 1.0] if close else [0, 0.40, 0.74, 1.0]
        )
        pp = []
        cc = []
        ff = []
        for j, t in enumerate(ts):
            r = (
                base
                + a * stretch * (0.10 * t + 0.90 * t * t)
                + b * (0.07 * stretch * math.sin(math.pi * t) * twist)
            )
            r[1] += lift * (math.sin(1.83 * t) ** 1.08 - 0.26 * t * t * t)
            w = wide * (0.16 * (1 - t) + math.sin(math.pi * (t**0.85)) ** 0.75)
            theta = twist * (0.19 + 0.94 * t)
            wing = b * (math.cos(theta) * w) + np.array([0, math.sin(theta) * w, 0])
            ridge = np.array([0, 0.26 * w, 0])
            cols = (
                np.array([0.054, 0.080, 0.023])
                if t < 0.18
                else np.array([0.122, 0.166, 0.054])
            )
            gold = (
                np.array([0.322, 0.204, 0.081])
                if dry
                else np.array([0.211, 0.232, 0.081])
            )
            blend = (max(0.0, (t - 0.37) / 0.63)) ** 1.45
            cols = cols * (1 - blend) + gold * blend
            if j < len(ts) - 1:
                pp.extend((r - wing, r + ridge, r + wing * 0.82))
                cc.extend((cols * 0.76, cols * 1.15, cols * 0.99))
                k = 3 * j
                if j < len(ts) - 2:
                    ff.extend(
                        (
                            (k, k + 3, k + 1),
                            (k + 1, k + 3, k + 4),
                            (k + 1, k + 4, k + 2),
                            (k + 2, k + 4, k + 5),
                        )
                    )
                else:
                    ff.extend(((k, k + 3, k + 1), (k + 1, k + 3, k + 2)))
            else:
                pp.append(r)
                cc.append(gold * 1.07)
        add(pp, cc, ff)

    # All fan bases are close enough to share the same tiny physical root.
    for j in range(0 if close == "guardian" else 2 + int(form == 2 and close)):
        base = np.array([0.028 * j - 0.02, -0.014, (0.032 if j % 2 else -0.024)])
        n = (6 + int(form % 2 == 0)) if close else 3
        for k in range(n):
            step = (k + 0.5) / n
            th = (
                math.pi
                + (0.81 if j % 2 else -0.80) * (step - 0.27)
                + rng.normal(0, 0.25)
            )
            length = (0.16 + 0.27 * step) * rng.uniform(0.83, 1.12)
            high = (0.14 + 0.13 * (1 - step)) * rng.uniform(0.83, 1.12)
            if k == 1 and j == 0:
                th += 1.43
                length *= 0.75
                high *= 0.57
            if j == 1:
                high *= 0.84
                length *= 0.92
            leaf(
                base,
                th,
                high,
                length,
                rng.uniform(0.010, 0.021),
                rng.uniform(-1.16, 1.16),
                k % 4 == 1 and form != 3,
            )
    if close == "guardian":
        vertices = []
        colours = []
        faces = []
        for j in range(2 + form % 2):
            th = math.pi + (-0.67 + j * 0.58 + form * 0.075)
            l = 0.25 + 0.036 * j
            axis = np.array([math.cos(th), 0, math.sin(th)])
            wide = np.array([-math.sin(th), 0, math.cos(th)]) * 0.019
            base = np.array([0, -0.012, (j - 1) * 0.028])
            mid = base + axis * l * 0.50 + [0, 0.16 + 0.018 * j, 0]
            tip = base + axis * l + [0, 0.135 - 0.012 * j, 0]
            add(
                [base, mid - wide, mid + wide, tip],
                [
                    [0.065, 0.080, 0.029],
                    [0.127, 0.152, 0.042],
                    [0.150, 0.177, 0.063],
                    [0.214, 0.20, 0.076],
                ],
                [(0, 1, 2), (1, 3, 2)],
            )
    p = np.array(vertices, dtype="<f4")
    tri = np.array(faces, dtype="<u2")
    v = np.cross(p[tri[:, 1]] - p[tri[:, 0]], p[tri[:, 2]] - p[tri[:, 0]])
    back = v[:, 1] < 0
    tri[back] = tri[back][:, [0, 2, 1]]
    v[back] *= -1
    N = np.zeros_like(p)
    for i in range(3):
        np.add.at(N, tri[:, i], v)
    N /= np.maximum(1e-9, np.linalg.norm(N, axis=1, keepdims=True))
    return p, N, np.array(colours, dtype="<f4"), tri
