"""Little calcareous tide cups: real open throats, four to six chipped valves.

Mesh units are one horizontal shell radius. Placement supplies the exact native
surface, tangent, yaw and metre scale. These are static decorative casts.
"""

from pathlib import Path
import csv, sys, json, math
import numpy as np

HERE = Path(__file__).resolve().parent
sys.path[:0] = [
    str(HERE.parents[1] / "lanternback-rim"),
    str(HERE.parents[2] / "scripts"),
]
from fissures import Piece, export
from finished_island_glb import parse, save, values, set_values
from island_erratic_glb import streams


def model(kind, middle):
    rng = np.random.default_rng(720 + int(kind) * 41)
    g = Piece()
    n = {1: 5, 2: 6, 3: 4}[kind]
    phase = 0.78 * kind

    # The settling skin is buried at its lowest edge. Five/six valves open
    # around a copper-red cavity, with a papery chalk-coloured rolled lip.
    def ring(t, angle, offset, height, shift=0.0):
        strain = (
            0.86
            + 0.13 * math.sin(3 * angle + phase)
            + 0.05 * math.cos(2 * angle + phase * 0.5)
        )
        rr = (t + offset) * strain
        x = rr * math.cos(angle) + 0.16 * shift
        z = rr * math.sin(angle) * (0.86 + 0.05 * kind) - 0.11 * shift
        return np.array([x, height, z])

    def patch(rows, col):
        w = len(rows[0])
        idx = len(g.p)
        g.p.extend(np.concatenate(rows))
        g.c.extend(np.concatenate(col))
        for i in range(len(rows) - 1):
            for j in range(w - 1):
                a = idx + i * w + j
                g.t.extend(((a, a + w + 1, a + 1), (a, a + w, a + w + 1)))

    # One opening per shell. A matured pale cast has two broken rims at
    # different phases; the others hold their mineralised teeth.
    for j in range(n):
        life = 1 if kind != 3 else 0.56
        broken = kind == 3 and j in (1, 3)
        angle = phase + j * math.tau / n
        dz = rng.uniform(-0.045, 0.045)
        tone = rng.uniform(0.88, 1.08)
        rr = []
        cc = []
        for radius, y, shift in (
            (1.02, -0.048, 0),
            (0.94, 0.065, 0),
            (0.73, 0.28, 0.3),
            (0.52, 0.51, 0.76),
            (0.35, 0.49, 0.97),
            (0.30, 0.27, 1.0),
        ):
            row = []
            clr = []
            coords = (-1.0, -0.52, 0, 0.52, 1.0) if not middle else (-1.0, 0, 1.0)
            for w in coords:
                fang = (1 - abs(w)) ** 1.3
                A = angle + w * (math.pi / n - 0.018) + 0.035 * shift
                h = (y + dz * shift) * life + (
                    0.16 * fang * (1 if not broken else -0.98)
                ) * shift
                if kind == 1:
                    h *= 0.83
                rid = 0.042 * fang - 0.025 * abs(w)
                row.append(ring(radius, A, rid * (1 - shift * 0.4), h, shift))
                if radius < 0.38:
                    ink = (
                        np.array([146.0, 92.0, 65.0])
                        if radius < 0.33
                        else np.array([210.0, 184.0, 140.0])
                    )
                elif radius < 0.55:
                    ink = np.array([205.0, 190.0, 154.0])
                elif radius < 0.8:
                    ink = (
                        np.array([169.0, 152.0, 111.0])
                        + np.array([12.0, 14.0, 19.0]) * fang
                    )
                else:
                    ink = np.array([112.0, 113.0, 91.0])
                if kind == 2:
                    ink = ink * np.array([1.07, 0.96, 0.87])
                if broken:
                    ink = 0.66 * ink + 0.34 * np.array([165, 156, 143])
                clr.append(ink * tone * (0.87 + 0.13 * fang))
            rr.append(row)
            cc.append(clr)
        if middle:
            rr = [r for k, r in enumerate(rr) if k in (0, 2, 3, 5)]
            cc = [r for k, r in enumerate(cc) if k in (0, 2, 3, 5)]
        patch(rr, cc)
    # A deep oxidised throat, not a bright painted black dot.
    pts = [[0.09, 0.075, -0.06]]
    cs = [[84, 68, 61]]
    for j in range(n * 2):
        a = phase + j * math.tau / (n * 2)
        pts.append(ring(0.34, a, 0.0, 0.14, 1.0))
        cs.append([126, 99, 73] if kind != 3 else [147, 133, 107])
    idx = len(g.p)
    g.p.extend(pts)
    g.c.extend(cs)
    g.t.extend((idx, idx + 1 + (j + 1) % (n * 2), idx + 1 + j) for j in range(n * 2))
    return g


def compose(table, out, middle):
    doc, raw = parse(table)
    emit = streams(doc, raw)
    ledger = list(csv.DictReader((out / "shell-seat.csv").open()))
    stats = {}
    for node in doc["nodes"]:
        form = node["name"][-1]
        m = doc["meshes"][node["mesh"]]
        prim = m["primitives"][0]
        a = prim["attributes"]["POSITION"]
        v = np.array(values(doc, raw, a), "<f4") + np.array(
            node.pop("translation"), "<f4"
        )
        set_values(doc, raw, a, v)
        doc["accessors"][a].update(min=v.min(0).tolist(), max=v.max(0).tolist())
        rr = [q for q in ledger if q["form"] == form]
        trans = np.array(
            [[float(q[k]) for k in ("x", "native_y", "z")] for q in rr], "<f4"
        )
        rot = np.array(
            [[float(q[k]) for k in ("qx", "qy", "qz", "qw")] for q in rr], "<f4"
        )
        scale = np.array([[float(q["radius_m"])] * 3 for q in rr], "<f4")
        att = dict(
            TRANSLATION=emit(trans, 5126, "VEC3", bounds=True),
            ROTATION=emit(rot, 5126, "VEC4"),
            SCALE=emit(scale, 5126, "VEC3"),
        )
        node["extensions"] = {"EXT_mesh_gpu_instancing": {"attributes": att}}
        stats[form] = dict(
            roots=len(rr),
            triangles_each=doc["accessors"][prim["indices"]]["count"] // 3,
        )
    doc["extensionsUsed"] = doc["extensionsRequired"] = ["EXT_mesh_gpu_instancing"]
    doc["buffers"][0]["byteLength"] = len(raw)
    save(doc, raw, out / ("cups-middle.glb" if middle else "cups-near.glb"))
    return stats


def author(out):
    out.mkdir(exist_ok=True, parents=True)
    report = {}
    for middle in (False, True):
        key = "middle" if middle else "near"
        obj = {f"SeaCup{i}": model(i, middle) for i in (1, 2, 3)}
        path = out / ("specimens-" + key + ".glb")
        export(obj, path, "Limestone rim, copper throat, weathered shell")
        report[key] = compose(path, out, middle)
    (out / "shells.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    author(Path(sys.argv[1]))
