"""True crest visibility from possible body-height framing stations.

Survey the original registered props as well as V1. Body-height camera choice is
an artwork decision, not a player path. For masked crowns use the actual alpha
cutout, not their much larger collider or a made-up cone.
"""

from pathlib import Path
import json, math, sys, io
import numpy as np
from PIL import Image

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H))
sys.path.insert(0, str(H.parents[1] / "scripts"))
from profile import cast
from island_inlet_ground import Inlet
from finished_island_glb import parse, values


def scout(native, out):
    land = Inlet(native / "island-v1.bin")
    props = json.loads((native / "props.json").read_text())
    forms = json.loads((native / "grove-crowns.json").read_text())
    src = {}
    meshes = {}
    bits = {}

    def source(kind, name):
        key = (kind, name)
        if key in meshes:
            return meshes[key]
        if kind not in src:
            src[kind] = parse(native / (kind + ".glb"))
        d, r = src[kind]
        node = next(v for v in d["nodes"] if v["name"] == name)
        result = []
        for p in d["meshes"][node["mesh"]]["primitives"]:
            a = p["attributes"]
            v = np.array(values(d, r, a["POSITION"]), "f")
            ii = np.array(values(d, r, p["indices"]), int).reshape(-1, 3)
            tex = None
            m = d["materials"][p["material"]]
            if m.get("alphaMode") == "MASK":
                t = m["pbrMetallicRoughness"]["baseColorTexture"]["index"]
                im = d["images"][d["textures"][t]["source"]]
                tip = (kind, t)
                uv = np.array(values(d, r, a["TEXCOORD_0"]), "f")
                if tip not in bits:
                    if "uri" in im:
                        b = Image.open(native / im["uri"])
                    else:
                        bv = d["bufferViews"][im["bufferView"]]
                        i = bv.get("byteOffset", 0)
                        b = Image.open(io.BytesIO(r[i : i + bv["byteLength"]]))
                    bits[tip] = np.asarray(b.convert("RGBA"))[:, :, 3]
                tex = (uv[ii], bits[tip], m.get("alphaCutoff", 0.5))
            result.append((v[ii], tex))
        meshes[key] = result
        return result

    def hits(start, end):
        start = np.array(start)
        end = np.array(end)
        v = end - start
        ln = np.linalg.norm(v)
        direction = v / ln
        ax = start[[0, 2]]
        aim = v[[0, 2]]
        den = np.dot(aim, aim)
        out = []
        for i, p in enumerate(props):
            pos = np.array(p["position"])
            t = np.clip(np.dot(pos[[0, 2]] - ax, aim) / den, 0, 1)
            dr = np.linalg.norm(pos[[0, 2]] - ax - aim * t)
            scale = p["scale"]
            kind = p["kind"]
            rad = 12 * scale if kind == "Cliff" else 7 * scale
            if dr > rad or abs((start + v * t)[1] - pos[1]) > 22 * scale:
                continue
            form = forms.get(str(i)) if kind == "Conifer" else None
            lib = "grove-crowns" if form else kind
            name = "PineForm" + str(form["form"]) if form else kind
            # Head/crown rays right beside the eye use L0; far forest uses
            # the published L1/L2 geometry. Shrub widths are not colliders.
            dist = np.linalg.norm(pos - start)
            layer = 0 if dist < 55 else (1 if dist < 130 else 2)
            co, si = math.cos(p["yaw"]), math.sin(p["yaw"])
            R = np.array([[co, 0, si], [0, 1, 0], [-si, 0, co]])
            best = float("inf")
            for tris, tex in source(lib, name + "_L" + str(layer)):
                tri = (tris * scale) @ R.T + pos
                a, b, c = tri[:, 0], tri[:, 1], tri[:, 2]
                u, vv = b - a, c - a
                cross = np.cross(np.broadcast_to(direction, vv.shape), vv)
                det = np.einsum("ij,ij->i", u, cross)
                keep = abs(det) > 1e-9
                f = np.divide(1, det, out=np.zeros_like(det), where=keep)
                s = start - a
                beta = f * np.einsum("ij,ij->i", s, cross)
                q = np.cross(s, u)
                gam = f * (q @ direction)
                dd = f * np.einsum("ij,ij->i", vv, q)
                ids = np.flatnonzero(
                    keep
                    & (beta >= 0)
                    & (gam >= 0)
                    & (beta + gam <= 1)
                    & (dd > 0.06)
                    & (dd < ln - 0.02)
                )
                if not len(ids):
                    continue
                if tex is not None:
                    uu, mask, th = tex
                    uv = (
                        (1 - beta[ids, None] - gam[ids, None]) * uu[ids, 0]
                        + beta[ids, None] * uu[ids, 1]
                        + gam[ids, None] * uu[ids, 2]
                    )
                    hh, ww = mask.shape
                    sy = np.minimum(hh - 1, ((uv[:, 1] % 1.0) * hh).astype(int))
                    sx = np.minimum(ww - 1, ((uv[:, 0] % 1.0) * ww).astype(int))
                    ids = ids[mask[sy, sx] >= 255 * th]
                if len(ids):
                    best = min(best, float(dd[ids].min()))
            if best < float("inf"):
                out.append([i, kind, round(best, 3)])
        return sorted(out, key=lambda a: a[2])

    # All candidate eyes are on the documented genuine high terrace.
    starts = [
        (82.0, -408.0),
        (80.0, -413.0),
        (85.25, -416.0),
        (86.62, -404.75),
        (72.75, -414.5),
    ]
    summit = (28.125, 240.0, -275.0)
    f = [
        ("true summit", summit),
        ("left spur", (3.0, float(land.floor(3.0, -300.0)[0]), -300.0)),
        ("near pine shoulder", (62.0, float(land.floor(62.0, -373.0)[0]), -373.0)),
    ]
    rr = []
    for x, z in starts:
        y, dx, dz = map(float, land.floor(x, z))
        eye = [x, y + 1.04, z]
        r = dict(eye=eye, ground_grade=round(math.hypot(dx, dz), 3), rays={})
        for label, target in f:
            r["rays"][label] = dict(
                target=target,
                terrain=cast(land, eye, target),
                prop_hits=hits(eye, target),
            )
        rr.append(r)
    (out / "sightlines.json").write_text(
        json.dumps(
            dict(
                method="Existing original terrain and triangle alpha-mask rays, not a rendered image or navigation proof",
                stations=rr,
            ),
            indent=2,
        )
        + "\n"
    )
    return [
        (
            r["eye"],
            r["rays"]["true summit"]["prop_hits"],
            r["rays"]["left spur"]["prop_hits"],
        )
        for r in rr
    ]


if __name__ == "__main__":
    print(scout(*(Path(p) for p in sys.argv[1:])))
