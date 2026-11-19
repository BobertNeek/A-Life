"""Build close PBR substrates on the original indexed triangles, without changing terrain.
One standard glTF primitive per dry near chunk. Extra vertices only express atlas seams.
"""

import argparse
import json
import math
from pathlib import Path
import struct
import numpy as np

from build_island_geology import Glb, SURFACE, TERRAIN
from island_habitat_field import (
    PATCH_CELLS,
    canonical,
    habitat,
    orient_uv,
    signature,
    signatures,
)
from island_habitat_tiles import GUTTER, N, bake
from island_habitat_massing import linear, srgb
from island_geology_paint import Island
from PIL import Image
import io


def build(terrain, surface, placements, out, rock_ground=False):
    h, labels, (x0, z0, step), cover = habitat(surface, placements, rock_ground)
    inventory = signatures(labels)
    variants = {key: canonical(key) for key in inventory}
    keys = sorted({v[0] for v in variants.values()})
    cols, rows = bake(keys, out / "habitats")
    # Synthesize from the ungraded terrain recipe, not the last saved PNG; a
    # rebuild of the weekly checkout must never apply the understory grade twice.
    raw, _ = Island(surface).atlas()
    pigment = np.array(Image.open(io.BytesIO(raw)).convert("RGB"))
    atlas_height, atlas_width = pigment.shape[:2]
    xx = (
        x0
        + (np.arange(atlas_width)[None, :] + 0.5)
        * step
        * (h.shape[1] - 1)
        / atlas_width
    )
    for j in range(0, atlas_height, 48):
        rr = np.arange(j, min(j + 48, atlas_height))
        zz = z0 + (rr[:, None] + 0.5) * step * (h.shape[0] - 1) / atlas_height
        graded = linear(pigment[rr] / 255.0) * cover.multiplier(xx, zz)
        if cover.deposit is not None:
            graded = cover.deposit.mix(graded, xx, zz)
        corrected = srgb(graded)
        pigment[rr] = np.clip(np.rint(corrected * 255), 0, 255).astype("u1")
    (out / "geology").mkdir(exist_ok=True)
    Image.fromarray(pigment).save(out / "geology/pigment.png", compress_level=8)
    slots = {key: i for i, key in enumerate(keys)}
    size = N + GUTTER * 2
    src = Glb(terrain)
    doc = {
        "asset": {"version": "2.0", "generator": "Qualia rooted habitat atlas"},
        "scene": 0,
        "scenes": [{"nodes": []}],
        "nodes": [],
        "meshes": [],
        "accessors": [],
        "bufferViews": [],
        "images": [
            {"uri": "habitats/" + name}
            for name in (
                "substrates.png",
                "substrates-normal.png",
                "substrates-orm.png",
            )
        ],
        "samplers": [
            {"magFilter": 9729, "minFilter": 9987, "wrapS": 33071, "wrapT": 33071}
        ],
        "textures": [{"sampler": 0, "source": i} for i in range(3)],
        "materials": [
            {
                "name": "Salt turf, wind sand, talus and grove loam",
                "doubleSided": True,
                "normalTexture": {"index": 1},
                "occlusionTexture": {"index": 2, "strength": 0.74},
                "pbrMetallicRoughness": {
                    "baseColorTexture": {"index": 0},
                    "metallicRoughnessTexture": {"index": 2},
                    "roughnessFactor": 1,
                    "metallicFactor": 0,
                },
            }
        ],
    }
    data = bytearray()

    def attr(array, component, shape, *, normalized=False, bounds=False):
        a = np.ascontiguousarray(array)
        data.extend(b"\0" * (-len(data) % 4))
        ix = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=len(data),
                byteLength=a.nbytes,
                target=34963 if shape == "SCALAR" else 34962,
            )
        )
        data.extend(a.tobytes())
        record = dict(bufferView=ix, componentType=component, type=shape, count=len(a))
        if normalized:
            record["normalized"] = True
        if bounds:
            record.update(min=a.min(0).tolist(), max=a.max(0).tolist())
        doc["accessors"].append(record)
        return len(doc["accessors"]) - 1

    triangles = vertices = old_vertices = 0
    patch_m = step * PATCH_CELLS
    for m in src.doc["meshes"]:
        _, cx, cz, level = m["name"].split("_")
        cx, cz = int(cx), int(cz)
        if level != "L0":
            continue
        prim = m["primitives"][0]
        ref = prim["attributes"]
        p = src.attr(ref["POSITION"])
        if p[:, 1].max() < -0.15:
            continue
        faces = src.attr(prim["indices"]).reshape(-1, 3)
        centers = p[faces][:, :, [0, 2]].mean(axis=1)
        patch = np.floor((centers - (x0, z0)) / patch_m).astype(np.int32)
        patch = np.clip(patch, (cx * 16, cz * 16), (cx * 16 + 15, cz * 16 + 15))
        spec = np.column_stack((faces.ravel(), np.repeat(patch, 3, axis=0)))
        unique, inv = np.unique(spec, axis=0, return_inverse=True)
        original, ix, iz = unique.T
        assert len(unique) < 65535
        local = (p[original][:, (0, 2)] - (x0, z0)) / patch_m - unique[:, 1:]
        assert local.min() > -0.001 and local.max() < 1.001
        local = np.clip(local, 0, 1)
        uv = np.empty_like(local)
        for tx, tz in np.unique(unique[:, 1:], axis=0):
            key, turn = variants[signature(labels, tx, tz)]
            slot = slots[key]
            mask = (ix == tx) & (iz == tz)
            u, v = orient_uv(local[mask, 0], local[mask, 1], turn)
            uv[mask, 0] = ((slot % cols) * size + GUTTER + 0.5 + u * (N - 1)) / (
                cols * size
            )
            uv[mask, 1] = ((slot // cols) * size + GUTTER + 0.5 + v * (N - 1)) / (
                rows * size
            )
        color = src.attr(ref["COLOR_0"])[original].astype("u1").copy()
        factor = cover.multiplier(p[original, 0], p[original, 2])
        graded = color[:, :3] * factor
        if cover.deposit is not None:
            graded = cover.deposit.mix(
                graded,
                p[original, 0],
                p[original, 2],
                reference=linear(220 / 255),
                units=255,
            )
        color[:, :3] = np.clip(np.rint(graded), 0, 255).astype("u1")
        attrs = {
            "POSITION": attr(p[original].astype("<f4"), 5126, "VEC3", bounds=True),
            "NORMAL": attr(
                src.attr(ref["NORMAL"])[original].astype("<f4"), 5126, "VEC3"
            ),
            "TEXCOORD_0": attr(uv.astype("<f4"), 5126, "VEC2"),
            "COLOR_0": attr(
                color,
                5121,
                "VEC4",
                normalized=True,
            ),
        }
        indices = attr(inv.ravel().astype("<u2"), 5123, "SCALAR")
        name = f"Habitat_{cx}_{cz}"
        idx = len(doc["meshes"])
        doc["meshes"].append(
            dict(
                name=name,
                primitives=[dict(attributes=attrs, indices=indices, material=0)],
            )
        )
        doc["nodes"].append(dict(name=name, mesh=idx))
        doc["scenes"][0]["nodes"].append(idx)
        triangles += len(faces)
        vertices += len(unique)
        old_vertices += len(p)
    doc["buffers"] = [{"byteLength": len(data)}]
    raw = json.dumps(doc, separators=(",", ":")).encode()
    raw += b" " * (-len(raw) % 4)
    data += b"\0" * (-len(data) % 4)
    target = out / "habitat-ground.glb"
    with target.open("wb") as f:
        f.write(
            struct.pack(
                "<4sII I4s", b"glTF", 2, 28 + len(raw) + len(data), len(raw), b"JSON"
            )
        )
        f.write(raw)
        f.write(struct.pack("<I4s", len(data), b"BIN\0"))
        f.write(data)
    receipt = dict(
        chunks=len(doc["meshes"]),
        triangles=triangles,
        new_vertices=vertices,
        old_vertices=old_vertices,
        chart_count=len(keys),
        atlas_px=[cols * size, rows * size],
        file_bytes=target.stat().st_size,
    )
    if cover.deposit is not None:
        receipt["deposits"] = cover.deposit.receipt()
    (out / "habitat-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(receipt, flush=True)


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--terrain", type=Path, default=TERRAIN)
    cli.add_argument("--surface", type=Path, default=SURFACE)
    cli.add_argument("--placements", type=Path, default=TERRAIN.parent / "props.json")
    cli.add_argument("--out", type=Path, required=True)
    cli.add_argument("--rock-ground", action="store_true")
    args = cli.parse_args()
    build(args.terrain, args.surface, args.placements, args.out, args.rock_ground)
