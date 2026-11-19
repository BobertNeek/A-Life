"""A compact read-only view of the archived native terrain; don't import 360 unused parts."""

from copy import deepcopy
from pathlib import Path
import math, sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
from finished_island_glb import parse, save


def subset(src, dest, pick):
    d, raw = parse(src)
    nodes = []
    meshes = []
    acc = []
    views = []
    buf = bytearray()
    seen = {}
    seenacc = {}

    def view(k):
        if k not in seen:
            old = d["bufferViews"][k]
            pad = (-len(buf)) % 4
            buf.extend(b"\0" * pad)
            v = dict(old)
            v["byteOffset"] = len(buf)
            start = old.get("byteOffset", 0)
            buf.extend(raw[start : start + old["byteLength"]])
            seen[k] = len(views)
            views.append(v)
        return seen[k]

    def accessor(k):
        if k not in seenacc:
            a = deepcopy(d["accessors"][k])
            a["bufferView"] = view(a["bufferView"])
            seenacc[k] = len(acc)
            acc.append(a)
        return seenacc[k]

    for item in d["nodes"]:
        if not pick(item["name"]):
            continue
        node = dict(item)
        m = deepcopy(d["meshes"][node["mesh"]])
        node["mesh"] = len(meshes)
        for p in m["primitives"]:
            p["indices"] = accessor(p["indices"])
            p["attributes"] = {k: accessor(a) for k, a in p["attributes"].items()}
        meshes.append(m)
        nodes.append(node)
    for im in d.get("images", []):
        if "bufferView" in im:
            im["bufferView"] = view(im["bufferView"])
    d.update(
        meshes=meshes,
        nodes=nodes,
        scenes=[dict(nodes=list(range(len(nodes))))],
        scene=0,
        accessors=acc,
        bufferViews=views,
        buffers=[dict(byteLength=len(buf))],
    )
    save(d, buf, dest)
    return len(nodes)


def stage(native, out):
    out.mkdir(exist_ok=True, parents=True)
    for name in ["geology", "habitats"]:
        if not (out / name).exists():
            (out / name).symlink_to((native / name).resolve(), target_is_directory=True)

    def local(i, j):
        return 145 < (-450 + i * 100) < 355 and -250 <= (-550 + j * 100) <= -50

    hd, _ = parse(native / "habitat-ground.glb")
    hab = {tuple(map(int, n["name"].split("_")[1:])) for n in hd["nodes"]}

    def chosen(n):
        _, a, b, l = n.split("_")
        i, j = int(a), int(b)
        cx, cz = -450 + 100 * i, -550 + 100 * j
        dist = math.hypot(cx - 249, cz + 123)
        if local(i, j) and (i, j) in hab:
            return False
        return l == ("L0" if local(i, j) else "L1" if dist < 185 else "L2")

    subset(native / "terrain-chunks.glb", out / "ground.glb", chosen)
    return subset(
        native / "habitat-ground.glb",
        out / "near.glb",
        lambda n: local(*map(int, n.split("_")[1:])),
    )


if __name__ == "__main__":
    print(stage(Path(sys.argv[1]), Path(sys.argv[2])))
