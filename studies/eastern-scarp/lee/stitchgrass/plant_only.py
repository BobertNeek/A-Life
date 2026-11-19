"""Pack only the twelve new plant forms and their native-world instance streams.
No duplicated input terrain, conifer, water, old Hollow stem or image.
"""

from pathlib import Path
import copy
from finished_island_glb import save


def extract(source, raw, rows, out):
    g = dict(
        asset={"version": "2.0", "generator": "Twin Needle / native-root artist kit"},
        scene=0,
        scenes=[{"nodes": []}],
        nodes=[],
        meshes=[],
        accessors=[],
        bufferViews=[],
        materials=[],
        extensionsUsed=["EXT_mesh_gpu_instancing"],
        extensionsRequired=["EXT_mesh_gpu_instancing"],
    )
    buf = bytearray()
    cache = {}
    mv = {}

    def get(i):
        if i in cache:
            return cache[i]
        a = copy.deepcopy(source["accessors"][i])
        v = copy.deepcopy(source["bufferViews"][a["bufferView"]])
        old = v.get("byteOffset", 0)
        buf.extend(b"\x00" * (-len(buf) % 4))
        v["byteOffset"] = len(buf)
        buf.extend(raw[old : old + v["byteLength"]])
        a["bufferView"] = len(g["bufferViews"])
        g["bufferViews"].append(v)
        i2 = len(g["accessors"])
        g["accessors"].append(a)
        cache[i] = i2
        return i2

    for i in rows:
        node = copy.deepcopy(source["nodes"][i])
        mesh = copy.deepcopy(source["meshes"][node["mesh"]])
        for p in mesh["primitives"]:
            mi = p["material"]
            if mi not in mv:
                mv[mi] = len(g["materials"])
                g["materials"].append(copy.deepcopy(source["materials"][mi]))
            p["material"] = mv[mi]
            p["indices"] = get(p["indices"])
            p["attributes"] = {k: get(v) for k, v in p["attributes"].items()}
        ext = node["extensions"]["EXT_mesh_gpu_instancing"]
        ext["attributes"] = {k: get(v) for k, v in ext["attributes"].items()}
        node["mesh"] = len(g["meshes"])
        g["meshes"].append(mesh)
        g["scenes"][0]["nodes"].append(len(g["nodes"]))
        g["nodes"].append(node)
    g["buffers"] = [{"byteLength": len(buf)}]
    save(g, buf, out)
