"""Grounded till and bent woodrush below an actual wooded glacial deposit.

No buried foundation or invented boulder: regolith collects in discontinuous
lenses in the lee and on two sides of real source footprints. Above the
ground only a few cm of stone and thin, folded grass leaves are exposed.
An offline tile for the selected native deposit, not a global scatter.
"""

from pathlib import Path
import json, math
import numpy as np
from scipy.spatial import ConvexHull
from build_island_strandline import Beach
from finished_island_glb import parse, values, save
from island_inlet_sod import Fibres


def unit(p):
    return p / max(1e-9, np.linalg.norm(p))


def bake(surface, original_library, props, dest, sites=(1570, 1383, 1675)):
    land = Beach(Path(surface))
    rocks = Path(original_library)
    props = json.loads(Path(props).read_text())
    groups = {}
    for index in sites:
        prop = props[index]
        kind = prop["kind"]
        if kind not in ("BoulderA", "BoulderB", "BoulderC"):
            raise ValueError("requires an original boulder")
        d, b = parse(rocks / (kind + ".glb"))
        prim = d["meshes"][0]["primitives"][0]
        p = np.array(values(d, b, prim["attributes"]["POSITION"]))
        center = np.asarray(prop["position"])[[0, 2]]
        s, c = np.sin(prop["yaw"]), np.cos(prop["yaw"])
        footprint = p[:, [0, 2]] @ np.array([[c, -s], [s, c]]) * prop["scale"] + center
        hull = ConvexHull(np.unique(footprint, axis=0))
        eq = hull.equations
        rng = np.random.default_rng(20913 + index * 2161)
        h, dx, dz = land.surface(*center)
        down = unit(-np.array([dx, dz]))
        cross = np.array([-down[1], down[0]])
        g = Fibres()

        def point(xz, up, col):
            return g.point([xz[0], land.surface(*xz)[0] + up, xz[1]], col)

        def outside(xz, bias=0.0):
            return np.max(eq[:, :2] @ xz + eq[:, 2]) > bias

        def edge(vec):
            den = eq[:, :2] @ vec
            return center + vec * min(
                (-(eq[:, :2] @ center + eq[:, 2]) / np.maximum(1e-9, den))[den > 1e-7]
            )

        def flake(xz, length, slender, rose=False):
            angle = math.atan2(down[1], down[0]) + rng.normal(0.77, 0.68)
            a = np.array([math.cos(angle), math.sin(angle)])
            b = np.array([-a[1], a[0]])
            side = rng.integers(5, 8)
            rad = rng.uniform(0.74, 1.2, side)
            base, top = [], []
            height = min(0.049, length * rng.uniform(0.21, 0.34))
            ink = np.array(
                rng.choice(
                    (
                        (125, 128, 123),
                        (115, 117, 112),
                        (139, 139, 127),
                        (133, 124, 110),
                        (147, 142, 125),
                    )
                )
            )
            ink = ink + ([8, 0, -3] if rose else [0, 0, 0])
            for j in range(side):
                t = math.tau * j / side
                dr = (
                    length * rad[j] * math.cos(t) * a
                    + length * slender * rad[j] * math.sin(t) * b
                )
                base.append(point(xz + dr, -0.008, ink * 0.80))
                top.append(
                    point(
                        xz + dr * rng.uniform(0.57, 0.81) + a * length * 0.07,
                        height * (0.78 + 0.16 * math.cos(t + 0.4)),
                        ink * (1 + 0.04 * math.cos(t)),
                    )
                )
            for j in range(side):
                k = (j + 1) % side
                g.face(base[j], base[k], top[k])
                g.face(base[j], top[k], top[j])
            for j in range(1, side - 1):
                g.face(top[0], top[j], top[j + 1])

        def woodrush(xz, scale):
            count = rng.integers(7, 13)
            ph = rng.uniform(0, math.tau)
            for j in range(count):
                angle = ph + j * 2.399 + rng.normal(0, 0.32)
                a = np.array([math.cos(angle), math.sin(angle)])
                b = np.array([-a[1], a[0]])
                length = scale * rng.uniform(0.42, 1.2)
                root = xz + a * rng.uniform(0.001, 0.027)
                if not outside(root, -0.045):
                    continue
                c = np.array(
                    rng.choice(
                        ((105, 125, 70), (114, 132, 78), (127, 139, 81), (142, 139, 84))
                    )
                )
                rows = []
                for j, t in enumerate((0, 0.42, 0.78, 0.99)):
                    v = root + length * (
                        (0.07 * t + 0.68 * t * t) * a + 0.17 * t * t * down
                    )
                    ww = length * 0.070 * (1 - 0.90 * t)
                    hh = length * (1.21 * t - 0.73 * t * t) - 0.005
                    rows.append(
                        [
                            point(
                                v + b * ww * k,
                                hh + (0.004 * (1 - t) if k == 0 else 0),
                                c * (0.63 + 0.43 * t + (0.045 if k == 0 else 0)),
                            )
                            for k in (-1, 0, 1)
                        ]
                    )
                for aa, bb in zip(rows, rows[1:]):
                    for n in range(2):
                        g.face(aa[n], bb[n], bb[n + 1])
                        g.face(aa[n], bb[n + 1], aa[n + 1])

        # Three irregular shoals, not a decorative pebble collar or circular mat.
        # Fine fragments thin rapidly downhill, bare gaps separate the other lenses.
        for which, az in enumerate(
            (
                rng.uniform(-0.55, 0.21),
                rng.uniform(0.92, 1.72),
                rng.uniform(-2.35, -1.42),
            )
        ):
            heading = down * math.cos(az) + cross * math.sin(az)
            base = edge(heading) + heading * 0.11
            axis = unit(0.71 * down + 0.45 * heading + 0.14 * cross)
            tangent = np.array([-axis[1], axis[0]])
            span = (2.4, 1.0, 0.85)[which] * prop["scale"]
            for j in range((45, 25, 13)[which]):
                t = rng.beta(0.98, 2.2)
                pos = base + t * span * axis + rng.normal(0, 0.095 + 0.17 * t) * tangent
                sz = 0.018 + rng.lognormal(-3.15, 0.52) * (1 - 0.45 * t)
                if outside(pos, 0.025):
                    flake(pos, min(0.145, sz), rng.uniform(0.42, 0.81), j % 17 == 4)
            if which != 0:
                grove = base + 0.17 * tangent * np.sign(az)
                for j in range((8, 5)[which - 1]):
                    pos = (
                        grove
                        + axis * rng.uniform(-0.08, 0.64)
                        + tangent * rng.normal(0, 0.13)
                    )
                    if outside(pos, 0.015):
                        woodrush(pos, rng.uniform(0.11, 0.29))
        groups[index] = g
    doc = dict(
        asset=dict(version="2.0", generator="Qualia: rooted glacial till and woodrush"),
        scene=0,
        scenes=[dict(nodes=[])],
        nodes=[],
        meshes=[],
        accessors=[],
        bufferViews=[],
        materials=[
            dict(
                name="Dry mineral and living woodrush",
                doubleSided=True,
                pbrMetallicRoughness=dict(metallicFactor=0, roughnessFactor=0.88),
            )
        ],
    )
    binary = bytearray()
    counts = {}

    def attr(a, ct, ty, indices=False, color=False, bounds=False):
        a = np.ascontiguousarray(a)
        binary.extend(b"\0" * (-len(binary) % 4))
        off = len(binary)
        binary.extend(a.tobytes())
        j = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=off,
                byteLength=a.nbytes,
                target=34963 if indices else 34962,
            )
        )
        record = dict(bufferView=j, componentType=ct, count=len(a), type=ty)
        if color:
            record["normalized"] = True
        if bounds:
            record.update(min=a.min(0).tolist(), max=a.max(0).tolist())
        i = len(doc["accessors"])
        doc["accessors"].append(record)
        return i

    for i, (index, g) in enumerate(groups.items()):
        origin = np.array(props[index]["position"])
        v = (np.asarray(g.v) - origin).astype("<f4")
        t = np.array(g.tri, "<u2")
        n = np.zeros_like(v)
        fn = np.cross(v[t[:, 1]] - v[t[:, 0]], v[t[:, 2]] - v[t[:, 0]])
        for j in range(3):
            np.add.at(n, t[:, j], fn)
        n /= np.maximum(1e-8, np.linalg.norm(n, axis=1, keepdims=True))
        attrs = dict(
            POSITION=attr(v, 5126, "VEC3", bounds=True),
            NORMAL=attr(n, 5126, "VEC3"),
            COLOR_0=attr(np.asarray(g.c, "u1"), 5121, "VEC4", color=True),
        )
        name = f"Till_{index}"
        doc["meshes"].append(
            dict(
                name=name,
                primitives=[
                    dict(
                        attributes=attrs,
                        indices=attr(t.ravel(), 5123, "SCALAR", indices=True),
                        material=0,
                    )
                ],
            )
        )
        doc["nodes"].append(dict(name=name, mesh=i, translation=origin.tolist()))
        doc["scenes"][0]["nodes"].append(i)
        counts[str(index)] = len(t)
    doc["buffers"] = [dict(byteLength=len(binary))]
    dest = Path(dest)
    dest.parent.mkdir(exist_ok=True, parents=True)
    save(doc, binary, dest)
    return counts
