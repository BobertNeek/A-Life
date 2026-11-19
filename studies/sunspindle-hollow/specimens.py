"""Four fictional dry-slack species. Solid, folded silhouettes with real branching."""

import math
import numpy as np
from finished_island_glb import save
from island_erratic_glb import streams

TAU = math.tau


class Model:
    def __init__(self):
        self.p = []
        self.c = []
        self.t = []

    def add(self, pts, colors, faces):
        k = len(self.p)
        self.p.extend(pts)
        self.c.extend(np.broadcast_to(colors, (len(pts), 3)))
        self.t.extend(tuple(k + i for i in f) for f in faces)

    def blade(self, a, b, width, base, *, bow=0.06, roll=0.25):
        a, b = np.asarray(a), np.asarray(b)
        v = b - a
        n = np.array([v[1], -v[0], 0.0])
        n /= max(np.linalg.norm(n), 1e-7)
        height = np.array([0, 0, bow])
        m = a + 0.48 * v + height
        w = n * width
        tip = a + 0.81 * v + height * 0.57
        pts = [
            a,
            m - w,
            m + height * 0.30,
            m + w,
            tip - w * 0.55,
            tip + height * 0.1,
            tip + w * 0.55,
            b,
        ]
        c = (
            np.array(base)
            * np.array([0.55, 0.87, 1.04, 1 + roll, 0.98, 1.12, 1 + roll, 1.05])[
                :, None
            ]
        )
        self.add(
            pts,
            c,
            [
                (0, 2, 1),
                (0, 3, 2),
                (1, 2, 4),
                (2, 5, 4),
                (2, 3, 6),
                (2, 6, 5),
                (4, 5, 7),
                (5, 6, 7),
            ],
        )

    def rod(self, pts, widths, base, sides=4):
        ring = []
        color = []
        fs = []
        p = np.array(pts)
        for j, (cen, width) in enumerate(zip(p, widths)):
            v = p[min(j + 1, len(p) - 1)] - p[max(j - 1, 0)]
            v /= max(np.linalg.norm(v), 1e-9)
            n = np.cross(v, [1, 0, 0.13])
            n /= max(np.linalg.norm(n), 1e-9)
            w = np.cross(v, n)
            for i in range(sides):
                th = TAU * i / sides
                ring.append(cen + width * (n * math.cos(th) + w * math.sin(th)))
                color.append(
                    np.array(base)
                    * (0.74 + 0.20 * j / max(len(p) - 1, 1) + 0.10 * math.cos(th))
                )
            if j:
                for i in range(sides):
                    k = (j - 1) * sides + i
                    o = (j - 1) * sides + (i + 1) % sides
                    fs.extend([(k, o, k + sides), (o, o + sides, k + sides)])
        self.add(ring, color, fs)

    def spindle(self, cen, r, h, th, cream):
        p = []
        c = []
        f = []
        n = 6
        for j, (rr, zz) in enumerate(
            [(0.24, -0.51), (1.0, -0.19), (0.73, 0.2), (0.05, 0.56)]
        ):
            for i in range(n):
                t = th + i * TAU / n
                p.append(
                    cen
                    + [
                        math.cos(t) * rr * r + 0.14 * r * zz,
                        math.sin(t) * rr * r,
                        zz * h,
                    ]
                )
                c.append(
                    np.array(cream)
                    * ([0.68, 0.90, 1.04, 1.14][j])
                    * (0.95 + 0.06 * math.cos(t * 3))
                )
                if j:
                    a = (j - 1) * n + i
                    b = (j - 1) * n + (i + 1) % n
                    f.extend([(a, b, b + n), (a, b + n, a + n)])
        self.add(p, c, f)

    def crown(self, center, angle, size, R, maturity=0.45):
        # Three recurved pearl bracts wrap the amber seed. They grow apart
        # at the tip: this is not a billboarded flower or spherical lamp.
        self.spindle(
            center + [0, 0, 0.021 * size],
            0.025 * size,
            0.092 * size,
            angle,
            (193, 140, 62),
        )
        for i in range(3):
            a = angle + i * TAU / 3
            v = np.array([math.cos(a), math.sin(a), 0])
            n = np.array([-v[1], v[0], 0])
            H = 0.083 * size
            W = 0.036 * size
            t = R.uniform(0.87, 1.12)
            bloom = max(0, min(1, maturity + 0.16 * math.sin(i * 2.17 + angle)))
            spread = 1 + 0.89 * bloom
            H *= 1 - 0.42 * bloom
            W *= 1 + 0.25 * bloom
            p = [
                center - [0, 0, 0.022 * size],
                center + v * 0.036 * size - n * W + [0, 0, 0.027 * size],
                center + v * 0.049 * size + [0, 0, 0.023 * size],
                center + v * 0.036 * size + n * W + [0, 0, 0.027 * size],
                center + v * (0.047 * spread) * size - n * W * 0.61 + [0, 0, H * 0.74],
                center + v * (0.056 * spread) * size + [0, 0, H * 0.73],
                center + v * (0.047 * spread) * size + n * W * 0.61 + [0, 0, H * 0.74],
                center
                + v * (0.026 + 0.086 * bloom) * size
                + n * 0.012 * bloom * size
                + [0, 0, H * t],
            ]
            colors = np.array(
                [
                    [116, 132, 102],
                    [168, 163, 139],
                    [205, 199, 162],
                    [177, 163, 143],
                    [219, 212, 182],
                    [234, 224, 181],
                    [199, 184, 158],
                    [241, 227, 186],
                ]
            )
            # Open ripe bracts go warm rose at the seam, green at the
            # calyx. A raised double vein and the slightly recoiling tip
            # give the foreground specimens their age rather than identical cups.
            colors = colors.astype(float)
            colors[[1, 3, 4, 6]] += np.array([15.0, -11.0, 5.0]) * bloom
            colors[2] = [186, 158, 130]
            colors[5] = [216, 188, 164]
            if bloom > 0.5:
                vein = np.array([p[0] * 0.27 + p[2] * 0.73, p[2], p[5], p[7]])
                self.rod(
                    vein,
                    [0.0014 * size, 0.0018 * size, 0.0014 * size, 0.0005 * size],
                    [194, 153, 130],
                    3,
                )
                start = center + v * 0.011 * size + [0, 0, 0.016 * size]
                tip = center + v * 0.029 * size + [0, 0, 0.066 * size]
                self.rod(
                    [start, tip], [0.0016 * size, 0.0011 * size], [206, 165, 92], 3
                )
                self.spindle(tip, 0.0043 * size, 0.013 * size, a, [226, 183, 109])
            self.add(
                p,
                colors,
                [
                    (0, 1, 2),
                    (0, 2, 3),
                    (1, 4, 2),
                    (4, 5, 2),
                    (2, 5, 6),
                    (2, 6, 3),
                    (4, 7, 5),
                    (5, 7, 6),
                ],
            )


def plant(kind, v, detail):
    R = np.random.default_rng(
        2231
        + 31 * v
        + 851 * ["milkspindle", "copper-rush", "silver-mat", "windwillow"].index(kind)
    )
    g = Model()
    base = np.array([0.0, 0.0, -0.039])
    wind = np.array([0.55, -0.25, 0.0])

    def axis(th):
        return np.array([math.cos(th), math.sin(th), 0.0])

    if kind == "windwillow":
        for j in range(7 if detail else 5):
            th = j * TAU / 7 + 0.5 * v + R.uniform(-0.29, 0.29)
            aim = axis(th)
            span = R.uniform(0.44, 0.94)
            hi = R.uniform(0.39, 0.77)
            root = base + aim * 0.043
            elbow = 0.31 * span * aim + 0.1 * wind + [0, 0, 0.29 * hi]
            end = aim * span + wind * 0.11 + [0, 0, hi]
            g.rod([root, elbow, end], [0.018, 0.012, 0.005], [143, 117, 81])
            for i in range(5 if detail else 3):
                f = 0.24 + i * (0.145 if detail else 0.27)
                pivot = elbow + (end - elbow) * f
                for side in [-1, 1]:
                    w = axis(th + side * R.uniform(0.57, 1.14))
                    length = R.uniform(0.15, 0.295) * (1 - 0.22 * f)
                    tip = pivot + w * length + [0, 0, R.uniform(0.035, 0.14)]
                    if detail:
                        g.rod([pivot, tip], [0.0061, 0.0024], [124, 129, 92], 3)
                    for l in range(3 if detail else 2):
                        start = pivot + (tip - pivot) * (0.27 * l)
                        vec = axis(th + side * (0.65 + 0.41 * l))
                        t = R.uniform(0.085, 0.159) * (1 if detail else 1.30)
                        g.blade(
                            start,
                            start + vec * t + [0, 0, 0.063],
                            t * 0.25,
                            np.array([143, 159, 126]) * (0.87 + 0.16 * f),
                            bow=0.024,
                            roll=0.11,
                        )
            if detail and j % 2 == 0:
                g.spindle(end + [0, 0, 0.01], 0.019, 0.079, th, [176, 148, 93])
    elif kind == "silver-mat":
        for j in range(16 if detail else 9):
            th = j * 2.39996 + R.uniform(-0.21, 0.21)
            aim = axis(th)
            reach = R.uniform(0.17, 0.42)
            tip = aim * reach + [0, 0, R.uniform(0.035, 0.17)]
            g.blade(
                base + aim * 0.017,
                tip,
                reach * R.uniform(0.11, 0.17),
                np.array([148, 167, 144]) * R.uniform(0.91, 1.10),
                bow=R.uniform(0.036, 0.085),
                roll=0.15,
            )
            if detail and j % 5 == 0:
                g.blade(
                    base + aim * reach * 0.38 + [0, 0, 0.060],
                    tip + axis(th + 0.7) * 0.061,
                    reach * 0.042,
                    [165, 179, 150],
                    bow=0.019,
                )
    else:
        milk = kind == "milkspindle"
        for j in range((14 if detail else 8) if milk else (22 if detail else 13)):
            th = j * 2.39996 + R.uniform(-0.30, 0.30)
            vec = axis(th)
            radius = R.uniform(0.16, 0.45)
            height = R.uniform(0.09, 0.40) if milk else R.uniform(0.13, 0.46)
            tip = vec * radius + wind * 0.066 + [0, 0, height]
            col = np.array([107, 144, 115] if milk else [154, 135, 81]) * R.uniform(
                0.80, 1.17
            )
            g.blade(
                base + vec * 0.019,
                tip,
                R.uniform(0.022, 0.038) if milk else R.uniform(0.009, 0.017),
                col,
                bow=R.uniform(0.03, 0.09),
                roll=0.18,
            )
        for j in range((4 if detail else 3) if milk else (8 if detail else 5)):
            th = j * 2.39996 + v * 0.74 + R.uniform(-0.21, 0.21)
            aim = axis(th)
            h = R.uniform(0.37, 0.70) if milk else R.uniform(0.40, 0.82)
            reach = R.uniform(0.08, 0.31)
            root = base + aim * 0.009
            mid = aim * reach * 0.31 + [0, 0, h * 0.64]
            end = aim * reach + wind * 0.103 + [0, 0, h]
            g.rod(
                [root, mid, end],
                [0.008, 0.006, 0.0035] if milk else [0.005, 0.004, 0.0021],
                [132, 152, 109] if milk else [163, 130, 79],
                3,
            )
            if milk:
                g.crown(
                    end,
                    th,
                    R.uniform(0.77, 1.17),
                    R,
                    ((v - 1) * 0.37 + 0.19 * j) % 1.12,
                )
                if j == 1 and detail:
                    a = mid + (end - mid) * 0.61
                    b = a + axis(th + 0.66) * 0.087 + [0, 0, 0.052]
                    g.rod([a, b], [0.004, 0.0028], [142, 148, 106], 3)
                    g.crown(b, th + 0.7, 0.65, R, 0.18)
            else:
                for k in range(3 if detail else 2):
                    s = 0.25 + k * 0.25
                    o = mid + (end - mid) * s
                    vec = axis(th + (0.72 if k % 2 else -0.76))
                    tip = o + vec * (0.093 - 0.014 * k) + [0, 0, 0.026]
                    if detail:
                        g.rod([o, tip], [0.003, 0.0017], [171, 134, 81], 3)
                    g.spindle(
                        tip,
                        0.012 if detail else 0.017,
                        0.057,
                        th,
                        [185, 135, 82] if k % 2 else [207, 162, 94],
                    )
    return g


def distant(kind, v):
    """Whole plant outline expressed in folded opaque clusters, never alpha quads."""
    r = np.random.default_rng(9123 + v * 7)
    g = Model()
    B = np.array([0, 0, -0.039])
    if kind == "windwillow":
        for i in range(7):
            a = 2.39996 * i + v * 0.3
            d = np.array([math.cos(a), math.sin(a), 0])
            N = np.array([d[1], -d[0], 0])
            h = r.uniform(0.4, 0.70)
            span = r.uniform(0.5, 0.90)
            tip = d * span + [0.06, 0, h]
            m = (tip + B) * 0.52 + [0, 0, 0.10]
            leaf = np.array([145, 158, 120]) * r.uniform(0.83, 1.05)
            g.add(
                [
                    B,
                    m - N * 0.115,
                    m + [0, 0, 0.08],
                    m + N * 0.13,
                    tip - N * 0.084,
                    tip + N * 0.09,
                    tip + d * 0.08,
                ],
                leaf * np.array([0.48, 0.77, 1.05, 1.10, 0.91, 1.06, 1.12])[:, None],
                [(0, 2, 1), (0, 3, 2), (1, 2, 4), (2, 5, 4), (3, 5, 2), (4, 5, 6)],
            )
    else:
        milk = kind == "milkspindle"
        cu = kind == "copper-rush"
        for i in range(6 if cu else 5):
            a = i * 2.4 + v * 0.47
            d = np.array([math.cos(a), math.sin(a), 0])
            N = np.array([d[1], -d[0], 0])
            span = r.uniform(0.22, 0.41)
            h = r.uniform(0.10, 0.36) if (milk or cu) else r.uniform(0.03, 0.13)
            tip = d * span + [0, 0, h]
            m = d * (span * 0.54) + [0, 0, h * 0.74 + 0.035]
            w = 0.029 if cu else 0.048 if milk else 0.060
            col = np.array(
                [146, 130, 79] if cu else [122, 147, 115] if milk else [153, 170, 140]
            )
            g.add(
                [B, m - N * w, m + N * w, m + [0, 0, 0.03], tip],
                col * np.array([0.6, 0.88, 1.10, 1.04, 1.08])[:, None],
                [(0, 1, 3), (0, 3, 2), (1, 4, 3), (3, 4, 2)],
            )
            if i < 3 and (cu or milk):
                tip = d * r.uniform(0.09, 0.23) + [0.055, 0, r.uniform(0.44, 0.69)]
                w = 0.010 if cu else 0.016
                g.add(
                    [B - N * 0.011, B + N * 0.011, tip + N * 0.008, tip - N * 0.008],
                    [145, 140, 91],
                    [(0, 1, 2), (0, 2, 3)],
                )
                col = [205, 154, 93] if cu else [218, 209, 170]
                o = 0.025 if cu else 0.047
                g.add(
                    [
                        tip + [0, 0, -0.035],
                        tip - N * o,
                        tip + d * o,
                        tip + N * o,
                        tip + [0, 0, 0.066],
                    ],
                    np.array(col) * np.array([0.65, 0.9, 1.07, 0.95, 1.09])[:, None],
                    [(0, 1, 2), (0, 2, 3), (1, 4, 2), (2, 4, 3)],
                )
    return g


def write(out, rows, detail):
    doc = dict(
        asset=dict(version="2.0", generator="Sunspindle / dry eastern slack"),
        scene=0,
        scenes=[dict(nodes=[])],
        nodes=[],
        meshes=[],
        bufferViews=[],
        accessors=[],
        materials=[
            dict(
                name="Dry slack / cuticle, awn and pearl",
                doubleSided=True,
                pbrMetallicRoughness=dict(metallicFactor=0.0, roughnessFactor=0.77),
            )
        ],
        extensionsUsed=["EXT_mesh_gpu_instancing"],
        extensionsRequired=["EXT_mesh_gpu_instancing"],
    )
    buf = bytearray()
    emit = streams(doc, buf)
    total = 0
    visible_tri = 0
    for name in sorted({r[0] for r in rows}):
        base, level = name.split(" / ") if " / " in name else (name, detail)
        kind, v = base.rsplit("-", 1)
        g = plant(kind, int(v), True) if level == "near" else distant(kind, int(v))
        p = np.asarray(g.p, dtype="<f4")[:, [0, 2, 1]].copy()
        # x,z,height author coordinates have opposite handedness to native x,y,z.
        t = np.asarray(g.t, dtype="<u2")[:, [0, 2, 1]].copy()
        c = np.clip(np.asarray(g.c) / 255, 0, 1)
        nn = np.zeros_like(p)
        fac = np.cross(p[t[:, 1]] - p[t[:, 0]], p[t[:, 2]] - p[t[:, 0]])
        for k in range(3):
            np.add.at(nn, t[:, k], fac)
        nn /= np.maximum(1e-10, np.linalg.norm(nn, axis=1, keepdims=True))
        lin = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
        rgba = np.column_stack(
            (np.uint8(np.rint(255 * lin)), np.full(len(p), 255, "B"))
        )
        att = dict(
            POSITION=emit(p, 5126, "VEC3", bounds=True),
            NORMAL=emit(nn, 5126, "VEC3"),
            COLOR_0=emit(rgba, 5121, "VEC4", normalized=True),
        )
        m = len(doc["meshes"])
        doc["meshes"].append(
            dict(
                name=name,
                primitives=[
                    dict(
                        attributes=att,
                        indices=emit(t.ravel(), 5123, "SCALAR", indices=True),
                        material=0,
                    )
                ],
            )
        )
        rws = np.asarray([r[1:] for r in rows if r[0] == name], dtype="<f4")
        scl = rws[:, 3]
        rot = rws[:, 4]
        attrs = dict(
            TRANSLATION=emit(rws[:, :3], 5126, "VEC3", bounds=True),
            SCALE=emit(np.column_stack((scl, scl, scl)), 5126, "VEC3"),
            ROTATION=emit(
                np.column_stack(
                    (rot * 0, np.sin(rot / 2), rot * 0, np.cos(rot / 2))
                ).astype("<f4"),
                5126,
                "VEC4",
            ),
        )
        doc["nodes"].append(
            dict(
                name=name,
                mesh=m,
                extensions={"EXT_mesh_gpu_instancing": {"attributes": attrs}},
            )
        )
        doc["scenes"][0]["nodes"].append(m)
        total += len(t)
        visible_tri += len(rws) * len(t)
    doc["buffers"] = [dict(byteLength=len(buf))]
    save(doc, buf, out)
    return dict(
        specimens=len(doc["meshes"]),
        unique_triangles=total,
        triangles_over_all_instances=visible_tri,
    )
