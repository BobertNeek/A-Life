"""Compact preview desk of the unchanged original V1 triangles, no Blender required.

Bake the original native habitat albedo, original vertex tint, previous Hollow
coat and optional rise coat to one small metric world-space sheet. This desk
does NOT claim the engine's mixed normals, view-dependent sea, or exact lights.
"""

import io, json, math, sys
from pathlib import Path
import numpy as np
from PIL import Image

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[2] / "scripts"))
from finished_island_glb import parse, values

CUT = (331.25, 418.75, -109.375, -56.25)


def linear(c):
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def srgb(c):
    return np.where(
        c <= 0.0031308, 12.92 * c, 1.055 * np.maximum(0, c) ** (1 / 2.4) - 0.055
    )


def read_image(path):
    return np.asarray(Image.open(path).convert("RGBA"))


def sample(img, u, v, wrap=False):
    if wrap:
        u = u % 1.0
        v = v % 1.0
    h, w = img.shape[:2]
    fx = u * w - 0.5
    fy = v * h - 0.5
    x = np.clip(np.floor(fx).astype("i"), 0, w - 1)
    y = np.clip(np.floor(fy).astype("i"), 0, h - 1)
    x1 = np.minimum(w - 1, x + 1)
    y1 = np.minimum(h - 1, y + 1)
    f = np.clip(fx - x, 0, 1).astype("f")[..., None]
    g = np.clip(fy - y, 0, 1).astype("f")[..., None]
    return (
        (img[y, x] * (1 - f) + img[y, x1] * f) * (1 - g)
        + (img[y1, x] * (1 - f) + img[y1, x1] * f) * g
    ) / 255.0


def glaze(rgb, img, x, z, box):
    a, b, c, d = box
    u = (x - a) / (b - a)
    v = (z - c) / (d - c)
    paint = sample(img, u, v)
    weight = paint[:, :, 3:] * ((u >= 0) & (u <= 1) & (v >= 0) & (v <= 1))[:, :, None]
    return rgb * (1 - weight) + linear(paint[:, :, :3]) * weight


def bake(scene, native, hollow, rise):
    doc, raw = parse(scene / "near.glb")
    a, b, c, d = CUT
    W, H = 720, 510
    uvmap = np.zeros((H, W, 2), "f")
    mult = np.ones((H, W, 3), "f")
    written = np.zeros((H, W), bool)
    positions = []
    normals = []
    uv = []
    indices = []
    for node in doc["nodes"]:
        prim = doc["meshes"][node["mesh"]]["primitives"][0]
        atts = prim["attributes"]
        pt = np.array(values(doc, raw, atts["POSITION"]), "f")
        nn = np.array(values(doc, raw, atts["NORMAL"]), "f")
        uu = np.array(values(doc, raw, atts["TEXCOORD_0"]), "f")
        cl = np.array(values(doc, raw, atts["COLOR_0"]), "f")[:, :3]
        tri = np.array(values(doc, raw, prim["indices"]), int).reshape(-1, 3)
        # Use complete original triangles only. The chosen cut sides are
        # exact 3.125 m native grid lines; no interpolated ground vertices.
        ok = (
            (pt[:, 0] >= a - 0.00001)
            & (pt[:, 0] <= b + 0.00001)
            & (pt[:, 2] >= c - 0.00001)
            & (pt[:, 2] <= d + 0.00001)
        )
        tri = tri[ok[tri].all(axis=1)]
        if not len(tri):
            continue
        used = np.unique(tri)
        offset = len(positions)
        remap = {int(k): i + offset for i, k in enumerate(used)}
        positions.extend(pt[used])
        normals.extend(nn[used])
        buf = np.stack([(pt[:, 0] - a) / (b - a), (pt[:, 2] - c) / (d - c)], axis=1)
        uv.extend(buf[used])
        buf = buf * np.array([W, H])
        for i, j, k in tri:
            indices.extend([remap[i], remap[j], remap[k]])
            xy = buf[[i, j, k]]
            x0, y0 = np.maximum(0, np.floor(xy.min(0)).astype(int))
            x1, y1 = np.minimum([W, H], np.ceil(xy.max(0)).astype(int) + 1)
            if x1 <= x0 or y1 <= y0:
                continue
            Y, X = np.mgrid[y0:y1, x0:x1]
            t = xy[1] - xy[0]
            q = xy[2] - xy[0]
            det = t[0] * q[1] - t[1] * q[0]
            if abs(det) < 1e-8:
                continue
            xx = X + 0.5 - xy[0, 0]
            yy = Y + 0.5 - xy[0, 1]
            beta = (xx * q[1] - yy * q[0]) / det
            gamma = (t[0] * yy - t[1] * xx) / det
            alpha = 1 - beta - gamma
            mask = (alpha >= -1e-7) & (beta >= -1e-7) & (gamma >= -1e-7)
            v = np.stack([alpha, beta, gamma], -1)
            uvmap[y0:y1, x0:x1][mask] = (v @ uu[[i, j, k]])[mask]
            mult[y0:y1, x0:x1][mask] = (v @ cl[[i, j, k]])[mask]
            written[y0:y1, x0:x1][mask] = True
    if written.mean() < 0.997:
        raise ValueError(
            "Native habitat cut does not cover desk; do not fill with made-up ground"
        )
    atlas = read_image(native / "habitats/substrates.png")
    old = read_image(hollow / "hollow-ground.png")
    new = read_image(rise / "wind-duff.png")
    hb = json.loads((hollow / "site.json").read_text())["bounds"]
    rb = json.loads((rise / "wind-duff.json").read_text())["plan"]
    left = np.zeros((H, W, 3), "B")
    right = left.copy()
    for i in range(0, H, 32):
        end = min(H, i + 32)
        u = uvmap[i:end]
        col = linear(sample(atlas, u[:, :, 0], u[:, :, 1])[:, :, :3]) * mult[i:end]
        X, Z = np.meshgrid(
            a + (np.arange(W) + 0.5) * (b - a) / W,
            c + (np.arange(i, end) + 0.5) * (d - c) / H,
        )
        base = glaze(col, old, X, Z, hb)
        candidate = glaze(base, new, X, Z, rb)
        left[i:end] = np.uint8(np.clip(np.rint(255 * srgb(base)), 0, 255))
        right[i:end] = np.uint8(np.clip(np.rint(255 * srgb(candidate)), 0, 255))

    def png(x):
        dest = io.BytesIO()
        Image.fromarray(x).save(dest, format="PNG", optimize=True)
        return dest.getvalue()

    return (
        [
            np.array(positions, "<f4"),
            np.array(normals, "<f4"),
            np.array(uv, "<f4"),
            np.array(indices, "<u2"),
        ],
        (png(left), png(right)),
        dict(
            bounds=CUT,
            unchanged_native_triangles=len(indices) // 3,
            retained_original_vertices=len(positions),
            atlas_pixels=[W, H],
            filled_percentage=round(100 * written.mean(), 5),
            previous_hollow_coat=True,
            portable_colour_desk_not_pbr_match=True,
        ),
    )
