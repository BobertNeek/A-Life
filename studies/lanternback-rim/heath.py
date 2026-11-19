"""Three rooted shoulder communities. Each plant is a linked, editable specimen."""

import math, json
from collections import Counter
import numpy as np
from scipy.spatial.transform import Rotation
from island_erratic_glb import streams
from finished_island_glb import save
from strata import BOX, nz
from fissures import Piece


def model(kind, variant):
    r = np.random.default_rng(
        408 + variant * 17 + 73 * ["heather", "pennon", "crust"].index(kind)
    )
    g = Piece()
    Y = np.array([0, 1, 0])
    base = np.array([0, -0.015, 0])

    def direction(a):
        return np.array([math.cos(a), 0, math.sin(a)])

    def leaf(a, b, w, rise, col):
        d = b - a
        lat = np.array([-d[2], 0, d[0]])
        lat /= max(0.00001, np.linalg.norm(lat))
        m = 0.54 * (b - a) + a + Y * rise
        pts = np.array([a, m - lat * w, m + Y * (rise * 0.36), m + lat * w, b])
        g.face(
            pts,
            np.array(col) * np.array([0.60, 0.86, 1.06, 0.95, 1.10])[:, None],
            [(0, 1, 2), (0, 2, 3), (1, 4, 2), (2, 4, 3)],
        )

    def twig(a, b, w, col):
        d = b - a
        n = np.cross(d, Y)
        n /= max(1e-8, np.linalg.norm(n))
        q = np.cross(d, n)
        q /= max(1e-8, np.linalg.norm(q))
        start = [
            a + (n * math.cos(j * 2.094) + q * math.sin(j * 2.094)) * w
            for j in range(3)
        ]
        end = [b + (p - a) * 0.43 for p in start]
        g.face(
            np.array(start + end),
            np.array(col) * np.array([0.70, 0.82, 0.78, 1, 0.99, 0.95])[:, None],
            [(i, (i + 1) % 3, 3 + i) for i in range(3)]
            + [((i + 1) % 3, 3 + (i + 1) % 3, 3 + i) for i in range(3)],
        )

    if kind == "heather":
        for j in range(7):
            d = direction(j * 2.399963 + variant * 0.81 + r.uniform(-0.37, 0.37))
            length = r.uniform(0.27, 0.58)
            a = base + d * 0.026
            m = d * length * 0.46 + Y * r.uniform(0.13, 0.20)
            end = d * length + np.array([0.07, r.uniform(0.22, 0.38), -0.02])
            twig(a, m, 0.010, [118, 106, 80])
            twig(m, end, 0.0075, [147, 127, 90])
            for k in range(5):
                t = 0.13 + 0.159 * k
                start = m + (end - m) * t
                for side in (-1, 1):
                    axis = direction(
                        j * 2.399963 + variant * 0.81 + side * r.uniform(0.59, 1.28)
                    )
                    l = r.uniform(0.086, 0.149) * (1 - 0.28 * t)
                    p = start + axis * l + Y * r.uniform(0.024, 0.072)
                    leaf(
                        start,
                        p,
                        l * 0.28,
                        0.019,
                        np.array([122, 140, 106]) * r.uniform(0.85, 1.09),
                    )
                    leaf(
                        start + 0.48 * (p - start),
                        p + axis * 0.026 + Y * 0.019,
                        l * 0.15,
                        0.011,
                        [151, 156, 111],
                    )
            if j % 2 == 0:
                leaf(
                    end - Y * 0.019,
                    end + d * 0.042 + Y * 0.026,
                    0.015,
                    0.006,
                    [187, 161, 113],
                )
    elif kind == "pennon":
        for j in range(13):
            d = direction(j * 2.39996 + variant)
            reach = r.uniform(0.12, 0.34)
            leaf(
                base,
                base + d * reach + Y * r.uniform(0.09, 0.22),
                r.uniform(0.018, 0.034),
                0.038,
                [141, 155, 126],
            )
        for j in range(4):
            d = direction(j * 2.4 + variant + 0.18)
            p = base + d * r.uniform(0.065, 0.18) + Y * r.uniform(0.22, 0.39)
            twig(base, p, 0.0065, [154, 136, 103])
            for k in range(3):
                a = p + Y * k * 0.026
                v = direction(j * 2.1 + variant + 2.4 * k)
                leaf(
                    a,
                    a + v * (0.045 + 0.013 * k) + Y * (0.047 - 0.009 * k),
                    0.024,
                    0.015,
                    np.array([167, 121, 112]) + k * np.array([8, 10, 6]),
                )
    else:
        for j in range(14):
            d = direction(j * 2.4 + variant * 1.2)
            l = r.uniform(0.045, 0.16)
            p = base + d * l * 0.34
            end = d * l + Y * r.uniform(0.036, 0.10)
            leaf(
                p,
                end,
                r.uniform(0.019, 0.043),
                0.024,
                np.array([151, 156, 119]) * r.uniform(0.83, 1.11),
            )
    return g


def build(b, out):
    r = np.random.default_rng(38044)
    x0, x1, z0, z1 = BOX
    n = 5700
    x = r.uniform(x0 + 1, x1 - 1, n)
    z = r.uniform(z0 + 1, z1 - 1, n)
    h, sl, d, br, mi, fi, rock, skirt, edge = b.field(x, z)
    rows = []
    counts = Counter()
    for i in range(n):
        if not 0.27 < sl[i] < 0.9 or edge[i] < 0.25 or skirt[i] < 0.17:
            continue
        shoulder = (
            (1 - rock[i]) * (0.15 + 0.75 * skirt[i]) * (0.10 + 0.90 * (mi[i] + 0.5))
        )
        prob = 0.11 * shoulder + 0.011 * rock[i]
        if r.random() > prob:
            continue
        kind = (
            "crust" if rock[i] > 0.52 else "pennon" if r.random() < 0.34 else "heather"
        )
        sz = r.uniform(0.7, 1.26)
        x0_, z0_ = x[i], z[i]
        for j in range(r.integers(3, 14) if kind != "crust" else r.integers(8, 23)):
            xx, zz = np.array([x0_, z0_]) + r.normal(0, [1.11, 0.63], 2) * sz
            sc = sz * r.uniform(0.56, 1.09)
            if b.blocked(
                xx, zz, sc * dict(heather=0.78, pennon=0.44, crust=0.21)[kind]
            ):
                continue
            yy, gx, gz = b.floor(xx, zz)
            s = np.hypot(gx, gz)
            if s > 0.93:
                continue
            # Plants inherit part of the true tangent, the lichen cushions inherit
            # almost all of it. Every origin remains on its actual native triangle.
            k = 0.95 if kind == "crust" else 0.63
            normal = np.array([-gx * k, 1.0, -gz * k])
            normal /= np.linalg.norm(normal)
            axis = np.cross([0, 1, 0], normal)
            q = Rotation.from_rotvec(
                axis / max(1e-8, np.linalg.norm(axis)) * np.arccos(normal[1])
            ) * Rotation.from_rotvec([0.0, r.uniform(-math.pi, math.pi), 0])
            var = int(r.integers(1, 4))
            rows.append([kind + str(var), xx, yy, zz, sc, *q.as_quat()])
            counts[kind] += 1
    doc = dict(
        asset=dict(version="2.0", generator="Lanternback wind-pruned heath"),
        scene=0,
        scenes=[dict(nodes=[])],
        nodes=[],
        meshes=[],
        accessors=[],
        bufferViews=[],
        materials=[
            dict(
                name="Wax leaf and occasional rose scale",
                doubleSided=True,
                pbrMetallicRoughness=dict(roughnessFactor=0.83, metallicFactor=0.0),
            )
        ],
        extensionsUsed=["EXT_mesh_gpu_instancing"],
        extensionsRequired=["EXT_mesh_gpu_instancing"],
    )
    buf = bytearray()
    emit = streams(doc, buf)
    tri = 0
    unique = 0
    for key in sorted({row[0] for row in rows}):
        m = model(key[:-1], int(key[-1]))
        p = np.array(m.p, "<f4")
        t = np.array(m.t, "<u2")
        c = np.clip(np.array(m.c) / 255, 0, 1)
        nn = np.zeros_like(p)
        fac = np.cross(p[t[:, 1]] - p[t[:, 0]], p[t[:, 2]] - p[t[:, 0]])
        for j in range(3):
            np.add.at(nn, t[:, j], fac)
        nn /= np.maximum(1e-9, np.linalg.norm(nn, axis=1, keepdims=True))
        c = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
        rgba = np.column_stack((np.uint8(np.rint(c * 255)), np.full(len(p), 255, "B")))
        ix = len(doc["meshes"])
        att = dict(
            POSITION=emit(p, 5126, "VEC3", bounds=True),
            NORMAL=emit(nn, 5126, "VEC3"),
            COLOR_0=emit(rgba, 5121, "VEC4", normalized=True),
        )
        doc["meshes"].append(
            dict(
                name=key,
                primitives=[
                    dict(
                        attributes=att,
                        indices=emit(t.ravel(), 5123, "SCALAR", indices=True),
                        material=0,
                    )
                ],
            )
        )
        rs = np.array([rr[1:] for rr in rows if rr[0] == key], "<f4")
        s = rs[:, 3]
        at = dict(
            TRANSLATION=emit(rs[:, :3], 5126, "VEC3", bounds=True),
            SCALE=emit(np.column_stack([s, s, s]), 5126, "VEC3"),
            ROTATION=emit(rs[:, 4:], 5126, "VEC4"),
        )
        doc["nodes"].append(
            dict(
                name=key,
                mesh=ix,
                extensions={"EXT_mesh_gpu_instancing": dict(attributes=at)},
            )
        )
        doc["scenes"][0]["nodes"].append(ix)
        tri += len(t) * len(rs)
        unique += len(t)
    doc["buffers"] = [dict(byteLength=len(buf))]
    save(doc, buf, out / "wind-heath.glb")
    with (out / "heath-roots.csv").open("w") as f:
        f.write("specimen,x,ground_y,z,scale,qx,qy,qz,qw\n")
        for r_ in rows:
            f.write(r_[0] + "," + ",".join(f"{v:.6f}" for v in r_[1:]) + "\n")
    return dict(
        communities=dict(counts),
        roots=len(rows),
        palette_triangles=unique,
        all_instances_triangles=tri,
    )
