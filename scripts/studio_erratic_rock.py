"""Native deposit variant for an artist's glacial-boulder workbench."""

from pathlib import Path
import json
import numpy as np
from scipy.spatial import cKDTree
from finished_island_glb import parse, save, values
from island_fault_scarp import channels
from island_erratic_glb import streams
from island_erratic_blocks import clusters, abraded, worn_sole


def carve(
    source, destination, *, crystalline=False, crown_rind=False, rolled_toe=False
):
    source = Path(source)
    dest = Path(destination)
    base, body = parse(source)
    name = base["meshes"][0]["name"].split("_L")[0]
    if name not in ("BoulderA", "BoulderB", "BoulderC"):
        raise ValueError("Use an original native erratic asset.")
    p0 = base["meshes"][0]["primitives"][0]
    pos = np.array(values(base, body, p0["attributes"]["POSITION"]))
    col = np.array(values(base, body, p0["attributes"]["COLOR_0"]))
    tree = cKDTree(pos)
    original_faces = np.array(values(base, body, p0["indices"])).reshape(-1, 3)
    stones = clusters(pos, original_faces)
    out = {**base, "accessors": [], "bufferViews": [], "meshes": []}
    if crystalline:
        from copy import deepcopy

        # Reserve the old mineral material AND charts for the unaltered far.
        # Native scree and cliffs in this scenery never get this new surface.
        images = list(out["images"])
        maps = {i: len(images) + k for k, i in enumerate((0, 1, 2))}
        images.extend(
            {"uri": "erratic/" + v + ".png"}
            for v in ("granite", "granite-normal", "granite-orm")
        )
        textures = list(out["textures"])
        offset = len(textures)
        textures.extend({**t, "source": maps[t["source"]]} for t in base["textures"])
        material = deepcopy(out["materials"][0])
        material["name"] = "Glacial granodiorite: plagioclase, biotite and quartz"
        for ref in (
            material["normalTexture"],
            material["occlusionTexture"],
            material["pbrMetallicRoughness"]["baseColorTexture"],
            material["pbrMetallicRoughness"]["metallicRoughnessTexture"],
        ):
            ref["index"] += offset
        out.update(
            images=images, textures=textures, materials=[*out["materials"], material]
        )
    buf = bytearray()

    a = streams(out, buf)

    budgets = {}
    for lod in range(3):
        if lod == 2:
            # Preserve the old cheap silhouette at far range for the first
            # art study; do not imply that a static plate proved its handoff.
            base_prim = base["meshes"][lod]["primitives"][0]
            attrs = {
                k: np.array(values(base, body, base_prim["attributes"][k]))
                for k in ("POSITION", "NORMAL", "TEXCOORD_0", "COLOR_0")
            }
            faces = np.asarray(values(base, body, base_prim["indices"])).ravel()
            P, N, U, C = (
                attrs["POSITION"],
                attrs["NORMAL"],
                attrs["TEXCOORD_0"],
                np.uint8(np.rint(255 * attrs["COLOR_0"])),
            )
            indices = faces.astype("<u2")
        else:
            atlas = []
            ss = []
            ps = []
            fs = []
            total = 0
            for k, stone in enumerate(stones):
                count = (65, 49, 38)[min(k, 2)] if lod == 0 else (27, 19, 11)[min(k, 2)]
                q, t, norm = abraded(stone, count, ord(name[-1]) - 62 + k * 4)
                if rolled_toe:
                    q, norm = worn_sole(q, norm)
                if crown_rind:
                    from island_erratic_rind import project

                    atlas.extend(project(q, t, 0.91 * (ord(name[-1]) - 64) + 2.14 * k))
                ss.extend(norm)
                ps.extend(q)
                fs.extend(t + total)
                total += len(q)
            points, faces = np.asarray(ps), np.asarray(fs)
            N, U, C = channels(points, faces, tree, col)
            if crown_rind:
                U = np.asarray(atlas)
            P = points[faces].reshape(-1, 3)
            N = np.asarray(ss)[faces].reshape(-1, 3)
            # Share exact PBR corners where UV projection and shading agree.
            # Keep UV seams separate; the renderer uses a single primitive.
            rows = np.column_stack((P, N, U, C)).astype("<f4")
            _, first, indices = np.unique(
                rows, axis=0, return_index=True, return_inverse=True
            )
            P, N, U, C = P[first], N[first], U[first], C[first]
            indices = indices.astype("<u2")
        ad = {
            "POSITION": a(P.astype("<f4"), 5126, "VEC3", bounds=True),
            "NORMAL": a(N.astype("<f4"), 5126, "VEC3"),
            "TEXCOORD_0": a(U.astype("<f4"), 5126, "VEC2"),
            "COLOR_0": a(C, 5121, "VEC4", normalized=True),
        }
        prim = dict(
            attributes=ad,
            indices=a(indices, 5123, "SCALAR", indices=True),
            material=(1 if crystalline and lod < 2 else 0),
        )
        key = f"{name}_L{lod}"
        out["meshes"].append(dict(name=key, primitives=[prim]))
        budgets[key] = len(indices) // 3
    out["buffers"] = [{"byteLength": len(buf)}]
    dest.parent.mkdir(parents=True, exist_ok=True)
    save(out, buf, dest)
    return {
        "original_cluster_count": len(stones),
        "new_cluster_count": len(stones),
        "budgets": budgets,
        "fallback_far": "original native overview",
        "source": (
            "individual native component bounds; worn inward sole"
            if rolled_toe
            else "individual native component bounds and original contiguous foot"
        ),
    }
