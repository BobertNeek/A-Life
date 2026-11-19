"""Independently compare final triangles, positions and smooth normals to the source GLB."""

import argparse, json, hashlib
from pathlib import Path
import numpy as np
from PIL import Image
from build_island_geology import Glb


def verify(source, pack):
    base = Glb(source)
    study = Glb(pack)
    original = {m["name"]: m for m in base.doc["meshes"]}
    total = 0
    chunks = 0
    assert len(study.doc["materials"]) == 1 and len(study.doc["textures"]) == 3
    for n, m in zip(study.doc["nodes"], study.doc["meshes"], strict=True):
        assert n["name"] == m["name"] and set(n) == {"name", "mesh"}
        key = m["name"].replace("Habitat", "Island") + "_L0"
        old = original[key]["primitives"][0]
        new = m["primitives"][0]
        a, b = base.attr(old["indices"]).ravel(), study.attr(new["indices"]).ravel()
        assert len(a) == len(b) and len(a) % 3 == 0
        for k in ("POSITION", "NORMAL", "COLOR_0"):
            assert np.array_equal(
                base.attr(old["attributes"][k])[a], study.attr(new["attributes"][k])[b]
            ), (key, k)
        uv = study.attr(new["attributes"]["TEXCOORD_0"])
        assert np.isfinite(uv).all() and 0 < uv.min() < uv.max() < 1
        total += len(b) // 3
        chunks += 1
    for tex in study.doc["images"]:
        f = pack.parent / tex["uri"]
        a = Image.open(f)
        a.verify()
        assert f.stat().st_size < 12_000_000, (f, f.stat().st_size)
    assert pack.stat().st_size < 12_000_000
    return dict(
        checked_chunks=chunks,
        checked_triangles=total,
        preserves="ordered triangle corners, smooth normals, pigment and skirts identical",
        sha256=hashlib.sha256(pack.read_bytes()).hexdigest(),
    )


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("source", type=Path)
    cli.add_argument("pack", type=Path)
    args = cli.parse_args()
    print(json.dumps(verify(args.source, args.pack), indent=2))
