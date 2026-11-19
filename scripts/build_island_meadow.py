"""Grow a view-only wild ground layer on the untouched lower valley.

The meadow reuses the original near mesh's complete glTF material including
UVs, normals and its linear vertex pigment. A smooth botanical colour wash
therefore has no pasted-on alpha outline and no new sampler. Only the short
rooted blades are new geometry. The two native plant libraries are separate.
"""

from collections import defaultdict
from copy import deepcopy
from pathlib import Path
import json, math
import numpy as np
from build_island_geology import Glb
from build_island_inlet import V2
from finished_island_glb import save
from island_inlet_sod import Fibres
from island_meadow_swales import Meadow, BOUNDS, paint_grass


def export(surface, habitats, placements, dest, inked=False):
    m = Meadow(surface, V2, placements)
    glb = Glb(habitats)
    x0, x1, z0, z1 = BOUNDS
    patches = defaultdict(list)
    shoots = defaultdict(Fibres)

    def key(x, z):
        return (math.floor(x / 50), math.floor(z / 50))

    def root(x, z):
        return shoots[key(x, z)]

    if inked:
        from island_meadow_swales import paint_sprays
        from island_meadow_ink import draw

        draw(Path(dest) / "meadow/sedge-ink.png")
        note = paint_sprays(root, m)
    else:
        note = paint_grass(root, m)
    source_budget = 0
    for item in glb.doc["meshes"]:
        prim = item["primitives"][0]
        source = prim["attributes"]
        pos = glb.attr(source["POSITION"])
        area = pos[:, [0, 2]]
        if (
            area[:, 0].max() < x0
            or area[:, 0].min() > x1
            or area[:, 1].max() < z0
            or area[:, 1].min() > z1
        ):
            continue
        face_idx = glb.attr(prim["indices"]).reshape(-1, 3)
        pts = area[face_idx]
        c = pts.mean(axis=1)
        mask = (c[:, 0] > x0) & (c[:, 0] < x1) & (c[:, 1] > z0) & (c[:, 1] < z1)
        c = c[mask]
        faces = face_idx[mask]
        if not len(faces):
            continue
        keep = (m.field(c[:, 0], c[:, 1]) > 0.0003) & m.safe_mask(c[:, 0], c[:, 1])
        c = c[keep]
        faces = faces[keep]
        norm = glb.attr(source["NORMAL"])
        uv = glb.attr(source["TEXCOORD_0"])
        orig = glb.attr(source["COLOR_0"])
        for cell in sorted(set(key(*q) for q in c)):
            which = np.array([key(*q) == cell for q in c])
            use = faces[which]
            arr, mapped = np.unique(use.ravel(), return_inverse=True)
            vertices = pos[arr].copy()
            # A shallow separation only on the original near triangles,
            # which are the same in V1 and the opt-in highland study.
            vertices[:, 1] += 0.004
            col = orig[arr].copy()
            col[:, :3] = np.uint8(
                np.rint(col[:, :3] * m.multiplier(vertices[:, 0], vertices[:, 2])).clip(
                    0, 255
                )
            )
            patches[cell].append(
                (vertices, norm[arr], uv[arr], col, mapped.reshape(-1, 3))
            )
            source_budget += len(use)
    d = {
        k: deepcopy(glb.doc[k])
        for k in ["asset", "images", "textures", "samplers", "materials"]
    }
    d["asset"][
        "generator"
    ] = "Qualia wild-meadow artist: V1 slope, native material and curved seedbank"
    d.update(
        scene=0,
        scenes=[dict(nodes=[])],
        nodes=[],
        meshes=[],
        bufferViews=[],
        accessors=[],
    )
    botan = len(d["materials"])
    d["materials"].append(
        dict(
            name="Sheltered sedge and straw",
            doubleSided=True,
            pbrMetallicRoughness=dict(roughnessFactor=0.84, metallicFactor=0),
        )
    )
    if inked:
        image = len(d["images"])
        texture = len(d["textures"])
        sampler = len(d["samplers"])
        d["samplers"].append(
            dict(wrapS=33071, wrapT=33071, magFilter=9729, minFilter=9987)
        )
        d["images"].append(
            dict(
                uri="meadow/sedge-ink.png", name="Original four folded sedge printings"
            )
        )
        d["textures"].append(dict(sampler=sampler, source=image))
        d["materials"][botan].update(alphaMode="MASK", alphaCutoff=0.39)
        d["materials"][botan]["pbrMetallicRoughness"]["baseColorTexture"] = dict(
            index=texture
        )
    buf = bytearray()

    def att(a, kind, bounds=False, index=False, normalized=False):
        a = np.ascontiguousarray(a)
        buf.extend(b"\0" * (-len(buf) % 4))
        p = len(buf)
        buf.extend(a.tobytes())
        v = len(d["bufferViews"])
        d["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=p,
                byteLength=a.nbytes,
                target=34963 if index else 34962,
            )
        )
        t = {
            np.dtype("<f4"): 5126,
            np.dtype("u1"): 5121,
            np.dtype("<u2"): 5123,
            np.dtype("<u4"): 5125,
        }[a.dtype]
        rec = dict(bufferView=v, componentType=t, count=len(a), type=kind)
        if bounds:
            rec.update(min=a.min(axis=0).tolist(), max=a.max(axis=0).tolist())
        if normalized:
            rec["normalized"] = True
        j = len(d["accessors"])
        d["accessors"].append(rec)
        return j

    def build(key, part, pos, nrm, uv, col, faces, matid):
        tx, tz = key
        ox, oz = (tx + 0.5) * 50, (tz + 0.5) * 50
        attr = dict(
            POSITION=att(
                np.array(pos, "<f4") - np.array([ox, 0, oz], "<f4"), "VEC3", bounds=True
            ),
            NORMAL=att(np.array(nrm, "<f4"), "VEC3"),
            COLOR_0=att(np.asarray(col, "u1"), "VEC4", normalized=True),
        )
        if uv is not None:
            attr["TEXCOORD_0"] = att(np.array(uv, "<f4"), "VEC2")
        ix = att(
            np.asarray(faces, "<u4" if len(pos) > 65535 else "<u2").ravel(),
            "SCALAR",
            index=True,
        )
        name = f"Sward_{tx}_{tz}_{part}"
        j = len(d["nodes"])
        d["meshes"].append(
            dict(
                name=name,
                primitives=[dict(attributes=attr, indices=ix, material=matid)],
            )
        )
        d["nodes"].append(dict(name=name, mesh=j, translation=[ox, 0, oz]))
        d["scenes"][0]["nodes"].append(j)

    for cell, groups in sorted(patches.items()):
        v, n, uv, c, f = [], [], [], [], []
        off = 0
        for vv, nn, uu, cc, ff in groups:
            v.extend(vv)
            n.extend(nn)
            uv.extend(uu)
            c.extend(cc)
            f.extend(ff + off)
            off += len(vv)
        build(cell, "soil", v, n, uv, c, f, 0)
    blades = 0
    for cell, sprays in sorted(shoots.items()):
        v = np.array(sprays.v, "<f4")
        face = np.array(sprays.tri, "<i4")
        if not len(v):
            continue
        n = np.zeros_like(v)
        per = np.cross(v[face[:, 1]] - v[face[:, 0]], v[face[:, 2]] - v[face[:, 0]])
        for j in range(3):
            np.add.at(n, face[:, j], per)
        mag = np.linalg.norm(n, axis=1)
        n[mag > 1e-9] /= mag[mag > 1e-9, None]
        n[mag <= 1e-9] = (0, 1, 0)
        if inked:
            # Blended sky-facing normals prevent upright surfaces from
            # casting their entire back half as pitch-black paper.
            n = n * 0.30 + np.array([0.04, 0.80, -0.075])
            n /= np.linalg.norm(n, axis=1)[:, None]
        build(cell, "leaves", v, n, sprays.uv if inked else None, sprays.c, face, botan)
        blades += len(face)
    d["buffers"] = [dict(byteLength=len(buf))]
    dest = Path(dest)
    dest.mkdir(exist_ok=True, parents=True)
    asset = dest / "meadow-sward.glb"
    save(d, buf, asset)
    note.update(
        existing_surface_triangles=source_budget,
        low_sward_triangles=blades,
        soil_tiles=len(patches),
        leaf_tiles=len(shoots),
        region_xz=BOUNDS,
    )
    (dest / "sward-source.json").write_text(json.dumps(note, indent=2) + "\n")
    return asset
