"""Small CPU illustration of the real two-state GLB, no Blender/native game or fabricated backdrop.

A portable field editor may not have a heavyweight PBR renderer available. This
rasterises the actual camera, triangles, rooted instancing and base colours,
with a simple directional studio diffuse. It does not emulate HDRP/Bevy, sky,
contact shadows, wind or the normal maps. Use it as an assembly illustration.
"""

import io, json, sys, math
from pathlib import Path
from PIL import Image
import numpy as np

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[2] / "scripts"))
from finished_island_glb import parse, values


def rot(q):
    x, y, z, w = q
    return np.array(
        [
            [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
        ],
        "f",
    )


def unit(v):
    return v / np.maximum(1e-8, np.linalg.norm(v, axis=-1, keepdims=True))


def lin(v):
    return np.where(v <= 0.04045, v / 12.92, ((v + 0.055) / 1.055) ** 2.4)


def srgb(v):
    return np.where(
        v <= 0.0031308, v * 12.92, 1.055 * np.maximum(0, v) ** (1 / 2.4) - 0.055
    )


def picture(p, scene_index, cam_index, out):
    d, raw = parse(p)
    W, H = 720, 428
    facebuf = np.full((H, W), np.inf, "f")
    rgb = np.empty((H, W, 3), "B")
    rgb[:] = [209, 211, 202]
    accessor = {}

    def ac(i):
        if i not in accessor:
            accessor[i] = np.array(values(d, raw, i), "f")
        return accessor[i]

    textures = {}

    def material(m):
        desc = d["materials"][m]
        mat = desc["pbrMetallicRoughness"]
        t = mat.get("baseColorTexture", {}).get("index")
        if t is not None and t not in textures:
            im = d["images"][d["textures"][t]["source"]]
            v = d["bufferViews"][im["bufferView"]]
            i = v.get("byteOffset", 0)
            textures[t] = np.asarray(
                Image.open(io.BytesIO(raw[i : i + v["byteLength"]])).convert("RGBA")
            )
        cutoff = desc.get("alphaCutoff", 0.5) if desc.get("alphaMode") == "MASK" else 0
        return t, cutoff

    cameras = [v for v in d["nodes"] if "camera" in v]
    cam = cameras[cam_index]
    k = d["cameras"][cam["camera"]]
    R = rot(cam["rotation"])
    eye = np.array(cam["translation"], "f")
    persp = k["type"] == "perspective"
    if persp:
        f = H / (2 * math.tan(k["perspective"]["yfov"] / 2))
    else:
        f = W / (2 * k["orthographic"]["xmag"])
    sun = unit(np.array([-0.69, 0.66, 0.30], "f"))

    def paint(vv, screen, idx, texture):
        # vv = camera x,y,positive depth; lit RGB; original UV.
        uv = screen[idx]
        v = vv[idx]
        lo = np.floor(uv.min(0)).astype(int)
        hi = np.ceil(uv.max(0)).astype(int)
        x0, y0 = max(0, lo[0]), max(0, lo[1])
        x1, y1 = min(W - 1, hi[0]), min(H - 1, hi[1])
        if x1 < x0 or y1 < y0:
            return
        u = uv[1] - uv[0]
        w = uv[2] - uv[0]
        det = u[0] * w[1] - u[1] * w[0]
        if abs(det) < 0.20:
            return
        # Tiny art triangles are sampled cheaply using their stochastic-free
        # centre, never inflated into correctly-looking but invented foliage.
        x = np.arange(x0, x1 + 1, dtype="f")[None, :] + 0.5 - uv[0, 0]
        y = np.arange(y0, y1 + 1, dtype="f")[:, None] + 0.5 - uv[0, 1]
        beta = (x * w[1] - y * w[0]) / det
        gamma = (u[0] * y - u[1] * x) / det
        alpha = 1 - beta - gamma
        inside = (alpha >= 0) & (beta >= 0) & (gamma >= 0)
        if not inside.any():
            return
        ys, xs = np.nonzero(inside)
        a = alpha[ys, xs]
        b = beta[ys, xs]
        c = gamma[ys, xs]
        if persp:
            a = a / v[0, 2]
            b = b / v[1, 2]
            c = c / v[2, 2]
            depth = 1 / (a + b + c)
            a *= depth
            b *= depth
            c *= depth
        else:
            depth = a * v[0, 2] + b * v[1, 2] + c * v[2, 2]
        ys += y0
        xs += x0
        keep = depth < facebuf[ys, xs]
        if not keep.any():
            return
        ys = ys[keep]
        xs = xs[keep]
        depth = depth[keep]
        a = a[keep, None]
        b = b[keep, None]
        c = c[keep, None]
        col = a * v[0, 3:6] + b * v[1, 3:6] + c * v[2, 3:6]
        texture, cutoff = texture
        if texture is not None:
            uv = a * v[0, 6:8] + b * v[1, 6:8] + c * v[2, 6:8]
            pic = textures[texture]
            hh, ww = pic.shape[:2]
            # Artist GLB uses glTF's default repeat and linear magnification.
            # Respect real crown cutouts before committing visibility.
            xy = uv * np.array([ww, hh], "f") - 0.5
            low = np.floor(xy).astype("i")
            f = xy - low
            u0, v0 = low[:, 0] % ww, low[:, 1] % hh
            u1, v1 = (u0 + 1) % ww, (v0 + 1) % hh
            fx, fy = f[:, :1], f[:, 1:]

            # Interpolate source texels in linear light, not sRGB.
            def texel(iy, ix):
                a = pic[iy, ix].astype("f") / 255
                a[:, :3] = lin(a[:, :3])
                return a

            sample = (1 - fy) * ((1 - fx) * texel(v0, u0) + fx * texel(v0, u1)) + fy * (
                (1 - fx) * texel(v1, u0) + fx * texel(v1, u1)
            )
            if cutoff:
                shows = sample[:, 3] >= cutoff
                ys, xs, depth, col, sample = (
                    ys[shows],
                    xs[shows],
                    depth[shows],
                    col[shows],
                    sample[shows],
                )
            col *= sample[:, :3]
        facebuf[ys, xs] = depth
        rgb[ys, xs] = np.uint8(np.clip(255 * srgb(col), 0, 255))

    # Geometric near clipping, not deleting the genuine terrain triangle that
    # happens to straddle a camera. The rear of the cropped desk stays neutral.
    def submit(model, normal, vc, uv, tri, tex):
        cam = (model - eye) @ R
        z = -cam[:, 2]
        cam[:, 2] = z
        nz = np.clip(normal @ sun, 0, 1)
        direct = 0.87 * nz[:, None] * np.array([1.0, 0.82, 0.62], dtype="f")
        ambient = np.array([0.48, 0.53, 0.59], dtype="f") * (
            0.76 + 0.24 * np.maximum(0, normal[:, 1, None])
        )
        col = np.clip(vc * (ambient + direct), 0.0, 1.0)
        vv = np.column_stack((cam, col, uv))

        def project(v):
            s = f / np.maximum(0.065, v[:, 2]) if persp else np.full(len(v), f)
            return np.column_stack((W * 0.5 + v[:, 0] * s, H * 0.5 - v[:, 1] * s))

        screen = project(vv)
        zmin, zmax = z[tri].min(1), z[tri].max(1)
        keep = zmax > 0.08
        visible = screen[tri]
        ordinary = (
            (zmin > 0.08)
            & keep
            & (visible[:, :, 0].max(1) >= 0)
            & (visible[:, :, 0].min(1) < W)
            & (visible[:, :, 1].max(1) >= 0)
            & (visible[:, :, 1].min(1) < H)
        )
        for ix in tri[ordinary]:
            paint(vv, screen, ix, tex)
        for ix in tri[(zmin <= 0.08) & keep]:
            ring = []
            for i, j in zip(ix, np.roll(ix, -1)):
                va, vb = vv[i], vv[j]
                lo = va[2] > 0.08
                hi = vb[2] > 0.08
                if lo:
                    ring.append(va)
                if lo != hi:
                    ring.append(va + (vb - va) * ((0.08 - va[2]) / (vb[2] - va[2])))
            if len(ring) >= 3:
                r = np.array(ring)
                s = project(r)
                for i in range(1, len(ring) - 1):
                    paint(r, s, np.array([0, i, i + 1]), tex)

    for k in d["scenes"][scene_index]["nodes"]:
        node = d["nodes"][k]
        if "mesh" not in node:
            continue
        ext = (
            node.get("extensions", {})
            .get("EXT_mesh_gpu_instancing", {})
            .get("attributes", {})
        )
        if ext:
            pos = ac(ext["TRANSLATION"])
            qq = ac(ext["ROTATION"])
            ss = ac(ext["SCALE"])
        else:
            pos = [node.get("translation", [0, 0, 0])]
            qq = [node.get("rotation", [0, 0, 0, 1])]
            ss = [node.get("scale", [1, 1, 1])]
        for pr in d["meshes"][node["mesh"]]["primitives"]:
            aa = pr["attributes"]
            v = ac(aa["POSITION"])
            norm = ac(aa["NORMAL"])
            colour = ac(aa["COLOR_0"])[:, :3] if "COLOR_0" in aa else np.ones_like(v)
            uv = (
                ac(aa["TEXCOORD_0"])
                if "TEXCOORD_0" in aa
                else np.zeros((len(v), 2), "f")
            )
            tri = ac(pr["indices"]).ravel().astype(int).reshape(-1, 3)
            texture = material(pr.get("material", 0))
            for trans, q, scale in zip(pos, qq, ss):
                model = (v * scale) @ rot(q).T + np.array(trans, "f")
                normals = unit((norm / scale) @ rot(q).T)
                submit(model, normals, colour, uv, tri, texture)
    out.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(rgb).save(out)
    return dict(
        pixels=[W, H],
        colour_scope="GLB albedo/vertex tint, fixed studio diffuse, no sky or native shading",
        state=scene_index,
        camera=cam_index,
    )


if __name__ == "__main__":
    print(
        picture(
            Path(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), Path(sys.argv[4])
        )
    )
