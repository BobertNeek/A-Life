"""Copy the held decorative heath without changing a single instance transform.

Only the original pennons inside one meter-registered seven-plant colony get
the new near geometry. Every fern, heath and distant old crown remains held.
"""

import json, sys
from pathlib import Path
import numpy as np
from specimen import HERE
from finished_island_glb import parse, save, values
from island_erratic_glb import streams


def transfer(held, palette, out):
    doc, raw = parse(held)
    lib, lb = parse(palette)
    emit = streams(doc, raw)
    ledger = []

    def sub(attrs, indices):
        res = {}
        for key, old in attrs.items():
            a = doc["accessors"][old]
            lengths = a["type"]
            ps = np.asarray(values(doc, raw, old), "<f4")[indices]
            res[key] = emit(ps, 5126, lengths, bounds=(key == "TRANSLATION"))
        return res

    for node in list(doc["nodes"]):
        name = node["name"]
        if not name.startswith("pennon"):
            continue
        attrs = node["extensions"]["EXT_mesh_gpu_instancing"]["attributes"]
        xy = np.array(values(doc, raw, attrs["TRANSLATION"]))
        selected = np.hypot(xy[:, 0] - 245.04, xy[:, 2] + 143.43) < 1.58
        far = np.nonzero(~selected)[0]
        close = np.nonzero(selected)[0]
        if not len(close):
            continue
        ix = next(q for q in lib["nodes"] if q["name"] == "FoldedPennon" + name[-1])
        mesh = lib["meshes"][ix["mesh"]]
        src = mesh["primitives"][0]
        attrs2 = {}
        for key, acc in src["attributes"].items():
            a = lib["accessors"][acc]
            dtype = {5121: "u1", 5126: "<f4"}[a["componentType"]]
            vals = np.asarray(values(lib, lb, acc, normalize=False), dtype=dtype)
            if key == "POSITION":
                # The independent specimen files express local mesh positions
                # relative to a pivot. Restore local root-space vertices first
                # so the existing world scale, yaw and slope tilt stay exact.
                vals += np.array(ix["translation"], dtype="<f4")
            attrs2[key] = emit(
                vals,
                a["componentType"],
                a["type"],
                normalized=a.get("normalized", False),
                bounds=key == "POSITION",
            )
        oldidx = src["indices"]
        a = lib["accessors"][oldidx]
        triangles = np.asarray(values(lib, lb, oldidx), dtype="<u2").ravel()
        pr = dict(
            attributes=attrs2,
            material=0,
            indices=emit(triangles, 5123, "SCALAR", indices=True),
        )
        mi = len(doc["meshes"])
        doc["meshes"].append(dict(name="LivingValve" + name[-1], primitives=[pr]))
        reg = sub(attrs, close)
        keep = sub(attrs, far)
        new = dict(
            name="LivingValve" + name[-1],
            mesh=mi,
            extensions=dict(EXT_mesh_gpu_instancing=dict(attributes=reg)),
        )
        doc["scenes"][0]["nodes"].append(len(doc["nodes"]))
        doc["nodes"].append(new)
        node["extensions"]["EXT_mesh_gpu_instancing"]["attributes"] = keep
        dirs = values(doc, raw, attrs["ROTATION"])
        scales = values(doc, raw, attrs["SCALE"])
        for i in close:
            ledger.append(
                dict(
                    original=name,
                    index=int(i),
                    position=xy[i].tolist(),
                    quaternion=dirs[i],
                    scale=scales[i],
                    new_mesh=new["name"],
                )
            )
    out.mkdir(exist_ok=True, parents=True)
    doc["buffers"] = [dict(byteLength=len(raw))]
    save(doc, raw, out / "wind-heath.glb")
    (out / "seven-roots.json").write_text(json.dumps(ledger, indent=2) + "\n")


if __name__ == "__main__":
    transfer(*(Path(a) for a in sys.argv[1:4]))
