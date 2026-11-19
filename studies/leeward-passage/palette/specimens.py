"""Ordinary core glTF prototype shelf for importers without instancing support.

This is not a second landscape. The mesh centres are soil-level local origins;
studio shelf translations are layout aids only. Mount the named mesh, not its
shelf transform, with an original site transform from the instanced art bank.
"""
import argparse
from pathlib import Path
from finished_island_glb import parse, save


def create(src, dst):
    doc, binary = parse(src)
    scenes = doc["scenes"]
    if len(scenes) != 3:
        raise ValueError("Expected the Shellback bank's three artist levels.")
    shelves, nodes = [], []
    for i, level in enumerate(scenes):
        ids = []
        for j, src_id in enumerate(level["nodes"]):
            ref = doc["nodes"][src_id]
            mesh = ref["mesh"]
            name = doc["meshes"][mesh]["name"]
            ids.append(len(nodes))
            nodes.append(dict(name=name, mesh=mesh,
                              translation=[(j % 4) * 2.5, 0, (j // 4) * 2.5],
                              extras={"shelf_only": True, "botanical_lod": i}))
        shelves.append(dict(name=level["name"]+" / specimens only", nodes=ids))
    # The authored GLB writes all regular mesh data first, then the hidden
    # placement accessors. Keep only the actual core vertex/index stream prefix.
    used = {a for m in doc["meshes"] for p in m["primitives"]
            for a in (*p["attributes"].values(), p["indices"])}
    if used != set(range(max(used)+1)):
        raise ValueError("Not a contiguous mesh-only source; reindex before recasting.")
    acc = doc["accessors"][:max(used)+1]
    keep_views = {a["bufferView"] for a in acc}
    if keep_views != set(range(max(keep_views)+1)):
        raise ValueError("Unexpectedly interleaved placement bytes.")
    views = doc["bufferViews"][:max(keep_views)+1]
    end = max(v.get("byteOffset", 0)+v["byteLength"] for v in views)
    doc["scenes"], doc["nodes"] = shelves, nodes
    doc["accessors"], doc["bufferViews"] = acc, views
    doc["asset"]["generator"] = "Qualia Shellback uninstanced, local-origin specimen shelf"
    doc.pop("extensionsRequired")
    doc.pop("extensionsUsed")
    doc["buffers"] = [dict(byteLength=end)]
    save(doc, binary[:end], dst)


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("bank", type=Path)
    p.add_argument("shelf", type=Path)
    a = p.parse_args()
    create(a.bank, a.shelf)
