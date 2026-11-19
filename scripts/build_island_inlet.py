"""Fit the southern bank artwork to the literal, shared native terrain triangles.

There is no second height field: triangles are included only if the alternative
ridge fixture has exactly the same four corner samples. A six-millimetre offset
is a render separation, not collision relief and not a new run of ledges.
"""

from pathlib import Path
import json, math
from collections import defaultdict
import numpy as np
from PIL import Image
from finished_island_glb import save
from island_inlet_ground import Inlet, X0, X1, Z0, Z1, paint

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "crates/alife_world/assets/island-v1.bin"
V2 = ROOT / "studies/island-ridge-v2/island-ridge-v2.bin"
TILE = 50


def export(dest, source=SOURCE, variant=V2, rooted=False):
    dest = Path(dest)
    panel = dest / "inlet"
    note = paint(source, panel)
    base = Inlet(source)
    alternate = Inlet(variant)
    alpha = np.array(Image.open(panel / "inlet-bed.png").getchannel("A"))
    H, W = alpha.shape
    gx = base.x0 + base.s * np.arange(base.h.shape[1])
    gz = base.z0 + base.s * np.arange(base.h.shape[0])
    gridz, gridx = np.gradient(base.h, base.s)
    tiles = defaultdict(list)
    skip = 0
    for j in np.where((gz[:-1] >= Z0 - 1e-4) & (gz[1:] <= Z1 + 1e-4))[0]:
        for i in np.where((gx[:-1] >= X0 - 1e-4) & (gx[1:] <= X1 + 1e-4))[0]:
            u0 = math.floor((gx[i] - X0) * W / (X1 - X0))
            u1 = math.ceil((gx[i + 1] - X0) * W / (X1 - X0))
            v0 = math.floor((gz[j] - Z0) * H / (Z1 - Z0))
            v1 = math.ceil((gz[j + 1] - Z0) * H / (Z1 - Z0))
            if (
                np.max(alpha[max(0, v0) : min(H, v1 + 1), max(0, u0) : min(W, u1 + 1)])
                < 82
            ):
                continue
            if not np.array_equal(
                base.h[j : j + 2, i : i + 2], alternate.h[j : j + 2, i : i + 2]
            ):
                skip += 1
                continue
            # Same A--C diagonal and original vertex ordering as the real cell.
            tx = math.floor((gx[i] + base.s * 0.5) / TILE)
            tz = math.floor((gz[j] + base.s * 0.5) / TILE)
            tiles[tx, tz].append((i, j))
    doc = {
        "asset": {
            "version": "2.0",
            "generator": "Qualia southern cut bank: original shared triangles",
        },
        "scene": 0,
        "scenes": [{"nodes": []}],
        "nodes": [],
        "meshes": [],
        "accessors": [],
        "bufferViews": [],
        "images": [
            {"uri": "inlet/" + n}
            for n in ["inlet-bed.png", "inlet-normal.png", "inlet-orm.png"]
        ],
        "textures": [{"sampler": 0, "source": i} for i in range(3)],
        "samplers": [
            {"magFilter": 9729, "minFilter": 9987, "wrapS": 33071, "wrapT": 33071}
        ],
        "materials": [
            {
                "name": "Wind-cut turf, silt and bank rubble",
                "doubleSided": False,
                "alphaMode": "MASK",
                "alphaCutoff": 0.43,
                "normalTexture": {"index": 1},
                "occlusionTexture": {"index": 2, "strength": 0.71},
                "pbrMetallicRoughness": {
                    "baseColorTexture": {"index": 0},
                    "metallicRoughnessTexture": {"index": 2},
                    "roughnessFactor": 1,
                    "metallicFactor": 0,
                },
            }
        ],
    }
    plants = {}
    if rooted:
        from island_inlet_sod import fringe

        plants, botany = fringe(base, alternate)
        note.update(botany)
        for key in plants:
            tiles.setdefault(key, [])
        doc["materials"].append(
            dict(
                name="Cut-edge turf and embedded shingle",
                doubleSided=True,
                pbrMetallicRoughness=dict(roughnessFactor=0.87, metallicFactor=0.0),
            )
        )
    blob = bytearray()

    def accessor(a, component, kind, index=False, bounds=False, normalized=False):
        blob.extend(b"\0" * (-len(blob) % 4))
        at = len(blob)
        blob.extend(a.tobytes())
        ix = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=at,
                byteLength=a.nbytes,
                target=34963 if index else 34962,
            )
        )
        rec = dict(bufferView=ix, componentType=component, count=len(a), type=kind)
        if normalized:
            rec["normalized"] = True
        if bounds:
            rec.update(min=a.min(axis=0).tolist(), max=a.max(axis=0).tolist())
        key = len(doc["accessors"])
        doc["accessors"].append(rec)
        return key

    for (tx, tz), cells in sorted(tiles.items()):
        ox, oz = (tx + 0.5) * TILE, (tz + 0.5) * TILE
        points = []
        norm = []
        uv = []
        indices = []
        used = {}

        def vertex(i, j):
            key = (i, j)
            if key in used:
                return used[key]
            n = np.array([-gridx[j, i], 1, -gridz[j, i]], dtype=float)
            n /= np.linalg.norm(n)
            k = len(points)
            used[key] = k
            points.append((gx[i] - ox, float(base.h[j, i]) + 0.006, gz[j] - oz))
            norm.append(n)
            uv.append(((gx[i] - X0) / (X1 - X0), (gz[j] - Z0) / (Z1 - Z0)))
            return k

        for i, j in cells:
            a, b, c, d = (
                vertex(i, j),
                vertex(i + 1, j),
                vertex(i + 1, j + 1),
                vertex(i, j + 1),
            )
            indices.extend([a, c, b, a, d, c])
        primitives = []
        if points:
            attr = {
                "POSITION": accessor(
                    np.asarray(points, "<f4"), 5126, "VEC3", bounds=True
                ),
                "NORMAL": accessor(np.asarray(norm, "<f4"), 5126, "VEC3"),
                "TEXCOORD_0": accessor(np.asarray(uv, "<f4"), 5126, "VEC2"),
            }
            ix = accessor(np.asarray(indices, "<u2"), 5123, "SCALAR", index=True)
            primitives.append(dict(attributes=attr, indices=ix, material=0))
        tuft = plants.get((tx, tz))
        if tuft is not None and tuft.tri:
            v = np.asarray(tuft.v, dtype="<f4") - np.array([ox, 0, oz], dtype="<f4")
            faces = np.asarray(tuft.tri, "<i4")
            perface = np.cross(
                v[faces[:, 1]] - v[faces[:, 0]], v[faces[:, 2]] - v[faces[:, 0]]
            )
            normals = np.zeros_like(v)
            for i in range(3):
                np.add.at(normals, faces[:, i], perface)
            ln = np.linalg.norm(normals, axis=1)
            normals[ln > 1e-10] /= ln[ln > 1e-10, None]
            normals[ln <= 1e-10] = (0, 1, 0)
            attr = {
                "POSITION": accessor(v, 5126, "VEC3", bounds=True),
                "NORMAL": accessor(normals, 5126, "VEC3"),
                "COLOR_0": accessor(
                    np.asarray(tuft.c, "u1"), 5121, "VEC4", normalized=True
                ),
            }
            wide = len(v) > 65535
            ix = accessor(
                faces.ravel().astype("<u4" if wide else "<u2"),
                5125 if wide else 5123,
                "SCALAR",
                index=True,
            )
            primitives.append(dict(attributes=attr, indices=ix, material=1))
        name = f"Inlet_{tx}_{tz}"
        k = len(doc["nodes"])
        doc["meshes"].append(dict(name=name, primitives=primitives))
        doc["nodes"].append(dict(name=name, mesh=k, translation=[ox, 0, oz]))
        doc["scenes"][0]["nodes"].append(k)
    doc["buffers"] = [dict(byteLength=len(blob))]
    asset = dest / "inlet-bank.glb"
    save(doc, blob, asset)
    note.update(
        near_tiles=len(tiles),
        fitted_terrain_triangles=2 * sum(map(len, tiles.values())),
        root_and_embedded_gravel_triangles=sum(len(p.tri) for p in plants.values()),
        omitted_changed_cells=skip,
        render_separation_mm=6,
    )
    (dest / "inlet-painting.json").write_text(json.dumps(note, indent=2) + "\n")
    return asset
