"""Bake contact-grounded intertidal drifts from the island's real triangles.
Writes a single, optional near-field decoration GLB. No placement or world data.
"""

import argparse
from collections import Counter
from array import array
import json
import math
from pathlib import Path
import struct
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
SURFACE = ROOT / "crates/alife_world/assets/island-v1.bin"
ASSET = ROOT / "crates/alife_game_app/assets/landscape/island/strandline.glb"
TILE = 50
SEED = 1974682


class Beach:
    def __init__(self, path):
        raw = path.read_bytes()
        assert raw[:8] == b"HLAND001"
        w, d, self.x0, self.z0, self.s, _ = struct.unpack_from("<IIfffI", raw, 8)
        self.h = np.frombuffer(raw, "<f4", w * d, 32).reshape(d, w)

    def surface(self, x, z):
        u, v = (x - self.x0) / self.s, (z - self.z0) / self.s
        i, j = int(u), int(v)
        u, v = u - i, v - j
        a, b, c, d = map(
            float,
            (self.h[j, i], self.h[j, i + 1], self.h[j + 1, i + 1], self.h[j + 1, i]),
        )
        dx, dz = (b - a, c - b) if u >= v else (c - d, d - a)
        return a + dx * u + dz * v, dx / self.s, dz / self.s

    def contour(self, level):
        h = self.h
        lo = np.minimum.reduce((h[:-1, :-1], h[1:, 1:], h[1:, :-1], h[:-1, 1:]))
        hi = np.maximum.reduce((h[:-1, :-1], h[1:, 1:], h[1:, :-1], h[:-1, 1:]))
        for j, i in zip(*np.where((lo <= level) & (hi > level)), strict=True):
            x, z = self.x0 + i * self.s, self.z0 + j * self.s
            if math.hypot(x / 445, z / 535) < 0.68:
                continue  # No sea wrack in the river.
            p = [
                (x, z, h[j, i]),
                (x + self.s, z, h[j, i + 1]),
                (x + self.s, z + self.s, h[j + 1, i + 1]),
                (x, z + self.s, h[j + 1, i]),
            ]
            for corners in [(0, 1, 2), (0, 2, 3)]:
                crossings = []
                for a, b in zip(corners, corners[1:] + corners[:1], strict=True):
                    ax, az, ah = p[a]
                    bx, bz, bh = p[b]
                    if (ah <= level) != (bh <= level):
                        f = (level - ah) / (bh - ah)
                        crossings.append((ax + f * (bx - ax), az + f * (bz - az)))
                if len(crossings) == 2:
                    pair = np.asarray(crossings, float)
                    length = np.linalg.norm(pair[1] - pair[0])
                    if length > 0.025:
                        yield pair[0], pair[1], length


LINEAR = [
    round(255 * ((i / 255 / 12.92) if i < 11 else ((i / 255 + 0.055) / 1.055) ** 2.4))
    for i in range(256)
]


def tint(rgb, mul):
    return [LINEAR[max(0, min(255, round(c * mul)))] for c in rgb] + [255]


class Patch:
    def __init__(self, beach):
        self.beach = beach
        self.vertices = array("f")
        self.colors = array("B")
        self.tris = array("I")
        self.uv = array("f")

    def put(self, x, z, lift, color):
        return self.put_at(x, z, self.beach.surface(x, z)[0] + lift, color)

    def put_at(self, x, z, y, color):
        self.vertices.extend((x, y, z))
        self.colors.extend(color)
        return len(self.vertices) // 3 - 1

    def fan(self, a, b, c):
        self.tris.extend((a, c, b))


def pebble(p, r, x, z, scale, angle):
    # An irregular, partly buried wave-rounded lens, not a noisy sphere.
    c, s = math.cos(angle), math.sin(angle)
    wide = scale * r.uniform(0.55, 0.85)
    lift = min(0.042, scale * r.uniform(0.22, 0.39))
    palette = [
        (102, 109, 106),
        (126, 124, 106),
        (142, 132, 115),
        (111, 126, 122),
        (157, 150, 132),
        (86, 91, 87),
    ]
    stone = palette[r.integers(len(palette))]
    sides = 7 if scale < 0.10 else 9
    phase = r.uniform(-0.22, 0.22)
    rims = r.uniform(0.82, 1.13, sides)
    layers = []
    is_veined = r.random() < 0.13 and scale > 0.105
    for rad, y, light in [(1, -0.007, 0.76), (0.93, 0.29, 1.0), (0.59, 0.82, 1.04)]:
        ring = []
        for i in range(sides):
            a = math.tau * i / sides + phase
            u, v = np.cos(a) * rims[i] * rad * scale, np.sin(a) * rims[i] * rad * wide
            rgb = (171, 162, 138) if is_veined and i in (1, 2) and rad < 1 else stone
            h = y if y < 0 else y * lift
            ring.append(
                p.put(
                    x + u * c - v * s,
                    z + u * s + v * c,
                    h,
                    tint(rgb, light + r.uniform(-0.025, 0.025)),
                )
            )
        layers.append(ring)
    for a, b in zip(layers[:-1], layers[1:], strict=True):
        for i in range(sides):
            k = (i + 1) % sides
            p.fan(a[i], a[k], b[k])
            p.fan(a[i], b[k], b[i])
    top = p.put(x + 0.10 * scale * c, z + 0.10 * scale * s, lift, tint(stone, 1.05))
    for i in range(sides):
        p.fan(layers[-1][i], layers[-1][(i + 1) % sides], top)


def shell(p, r, x, z, angle):
    # Two spreading hinge rays and a scalloped, domed lip; asymmetrical shells
    # wash in open-side down and remain distinct from grey mineral shingle.
    length = r.uniform(0.025, 0.064)
    wide = length * r.uniform(0.74, 1.03)
    s = math.sin(angle)
    c = math.cos(angle)
    base = [(187, 169, 135), (174, 152, 126), (152, 141, 121)][r.integers(3)]
    rows = []
    for radius in (0.08, 0.56, 1.0):
        row = []
        for j in range(9):
            a = -1.0 + j * 0.25
            u = length * radius * math.cos(a)
            v = wide * radius * math.sin(a)
            ridge = (j % 2) * 0.003
            h = (0.005 + length * 0.27 * math.sin(radius * math.pi)) * (
                0.6 + 0.4 * math.cos(a)
            ) + ridge
            row.append(
                p.put(
                    x + c * u - s * v,
                    z + s * u + c * v,
                    h,
                    tint(base, 0.89 + ridge * 38 + radius * 0.06),
                )
            )
        rows.append(row)
    for a, b in zip(rows[:-1], rows[1:], strict=True):
        for j in range(8):
            p.fan(a[j], b[j], b[j + 1])
            p.fan(a[j], b[j + 1], a[j + 1])


def ribbon(p, r, x, z, angle, length, grass=False):
    # Folded three-point cross sections catch a highlight on just one edge.
    amp = r.uniform(-0.75, 0.75)
    width = length * (r.uniform(0.020, 0.032) if grass else r.uniform(0.043, 0.072))
    if grass:
        base = [(130, 145, 91), (154, 145, 90), (108, 131, 98)][r.integers(3)]
    else:
        base = [(91, 85, 50), (112, 91, 55), (95, 99, 56), (126, 105, 65)][
            r.integers(4)
        ]
    c, s = math.cos(angle), math.sin(angle)
    rows = []
    for j in range(6):
        t = j / 5
        u = length * t * (0.54 if grass else 1.0)
        v = length * (
            0.25 * amp * math.sin(t * math.pi) + 0.07 * math.sin(t * math.tau + amp) * t
        )
        w = width * (0.15 + 0.85 * math.sin(t * math.pi) ** 0.62) * (1 - t * 0.72)
        h = (
            length * (1.45 * t - 0.65 * t * t)
            if grass
            else 0.0045 + 0.013 * math.sin(t * math.pi + amp) ** 2
        )
        row = []
        for side in (-1, 0, 1):
            a = v + w * side
            row.append(
                p.put(
                    x + c * u - s * a,
                    z + s * u + c * a,
                    h + (w * 0.21 if side == 0 else 0),
                    tint(base, 0.72 + 0.39 * t + (0.12 if side == 0 else 0)),
                )
            )
        rows.append(row)
    for a, b in zip(rows[:-1], rows[1:], strict=True):
        for j in range(2):
            p.fan(a[j], b[j], b[j + 1])
            p.fan(a[j], b[j + 1], a[j + 1])


def driftfield(x, z):
    # Interpolated aperiodic shelter in metres; gaps matter more than density.
    def value(i, j):
        v = ((i * 0x165667B1) ^ (j * 0x27D4EB2D) ^ SEED) & 0xFFFFFFFF
        v = ((v ^ (v >> 15)) * 0x2C1B3C6D) & 0xFFFFFFFF
        return ((v ^ (v >> 12)) & 65535) / 65535.0

    i, j = math.floor(x / 28), math.floor(z / 28)
    u, v = x / 28 - i, z / 28 - j
    u = u * u * (3 - 2 * u)
    v = v * v * (3 - 2 * v)
    return (1 - v) * (value(i, j) * (1 - u) + value(i + 1, j) * u) + v * (
        value(i, j + 1) * (1 - u) + value(i + 1, j + 1) * u
    )


def deposits(beach):
    # Placement and mesh RNGs are separate: editing one pebble doesn't move a cove.
    for band, level in [("wrack", 0.77), ("reed", 2.1)]:
        rng = np.random.default_rng(SEED + (band == "reed"))
        for start, end, dist in beach.contour(level):
            mid = 0.5 * (start + end)
            h, dx, dz = beach.surface(*mid)
            slope = math.hypot(dx, dz)
            if slope > (0.48 if band == "wrack" else 0.33) or slope < 0.025:
                continue
            shelter = driftfield(*mid)
            rate = max(0, (shelter - (0.35 if band == "wrack" else 0.45))) * (
                0.52 if band == "wrack" else 0.23
            )
            for _ in range(rng.poisson(rate * dist)):
                x, z = start + (end - start) * rng.random()
                angle = math.atan2(-dx, dz)
                yield band, x, z, angle, int(rng.integers(2**31))


def make(beach):
    patches = {}
    counts = Counter()
    anchors = []

    def patch(x, z, mat):
        key = (math.floor(x / TILE), math.floor(z / TILE), mat)
        if key not in patches:
            patches[key] = Patch(beach)
        return patches[key]

    for band, x, z, angle, seed in deposits(beach):
        r = np.random.default_rng(seed)
        c, s = math.cos(angle), math.sin(angle)
        anchors.append((band, x, z))
        if band == "reed":
            for _ in range(r.integers(2, 6)):
                a, b = r.normal(0, 0.5), r.normal(0, 0.22)
                px, pz = x + a * c - b * s, z + a * s + b * c
                p = patch(px, pz, 1)
                counts["tufts"] += 1
                for j in range(r.integers(5, 10)):
                    ribbon(
                        p,
                        r,
                        px + r.normal(0, 0.03),
                        pz + r.normal(0, 0.03),
                        angle + 1.1 + r.normal(0, 0.88),
                        r.uniform(0.12, 0.35),
                        True,
                    )
            continue
        # Swash sorts clasts into pockets. One broad offset per deposit breaks
        # the constant-elevation chain; satellites nest beside an anchor stone.
        delta = (driftfield(x * 0.72 + 21, z * 0.72 - 33) - 0.5) * 3.3
        x, z = x - delta * s, z + delta * c
        seeds = [
            (r.normal(0, 0.77), r.normal(0, 0.41)) for _ in range(r.integers(2, 4))
        ]
        for k in range(r.integers(8, 23)):
            anchor = k < len(seeds)
            a, b = seeds[k % len(seeds)]
            if not anchor:
                a += r.normal(0, 0.22)
                b += r.normal(0, 0.13)
            px, pz = x + a * c - b * s, z + a * s + b * c
            if beach.surface(px, pz)[0] < 0.04:
                continue
            p = patch(px, pz, 0)
            if r.random() < 0.12:
                shell(p, r, px, pz, r.random() * math.tau)
                counts["shells"] += 1
            else:
                pebble(
                    p,
                    r,
                    px,
                    pz,
                    (
                        r.uniform(0.105, 0.165)
                        if anchor
                        else 0.023 + r.random() ** 2 * 0.082
                    ),
                    angle + r.normal(0, 1.0),
                )
                counts["pebbles"] += 1
        if r.random() < 0.66:
            px, pz = x + r.normal(0, 0.23), z + r.normal(0, 0.23)
            p = patch(px, pz, 0)
            counts["wrack"] += 1
            for j in range(r.integers(3, 8)):
                ribbon(
                    p,
                    r,
                    px + r.normal(0, 0.03),
                    pz + r.normal(0, 0.03),
                    angle + r.normal(0, 0.78),
                    r.uniform(0.16, 0.47),
                )
    from island_tidal_film import lay

    lay(beach, patch, driftfield, tint)
    from island_coast_plants import thickets

    (
        counts["thickets"],
        counts["clumps"],
        counts["matrix_tussocks"],
        counts["heath_sprays"],
    ) = thickets(beach, patch, driftfield, tint)
    return {k: p for k, p in patches.items() if len(p.tris)}, counts, anchors


def write(patches, out):
    doc = {
        "asset": {
            "version": "2.0",
            "generator": "Qualia tidal strandline; seeded, source-triangle grounded",
        },
        "scene": 0,
        "scenes": [{"nodes": []}],
        "nodes": [],
        "meshes": [],
        "accessors": [],
        "bufferViews": [],
        "materials": [
            {
                "name": label,
                "doubleSided": True,
                "pbrMetallicRoughness": {
                    "metallicFactor": 0.0,
                    "roughnessFactor": rough,
                },
            }
            for label, rough in [
                ("Tideworn mineral and wrack", 0.56),
                ("Dry salt reed", 0.87),
                ("Damp tidal sand film", 0.24),
                ("Spent edge wash", 0.27),
                ("Windrail thicket", 0.84),
                ("Wind-bent salt sward", 0.9),
            ]
        ],
    }
    for material in doc["materials"][2:4]:
        material["alphaMode"] = "BLEND"
    buf = bytearray()

    def attribute(a, component, shape, target, normalized=False, bounds=False):
        a = np.ascontiguousarray(a)
        buf.extend(b"\0" * (-len(buf) % 4))
        off = len(buf)
        buf.extend(a.tobytes())
        view = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(buffer=0, byteOffset=off, byteLength=a.nbytes, target=target)
        )
        acc = dict(bufferView=view, componentType=component, type=shape, count=len(a))
        if normalized:
            acc["normalized"] = True
        if bounds:
            acc.update(min=a.min(0).tolist(), max=a.max(0).tolist())
        doc["accessors"].append(acc)
        return len(doc["accessors"]) - 1

    # Alpha variation is real metre-scaled cellular lace. A single repeating
    # water material keeps the vertex-painted shallows and only breaks their wash.
    from island_tidal_film import wash_lace

    image = wash_lace()
    doc["bufferViews"].append(
        dict(buffer=0, byteOffset=len(buf), byteLength=len(image))
    )
    buf.extend(image)
    doc["images"] = [dict(bufferView=0, mimeType="image/png", name="Tidal wash lace")]
    doc["samplers"] = [dict(magFilter=9729, minFilter=9987, wrapS=10497, wrapT=10497)]
    doc["textures"] = [dict(source=0, sampler=0)]
    doc["materials"][3]["pbrMetallicRoughness"]["baseColorTexture"] = {"index": 0}

    from island_salt_sward import brush

    binary = brush()
    buf.extend(b"\0" * (-len(buf) % 4))
    view = len(doc["bufferViews"])
    doc["bufferViews"].append(
        dict(buffer=0, byteOffset=len(buf), byteLength=len(binary))
    )
    buf.extend(binary)
    doc["images"].append(
        dict(bufferView=view, mimeType="image/png", name="Salt meadow sheaves")
    )
    doc["samplers"].append(
        dict(magFilter=9729, minFilter=9987, wrapS=33071, wrapT=33071)
    )
    doc["textures"].append(dict(source=1, sampler=1))
    doc["materials"][5]["pbrMetallicRoughness"]["baseColorTexture"] = {"index": 1}
    doc["materials"][5].update(alphaMode="MASK", alphaCutoff=0.46)
    for (cx, cz, mat), p in sorted(patches.items()):
        pos = np.frombuffer(p.vertices, "f").reshape(-1, 3).copy()
        colors = np.frombuffer(p.colors, np.uint8).reshape(-1, 4)
        idx = np.frombuffer(p.tris, "I").reshape(-1, 3)
        assert idx.max() < 65535
        offset = np.array(((cx + 0.5) * TILE, 0.0, (cz + 0.5) * TILE), np.float32)
        pos -= offset
        n = np.zeros_like(pos)
        a, b, c = idx.T
        fn = np.cross(pos[b] - pos[a], pos[c] - pos[a])
        for points in (a, b, c):
            np.add.at(n, points, fn)
        long = np.linalg.norm(n, axis=1)
        bad = long < 1e-9
        n[bad] = [0, 1, 0]
        long[bad] = 1.0
        n /= long[:, None]
        attrib = {
            "POSITION": attribute(pos, 5126, "VEC3", 34962, bounds=True),
            "NORMAL": attribute(n, 5126, "VEC3", 34962),
            "COLOR_0": attribute(colors, 5121, "VEC4", 34962, True),
        }
        if mat == 3:
            world = pos + offset
            u = 0.86 * world[:, 0] + 0.51 * world[:, 2]
            v = -0.51 * world[:, 0] + 0.86 * world[:, 2]
            uv = np.column_stack(
                (u / 2.3 + 0.09 * np.sin(v * 0.31), v / 1.9 + 0.08 * np.sin(u * 0.34))
            ).astype("<f4")
            attrib["TEXCOORD_0"] = attribute(uv, 5126, "VEC2", 34962)
        if mat == 5:
            assert len(p.uv) == 2 * len(pos)
            attrib["TEXCOORD_0"] = attribute(
                np.frombuffer(p.uv, "f").reshape(-1, 2), 5126, "VEC2", 34962
            )
        ix = attribute(idx.ravel().astype("<u2"), 5123, "SCALAR", 34963)
        name = (
            f'Strand_{cx}_{cz}_{("wet","reed","film","wash","headland","sward")[mat]}'
        )
        i = len(doc["meshes"])
        doc["meshes"].append(
            {
                "name": name,
                "primitives": [dict(attributes=attrib, indices=ix, material=mat)],
            }
        )
        doc["nodes"].append({"name": name, "mesh": i, "translation": offset.tolist()})
        doc["scenes"][0]["nodes"].append(i)
    doc["buffers"] = [dict(byteLength=len(buf))]
    raw = json.dumps(doc, separators=(",", ":")).encode()
    raw += b" " * (-len(raw) % 4)
    buf += b"\0" * (-len(buf) % 4)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("wb") as f:
        f.write(
            struct.pack(
                "<4sII I4s", b"glTF", 2, 28 + len(raw) + len(buf), len(raw), b"JSON"
            )
        )
        f.write(raw)
        f.write(struct.pack("<I4s", len(buf), b"BIN\0"))
        f.write(buf)


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, default=ASSET)
    ap.add_argument("--surface", type=Path, default=SURFACE)
    ap.add_argument(
        "--anchors",
        type=Path,
        help="Optional camera-scout receipt; not a shipping placement file.",
    )
    args = ap.parse_args()
    patches, counts, anchors = make(Beach(args.surface))
    write(patches, args.out)
    if args.anchors:
        args.anchors.write_text(json.dumps(anchors))
    print(
        dict(counts),
        len(patches),
        "bounded patches",
        sum(len(p.tris) // 3 for p in patches.values()),
        "triangles",
        args.out.stat().st_size,
        "bytes",
    )
