"""Palette and original-root register for later native adoption, not game code.

Never import the four full study scenes together into gameplay. This desk
contains one local origin specimen per published distance choice, no island
ground, sea, legacy Hollow root, static colliders, gameplay entity or UI.
The separate near litter mesh uses true world coordinates; it is never a plant.
"""

import copy, csv, hashlib, json, math, sys
from collections import Counter
from pathlib import Path
import numpy as np

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[3] / "scripts"))
from finished_island_glb import parse, save, values, payload
from island_erratic_glb import streams


def pack(rise, woven, out):
    out.mkdir(parents=True, exist_ok=True)
    doc = dict(
        asset=dict(
            version="2.0", generator="Twin Needle site-specific reusable art palette"
        ),
        scene=0,
        scenes=[],
        nodes=[],
        meshes=[],
        materials=[],
        accessors=[],
        bufferViews=[],
    )
    dest = bytearray()
    emit = streams(doc, dest)
    material = {}
    families = {}
    roots = {}
    for file, habitat, mode in [
        (rise / "rise-edit.glb", "rise", "close"),
        (rise / "rise-guardian.glb", "rise", "guardian"),
        (woven / "stitchgrass-eye.glb", "stitchgrass", "read"),
        (woven / "stitchgrass-guardian.glb", "stitchgrass", "guardian"),
    ]:
        d, raw = parse(file)
        cache = {}

        def cp(ix):
            if ix in cache:
                return cache[ix]
            a = d["accessors"][ix]
            typ = a["componentType"]
            dtype = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}[typ]
            arr = np.frombuffer(payload(d, raw, ix), dtype=dtype).reshape(
                a["count"], -1
            )
            ans = emit(
                arr,
                typ,
                a["type"],
                indices=a["type"] == "SCALAR",
                bounds="min" in a,
                normalized=a.get("normalized", False),
            )
            cache[ix] = ans
            return ans

        for n in d["nodes"]:
            ext = (
                n.get("extensions", {})
                .get("EXT_mesh_gpu_instancing", {})
                .get("attributes")
            )
            if not ext:
                continue  # Never turn the litter's absolute positions into local plant geometry
            stem, stocklod = n["name"].split(" / ")
            tag = stem.removeprefix("Rise ").replace("Stitchgrass ", "stitchgrass-")
            lod = (
                mode
                if mode != "read"
                else {"folded": "close", "silhouette": "middle"}[stocklod]
            )
            fam = families.setdefault(tag, {})
            m = d["meshes"][n["mesh"]]
            prims = []
            lo = [math.inf] * 3
            hi = [-math.inf] * 3
            tri = 0
            for p in m["primitives"]:
                mat = d["materials"][p["material"]]
                key = json.dumps(mat, sort_keys=True)
                if key not in material:
                    material[key] = len(doc["materials"])
                    doc["materials"].append(copy.deepcopy(mat))
                att = {k: cp(v) for k, v in p["attributes"].items()}
                prims.append(
                    dict(
                        attributes=att, indices=cp(p["indices"]), material=material[key]
                    )
                )
                a = d["accessors"][p["attributes"]["POSITION"]]
                lo = np.minimum(lo, a["min"]).tolist()
                hi = np.maximum(hi, a["max"]).tolist()
                tri += d["accessors"][p["indices"]]["count"] // 3
            idx = len(doc["meshes"])
            name = tag + " / " + lod
            doc["meshes"].append(dict(name=name, primitives=prims))
            doc["nodes"].append(dict(name=name, mesh=idx))
            doc["scenes"].append(dict(name=name, nodes=[idx]))
            fam[lod] = dict(mesh=idx, local_bounds_m=[lo, hi], triangles=tri)
            T = values(d, raw, ext["TRANSLATION"])
            Q = values(d, raw, ext["ROTATION"])
            S = values(d, raw, ext["SCALE"])
            for t, q, s in zip(T, Q, S):
                key = tuple(round(v, 5) for v in t)
                if key not in roots:
                    roots[key] = dict(
                        form=tag, p=key, q=tuple(round(v, 8) for v in q), sizes={}
                    )
                rec = roots[key]
                if rec["form"] != tag:
                    raise ValueError("Different plants at an identical root")
                rec["sizes"][lod] = round(float(s[0]), 8)
    output = []
    for xyz, row in sorted(roots.items()):
        tag = row["form"]
        sizes = row["sizes"]
        if tag.startswith("stitchgrass-"):
            # Fine and middle use the same specimen metres and root frame.
            # Authoring eye GLB contains one or the other per root, never both.
            v = sizes.get("close", sizes.get("middle"))
            sizes.setdefault("close", v)
            sizes.setdefault("middle", v)
        else:
            if "middle" not in families[tag]:
                families[tag]["middle"] = {
                    **families[tag]["guardian"],
                    "shares": "guardian",
                }
            sizes["middle"] = sizes["guardian"]
        if set(sizes) != set(("close", "middle", "guardian")):
            raise ValueError("Incomplete measured pose " + tag)
        cx = int(math.floor(xyz[0] / 8))
        cz = int(math.floor(xyz[2] / 8))
        output.append(
            [
                tag,
                *xyz,
                *row["q"],
                sizes["close"],
                sizes["middle"],
                sizes["guardian"],
                cx,
                cz,
            ]
        )
    doc["buffers"] = [dict(byteLength=len(dest))]
    save(doc, dest, out / "meadow-forms.glb")
    with (out / "meadow-roots.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            (
                "form",
                "world_x",
                "native_y",
                "world_z",
                "qx",
                "qy",
                "qz",
                "qw",
                "scale_close",
                "scale_middle",
                "scale_guardian",
                "cell_x_8m",
                "cell_z_8m",
            )
        )
        w.writerows(output)
    static = Counter(r[0] for r in output)
    cells = Counter((r[-2], r[-1]) for r in output)
    note = dict(
        version=1,
        units="metres; glTF Y-up, quaternion x/y/z/w, one scale at a time",
        source_V1_sha256="8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38",
        roots=len(output),
        families=families,
        unique_local_meshes=len(doc["meshes"]),
        root_counts=static,
        spatial_cells_8m=len(cells),
        largest_cell=int(max(cells.values())),
        native_defaults_changed=False,
        optional_litter="Locally rooted world-space debris is ONLY in the standalone stitchgrass-eye.glb. Do not draw the instanced plants from that file on top of this register; when adopting register, extract its one non-instanced debris node.",
    )
    (out / "meadow-palette.json").write_text(json.dumps(note, indent=2) + "\n")
    return dict(
        roots=len(output),
        cells=len(cells),
        largest_cell=max(cells.values()),
        forms=len(families),
        unique_local_meshes=len(doc["meshes"]),
        local_geometry_bytes=(out / "meadow-forms.glb").stat().st_size,
    )


if __name__ == "__main__":
    print(pack(*(Path(p) for p in sys.argv[1:])))
