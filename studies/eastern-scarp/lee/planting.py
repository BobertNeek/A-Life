"""Reuse six actual LOD shapes per community, with fresh original-ground roots only.

Filter and repack the authored MESH accessors, not prior instance accessors;
the 220 held overlapping original Hollow placements are NOT cloned.
"""

from pathlib import Path
import csv, sys, json, math
import numpy as np
from scipy.spatial.transform import Rotation

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parents[2] / "scripts"), str(H)]
from finished_island_glb import parse, save, payload
from island_erratic_glb import streams
from island_inlet_ground import Inlet

EYE = (366.2, -83.7)
GROUP = {1: "windwillow", 2: "copper-rush", 3: "silver-mat"}


def root_frame(row, b):
    x, y, z = [float(row[k]) for k in ("x", "native_y", "z")]
    yy, gx, gz = b.floor(x, z)
    n = np.array([-gx, 1.0, -gz])
    n = n / np.linalg.norm(n)
    ax = np.cross([0.0, 1.0, 0.0], n)
    l = np.linalg.norm(ax)
    rot = Rotation.from_rotvec(ax / max(l, 1e-9) * math.acos(float(n[1])))
    yaw = Rotation.from_rotvec(np.array([0.0, float(row["yaw"]), 0.0]))
    # Whole local footprint plausibly sits on its actual substrate. The small
    # published root extends slightly below local soil; do not shift its Y.
    return [x, y, z], (rot * yaw).as_quat()


def compose(src, native, out, mode):
    b = Inlet(native / "island-v1.bin")
    orig, raw = parse(src / "registered-close.glb")
    read = list(csv.DictReader((out / "lee-roots.csv").open()))
    doc = dict(
        asset=dict(
            version="2.0", generator="Twin Needle Rise / native rooted habitat palette"
        ),
        scene=0,
        scenes=[dict(nodes=[])],
        nodes=[],
        meshes=[],
        materials=orig["materials"],
        accessors=[],
        bufferViews=[],
    )
    binary = bytearray()
    emit = streams(doc, binary)
    groups = {}
    totals = {"near": 0, "silhouette": 0}
    counts = {"near": 0, "silhouette": 0}
    for r in read:
        x, z = float(r["x"]), float(r["z"])
        lod = (
            "near"
            if mode == "edit"
            or (mode == "eye" and math.hypot(x - EYE[0], z - EYE[1]) < 6.2)
            else "silhouette"
        )
        group = int(r["habitat"])
        key = GROUP[group] + "-" + str(r["form"]) + " / " + lod
        groups.setdefault(key, []).append(r)

    def copy(i):
        a = orig["accessors"][i]
        ty = a["componentType"]
        comp = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}[ty]
        stream = np.frombuffer(payload(orig, raw, i), dtype=comp).reshape(
            a["count"], -1
        )
        return emit(
            stream,
            ty,
            a["type"],
            indices=a["type"] == "SCALAR",
            bounds="min" in a,
            normalized=a.get("normalized", False),
        )

    for n in orig["nodes"]:
        name = n["name"]
        if name not in groups:
            continue
        rr = groups[name]
        lod = name.rsplit(" / ", 1)[1]
        old = orig["meshes"][n["mesh"]]
        primitives = []
        tri = 0
        for p in old["primitives"]:
            primitives.append(
                dict(
                    material=p["material"],
                    indices=copy(p["indices"]),
                    attributes={k: copy(v) for k, v in p["attributes"].items()},
                )
            )
            tri += orig["accessors"][p["indices"]]["count"] // 3
        pos, quat = zip(*(root_frame(r, b) for r in rr))
        a = orig["accessors"][old["primitives"][0]["attributes"]["POSITION"]]
        worldwidth = max(a["max"][0] - a["min"][0], a["max"][2] - a["min"][2])
        scal = np.array(
            [[2 * float(r["canopy_m"]) / worldwidth] * 3 for r in rr], "<f4"
        )
        transforms = dict(
            TRANSLATION=emit(np.array(pos, "<f4"), 5126, "VEC3", bounds=True),
            ROTATION=emit(np.array(quat, "<f4"), 5126, "VEC4"),
            SCALE=emit(scal, 5126, "VEC3"),
        )
        i = len(doc["nodes"])
        doc["nodes"].append(
            dict(
                name="Rise " + name,
                mesh=i,
                extensions={"EXT_mesh_gpu_instancing": {"attributes": transforms}},
            )
        )
        doc["meshes"].append(dict(name=name, primitives=primitives))
        doc["scenes"][0]["nodes"].append(i)
        counts[lod] += len(rr)
        totals[lod] += tri * len(rr)
    if sum(counts.values()) != len(read):
        raise ValueError(
            "Incomplete true source plant palette: do not substitute a generic shape"
        )
    doc["extensionsUsed"] = doc["extensionsRequired"] = ["EXT_mesh_gpu_instancing"]
    doc["buffers"] = [dict(byteLength=len(binary))]
    save(doc, binary, out / ("rise-" + mode + ".glb"))
    return dict(roots=counts, triangles=totals, meshes=len(groups))


def prep(native, hollow, out):
    report = {
        mode: compose(hollow, native, out, mode) for mode in ("eye", "guardian", "edit")
    }
    (out / "rise-lods.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


if __name__ == "__main__":
    print(prep(*(Path(a) for a in sys.argv[1:4])))
