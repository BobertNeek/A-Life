"""Low mineral rosettes, warm awned windgrass, shade trifolium and fern fans.

Species overlap in a real two-spruce woodland; vary their stature with
exposure and true-soil shelter, rather than placing four planting strips.
No prop, food, growth simulation or collision identity is established here.
"""

import math
import numpy as np
from island_inlet_sod import Fibres
from island_woodland_edge import write_growth_mesh


def communities(edge, out):
    r = np.random.default_rng(640572)
    g = Fibres()
    stats = {k: [] for k in ("fern", "shade_leaf", "awn_grass", "grit_rosette")}

    def pt(p, y, c):
        return g.point([p[0], float(y), p[1]], c)

    def vec(a):
        return np.array([math.cos(a), math.sin(a)])

    def side(d):
        return np.array([-d[1], d[0]])

    def ribbon(a, b, ya, yb, w, c):
        v = b - a
        v /= max(1e-6, np.linalg.norm(v))
        n = side(v)
        base = pt(a - n * w, ya, c * 0.92)
        rb = pt(a + n * w, ya, c)
        tip = pt(b + n * w * 0.62, yb, c * 1.05)
        lt = pt(b - n * w * 0.62, yb, c * 0.96)
        g.face(base, lt, tip)
        g.face(base, tip, rb)

    def leaf(root, angle, span, width, base_y, arch, col, habit):
        # Folded spoon/heart or spindly grass. A real centre ridge, not
        # a textured transparent plane. Trifolium terminal has a true cleft.
        u = vec(angle)
        n = side(u)
        bend = r.uniform(-0.17, 0.17)
        prev = [pt(root, base_y, col * 0.79)]
        profiles = {
            "heart": ((0.34, 0.60, 0.58), (0.79, 1.0, 1.0)),
            "spoon": ((0.34, 0.61, 0.63), (0.72, 1.0, 1.0)),
            "blade": ((0.26, 0.94, 0.69), (0.69, 0.72, 1.0)),
        }
        for t, w, h in profiles[habit]:
            c = root + span * (u * t + n * bend * math.sin(t * math.pi))
            yy = max(edge.y(c) + 0.004, base_y + h * arch)
            row = [
                pt(
                    c + n * k * w * width,
                    yy + (width * 0.27 * w if k == 0 else 0.0),
                    col
                    * (
                        0.87
                        + 0.13 * t
                        + (0.075 if k == 0 else 0.015 if k < 0 else -0.018)
                    ),
                )
                for k in (-1, 0, 1)
            ]
            if len(prev) == 1:
                g.face(prev[0], row[0], row[1])
                g.face(prev[0], row[1], row[2])
            else:
                for k in (0, 1):
                    g.face(prev[k], row[k], row[k + 1])
                    g.face(prev[k], row[k + 1], prev[k + 1])
            prev = row
        q = root + u * span
        yy = max(
            edge.y(q) + 0.003, base_y + arch * (0.69 if habit != "heart" else 0.88)
        )
        if habit == "heart":
            a, b = pt(q + n * width * 0.39, yy, col * 1.05), pt(
                q - n * width * 0.39, yy, col * 1.02
            )
            notch = pt(q - u * width * 0.42, yy + width * 0.07, col)
            g.face(prev[0], b, notch)
            g.face(prev[0], notch, prev[1])
            g.face(prev[1], notch, a)
            g.face(prev[1], a, prev[2])
        else:
            tip = pt(q, yy, col * 1.03)
            g.face(prev[0], prev[1], tip)
            g.face(prev[1], prev[2], tip)

    def fern(p, size):
        floor = edge.y(p) - 0.008
        green = np.array(
            r.choice([(134, 161, 89), (125, 151, 83), (147, 162, 97)]), float
        )
        az = r.uniform(0, math.tau)
        for j in range(int(r.integers(4, 7))):
            a = az + j * 2.399 + r.normal(0, 0.27)
            u = vec(a)
            n = side(u)
            L = size * r.uniform(0.78, 1.26)
            last = p
            ly = floor
            for k in range(1, 8):
                t = k / 7
                cur = p + L * (
                    u * (0.18 * t + 0.91 * t * t) + n * 0.075 * math.sin(t * math.pi)
                )
                cy = max(edge.y(cur) + 0.008, floor + L * (1.13 * t - 0.54 * t * t))
                ribbon(
                    last,
                    cur,
                    ly,
                    cy,
                    0.0045 * (1 - 0.70 * t),
                    green * np.array([0.96, 0.92, 0.79]),
                )
                if k > 1:
                    for sign in (-1, 1):
                        pin = (
                            L
                            * (0.019 + 0.277 * (math.sin(t * math.pi * 0.90) ** 0.9))
                            * (1.0 - 0.28 * t)
                            * r.uniform(0.89, 1.09)
                        )
                        vein = u * 0.41 + n * sign * 0.93
                        q = cur + vein * pin
                        root = pt(cur, cy, green * 0.84)
                        off = u * pin * 0.31
                        mid = cur + (q - cur) * 0.58
                        left = pt(mid - off, cy + pin * 0.15, green * (0.9 + 0.13 * t))
                        ridge = pt(mid, cy + pin * 0.25, green * (1.04 + 0.04 * t))
                        right = pt(
                            mid + off, cy + pin * 0.14, green * (0.88 + 0.08 * t)
                        )
                        tip = pt(q, cy + pin * 0.37, green * (1.08 + 0.03 * t))
                        for tri in (
                            (root, left, ridge),
                            (root, ridge, right),
                            (left, tip, ridge),
                            (ridge, tip, right),
                        ):
                            g.face(*tri)
                last, ly = cur, cy

    def shade_leaf(p, size):
        ground = edge.y(p) - 0.004
        az = r.uniform(0, math.tau)
        ink = np.array(
            r.choice(
                [(119, 149, 100), (132, 157, 104), (119, 149, 115), (145, 163, 108)]
            ),
            float,
        )
        for i in range(int(r.integers(3, 7))):
            a = az + i * 2.399 + r.normal(0, 0.28)
            d = vec(a)
            # A three-lobed leaf held above an angled petiole. The leaf's
            # mass is at its actual tip, not strung out along a wire runner.
            L = r.uniform(0.6, 1.12) * size
            q = p + d * L * 0.49
            h = max(ground + L * 0.69, edge.y(q) + 0.025)
            ribbon(p, q, ground, h, 0.0028, ink * 0.89)
            for k in (0, 1, 2):
                span = L * r.uniform(0.49, 0.81)
                leaf(
                    q,
                    a + (k - 1) * r.uniform(1.73, 2.17),
                    span,
                    span * 0.55,
                    h,
                    span * 0.26,
                    ink * r.uniform(0.91, 1.1),
                    "heart",
                )

    def awn(p, size):
        ground = edge.y(p) - 0.006
        az = r.uniform(0, math.tau)
        green = np.array(
            r.choice([(147, 156, 89), (155, 154, 91), (133, 148, 93), (167, 159, 96)]),
            float,
        )
        for k in range(int(r.integers(6, 11))):
            angle = az + k * 2.399 + r.normal(0, 0.38)
            L = size * r.uniform(0.60, 1.23)
            leaf(
                p + vec(angle) * 0.012,
                angle,
                L,
                L * r.uniform(0.049, 0.087),
                ground,
                L * r.uniform(0.33, 0.61),
                green * r.uniform(0.89, 1.06),
                "blade",
            )
        if r.random() < 0.33:
            d = vec(az + r.normal(0, 0.45))
            L = size * r.uniform(0.82, 1.2)
            q = p + d * L * 0.35
            y = ground + L * 0.94
            ribbon(p, q, ground, y, 0.0028, green * 1.03)
            for k in range(3):
                start = q + d * L * 0.05 * k + side(d) * L * 0.025 * (k % 2)
                zy = y + L * 0.057 * k
                tip = start + d * L * 0.19 + side(d) * (-1) ** k * L * 0.13
                w = L * 0.016
                l = pt(start - side(d) * w, zy, green * np.array([1.23, 1.10, 0.96]))
                rr = pt(start + side(d) * w, zy, green * 1.12)
                far = pt(tip, zy + L * 0.19, green * np.array([1.22, 1.15, 1.08]))
                g.face(l, rr, far)

    def rosette(p, size):
        ground = edge.y(p) + 0.005
        az = r.uniform(0, math.tau)
        ink = np.array(
            r.choice(
                [(160, 176, 149), (153, 172, 143), (167, 177, 150), (141, 162, 136)]
            ),
            float,
        )
        for k in range(int(r.integers(6, 11))):
            a = az + k * 2.399 + r.normal(0, 0.32)
            L = size * r.uniform(0.57, 1.11)
            leaf(
                p,
                a,
                L,
                L * r.uniform(0.20, 0.36),
                ground,
                L * r.uniform(0.18, 0.38),
                ink * r.uniform(0.89, 1.07),
                "spoon",
            )

    def ellipse(u, v, x, z, w, h, skew=0.0):
        return math.exp(-(((u - x) / w) ** 2) - ((v - z - (u - x) * skew) / h) ** 2)

    def niche(p):
        x, z = p
        u, v, cover, _, wash = edge.fields(x, z)
        _, gx, gz = edge.land.floor(np.array(x), np.array(z))
        grade = math.hypot(gx, gz)
        # Overlapping, two-dimensional coves in the original woodland,
        # rather than planting along paths or bands at fixed distances.
        region = max(
            ellipse(u, v, 3.8, 0.0, 8.3, 6.6, -0.15),
            ellipse(u, v, 12.6, 1.0, 6.2, 5.3, -0.35),
            ellipse(u, v, 19.4, 2.1, 4.7, 5.8, 0.35),
        )
        opening = 1 - 0.90 * ellipse(u, v, 10.1, -2.6, 4.9, 2.6, 0.47)
        edge_noise = (
            0.53
            + 0.18 * math.sin(0.83 * u + 0.65 * v)
            + 0.14 * math.cos(0.71 * u - 1.13 * v)
            + 0.13 * math.sin(0.45 * u + 0.29 * v)
        )
        common = max(0, region * opening * (0.46 + 0.88 * edge_noise) - 0.10)
        # This real fern root, not the rectangle centre, seeds the fern margin.
        fx, _, fz = edge.props[2510]["position"]
        df = math.hypot(x - fx, z - fz)
        f = (1 - math.exp(-((df / 0.8) ** 2))) * math.exp(-(((df - 3.1) / 5.0) ** 2))
        # A sheltered middle hollow breaks the one-direction succession.
        hollow = ellipse(u, v, 12.2, 2.6, 4.3, 2.8, -0.55)
        return (
            np.array(
                [
                    common * (f * 0.87 + 0.44 * hollow) * (0.46 + 0.54 * cover),
                    common
                    * (cover * 0.68 + hollow * 0.59)
                    * (0.68 + 0.32 * edge_noise),
                    common
                    * max(0, 0.87 - cover)
                    * (0.5 + 0.88 * hollow + 0.28 * (1 - wash)),
                    common
                    * (0.62 * wash + 0.47 * ellipse(u, v, 19.2, 2.0, 4.5, 3.1))
                    * (1 - cover),
                ]
            ),
            float(cover),
            grade,
        )

    locations = {k: [] for k in stats}
    budget = dict(fern=23, shade_leaf=38, awn_grass=67, grit_rosette=33)
    ids = list(stats)
    # Families arrive at irregular nurse nuclei, then spread into the
    # overlapping habitat. Uniform rejection across 300 m² gave the first
    # study speckles: join siblings into palpable low communities without
    # returning to one stripe per plant type. The final fifth can wander.
    nuclei = {
        "fern": (
            (1.3, -1.9, 1.4, 0.80),
            (5.1, -2.5, 1.5, 1.2),
            (8.4, 2.2, 1.7, 0.9),
            (12.6, 2.2, 1.2, 1.0),
        ),
        "shade_leaf": (
            (3.3, 1.4, 1.6, 1.8),
            (5.7, -2.5, 1.9, 1.3),
            (10.5, 2.9, 2.2, 1.4),
            (15.5, -0.8, 1.1, 0.95),
        ),
        "awn_grass": (
            (8.5, 3.7, 2.5, 1.4),
            (15.3, -0.95, 1.9, 1.45),
            (16.2, 4.1, 1.65, 0.90),
            (20.2, -1.2, 1.5, 1.2),
        ),
        "grit_rosette": (
            (17.3, -1.1, 1.25, 0.83),
            (18.3, 2.3, 1.0, 0.66),
            (21.1, 0.65, 1.59, 1.05),
            (24.4, -1.4, 1.4, 0.62),
        ),
    }
    for _ in range(18000):
        if all(len(locations[k]) >= n for k, n in budget.items()):
            break
        j = int(r.integers(4))
        key = ids[j]
        if len(locations[key]) >= budget[key]:
            continue
        if r.random() < 0.85:
            a, b, wide, deep = nuclei[key][int(r.integers(4))]
            noise_u, noise_v = r.normal(size=2)
            u = a + noise_u * wide
            v = (
                b
                + noise_v * deep
                + 0.22 * noise_u * deep
                + 0.26 * math.sin(1.31 * noise_u) * deep
            )
        else:
            u, v = r.uniform(-5.7, 27.0), r.uniform(-9.3, 10.0)
        p = edge.world(u, v)
        w, shade, grade = niche(p)
        if grade > 0.83 or w.max() < 0.048:
            continue
        w /= np.array([0.57, 0.87, 0.81, 0.69])
        if len(locations[key]) >= budget[key] or r.random() > min(0.87, w[j] ** 1.38):
            continue
        if any(
            np.linalg.norm(p - q) < (0.32 if key == "fern" else 0.17)
            for q in locations[key]
        ):
            continue
        if any(
            np.linalg.norm(p - np.array(edge.props[k]["position"])[[0, 2]]) < limit
            for k, limit in (
                (111, 0.35),
                (131, 0.35),
                (2510, 0.62),
                (1383, 1.78),
                (1303, 1.76),
            )
        ):
            continue
        locations[key].append(p)
        # Height follows resources, not distance alone: tall divided fans
        # can recur in a pocket of shade, but never on exposed shingle.
        wash = float(edge.fields(*p)[4])
        scale = {
            "fern": 0.41 + 0.25 * shade,
            "shade_leaf": 0.13 + 0.105 * shade,
            "awn_grass": 0.20 + 0.16 * (1 - wash),
            "grit_rosette": 0.175,
        }[key]
        daughter = {
            "fern": (1, 3),
            "shade_leaf": (2, 5),
            "awn_grass": (1, 4),
            "grit_rosette": (2, 5),
        }[key]
        spread = {
            "fern": 0.26,
            "shade_leaf": 0.21,
            "awn_grass": 0.40,
            "grit_rosette": 0.23,
        }[key]
        for n in range(int(r.integers(*daughter))):
            q = p + r.normal(0, spread, 2)
            size = scale * r.uniform(0.66, 1.13)
            {
                "fern": fern,
                "shade_leaf": shade_leaf,
                "awn_grass": awn,
                "grit_rosette": rosette,
            }[key](q, size)
            stats[key].append(round(size, 3))
    return dict(
        communities={
            key: dict(
                mother_sites=len(locations[key]),
                rooted_groups=len(a),
                nominal_size_m=[float(min(a)), float(max(a))],
            )
            for key, a in stats.items()
        },
        triangles=write_growth_mesh(g, edge, out),
        scope_of_growth="Shade trifolium / divided nurse fronds; flexible warm awned rush at the opening; short waxy stone rosettes. Rooted only in the bounded ordinary woodland edge, no invented landmarks.",
    )
