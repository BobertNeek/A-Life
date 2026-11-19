"""Refresh only presentation pigment and distant LODs in the production island GLB.

Run after rebuilding the Blender base terrain. Accepts its own output as input.
Dependencies: NumPy, SciPy and Pillow. Never regenerates the world's heightfield.
"""

import argparse
import json
from pathlib import Path
import struct

import numpy as np

from island_geology_lod import chunk, leaves
from island_geology_paint import Island, ground_micro

ROOT = Path(__file__).resolve().parents[1]
TERRAIN = ROOT / "crates/alife_game_app/assets/landscape/island/terrain-chunks.glb"
SURFACE = ROOT / "crates/alife_world/assets/island-v1.bin"
TYPES = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}
FORMATS = {5126: "<f4", 5123: "<u2", 5121: "u1", 5125: "<u4"}


class Glb:
    def __init__(self, path):
        data = path.read_bytes()
        if data[:4] != b"glTF" or struct.unpack_from("<I", data, 4)[0] != 2:
            raise ValueError("Expected glTF 2 binary terrain")
        n = struct.unpack_from("<I", data, 12)[0]
        self.doc = json.loads(data[20 : 20 + n])
        self.bin = data[28 + n :]

    def view(self, i):
        view = self.doc["bufferViews"][i]
        off = view.get("byteOffset", 0)
        if "byteStride" in view:
            raise ValueError(
                "This bake requires the island's tightly packed source streams"
            )
        return self.bin[off : off + view["byteLength"]]

    def attr(self, i):
        a = self.doc["accessors"][i]
        return np.frombuffer(
            self.view(a["bufferView"]),
            FORMATS[a["componentType"]],
            a["count"] * TYPES[a["type"]],
            a.get("byteOffset", 0),
        ).reshape(a["count"], -1)


def build(source, destination):
    terrain, src = Island(SURFACE), Glb(source)
    doc = src.doc.copy()
    doc.update(accessors=[], bufferViews=[], images=[])
    buf = bytearray()
    cache = {}

    def view(data, target=None):
        buf.extend(b"\0" * (-len(buf) % 4))
        v = dict(buffer=0, byteOffset=len(buf), byteLength=len(data))
        if target:
            v["target"] = target
        doc["bufferViews"].append(v)
        buf.extend(data)
        return len(doc["bufferViews"]) - 1

    def attr(data, kind, bounds=False):
        array = np.ascontiguousarray(data)
        shape = (
            "SCALAR"
            if array.ndim == 1 or array.shape[1] == 1
            else f"VEC{array.shape[1]}"
        )
        target = 34963 if shape == "SCALAR" else 34962
        raw = array.tobytes()
        # Many small coast/ocean blocks intentionally share identical attribute streams.
        key = (shape, kind, raw, bounds)
        if key not in cache:
            a = {
                "bufferView": view(raw, target),
                "componentType": kind,
                "count": len(array),
                "type": shape,
            }
            if bounds:
                a.update(min=array.min(axis=0).tolist(), max=array.max(axis=0).tolist())
            cache[key] = len(doc["accessors"])
            doc["accessors"].append(a)
        return cache[key]

    albedo, relief = terrain.atlas()
    # Keep the two authored macro maps available to Blender and other material
    # editors. A paint-only revision need not duplicate the entire geometry blob.
    art = destination.parent / "geology"
    art.mkdir(parents=True, exist_ok=True)
    (art / "relief.png").write_bytes(relief)
    (art / "pigment.png").write_bytes(albedo)
    doc["images"] = [
        {
            "uri": "geology/relief.png",
            "mimeType": "image/png",
            "name": "Island geological relief",
        },
        {
            "uri": "geology/pigment.png",
            "mimeType": "image/png",
            "name": "Island salt and stone",
        },
        {
            "bufferView": view(ground_micro()),
            "mimeType": "image/png",
            "name": "Ground pore roughness and cavity",
        },
    ]
    doc["textures"] = [{"source": i, "sampler": int(i == 2)} for i in range(3)]
    doc["samplers"] = [
        {"magFilter": 9729, "minFilter": 9987, "wrapS": 33071, "wrapT": 33071},
        {"magFilter": 9729, "minFilter": 9987},
    ]
    doc["materials"] = [
        {
            "name": "Salt meadow, tidecut stone and alpine mantle",
            "doubleSided": True,
            "normalTexture": {"index": 0},
            "occlusionTexture": {"index": 2, "texCoord": 1, "strength": 0.8},
            "pbrMetallicRoughness": {
                "baseColorTexture": {"index": 1},
                "metallicFactor": 0,
                "roughnessFactor": 1.0,
                "metallicRoughnessTexture": {"index": 2, "texCoord": 1},
            },
        }
    ]
    geometry = {1: leaves(terrain, 2, 0.12, 0.76), 2: leaves(terrain, 4, 0.33, 2.4)}
    near = {}
    meshes = []
    counts = [0, 0, 0]
    for mesh in src.doc["meshes"]:
        assert len(mesh["primitives"]) == 1
        _, cx, cz, level = mesh["name"].split("_")
        x0, z0, lod = int(cx) * 32, int(cz) * 32, int(level[1:])
        prim = mesh["primitives"][0]
        original = prim["attributes"]
        if lod == 0:
            pos = src.attr(original["POSITION"])
            gx = np.rint((pos[:, 0] - terrain.x) / terrain.spacing).astype(int)
            gz = np.rint((pos[:, 2] - terrain.z) / terrain.spacing).astype(int)
            skirt = pos[:, 1] < terrain.h[gz, gx] - 10
            near[x0, z0] = (
                original,
                {
                    (i, j, s): k
                    for k, (i, j, s) in enumerate(zip(gx, gz, skirt, strict=True))
                },
            )
        if lod == 0:
            indices = src.attr(prim["indices"]).ravel()
            attrs = {name: src.attr(original[name]) for name in ("POSITION", "NORMAL")}
        else:
            keys, indices = chunk(x0, z0, geometry[lod])
            values, lookup = near[x0, z0]
            pick = [lookup[key] for key in keys]
            attrs = {
                name: src.attr(values[name])[pick] for name in ("POSITION", "NORMAL")
            }
        p = attrs["POSITION"]
        attrs["TEXCOORD_0"] = np.stack(
            (
                (p[:, 0] - terrain.x) / terrain.size[0],
                (p[:, 2] - terrain.z) / terrain.size[1],
            ),
            axis=1,
        )
        attrs["TEXCOORD_1"] = p[:, (0, 2)] / 3.7
        attributes = {
            k: attr(v.astype("<f4", copy=False), 5126, bounds=k == "POSITION")
            for k, v in attrs.items()
        }
        ix = attr(indices.astype("<u2", copy=False), 5123)
        counts[lod] += len(indices) // 3
        meshes.append(
            {
                "name": mesh["name"],
                "primitives": [
                    {"attributes": attributes, "indices": ix, "material": 0}
                ],
            }
        )
    doc["meshes"] = meshes
    doc["asset"] = {
        "version": "2.0",
        "generator": "Qualia island: Blender source and offline geology finish",
    }
    doc["buffers"] = [{"byteLength": len(buf)}]
    raw = json.dumps(doc, separators=(",", ":")).encode()
    raw += b" " * (-len(raw) % 4)
    buf += b"\0" * (-len(buf) % 4)
    blob = (
        struct.pack("<4sII", b"glTF", 2, 28 + len(raw) + len(buf))
        + struct.pack("<I4s", len(raw), b"JSON")
        + raw
        + struct.pack("<I4s", len(buf), b"BIN\x00")
        + buf
    )
    destination.write_bytes(blob)
    return counts


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--source", type=Path, default=TERRAIN)
    cli.add_argument("--out", type=Path, default=TERRAIN)
    args = cli.parse_args()
    print("Triangles by LOD:", build(args.source, args.out))
