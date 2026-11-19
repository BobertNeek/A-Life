"""Native deposit variant for an artist's glacial-boulder workbench."""

from pathlib import Path
import json
import numpy as np
from scipy.spatial import cKDTree
from finished_island_glb import parse, save, values
from island_fault_scarp import channels
from island_erratic_blocks import clusters, block


def carve(source, destination):
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
    buf = bytearray()

    def a(stream, ct, kind, idx=False, bound=False, norm=False):
        stream = np.ascontiguousarray(stream)
        buf.extend(b"\0" * (-len(buf) % 4))
        off = len(buf)
        buf.extend(stream.tobytes())
        vi = len(out["bufferViews"])
        out["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=off,
                byteLength=stream.nbytes,
                target=34963 if idx else 34962,
            )
        )
        accessor = dict(bufferView=vi, componentType=ct, count=len(stream), type=kind)
        if bound:
            accessor.update(min=stream.min(0).tolist(), max=stream.max(0).tolist())
        if norm:
            accessor["normalized"] = True
        ix = len(out["accessors"])
        out["accessors"].append(accessor)
        return ix

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
            ps = []
            fs = []
            total = 0
            for k, stone in enumerate(stones):
                count = (65, 49, 38)[min(k, 2)] if lod == 0 else (27, 19, 11)[min(k, 2)]
                q, t = block(stone, count, ord(name[-1]) - 62 + k * 4)
                ps.extend(q)
                fs.extend(t + total)
                total += len(q)
            points, faces = np.asarray(ps), np.asarray(fs)
            N, U, C = channels(points, faces, tree, col)
            P = points[faces].reshape(-1, 3)
            indices = np.arange(len(P), dtype="<u2")
        ad = {
            "POSITION": a(P.astype("<f4"), 5126, "VEC3", bound=True),
            "NORMAL": a(N.astype("<f4"), 5126, "VEC3"),
            "TEXCOORD_0": a(U.astype("<f4"), 5126, "VEC2"),
            "COLOR_0": a(C, 5121, "VEC4", norm=True),
        }
        prim = dict(
            attributes=ad, indices=a(indices, 5123, "SCALAR", idx=True), material=0
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
        "source": "individual convex bounds and actual preserved upper/foot supports, original UV-scale/rock pigments",
    }
