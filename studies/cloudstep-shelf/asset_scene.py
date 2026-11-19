"""Stream original V1 slope chunks, sky, stock forest floor and located source props.
Reader deliberately avoids loading the large complete island or any simulator.
"""

import json, struct, mmap, io, math
from pathlib import Path
from collections import defaultdict
import numpy as np
from PIL import Image


def unit(a):
    return a / np.maximum(1e-12, np.linalg.norm(a, axis=-1, keepdims=True))


class Source:
    def __init__(self, p):
        self.file = Path(p).open("rb")
        self.path = Path(p)
        self.mm = mmap.mmap(self.file.fileno(), 0, access=mmap.ACCESS_READ)
        (lens,) = struct.unpack_from("<I", self.mm, 12)
        self.doc = json.loads(self.mm[20 : 20 + lens])
        self.start = 28 + lens
        self.images = {}

    def read(self, i):
        d = self.doc
        a = d["accessors"][i]
        b = d["bufferViews"][a["bufferView"]]
        k = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[a["type"]]
        typ = np.dtype(
            {5126: "<f4", 5125: "<u4", 5123: "<u2", 5121: "u1"}[a["componentType"]]
        )
        stride = b.get("byteStride", typ.itemsize * k)
        v = np.ndarray(
            (a["count"], k),
            typ,
            buffer=self.mm,
            offset=self.start + b.get("byteOffset", 0) + a.get("byteOffset", 0),
            strides=(stride, typ.itemsize),
        )
        return v.astype("f") / np.iinfo(typ).max if a.get("normalized") else v

    def image(self, link):
        d = self.doc
        t = d["textures"][link["index"]]
        a = d["images"][t["source"]]
        if t["source"] not in self.images:
            if "uri" in a:
                im = Image.open(self.path.parent / a["uri"])
            else:
                v = d["bufferViews"][a["bufferView"]]
                start = self.start + v.get("byteOffset", 0)
                im = Image.open(io.BytesIO(self.mm[start : start + v["byteLength"]]))
            # Limit in-memory texture footprint, keep actual texels. Do not
            # downsample narrow cutout atlas sources used for the native crowns.
            im.thumbnail((1536, 1536), Image.Resampling.LANCZOS)
            self.images[t["source"]] = np.asarray(im.convert("RGBA"))
        s = d.get("samplers", [{}])[t.get("sampler", 0)]
        return (self.images[t["source"]], s.get("wrapS", 10497), s.get("wrapT", 10497))

    def parts(self, node, rot, scale, position, shadow=False):
        for prim in self.doc["meshes"][node["mesh"]]["primitives"]:
            a = prim["attributes"]
            v = self.read(a["POSITION"])
            mat = self.doc["materials"][prim.get("material", 0)]
            pbr = mat["pbrMetallicRoughness"]
            cut = mat.get("alphaCutoff", 0.5) if mat.get("alphaMode") == "MASK" else 0.0
            link = pbr.get("baseColorTexture")
            tex = self.image(link) if link and (not shadow or cut) else None
            uv = (
                self.read(a["TEXCOORD_" + str(link.get("texCoord", 0))])
                if tex is not None
                else np.zeros((len(v), 2), "f")
            )
            norm = (
                (self.read(a["NORMAL"]) @ rot.T)
                if "NORMAL" in a
                else np.broadcast_to([0.0, 1.0, 0.0], v.shape)
            )
            colour = (
                self.read(a["COLOR_0"])[:, :3] if "COLOR_0" in a else np.ones_like(v)
            )
            factor = np.array(pbr.get("baseColorFactor", [1, 1, 1, 1])[:3], "f")
            tr = self.read(prim["indices"]).reshape(-1, 3)
            yield (
                v @ rot.T * scale + position,
                norm,
                colour * factor,
                uv,
                tr,
                tex,
                cut,
            )

    def close(self):
        self.images.clear()
        self.file.close()
        # Accessor views may still be alive in a consumer: release through
        # normal mapping GC rather than attempting to invalidate a live view.


def qrot(q):
    x, y, z, w = q
    return np.array(
        [
            [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
        ],
        "f",
    )


def traverse(draw, native, copse, folder, with_cones=False, shoulders=False):
    eye = draw.eye
    ident = np.eye(3, dtype="f")
    zero = np.zeros(3, "f")
    seen = set()

    def node(s, n, rot=ident, sc=1.0, pos=zero, tag="solid", is_ext=False):
        if "mesh" not in n:
            return
        d = s.doc
        pr = d["meshes"][n["mesh"]]["primitives"][0]
        meta = d["accessors"][pr["attributes"]["POSITION"]]
        centre = ((np.array(meta["min"]) + meta["max"]) * 0.5) @ rot.T * sc + pos
        r = 0.5 * sc * np.linalg.norm(np.array(meta["max"]) - meta["min"])
        if not draw.admits(centre, r):
            return
        draw.subject = (
            s.path.name,
            n.get("name", ""),
            [round(float(a), 2) for a in pos],
        )
        for part in s.parts(n, rot, sc, pos, draw.shadow):
            draw.send(*part, tag)

    # Where the art stage has L0 habitat tiles, do not mount a second chunk.
    for filename in ("habitat-ground", "terrain-chunks"):
        s = Source(native / (filename + ".glb"))
        for n in s.doc["nodes"]:
            name = n["name"].split("_")
            i, j = map(int, name[1:3])
            hi = 4 <= i <= 6 and 1 <= j <= 3
            if filename == "habitat-ground":
                if not hi:
                    continue
                seen.add((i, j))
            else:
                if (i, j) in seen:
                    continue
                dd = math.hypot(-450 + 100 * i - 72, -550 + 100 * j + 375)
                if name[3] != ("L0" if hi else "L1" if dd < 230 else "L2"):
                    continue
            node(s, n, tag="ground")
        s.close()
    s = Source(copse)
    for n in s.doc["nodes"]:
        node(s, n, pos=np.asarray(n.get("translation", [0, 0, 0]), "f"), tag="copse")
    s.close()
    props = json.loads((native / "props.json").read_text())
    forms = json.loads((native / "grove-crowns.json").read_text())
    sets = defaultdict(list)
    for i, p in enumerate(props):
        if shoulders and i in (1335, 1695, 1847):
            continue
        x, y, z = p["position"]
        ds = math.hypot(x - 71.0, z + 379.0)
        k = p["kind"]
        if ds > (
            265
            if k in ("Conifer", "Broadleaf", "Sapling")
            else 185 if k.startswith("Boulder") or k in ("Cliff", "Scree") else 115
        ):
            continue
        crown = forms.get(str(i)) if k == "Conifer" else None
        asset = "grove-crowns" if crown else k
        lod = 0 if ds < 70 else 1 if ds < 156 else 2
        sets[asset].append(
            (("PineForm" + str(crown["form"]) if crown else k) + "_L" + str(lod), p)
        )
    for asset, placements in sets.items():
        s = Source(native / (asset + ".glb"))
        nodes = {n["name"]: n for n in s.doc["nodes"]}
        for name, p in placements:
            ang = p["yaw"]
            c = math.cos(ang)
            t = math.sin(ang)
            R = np.array([[c, 0, t], [0, 1, 0], [-t, 0, c]], "f")
            node(s, nodes[name], R, p["scale"], np.asarray(p["position"], "f"), "prop")
        s.close()
    if shoulders:
        s = Source(folder / ("slope-shoulders-L" + str(draw.shoulder_lod) + ".glb"))
        for n in s.doc["nodes"]:
            node(
                s,
                n,
                qrot(n["rotation"]),
                n["scale"][0],
                np.array(n["translation"], "f"),
                "shoulder",
            )
        s.close()
    if not draw.shadow:
        s = Source(native / "marine-day.glb")
        for n in s.doc["nodes"]:
            node(s, n, tag="sky")
        s.close()
    if not draw.shadow and draw.water is not None:
        cfg=json.loads((draw.water/"water.json").read_text())
        water=np.asarray(Image.open(draw.water/"shore.png").convert("RGBA"))
        points=np.array([[-3000,0,-3000],[3000,0,-3000],[3000,0,3000],[-3000,0,3000]],"f")
        uv=(points[:,(0,2)]-cfg["origin_xz"])/cfg["span_xz"]
        draw.subject=("y0 ocean","Original V1 water artwork",[0.,0.,0.])
        draw.send(points,np.tile([0.,1.,0.],(4,1)),np.ones((4,3),"f"),uv,
                  np.array([[0,2,1],[0,3,2]]), (water,33071,33071),0,'ocean')
    if with_cones:
        s = Source(folder / "cone-husks-eye.glb")
        for n in s.doc["nodes"]:
            a = n["extensions"]["EXT_mesh_gpu_instancing"]["attributes"]
            for p, q, k in zip(
                s.read(a["TRANSLATION"]), s.read(a["ROTATION"]), s.read(a["SCALE"])
            ):
                node(s, n, qrot(q), k[0], p, "cone")
        s.close()
