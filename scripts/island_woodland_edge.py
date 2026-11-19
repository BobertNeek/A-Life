"""The real V1 two-spruce dripline, sunlit rushes and glacial wash.

The material is plotted on the original terrain, not an offset ground mesh.
At the brush margin contiguous living tongues follow the slope; the two
large gaps are intentional. No new rocks, trees or interaction cues.
"""

from pathlib import Path
import json, math
import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter, map_coordinates
from island_inlet_ground import Inlet, ease
from island_inlet_sod import Fibres
from finished_island_glb import save
from island_erratic_glb import streams

X0, X1, Z0, Z1 = 239.0, 277.0, -219.0, -181.0
N = 704


class Edge:
    def __init__(self, surface, placements):
        self.land = Inlet(surface)
        props = json.loads(Path(placements).read_text())
        self.origin = np.array(props[131]["position"])[[0, 2]]
        end = np.array(props[1303]["position"])[[0, 2]]
        self.u = end - self.origin
        self.length = np.linalg.norm(self.u)
        self.u /= self.length
        self.v = np.array([-self.u[1], self.u[0]])
        self.props = props

    def world(self, u, v):
        return self.origin + self.u * u + self.v * v

    def uv(self, x, z):
        xx, zz = x - self.origin[0], z - self.origin[1]
        return xx * self.u[0] + zz * self.u[1], xx * self.v[0] + zz * self.v[1]

    def fields(self, x, z):
        u, v = self.uv(x, z)
        # Separate true stem driplines, smaller fern umbra and their joined lee.
        cover = np.zeros_like(u)
        for i, ru, rv in (
            (131, 6.6, 5.8),
            (111, 7.0, 4.8),
            (2510, 4.2, 3.2),
            (49, 4.7, 4.9),
        ):
            tx, _, tz = self.props[i]["position"]
            pu, pv = self.uv(tx, tz)
            cover = np.maximum(
                cover, np.exp(-(((u - pu - 1.7) / ru) ** 2) - ((v - pv) / rv) ** 2)
            )
        # Tongues are broad and asymmetrical. A genuine open turf passage
        # cuts across the edge and a mineral wash opens below the boulder.
        winding = 0.58 * np.sin(u * 0.44) + 0.34 * np.sin(u * 0.83 + v * 0.26)
        moss = np.maximum.reduce(
            [
                np.exp(-(((u - a) / b) ** 2) - ((v - c - winding) / d) ** 2)
                for a, b, c, d in (
                    (3.9, 6.0, -0.9, 3.2),
                    (9.6, 3.8, 1.9, 1.8),
                    (11.1, 4.0, -3.4, 2.2),
                    (15.8, 3.5, -2.6, 1.7),
                    (19.7, 2.8, 0.8, 1.35),
                )
            ]
        )
        corridor = ease(
            0.9, 3.2, np.abs(v - (4.9 - 0.30 * u + 1.12 * np.sin(u * 0.24 + 0.3)))
        )
        valley = np.exp(
            -(((u - 19.8) / 6.2) ** 2)
            - ((v - 1.6 + 1.85 * np.sin(u * 0.24)) / 3.5) ** 2
        )
        return u, v, cover, moss * (0.23 + 0.77 * corridor), valley

    def y(self, p):
        return float(self.land.floor(np.array(float(p[0])), np.array(float(p[1])))[0])


def paint(edge, out):
    j, i = np.mgrid[0:N, 0:N]
    X = X0 + (i + 0.5) * (X1 - X0) / N
    Z = Z0 + (j + 0.5) * (Z1 - Z0) / N
    u, v, cover, moss, grit = edge.fields(X, Z)
    h, dx, dz = edge.land.floor(X.copy(), Z.copy())
    r = np.random.default_rng(77141)

    def plen(s):
        t = gaussian_filter(r.normal(size=X.shape).astype("f"), s * N / (X1 - X0))
        return t / max(0.0001, t.std())

    coarse, clumps, grain, pepper = [plen(k) for k in (1.5, 0.42, 0.105, 0.030)]
    # Broad ecotone before individual curls. Finer patches do not create
    # breakup on bare ground, and the transparent border joins the original.
    feather = ease(0, 3.4, np.minimum.reduce([X - X0, X1 - X, Z - Z0, Z1 - Z]))
    hollow = (gaussian_filter(h, 13) - h) / 0.4
    humus = (
        ease(0.09, 0.85, cover + 0.094 * moss + 0.068 * coarse + 0.053 * clumps)
        * feather
    )
    plant = (
        ease(
            0.21,
            0.86,
            moss * (0.81 + 0.10 * coarse + 0.14 * clumps + 0.071 * grain)
            + 0.10 * cover
            + 0.10 * np.clip(hollow, -1, 1),
        )
        * feather
    )
    wash = (
        ease(0.16, 0.88, grit * (0.85 + 0.13 * coarse + 0.14 * clumps + 0.07 * grain))
        * (1 - 0.7 * plant)
        * feather
    )
    # Actual needle bed and pebble grey show through between olive crowns.
    # The Alpha is coverage; the source ground underneath always survives.
    masses = np.stack(
        [0.64 * humus * (1 - 0.50 * plant), 0.72 * plant, 0.42 * wash], -1
    )
    amount = np.clip(masses.sum(-1), 0, 0.82)
    base = np.asarray([[108.0, 83, 54], [119, 132, 70], [152, 144, 119]])
    col = (masses @ base) / np.maximum(0.00001, masses.sum(-1, keepdims=True))
    col += (2.8 * coarse + 4.1 * clumps + 3.5 * grain + 2.2 * pepper)[..., None]
    # Two kinds of curled fallen needles run along the actual local catchment,
    # never across the clearing. Broken loose lengths, not painted stripes.
    rgba = Image.fromarray(
        np.uint8(np.dstack([np.clip(col, 0, 255), np.clip(255 * amount, 0, 255)]))
    )
    pen = ImageDraw.Draw(rgba)
    for k in range(4800):
        row, col_i = r.integers(2, N - 3, 2)
        f = humus[row, col_i]
        if f < 0.14 or r.random() > f * 0.86:
            continue
        ang = (0.68 if k % 3 else -0.82) + r.normal(0, 0.74)
        length = r.uniform(0.65, 3.9)
        xx = int(col_i + math.cos(ang) * length)
        yy = int(row + math.sin(ang) * length)
        old = np.array(rgba.getpixel((int(col_i), int(row))))
        ink = tuple(
            np.uint8(
                np.clip(
                    old[:3] + r.choice([-1.0, 1.0]) * np.array([20, 16, 11]), 0, 255
                )
            )
        ) + (int(old[3]),)
        pen.line([(int(col_i), int(row)), (xx, yy)], fill=ink, width=1)
    rgba.save(out / "woodland-bed.png", optimize=True)
    return {
        "bounds": [X0, X1, Z0, Z1],
        "chart_texels": [N, N],
        "painted_area_m2": round(
            float((amount > 0.12).sum()) * ((X1 - X0) / N) ** 2, 2
        ),
    }


def growth(edge, out):
    g = Fibres()
    r = np.random.default_rng(16583)

    def point(p, up, col):
        return g.point([p[0], edge.y(p) + up, p[1]], col)

    def blade(origin, angle, length, half_width, color, fold=0.53):
        d = np.array([math.cos(angle), math.sin(angle)])
        side = np.array([-d[1], d[0]])
        bow = r.uniform(-0.20, 0.20)
        lee = edge.u * r.uniform(0.02, 0.25)
        foot = edge.y(origin)
        last = None
        for t, width, arch in (
            (0, 0.18, 0),
            (0.29, 0.92, 0.65),
            (0.72, 0.76, 1.0),
            (1, 0, 0.68),
        ):
            loc = (
                origin
                + (
                    d * (t * t * 0.77)
                    + lee * t * t
                    + side * (math.sin(t * math.pi) * bow)
                )
                * length
            )
            yy = max(
                edge.y(loc) + (0.006 if t else -0.004),
                foot + length * arch * fold - 0.006,
            )
            rgb = color * (0.65 + 0.38 * t)
            if t == 1:
                tip = g.point((loc[0], yy, loc[1]), rgb)
                g.face(last[0], last[1], tip)
                g.face(last[1], last[2], tip)
                break
            row = []
            for side_n in (-1, 0, 1):
                q = loc + side * half_width * width * side_n
                row.append(
                    g.point(
                        (
                            q[0],
                            yy + (half_width * 0.38 * width if not side_n else 0),
                            q[1],
                        ),
                        rgb * (1.08 if not side_n else 0.94),
                    )
                )
            if last:
                for n in (0, 1):
                    g.face(last[n], row[n], row[n + 1])
                    g.face(last[n], row[n + 1], last[n + 1])
            last = row

    # Above-ground plants occur as connected communities. More candidates
    # near the leading lip, wide gaps between metres-long irregular tongues.
    plants = leaves = 0
    for a, b, c, d in (
        (2.0, 1.9, -1.5, 1.1),
        (6.2, 2.3, 0.1, 1.4),
        (10.0, 1.6, 2.0, 0.85),
        (11.7, 2.2, -3.7, 0.85),
        (16.1, 1.8, -2.95, 0.66),
        (20.1, 1.35, 0.95, 0.55),
    ):
        for j in range(67):
            u, v = r.normal(a, b), r.normal(c, d)
            p = edge.world(u, v)
            _, _, shade, moss, wash = edge.fields(*p)
            if moss < 0.38 or r.random() > min(0.95, float(moss) ** 1.2):
                continue
            if (
                min(
                    np.linalg.norm(p - np.array(edge.props[k]["position"])[[0, 2]])
                    / edge.props[k]["scale"]
                    for k in (1383, 1303)
                )
                < 2.35
            ):
                continue
            scale = r.uniform(0.19, 0.49) * (1 - 0.45 * shade)
            tone = np.array(
                r.choice(
                    [
                        (126, 141, 77),
                        (140, 152, 86),
                        (117, 139, 82),
                        (156, 148, 85),
                        (120, 140, 93),
                    ]
                )
            ) * r.uniform(0.94, 1.06)
            az = r.uniform(0, math.tau)
            count = int(r.integers(6, 12))
            plants += 1
            for k in range(count):
                angle = az + 2.399 * k + r.normal(0, 0.29)
                root = p + r.normal(0, 0.024, 2)
                blade(
                    root,
                    angle,
                    scale * r.uniform(0.58, 1.17),
                    r.uniform(0.009, 0.021),
                    tone * r.uniform(0.87, 1.04),
                    fold=r.uniform(0.48, 0.9),
                )
                leaves += 1
    # Actual low paired leaves on joined evergreen runners. Each runner
    # terminal leaves a branching daughter connected to it, not a new clump.
    runners = 0
    for a, b, c, d in (
        (4.0, 1.45, -1.2, 0.9),
        (8.4, 1.38, 1.5, 0.8),
        (13.7, 1.55, -3.7, 0.7),
    ):
        for j in range(20):
            p = edge.world(r.normal(a, b), r.normal(c, d))
            if edge.fields(*p)[3] < 0.46:
                continue
            ang = math.atan2(edge.u[1], edge.u[0]) + r.normal(0.2, 0.9)
            c0 = np.array(r.choice([(105, 130, 94), (119, 142, 99), (131, 145, 97)]))
            runners += 1
            for k in range(int(r.integers(6, 11))):
                ang += r.normal(0, 0.21)
                direct = np.array([math.cos(ang), math.sin(ang)])
                side = np.array([-direct[1], direct[0]])
                q = p + direct * r.uniform(0.095, 0.16)
                l = point(p - side * 0.007, 0.009, (118, 98, 58))
                rr = point(p + side * 0.007, 0.009, (127, 108, 63))
                ll = point(q - side * 0.005, 0.014, (132, 111, 72))
                rb = point(q + side * 0.005, 0.014, (141, 117, 75))
                g.face(l, ll, rb)
                g.face(l, rb, rr)
                for sign in (-1, 1):
                    length = r.uniform(0.065, 0.139)
                    blade(
                        q,
                        ang + sign * r.uniform(0.82, 1.23),
                        length,
                        length * r.uniform(0.15, 0.24),
                        c0,
                        fold=r.uniform(0.34, 0.52),
                    )
                    leaves += 1
                p = q
    doc = dict(
        asset={"version": "2.0", "generator": "Qualia two-spruce woodland edge"},
        scene=0,
        scenes=[{"nodes": [0]}],
        nodes=[],
        meshes=[],
        materials=[
            {
                "name": "Wind-combed understory, no food state",
                "doubleSided": True,
                "pbrMetallicRoughness": {"metallicFactor": 0, "roughnessFactor": 0.87},
            }
        ],
        bufferViews=[],
        accessors=[],
    )
    buf = bytearray()
    attr = streams(doc, buf)
    origin = np.array([*edge.world(12, 0), 0.0])
    origin = np.array([origin[0], edge.y(origin[:2]), origin[1]])
    v = (np.array(g.v) - origin).astype("<f4")
    t = np.array(g.tri, dtype="<u4")
    n = np.zeros_like(v)
    fn = np.cross(v[t[:, 1]] - v[t[:, 0]], v[t[:, 2]] - v[t[:, 0]])
    for k in range(3):
        np.add.at(n, t[:, k], fn)
    n /= np.maximum(1e-9, np.linalg.norm(n, axis=1, keepdims=True))
    idx_type = 5123 if len(v) < 65536 else 5125
    idx = t.ravel().astype("<u2" if idx_type == 5123 else "<u4")
    at = dict(
        POSITION=attr(v, 5126, "VEC3", bounds=True),
        NORMAL=attr(n, 5126, "VEC3"),
        COLOR_0=attr(np.array(g.c, "B"), 5121, "VEC4", normalized=True),
    )
    doc["meshes"].append(
        dict(
            name="WoodlandEdge",
            primitives=[
                dict(
                    attributes=at,
                    indices=attr(idx, idx_type, "SCALAR", indices=True),
                    material=0,
                )
            ],
        )
    )
    doc["nodes"].append(
        dict(name="Two spruce margin", mesh=0, translation=origin.tolist())
    )
    doc["buffers"] = [dict(byteLength=len(buf))]
    save(doc, buf, out / "woodland-edge.glb")
    return dict(
        rush_stools=plants, connected_runners=runners, leaves=leaves, triangles=len(t)
    )


def build(surface, placements, folder):
    out = Path(folder)
    out.mkdir(parents=True, exist_ok=True)
    edge = Edge(surface, placements)
    data = {
        **paint(edge, out),
        **growth(edge, out),
        "actual_canopy": [111, 131],
        "fern": 2510,
        "scree": 1303,
        "boulder": 1383,
        "origin": edge.origin.tolist(),
        "span_between_real_anchors_metres": round(float(edge.length), 2),
        "scope": "Offline woodland edge at original V1 placements. Change no physical surface, landmarks, saves, lights or gameplay.",
    }
    (out / "woodland-source.json").write_text(json.dumps(data, indent=2) + "\n")
    return data
