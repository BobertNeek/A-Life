"""Pocketed shell, wet lamina and short wrack on the V1 estuary's lower shoulders.

A weathered few centimetres of tidecast, never a new natural obstacle. This
is independent of the editable inlet proposal and authored village causeways.
"""

from pathlib import Path
from collections import defaultdict
import json, math
import numpy as np
from finished_island_glb import parse, save
from island_inlet_ground import Inlet
from island_inlet_sod import Fibres

# Two loose embayments face the river on different banks, separated by bare
# reaches. Author from actual tide reach, not a continuous water-height ring.
BEDS = [
    (93.0, 451.1, 0.50, 2.5, 0.96),
    (94.5, 447.0, 0.44, 1.12, 0.53),
    (92.5, 442.8, 0.38, 0.92, 0.56),
    (118.3, 448.0, 0.57, 1.6, 0.65),
    (110.3, 432.0, 0.41, 2.2, 0.63),
    (89.8, 427.7, 0.54, 1.6, 0.63),
]


def tidecast(terrain, variant):
    ground, alt = Inlet(terrain), Inlet(variant)
    rng = np.random.default_rng(59184)
    tiles = defaultdict(Fibres)
    num = {
        "shells": 0,
        "shale_pieces": 0,
        "short_wrack_ribbons": 0,
        "contact_shards": 0,
    }

    def ht(p):
        return float(ground.floor(*p)[0])

    def safe(p):
        i = math.floor((p[0] - ground.x0) / ground.s)
        j = math.floor((p[1] - ground.z0) / ground.s)
        return np.array_equal(
            ground.h[j : j + 2, i : i + 2], alt.h[j : j + 2, i : i + 2]
        )

    def pin(g, p, lift, ink):
        return g.point((p[0], ht(p) + lift, p[1]), ink)

    def shell(g, p, az, L, rolled):
        d = np.array((math.cos(az), math.sin(az)))
        c = np.array((-d[1], d[0]))
        # Broken mussel fans: narrowing hinge and blunt, asymmetrically
        # rounded lip. Each growth rib ends at its own chipped outer edge.
        stone = rng.random() < 0.57
        rim = np.array((170, 162, 133) if stone else (172, 173, 162))
        core = np.array((120, 112, 92) if stone else (109, 107, 112))
        w = L * rng.uniform(0.38, 0.55)
        rise = min(0.022, L * 0.23)
        last = None
        for s in (0, 0.24, 0.49, 0.73, 1.0):
            row = []
            for n in range(12):
                a = -1 + 2 * n / 11
                wid = (0.04 + 0.96 * math.sin(s * math.pi * 0.62)) * (1 - 0.14 * a) * w
                pos = p + d * L * (s - 0.46 - 0.24 * s * a * a) + c * wid * a
                if rolled:
                    relief = rise * ((1 - a * a) * math.sin(s * math.pi * 0.73))
                else:
                    relief = rise * (
                        (a * a * 0.71 + 0.10) * math.sin(s * math.pi * 0.81)
                    )
                chip = 0.0009 * (1 if n % 2 else -1) * s + rise * 0.12 * math.sin(
                    s * math.pi
                )
                lift = -0.003 + relief + chip + (rise * 0.19 * s if not rolled else 0)
                col = (core * (1 - s) + rim * s) * (1.025 if n % 2 else 0.94)
                col += (
                    np.array((1, 1, 0.8))
                    * 6
                    * math.sin(s * math.pi * 6 + (a + 0.3) ** 2)
                )
                row.append(pin(g, pos, lift, col))
            if last:
                for i in range(len(row) - 1):
                    g.face(last[i], last[i + 1], row[i + 1])
                    g.face(last[i], row[i + 1], row[i])
            last = row
        num["shells"] += 1

    def flake(g, p, az, L, chip=False):
        d = np.array((math.cos(az), math.sin(az)))
        c = np.array((-d[1], d[0]))
        n = rng.integers(6, 10)
        rough = rng.uniform(0.68, 1.25, n)
        w = L * rng.uniform(0.34, 0.65)
        lift = rng.uniform(0.004, 0.014) if chip else rng.uniform(0.010, 0.026)
        pal = np.array(
            rng.choice(
                [
                    (134, 129, 115),
                    (119, 129, 124),
                    (146, 143, 126),
                    (136, 124, 107),
                    (129, 125, 123),
                ]
            )
        )
        toe = []
        edge = []
        for j in range(n):
            a = math.tau * j / n
            off = rough[j] * (L * math.cos(a) * d + w * math.sin(a) * c)
            q = p + off
            toe.append(pin(g, q, -0.0045, pal * 0.79))
            edge.append(
                pin(
                    g,
                    p + off * rng.uniform(0.74, 0.94),
                    lift * (0.31 + 0.19 * math.sin(a + 0.5)),
                    pal * rng.uniform(0.89, 0.96),
                )
            )
        top = pin(g, p + 0.16 * L * d, lift, pal * (0.97 if chip else 1.08))
        for j in range(n):
            k = (j + 1) % n
            g.face(toe[j], toe[k], edge[k])
            g.face(toe[j], edge[k], edge[j])
            g.face(edge[j], edge[k], top)
        num["contact_shards" if chip else "shale_pieces"] += 1

    def ribbon(g, p, az, L):
        d = np.array((math.cos(az), math.sin(az)))
        c = np.array((-d[1], d[0]))
        ph = rng.uniform(0, math.tau)
        width = rng.uniform(0.004, 0.012)
        tone = np.array(rng.choice([(139, 104, 69), (120, 108, 73), (117, 120, 80)]))
        last = None
        for k in range(10):
            u = k / 9
            q = (
                p
                + d * L * (u - 0.5)
                + c * 0.085 * math.sin(u * 5.2 + ph) * math.sin(math.pi * u)
            )
            w = width * (0.18 + 0.82 * math.sin(math.pi * u) ** 0.6)
            delta = 0.003 + 0.010 * math.sin(math.pi * u) ** 2
            row = [
                pin(
                    g,
                    q + c * j * w,
                    delta + 0.002 * (1 - abs(j)),
                    tone * (0.95 + 0.07 * (j == 0)),
                )
                for j in (-1, 0, 1)
            ]
            if last:
                for j in range(2):
                    g.face(last[j], last[j + 1], row[j + 1])
                    g.face(last[j], row[j + 1], row[j])
            last = row
        num["short_wrack_ribbons"] += 1

    for x, z, wide, long, density in BEDS:
        for idx in range(round(51 * density)):
            step = rng.normal(0, 0.69)
            cove = np.array(
                (
                    x + wide * rng.normal(0, 0.75) + 0.22 * math.sin(step * 2),
                    z + long * step,
                )
            )
            h, dx, dz = ground.floor(*cove)
            grade = np.hypot(dx, dz)
            if not 0.3 < h < 3.3 or not 0.27 < grade < 1.5 or not safe(cove):
                continue
            tile = tiles[math.floor(cove[0] / 50), math.floor(cove[1] / 50)]
            az = math.atan2(dx, -dz) + rng.normal(0, 0.57)
            f = rng.random()
            if f < 0.11:
                shell(tile, cove, az, rng.uniform(0.047, 0.119), rng.random() < 0.59)
            elif f < 0.20:
                ribbon(tile, cove, az, rng.uniform(0.15, 0.36))
            elif f < 0.60:
                flake(tile, cove, az, rng.uniform(0.046, 0.144))
            else:
                flake(tile, cove, az, rng.uniform(0.018, 0.051), True)
    return tiles, num


def embed(glb, terrain, variant):
    glb = Path(glb)
    d, buf = parse(glb)
    tiles, note = tidecast(terrain, variant)
    byname = {o["name"]: o for o in d["nodes"]}

    def access(arr, fmt, kind, bound=False, index=False, norm=False):
        a = np.ascontiguousarray(arr)
        buf.extend(b"\0" * (-len(buf) % 4))
        offset = len(buf)
        buf.extend(a.tobytes())
        vi = len(d["bufferViews"])
        d["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=offset,
                byteLength=a.nbytes,
                target=34963 if index else 34962,
            )
        )
        meta = dict(bufferView=vi, componentType=fmt, count=len(a), type=kind)
        if norm:
            meta["normalized"] = True
        if bound:
            meta.update(min=a.min(axis=0).tolist(), max=a.max(axis=0).tolist())
        ix = len(d["accessors"])
        d["accessors"].append(meta)
        return ix

    count = 0
    for (x, z), g in sorted(tiles.items()):
        n = byname.get(f"Inlet_{x}_{z}")
        if n is None:
            continue
        pos = np.array(g.v, "<f4") - np.array(n["translation"], "<f4")
        faces = np.array(g.tri, "<u2")
        nor = np.zeros_like(pos)
        fn = np.cross(
            pos[faces[:, 1]] - pos[faces[:, 0]], pos[faces[:, 2]] - pos[faces[:, 0]]
        )
        for i in range(3):
            np.add.at(nor, faces[:, i], fn)
        mag = np.linalg.norm(nor, axis=1)
        nor[mag > 1e-9] /= mag[mag > 1e-9, None]
        nor[mag <= 1e-9] = [0, 1, 0]
        att = {
            "POSITION": access(pos, 5126, "VEC3", bound=True),
            "NORMAL": access(nor, 5126, "VEC3"),
            "COLOR_0": access(np.array(g.c, "u1"), 5121, "VEC4", norm=True),
        }
        d["meshes"][n["mesh"]]["primitives"].append(
            dict(
                attributes=att,
                indices=access(faces.ravel(), 5123, "SCALAR", index=True),
                material=1,
            )
        )
        count += len(faces)
    d["buffers"][0]["byteLength"] = len(buf)
    save(d, buf, glb)
    note["meshed_tidecast_triangles"] = count
    return note
