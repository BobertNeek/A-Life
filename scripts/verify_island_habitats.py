"""Independently compare final triangles, positions and smooth normals to the source GLB."""

import argparse, json, hashlib
from pathlib import Path
import numpy as np
from PIL import Image
from build_island_geology import Glb, SURFACE, TERRAIN
from island_habitat_field import habitat


def verify(source, pack, surface, placements, rock_ground=False):
    _, _, _, cover = habitat(surface, placements, rock_ground)
    correction = None
    if rock_ground:
        from island_geology_paint import Island
        from island_exposed_beds import CornerCorrection

        correction = CornerCorrection(
            Island(surface, fractured_rock=True),
            source.parent / "geology/close-pigment.png",
        )
        assert np.any(correction.delta) and np.any(
            np.all(correction.delta == 0, axis=-1)
        )
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
        albedo = old_rgb[:, :3]
        if correction is not None:
            delta = correction.at(xyz[:, 0], xyz[:, 2])
            albedo = np.clip(albedo.astype("i2") + delta, 0, 255).astype("u1")
        expected = albedo * factor
        if cover.deposit is not None:
            from island_habitat_massing import linear

            w, warm = cover.deposit.at(xyz[:, 0], xyz[:, 2])
            # Independent reconstruction, double checks source-limited effects:
            # exact prior colour anywhere outside a source apron.
            assert np.all((0 <= warm) & (warm <= w + 1e-6)) and w.max() <= 0.90
            mineral = w[:, None] * np.array([0.217, 0.258, 0.274]) + warm[
                :, None
            ] * np.array([0.148, 0.020, -0.077])
            expected = expected * (1 - w[:, None]) + 255 * mineral / linear(220 / 255)
        assert np.array_equal(np.clip(np.rint(expected), 0, 255), new_rgb[:, :3])
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
    cli.add_argument("--rock-ground", action="store_true")
    args = cli.parse_args()
    print(
        json.dumps(
            verify(
                args.source, args.pack, args.surface, args.placements, args.rock_ground
            ),
            indent=2,
        )
    )
