"""Independently compare final triangles, positions and smooth normals to the source GLB."""

import argparse, json, hashlib
from pathlib import Path
import numpy as np
from PIL import Image
from build_island_geology import Glb, SURFACE, TERRAIN
from island_habitat_field import habitat


def verify(source, pack, surface, placements):
    _, _, _, cover = habitat(surface, placements)
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
        for k in ("POSITION", "NORMAL"):
            assert np.array_equal(
                base.attr(old["attributes"][k])[a], study.attr(new["attributes"][k])[b]
            ), (key, k)
        xyz = base.attr(old["attributes"]["POSITION"])[a]
        old_rgb = base.attr(old["attributes"]["COLOR_0"])[a]
        new_rgb = study.attr(new["attributes"]["COLOR_0"])[b]
        # The only non-UV vertex change is a bounded geographic grade. Verify
        # every triangle corner, including skirts, borders and repeated vertices.
        factor = cover.multiplier(xyz[:, 0], xyz[:, 2])
        assert 0.63 - 1e-5 <= factor.min() and factor.max() <= 1.241
        assert np.array_equal(old_rgb[:, 3], new_rgb[:, 3])
        assert np.array_equal(
            np.clip(np.rint(old_rgb[:, :3] * factor), 0, 255), new_rgb[:, :3]
        )
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
        preserves="ordered triangle corners, smooth normals, skirts identical; bounded pigment grade verified at every corner",
        sha256=hashlib.sha256(pack.read_bytes()).hexdigest(),
    )


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("source", type=Path)
    cli.add_argument("pack", type=Path)
    cli.add_argument("--surface", type=Path, default=SURFACE)
    cli.add_argument("--placements", type=Path, default=TERRAIN.parent / "props.json")
    args = cli.parse_args()
    print(
        json.dumps(
            verify(args.source, args.pack, args.surface, args.placements), indent=2
        )
    )
