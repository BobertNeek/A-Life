"""Replace broad cosmetic roofs, never the island underneath them.

The guardian pick showed the long straight marks on the old Gneiss primitives.
Preserve the old apron bit-for-bit. Represent broad bed colour on the existing
ground, with short individually bedded narrow chips as the physical accents.
"""

import sys, json, csv, math
from pathlib import Path
from collections import defaultdict
import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parent), str(H.parents[2] / "scripts")]
from strata import Rim, BOX, nz
from fissures import Piece, export
from layout import subset
from finished_island_glb import parse, save, values


def palette(b, held, out):
    col = np.array(Image.open(held / "rim-ground.png").convert("RGBA"))
    hh, ww = col.shape[:2]
    a, bx, c, d = BOX
    dr = Image.new("L", (ww, hh))
    pen = ImageDraw.Draw(dr)
    dd, raw = parse(held / "fitted-outcrops.glb")
    for n in dd["nodes"]:
        if "Gneiss" not in n["name"]:
            continue
        p = dd["meshes"][n["mesh"]]["primitives"][0]
        v = np.asarray(values(dd, raw, p["attributes"]["POSITION"])) + np.asarray(
            n["translation"]
        )
        ij = np.asarray(values(dd, raw, p["indices"]), int).reshape(-1, 3)
        heights = v[:, 1] - b.floor(v[:, 0], v[:, 2])[0]
        uv = np.column_stack(
            ((v[:, 0] - a) * ww / (bx - a), (v[:, 2] - c) * hh / (d - c))
        )
        # Never trace a polygonal roof as a pigment boundary: this intermediary
        # gets diffused at two geological scales before it can reach the patch.
        for t in ij:
            f = np.clip((heights[t].mean() + 0.007) / 0.045, 0, 1)
            if f > 0.04:
                pen.polygon([tuple(pt) for pt in uv[t]], fill=int(66 + 189 * f))
    source = np.asarray(dr, dtype="f") / 255
    fine = gaussian_filter(source, 4.4)
    wide = gaussian_filter(source, 11.8)
    x, z = np.meshgrid(
        a + (np.arange(ww) + 0.5) * (bx - a) / ww,
        c + (np.arange(hh) + 0.5) * (d - c) / hh,
    )
    grain = nz(x, z, 0.17, 81)
    var = nz(x, z, 1.1, 42)
    strike = nz(0.66 * x + 0.42 * z, 0.23 * z - 0.57 * x, 0.67, 51)
    current = col[:, :, :3].astype(float)
    k = np.clip(0.32 * wide + 0.43 * fine, 0, 0.65)
    # Remain in the same earth/mineral family. No blue polygon silhouettes.
    mineral = (
        np.array([140.0, 139.0, 122.0])
        + ((grain - 0.5) * 12 + (var - 0.5) * 18 + (strike - 0.5) * 15)[..., None]
    )
    col[:, :, :3] = np.uint8(
        np.clip(current * (1 - k[..., None]) + mineral * k[..., None], 0, 255)
    )
    Image.fromarray(col).save(out / "rim-ground.png", optimize=True)
    return fine


def stones(b, held, out, field):
    rng = np.random.default_rng(731)
    grp = defaultdict(Piece)
    count = 0
    maxl = 0
    a, bx, c, d = BOX
    hh, ww = field.shape
    for q in csv.DictReader((held / "outcrop-ledger.csv").open()):
        cx, cz = float(q["x"]), float(q["z"])
        span = float(q["half_length_m"])
        for _ in range(int(rng.integers(8, 16))):
            x = cx + rng.normal(0, span * 0.7)
            z = cz + rng.normal(0, span * 0.46)
            u = int((x - a) * ww / (bx - a))
            v = int((z - c) * hh / (d - c))
            if (
                not (0 <= u < ww and 0 <= v < hh)
                or field[v, u] < 0.20
                or b.blocked(x, z, 0.65)
            ):
                continue
            _, gx, gz = b.floor(x, z)
            theta = 0.4 + 0.63 * float(nz(x, z, 19, 44)) + rng.normal(0, 0.32)
            along = np.array([math.cos(theta), math.sin(theta)])
            cross = np.array([-along[1], along[0]])
            L = rng.uniform(0.22, 1.03)
            W = rng.uniform(0.045, 0.14)
            T = rng.uniform(0.012, 0.042)
            bend = rng.uniform(-0.06, 0.06)
            cloud = rng.uniform(0.91, 1.03)
            pts = []
            color = []
            tri = []
            for i, t in enumerate((0.0, 0.18, 0.46, 0.73, 1.0)):
                g = math.sin(math.pi * t)
                center = np.array([x, z]) + (t - 0.5) * L * along + cross * bend * g
                for j, (offset, high) in enumerate(
                    ((-1, -0.003), (-0.31, 0.41), (0.02, 1), (0.52, 0.42), (1, -0.005))
                ):
                    xz = center + cross * W * offset * (0.035 + g)
                    floor = b.floor(*xz)[0]
                    rise = high if high < 0 else high * T * g - 0.004
                    pts.append([xz[0], floor + rise, xz[1]])
                    color.append(
                        np.array([150, 150, 131])
                        * cloud
                        * (0.86 + 0.12 * (j in (1, 2)) + 0.06 * g)
                    )
                if i:
                    for j in range(4):
                        k = (i - 1) * 5 + j
                        tri.extend(((k, k + 1, k + 6), (k, k + 6, k + 5)))
            tag = "Bedded splinters %d-%d" % (int((x - a) // 27), int((z - c) // 26))
            grp[tag].face(np.array(pts), np.array(color), tri)
            count += 1
            maxl = max(L, maxl)
    stats = export(grp, out / "splinters.glb", "Short grain-bound gneiss exhumations")
    return count, maxl, stats


def combine(held, out):
    subset(
        held / "fitted-outcrops.glb",
        out / "apron.glb",
        lambda n: n.startswith("Fine apron"),
    )
    d, base = parse(out / "apron.glb")
    e, src = parse(out / "splinters.glb")
    off = len(base)
    base.extend(src)
    bv = len(d["bufferViews"])
    ac = len(d["accessors"])
    mesh = len(d["meshes"])
    node = len(d["nodes"])
    for v in e["bufferViews"]:
        v["byteOffset"] = v.get("byteOffset", 0) + off
    for a in e["accessors"]:
        a["bufferView"] += bv
    for m in e["meshes"]:
        for p in m["primitives"]:
            p["attributes"] = {k: v + ac for k, v in p["attributes"].items()}
            p["indices"] += ac
    for n in e["nodes"]:
        n["mesh"] += mesh
    for key in ["bufferViews", "accessors", "meshes", "nodes"]:
        d[key].extend(e[key])
    d["scenes"][0]["nodes"].extend(range(node, node + len(e["nodes"])))
    d["buffers"][0]["byteLength"] = len(base)
    save(d, base, out / "fitted-outcrops.glb")


def main(native, held, prior, out):
    out.mkdir(exist_ok=True, parents=True)
    r = Rim(native / "island-v1.bin")
    field = palette(r, held, out)
    num, maxl, stats = stones(r, held, out, field)
    combine(held, out)
    for src in prior.iterdir():
        p = out / src.name
        if src.is_file() and not p.exists():
            p.symlink_to(src.resolve())
    (out / "bedding.json").write_text(
        json.dumps(
            dict(
                source="237 broad roof primitives removed; true V1 unchanged",
                retained_embedded_clasts=616,
                short_rakes=num,
                longest_m=maxl,
                extra=stats,
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    main(*(Path(a) for a in sys.argv[1:5]))
