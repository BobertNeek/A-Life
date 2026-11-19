"""Dress the actual five rock GLBs without moving or subdividing a single face.

UVs follow each original block around its own axis. A hard cap is a separate
planar chart. All LODs use the same L0 block origin, circumference and palette.
"""

import argparse
import json
from pathlib import Path
import numpy as np
from scipy.optimize import linear_sum_assignment
from scipy.sparse.csgraph import connected_components
from scipy.sparse import coo_matrix
from finished_island_glb import parse, values, save
from island_geology_paint import grain, ramp
from island_mineral_skin import METRES, skins

KINDS = ("Cliff", "BoulderA", "BoulderB", "BoulderC", "Scree")
ROOT = (
    Path(__file__).resolve().parents[1]
    / "crates/alife_game_app/assets/landscape/island"
)


def shells(pos, tri):
    # The input is split-normal geometry. Merge exact positions in this scratch
    # graph only to recover each separate original block (3 boulders/7 cliffs/
    # 22 scree fragments). Don't weld normals or positions in the shipped mesh.
    xyz, inv = np.unique(np.round(pos, 5), axis=0, return_inverse=True)
    faces = inv[tri]
    v = faces.ravel()
    graph = coo_matrix(
        (np.ones(len(v), "u1"), (v, np.roll(faces, 1, axis=1).ravel())),
        shape=(len(xyz), len(xyz)),
    )
    count, ids = connected_components(graph, directed=False)
    labs = ids[inv]
    ctr = []
    cycles = []
    for i in range(count):
        q = xyz[ids == i]
        ctr.append((q.min(0) + q.max(0)) / 2)
        cycles.append(
            max(1, round(np.pi * (np.ptp(q[:, 0]) + np.ptp(q[:, 2])) / (2 * METRES)))
        )
    return labs, np.array(ctr), np.array(cycles)


def pigment(pos, norm, kind, source):
    x, h, z = pos.T
    s = KINDS.index(kind)
    scale = 1 if kind != "Cliff" else 0.43
    weather = grain(x + z * 0.29, h * 0.81 + z * 0.34, 1.37 / scale, 812 + s)
    vein = grain(
        x + z * 0.43 + 0.29 * weather, h * 0.92 - z * 0.54, 0.74 / scale, 392 + s
    )
    bed = ramp(-0.59, 0.52, 0.73 * weather + 0.39 * vein + 0.18 * norm[:, 1])
    # Silvery blue fresh fractures and warm, partly lichened dry crowns. Only
    # the low rock foot gets a damp rind. Never paint a fixed solar direction.
    cool = np.array((0.159, 0.196, 0.226))
    warm = np.array((0.311, 0.265, 0.201))
    rgb = cool + bed[:, None] * (warm - cool)
    damp = (1 - ramp(0.035, 0.48 if kind != "Cliff" else 1.5, h + 0.066 * vein)) * (
        0.62 + 0.38 * ramp(-0.55, 0.65, weather)
    )
    rgb *= 1 - 0.42 * damp[:, None]
    exposed = ramp(0.44, 0.95, norm[:, 1]) * (
        0.4 + 0.6 * ramp(-0.24, 0.63, weather + 0.28 * vein)
    )
    lichen = exposed * ramp(
        0.15, 0.71, weather + 0.37 * vein + 0.26 * (source[:, 1] > source[:, 0] * 1.25)
    )
    rgb += lichen[:, None] * np.array((0.079, 0.097, 0.008))
    return np.clip(rgb + 0.016 * vein[:, None], 0, 1)


def material():
    return dict(
        name="Island stone: iron rind, quartz and mica",
        doubleSided=True,
        normalTexture=dict(index=1),
        occlusionTexture=dict(index=2, strength=0.74),
        pbrMetallicRoughness=dict(
            baseColorTexture=dict(index=0),
            metallicRoughnessTexture=dict(index=2),
            roughnessFactor=1,
            metallicFactor=0,
        ),
    )


def rebuild(kind, srcpath, target):
    src, buf = parse(srcpath)
    if src.get("textures"):
        raise ValueError(
            "Expected the approved untextured source; point --source at the frozen pre-mineral assets"
        )
    doc = {k: src[k] for k in ("asset", "scene", "scenes", "nodes")}
    doc.update(
        asset=dict(version="2.0", generator="Qualia rooted mineral skin"),
        meshes=[],
        accessors=[],
        bufferViews=[],
        materials=[material()],
        images=[
            dict(uri="minerals/" + n)
            for n in ("fracture.png", "fracture-normal.png", "fracture-orm.png")
        ],
        samplers=[dict(magFilter=9729, minFilter=9987, wrapS=10497, wrapT=10497)],
        textures=[dict(sampler=0, source=i) for i in range(3)],
    )
    data = bytearray()

    def attr(a, dtype, component, kind, *, norm=False, bounds=False):
        arr = np.ascontiguousarray(a, dtype=dtype)
        data.extend(bytes(-len(data) % 4))
        view = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=len(data),
                byteLength=arr.nbytes,
                target=34963 if kind == "SCALAR" else 34962,
            )
        )
        data.extend(arr.tobytes())
        acc = dict(bufferView=view, componentType=component, type=kind, count=len(arr))
        if bounds:
            acc.update(min=arr.min(0).tolist(), max=arr.max(0).tolist())
        if norm:
            acc["normalized"] = True
        i = len(doc["accessors"])
        doc["accessors"].append(acc)
        return i

    ref = None
    numbers = []
    for m in src["meshes"]:
        prim = m["primitives"][0]
        attrs = prim["attributes"]
        p = np.array(values(src, buf, attrs["POSITION"]), dtype="<f4")
        n = np.array(values(src, buf, attrs["NORMAL"]), dtype="<f4")
        c = np.array(values(src, buf, attrs["COLOR_0"]))
        tri = np.array(values(src, buf, prim["indices"]), dtype=np.int32).reshape(-1, 3)
        labs, ct, cycles = shells(p, tri)
        if ref is None:
            assert m["name"] == kind + "_L0"
            ref = (ct, cycles)
        d = np.linalg.norm(ct[:, None, :] - ref[0][None, :, :], axis=-1)
        ii, jj = linear_sum_assignment(d)
        assert len(ii) == len(ref[0]) == len(ct) and np.array_equal(
            ii, np.arange(len(ii))
        )
        comp = jj[labs]
        ctr = ref[0][comp]
        r = p - ctr
        angle = (np.arctan2(r[:, 2], r[:, 0]) / np.pi / 2 + 1) % 1
        xyz = p[tri]
        face = np.cross(xyz[:, 1] - xyz[:, 0], xyz[:, 2] - xyz[:, 0])
        cap = np.abs(face[:, 1]) > 0.74 * np.linalg.norm(face, axis=1)
        turn = (np.ptp(angle[tri], axis=1) > 0.5)[:, None] & (angle[tri] < 0.5)
        spec = np.column_stack(
            (
                tri.ravel(),
                np.repeat(cap, 3),
                np.where(np.repeat(cap[:, None], 3, axis=1), 0, turn).ravel(),
            )
        )
        hand, inv = np.unique(spec, axis=0, return_inverse=True)
        idx, caps, wind = hand.T
        uv = np.column_stack(
            (
                (angle[idx] + wind) * ref[1][comp[idx]] + 0.13 * comp[idx],
                -p[idx, 1] / METRES + 0.09 * comp[idx],
            )
        )
        mask = caps.astype(bool)
        uv[mask] = p[idx[mask]][:, [0, 2]] / METRES + 0.11 * comp[idx[mask], None]
        col = pigment(p, n, kind, c)
        cc = np.rint(255 * col[idx]).astype("u1")
        outattr = dict(
            POSITION=attr(p[idx], "<f4", 5126, "VEC3", bounds=True),
            NORMAL=attr(n[idx], "<f4", 5126, "VEC3"),
            TEXCOORD_0=attr(uv, "<f4", 5126, "VEC2"),
            COLOR_0=attr(cc, "u1", 5121, "VEC3", norm=True),
        )
        indices = attr(inv.ravel(), "<u2", 5123, "SCALAR")
        doc["meshes"].append(
            dict(
                name=m["name"],
                primitives=[dict(attributes=outattr, indices=indices, material=0)],
            )
        )
        numbers.append(
            dict(
                lod=m["name"],
                triangles=len(tri),
                original_vertices=len(p),
                new_vertices=len(idx),
                source_shells=len(ct),
            )
        )
    doc["buffers"] = [dict(byteLength=len(data))]
    save(doc, data, target)
    return numbers


def verify(kind, source, result):
    orig, b0 = parse(source)
    art, b1 = parse(result)
    for a, b in zip(orig["meshes"], art["meshes"], strict=True):
        assert a["name"] == b["name"]
        pa, pb = a["primitives"][0], b["primitives"][0]
        i, j = (
            np.array(values(d, p, q["indices"])).ravel()
            for d, p, q in ((orig, b0, pa), (art, b1, pb))
        )
        assert len(i) == len(j) and len(b["primitives"]) == 1
        for key in ("POSITION", "NORMAL"):
            assert np.array_equal(
                np.array(values(orig, b0, pa["attributes"][key]))[i],
                np.array(values(art, b1, pb["attributes"][key]))[j],
            )
        uv = np.array(values(art, b1, pb["attributes"]["TEXCOORD_0"]))[j].reshape(
            -1, 3, 2
        )
        assert np.isfinite(uv).all() and np.abs(uv).max() < 32
    assert orig["nodes"] == art["nodes"] and orig["scenes"] == art["scenes"]
    assert len(art["materials"]) == 1 and len(art["images"]) == 3


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--source", type=Path, default=ROOT)
    cli.add_argument("--out", type=Path, required=True)
    a = cli.parse_args()
    skins(a.out / "minerals")
    result = {}
    for name in KINDS:
        src = a.source / (name + ".glb")
        target = a.out / (name + ".glb")
        result[name] = rebuild(name, src, target)
        verify(name, src, target)
    (a.out / "verification.json").write_text(json.dumps(result, indent=2) + "\n")
    print(
        sum(x["triangles"] for meshes in result.values() for x in meshes),
        "ordered triangles, positions and source normals unchanged",
    )
