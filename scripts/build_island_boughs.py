"""Prototype replacement of the actual pine canopy topology, not a new tree population."""

import argparse
import json
import math
from pathlib import Path
import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from finished_island_glb import parse, values, save
from build_island_grove_forms import D, SURFACE, layout
from island_bough_mesh import center, scaffold, mesh


def rooted_trunk(doc, blob, prim):
    att = prim["attributes"]
    p = np.array(values(doc, blob, att["POSITION"]), dtype="f4")
    n = np.array(values(doc, blob, att["NORMAL"]), dtype="f4")
    co = np.rint(255 * np.array(values(doc, blob, att["COLOR_0"]))).astype("u1")
    if co.shape[1] == 3:
        co = np.column_stack((co, np.full(len(co), 255, "u1")))
    tri = np.array(values(doc, blob, prim["indices"])).reshape(-1, 3)
    xyz, inv = np.unique(np.round(p, 5), axis=0, return_inverse=True)
    f = inv[tri]
    v = f.ravel()
    graph = coo_matrix(
        (np.ones(v.size, "u1"), (v, np.roll(f, 1, axis=1).ravel())),
        shape=(len(xyz), len(xyz)),
    )
    _, labels = connected_components(graph, directed=False)
    root = np.unique(
        labels[(xyz[:, 1] < 0.12) & (np.hypot(xyz[:, 0], xyz[:, 2]) < 0.27)]
    )
    assert len(root) == 1
    picked = tri[labels[f[:, 0]] == root[0]]
    ids, remap = np.unique(picked.ravel(), return_inverse=True)
    assert p[ids, 1].min() < 0.015 and p[ids, 1].max() > 10
    return p[ids], n[ids], co[ids], remap.reshape(-1, 3), len(tri)


def bent_stem(p, n, profile, quarter):
    q = p.astype("f8", copy=True)
    nn = n.astype("f8", copy=True)
    for j, y in enumerate(p[:, 1]):
        if y <= 3.2:
            continue
        fc = center(profile, y, quarter)
        deriv = (
            center(profile, y + 0.001, quarter) - center(profile, y - 0.001, quarter)
        ) / 0.002
        q[j] += fc - (0.0, y, 0.0)
        nn[j, 1] = (n[j, 1] - n[j, 0] * deriv[0] - n[j, 2] * deriv[2]) / deriv[1]
        nn[j] /= np.linalg.norm(nn[j])
    return q.astype("f4"), nn.astype("f4")


def all_forms(placements):
    mapped = layout(placements)
    for i, p in enumerate(json.loads(placements.read_text())):
        if p["kind"] == "Conifer" and str(i) not in mapped:
            quarter = round((math.atan2(-0.30, 0.69) + p["yaw"]) * 2 / math.pi) % 4
            mapped[str(i)] = dict(
                form=9 + quarter, xz=[p["position"][0], p["position"][2]]
            )
    return mapped


def normals(p, f):
    v = p[f]
    cross = np.cross(v[:, 1] - v[:, 0], v[:, 2] - v[:, 0])
    length = np.linalg.norm(cross, axis=1)
    assert np.all(length > 1e-7)
    fn = cross / length[:, None]
    nn = np.zeros_like(p)
    # Angle-weighted curves on each isolated branch fan, never across canopy
    # gaps. Retain tapered silhouettes while avoiding laser-cut facet shading.
    for k in range(3):
        a = v[:, (k + 1) % 3] - v[:, k]
        b = v[:, (k + 2) % 3] - v[:, k]
        cos = np.sum(a * b, axis=1) / (
            np.linalg.norm(a, axis=1) * np.linalg.norm(b, axis=1)
        )
        weight = np.arccos(cos.clip(-1, 1))
        np.add.at(nn, f[:, k], fn * weight[:, None])
    lengths = np.linalg.norm(nn, axis=1)
    assert lengths.min() > 1e-5
    return nn / lengths[:, None]


def build(source, placements, out):
    doc0, bin0 = parse(source)
    stems = []
    for m in doc0["meshes"]:
        stems.append(rooted_trunk(doc0, bin0, m["primitives"][0]))
    out.mkdir(exist_ok=True, parents=True)
    from island_needle_surface import bake

    mean = bake(out / "canopy")
    doc = dict(
        asset=dict(version="2.0", generator="Qualia interlocking maritime boughs"),
        scene=0,
        scenes=[dict(nodes=[])],
        nodes=[],
        meshes=[],
        materials=[
            dict(
                name="Rooted pine bark and needle sprays",
                doubleSided=True,
                normalTexture=dict(index=1),
                occlusionTexture=dict(index=2, strength=0.72),
                pbrMetallicRoughness=dict(
                    baseColorTexture=dict(index=0),
                    metallicRoughnessTexture=dict(index=2),
                    metallicFactor=0,
                    roughnessFactor=1,
                ),
            )
        ],
        images=[
            dict(uri="canopy/" + s)
            for s in ("needles.png", "needles-normal.png", "needles-orm.png")
        ],
        samplers=[dict(magFilter=9729, minFilter=9987, wrapS=33071, wrapT=33071)],
        textures=[dict(sampler=0, source=k) for k in range(3)],
        bufferViews=[],
        accessors=[],
    )
    bin = bytearray()
    receipt = {}

    def attr(rows, dtype, component, kind, norm=False, bounds=False):
        a = np.ascontiguousarray(rows, dtype=dtype)
        bin.extend(bytes(-len(bin) % 4))
        v = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=len(bin),
                byteLength=a.nbytes,
                target=34963 if kind == "SCALAR" else 34962,
            )
        )
        bin.extend(a.tobytes())
        ac = dict(bufferView=v, componentType=component, type=kind, count=len(a))
        if bounds:
            ac.update(min=a.min(0).tolist(), max=a.max(0).tolist())
        if norm:
            ac["normalized"] = True
        j = len(doc["accessors"])
        doc["accessors"].append(ac)
        return j

    for profile in (1, 2, 3):
        for quarter in range(4):
            bough = scaffold(profile, quarter)
            for l, (old, oldn, oldc, ft, budget) in enumerate(stems):
                pt, nt = bent_stem(old, oldn, profile, quarter)
                new = mesh(bough, profile, quarter, l)
                p = np.array(new.v)
                # Widen only the upper artistic surface of simplified foliage.
                # Preserve original stem and endpoints at the true root; do not
                # resize the whole prop or change any native transform.
                if l == 0:
                    ref_bounds = np.stack((p.min(0), p.max(0)))
                else:
                    here = np.stack((p.min(0), p.max(0)))
                    for axis in (0, 2):
                        gain = np.clip(ref_bounds[:, axis] / here[:, axis], 0.88, 1.29)
                        p[:, axis] *= np.where(p[:, axis] < 0, gain[0], gain[1])
                f = np.array(new.f)
                n = normals(p, f)
                foliage = np.full((len(p), 4), 255, "u1")
                foliage[:, :3] = (
                    np.rint(255 * np.array(new.c) / mean).clip(0, 255).astype("u1")
                )
                xyz = np.vstack((pt, p))
                norm = np.vstack((nt, n))
                col = np.vstack((oldc, foliage))
                faces = np.vstack((ft, len(pt) + f))
                assert len(faces) <= budget and xyz[:, 1].max() < 12.001
                name = f"PineForm{(profile-1)*4+quarter+1}_L{l}"
                # White, flat, padded bark swatch. Nondegenerate coordinates
                # let native tangent generation run even on triangles wholly
                # inside the unchanged original trunk.
                bark_uv = np.column_stack(
                    (
                        0.048 + 0.071 * old[:, 0],
                        0.944 + 0.024 * old[:, 1] / 12 + 0.046 * old[:, 2],
                    )
                )
                bark_uv = np.clip(bark_uv, (0.006, 0.915), (0.081, 0.984))
                tex = np.vstack((bark_uv, new.uv))
                raw = dict(
                    TEXCOORD_0=attr(tex, "<f4", 5126, "VEC2"),
                    POSITION=attr(xyz, "<f4", 5126, "VEC3", bounds=True),
                    NORMAL=attr(norm, "<f4", 5126, "VEC3"),
                    COLOR_0=attr(col, "u1", 5121, "VEC4", norm=True),
                )
                model = dict(
                    name=name,
                    primitives=[
                        dict(
                            attributes=raw,
                            indices=attr(faces.ravel(), "<u2", 5123, "SCALAR"),
                            material=0,
                        )
                    ],
                )
                k = len(doc["meshes"])
                doc["meshes"].append(model)
                doc["nodes"].append(dict(name=name, mesh=k))
                doc["scenes"][0]["nodes"].append(k)
                receipt[name] = dict(
                    source_triangles=budget,
                    triangles=len(faces),
                    unchanged_root_faces=len(ft),
                    height_m=round(float(xyz[:, 1].max()), 3),
                )
    doc["buffers"] = [dict(byteLength=len(bin))]
    save(doc, bin, out / "grove-crowns.glb")
    mapping = all_forms(placements)
    (out / "grove-crowns.json").write_text(
        json.dumps(mapping, separators=(",", ":")) + "\n"
    )
    (out / "bough-budget.json").write_text(json.dumps(receipt, indent=2) + "\n")


def verify(source, placements, surface, out):
    d0, b0 = parse(source)
    d, b = parse(out / "grove-crowns.glb")
    assert (
        len(d["meshes"]) == 36
        and len(d["materials"]) == 1
        and len(d["images"]) == 3
        and d["materials"][0]["normalTexture"]["index"] == 1
    )
    from PIL import Image

    for e in d["images"]:
        Image.open(out / e["uri"]).verify()
    roots = [rooted_trunk(d0, b0, m["primitives"][0]) for m in d0["meshes"]]
    bounds = {}
    outlines = {}
    tri_counts = {}
    for m in d["meshes"]:
        name, lev = m["name"].split("_L")
        l = int(lev)
        p0, n0, c0, ft, maxtri = roots[l]
        assert len(m["primitives"]) == 1
        pr = m["primitives"][0]
        a = pr["attributes"]
        pp = np.array(values(d, b, a["POSITION"]))
        nn = np.array(values(d, b, a["NORMAL"]))
        cc = np.array(values(d, b, a["COLOR_0"], normalize=False))
        ix = np.array(values(d, b, pr["indices"])).reshape(-1, 3)
        tex = np.array(values(d, b, a["TEXCOORD_0"]))
        assert np.isfinite(tex).all() and tex.min() > 0 and tex.max() < 1
        a_uv, b_uv = tex[ix[:, 1]] - tex[ix[:, 0]], tex[ix[:, 2]] - tex[ix[:, 0]]
        assert np.min(np.abs(a_uv[:, 0] * b_uv[:, 1] - a_uv[:, 1] * b_uv[:, 0])) > 1e-12
        assert np.all(tex[: len(p0), 0] < 47 / 512) and np.all(
            tex[: len(p0), 1] > 461 / 512
        )
        assert (
            np.array_equal(ix[: len(ft)], ft)
            and len(ix) <= maxtri
            and nn.shape == pp.shape
        )
        low = p0[:, 1] <= 3.2
        assert (
            np.array_equal(p0[low], pp[: len(p0)][low])
            and np.array_equal(n0[low], nn[: len(n0)][low])
            and np.array_equal(c0[low], cc[: len(c0)][low])
        )
        assert (
            np.isfinite(pp).all()
            and np.max(np.abs(np.linalg.norm(nn, axis=1) - 1)) < 0.0003
        )
        tri = pp[ix[len(ft) :]]
        assert (
            np.min(
                np.linalg.norm(
                    np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0]), axis=1
                )
            )
            > 1e-7
        )
        outlines.setdefault(name, []).append((pp, ix))
        bounds.setdefault(name, []).append((pp[:, 1].max(), np.ptp(pp, axis=0)[[0, 2]]))
        tri_counts.setdefault(l, []).append(len(ix))
    from island_canopy_outline import score

    screens = np.array([score(levels) for levels in outlines.values()])
    projected = {
        f"L{i+1}": dict(
            min_iou=round(float(screens[:, i, :, 0].min()), 3),
            median_iou=round(float(np.median(screens[:, i, :, 0])), 3),
            min_area_ratio=round(float(screens[:, i, :, 1].min()), 3),
            max_area_ratio=round(float(screens[:, i, :, 1].max()), 3),
        )
        for i in range(2)
    }
    # Eight real projected canopy outlines per form; simply matching height
    # and axes does not prevent far trees from turning into thin black pins.
    for l, (iou, area) in enumerate(((0.72, 0.78), (0.68, 0.75))):
        assert screens[:, l, :, 0].min() > iou, projected
        assert screens[:, l, :, 1].min() > area, projected
        assert screens[:, l, :, 1].max() < 1.20, projected
    ratio = []
    for v in bounds.values():
        assert np.ptp([q[0] for q in v]) < 0.02
        for _, extent in v[1:]:
            ratio.extend(extent / v[0][1])
    assert min(ratio) > 0.82 and max(ratio) < 1.19, (min(ratio), max(ratio))
    import struct

    raw = surface.read_bytes()
    w, h, _, _, _, n = struct.unpack_from("<IIfffI", raw, 8)
    boxes = np.frombuffer(raw, "<f4", n * 6, 32 + w * h * 4).reshape(-1, 6)
    mapping = json.loads((out / "grove-crowns.json").read_text())
    props = json.loads(placements.read_text())
    idx = 0
    anchors = 0
    for j, p in enumerate(props):
        kind = p["kind"]
        if kind not in (
            "Conifer",
            "Broadleaf",
            "Sapling",
            "Cliff",
        ) and not kind.startswith("Boulder"):
            continue
        box = boxes[idx]
        idx += 1
        if kind == "Conifer":
            x, y, z = p["position"]
            s = p["scale"]
            record = mapping[str(j)]
            assert record["xz"] == [x, z] and 1 <= record["form"] <= 12
            assert np.allclose(
                box,
                [
                    x - 0.175 * s,
                    z - 0.175 * s,
                    x + 0.175 * s,
                    z + 0.175 * s,
                    y,
                    y + 3 * s,
                ],
                atol=0.00011,
                rtol=0,
            )
            anchors += 1
    assert idx == len(boxes) and len(mapping) == anchors == 421
    return dict(
        confirmed_original_pine_anchors_and_proxies=anchors,
        preserved_original_bark_through_m=3.2,
        pooled_forms=12,
        primitives_per_lod=1,
        new_min_max_triangles_by_lod={
            str(k): [min(x), max(x)] for k, x in tri_counts.items()
        },
        stock_lod_budgets=[3456, 668, 172],
        axis_extent_ratio_range_l1_l2_to_l0=[
            round(float(min(ratio)), 3),
            round(float(max(ratio)), 3),
        ],
        all_new_canopy_faces_nondegenerate=True,
        projected_silhouettes_eight_views_each=projected,
    )


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--source", type=Path, default=D / "Conifer.glb")
    cli.add_argument("--placements", type=Path, default=D / "props.json")
    cli.add_argument("--surface", type=Path, default=SURFACE)
    cli.add_argument("--out", type=Path, required=True)
    a = cli.parse_args()
    build(a.source, a.placements, a.out)
    result = verify(a.source, a.placements, a.surface, a.out)
    (a.out / "validation.json").write_text(json.dumps(result, indent=2) + "\n")
    print(result)
