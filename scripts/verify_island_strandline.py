"""Validate the real exported study without sampling the model generator."""

import argparse
import hashlib
import json
import struct
from pathlib import Path
import numpy as np
from finished_island_glb import parse, values

ROOT = Path(__file__).resolve().parents[1]


def data(path):
    b = path.read_bytes()
    w, d, ox, oz, s, _ = struct.unpack_from("<IIfffI", b, 8)
    return np.frombuffer(b, "<f4", w * d, 32).reshape(d, w), ox, oz, s


def heights(field, pos):
    h, ox, oz, s = field
    u, v = (pos[:, 0] - ox) / s, (pos[:, 2] - oz) / s
    x, z = np.floor(u).astype(int), np.floor(v).astype(int)
    u, v = u - x, v - z
    a, b, c, d = h[z, x], h[z, x + 1], h[z + 1, x + 1], h[z + 1, x]
    return np.where(
        v >= u, a * (1 - v) + d * (v - u) + c * u, a * (1 - u) + b * (u - v) + c * v
    )


def verify(path):
    doc, buffer = parse(path)
    original = data(ROOT / "crates/alife_world/assets/island-v1.bin")
    alternate = data(ROOT / "studies/island-ridge-v2/island-ridge-v2.bin")
    assert 0 < len(doc["meshes"]) == len(doc["nodes"]) < 350
    assert len(doc["materials"]) == 6
    assert doc["materials"][5]["alphaMode"] == "MASK"
    assert doc["materials"][3]["pbrMetallicRoughness"]["baseColorTexture"] == {
        "index": 0
    }
    total, maximum = 0, 0
    upper = [0.0] * 6
    patches = [0] * 6
    for node in doc["nodes"]:
        mesh = doc["meshes"][node["mesh"]]
        assert node["name"] == mesh["name"] and len(mesh["primitives"]) == 1
        p = mesh["primitives"][0]
        mat = p["material"]
        assert mat in (0, 1, 2, 3, 4, 5)
        patches[mat] += 1
        a = p["attributes"]
        pos = np.array(values(doc, buffer, a["POSITION"])) + node["translation"]
        normals = np.array(values(doc, buffer, a["NORMAL"]))
        col = np.array(values(doc, buffer, a["COLOR_0"]))
        ix = np.array(values(doc, buffer, p["indices"])).ravel()
        assert np.isfinite(pos).all() and len(pos) == len(normals) == len(col)
        assert np.allclose(np.linalg.norm(normals, axis=1), 1.0, atol=2e-5)
        assert ix.min() >= 0 and ix.max() < len(pos)
        if mat in (0, 1, 4, 5):
            assert (col[:, 3] == 1.0).all()
        else:
            assert (col[:, 3] < 1.0).any()
        assert max(np.ptp(pos[:, 0]), np.ptp(pos[:, 2])) < 54.0
        lift = pos[:, 1] - heights(original, pos)
        if mat < 2:
            assert -0.009 < lift.min() and lift.max() < (0.046 if mat == 0 else 0.29)
        elif mat == 2:
            assert np.max(abs(lift - 0.003)) < 0.00004
        elif mat == 5:
            uv = np.array(values(doc, buffer, a["TEXCOORD_0"]))
            assert 0 < uv.min() < uv.max() < 1
            assert lift.min() >= 0.0008 and lift.max() < 0.62
        elif mat == 4:
            assert lift.min() >= -0.001 and lift.max() < 1.35
        else:
            assert np.max(abs(pos[:, 1] - 0.009)) < 0.000001 and lift.min() > 0
            assert np.isfinite(values(doc, buffer, a["TEXCOORD_0"])).all()
        assert np.array_equal(heights(original, pos), heights(alternate, pos))
        upper[mat] = max(upper[mat], lift.max())
        nt = len(ix) // 3
        total += nt
        maximum = max(maximum, nt)
    assert total < 287_000 and maximum < 18_000 and path.stat().st_size < 11_800_000
    return dict(
        patches_stone_reed_film_wash_thicket_sward=patches,
        triangles=total,
        largest_patch_triangles=maximum,
        maximum_height_above_ground_m=upper,
        bytes=path.stat().st_size,
        sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
        terrain="Every decoration vertex uses the same underlying V1/V2 height",
    )


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("asset", type=Path)
    print(json.dumps(verify(ap.parse_args().asset), indent=2))
