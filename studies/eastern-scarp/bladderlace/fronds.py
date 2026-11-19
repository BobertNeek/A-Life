"""Bladderlace: flattened branching fronds with a midrib, paired float bulbs and a socketed holdfast.

Coordinates are in one root-span unit. Local +Y is the original substrate
normal, +Z is down along the native slope; local X spreads across it. It is
static decorative coastal cast art, not part of a creature or its physiology.
"""

from pathlib import Path
import math, json, csv, sys
import numpy as np

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parents[1] / "lanternback-rim"), str(H.parents[2] / "scripts")]
from fissures import Piece, export
from finished_island_glb import parse, save, values, set_values
from island_erratic_glb import streams


def unit(x):
    return x / max(1e-9, np.linalg.norm(x))


def leaf_unit(kind, middle):
    rng = np.random.default_rng(45183 + kind * 291)
    g = Piece()
    up = np.array([0.0, 1.0, 0.0])
    browns = [
        np.array([91, 91, 53.0]),
        np.array([89, 90, 56.0]),
        np.array([125, 90, 53.0]),
    ]
    olives = [
        np.array([124, 150, 79.0]),
        np.array([96, 144, 108.0]),
        np.array([143, 143, 72.0]),
    ]
    tips = [
        np.array([163, 156, 77.0]),
        np.array([134, 161, 97.0]),
        np.array([179, 143, 74.0]),
    ]
    base, leaf, tip = browns[kind - 1], olives[kind - 1], tips[kind - 1]

    def addloop(
        center, axis, side, normal, t, thick, width, k, tones, phase, midrib=False
    ):
        pts = []
        cl = []
        for j in range(k):
            a = 2 * math.pi * j / k
            s, c = math.cos(a), math.sin(a)
            roll = 0.015 * width * math.sin(3 * a + phase)
            section = normal * (thick * c + roll) + side * (width * s)
            if midrib and c > 0:
                section += normal * (thick * 1.15 * (max(0, 1 - abs(s)) ** 3))
            if midrib:
                # The border undulates while the blade remains physically very
                # thin. A notch in the last margin splits the broad kelp tip.
                section += normal * (
                    width * 0.16 * abs(s) ** 2 * math.sin(phase + t * 9.1 + 2 * s)
                )
                section += (
                    axis
                    * (0.034 - 0.067 * (1 - abs(s)) ** 2)
                    * max(0, (t - 0.83) / 0.17)
                )
            pts.append(center + section)
            pigment = tones * (0.91 + 0.09 * c) + np.array([5, 9, 6]) * (
                0.5 + 0.5 * math.sin(phase + 1.8 * t + a)
            )
            if midrib:
                pigment += (
                    np.array([39, 33, 9]) * max(0, c) ** 2 * (1 - abs(s)) ** 2
                    + np.array([12, 11, -1]) * abs(s) ** 4
                )
            cl.append(pigment)
        return pts, cl

    def surface(loops, colors):
        o = len(g.p)
        segments = len(loops[0])
        g.p.extend(np.concatenate(loops))
        g.c.extend(np.concatenate(colors))
        for k in range(len(loops) - 1):
            for j in range(segments):
                p = o + k * segments + j
                q = o + k * segments + (j + 1) % segments
                g.t.extend(((p, q, p + segments), (q, q + segments, p + segments)))

    def blade(start, ang, L, width, curve, phase, main=False, old=False):
        tangent = np.array([math.sin(ang), 0.0, math.cos(ang)])
        side = np.array([math.cos(ang), 0.0, -math.sin(ang)])
        bow = rng.uniform(0.030, 0.095)

        def xy(t):
            return (
                start
                + tangent * (L * t)
                + side
                * (
                    L
                    * (
                        curve * t * t
                        + 0.053 * math.sin(math.pi * t + phase) * math.sin(math.pi * t)
                    )
                )
                + up * (bow * math.sin(math.pi * t) ** 0.9 + 0.028 * t)
            )

        ts = (
            [0.0, 0.16, 0.36, 0.58, 0.80, 0.96, 1.0]
            if middle
            else [0.0, 0.075, 0.16, 0.26, 0.38, 0.50, 0.61, 0.72, 0.83, 0.91, 0.97, 1.0]
        )
        loops = []
        colors = []
        for i, t in enumerate(ts):
            p = xy(t)
            tv = unit(xy(min(1, t + 0.016)) - xy(max(0, t - 0.016)))
            se = unit(np.cross(up, tv))
            no = unit(np.cross(tv, se))
            angle = (0.30 * math.sin(phase + 5 * t) + 0.35 * t * math.sin(phase)) * min(
                1, t * 3 + 0.2
            )
            se1 = se * math.cos(angle) + no * math.sin(angle)
            no1 = no * math.cos(angle) - se * math.sin(angle)
            wscale = (0.10 + 0.90 * min(1.0, t / 0.29)) * (
                0.77 + 0.23 * math.sin(t * math.pi)
            )
            ww = (
                width
                * (0.17 + 0.83 * wscale)
                * (0.84 + 0.16 * math.sin(phase + 10 * t))
            )
            ww *= (1 - 0.47 * max(0, (t - 0.89) / 0.11)) * (
                1 + 0.13 * math.sin(6 * math.pi * t + phase)
            )
            cold = (1 - t) * base * 0.20 + (leaf * (1 - 0.20 * (1 - t)))
            clr = cold * (1 - t * 0.48) + tip * t * 0.48
            if old:
                clr = clr * 0.81 + np.array([145, 113, 67.0]) * 0.19
            v, c = addloop(
                p,
                tv,
                se1,
                no1,
                t,
                0.0013 + 0.0020 * wscale,
                ww,
                8 if middle else 12,
                clr,
                phase,
                True,
            )
            loops.append(v)
            colors.append(c)
        surface(loops, colors)
        return xy

    def bulb(center, tan, half, rad, phase, faded=False):
        tan = unit(tan)
        side = unit(np.cross(up, tan))
        face = unit(np.cross(tan, side))
        ts = np.linspace(-1, 1, 5 if middle else 7)
        loops = []
        cols = []
        for t in ts:
            v = center + tan * t * half
            width = math.sqrt(max(0.009, 1 - t * t)) * rad * (0.97 + 0.08 * t)
            clr = (
                np.array([174, 148, 80]) if faded else np.array([207, 159, 73])
            ) + np.array([17, 10, 3]) * t
            ring, c = addloop(
                v,
                tan,
                side,
                face,
                t,
                width * 0.84,
                width,
                6 if middle else 8,
                clr,
                phase,
            )
            loops.append(ring)
            cols.append(c)
        surface(loops, cols)

    # Buried uneven holdfast rather than star-shaped radiating toes. A few
    # off-centre knobs bind a low saddle and recede into the coloured stone.
    nr = 8 if middle else 11
    loop = []
    col = []
    for j in range(nr):
        q = 2 * math.pi * j / nr
        r = 0.076 * (1 + 0.16 * math.cos(3 * q + kind) + 0.16 * math.sin(2 * q + 0.3))
        loop.append([r * math.cos(q), -0.008, r * 0.65 * math.sin(q)])
        col.append(base * 0.65)
    o = len(g.p)
    g.p.extend(loop)
    g.p.append([-0.015, 0.030, 0.012])
    g.c.extend(col)
    g.c.append(base * 0.85)
    for j in range(nr):
        g.t.append((o + j, o + nr, o + (j + 1) % nr))
    fans = {1: 4, 2: 3, 3: 4}[kind]
    for k in range(fans):
        ang = (
            -1.10 + 2.2 * (k + 0.24 + rng.uniform(-0.15, 0.15)) / fans
        ) + 0.19 * math.sin(kind)
        L = rng.uniform(0.63, 0.94) * (1 if kind != 3 else 0.82)
        phase = rng.uniform(0, math.tau)
        root = np.array([rng.uniform(-0.034, 0.034), 0.010, rng.uniform(-0.035, 0.04)])
        main = blade(
            root,
            ang,
            L,
            rng.uniform(0.074, 0.105),
            rng.uniform(-0.34, 0.30),
            phase,
            True,
        )
        split = rng.uniform(0.45, 0.62)
        for side in (-1, 1):
            if middle and side > 0 and k % 2:
                continue
            h = split + rng.uniform(-0.08, 0.12)
            a = ang + side * rng.uniform(0.44, 0.84)
            st = main(h)
            blade(
                st,
                a,
                L * rng.uniform(0.32, 0.51),
                rng.uniform(0.049, 0.079),
                side * rng.uniform(0.02, 0.15),
                phase + side,
                old=k % 3 == 1,
            )
            if (k + kind) % 3 != 1:
                direction = np.array([math.sin(a), 0.08, math.cos(a)])
                pod = st + direction * 0.095 + up * 0.048
                bulb(pod, direction, 0.062, 0.046, phase + side, faded=kind == 3)
    return g


def assemble(template, out, middle, select=None, output=None):
    d, buf = parse(template)
    emit = streams(d, buf)
    d["materials"][0]["pbrMetallicRoughness"]["roughnessFactor"] = 0.43
    rows = list(csv.DictReader((out / "lace-roots.csv").open()))
    report = {}
    original = d["nodes"]
    nodes = []
    for node in original:
        form = int(node["name"][-1])
        p = d["meshes"][node["mesh"]]["primitives"][0]
        a = p["attributes"]["POSITION"]
        verts = np.asarray(values(d, buf, a), "<f4") + np.array(
            node.pop("translation"), "<f4"
        )
        set_values(d, buf, a, verts)
        d["accessors"][a].update(min=verts.min(0).tolist(), max=verts.max(0).tolist())
        sel = [
            x for x in rows if int(x["form"]) == form and (select is None or select(x))
        ]
        if not sel:
            continue
        nodes.append(node)

        def take(keys, typ, bound=False):
            return emit(
                np.array([[float(r[i]) for i in keys] for r in sel], "<f4"),
                5126,
                typ,
                bounds=bound,
            )

        att = dict(
            TRANSLATION=take(["x", "native_y", "z"], "VEC3", True),
            ROTATION=take(["qx", "qy", "qz", "qw"], "VEC4"),
            SCALE=take(["span_m"] * 3, "VEC3"),
        )
        node["extensions"] = {"EXT_mesh_gpu_instancing": {"attributes": att}}
        report[form] = dict(
            root_count=len(sel), triangles=d["accessors"][p["indices"]]["count"] // 3
        )
    d["extensionsUsed"] = d["extensionsRequired"] = ["EXT_mesh_gpu_instancing"]
    d["buffers"][0]["byteLength"] = len(buf)
    d["nodes"] = nodes
    d["scenes"][0]["nodes"] = list(range(len(nodes)))
    save(
        d,
        buf,
        out
        / (output or ("bladderlace-middle.glb" if middle else "bladderlace-near.glb")),
    )
    return report


def make(out):
    report = {}
    for mid in (False, True):
        n = "middle" if mid else "near"
        path = out / ("lace-forms-" + n + ".glb")
        export(
            {"Bladderlace" + str(i): leaf_unit(i, mid) for i in (1, 2, 3)},
            path,
            "Old gold floats, folded olive blade and dark holdfast",
        )
        specimen, data = parse(path)
        specimen["materials"][0]["pbrMetallicRoughness"]["roughnessFactor"] = 0.43
        save(specimen, data, path)
        report[n] = assemble(path, out, mid)
    (out / "lace-forms.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    make(Path(sys.argv[1]))
