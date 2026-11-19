"""Natural material transitions at the real southern inlet, from its original UVs.

The near bank remains one masked lit material. Its last metre of actual pigment
is underpainted with the local habitat albedo, roughness and reoriented normal,
not colour from a camera photograph. All projected edges become the surface
already underneath. This file contains no alternate height/physics/simulation.
"""

from pathlib import Path
import json
import numpy as np
from PIL import Image
from scipy.ndimage import distance_transform_edt, map_coordinates
from build_island_geology import Glb
from finished_island_glb import parse, save, values, set_values
from island_inlet_ground import X0, X1, Z0, Z1, Inlet, ease
from island_inlet_distance import linear, srgb


def unit(v):
    return v / np.maximum(np.linalg.norm(v, axis=-1, keepdims=True), 1e-9)


class Habitat:
    def __init__(self, source, terrain):
        self.g = g = Inlet(terrain)
        self.mesh = mesh = Glb(source)
        dat = {k: [] for k in ("p", "n", "uv", "rgb")}
        corners = []
        n = 0
        self.n_grid = {}
        for record in mesh.doc["meshes"]:
            prim = record["primitives"][0]
            a = prim["attributes"]
            p = mesh.attr(a["POSITION"])
            if (
                p[:, 0].max() < X0
                or p[:, 0].min() > X1
                or p[:, 2].max() < Z0
                or p[:, 2].min() > Z1
            ):
                continue
            normals = mesh.attr(a["NORMAL"])
            pix = np.rint((p[:, (0, 2)] - (g.x0, g.z0)) / g.s).astype(int)
            for (ix, iz), nn in zip(pix, normals):
                self.n_grid[int(ix), int(iz)] = nn
            for k, v in [
                ("p", p),
                ("n", normals),
                ("uv", mesh.attr(a["TEXCOORD_0"])),
                ("rgb", mesh.attr(a["COLOR_0"])[:, :3]),
            ]:
                dat[k].append(v)
            faces = mesh.attr(prim["indices"]).reshape(-1, 3)
            corners.append(faces.astype("i4") + n)
            n += len(p)
        self.p, self.n, self.uv, self.rgb = [
            np.concatenate(dat[k]).astype("f") for k in ("p", "n", "uv", "rgb")
        ]
        self.tri = np.concatenate(corners)
        pos = self.p[self.tri]
        cx, cz = pos.mean(axis=1)[:, (0, 2)].T
        iu = np.floor((cx - g.x0) / g.s).astype("i4")
        jv = np.floor((cz - g.z0) / g.s).astype("i4")
        upper = (cx - g.x0) / g.s - iu >= (cz - g.z0) / g.s - jv
        self.lookup = np.full((g.h.shape[0] - 1, g.h.shape[1] - 1, 2), -1, "i4")
        self.lookup[jv, iu, upper.astype(int)] = np.arange(len(cx))
        self.material = mesh.doc["materials"][0]
        pbr = self.material["pbrMetallicRoughness"]
        self.pigment = np.array(pbr.get("baseColorFactor", [1, 1, 1, 1])[:3])
        self.rough = pbr.get("roughnessFactor", 1)
        self.ns = self.material["normalTexture"].get("scale", 1)
        self.oc = self.material.get("occlusionTexture", {}).get("strength", 1)

        def art(v):
            tex = mesh.doc["textures"][v["index"]]
            path = source.parent / mesh.doc["images"][tex["source"]]["uri"]
            return np.asarray(Image.open(path).convert("RGB"))

        self.ink, self.relief, self.orm = map(
            art,
            (
                pbr["baseColorTexture"],
                self.material["normalTexture"],
                pbr["metallicRoughnessTexture"],
            ),
        )

    def record(self, x, z):
        g = self.g
        gx = (x - g.x0) / g.s
        gz = (z - g.z0) / g.s
        i = gx.astype("i4")
        j = gz.astype("i4")
        face = self.lookup[j, i, ((gx - i) >= (gz - j)).astype("i4")]
        if (face < 0).any():
            raise ValueError("A cut-bank face has no matching native habitat ground")
        tri = self.tri[face]
        a, b, c = np.moveaxis(self.p[tri], -2, 0)
        e = b - a
        f = c - a
        ax = x - a[..., 0]
        az = z - a[..., 2]
        det = e[..., 0] * f[..., 2] - f[..., 0] * e[..., 2]
        u = (ax * f[..., 2] - az * f[..., 0]) / det
        v = (az * e[..., 0] - ax * e[..., 2]) / det
        weights = np.stack([1 - u - v, u, v], axis=-1)

        def interp(array):
            return (array[tri] * weights[..., None]).sum(axis=-2)

        uv = interp(self.uv)
        nn = unit(interp(self.n))
        p0, p1, p2 = np.moveaxis(self.uv[tri], -2, 0)
        s = p1 - p0
        t = p2 - p0
        d = s[..., 0] * t[..., 1] - s[..., 1] * t[..., 0]
        ts = (t[..., 1, None] * e - s[..., 1, None] * f) / d[..., None]
        bs = (s[..., 0, None] * f - t[..., 0, None] * e) / d[..., None]
        tx = (f[..., 2, None] * e - e[..., 2, None] * f) / det[..., None]
        bz = (e[..., 0, None] * f - f[..., 0, None] * e) / det[..., None]

        def frame(tangent, bitangent):
            tt = unit(tangent - nn * np.sum(tangent * nn, axis=-1, keepdims=True))
            bt = unit(np.cross(nn, tt))
            bt *= np.where(np.sum(bt * bitangent, axis=-1, keepdims=True) < 0, -1, 1)
            return tt, bt

        s1, s2 = frame(ts, bs)
        d1, d2 = frame(tx, bz)

        def sample(chart):
            xy = [uv[..., 1] * chart.shape[0] - 0.5, uv[..., 0] * chart.shape[1] - 0.5]
            return (
                np.stack(
                    [
                        map_coordinates(chart[..., k], xy, order=1, mode="nearest")
                        for k in range(3)
                    ],
                    -1,
                ).astype("f")
                / 255
            )

        rgb = linear(sample(self.ink)) * self.pigment * interp(self.rgb) / 255
        orm = sample(self.orm)
        orm[..., 0] = 1 - self.oc * (1 - orm[..., 0])
        orm[..., 1] *= self.rough
        n = 2 * sample(self.relief) - 1
        n[..., :2] *= self.ns
        n = unit(n)
        world = s1 * n[..., 0, None] + s2 * n[..., 1, None] + nn * n[..., 2, None]
        norm = np.stack([np.sum(world * vec, -1) for vec in (d1, d2, nn)], axis=-1)
        return rgb, norm, orm


def smooth_bank(source, terrain, original, dest):
    original, dest = map(Path, (original, dest))
    panel = dest / "inlet"
    panel.mkdir(parents=True, exist_ok=True)
    source = Path(source)
    hab = Habitat(source, terrain)
    old = original / "inlet"
    raw = np.array(Image.open(old / "inlet-bed.png").convert("RGBA"))
    h, w = raw.shape[:2]
    ink = linear(raw[..., :3] / 255)
    old_n = unit(
        2 * np.array(Image.open(old / "inlet-normal.png").convert("RGB")) / 255 - 1
    )
    small = Image.open(old / "inlet-orm.png").convert("RGB")
    orm = np.asarray(small.resize((w, h), Image.Resampling.BILINEAR)) / 255
    old_mask = raw[..., 3] >= 110
    distance = distance_transform_edt(old_mask, sampling=((Z1 - Z0) / h, (X1 - X0) / w))
    depth = raw[..., 3] / 255
    # Texture dabs taper along the bank over a metre or so. Native foreground
    # already owns the mid-grey pale seams in the exact outermost ribbon.
    blend = ease(0.12, 1.35, distance) * (0.63 + 0.37 * ease(0.47, 0.94, depth))
    blend = blend.astype("f")
    out_c = raw.copy()
    out_n = np.empty((h, w, 3), "u1")
    out_o = np.empty_like(out_n)
    out_o[..., 2] = 0
    for v0 in range(0, h, 24):
        sl = slice(v0, min(h, v0 + 24))
        v, u = np.mgrid[sl, :w]
        x = X0 + (u + 0.5) * (X1 - X0) / w
        z = Z0 + (v + 0.5) * (Z1 - Z0) / h
        base, n, rough = hab.record(x, z)
        m = blend[sl, :, None]
        mix = lambda a, b: a * (1 - m) + b * m
        out_c[sl, :, :3] = np.uint8(
            np.rint(255 * np.clip(srgb(mix(base, ink[sl])), 0, 1))
        )
        out_n[sl] = np.uint8(np.rint(127.5 * (1 + unit(mix(n, old_n[sl])))))
        # The fitted material retains the original 0.71 AO strength. Undo its
        # attenuation when copying already-effective base occlusion into it.
        target = rough.copy()
        target[..., 0] = np.clip(1 - (1 - target[..., 0]) / 0.71, 0, 1)
        out_o[sl] = np.uint8(np.rint(255 * np.clip(mix(target, orm[sl]), 0, 1)))

    def write(a, name, size=None):
        im = Image.fromarray(a)
        im = im.resize(size, Image.Resampling.BOX) if size else im
        im.save(panel / name, optimize=True)

    write(out_c, "inlet-bed.png")
    write(out_n, "inlet-normal.png")
    write(out_o, "inlet-orm.png", small.size)
    # Apply source smoothing normals to the fitted terrain film only; leave
    # every stone/plant and every position and triangle exactly as authored.
    glb, buf = parse(original / "inlet-bank.glb")
    for node in glb["nodes"]:
        for prim in glb["meshes"][node["mesh"]]["primitives"]:
            if prim["material"] != 0:
                continue
            attrs = prim["attributes"]
            a = attrs["POSITION"]
            idx = attrs["NORMAL"]
            ox, _, oz = node["translation"]
            rows = []
            for x, _, z in values(glb, buf, a):
                key = (
                    round((x + ox - hab.g.x0) / hab.g.s),
                    round((z + oz - hab.g.z0) / hab.g.s),
                )
                rows.append(tuple(hab.n_grid[key]))
            set_values(glb, buf, idx, rows)
    save(glb, buf, dest / "inlet-bank.glb")
    note = dict(
        subject="Material transition independent of the editable southern inlet",
        native_triangle_geometry="unchanged original bank",
        gradual_undercoat_metres=[0.12, 1.35],
        old_covered_pixels=int(old_mask.sum()),
        core_repainted_pixels=int((blend > 0.92).sum()),
        scope="Actual native near habitat sampled along original triangles; corresponding UV basis used for normals.",
    )
    (dest / "bank-source.json").write_text(json.dumps(note, indent=2) + "\n")
    return dest / "inlet-bank.glb"
