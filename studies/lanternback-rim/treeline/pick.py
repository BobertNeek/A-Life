"""A camera-space artist picking sheet: determine which real mesh draws a visible sliver."""

from pathlib import Path
import sys, math
import numpy as np
from PIL import Image

here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[2] / "scripts"))
from finished_island_glb import parse, values


def unit(a):
    return a / np.linalg.norm(a)


def draw(scene, rock, out):
    out.mkdir(parents=True, exist_ok=True)
    w, h = 630, 375
    eye = np.array([275.0, 73.0, -149.0])
    aim = np.array([242.5, 50.0, -170.9])
    v = unit(aim - eye)
    right = unit(np.cross(v, [0.0, 1.0, 0.0]))
    up = np.cross(right, v)
    depth = np.full((h, w), np.inf)
    ids = np.zeros((h, w), "H")
    caps = np.zeros((h, w), "B")
    nd = np.zeros((h, w), float)
    legend = {}
    index = 0
    for src, kind in [
        (scene / "ground.glb", "native"),
        (scene / "near.glb", "native"),
        (rock / "fitted-outcrops.glb", "cap"),
    ]:
        doc, raw = parse(src)
        for node in doc["nodes"]:
            idx = index + 1
            index = idx
            legend[idx] = node["name"]
            for p in doc["meshes"][node["mesh"]]["primitives"]:
                pos = np.asarray(
                    values(doc, raw, p["attributes"]["POSITION"])
                ) + np.array(node.get("translation", [0.0, 0.0, 0.0]))
                pp = pos - eye
                uv = np.column_stack(
                    [w * (0.5 + pp @ right / 58), h * 0.5 - (w / 58) * (pp @ up)]
                )
                dist = pp @ v
                if (
                    uv[:, 0].max() < 0
                    or uv[:, 0].min() >= w
                    or uv[:, 1].max() < 0
                    or uv[:, 1].min() >= h
                ):
                    continue
                indices = np.asarray(values(doc, raw, p["indices"]), int).reshape(-1, 3)
                for i, j, k in indices:
                    a, b, c = uv[i], uv[j], uv[k]
                    xs = np.array([a, b, c])
                    x0, x1 = max(0, math.floor(xs[:, 0].min())), min(
                        w, math.ceil(xs[:, 0].max())
                    )
                    y0, y1 = max(0, math.floor(xs[:, 1].min())), min(
                        h, math.ceil(xs[:, 1].max())
                    )
                    if x0 >= x1 or y0 >= y1:
                        continue
                    u = b - a
                    vv = c - a
                    det = u[0] * vv[1] - u[1] * vv[0]
                    if abs(det) < 1e-7:
                        continue
                    y, x = np.mgrid[y0:y1, x0:x1]
                    x = x + 0.5 - a[0]
                    y = y + 0.5 - a[1]
                    beta = (x * vv[1] - y * vv[0]) / det
                    gamma = (u[0] * y - u[1] * x) / det
                    dd = (
                        dist[i]
                        + beta * (dist[j] - dist[i])
                        + gamma * (dist[k] - dist[i])
                    )
                    q = depth[y0:y1, x0:x1]
                    ok = (
                        (beta >= 0)
                        & (gamma >= 0)
                        & (beta + gamma <= 1)
                        & (dd < q)
                        & (dd > 0)
                    )
                    q[ok] = dd[ok]
                    ids[y0:y1, x0:x1][ok] = idx
                    caps[y0:y1, x0:x1][ok] = (
                        2
                        if kind == "cap" and "Gneiss" in node["name"]
                        else 1 if kind == "cap" else 0
                    )
                    n = np.cross(pos[j] - pos[i], pos[k] - pos[i])
                    nd[y0:y1, x0:x1][ok] = n[1] / max(0.0001, np.linalg.norm(n))
    raw = np.asarray(
        Image.open(out.parent / "photos/before/spruce-lee.png")
        .convert("RGB")
        .resize((w, h)),
        float,
    )
    overlay = raw.copy()
    overlay[caps == 2] = raw[caps == 2] * 0.4 + np.array([190, 58, 34]) * 0.6
    overlay[caps == 1] = raw[caps == 1] * 0.4 + np.array([248, 178, 32]) * 0.6
    Image.fromarray(np.uint8(overlay)).save(out / "artist-picks.jpg", quality=93)
    Image.fromarray(caps).save(out / "cap-id.png")
    with (out / "legend.tsv").open("w") as f:
        for i in np.unique(ids):
            f.write(
                str(i)
                + "\t"
                + legend.get(i, "sky")
                + "\t"
                + str(int((ids == i).sum()))
                + "\n"
            )


if __name__ == "__main__":
    draw(*(Path(s) for s in sys.argv[1:4]))
